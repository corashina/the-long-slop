"""Checked source-only G11 driving captures. Raw runs stay in ignored storage."""
import argparse
from decimal import Decimal
import json
import math
import os
import shutil
import subprocess
from pathlib import Path

from .driving import ASSEMBLY_SHA256, DECOMPILED_SHA256
from .driving_body_hold import validate_body_hold
from .driving_curve import validate_curve_scenario, validate_curve_run, require_same_curve
from .driving_angular import validate_angular_schedule, validate_angular, angular_initial_inputs, validate_angular_body_input, angular_transport_initial, validate_angular_supplied
from .driving_contact import validate_contact_schedule, validate_contact, contact_initial_inputs, validate_contact_body_input, validate_contact_supplied, contact_exact, contact_pre_inputs
from .driving_driver import validate_driver
from .driving_controls import validate_schedule, validate_controls
from .driving_lifecycle import validate_lifecycle
from .driving_geometry import source_geometry_map, validate_geometry, physical_inputs
from .export import native_engine_evidence
from .library.archive import load_snapshot, verify_snapshot, sha256
from .library.types import APP_ID, BUILD_ID, UNITY_VERSION, LibraryReceipt
from .library.reader import LibraryReader, checked

PROJECT = Path(__file__).resolve().parents[2]
FILES = ('driving_contact.cs', 'driving_contact.py', 'driving_curve.py', 'driving_angular.cs', 'driving_angular.py', 'driving_body_hold.py', 'driving_cargo.cs', 'driving_body_trace.cs', 'driving_controls.cs', 'driving_controls.py', 'driving_probe.py', 'driving_capture.cs', 'driving_driver.cs', 'driving_driver.py', 'driving_lifecycle.cs', 'driving_lifecycle.py', 'driving_geometry.cs', 'driving_geometry.py', 'driving_capture.ps1')
PHASES = {'input2.entry', 'input2.exit', 'actuation.entry', 'actuation.exit', 'torque.entry', 'torque.exit', 'input.entry', 'input.exit', 'car.entry', 'car.exit', 'engine.entry',
          'engine.exit', 'control.entry', 'control.exit', 'rpm.entry', 'rpm.exit', 'physics.entry', 'physics.exit'}


def read_json(path):
    try:
        text = path.read_text(encoding='utf-8-sig')
        data = json.loads(text)
        if isinstance(data, dict) and data.get('probe_kind') == 'curve':
            data['curves'] = json.loads(text, parse_float=Decimal).get('curves')
        return data
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
    if isinstance(value, (float, Decimal)) and not math.isfinite(value):
        raise ValueError('Nonfinite native sample')
    if isinstance(value, dict):
        for item in value.values(): finite(item)
    if isinstance(value, list):
        for item in value: finite(item)


def sustained_control_frames(scenario):
    duration, _ = validate_schedule(scenario)
    frames, previous = [], None
    for control in scenario['controls']:
        state = {key: value for key, value in control.items()
                 if key not in ('frame', 'handbrake_press', 'gear_press')}
        if state != previous:
            frames.append(control['frame'])
        previous = state
    return duration, frames


def validate_timing(samples, scenario):
    duration, controls = sustained_control_frames(scenario)
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


TRANSITION_ACTIONS = {
    'refresh_tire_condition', 'remove_tire', 'remove_cover', 'remove_driven_rim',
    'remove_other_cover', 'remove_other_driven_rim', 'remount_driven_rim',
    'remount_other_driven_rim', 'remount_tire', 'remount_cover', 'remount_other_cover',
}


