"""Explicit keyboard events and isolated native actuator declarations."""
import argparse
import json
import math
import struct
from pathlib import Path

SOURCE_FIELDS = {'frame', 'throttle', 'brake', 'clutch', 'steer', 'ignition_button_held', 'handbrake_press'}
ISOLATED_FIELDS = {'frame', 'motor_scale', 'service_brake_fraction', 'handbrake_fraction', 'steer'}
MODE_FIELDS = {'h_shifter': False, 'analog_gas': False, 'analog_brake': False,
               'analog_clutch': False, 'analog_steer': False, 'analog_handbrake': False,
               'mouse_drive': False, 'force_manual': False}


def validate_schedule(scenario):
    controls = scenario.get('controls')
    duration = scenario.get('duration_frames')
    if type(duration) is not int or duration < 2 or not isinstance(controls, list) or not controls:
        raise ValueError('Declared finite duration and control schedule are required')
    isolated = scenario.get('probe_kind') == 'wheel_collider'
    if scenario.get('probe_kind') not in ('curve', 'assembled_lada', 'wheel_collider'):
        raise ValueError('Unknown control protocol')
    fields = ISOLATED_FIELDS if isolated else SOURCE_FIELDS
    frames = []
    for control in controls:
        if not isinstance(control, dict) or not fields <= control.keys() or control.keys() - fields - ({'gear_press'} if not isolated else set()):
            raise ValueError('Wrong control protocol fields')
        frame = control['frame']
        if type(frame) is not int or not 0 <= frame < duration:
            raise ValueError('Declared control schedule must increase from zero inside the finite interval')
        frames.append(frame)
        numeric = fields - {'frame', 'ignition_button_held', 'handbrake_press'}
        for name in numeric:
            value = control[name]
            low, high = (-1, 1) if name in ('steer', 'motor_scale') else (0, 1)
            if type(value) not in (int, float) or not math.isfinite(value) or not low <= value <= high:
                raise ValueError('Invalid control axis or actuator fraction: ' + name)
        if not isolated:
            if any(type(control[name]) is not bool for name in ('ignition_button_held', 'handbrake_press')):
                raise ValueError('Source button declarations must be Boolean')
            if 'gear_press' in control:
                event = control['gear_press']
                if not isinstance(event, dict) or event.keys() != {'value'} or type(event['value']) is not int or not -1 <= event['value'] <= 6:
                    raise ValueError('Direct gear event requires one integer value from -1 through 6')
                if scenario.get('setup', {}).get('auto_shift') is not False:
                    raise ValueError('Direct gear events require original manual input mode')
    if frames[0] != 0 or frames != sorted(set(frames)):
        raise ValueError('Declared control schedule must increase from zero inside the finite interval')
    if not isolated:
        if scenario.get('input_mode') != 'keyboard-digital':
            raise ValueError('Declared keyboard/digital input mode is required')
        if any(type(scenario.get('setup', {}).get(k)) is not bool for k in ('auto_clutch', 'auto_shift')):
            raise ValueError('Declared original automatic assist modes are required')
        intent = scenario.get('parking_brake_intent')
        if intent not in ('parked', 'release-before-acceleration'):
            raise ValueError('Explicit parking brake intent is required')
        if intent == 'parked' and any(c['handbrake_press'] or c['throttle'] for c in controls):
            raise ValueError('Parked experiment cannot declare acceleration or a release event')
        if intent == 'release-before-acceleration':
            first = next((c['frame'] for c in controls if c['throttle'] > 0), None)
            releases = [c['frame'] for c in controls if c['handbrake_press'] and (first is None or c['frame'] < first)]
            if first is None or len(releases) != 1:
                raise ValueError('Free launch requires one prior parking brake release event')
    phase = scenario.get('brake_phase')
    if phase is not None:
        channels = {'motor', 'engine', 'service', 'parking'}
        if isolated or not isinstance(phase, dict) or phase.keys() != {'start_frame','end_frame','required_contributors','excluded_contributors','observe_engine_interlock'}:
            raise ValueError('Invalid source brake phase declaration')
        if type(phase['start_frame']) is not int or type(phase['end_frame']) is not int or not 0 <= phase['start_frame'] < phase['end_frame'] <= duration or type(phase['observe_engine_interlock']) is not bool:
            raise ValueError('Invalid source brake phase interval')
        for key in ('required_contributors','excluded_contributors'):
            if not isinstance(phase[key],list) or any(type(name) is not str for name in phase[key]) or len(set(phase[key])) != len(phase[key]) or not set(phase[key]) <= channels:
                raise ValueError('Invalid source brake contributor declaration')
        if set(phase['required_contributors']) & set(phase['excluded_contributors']):
            raise ValueError('Source brake contributors cannot be required and excluded')
    return duration, frames


