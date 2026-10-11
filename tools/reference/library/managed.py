"""Index archived assemblies and retain separate dynamic-analysis evidence."""
import hashlib
import json
import subprocess
from collections import Counter, defaultdict
from dataclasses import asdict
from pathlib import Path

from .archive import sha256
from .records import _file, _json
from .types import Catalog, DATA_DIRECTORY, FileRecord, ObjectId, SCHEMA_VERSION, Snapshot

SCRIPT = Path(__file__).resolve().parents[1] / 'decompile.ps1'


def index_managed(snapshot: Snapshot, output: Path) -> tuple[FileRecord, ...]:
    assemblies = tuple(item for item in snapshot.files if item.file.startswith(f'{DATA_DIRECTORY}/Managed/')
                       and item.file.lower().endswith('.dll'))
    if not assemblies:
        raise ValueError('No archived managed assemblies')
    for item in assemblies:
        path = snapshot.root / item.file
        if path.stat().st_size != item.bytes or sha256(path) != item.sha256:
            raise ValueError(f'Managed source hash mismatch: {item.file}')
    managed = output / 'managed'
    managed.mkdir(parents=True, exist_ok=True)
    steps = (['-Mode', 'Index', '-AssemblyPath', str(snapshot.root / DATA_DIRECTORY / 'Managed')],
             ['-AssemblyPath', str(snapshot.root / DATA_DIRECTORY / 'Managed/Assembly-CSharp.dll')])
    for arguments in steps:
        result = subprocess.run(['pwsh', '-NoProfile', '-File', str(SCRIPT), *arguments,
                                 '-OutputPath', str(managed)], capture_output=True, text=True)
        if result.returncode:
            _file(output, 'managed/diagnostics.json', _json({'schema_version': SCHEMA_VERSION,
                  'complete': False, 'arguments': arguments, 'stdout': result.stdout,
                  'stderr': result.stderr, 'source_files': [asdict(item) for item in assemblies]}))
            raise ValueError(f'Managed analysis incomplete: {result.stderr.strip()}')
    index = json.loads((managed / 'index.json').read_bytes())
    decompilation = json.loads((managed / 'decompilation.json').read_bytes())
    expected = {item.file: item.sha256 for item in assemblies}
    observed = {item['file']: item['sha256'] for item in index['assemblies']}
    if expected != observed or not index['complete'] or not decompilation['complete']:
        raise ValueError('Managed analysis incomplete: source index or decompilation mismatch')
    if decompilation['input']['sha256'] != expected[f'{DATA_DIRECTORY}/Managed/Assembly-CSharp.dll']:
        raise ValueError('Managed analysis incomplete: decompiler input hash mismatch')
    _checked_file(output, FileRecord(**decompilation['output']))
    for item in assemblies:
        if sha256(snapshot.root / item.file) != item.sha256:
            raise ValueError(f'Managed source changed: {item.file}')
    _file(output, 'managed/diagnostics.json', _json({'schema_version': SCHEMA_VERSION,
          'complete': True, 'source_manifest_sha256': snapshot.manifest_sha256,
          'source_files': [asdict(item) for item in assemblies],
          'index_diagnostics': index['diagnostics'], 'decompiler_diagnostics': decompilation['diagnostics']}))
    names = ('Assembly-CSharp.cs', 'decompilation.json', 'diagnostics.json', 'index.json')
    return tuple(FileRecord(f'managed/{name}', (managed / name).stat().st_size, sha256(managed / name))
                 for name in names)


def _checked_file(output, record):
    path = output / record.file
    raw = path.read_bytes()
    if len(raw) != record.bytes or hashlib.sha256(raw).hexdigest() != record.sha256:
        raise ValueError(f'Managed output hash mismatch: {record.file}')
    return raw


def _checked_json(output, record):
    result = json.loads(_checked_file(output, record))
    if result['schema_version'] != SCHEMA_VERSION:
        raise ValueError('Obsolete managed schema')
    return result