def validate_transition_schedule(scenario):
    """Validate an explicit source transition sequence before native setup."""
    events = scenario.get('transitions')
    if scenario.get('probe_kind') != 'assembled_lada' or not isinstance(events, list) or not events:
        raise ValueError('Source transitions require an assembled-Lada event list')
    duration, _ = validate_schedule(scenario)
    ids, last_frame, last_observation_frame = set(), -1, -1
    for event in events:
        if not isinstance(event, dict):
            raise ValueError('Source transition event must be an object')
        event_id, action, frame = event.get('id'), event.get('action'), event.get('frame')
        if (not isinstance(event_id, str) or not event_id.replace('-', '').isalnum() or event_id in ids or
            action not in TRANSITION_ACTIONS or type(frame) is not int or frame <= last_frame or
            frame <= last_observation_frame or frame >= duration):
            raise ValueError('Source transition identity, action, or frame is invalid')
        if event.get('target') not in ('primary-driven', 'other-driven'):
            raise ValueError('Source transition target must name a driven slot')
        if event.get('source_context') != 'direct-component-api':
            raise ValueError('Source transition must declare direct component API semantics')
        if type(event.get('observe_frames')) is not int or event['observe_frames'] < 1:
            raise ValueError('Source transition must declare a later observation window')
        if frame + event['observe_frames'] >= duration:
            raise ValueError('Source transition later observation exceeds the run')
        if action == 'refresh_tire_condition' and (type(event.get('condition_state')) is not int or
                not 0 <= event['condition_state'] <= 4):
            raise ValueError('Tire condition state must be a source state from zero to four')
        ids.add(event_id)
        last_frame = frame
        last_observation_frame = frame + event['observe_frames']
    return events


def _transition_number(value, name):
    if type(value) not in (int, float) or not math.isfinite(value):
        raise ValueError('Source transition channel must be finite numeric data: ' + name)


def _transition_snapshot(snapshot):
    if not isinstance(snapshot, dict):
        raise ValueError('Source transition snapshot is missing')
    for name in ('frame', 'physics_step', 'driven_wheels_num', 'installed_driven_rims',
                 'source_lock_events', 'wheel_activation_events'):
        if type(snapshot.get(name)) is not int or snapshot[name] < 0:
            raise ValueError('Source transition snapshot count is invalid: ' + name)
    if not isinstance(snapshot.get('phase'), str) or not snapshot['phase']:
        raise ValueError('Source transition snapshot phase is missing')
    slots = snapshot.get('slots')
    if not isinstance(slots, list) or len(slots) != 4:
        raise ValueError('Source transition snapshot must include all four wheel slots')
    slot_keys = [slot.get('slot_key') for slot in slots if isinstance(slot, dict)]
    if len(slot_keys) != 4 or any(not isinstance(key, str) or not key for key in slot_keys) or len(set(slot_keys)) != 4:
        raise ValueError('Source transition snapshot has missing or duplicate wheel slots')
    for slot in slots:
        for name in ('retained_rim_key', 'retained_tire_key', 'retained_cover_key'):
            if not isinstance(slot.get(name), str) or not slot[name]:
                raise ValueError('Source transition slot identity is missing: ' + name)
        for name in ('rim_key', 'tire_key', 'cover_key'):
            if slot.get(name) is not None and not isinstance(slot[name], str):
                raise ValueError('Source transition current slot key is invalid: ' + name)
        for name in ('rim_instance_id', 'tire_instance_id', 'cover_instance_id'):
            if type(slot.get(name)) is not int or slot[name] <= 0:
                raise ValueError('Source transition part identity is invalid: ' + name)
        if type(slot.get('condition_state')) is not int or slot['condition_state'] < -1:
            raise ValueError('Source transition condition state is invalid')
        if type(slot.get('native_wheel_index')) is not int or slot['native_wheel_index'] < -1:
            raise ValueError('Source transition native wheel binding is invalid')
        for name in ('driven', 'condition_available', 'rim_data_available', 'rim_present', 'tire_present',
                     'cover_present', 'rim_active', 'tire_active', 'cover_active'):
            if type(slot.get(name)) is not bool:
                raise ValueError('Source transition slot flag is invalid: ' + name)
        if not slot['condition_available'] or not slot['rim_data_available']:
            raise ValueError('Source transition condition cache or rim data is unavailable')
        for name in ('tire_slip1', 'tire_slip2', 'rim_felni_radius', 'rim_data_radius',
                     'rim_data_slip1', 'rim_data_slip2', 'rim_data_mass'):
            _transition_number(slot.get(name), name)
        if slot['rim_present'] != bool(slot['rim_key']) or slot['tire_present'] != bool(slot['tire_key']) or slot['cover_present'] != bool(slot['cover_key']):
            raise ValueError('Source transition current part key differs from slot presence')
    driven_slots = [slot for slot in slots if slot['driven']]
    if len(driven_slots) != 2:
        raise ValueError('Source transition snapshot must include both driven slots')
    if (any(slot['native_wheel_index'] < 0 for slot in driven_slots) or
            any(slot['native_wheel_index'] != -1 for slot in slots if not slot['driven']) or
            len({slot['native_wheel_index'] for slot in driven_slots}) != 2):
        raise ValueError('Source transition driven wheel bindings are missing or duplicated')
    installed = sum(slot['rim_present'] for slot in driven_slots)
    if snapshot['installed_driven_rims'] != installed or snapshot['driven_wheels_num'] != max(installed, 1):
        raise ValueError('Source driven-rim count or clamped divisor differs from observed slots')
    native = snapshot.get('native_wheels')
    if not isinstance(native, list) or len(native) < 4:
        raise ValueError('Source transition native wheel channels are missing')
    for index, wheel in enumerate(native):
        if not isinstance(wheel, dict) or not isinstance(wheel.get('key'), str) or not wheel['key']:
            raise ValueError('Source transition native wheel identity is missing')
        for name in ('index', 'instance_id', 'body_instance_id'):
            minimum = 0 if name in ('index', 'body_instance_id') else 1
            if type(wheel.get(name)) is not int or wheel[name] < minimum:
                raise ValueError('Source transition native wheel identity is invalid: ' + name)
        if wheel['index'] != index:
            raise ValueError('Source transition native wheel order changed')
        if wheel.get('body_key') is not None and not isinstance(wheel['body_key'], str):
            raise ValueError('Source transition native body key is invalid')
        for name in ('enabled', 'active_self', 'active_hierarchy', 'contact'):
            if type(wheel.get(name)) is not bool:
                raise ValueError('Source transition native wheel flag is invalid: ' + name)
        for name in ('rpm', 'radius', 'mass', 'damping', 'forward_stiffness', 'sideways_stiffness'):
            _transition_number(wheel.get(name), name)
        for name, components in (('pose', ('x', 'y', 'z')), ('rotation', ('x', 'y', 'z', 'w'))):
            vector = wheel.get(name)
            if not isinstance(vector, dict):
                raise ValueError('Source transition native wheel pose is missing: ' + name)
            for component in components:
                _transition_number(vector.get(component), name + '.' + component)
    if (len({wheel['key'] for wheel in native}) != len(native) or
            len({wheel['instance_id'] for wheel in native}) != len(native) or
            any(slot['native_wheel_index'] >= len(native) for slot in driven_slots)):
        raise ValueError('Source transition native wheel bindings are not unique')