def row_at(scenario, frame):
    return next(c for c in reversed(scenario['controls']) if c['frame'] <= frame)


def ignition_edges(scenario, frame):
    rows = scenario['controls']
    index = next(i for i in range(len(rows)-1, -1, -1) if rows[i]['frame'] <= frame)
    row = rows[index]
    held = row['ignition_button_held']
    prior = rows[index-1]['ignition_button_held'] if index else False
    return held, frame == row['frame'] and held and not prior, frame == row['frame'] and prior and not held


def validate_mode(mode, scenario):
    expected = dict(MODE_FIELDS, auto_clutch=scenario['setup']['auto_clutch'], auto_shift=scenario['setup']['auto_shift'])
    if not isinstance(mode, dict) or not expected.keys() <= mode.keys() or any(type(mode[k]) is not bool or mode[k] != v for k, v in expected.items()) or type(mode.get('force_automatic_transmission')) is not bool:
        raise ValueError('Missing or different actual keyboard/digital mode evidence')


def validate_controls(run, scenario):
    validate_schedule(scenario)
    if scenario['probe_kind'] == 'wheel_collider':
        return validate_isolated(run, scenario)
    initial = run['initial']
    validate_mode(initial.get('control_mode'), scenario)
    if type(initial.get('stored_handbrake')) is not bool or initial['stored_handbrake'] is not True:
        raise ValueError('Original garage parking brake initial state is required')
    wheels = initial.get('wheels', [])
    if len(wheels) != 4 or any(any(type(w.get(k)) is not bool for k in ('driven', 'service_brake', 'handbrake')) for w in wheels) or not any(w['handbrake'] for w in wheels):
        raise ValueError('Source wheel brake roles are required')
    samples = run['samples']
    by_phase = {}
    for s in samples:
        by_phase.setdefault(s['phase'], []).append(s)
        state = s.get('controls')
        if not isinstance(state, dict):
            raise ValueError('Missing source control observation')
        validate_mode(state.get('mode'), scenario)
        if state['mode'] != initial['control_mode']:
            raise ValueError('Actual source control mode changed after arming')
        bools = ('input_handbrake_press', 'car_handbrake_press', 'stored_handbrake', 'input_ignition_held',
                 'input_ignition_down', 'input_ignition_up', 'car_ignition_held', 'car_ignition_down',
                 'seat_driver0', 'seat_driver1', 'driving', 'driving2', 'player_driving', 'player_driving2')
        if any(type(state.get(k)) is not bool for k in bools) or any(state.get(k) is not True for k in ('seat_driver0', 'seat_driver1', 'driving', 'driving2', 'player_driving', 'player_driving2')):
            raise ValueError('Missing source control group ownership or Boolean observation')
        if type(state.get('input_revision')) is not int or state['input_revision'] < 0 or any(type(state.get(k)) is not int for k in ('input_gears', 'car_gears')):
            raise ValueError('Missing source input revision or digital gear evidence')
        axes = ('input_throttle', 'input_brake', 'input_clutch', 'input_steer', 'input_handbrake',
                'car_throttle', 'car_brake', 'car_clutch', 'car_steer', 'car_handbrake', 'selected_ratio', 'differential_ratio', 'brake_power', 'max_handbrake')
        if any(type(state.get(k)) not in (int, float) or not math.isfinite(state[k]) for k in axes):
            raise ValueError('Missing finite source control outputs')
    for phase in ('input.entry', 'input.exit', 'input2.entry', 'input2.exit', 'control.entry', 'control.exit', 'actuation.entry', 'actuation.exit', 'torque.entry', 'torque.exit'):
        if [s['frame'] for s in by_phase.get(phase, [])] != list(range(scenario['duration_frames']+1)):
            raise ValueError('Missing or duplicated source event callback: ' + phase)
    latch = initial['stored_handbrake']
    latest_input = latest_input2 = latest_consumed = latest_actuation = None
    events = []
    consumed_revisions = set()
    consumed_gears = set()
    gear_events = []
    phase_samples = {c['frame']: [] for c in scenario['controls']}
    event_input = None
    for sample in samples:
        phase, frame, state = sample['phase'], sample['frame'], sample['controls']
        row = row_at(scenario, frame)
        press = row['frame'] == frame and row['handbrake_press']
        held, down, up = ignition_edges(scenario, frame)
        if phase == 'input.exit':
            entry = next(s for s in by_phase['input.entry'] if s['frame'] == frame)
            if state['input_revision'] != entry['controls']['input_revision'] + 1:
                raise ValueError('Source input callback revision did not advance once')
            if state['input_handbrake_press'] != press or state['input_handbrake'] != 0:
                raise ValueError('Declared handbrake event was missed or replayed')
            gear = row.get('gear_press') if row['frame'] == frame else None
            if state['input_gears'] != (1 << (gear['value']+1) if gear is not None else 0):
                raise ValueError('Declared direct gear event was missed or replayed')
            latest_input = sample
        elif phase == 'input2.exit':
            if (state['input_ignition_held'], state['input_ignition_down'], state['input_ignition_up']) != (held, down, up):
                raise ValueError('Ignition held-state edge differs from declaration')
            latest_input2 = sample
        elif phase == 'control.exit':
            if latest_input is None or latest_input2 is None:
                if sample['input_sequence'] or state['car_handbrake_press'] or state['car_gears']:
                    raise ValueError('Source car event has no observed source producer')
                latest_consumed = sample
                event_input = None
                continue
            producer = latest_input['controls']
            producer2 = latest_input2['controls']
            if state['input_revision'] != producer['input_revision'] or sample['input_sequence'] != latest_input['input_sequence']:
                raise ValueError('Source car input revision differs from producer')
            pairs = [('car_handbrake_press', producer['input_handbrake_press']), ('car_gears', producer['input_gears']),
                     ('car_ignition_held', producer2['input_ignition_held']), ('car_ignition_down', producer2['input_ignition_down'])]
            pairs += [('car_'+k, producer['input_'+k]) for k in ('throttle', 'brake', 'clutch', 'steer', 'handbrake')]
            if any(state[k] != value for k, value in pairs):
                raise ValueError('Source GetPlayerInput did not consume original input outputs')
            latest_consumed = sample
            event_input = latest_input
            if state['car_gears']:
                revision = producer['input_revision']
                if revision in consumed_gears:
                    raise ValueError('Source direct gear event was consumed twice')
                consumed_gears.add(revision)
                gear_events.append({'declared_frame': latest_input['frame'], 'frame': frame, 'input_revision': revision,
                                    'input_sequence': sample['input_sequence'], 'consumed_sequence': sample['sequence'],
                                    'value': row_at(scenario, latest_input['frame'])['gear_press']['value']})
        elif phase == 'actuation.entry':
            if latest_consumed is None or latest_consumed['frame'] != frame or state['stored_handbrake'] != latch:
                raise ValueError('Source latch before control consumption differs')
        elif phase == 'actuation.exit':
            if state['car_handbrake_press']:
                if event_input is None or event_input['controls']['input_revision'] in consumed_revisions:
                    raise ValueError('Source handbrake event was consumed twice or lacks a producer')
                consumed_revisions.add(event_input['controls']['input_revision'])
                before = latch
                latch = not latch
                events.append({'declared_frame': event_input['frame'], 'frame': frame,
                               'input_callback_sequence': event_input['sequence'], 'consumed_sequence': latest_consumed['sequence'],
                               'sequence': sample['sequence'], 'input_sequence': sample['input_sequence'],
                               'input_revision': state['input_revision'], 'stored_handbrake_before': before, 'stored_handbrake': latch})
            if state['stored_handbrake'] != latch or sample.get('handbrake') != float(latch):
                raise ValueError('Measured source parking brake outcome differs')
            if row['throttle'] > 0 and scenario['parking_brake_intent'] == 'release-before-acceleration':
                # Later engage presses intentionally permit powered braking.
                previous_presses = sum(c['handbrake_press'] for c in scenario['controls'] if c['frame'] <= frame)
                if previous_presses == 0 or previous_presses == 1 and latch:
                    raise ValueError('Free launch retained the source parking brake')
            latest_actuation = sample
        elif phase == 'torque.exit':
            if latest_actuation is None or latest_actuation['frame'] != frame:
                raise ValueError('Native torque lacks original source actuation')
            contributions = []
            for role, wheel in zip(wheels, sample['wheels']):
                engine = f32(abs(f32(f32(f32(sample['engine_brake_nm']) * f32(state['selected_ratio'])) * f32(state['differential_ratio']))) / sample['torque_divisor']) if role['driven'] and state['selected_ratio'] != 0 else 0.
                service = f32(f32(state['brake_power']) * f32(sample['brake'])) if role['service_brake'] else 0.
                parking = f32(f32(state['max_handbrake']) * f32(sample['handbrake'])) if role['handbrake'] else 0.
                expected = f32(f32(engine + service) + parking)
                if f32(wheel['brake_torque']) != expected:
                    raise ValueError('Source wheel-role native brake torque differs')
                motor = f32(f32(f32(f32(sample['engine_output_nm']) * f32(state['selected_ratio'])) * f32(state['differential_ratio'])) / sample['torque_divisor']) if role['driven'] and state['selected_ratio'] != 0 else 0.
                contributions.append(dict(engine_brake_nm=engine, service_brake_nm=service, parking_brake_nm=parking,
                                          engine_motor_nm=motor, native_motor_nm=wheel['motor_torque'], native_brake_nm=wheel['brake_torque']))
            if event_input is not None:
                phase_samples[row_at(scenario, event_input['frame'])['frame']].append((sample, contributions))
        elif phase in ('physics.entry', 'physics.exit') and sample['input_sequence']:
            if latest_actuation is None or state['stored_handbrake'] != latest_actuation['controls']['stored_handbrake'] or sample.get('handbrake') != latest_actuation['handbrake']:
                raise ValueError('Native physics lost measured source parking brake outcome')
    declared = [c['frame'] for c in scenario['controls'] if c['handbrake_press']]
    if [e['declared_frame'] for e in events] != declared:
        raise ValueError('Declared handbrake event consumption is missing or duplicated')
    if [e['declared_frame'] for e in gear_events] != [c['frame'] for c in scenario['controls'] if 'gear_press' in c]:
        raise ValueError('Declared direct gear event consumption is missing or duplicated')
    brake_phase = validate_brake_phase(scenario, phase_samples)
    return {'protocol': 'keyboard-digital', 'handbrake_events': events, 'gear_events': gear_events, 'brake_phase': brake_phase,
            'requested_phases': phase_summary(scenario, wheels, phase_samples)}