def write_analysis_report(catalog: Catalog, managed_files: tuple[FileRecord, ...], output: Path) -> FileRecord:
    """Keep proven serialized links and unproven code links in separate lists."""
    records = {record.file: record for record in managed_files}
    index = _checked_json(output, records['managed/index.json'])
    if not index['complete']:
        raise ValueError('Managed analysis incomplete')
    type_index = {(item['assembly'], item['full_name']): item for item in index['types']}
    assemblies = {item['name']: item for item in index['assemblies']}
    display_names, short_names, uses = defaultdict(list), defaultdict(list), Counter()
    for item in index['types']:
        display_names[item['assembly'], item['full_name'].replace('+', '.')].append(item)
        short_names[item['assembly'], item['full_name'].split('+')[-1].rsplit('.', 1)[-1].casefold()].append(item)
    for identity, entry in catalog.objects.items():
        if entry.type == 'MonoBehaviour':
            for target in catalog.dependencies[identity]:
                if catalog.objects[target].type == 'MonoScript':
                    uses[target] += 1
    scripts, resources, limits = [], [], []
    # Read only these small native records. Complete component payloads remain shared.
    for identity, entry in sorted(catalog.objects.items()):
        if entry.type not in ('MonoScript', 'ResourceManager'):
            continue
        path = output / f'{identity.file}/{identity.path_id}.json'
        payload = json.loads(path.read_bytes())['data']
        if entry.type == 'MonoScript':
            assembly_name = payload['m_AssemblyName']
            assembly = assemblies.get(assembly_name.removesuffix('.dll'))
            full_name = '.'.join(part for part in (payload['m_Namespace'], payload['m_ClassName']) if part)
            owner = type_index.get((assembly['file'], full_name)) if assembly else None
            item = {'source': asdict(identity), 'assembly_name': assembly_name, 'type': full_name,
                    'assembly': assembly['file'] if assembly else None,
                    'assembly_sha256': assembly['sha256'] if assembly else None,
                    'type_token': owner['token'] if owner else None,
                    'status': 'resolved' if owner else 'unresolved',
                    'stored_component_count': uses[identity], 'candidates': [],
                    'reason': '' if owner else 'declared_assembly_not_shipped' if assembly is None
                              else 'declared_type_not_in_archived_assembly'}
            if owner is None and assembly:
                candidates = list(display_names[assembly['file'], full_name])
                if not payload['m_Namespace'] and '.' not in payload['m_ClassName']:
                    candidates.extend(short_names[assembly['file'], payload['m_ClassName'].casefold()])
                candidates = {candidate['full_name']: candidate for candidate in candidates}
                item['candidates'] = [{'full_name': candidate['full_name'], 'token': candidate['token'],
                                       'status': 'unproven', 'reason': 'Namespace, nested name, or letter case differs.'}
                                      for _, candidate in sorted(candidates.items())]
            scripts.append(item)
            if owner is None:
                limits.append({'kind': 'MonoScript_type', **item})
        else:
            resources.append({'source': asdict(identity), 'typed_record': f'records/{identity.file}/{identity.path_id}.json',
                              'dependencies': [asdict(value) for value in catalog.dependencies[identity]]})
    links = json.loads((output / 'native/managed-links.json').read_bytes())
    if links['schema_version'] != SCHEMA_VERSION:
        raise ValueError('Obsolete native analysis schema')
    calls = index['asset_calls']
    proven, creation_sites, resolved_calls = [], [], set()
    assembly_path = f'{DATA_DIRECTORY}/Managed/Assembly-CSharp.dll'
    assembly = next((item for item in index['assemblies'] if item['file'] == assembly_path), None)
    pinned = '9919a9eeddcce23bb4ff86e890437f32b4a3ef3c10af2efe3b8d889d894dd62e'
    code = ''
    code_receipt = records.get('managed/Assembly-CSharp.cs')
    if code_receipt:
        code = _checked_file(output, code_receipt).decode('utf-8')
    if assembly and assembly['sha256'] != pinned:
        raise ValueError('Dynamic analysis evidence does not match assembly')
    def call_key(call):
        return (call['assembly'], call['method_token'], call['il_offset'])
    for call in calls:
        target = call['target']
        if call['assembly'] != assembly_path:
            continue
        if target['method'] in ('.ctor', 'Create', 'GetTemporary'):
            creation_sites.append({'call': call, 'assembly_sha256': pinned,
                                   'reason': 'IL proves an asset creation call; the runtime result has no serialized ObjectId.'})
        if target['type'] == 'UnityEngine.Resources' and target['method'] == 'Load':
            literals = call['preceding_string_literals']
            literal = literals[-1] if literals else None
            if literal and literal['il_offset'] + 5 == call['il_offset']:
                matches = [row for row in links['resources'] if row['target'] and row['name'].casefold() == literal['value'].casefold()]
                for row in matches:
                    if ObjectId(**row['target']) not in catalog.objects:
                        raise ValueError('Resources call has an unknown target')
                    proven.append({'kind': 'Resources.Load', 'call': call, 'resource': row,
                                   'assembly_sha256': pinned, 'argument_proof': 'The immediately preceding ldstr supplies the name.'})
                    resolved_calls.add(call_key(call))
    creation_rules = {
        ('computerscript', 'defRenderTexture'): ('computerscript', 'Start', 'UnityEngine.RenderTexture', 'output = new RenderTexture(defRenderTexture);'),
        ('mainscript', 'defMirrorTexture'): ('mirrorscript', 'FStart', 'UnityEngine.RenderTexture', 'M.rt = new RenderTexture(mainscript.M.defMirrorTexture);'),
        ('mainscript', 'defMirrorMaterial'): ('mirrorscript', 'FStart', 'UnityEngine.Material', 'M.mat = new Material(mainscript.M.defMirrorMaterial);'),
        ('tavcsoscript', 'defRt'): ('tavcsoscript', 'Start', 'UnityEngine.RenderTexture', 'rt = new RenderTexture(defRt);'),
        ('tavcsoscript', 'defM'): ('tavcsoscript', 'Start', 'UnityEngine.Material', 'M = new Material(defM);')}
    for binding in links['creation_bindings']:
        owner, method, asset_type, statement = creation_rules[binding['type'], binding['field']]
        matches = [call for call in calls if call['assembly'] == assembly_path and call['type'] == owner
                   and call['method'] == method and call['target']['type'] == asset_type and call['target']['method'] == '.ctor']
        if statement not in code or len(matches) != 1:
            raise ValueError(f'Creation source proof missing: {owner}.{method}')
        target = ObjectId(**binding['target'])
        component = ObjectId(**binding['component'])
        if target not in catalog.dependencies[component]:
            raise ValueError('Creation declaration does not match serialized dependency')
        proven.append({'kind': 'source_descriptor_creation', 'binding': binding, 'call': matches[0],
                       'statement': statement, 'code_file': asdict(code_receipt), 'assembly_sha256': pinned,
                       'selection': 'Source declaration; component lifetime and singleton selection remain runtime behavior.'})
    if links['controller_assignments']:
        controller_calls = [call for call in calls if call['assembly'] == assembly_path
                            and call['type'] == 'weaponhandlingscript' and call['method'] == 'Update'
                            and call['target']['method'] == 'set_runtimeAnimatorController']
        if len(controller_calls) != 1 or 'anim.runtimeAnimatorController = melee.AnimationController;' not in code:
            raise ValueError('Controller assignment source proof missing')
        for assignment in links['controller_assignments']:
            if assignment['evidence']['sha256'] != pinned:
                raise ValueError('Controller evidence hash mismatch')
            proven.append({'kind': 'controller_assignment', 'assignment': assignment, 'call': controller_calls[0],
                           'code_file': asdict(code_receipt)})
        resolved_calls.add(call_key(controller_calls[0]))
    for call in calls:
        if call_key(call) not in resolved_calls:
            limits.append({'kind': 'code_asset_call', 'reason': 'The call alone does not prove a serialized result or runtime argument.',
                           'call': call})
    bone_calls = [call for call in calls if call['target']['method'] in ('set_bones', 'set_rootBone')]
    for rig in links['source_unbound_rigs']:
        limits.append({'kind': 'source_unbound_rig', 'rig': rig, 'bone_assignment_calls': bone_calls,
                       'reason': 'No exact bone identities follow from null serialized slots or the managed call index.'})
    summary = {'total': len(scripts), 'stored_component_references': sum(uses.values()),
               'used_script_types': sum(uses[ObjectId(**s['source'])] > 0 for s in scripts), 'exact_types': sum(s['status'] == 'resolved' for s in scripts),
               'unmatched': sum(s['status'] == 'unresolved' for s in scripts),
               'missing_assembly': sum(s['reason'] == 'declared_assembly_not_shipped' for s in scripts),
               'missing_declared_type': sum(s['reason'] == 'declared_type_not_in_archived_assembly' for s in scripts),
               'unmatched_referenced_scripts': sum(s['status'] == 'unresolved' and s['stored_component_count'] > 0 for s in scripts),
               'unmatched_referenced_components': sum(s['stored_component_count'] for s in scripts if s['status'] == 'unresolved'),
               'unmatched_unused_scripts': sum(s['status'] == 'unresolved' and s['stored_component_count'] == 0 for s in scripts),
               'unproven_candidate_declarations': sum(bool(s['candidates']) for s in scripts)}
    typed_decode = None
    if scripts:
        manifest_path = output / 'manifest.json'
        typed_decode = asdict(FileRecord('manifest.json', manifest_path.stat().st_size, sha256(manifest_path)))
    report = {'schema_version': SCHEMA_VERSION, 'script_summary': summary,
              'typed_decode_evidence': typed_decode, 'managed_files': [asdict(item) for item in managed_files],
              'scripts': scripts, 'resources': links['resources'], 'resource_managers': resources, 'asset_calls': calls,
              'proven_dynamic_links': proven, 'proven_creation_sites': creation_sites,
              'limits': limits, 'serialized_dependency_graph_changed': False}
    return _file(output, 'managed/analysis.json', _json(report))


