"""Run checked source-world probes. Proprietary helpers remain in ignored storage."""
import argparse
import csv
import hashlib
import json
import subprocess
import struct
import sys
import uuid
from pathlib import Path

from .export import NATIVE_ENGINE_SHA256, native_engine_evidence
from .library.reader import LibraryReader
from .library.types import ObjectId
from .library.validation import verify_library

STEAM_ROOT = Path('C:/Program Files (x86)/Steam/steamapps/common/The Long Drive')
REPO_ROOT = Path(__file__).resolve().parents[2]
PROBE_ROOT = REPO_ROOT / '.cache/world-source'
SEEDS = [42, 0, -1705217151, -2147483648, 2147483647]
SEGMENTS = [0, 1, 99, 100, 255, 256, 257, 999, 1000, 9999, 49999]
MANAGED_SHA256 = '9919a9eeddcce23bb4ff86e890437f32b4a3ef3c10af2efe3b8d889d894dd62e'
HARNESS_SHA256 = 'b4ed417e66d9e14aaa297e07ed2715b8d6fd023078f524481a985bffca9a77a1'
LIBRARY_SHA256 = '92a93abde82f7933090293e8c95880f98dcee1d50a7463e74b108e29e7b2db99'
INPUT_CONTRACT_SHA256 = 'b32029791397f5ceb00178adde8ed00dc5ec90ca8a65d5e5974ad640553ecaa6'
PAYLOAD_FIELDS = ('native', 'managed', 'assembled')
MANAGED_FILES = {'placement_diagnostic': 'placement.csv', 'terrain_diagnostic': 'placement.terrain.csv',
                 'road_diagnostic': 'placement.road.csv'}


def digest(path: Path) -> str:
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def payload_digest(envelope: dict) -> str:
    payload = {key: envelope[key] for key in PAYLOAD_FIELDS}
    return value_digest(payload)


def value_digest(value) -> str:
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(',', ':'), allow_nan=False).encode()).hexdigest()


def checked_harness() -> dict:
    path = PROBE_ROOT / 'harness.json'
    if not path.is_file() or digest(path) != HARNESS_SHA256:
        raise ValueError('Checked ignored source harness is missing or changed')
    manifest = json.loads(path.read_text())
    for name, expected in manifest['files'].items():
        target = (PROBE_ROOT / name).resolve()
        if not target.is_relative_to(PROBE_ROOT.resolve()) or digest(target) != expected:
            raise ValueError(f'Source harness hash changed: {name}')
    return manifest


def _run(args: list[str], log: Path | None = None) -> str:
    result = subprocess.run(args, cwd=REPO_ROOT, check=False, capture_output=True, text=True)
    if log:
        log.write_text(result.stdout + result.stderr, encoding='utf-8')
    if result.returncode:
        raise ValueError(f'Source probe failed ({result.returncode}); log: {log}')
    return result.stdout


def _read_payload(directory: Path, seeds: list[int]) -> dict:
    try:
        native = json.loads((directory / 'native.json').read_text())
        managed = {}
        for kind, name in MANAGED_FILES.items():
            with (directory / name).open(newline='') as stream:
                managed[kind] = list(csv.DictReader(stream))
        assembled = [json.loads((directory / f'seed-{seed}/runtime.json').read_text()) for seed in seeds]
    except (OSError, ValueError) as exc:
        raise ValueError(f'Missing or invalid retained source artifact: {directory}') from exc
    return {'native': native, 'managed': managed, 'assembled': assembled}


