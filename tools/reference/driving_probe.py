"""Checked source-only G11 driving captures. Raw runs stay in ignored storage."""
import argparse
import json
import math
import os
import shutil
import subprocess
from pathlib import Path

from .driving import ASSEMBLY_SHA256, DECOMPILED_SHA256
from .driving_driver import validate_driver
from .driving_lifecycle import validate_lifecycle
from .driving_geometry import source_geometry_map, validate_geometry, physical_inputs
from .export import native_engine_evidence
from .library.archive import load_snapshot, verify_snapshot, sha256
from .library.types import APP_ID, BUILD_ID, UNITY_VERSION, LibraryReceipt
from .library.reader import LibraryReader, checked

PROJECT = Path(__file__).resolve().parents[2]
FILES = ('driving_probe.py', 'driving_capture.cs', 'driving_driver.cs', 'driving_driver.py', 'driving_lifecycle.cs', 'driving_lifecycle.py', 'driving_geometry.cs', 'driving_geometry.py', 'driving_capture.ps1')
PHASES = {'input.entry', 'input.exit', 'car.entry', 'car.exit', 'engine.entry',
          'engine.exit', 'control.entry', 'control.exit', 'rpm.entry', 'rpm.exit', 'physics.entry', 'physics.exit'}


def read_json(path):
    try:
        return json.loads(path.read_text(encoding='utf-8-sig'))
    except (OSError, ValueError) as error:
        raise ValueError(f'Missing or invalid evidence: {path}') from error


def write_json(path, data):
    path.write_text(json.dumps(data, indent=2, allow_nan=False) + '\n', encoding='utf-8')


def observer_revision():
    return {name: sha256(Path(__file__).parent / name) for name in FILES}


def preflight(source_root: Path, reference_root: Path) -> dict:
    source = source_root.resolve()
    reference = reference_root.resolve()
    snapshot = load_snapshot(reference / '.cache/reference-library')
    if snapshot.root.resolve() != source:
        raise ValueError('Source root differs from the accepted archive')
    verify_snapshot(snapshot)
    native = native_engine_evidence(source)
    records = snapshot.manifest_path.parent / 'records'
    library = reference / 'web/assets/library/manifest.json'
    manifest = read_json(library)
    if manifest['source_manifest_sha256'] != snapshot.manifest_sha256:
        raise ValueError('Accepted library source manifest differs')
    reader = LibraryReader(LibraryReceipt(library, sha256(library),
        manifest['catalog_sha256'], manifest['source_manifest_sha256'], ()))
    managed = read_json(records / 'managed/manifest.json')
    record = next(r for r in managed['files'] if r['file'] == 'managed/Assembly-CSharp.cs')
    if record['sha256'] != DECOMPILED_SHA256:
        raise ValueError('Checked managed source receipt differs')
    checked(records, record)
    assembly = source / 'TheLongDrive_Data/Managed/Assembly-CSharp.dll'
    if sha256(assembly) != ASSEMBLY_SHA256:
        raise ValueError('Checked source assembly differs')
    required = ['TheLongDrive.exe', 'MonoBleedingEdge/EmbedRuntime/mono-2.0-bdwgc.dll',
        'TheLongDrive_Data/Managed/Unity.Burst.Cecil.dll',
        'TheLongDrive_Data/Managed/UnityEngine.PhysicsModule.dll',
        'TheLongDrive_Data/Managed/UnityEngine.CoreModule.dll',
        'TheLongDrive_Data/ScriptingAssemblies.json']
    receipts = {r.file: r for r in snapshot.files}
    for name in required:
        if name not in receipts or not (source / name).is_file():
            raise ValueError(f'Missing checked native capture dependency: {name}')
    powershell = shutil.which('pwsh')
    if not powershell:
        raise ValueError('PowerShell 7 with Roslyn is required')
    roslyn = [Path(powershell).parent / name for name in
              ('Microsoft.CodeAnalysis.dll', 'Microsoft.CodeAnalysis.CSharp.dll')]
    if any(not path.is_file() for path in roslyn):
        raise ValueError('Missing PowerShell Roslyn compiler')
    return {'schema_version': 1, 'app_id': APP_ID, 'build_id': BUILD_ID,
        'unity_version': UNITY_VERSION, 'source_root': str(source),
        'source_manifest_sha256': snapshot.manifest_sha256,
        'steam_receipt_sha256': sha256(snapshot.manifest_path.parent / 'steam-manifest.acf'),
        'archive_file_count': len(snapshot.files), 'archive_bytes': sum(r.bytes for r in snapshot.files),
        'library_manifest_sha256': reader.receipt.manifest_sha256,
        'managed_source_sha256': DECOMPILED_SHA256, 'assembly_sha256': ASSEMBLY_SHA256,
        'native_engine': native, 'dependencies': {name: receipts[name].sha256 for name in required},
        'roslyn': {str(path): sha256(path) for path in roslyn}, 'powershell': powershell,
        'observer_revision': observer_revision(), 'source_geometry_map': source_geometry_map(reader)}