def index_source_text(snapshot: Snapshot, output: Path) -> tuple[FileRecord, ...]:
    """Copy authored text files; the UI category owns installed Languages files."""
    files, entries = [], []
    for item in snapshot.files:
        if not item.file.lower().endswith('.txt') or item.file.startswith(f'{DATA_DIRECTORY}/Languages/'):
            continue
        payload = (snapshot.root / item.file).read_bytes()
        if len(payload) != item.bytes or hashlib.sha256(payload).hexdigest() != item.sha256:
            raise ValueError(f'Source text hash mismatch: {item.file}')
        receipt = _file(output, f'native/source-text/{item.file}', payload)
        files.append(receipt)
        entries.append({'source': asdict(item), 'output': asdict(receipt)})
    files.append(_file(output, 'native/source-text.json', _json({'schema_version': SCHEMA_VERSION, 'files': entries})))
    return tuple(sorted(files, key=lambda item: item.file))


def native_managed_links(source, catalog, output: Path) -> FileRecord:
    """Resolve serialized fields that the pinned module uses for dynamic creation."""
    from .records import _pointer
    from .animations import _dynamic_assignments
    rules = {'computerscript': ('defRenderTexture',), 'mainscript': ('defMirrorTexture', 'defMirrorMaterial'),
             'tavcsoscript': ('defRt', 'defM')}
    scripts, controllers, contexts, resources, bindings, rigs = {}, {}, [], [], [], []
    for identity, entry in sorted(catalog.objects.items()):
        if entry.type not in ('MonoScript', 'ResourceManager', 'Animator', 'AnimatorController', 'SkinnedMeshRenderer'):
            continue
        reader = source.get(identity.file, identity.path_id)
        data = source.read_record(reader)
        if entry.type == 'MonoScript':
            if data['m_ClassName'] in rules:
                scripts[identity] = data['m_ClassName']
        elif entry.type == 'ResourceManager':
            for name, pointer in data['m_Container']:
                target = _pointer(source, reader, pointer)
                if target is not None and target not in catalog.dependencies[identity]:
                    raise ValueError('Resource name has missing serialized dependency')
                resources.append({'name': name, 'manager': asdict(identity), 'target': asdict(target) if target else None})
        elif entry.type == 'AnimatorController':
            controllers[identity] = tuple(_pointer(source, reader, value) for value in data['m_AnimationClips'])
        elif entry.type == 'Animator':
            contexts.append({'owner': asdict(identity), 'clips': []})
        else:
            mesh = _pointer(source, reader, data['m_Mesh'])
            bones = tuple(_pointer(source, reader, value) for value in data['m_Bones'])
            root = _pointer(source, reader, data['m_RootBone'])
            if mesh and bones and not any(bones) and root is None:
                rigs.append({'renderer': asdict(identity), 'mesh': asdict(mesh), 'bone_slots': len(bones),
                             'root_bone': None, 'reason': 'Serialized bone slots and RootBone are null.'})
    for identity, entry in sorted(catalog.objects.items()):
        if entry.type != 'MonoBehaviour':
            continue
        owners = [value for value in catalog.dependencies[identity] if value in scripts]
        if not owners:
            continue
        if len(owners) != 1:
            raise ValueError('Component has multiple creation script owners')
        script = owners[0]
        reader = source.get(identity.file, identity.path_id)
        data = source.read_record(reader)
        for field in rules[scripts[script]]:
            target = _pointer(source, reader, data[field])
            if target is not None:
                if target not in catalog.dependencies[identity]:
                    raise ValueError('Creation field has missing serialized dependency')
                bindings.append({'component': asdict(identity), 'script': asdict(script), 'type': scripts[script],
                                 'field': field, 'target': asdict(target) if target else None})
    if contexts:
        _dynamic_assignments(source, catalog, controllers, contexts)
    assignments = [{'animator': context['owner'], **item} for context in contexts
                   for item in context.get('controller_assignments', [])]
    return _file(output, 'native/managed-links.json', _json({'schema_version': SCHEMA_VERSION,
           'resources': resources, 'creation_bindings': bindings, 'controller_assignments': assignments,
           'source_unbound_rigs': rigs}))