def _sample(destination: Path, seeds: list[int]) -> dict:
    destination.mkdir(parents=True)
    native = _run([sys.executable, '-B', str(PROBE_ROOT / 'native_probe.py'), str(STEAM_ROOT)])
    (destination / 'native.json').write_text(native)
    _run(['pwsh', '-NoProfile', '-File', str(PROBE_ROOT / 'compile_managed.ps1')], destination / 'compile-managed.log')
    _run([sys.executable, '-B', str(PROBE_ROOT / 'mono_probe.py'), str(destination / 'placement.csv')], destination / 'managed.log')
    _run(['pwsh', '-NoProfile', '-File', str(PROBE_ROOT / 'run_runtime.ps1'), '-OutputRoot', str(destination), '-SeedList', ','.join(map(str, seeds))], destination / 'runtime.log')
    return _read_payload(destination, seeds)


def capture(reference_root: Path, output: Path, seeds: list[int]) -> dict:
    """ReferenceRoot is the project containing the accepted library and source archive."""
    if seeds != SEEDS:
        raise ValueError('Capture requires the five approved seeds in order')
    output = output.resolve()
    if not output.is_relative_to(PROBE_ROOT.resolve()):
        raise ValueError('Capture output must stay in this worktree ignored probe directory')
    reference_root = reference_root.resolve()
    native = native_engine_evidence(STEAM_ROOT)
    harness = checked_harness()
    source = reference_root / '.cache/reference-library/source'
    managed = source / 'TheLongDrive_Data/Managed/Assembly-CSharp.dll'
    if digest(managed) != MANAGED_SHA256:
        raise ValueError('Source managed hash changed')
    receipt = verify_library(reference_root / 'web/assets/library', source)
    if receipt.manifest_sha256 != LIBRARY_SHA256:
        raise ValueError('Accepted library identity changed')
    reader = LibraryReader(receipt)
    # This private harness binds its paths to the checked archive, not an arbitrary installation.
    expected_source = json.loads((reference_root / '.cache/reference-library/source-manifest.json').read_text())
    for entry in expected_source['files']:
        path = STEAM_ROOT / entry['file']
        if not path.is_file() or digest(path) != entry['sha256']:
            raise ValueError(f'Installed source input changed: {entry["file"]}')

    for identity in harness['records']:
        reader.record(ObjectId(**identity))
    output.mkdir(parents=True, exist_ok=True)
    run_root = output / ('run-' + uuid.uuid4().hex)
    first = _sample(run_root / 'first', seeds)
    second = _sample(run_root / 'replay', seeds)
    checked_harness()
    envelope = {'schema_version': 1, 'native_engine': native, 'managed_sha256': MANAGED_SHA256,
                'harness_sha256': HARNESS_SHA256, 'source_context': harness['context'],
                'library': {'manifest_sha256': receipt.manifest_sha256, 'catalog_sha256': receipt.catalog_sha256,
                            'source_manifest_sha256': receipt.source_manifest_sha256},
                'seeds': seeds, 'segments': SEGMENTS, **first,
                'payload_sha256': payload_digest(first), 'replay_sha256': payload_digest(second)}
    if envelope['payload_sha256'] != envelope['replay_sha256']:
        (run_root / 'mismatch.json').write_text(json.dumps(envelope, indent=2))
        raise ValueError(f'Source replay differs; retained both runs: {run_root}')
    # Do not publish a failed or partial capture over the previous accepted file.
    pending = run_root / 'samples.json'
    pending.write_text(json.dumps(envelope, sort_keys=True, separators=(',', ':'), allow_nan=False))
    verify(run_root)
    pending.replace(output / 'samples.json')
    (output / 'last-run.txt').write_text(str(run_root))
    return envelope


def _capture_root(output: Path) -> Path:
    pointer = output / 'last-run.txt'
    run = Path(pointer.read_text()).resolve() if pointer.is_file() else output.resolve()
    if not run.is_relative_to(PROBE_ROOT.resolve()):
        raise ValueError('Source capture log path leaves ignored probe storage')
    return run


