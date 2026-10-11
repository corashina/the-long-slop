"""Offline export for the selected Steam build."""
import argparse
import hashlib
import json
import re
from pathlib import Path
from .library.types import Snapshot

APP_ID = 1017180
BUILD_ID = 11140890
SOURCE_WORK_ROOT = Path('.cache/reference-library')


def validate_build(steam_root: Path):
    path = steam_root.parents[1] / f'appmanifest_{APP_ID}.acf'
    values = dict(re.findall(r'"([^"\r\n]+)"\s+"([^"\r\n]+)"', path.read_text(encoding='utf-8')))
    if values.get('appid') != str(APP_ID):
        raise ValueError(f'Steam app mismatch: expected {APP_ID}')
    if values.get('buildid') != str(BUILD_ID):
        raise ValueError(f'Steam build mismatch: expected {BUILD_ID}, got {values.get("buildid")}')
    return values


def export_reference(snapshot: Snapshot, output: Path) -> None:
    from .library.archive import verify_snapshot
    from .source import Source
    verify_snapshot(snapshot)
    _export_selected(Source(snapshot.root), output)


def _export_selected(source, output: Path) -> None:
    from .gltf import export_gltf
    from .parameters import recover_parameters
    from .manifest import build_manifest
    from .conditions import prepare_conditions
    from .visibility import recover_camera_visibility
    if output.resolve().is_relative_to(source.root.resolve()):
        raise ValueError('Keep generated files outside the source installation')
    def transform(go):return source.transform_for(source.get('level1',go))
    roots = {'home':[source.get('level1',21400)], 'lada':[transform(17687)],
        'items':[transform(go) for go in (13023,13027,18878,13019,15637)]}
    spare=source.get('level1',71403)
    roots['items'].append(source.transform_for(source.resolve(spare,source.read(spare)['m_GameObject'])))
    prefab=source.get('sharedassets1.assets',82556)
    scene_paths=[n['path'] for n in source.hierarchy(roots['lada'][0])]
    prefab_paths=[n['path'] for n in source.hierarchy(prefab)]
    if scene_paths != prefab_paths or len(scene_paths)!=697:
        raise ValueError('Selected scene Lada differs from the required prefab hierarchy')
    if source.read(source.get('level1',64896))['FirstHome']['m_PathID']!=21400:
        raise ValueError('Selected home link differs from the required home')
    if source.read(source.get('level1',64898))['StartCar02']['m_PathID']!=206:
        raise ValueError('Selected Lada starter link differs from the required car')
    prepare_conditions(source,[root for readers in roots.values() for root in readers])
    output.mkdir(parents=True, exist_ok=True)
    # Remove acceptance before any export. A failed run cannot leave it usable.
    (output/'manifest.json').unlink(missing_ok=True)
    report = {}
    for name, readers in roots.items():
        report[name] = export_gltf((source,readers),output/f'{name}.gltf')
        print(f'Exported {name}: {report[name]["node_count"]} nodes',flush=True)
    (output/'geometry-evidence.json').write_text(json.dumps(report,indent=2,allow_nan=False),encoding='utf-8')
    parameter_roots={f'{name}-{i}':root for name,readers in roots.items() for i,root in enumerate(readers)}
    parameter_roots['required-prefab']=prefab
    parameters = recover_parameters(source,parameter_roots)
    parameters['conditions']=source.condition_evidence
    parameters['camera_visibility']=recover_camera_visibility(source)
    (output/'parameter-evidence.json').write_text(json.dumps(parameters,indent=2,allow_nan=False),encoding='utf-8')
    if parameters['errors']:
        raise ValueError(f'Incomplete parameter recovery: {parameters["errors"]}')
    data_root=source.root/'TheLongDrive_Data'
    inputs=[path for path in data_root.iterdir() if path.is_file() and (path.name in {'level0','level1','globalgamemanagers'} or path.suffix in {'.assets','.resS'})]
    inputs.append(data_root/'Managed'/'Assembly-CSharp.dll')
    hashes=[]
    for path in sorted(inputs):
        digest=hashlib.file_digest(path.open('rb'),'sha256').hexdigest()
        hashes.append({'file':str(path.relative_to(data_root)).replace('\\','/'),'sha256':digest})
    manifest=build_manifest(source,roots,report,hashes)
    parameters['controls']={
        'source':{'file':'level0','path_id':16865},
        'defaults':{'object_move':manifest['player']['obj_move_sensitivity'],'object_rotation':manifest['player']['obj_rotation_sensitivity'],
            'look_xy':manifest['player']['look_sensitivity'],'foot_fov':manifest['player']['field_of_view_degrees']},
        'slider_source_ids':[18564,18972,19178,17280,18711,17625,18360],
        'slider_multiplier_rule':'slidersettingscript.FStart changes zero valueMultipler to one; GetValue returns value * valueMultipler.',
        'carry_minimum_rule':'Distance range derives from the authored carry marker and maxPickupDist offset sphere.',
    }
    parameters['player_markers']=[{'source':{'file':'level1','path_id':pid},'fields':source.read(source.get('level1',pid))} for pid in (39823,23339)]
    (output/'parameter-evidence.json').write_text(json.dumps(parameters,indent=2,allow_nan=False),encoding='utf-8')
    (output/'manifest.json').write_text(json.dumps(manifest,indent=2,allow_nan=False),encoding='utf-8')
    print(f'Exported manifest: {len(manifest["colliders"])} colliders, {len(manifest["items"])} items',flush=True)