def finite(value):
    if isinstance(value, float) and not math.isfinite(value):
        raise ValueError('Nonfinite native sample')
    if isinstance(value, dict):
        for item in value.values(): finite(item)
    if isinstance(value, list):
        for item in value: finite(item)


def control_frames(scenario):
    duration = scenario.get('duration_frames')
    controls = scenario.get('controls')
    if type(duration) is not int or duration < 2 or not isinstance(controls, list) or not controls:
        raise ValueError('Declared finite duration and control schedule are required')
    frames = [control.get('frame') for control in controls]
    if (any(type(frame) is not int or not 0 <= frame < duration for frame in frames) or
        frames[0] != 0 or frames != sorted(set(frames))):
        raise ValueError('Declared control schedule must increase from zero inside the finite interval')
    return duration, frames


def validate_timing(samples, scenario):
    duration, controls = control_frames(scenario)
    frames = [sample['frame'] for sample in samples]
    if (any(type(frame) is not int or not 0 <= frame <= duration for frame in frames) or
        frames[0] != 0 or frames[-1] != duration):
        raise ValueError('Native samples do not cover the declared observation interval')
    if frames != sorted(frames) or set(frames) != set(range(duration + 1)):
        raise ValueError('Native render frame coverage is incomplete or out of order')
    links = {'input_sequence': 'input.entry', 'car_sequence': 'car.entry',
             'engine_sequence': 'engine.entry', 'rpm_sequence': 'rpm.exit'}
    latest = dict.fromkeys(links, 0)
    opened, completed = {}, set()
    step = 0
    for sample in samples:
        phase = sample['phase']
        if phase not in PHASES | {'autoshift.entry', 'autoshift.exit'}:
            raise ValueError(f'Unknown native callback phase: {phase}')
        callback, boundary = phase.split('.')
        if phase == 'physics.entry': step += 1
        if type(sample['step']) is not int or sample['step'] != step:
            raise ValueError('Native physics step order is invalid')
        if boundary == 'entry':
            if callback in opened:
                raise ValueError(f'Native callback pair has a missing exit: {callback}')
            opened[callback] = sample
        else:
            entry = opened.pop(callback, None)
            if entry is None or (entry['frame'], entry['step']) != (sample['frame'], sample['step']):
                raise ValueError(f'Native callback pair is missing or crosses a frame/step: {callback}')
            completed.update((entry['sequence'], sample['sequence']))
        # The observer sets entry revisions before their sample, but RPM at its exit.
        # Validate the observed order, without imposing a car/engine render order.
        for link, target in links.items():
            if phase == target: latest[link] = sample['sequence']
            revision = sample[link]
            if (type(revision) is not int or not 0 <= revision <= sample['sequence'] or
                (revision and samples[revision - 1]['phase'] != target) or revision != latest[link]):
                raise ValueError(f'Invalid native callback link: {link} at sample {sample["sequence"]}')
            if callback == 'physics':
                # Zero is only available before every producer's first observed callback.
                # Retain that native startup prefix; it cannot prove control consumption.
                if not revision and any(latest.values()):
                    raise ValueError(f'Missing native consuming callback link: {link}')
                if revision and revision not in completed:
                    raise ValueError(f'Native callback pair is incomplete before physics: {target}')
    if opened:
        raise ValueError(f'Native callback pair has no exit: {sorted(opened)}')
    callbacks = {s['phase'].split('.')[0] for s in samples} - {'physics'}
    for callback in callbacks:
        if [s['frame'] for s in samples if s['phase'] == callback + '.exit'] != list(range(duration + 1)):
            raise ValueError(f'Native callback pair frame coverage is incomplete: {callback}')
    physics = [s for s in samples if s['phase'] == 'physics.entry']
    for start, end in zip(controls, controls[1:] + [duration]):
        if not any(s['input_sequence'] and
                   start <= samples[s['input_sequence'] - 1]['frame'] < end for s in physics):
            raise ValueError(f'No native physics step consumed declared control interval [{start}, {end})')
    return {'initial_unlinked_physics_steps': [s['step'] for s in physics if not s['input_sequence']]}