def validate_transition_evidence(run, scenario):
    expected = validate_transition_schedule(scenario)
    observed = run.get('transitions')
    if not isinstance(observed, list) or len(observed) != len(expected):
        raise ValueError('Native source transition evidence is incomplete')
    for index, (declaration, event) in enumerate(zip(expected, observed)):
        methods = {
            'refresh_tire_condition': 'partconditionscript.Refresh(int)',
            'remove_tire': 'partslotscript.UnCraft(false)',
            'remove_cover': 'partslotscript.UnCraft(false)',
            'remove_other_cover': 'partslotscript.UnCraft(false)',
            'remove_driven_rim': 'partslotscript.UnCraft(false)',
            'remove_other_driven_rim': 'partslotscript.UnCraft(false)',
            'remount_driven_rim': 'partslotscript.CanCraft(partscript)+Craft(partscript)',
            'remount_other_driven_rim': 'partslotscript.CanCraft(partscript)+Craft(partscript)',
            'remount_tire': 'partslotscript.CanCraft(partscript)+Craft(partscript)',
            'remount_cover': 'partslotscript.CanCraft(partscript)+Craft(partscript)',
            'remount_other_cover': 'partslotscript.CanCraft(partscript)+Craft(partscript)',
        }
        if (event.get('id'), event.get('action'), event.get('target'), event.get('scheduled_frame'),
                event.get('source_context'), event.get('source_method')) != (
            declaration['id'], declaration['action'], declaration['target'], declaration['frame'],
            declaration['source_context'], methods[declaration['action']]):
            raise ValueError('Native source transition identity or method differs')
        if event.get('api_success') is not True:
            raise ValueError('Original source transition API failed')
        if (type(event.get('before_physics_step')) is not int or type(event.get('later_physics_step')) is not int or
                event['later_physics_step'] <= event['before_physics_step']):
            raise ValueError('Source transition lacks a later physics sample')
        _transition_number(event.get('rpm_before'), 'rpm_before')
        if event['rpm_before'] == 0:
            raise ValueError('Source transition requires nonzero pre-event wheel RPM')
        if (type(event.get('rpm_wheel_index')) is not int or event['rpm_wheel_index'] < 0 or
                event.get('target_slot_key') is None or not event.get('api_result')):
            raise ValueError('Source transition lacks target slot or immediate API result')
        if declaration['action'].startswith('remount_') and event.get('can_craft') is not True:
            raise ValueError('Source remount was not allowed by CanCraft')
        snapshots = [event.get(key) for key in ('before', 'immediate', 'later')]
        for snapshot in snapshots:
            _transition_snapshot(snapshot)
        if snapshots[0]['frame'] != declaration['frame'] or snapshots[1]['frame'] != declaration['frame']:
            raise ValueError('Source transition before and immediate frames differ from the declaration')
        expected_later_frame = declaration['frame'] + declaration['observe_frames']
        if (event.get('later_after_frame') != expected_later_frame or
                snapshots[2]['frame'] < expected_later_frame or snapshots[2]['frame'] <= snapshots[1]['frame']):
            raise ValueError('Source transition later frame misses its declared observation window')
        if (snapshots[0]['physics_step'] != event['before_physics_step'] or
                snapshots[1]['physics_step'] != event['before_physics_step'] or
                snapshots[2]['physics_step'] != event['later_physics_step'] or
                event['later_physics_step'] <= event['before_physics_step']):
            raise ValueError('Source transition later physics step is missing or stale')
        if event.get('api_result') != ('void-return' if declaration['action'] == 'refresh_tire_condition' else 'True'):
            raise ValueError('Source transition immediate API outcome differs')
        before_slots = {slot['slot_key']: slot for slot in event['before']['slots']}
        immediate_slots = {slot['slot_key']: slot for slot in event['immediate']['slots']}
        later_slots = {slot['slot_key']: slot for slot in event['later']['slots']}
        if set(before_slots) != set(immediate_slots) or set(before_slots) != set(later_slots):
            raise ValueError('Source transition lost or changed a wheel slot identity')
        target_before = before_slots.get(event['target_slot_key'])
        target_immediate = immediate_slots.get(event['target_slot_key'])
        if target_before is None or target_immediate is None:
            raise ValueError('Source transition target slot is missing from its snapshots')
        if target_before.get('native_wheel_index') != event['rpm_wheel_index']:
            raise ValueError('Source transition RPM wheel differs from its target slot')
        if event['rpm_wheel_index'] >= len(event['before']['native_wheels']):
            raise ValueError('Source transition target native wheel is missing')
        native = event['before']['native_wheels'][event['rpm_wheel_index']]
        if native.get('rpm') != event['rpm_before'] or native.get('rpm', 0) == 0:
            raise ValueError('Source transition RPM does not match the retained target wheel')
        expected_presence = {
            'remove_tire': ('tire_present', False), 'remove_cover': ('cover_present', False),
            'remove_other_cover': ('cover_present', False),
            'remove_driven_rim': ('rim_present', False), 'remove_other_driven_rim': ('rim_present', False),
            'remount_tire': ('tire_present', True), 'remount_cover': ('cover_present', True),
            'remount_other_cover': ('cover_present', True),
            'remount_driven_rim': ('rim_present', True), 'remount_other_driven_rim': ('rim_present', True),
        }.get(declaration['action'])
        if expected_presence and target_immediate.get(expected_presence[0]) is not expected_presence[1]:
            raise ValueError('Source transition immediate slot state differs from its source API action')
        if declaration['action'] == 'refresh_tire_condition' and target_immediate.get('condition_state') != declaration['condition_state']:
            raise ValueError('Source condition API did not apply the declared condition state')
        for slot_key, original in before_slots.items():
            for phase in ('immediate', 'later'):
                observed_slot = {slot['slot_key']: slot for slot in event[phase]['slots']}[slot_key]
                for identity, instance_id in (('retained_rim_key', 'rim_instance_id'),
                                              ('retained_tire_key', 'tire_instance_id'),
                                              ('retained_cover_key', 'cover_instance_id')):
                    if (not original.get(identity) or original.get(identity) != observed_slot.get(identity) or
                            type(original.get(instance_id)) is not int or original[instance_id] <= 0 or
                            original.get(instance_id) != observed_slot.get(instance_id)):
                        raise ValueError('Source transition changed an original part identity')
        if expected_presence and target_before.get(expected_presence[0]) is expected_presence[1]:
            raise ValueError('Source transition did not change the requested support state')
        if expected_presence and later_slots[event['target_slot_key']].get(expected_presence[0]) is not expected_presence[1]:
            raise ValueError('Later source slot state differs from the successful API result')
        source_wheels = snapshots[0]['native_wheels']
        for snapshot in snapshots:
            wheels = snapshot['native_wheels']
            if (len(wheels) != len(source_wheels) or
                    any(not wheel.get('key') or type(wheel.get('instance_id')) is not int or wheel['instance_id'] <= 0
                        for wheel in wheels) or
                    [(wheel['key'], wheel['instance_id']) for wheel in wheels] !=
                    [(wheel['key'], wheel['instance_id']) for wheel in source_wheels]):
                raise ValueError('Source transition changed or lost a native wheel identity')
        if (len({snapshot.get('source_lock_events') for snapshot in snapshots}) != 1 or
                len({snapshot.get('wheel_activation_events') for snapshot in snapshots}) != 1):
            raise ValueError('Source lock or wheel activation overlapped a transition')
        if declaration['action'] == 'refresh_tire_condition':
            if (event.get('condition_state') != declaration['condition_state'] or
                    target_before.get('condition_state') == declaration['condition_state'] or
                    target_immediate.get('condition_state') != declaration['condition_state'] or
                    later_slots[event['target_slot_key']].get('condition_state') != declaration['condition_state']):
                raise ValueError('Source condition API did not change to its declared condition state')
        for slot in event['later']['slots']:
            if not slot.get('slot_key') or not slot.get('retained_rim_key') or not slot.get('retained_tire_key') or not slot.get('retained_cover_key'):
                raise ValueError('Later source transition sample lost a slot or original part identity')
        expected_identities = sorted({identity for slot in event['before']['slots'] for identity in
                                      (slot['slot_key'], slot['retained_rim_key'], slot['retained_tire_key'], slot['retained_cover_key'])})
        if (not event.get('original_identities') or event.get('original_identities') != expected_identities or
                event.get('identities_after') != expected_identities):
            raise ValueError('Source transition lost an original part identity')
        if index and event['scheduled_frame'] <= observed[index-1].get('later_after_frame', -1):
            raise ValueError('Source transition events overlap a required later observation')

        rim_counts = {
            'remove_driven_rim': ((2, 2), (1, 1), (1, 1)),
            'remove_other_driven_rim': ((1, 1), (0, 1), (0, 1)),
            'remount_driven_rim': ((0, 1), (1, 1), (1, 1)),
            'remount_other_driven_rim': ((1, 1), (2, 2), (2, 2)),
        }.get(declaration['action'])
        observed_counts = tuple((snapshot['installed_driven_rims'], snapshot['driven_wheels_num']) for snapshot in snapshots)
        if rim_counts is not None and observed_counts != rim_counts:
            raise ValueError('Driven-rim count or clamped divisor differs across the declared support transition')