def stage_source(steam_root: Path | None, verify: bool = False) -> Snapshot:
    from .library.archive import load_snapshot, snapshot_source, verify_snapshot
    from .library.inventory import inventory_source, catalog_data
    from .source import Source
    work_root = SOURCE_WORK_ROOT
    if verify:
        snapshot = load_snapshot(work_root)
        verify_snapshot(snapshot)
    else:
        if steam_root is None:
            raise ValueError('Source staging needs --steam-root')
        snapshot = snapshot_source(steam_root, work_root)
    source = Source(snapshot.root)
    catalog = inventory_source(source, snapshot)
    if len(source.serialized_files) != 9 or len(catalog.objects) != 262784:
        raise ValueError(f'Installed inventory mismatch: {len(source.serialized_files)} serialized files, {len(catalog.objects)} objects')
    data = catalog_data(catalog, source, snapshot)
    catalog_path = work_root / 'catalog.json'
    if verify or catalog_path.exists():
        accepted = json.loads(catalog_path.read_text(encoding='utf-8'))
        for item in accepted['objects']:
            item['dependencies'] = []
        if accepted != data:
            raise ValueError('Archived catalog differs from the complete inventory')
    else:
        temporary = work_root / '.catalog.json'
        temporary.write_text(json.dumps(data, indent=2, allow_nan=False) + '\n', encoding='utf-8')
        temporary.replace(catalog_path)
    action = 'Verified' if verify else 'Archived'
    print(f'{action} source: {len(source.serialized_files)} serialized files, {len(source.resource_streams)} streams, {len(catalog.objects):,} objects', flush=True)
    return snapshot