def validate_run(run, scenario):
    finite(run)
    if run.get('runtime') != 'original-unity-player':
        raise ValueError('Original native Unity player evidence is required')
    if (run.get('scenario'), run.get('probe_kind')) != (scenario['id'], scenario['probe_kind']):
        raise ValueError('Native scenario identity differs')
    if not run.get('completed') or run.get('error'):
        raise ValueError('Native completion marker is missing or failed')
    if not all(run.get(key) for key in ('menu_ready', 'lada_ready', 'control_capture_active')):
        raise ValueError('Original menu, selected Lada, and source controls must initialize')
    if not run.get('initial'):
        raise ValueError('Native initial state is missing')
    samples = run.get('samples', [])
    if len(samples) < 2 or run.get('physics_steps', 0) < 2:
        raise ValueError('Native physics samples are missing')
    phases = {s['phase'] for s in samples}
    required = PHASES if scenario['probe_kind'] == 'assembled_lada' else {'physics.entry', 'physics.exit'}
    if not required <= phases:
        raise ValueError(f'Missing native callback phases: {sorted(required - phases)}')
    if scenario['probe_kind'] == 'curve' and not run.get('curves'):
        raise ValueError('Native Unity curve observations are missing')
    if scenario['probe_kind'] == 'wheel_collider' and not run.get('isolated_wheel_probe'):
        raise ValueError('Isolated native WheelCollider observations are missing')
    sequence = [s['sequence'] for s in samples]
    if sequence != list(range(1, len(samples) + 1)):
        raise ValueError('Native callback sequence is invalid')
    exits = [s for s in samples if s['phase'] == 'physics.exit']
    entries = [s for s in samples if s['phase'] == 'physics.entry']
    if [s['step'] for s in entries] != [s['step'] for s in exits]:
        raise ValueError('Native physics entry and exit samples differ')
    if len(exits) != run['physics_steps'] or len({s['step'] for s in exits}) != len(exits):
        raise ValueError('Native physics samples are incomplete')
    required_sample = {'phase', 'sequence', 'step', 'frame', 'time', 'fixed_dt', 'rpm',
        'gear', 'clutch', 'position', 'rotation', 'velocity', 'angular_velocity',
        'input_sequence', 'engine_sequence', 'car_sequence', 'rpm_sequence',
        'engine_output_nm', 'engine_brake_nm', 'average_rpm', 'torque_divisor'}
    required_wheel = {'contact', 'rpm', 'motor_torque', 'brake_torque', 'radius',
        'suspension_distance', 'suspension_offset_m', 'pose', 'rotation', 'contact_point',
        'contact_normal', 'force_n', 'forward_slip', 'sideways_slip'}
    for sample in samples:
        if not required_sample <= sample.keys():
            raise ValueError('Missing native sample metrics')
        for wheel in sample.get('wheels', []):
            if not required_wheel <= wheel.keys():
                raise ValueError('Missing native wheel metrics')
        if sample.get('fixed_dt', 0) <= 0:
            raise ValueError('Native fixed timestep is missing')
        if sample['phase'] == 'physics.exit' and len(sample.get('wheels', [])) != 4:
            raise ValueError('Four native wheel samples are required')
    timing = validate_timing(samples, scenario)
    return {'samples': len(samples), 'physics_steps': run['physics_steps'],
            'phases': sorted(phases), **timing}


