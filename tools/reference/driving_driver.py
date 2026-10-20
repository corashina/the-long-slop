"""Strict source driver lifecycle and stationary input evidence."""

POSE_FIELDS = ('Tb', 'Th', 'THeadDir', 'THeadBob', 'THeadLean', 'TdefStandPos', 'TStandHeadPos', 'TProneHeadPos')
STAMPS = ('before-get-in', 'get-in-exit', 'native-body-removed', 'collider-owners-transferred',
          'seated-update-exit', 'stationary-input-applied', 'stationary-update-exit', 'held-update-exit')


def _pose(pose):
    if not isinstance(pose, dict) or not all(pose.get(k) for k in ('field', 'key', 'parent')):
        raise ValueError('Missing driver pose reference')
    for name, axes in (('local_position', 'xyz'), ('local_rotation', 'xyzw'), ('local_scale', 'xyz')):
        value = pose.get(name)
        if not isinstance(value, dict) or not all(type(value.get(axis)) in (int, float) for axis in axes):
            raise ValueError('Incomplete driver local pose')


def _poses(poses):
    if not isinstance(poses, list) or [p.get('field') for p in poses] != list(POSE_FIELDS):
        raise ValueError('Incomplete driver source target chain')
    for pose in poses:
        _pose(pose)


def validate_driver(run):
    report = run.get('driver', {})
    transfer = report.get('expected_colliders')
    if (not report.get('original_body') or report.get('original_body_instance_id', 0) == 0 or
            not isinstance(transfer, list) or not transfer or len(set(transfer)) != len(transfer)):
        raise ValueError('Missing retained native driver transfer set')
    if report.get('input_applications') != 1:
        raise ValueError('Stationary driver input must apply exactly once')
    source_input = report.get('input', {})
    if (source_input != run.get('setup_state', {}).get('driver_input') or
            source_input != run.get('initial', {}).get('driver_input')):
        raise ValueError('Canonical driver input differs from the applied record')
    _poses(source_input.get('poses'))
    if not source_input.get('references') or not source_input.get('parent_body') or source_input.get('stored_dt', 0) <= 0:
        raise ValueError('Missing canonical driver source or stored dt input')
    if (source_input.get('duck_value') != 1 or
            any(source_input.get(k) is not False for k in ('duck', 'last_ducked', 'stand_lean'))):
        raise ValueError('Invalid stationary driver duck input')
    if (any(source_input.get(k) != 0 for k in ('look_x', 'look_y', 'scroll')) or
            any(source_input.get(k) is not False for k in ('lean', 'lean_down', 'lean_up', 'recenter', 'recenter_down',
                'recenter_up', 'key_looking', 'input_duck', 'input_duck_down', 'input_duck_up'))):
        raise ValueError('Invalid canonical neutral driver input')
    for name in ('parent_velocity', 'car_velocity_history', 'head_velocity_history', 'head_bob_history'):
        if source_input.get(name) != dict.fromkeys('xyz', 0):
            raise ValueError('Stationary driver histories must equal authored zero')
    stamps = report.get('stamps', [])
    if [s.get('phase') for s in stamps] != list(STAMPS):
        raise ValueError('Missing or repeated original driver lifecycle boundary')
    by_phase = {s['phase']: s for s in stamps}
    previous = (-1, -1, -1)
    for stamp in stamps:
        if not {'frame', 'physics_step', 'update_sequence', 'input_revision', 'time', 'fixed_time',
                'delta_time', 'stored_dt', 'consumed_dt', 'callback'} <= stamp.keys():
            raise ValueError('Missing driver callback timing evidence')
        current = (stamp['frame'], stamp['physics_step'], stamp['update_sequence'])
        if any(a < b for a, b in zip(current, previous)):
            raise ValueError('Driver lifecycle timing is out of order')
        previous = current
        if len(stamp.get('collider_owners', [])) != len(transfer):
            raise ValueError('Incomplete expected driver collider ownership evidence')
    if not by_phase['before-get-in'].get('original_body_present'):
        raise ValueError('Original driver body missing before GetIn')
    for phase in STAMPS[2:]:
        stamp = by_phase[phase]
        if stamp.get('original_body_present') is not False or stamp.get('managed_body_present') is not False:
            raise ValueError('Native driver body removal is incomplete')
    for phase in STAMPS[3:]:
        stamp = by_phase[phase]
        if not stamp.get('owners_transferred') or set(stamp['collider_owners']) != {source_input['parent_body']}:
            raise ValueError('Expected driver collider owner transfer is incomplete')
    for phase in STAMPS[4:]:
        if not by_phase[phase].get('consistent'):
            raise ValueError('Inconsistent local seated driver ownership')
    seated = by_phase['seated-update-exit']
    first, following = by_phase['stationary-update-exit'], by_phase['held-update-exit']
    if (seated['update_sequence'] <= by_phase['collider-owners-transferred']['update_sequence'] or
            first['update_sequence'] <= by_phase['stationary-input-applied']['update_sequence'] or
            following['update_sequence'] != first['update_sequence'] + 1 or
            any(by_phase[phase]['callback'] != 'driver.update.exit' for phase in
                ('seated-update-exit', 'stationary-update-exit', 'held-update-exit'))):
        raise ValueError('Driver readiness requires completed original seated Updates')
    states = report.get('states', [])
    if [s.get('phase') for s in states] != ['prepared-held', 'backlog-drained-held']:
        raise ValueError('Missing driver held output records')
    for state, stamp in zip(states, (first, following)):
        _poses(state.get('poses'))
        _pose(state.get('piss_pose'))
        if not {'look_point', 'yaw', 'pitch', 'stored_dt', 'consumed_dt', 'duck_value', 'duck', 'last_ducked',
                'stand_lean', 'parent_velocity', 'car_velocity_history', 'head_velocity_history', 'head_bob_history',
                'drive_head_bob', 'driver_update_flag', 'sleeping', 'look_x', 'look_y', 'scroll', 'input_revision'} <= state.keys():
            raise ValueError('Incomplete source driver output state')
        if (state.get('xr') is not False or state.get('died') is not False or state.get('sleeping') is not False or
                any(state.get(k) is not True for k in ('mind', 'cursor_locked', 'neutral')) or
                state.get('references') != source_input['references'] or state.get('parent_body') != source_input['parent_body'] or
                state.get('update_sequence') != stamp['update_sequence'] or
                any(state[k] != 0 for k in ('look_x', 'look_y', 'scroll'))):
            raise ValueError('Invalid non-XR stationary source driver state')
    return {'expected_colliders': len(transfer), 'input_applications': 1,
            'prepared_update_sequence': first['update_sequence'], 'held_update_sequence': following['update_sequence']}
