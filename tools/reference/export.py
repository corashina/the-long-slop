"""Offline export for the selected Steam build."""
import argparse
import hashlib
import json
import re
from pathlib import Path

APP_ID = 1017180
BUILD_ID = 11140890


def validate_build(steam_root: Path):
    path = steam_root.parents[1] / f'appmanifest_{APP_ID}.acf'
    values = dict(re.findall(r'"([^"\r\n]+)"\s+"([^"\r\n]+)"', path.read_text(encoding='utf-8')))
    if values.get('appid') != str(APP_ID):
        raise ValueError(f'Steam app mismatch: expected {APP_ID}')
    if values.get('buildid') != str(BUILD_ID):
        raise ValueError(f'Steam build mismatch: expected {BUILD_ID}, got {values.get("buildid")}')
    return values


def export_reference(steam_root: Path, output: Path) -> None:
    from .gltf import export_gltf
    from .source import Source
    from .parameters import recover_parameters
    from .manifest import build_manifest
    from .conditions import prepare_conditions
    from .visibility import recover_camera_visibility
    if output.resolve().is_relative_to(steam_root.resolve()):
        raise ValueError('Keep generated files outside the Steam installation')
    validate_build(steam_root)
    source = Source(steam_root)
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
    source.live_mirror_sources = []
    source.live_mirror_renderers = {}
    main_reader = source.get('level1', 64919)
    main = source.read(main_reader)
    mirror_material = source.resolve(main_reader, main['defMirrorMaterial'])
    mirror_texture = source.resolve(main_reader, main['defMirrorTexture'])
    from .source import source_id, key
    for node in source.hierarchy(roots['lada'][0], activate_root=True):
        for reader in node['components']:
            if reader.type.name == 'MonoBehaviour' and source.class_name(reader) == 'mirrorscript':
                data = source.read(reader)['M']
                if data['R']['m_PathID'] and data['C']['m_PathID']:
                    source.live_mirror_sources.append(source_id(reader))
                    source.live_mirror_renderers[key(source.resolve(reader, data['R']))] = mirror_material
    source.live_mirror_sources.append(source_id(mirror_texture))
    prepare_conditions(source,[root for readers in roots.values() for root in readers])
    output.mkdir(parents=True, exist_ok=True)
    # Remove acceptance before any export. A failed run cannot leave it usable.
    (output/'manifest.json').unlink(missing_ok=True)
    report = {}
    for name, readers in roots.items():
        report[name] = export_gltf((source,readers),output/f'{name}.gltf')
        print(f'Exported {name}: {report[name]["node_count"]} nodes',flush=True)
    (output/'material-evidence.json').write_text(json.dumps(list(source.material_evidence.values()), indent=2, allow_nan=False), encoding='utf-8')
    (output/'geometry-evidence.json').write_text(json.dumps(report,indent=2,allow_nan=False),encoding='utf-8')
    parameter_roots={f'{name}-{i}':root for name,readers in roots.items() for i,root in enumerate(readers)}
    parameter_roots['required-prefab']=prefab
    parameters = recover_parameters(source,parameter_roots)
    parameters['conditions']=source.condition_evidence
    parameters['camera_visibility']=recover_camera_visibility(source)
    (output/'parameter-evidence.json').write_text(json.dumps(parameters,indent=2,allow_nan=False),encoding='utf-8')
    if parameters['errors']:
        raise ValueError(f'Incomplete parameter recovery: {parameters["errors"]}')
    data_root=steam_root/'TheLongDrive_Data'
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


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--steam-root',type=Path,required=True)
    parser.add_argument('--output',type=Path,default=Path('web/assets/reference'))
    arguments = parser.parse_args()
    export_reference(arguments.steam_root,arguments.output)