def validate_run(run, scenario):
    curve = validate_curve_run(run, scenario)
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
    required = PHASES if scenario['probe_kind'] in ('assembled_lada','wheel_angular','single_wheel_contact') else {'physics.entry', 'physics.exit'}
    if not required <= phases:
        raise ValueError(f'Missing native callback phases: {sorted(required - phases)}')
    if scenario['probe_kind'] == 'curve' and not run.get('curves'):
        raise ValueError('Native Unity curve observations are missing')
    if scenario['probe_kind'] in ('wheel_collider','wheel_angular','single_wheel_contact') and not run.get('isolated_wheel_probe'):
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
        if sample['phase'] == 'physics.exit' and len(sample.get('wheels', [])) != (1 if scenario['probe_kind'] in ('wheel_angular','single_wheel_contact') else 4):
            raise ValueError('Declared native measurement wheel count differs')
    if scenario['probe_kind'] in ('wheel_angular','single_wheel_contact') and samples[-1]['frame']>scenario['duration_frames']:
        raise ValueError('Angular render observation bound was exceeded')
    checked_scenario = dict(scenario, duration_frames=samples[-1]['frame']) if scenario['probe_kind'] in ('wheel_angular','single_wheel_contact') else scenario
    timing = validate_timing(samples, checked_scenario)
    controls = validate_controls(run, checked_scenario)
    if 'transitions' in scenario:
        validate_transition_evidence(run, scenario)
    if scenario['probe_kind']=='wheel_angular':
        timing['angular'] = validate_angular(run, scenario)
    if scenario['probe_kind']=='single_wheel_contact':
        timing['contact'] = validate_contact(run, scenario)
    domain = (dict(kind='native-engine-curve', input_boundary='reported native curve inputs',
                   qualified_measurements=['enginescript.torqueCurve.Evaluate'],
                   physical_measurements_qualified=False,
                   unqualified_context=['chassis', 'driving', 'suspension'])
              if curve is not None else dict(kind='physical', physical_measurements_qualified=True))
    if scenario['probe_kind']=='single_wheel_contact':
        domain=dict(kind='physical-contact',physical_measurements_qualified=True,scope='single copied wheel; measured static support; direct velocity input; entry-to-exit body momentum residual',native_contact_impulse_qualified=False)
    return {**({'engine_curve': curve} if curve is not None else {}),
            'qualification_domain': domain,
            'samples': len(samples), 'physics_steps': run['physics_steps'],
            'phases': sorted(phases), 'controls': controls, **timing}