def stage_catalog(steam_root: Path | None, verify: bool = False):
    from .library.archive import load_snapshot, verify_snapshot
    from .library.inventory import catalog_data
    from .library.records import build_catalog, verify_catalog
    from .source import Source
    import shutil
    work_root = SOURCE_WORK_ROOT
    if not (work_root / 'source-manifest.json').exists():
        if verify:
            raise ValueError('No accepted source stage')
        stage_source(steam_root)
    snapshot = load_snapshot(work_root)
    verify_snapshot(snapshot)
    source = Source(snapshot.root)
    catalog_path = work_root / 'catalog.json'
    records_root = work_root / 'records'
    if verify:
        catalog = verify_catalog(source, snapshot, catalog_path, records_root)
    else:
        pending = work_root / 'work' / 'catalog-records'
        if not pending.resolve().is_relative_to(work_root.resolve()):
            raise ValueError('Catalog work path escapes work root')
        if pending.exists():
            shutil.rmtree(pending)
        pending.mkdir(parents=True)
        catalog = build_catalog(source, snapshot, pending)
        data = catalog_data(catalog, source, snapshot)
        content = (json.dumps(data, indent=2, allow_nan=False) + '\n').encode('utf-8')
        receipt = json.loads((pending / 'manifest.json').read_bytes())
        receipt['catalog_sha256'] = hashlib.sha256(content).hexdigest()
        (pending / 'manifest.json').write_text(json.dumps(receipt, sort_keys=True, separators=(',', ':'), allow_nan=False) + '\n', encoding='utf-8')
        temporary = work_root / 'work' / 'catalog.json'
        temporary.write_bytes(content)
        # Publish acceptance only after every required record and graph check passes.
        if not records_root.resolve().is_relative_to(work_root.resolve()):
            raise ValueError('Records path escapes work root')
        if records_root.exists():
            shutil.rmtree(records_root)
        pending.replace(records_root)
        temporary.replace(catalog_path)
    action = 'Verified' if verify else 'Indexed'
    edges = sum(len(value) for value in catalog.dependencies.values())
    print(f'{action} catalog: {len(catalog.objects):,} records, {edges:,} dependency edges', flush=True)
    return catalog


def _category_stages(snapshot, catalog):
    """Reuse source-bound stages, or build missing category prerequisites once."""
    import tempfile
    import shutil
    from dataclasses import asdict
    from .source import Source
    from .library import (export_images, export_ui, export_audio, export_models,
                          export_animations, export_native_assets, index_source_text,
                          index_managed, write_analysis_report)
    from .library.archive import sha256
    from .library.records import _file, _json
    from .library.validation import CATEGORIES, load_exports, _receipt, _check_files, _verify_managed, _publish_directory
    root = snapshot.manifest_path.parent
    path = root / 'category-stages.json'
    bindings = {'schema_version': 1, 'source_manifest_sha256': snapshot.manifest_sha256,
                'catalog_sha256': sha256(root / 'catalog.json'),
                'records_manifest_sha256': sha256(root / 'records/manifest.json')}
    if path.exists():
        saved = json.loads(path.read_bytes())
        if any(saved.get(k) != v for k, v in bindings.items()):
            raise ValueError('Category stages differ from the accepted source or catalog')
    else:
        (root / 'work').mkdir(exist_ok=True)
        pending = Path(tempfile.mkdtemp(prefix='category-assets-', dir=root / 'work'))
        source = Source(snapshot.root)
        prior = (*export_images(source, catalog, pending), *export_ui(source, catalog, pending),
                 *export_audio(source, catalog, pending))
        models = export_models(source, catalog, pending)
        prior = (*prior, *models, *export_animations(source, catalog, models, pending))
        records = root / 'records'
        native_root = pending / 'record-inputs'
        native_root.mkdir()
        # This partial root supplies producers only. It is not an accepted record stage.
        shutil.copyfile(records / 'manifest.json', native_root / 'manifest.json')
        for identity, item in catalog.objects.items():
            if item.type in ('MonoScript', 'ResourceManager'):
                name = f'{identity.file}/{identity.path_id}.json'
                (native_root / name).parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(records / name, native_root / name)
        export_native_assets(source, catalog, native_root, frozenset(e.source for e in prior))
        index_source_text(snapshot, native_root)
        if not (records / 'managed/manifest.json').exists():
            managed = index_managed(snapshot, native_root)
            analysis = write_analysis_report(catalog, managed, native_root)
            _file(native_root, 'managed/manifest.json', _json({'schema_version': 1,
                  'files': [asdict(f) for f in (*managed, analysis)]}))
            _verify_managed(snapshot, native_root)
            _publish_directory(native_root / 'managed', records / 'managed')
        else:
            _verify_managed(snapshot, records)
        saved = {**bindings, 'categories': [{'category': category,
                 'root': (native_root if category == 'native' else pending).relative_to(root).as_posix(),
                 'index': asdict(_receipt(native_root if category == 'native' else pending, f'{category}/index.json'))}
                 for category in CATEGORIES]}
        _file(root, 'category-stages.json', _json(saved))
    if [e['category'] for e in saved['categories']] != list(CATEGORIES):
        raise ValueError('Missing or duplicate category stages')
    stages = []
    for entry in saved['categories']:
        from .library.types import FileRecord
        from .library.archive import validate_paths, _check_path
        validate_paths([entry['root']])
        directory = root / entry['root']
        _check_path(directory, root.resolve())
        record = FileRecord(**entry['index'])
        if record.file != f"{entry['category']}/index.json":
            raise ValueError('Category index identity mismatch')
        _check_files(directory, {record.file: record})
        stages.append((entry['category'], directory, load_exports(directory / record.file)))
    return stages