def _verify_runtime_logs(output: Path) -> None:
    run = _capture_root(output)
    for group in ('first', 'replay'):
        for seed in SEEDS:
            path = run / group / f'seed-{seed}' / 'unity.log'
            if not path.is_file():
                raise ValueError('Missing source capture log')
            lines = path.read_text(errors='replace').splitlines()
            markers = [i for i, line in enumerate(lines) if line == 'WORLD_SOURCE_CAPTURE_FLUSHED error=none']
            if len(markers) != 1:
                raise ValueError('Missing successful source capture boundary')
            for i, line in enumerate(lines):
                headless_shader = (line.startswith('ERROR: Shader ') and line.endswith(
                    ' shader is not supported on this GPU (none of subshaders/fallbacks are suitable)'))
                if line.startswith('ERROR:') and not headless_shader:
                    raise ValueError(f'Source log error: {path}:{i + 1}')
                if 'Exception' not in line:
                    continue
                stack = '\n'.join(lines[i:i + 8])
                known_shutdown = (i > markers[0] and line.strip() == 'NullReferenceException'
                                  and 'S3DCamsHandlerScript.Upd' in stack
                                  and 'computerscript.OnDestroy' in stack)
                if not known_shutdown:
                    raise ValueError(f'Source exception outside checked shutdown: {path}:{i + 1}')


def verify(output: Path) -> None:
    envelope = json.loads((output / 'samples.json').read_text(encoding='utf-8-sig'))
    if envelope.get('native_engine', {}).get('sha256') != NATIVE_ENGINE_SHA256:
        raise ValueError('Source engine hash changed')
    native = envelope.get('native')
    if not isinstance(native, dict) or not native.get('native'):
        raise ValueError('Missing native sample rows')
    if envelope.get('schema_version') != 1 or envelope.get('managed_sha256') != MANAGED_SHA256:
        raise ValueError('Source schema or managed hash changed')
    if envelope.get('harness_sha256') != HARNESS_SHA256:
        raise ValueError('Source sampler hash changed')
    try:
        actual = payload_digest(envelope)
    except (KeyError, TypeError, ValueError) as exc:
        raise ValueError('Incomplete source payload') from exc
    if actual != envelope.get('payload_sha256'):
        raise ValueError('Source payload hash mismatch')
    if actual != envelope.get('replay_sha256'):
        raise ValueError('Source replay hash mismatch')
    if envelope.get('seeds') != SEEDS or envelope.get('segments') != SEGMENTS:
        raise ValueError('Missing approved source contexts')
    harness = checked_harness()
    if envelope.get('source_context') != harness['context'] or envelope.get('library', {}).get('manifest_sha256') != LIBRARY_SHA256:
        raise ValueError('Source context or library provenance changed')
    contract_path = PROBE_ROOT / 'input-contract.json'
    if not contract_path.is_file() or digest(contract_path) != INPUT_CONTRACT_SHA256:
        raise ValueError('Checked source input contract is missing or changed')
    contract = json.loads(contract_path.read_text())
    if envelope.get('library') != contract['library'] or envelope.get('native_engine') != contract['native_engine']:
        raise ValueError('Source provenance receipt is missing or changed')
    _validate_payload(envelope, harness, contract)
    run = _capture_root(output)
    for group in ('first', 'replay'):
        retained = _read_payload(run / group, SEEDS)
        _validate_payload(retained, harness, contract)
        if payload_digest(retained) != actual:
            raise ValueError(f'Retained {group} source payload differs from envelope')
    _verify_runtime_logs(output)