def validate_evidence(output: Path, directory: Path, run: dict, scenario: dict) -> dict:
    result = validate_run(run, scenario)
    result['body_hold'] = validate_body_hold(run)
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


def repeat_state(run, scenario, result):
    if scenario['probe_kind'] == 'curve':
        return result['engine_curve']['input_state']
    state = {key: value for key, value in run['initial'].items() if key != 'profile_path'}
    state['physical_geometry'] = physical_inputs(next(row for row in run['geometry'] if row['phase'] == 'armed-after-canonical'))
    if scenario['probe_kind'] == 'wheel_angular':
        state['angular_initial'] = angular_initial_inputs(run)
    if scenario['probe_kind']=='single_wheel_contact':
        state['contact_initial'] = contact_initial_inputs(run)
    return state


def verify(output_root: Path) -> dict:
    output = output_root.resolve()
    request = read_json(output / 'request.json')
    if request.get('repeats') != 3:
        raise ValueError('Three fresh native repeats are required')
    results = []
    contact_body_input = None
    contact_group_initial = None
    contact_group_pre = None
    contact_enabled_count = 0
    for scenario in request['scenarios']:
        validate_curve_scenario(scenario)
        first_curve = None
        initial = None
        canonical_body_input = None
        angular_setup_state = None
        for repeat in range(1, 4):
            directory = output / scenario['id'] / f'repeat-{repeat}'
            path = directory / 'runtime.json'
            run = read_json(path)
            if scenario['probe_kind']=='wheel_angular':
                supplied=read_json(directory/'scenario.json')
                validate_angular_supplied(run,scenario,supplied,angular_setup_state)
                validate_angular_body_input(run,canonical_body_input,repeat==1)
                if repeat==1:
                    angular_setup_state=angular_transport_initial(run)
                    canonical_body_input=angular_setup_state['angular_body_input']
            if scenario['probe_kind']=='single_wheel_contact':
                first_contact=contact_body_input is None
                supplied=read_json(directory/'scenario.json')
                validate_contact_supplied(run,scenario,supplied)
                if ('contact_body_input' not in supplied or supplied.get('contact_body_input_from_source') is not first_contact or supplied.get('contact_body_input')!=contact_body_input):
                    raise ValueError('Contact scenario canonical transport or origin differs')
                validate_contact_body_input(run,contact_body_input,first_contact)
                if first_contact: contact_body_input=run['contact']['body_input']
            result = validate_evidence(output, directory, run, scenario)
            if scenario['probe_kind'] == 'curve':
                require_same_curve(first_curve, result['engine_curve'])
                first_curve = result['engine_curve']
            state = repeat_state(run, scenario, result)
            if scenario['probe_kind']=='single_wheel_contact' and scenario['contact']['wheel_enabled']:
                physical = dict(initial=contact_initial_inputs(run),fixed_dt=run['initial']['fixed_dt'])
                before_input = contact_pre_inputs(run)
                if contact_group_pre is not None and contact_exact(before_input)!=contact_exact(contact_group_pre):
                    raise ValueError('Contact enabled rows have different step-121 pre_input states')
                contact_group_pre = before_input
                if contact_group_initial is not None and contact_exact(physical)!=contact_exact(contact_group_initial):
                    raise ValueError('Contact enabled rows have different physical Initial states')
                contact_group_initial = physical
                contact_enabled_count += 1
            if initial is not None and (contact_exact(state)!=contact_exact(initial) if scenario['probe_kind']=='single_wheel_contact' else state!=initial):
                raise ValueError('Fresh native repeats have different initial states')
            initial = state
            result.update(scenario=scenario['id'], repeat=repeat, sha256=sha256(path))
            results.append(result)
    if contact_group_initial is not None and contact_enabled_count!=12:
        raise ValueError('Contact requires twelve matching enabled physical Initial states')
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
            ('assembled_lada', 'wheel_collider', 'curve', 'wheel_angular', 'single_wheel_contact') or row['duration_frames'] < 2):
            raise ValueError('Invalid native scenario')
        for key in ('source_objects', 'setup', 'frame_schedule', 'controls'):
            if key not in row: raise ValueError(f'Missing scenario field: {key}')
        validate_curve_scenario(row)
        validate_schedule(row)
        if row['probe_kind']=='wheel_angular': validate_angular_schedule(row)
        if row['probe_kind']=='single_wheel_contact': validate_contact_schedule(row)
        if 'transitions' in row: validate_transition_schedule(row)
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
    contact_body_input = None
    contact_group_initial = None
    contact_group_pre = None
    contact_enabled_count = 0
    for row in rows:
        first_curve = None
        setup_state = None
        initial = None
        for repeat in range(1, 4):
            directory = output / row['id'] / f'repeat-{repeat}'
            directory.mkdir(parents=True)
            run_scenario = dict(row)
            if row['probe_kind']=='single_wheel_contact':
                run_scenario['contact_body_input']=contact_body_input
                run_scenario['contact_body_input_from_source']=contact_body_input is None
            if setup_state is not None:
                run_scenario['setup_state'] = setup_state
                run_scenario['has_setup_state'] = True
            write_json(directory / 'scenario.json', run_scenario)
            invoke(['-Mode', 'Run', '-RunRoot', str(directory), '-ProfileRoot', str(directory / 'profile')],
                   directory / 'launcher.log', 200)
            native_run = read_json(directory / 'runtime.json')
            if row['probe_kind']=='wheel_angular':
                validate_angular_supplied(native_run,row,read_json(directory/'scenario.json'),setup_state)
                validate_angular_body_input(native_run,None if setup_state is None else setup_state['angular_body_input'],repeat==1)
            if row['probe_kind']=='single_wheel_contact':
                validate_contact_supplied(native_run,row,read_json(directory/'scenario.json'))
                validate_contact_body_input(native_run,contact_body_input,contact_body_input is None)
                if contact_body_input is None: contact_body_input=native_run['contact']['body_input']
            result = validate_evidence(output, directory, native_run, row)
            if row['probe_kind'] == 'curve':
                require_same_curve(first_curve, result['engine_curve'])
                first_curve = result['engine_curve']
            state = repeat_state(native_run, row, result)
            if row['probe_kind']=='single_wheel_contact' and row['contact']['wheel_enabled']:
                physical = dict(initial=contact_initial_inputs(native_run),fixed_dt=native_run['initial']['fixed_dt'])
                before_input = contact_pre_inputs(native_run)
                if contact_group_pre is not None and contact_exact(before_input)!=contact_exact(contact_group_pre):
                    raise ValueError('Contact enabled rows have different step-121 pre_input states')
                contact_group_pre = before_input
                if contact_group_initial is not None and contact_exact(physical)!=contact_exact(contact_group_initial):
                    raise ValueError('Contact enabled rows have different physical Initial states')
                contact_group_initial = physical
                contact_enabled_count += 1
            if initial is not None and (contact_exact(state)!=contact_exact(initial) if row['probe_kind']=='single_wheel_contact' else state!=initial):
                raise ValueError('Fresh native repeats have different initial states')
            initial = state
            if setup_state is None:
                setup_state=angular_transport_initial(native_run) if row['probe_kind']=='wheel_angular' else native_run['setup_state']
    if contact_group_initial is not None and contact_enabled_count!=12:
        raise ValueError('Contact requires twelve matching enabled physical Initial states')
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