def f32(value):
    return struct.unpack('<f', struct.pack('<f', value))[0]


def bounds(values):
    return [min(values), max(values)] if values else None


def phase_summary(scenario, wheels, observed):
    phases = []
    for row in scenario['controls']:
        rows = observed[row['frame']]
        phase = {'declared_frame': row['frame'], 'requests': row, 'torque_samples': len(rows)}
        for name in ('gear', 'clutch', 'brake', 'handbrake', 'engine_output_nm', 'engine_brake_nm'):
            phase[name] = bounds([sample[name] for sample, _ in rows])
        for name in ('car_throttle', 'car_brake', 'car_clutch'):
            phase[name] = bounds([sample['controls'][name] for sample, _ in rows])
        phase['wheel_contributors'] = [dict(source_path=role.get('path'), roles=role,
            **{name: bounds([terms[i][name] for _, terms in rows]) for name in (
                'engine_brake_nm','service_brake_nm','parking_brake_nm','engine_motor_nm','native_motor_nm','native_brake_nm')}) for i, role in enumerate(wheels)]
        phases.append(phase)
    return phases


def validate_brake_phase(scenario, observed):
    phase = scenario.get('brake_phase')
    if phase is None:
        return None
    rows = [pair for frame, pairs in observed.items() if phase['start_frame'] <= frame < phase['end_frame'] for pair in pairs]
    if not rows:
        raise ValueError('Declared source brake phase has no native torque observations')
    channels = {'motor':'engine_motor_nm', 'engine':'engine_brake_nm', 'service':'service_brake_nm', 'parking':'parking_brake_nm'}
    measured = []
    for sample, terms in rows:
        present = {name for name, key in channels.items() if any(term[key] != 0 for term in terms)}
        if set(phase['excluded_contributors']) & present:
            raise ValueError('Named isolated brake phase includes an excluded measured contributor')
        if sample['rpm'] >= 0 and sample['engine_output_nm'] > 0 and sample['engine_brake_nm'] > 0:
            raise ValueError('Observed normal-RPM engine outputs violate original motor/brake interlock')
        if set(phase['required_contributors']) <= present:
            measured.append(sample['sequence'])
    if not measured:
        raise ValueError('Named brake phase lacks simultaneous measured required contributors')
    engine_channels = [name for name in ('motor','engine') if any(any(term[channels[name]] != 0 for term in terms) for _, terms in rows)]
    if phase['observe_engine_interlock'] and not engine_channels:
        raise ValueError('Requested engine-channel experiment has no measured engine response')
    return {'declaration':phase, 'torque_samples':len(rows), 'qualified_sequences':measured, 'observed_engine_channels':engine_channels,
            'engine_channels':'Original motor and engine braking cannot both be positive at nonnegative RPM'}