def stage_library(steam_root: Path | None, output: Path, verify: bool = False):
    import tempfile
    from .library.archive import load_snapshot
    from .library.validation import (load_catalog, check_installed_counts, assemble_library,
                                     publish_library, verify_library)
    library = output.parent / 'library'
    if verify:
        receipt = verify_library(library, SOURCE_WORK_ROOT / 'source')
        check_installed_counts(load_catalog(SOURCE_WORK_ROOT / 'catalog.json'))
        print(f'Verified library: {len(receipt.exports):,} owners; {receipt.manifest_sha256}', flush=True)
        return receipt
    if not (SOURCE_WORK_ROOT / 'source-manifest.json').exists():
        stage_source(steam_root)
    if not (SOURCE_WORK_ROOT / 'records/manifest.json').exists():
        stage_catalog(steam_root)
    snapshot = load_snapshot(SOURCE_WORK_ROOT)
    catalog = load_catalog(SOURCE_WORK_ROOT / 'catalog.json')
    check_installed_counts(catalog)
    stages = _category_stages(snapshot, catalog)
    pending = Path(tempfile.mkdtemp(prefix='library-', dir=SOURCE_WORK_ROOT / 'work'))
    exports = assemble_library(snapshot, stages, pending)
    print(f'Assembled library: {len(exports):,} owners', flush=True)
    receipt = publish_library(snapshot, catalog, exports, pending, library)
    print(f'Published library: {len(exports):,} owners; {receipt.manifest_sha256}', flush=True)
    return receipt


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--steam-root', type=Path)
    parser.add_argument('--output', type=Path, default=Path('web/assets/reference'))
    parser.add_argument('--stage', choices=('runtime', 'source', 'catalog', 'library'), default='runtime')
    parser.add_argument('--verify', action='store_true')
    arguments = parser.parse_args(argv)
    if arguments.stage == 'source':
        stage_source(arguments.steam_root, verify=arguments.verify)
    elif arguments.stage == 'catalog':
        stage_catalog(arguments.steam_root, verify=arguments.verify)
    elif arguments.stage == 'library':
        stage_library(arguments.steam_root, arguments.output, verify=arguments.verify)
    else:
        if arguments.steam_root is None:
            parser.error('Runtime export needs --steam-root')
        if arguments.output.resolve().is_relative_to(arguments.steam_root.resolve()):
            parser.error('Keep runtime output outside the Steam installation')
        if arguments.verify:
            parser.error('Runtime verification belongs to the runtime stage task')
        stage_library(arguments.steam_root, arguments.output)
        from .library.archive import load_snapshot
        export_reference(load_snapshot(SOURCE_WORK_ROOT), arguments.output)


if __name__ == '__main__':
    main()