def validate_evidence(output: Path, directory: Path, run: dict, scenario: dict) -> dict:
    result = validate_run(run, scenario)
    result['geometry'] = validate_geometry(run)
    result['driver'] = validate_driver(run)
    result['lifecycle'] = validate_lifecycle(run)
    marker = read_json(directory / 'completion.json')
    process = read_json(directory / 'process.json')
    exit_status = read_json(directory / 'exit.json')
    if (exit_status.get('exit_code') != 0 or process.get('pid', 0) <= 0 or
        Path(process.get('executable', '')).resolve() != output / 'scratch/game/TheLongDrive.exe'):
        raise ValueError('Native player process evidence failed')
    try:
        log = (directory / 'unity.log').read_text(encoding='utf-8-sig')
    except OSError as error:
        raise ValueError('Native Unity log is missing') from error
    flush = 'G11_NATIVE_CAPTURE_FLUSHED completed=True'
    if flush not in log:
        raise ValueError('Native completed-sample flush marker is missing')
    before, after = log.split(flush, 1)
    if 'Exception' in before:
        raise ValueError('Native runtime exception occurred before capture flush')
    concerns = []
    lines = after.splitlines()
    for index, line in enumerate(lines):
        if 'Exception' not in line: continue
        stack = lines[index:index + 5]
        exact = len(stack) == 5 and stack[0].strip() == 'NullReferenceException'
        expected = ('UnityEngine.Camera.CopyFrom(', 'S3DCamsHandlerScript.Upd (',
                    'S3DCamsHandlerScript.Upd ()', 'computerscript.OnDestroy ()')
        exact = exact and all(name in frame for name, frame in zip(expected, stack[1:]))
        if not exact:
            raise ValueError('Unclassified native exception occurred after capture flush')
        concerns.append('Camera.CopyFrom -> S3DCamsHandlerScript.Upd -> computerscript.OnDestroy after sample flush')
    result['post_capture_concerns'] = concerns
    result['runtime_log_sha256'] = sha256(directory / 'unity.log')
    if marker != {'completed': True, 'sample_count': len(run['samples']),
                  'physics_steps': run['physics_steps']}:
        raise ValueError('Native completion marker differs from samples')
    return result


def verify(output_root: Path) -> dict:
    output = output_root.resolve()
    request = read_json(output / 'request.json')
    if request.get('repeats') != 3:
        raise ValueError('Three fresh native repeats are required')
    results = []
    for scenario in request['scenarios']:
        initial = None
        for repeat in range(1, 4):
            directory = output / scenario['id'] / f'repeat-{repeat}'
            path = directory / 'runtime.json'
            run = read_json(path)
            result = validate_evidence(output, directory, run, scenario)
            state = {key: value for key, value in run['initial'].items()
                     if key != 'profile_path'}
            state['physical_geometry'] = physical_inputs(next(row for row in run['geometry'] if row['phase'] == 'armed-after-canonical'))
            if initial is not None and state != initial:
                raise ValueError('Fresh native repeats have different initial states')
            initial = state
            result.update(scenario=scenario['id'], repeat=repeat, sha256=sha256(path))
            results.append(result)
    setup = read_json(output / 'scratch/patch-receipt.json')
    if not setup.get('changed_methods') or setup.get('observer_revision') != request.get('observer_revision'):
        raise ValueError('Checked observer patch receipt differs')
    pre = read_json(output / 'preflight.json')
    if (pre.get('app_id'), pre.get('build_id'), pre.get('unity_version')) != (APP_ID, BUILD_ID, UNITY_VERSION):
        raise ValueError('Native build provenance differs')
    if (setup.get('native_engine_sha256') != pre.get('native_engine', {}).get('sha256') or
        setup.get('source_physics_module_sha256') != pre.get('dependencies', {}).get(
            'TheLongDrive_Data/Managed/UnityEngine.PhysicsModule.dll')):
        raise ValueError('Patched capture changed native physics dependencies')
    receipt = {'schema_version': 1, 'classification': 'completed native driving capture',
        'preflight': pre, 'request_sha256': sha256(output / 'request.json'),
        'patch_receipt_sha256': sha256(output / 'scratch/patch-receipt.json'), 'runs': results}
    accepted = output / 'receipt.json'
    if accepted.exists() and read_json(accepted) != receipt:
        raise ValueError('Completed capture receipt changed')
    return receipt