def validate_isolated(run, scenario):
    samples = [sample for sample in run.get('samples',[]) if sample['phase'] in ('physics.entry','physics.exit')]
    if not samples:
        raise ValueError('Isolated actuator observations are required')
    for sample in samples:
        row = row_at(scenario,sample['frame'])
        state = sample.get('controls',{})
        if any(type(state.get(key)) not in (int,float) or not math.isfinite(state[key]) for key in ('brake_power','max_handbrake')):
            raise ValueError('Isolated source actuator scales are missing')
        motor = f32(f32(row['motor_scale']) * f32(state['brake_power']))
        brake = f32(f32(f32(row['service_brake_fraction']) * f32(state['brake_power'])) + f32(f32(row['handbrake_fraction']) * f32(state['max_handbrake'])))
        if len(sample.get('wheels',[])) != 4 or any(f32(w['motor_torque']) != motor or f32(w['brake_torque']) != brake for w in sample['wheels']):
            raise ValueError('Declared isolated actuator fractions differ from native wheel torque')
    exits = [s for s in samples if s['phase']=='physics.exit']
    phases = []
    for row in scenario['controls']:
        observed = [s for s in exits if row_at(scenario,s['frame']) is row]
        wheels = []
        for index in range(4):
            crossings = []
            observed_sequences = {sample['sequence'] for sample in observed}
            for before,after in zip(exits,exits[1:]):
                if after['sequence'] not in observed_sequences: continue
                left,right = before['wheels'][index]['rpm'],after['wheels'][index]['rpm']
                if left * right < 0 or left != right and (left == 0 or right == 0):
                    crossings.append({'before_sequence':before['sequence'],'after_sequence':after['sequence'],
                                      'before_step':before['step'],'after_step':after['step'],
                                      'before_rpm':left,'after_rpm':right,
                                      'before_motor_nm':before['wheels'][index]['motor_torque'],'after_motor_nm':after['wheels'][index]['motor_torque'],
                                      'before_brake_nm':before['wheels'][index]['brake_torque'],'after_brake_nm':after['wheels'][index]['brake_torque']})
            wheels.append({'slot':index, 'rpm':bounds([s['wheels'][index]['rpm'] for s in observed]),
                           'derived_omega_rad_s':bounds([s['wheels'][index]['rpm'] * math.tau / 60 for s in observed]),
                           'native_motor_nm':bounds([s['wheels'][index]['motor_torque'] for s in observed]),
                           'native_brake_nm':bounds([s['wheels'][index]['brake_torque'] for s in observed]),
                           'zero_crossings':crossings, 'zero_crossing_observed':bool(crossings)})
        phases.append({'requests':row,'physics_samples':len(observed),'wheels':wheels})
    return {'protocol':'isolated-native-actuators','native_actuator_phases':phases}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('scenarios', type=Path)
    args = parser.parse_args()
    rows = json.loads(args.scenarios.read_text(encoding='utf-8-sig'))['scenarios']
    for row in rows:
        validate_schedule(row)
        print(row['id'] + ': ' + json.dumps({'handbrake_presses': [c['frame'] for c in row['controls'] if c.get('handbrake_press')], 'frames': [c['frame'] for c in row['controls']]}))
    print('CONTROL_PROTOCOL_VALIDATED scenarios=' + str(len(rows)))


if __name__ == '__main__':
    main()