def _caller_inputs(row: dict) -> dict:
    # This subset binds source frames and caller arrays, including valid empty arrays.
    road_fields = ('segment', 'coords', 'angles', 'roadFrame', 'skinFrame', 'bones', 'skinBones',
                   'bindPoses', 'pole', 'sourceVertices', 'sourceNormals', 'uv', 'weights', 'triangles')
    tile_fields = ('center', 'resolution', 'tileSize', 'roadCoords', 'buildingCoords', 'buildingRanges', 'holeCoords')
    try:
        return {'startObjects': row['startObjects'], 'holes': row['holes'], 'foundations': row['foundations'],
                'numeric': row['numeric'], 'spawnInputs': row['spawnInputs'],
                'distances': [row[key] for key in ('distances', 'minDistances', 'maxDistances')],
                'roads': [{key: road[key] for key in road_fields} for road in row['roads']],
                'terrain': [{key: tile[key] for key in tile_fields} for tile in row['terrain']],
                'terrainDirect': [{key: tile[key] for key in tile_fields} for tile in row['terrainDirect']],
                'rebaseFrames': [{key: rebase[key] for key in ('segment', 'delta', 'roadOrigin', 'terrainOrigin')}
                                 for rebase in row['rebases']]}
    except (KeyError, TypeError) as exc:
        raise ValueError('Missing required source caller inputs') from exc