def capture(source_root: Path, reference_root: Path,
            output_root: Path, scenarios: Path, repeats: int) -> dict:
    if repeats != 3:
        raise ValueError('Three fresh native repeats are required')
    output = output_root.resolve()
    if not output.is_relative_to(PROJECT / '.cache/g11-driving') or output.exists():
        raise ValueError('Use a new output directory inside this G11 worktree .cache/g11-driving')
    pre = preflight(source_root, reference_root)
    definitions = read_json(scenarios)
    rows = definitions['scenarios']
    if not rows or len({s['id'] for s in rows}) != len(rows):
        raise ValueError('Scenario identities must be unique')
    for row in rows:
        if (not row['id'].replace('-', '').isalnum() or row['probe_kind'] not in
            ('assembled_lada', 'wheel_collider', 'curve') or row['duration_frames'] < 2):
            raise ValueError('Invalid native scenario')
        for key in ('source_objects', 'setup', 'frame_schedule', 'controls'):
            if key not in row: raise ValueError(f'Missing scenario field: {key}')
        control_frames(row)
    output.mkdir(parents=True)
    write_json(output / 'preflight.json', {k: v for k, v in pre.items() if k != 'source_geometry_map'})
    write_json(output / 'source-geometry-map.json', pre.get('source_geometry_map', {}))
    write_json(output / 'request.json', {'repeats': repeats, 'scenarios': rows,
                                       'observer_revision': pre['observer_revision']})
    script = Path(__file__).with_name('driving_capture.ps1')
    common = [pre['powershell'], '-NoProfile', '-File', str(script),
        '-SourceRoot', str(source_root.resolve()), '-ScratchRoot', str(output / 'scratch'),
        '-ReferenceRoot', str(reference_root.resolve())]
    def invoke(arguments, log, timeout):
        with log.open('w', encoding='utf-8') as stream:
            process = subprocess.run(common + arguments, stdout=stream, stderr=subprocess.STDOUT,
                                     timeout=timeout, cwd=PROJECT, check=False)
        if process.returncode:
            raise ValueError(f'Native capture failed; evidence retained: {log}')
    invoke(['-Mode', 'Setup', '-RequestPath', str(output / 'request.json')], output / 'setup.log', 120)
    for row in rows:
        setup_state = None
        initial = None
        for repeat in range(1, 4):
            directory = output / row['id'] / f'repeat-{repeat}'
            directory.mkdir(parents=True)
            run_scenario = dict(row)
            if setup_state is not None:
                run_scenario['setup_state'] = setup_state
                run_scenario['has_setup_state'] = True
            write_json(directory / 'scenario.json', run_scenario)
            invoke(['-Mode', 'Run', '-RunRoot', str(directory), '-ProfileRoot', str(directory / 'profile')],
                   directory / 'launcher.log', 200)
            native_run = read_json(directory / 'runtime.json')
            validate_evidence(output, directory, native_run, row)
            state = {key: value for key, value in native_run['initial'].items()
                     if key != 'profile_path'}
            state['physical_geometry'] = physical_inputs(next(row for row in native_run['geometry'] if row['phase'] == 'armed-after-canonical'))
            if initial is not None and state != initial:
                raise ValueError('Fresh native repeats have different initial states')
            initial = state
            if setup_state is None: setup_state = native_run['setup_state']
    receipt = verify(output)
    write_json(output / 'receipt.json', receipt)
    return receipt


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest='command', required=True)
    for command in ('preflight', 'capture', 'verify'):
        sub = commands.add_parser(command)
        if command != 'verify':
            sub.add_argument('--source-root', required=True, type=Path)
            sub.add_argument('--reference-root', required=True, type=Path)
        if command != 'preflight': sub.add_argument('--output-root', required=True, type=Path)
        if command == 'capture':
            sub.add_argument('--scenarios', required=True, type=Path)
            sub.add_argument('--repeats', required=True, type=int)
    args = vars(parser.parse_args())
    command = args.pop('command')
    print(json.dumps(globals()[command](**args), indent=2, allow_nan=False))


if __name__ == '__main__':
    main()