def _validate_payload(envelope: dict, harness: dict, contract: dict) -> None:
    native = envelope['native']
    if value_digest(native) != contract['native_sha256']:
        raise ValueError('Missing or changed checked native rows or inputs')
    for name, shape in contract['managed_shapes'].items():
        rows = envelope['managed'].get(name, [])
        if len(rows) != shape['rows'] or any(sorted(row) != shape['fields'] for row in rows):
            raise ValueError('Missing managed diagnostic rows or inputs')
    positions = {(row['x'], row['y']) for row in native['native']}
    if len(positions) < 7 or not any(x < 0 < y for x, y in positions) or not any(y < 0 < x for x, y in positions):
        raise ValueError('Missing full 2D native rows')
    if not native.get('ranges'):
        raise ValueError('Missing native instruction ranges')
    placement = envelope['managed'].get('placement_diagnostic', [])
    expected = {(seed, index) for seed in SEEDS for index in range(2000)}
    if len(placement) != 10000 or {(int(row['world_seed']), int(row['index'])) for row in placement} != expected:
        raise ValueError('Missing placement rows')
    if any(not row.get('draws') or not row.get('initial_state') for row in placement):
        raise ValueError('Missing placement random inputs')
    native_bits = {(struct.pack('<f', sample['x']), struct.pack('<f', sample['y'])): int(sample['bits'], 16)
                   for sample in native['native']}
    assembled = envelope.get('assembled', [])
    if len(assembled) != 5 or [row['seed'] for row in assembled] != SEEDS:
        raise ValueError('Missing assembled source seeds')
    for row in assembled:
        if row.get('error') or row.get('mathMode') != 1 or row.get('scene') != 'SceneNewRandom':
            raise ValueError('Invalid assembled source execution')
        runtime_native = row.get('native', [])
        if len(runtime_native) != len(native_bits):
            raise ValueError('Missing assembled native primitive rows')
        if {(struct.pack('<f', sample['x']), struct.pack('<f', sample['y'])) for sample in runtime_native} != set(native_bits):
            raise ValueError('Missing assembled native coordinates')
        for sample in runtime_native:
            position = (struct.pack('<f', sample['x']), struct.pack('<f', sample['y']))
            if position not in native_bits or abs(int(sample['bits'], 16) - native_bits[position]) > 4:
                raise ValueError('Native primitive differs from assembled source')
        roads = row.get('roads', [])
        if len(roads) != len(SEGMENTS) or [item['segment'] for item in roads] != SEGMENTS:
            raise ValueError('Missing assembled road rows')
        if value_digest(_caller_inputs(row)) != contract['caller_inputs_sha256'].get(str(row['seed'])):
            raise ValueError('Missing or changed checked source caller inputs')
        for road in roads:
            if any(len(road.get(key, [])) != size for key, size in [('bones', 10), ('skinBones', 11), ('bindPoses', 11), ('bakedVertices', 88), ('bakedNormals', 88), ('coords', 10), ('angles', 10), ('sourceVertices', 88), ('sourceNormals', 88), ('weights', 88), ('uv', 88)]):
                raise ValueError('Missing source geometry rows')
            if not road.get('pole') or not road.get('triangles'):
                raise ValueError('Missing source pole or topology')
        if len(row.get('terrain', [])) != 4 or len(row.get('biomes', [])) < 5:
            raise ValueError('Missing terrain border or biome rows')
        if row.get('terrainDirect') != row['terrain'] or row.get('terrainManagedCalls') != [1] * 4 or row.get('burstEnabled') is not False:
            raise ValueError('Scheduled terrain execution context changed')
        if row.get('placementInput') != harness['context']['placement_input']:
            raise ValueError('Source placement caller inputs changed')
        settings = harness['context']['settings_input']
        if any(row.get(key) != settings[key] for key in ('defaultDrawDistance', 'liveDrawDistance', 'menuDrawDistance', 'menuSettingsSaved')):
            raise ValueError('Source draw distance inputs changed')
        if row.get('distances', {}).get('drawDistance') != row['liveDrawDistance']:
            raise ValueError('Missing source distance interpolation')
        if len({(tile['center']['x'], tile['center']['y']) for tile in row['terrain']}) != 4:
            raise ValueError('Missing distinct terrain border tiles')
        for tile in row['terrain']:
            count = tile['resolution'] ** 2
            if len(tile['vertices']) != count or len(tile['normals']) != count or len(tile['colors']) != count:
                raise ValueError('Missing terrain vertices, normals, or colors')
            if len(tile.get('triangles', [])) != (tile['resolution'] - 1) ** 2 * 6:
                raise ValueError('Missing terrain topology')
        if {item['path'] for item in row.get('holes', [])} != {'haz02/Luk', 'kut/Luk (1)'}:
            raise ValueError('Missing garage source holes')
        if len(row.get('foundations', [])) < 2 or len(row.get('rebases', [])) != 3 or len(row.get('numeric', [])) < 4:
            raise ValueError('Missing source caller or numeric boundary rows')
        if not row.get('decisions') or not row.get('draws') or not row.get('callerOrder') or not row.get('systemStreams') or not row.get('spawnInputs'):
            raise ValueError('Missing source random caller rows')
        if not any('Spawn enter Fridge(Clone)/' in call for call in row['callerOrder']):
            raise ValueError('Missing recursive source spawn rows')
        candidates = row.get('placement', [])
        streams = [stream for stream in row['systemStreams'] if 'GenerationJob::selectRandom(' in stream['caller']]
        if len(candidates) != 2000 or [item['index'] for item in candidates] != list(range(2000)) or len(streams) != 2000:
            raise ValueError('Missing assembled placement rows or random streams')
        for candidate, stream in zip(candidates, streams):
            if candidate['positionSeed'] != stream['seed'] or len(stream['initial']) != 56:
                raise ValueError('Placement random state does not match source candidate')
            expected_draws = 1 if candidate['type'] == -1 else 3
            if len(stream['draws']) != expected_draws:
                raise ValueError('Placement branch draw count changed')
        for key in ('initialRandomState', 'afterStartupRandomState', 'finalRandomState'):
            state = json.loads(row.get(key, '{}'))
            if set(state) != {'s0', 's1', 's2', 's3'}:
                raise ValueError('Missing checked Unity random state')
            if key == 'initialRandomState' and [state[f's{i}'] for i in range(4)] != harness['context']['shared_random_input']['state']:
                raise ValueError('Shared Unity random input changed')



def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--reference-root', type=Path)
    parser.add_argument('--output-root', type=Path, default=PROBE_ROOT)
    parser.add_argument('--verify', action='store_true')
    args = parser.parse_args()
    if args.verify:
        verify(args.output_root)
        print('Source world samples verified')
    else:
        if args.reference_root is None:
            parser.error('--reference-root is required for capture')
        result = capture(args.reference_root, args.output_root, SEEDS)
        print('Source world replay verified: ' + result['payload_sha256'])


if __name__ == '__main__':
    main()
