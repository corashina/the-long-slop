"""Checked authored geometry identities and exact physical-input comparisons."""
import math
from copy import deepcopy

from .library.types import ObjectId
from .driving_lifecycle import _mark, validate_audio


def source_geometry_map(reader):
    rows = []
    def visit(path_id, path):
        transform = reader.record(ObjectId('level1', path_id))
        game = reader.record(ObjectId('level1', transform['m_GameObject']['m_PathID']))
        components = []
        for reference in game['m_Component']:
            identity = ObjectId('level1', reference['component']['m_PathID'])
            record = reader.source_objects[identity]
            shape = reader.record(identity) if record['type'] in {'BoxCollider', 'CapsuleCollider', 'SphereCollider'} else {}
            material = shape.get('m_Material', {})
            components.append({'type': record['type'], 'key': f'level1:{identity.path_id}',
                               'material': f"level1-file{material.get('m_FileID', 0)}:{material.get('m_PathID', 0)}"})
        rows.append({'path': path, 'key': f'level1:{path_id}', 'components': components})
        counts = {}
        for child in transform['m_Children']:
            child_id = child['m_PathID']
            data = reader.record(ObjectId('level1', child_id))
            name = reader.record(ObjectId('level1', data['m_GameObject']['m_PathID']))['m_Name']
            occurrence = counts.get(name, 0)
            counts[name] = occurrence + 1
            visit(child_id, ('' if path == '.' else path + '/') + f'{name}[{occurrence}]')
    visit(37191, '.')
    return {'root': 'level1:37191', 'transforms': rows}


def physical_inputs(snapshot):
    """Exclude native IDs and computed outputs. Keep actual local shape and owner inputs."""
    return {'colliders': [{k: v for k, v in row.items() if k not in
             {'instance_id', 'owner_instance_id', 'world_matrix', 'body_matrix', 'sample', 'sprung_mass',
              'saved_enabled', 'held_enabled', 'restored_enabled', 'restoration_applied'}}
             for row in snapshot['colliders']],
            'transforms': [{k: v for k, v in row.items() if k not in
             {'instance_id', 'world_matrix', 'body_matrix', 'lossy_scale'} and
             not (row['key'] == snapshot['body']['transform_key'] and
                  k in {'local_position', 'local_rotation'})} for row in snapshot['transforms']]}


def first_difference(left, right, path=''):
    if type(left) is not type(right): return path, left, right
    if isinstance(left, dict):
        if left.keys() != right.keys(): return path + '.keys', list(left), list(right)
        for key in left:
            found = first_difference(left[key], right[key], path + '.' + key)
            if found: return found
    elif isinstance(left, list):
        if len(left) != len(right): return path + '.length', len(left), len(right)
        for index, (a, b) in enumerate(zip(left, right)):
            found = first_difference(a, b, f'{path}[{index}]')
            if found: return found
    elif left != right: return path, left, right
    return None


def scalar(value):
    return type(value) in (int, float) and math.isfinite(value)


def coordinates(value, fields=('x', 'y', 'z')):
    return isinstance(value, dict) and set(fields) <= value.keys() and all(scalar(value[k]) for k in fields)


def validate_ownership(snapshot):
    transforms = snapshot.get('transforms')
    if not isinstance(transforms, list) or not transforms:
        raise ValueError('Missing physical transform records')
    poses = {}
    for pose in transforms:
        key = pose.get('key')
        if not isinstance(key, str) or not key or key in poses:
            raise ValueError('Missing or ambiguous physical transform identity')
        if ('parent' not in pose or pose['parent'] is not None and not isinstance(pose['parent'], str) or
                not coordinates(pose.get('local_position')) or
                not coordinates(pose.get('local_rotation'), ('x', 'y', 'z', 'w')) or
                not coordinates(pose.get('local_scale')) or
                type(pose.get('active_self')) is not bool or type(pose.get('active_hierarchy')) is not bool):
            raise ValueError('Incomplete physical local transform: ' + key)
        poses[key] = pose
    chains = {}
    for key in poses:
        chain = set()
        current = key
        while current:
            if current in chain or current not in poses:
                raise ValueError('Broken physical transform parent chain: ' + key)
            chain.add(current)
            current = poses[current]['parent']
        chains[key] = chain
    bodies = snapshot.get('nested_bodies')
    if not isinstance(snapshot.get('body'), dict) or not isinstance(bodies, list):
        raise ValueError('Missing physical body ownership records')
    owners = {}
    for body in [snapshot['body'], *bodies]:
        key = body.get('key')
        transform = body.get('transform_key')
        if not isinstance(key, str) or not key or key in owners or not isinstance(transform, str) or transform not in poses:
            raise ValueError('Missing physical body ownership transform')
        if 'parent' not in body or body['parent'] != poses[transform]['parent']:
            raise ValueError('Physical body parent context differs from its transform')
        owners[key] = transform
    for shape in snapshot['colliders']:
        transform = shape.get('transform')
        if not isinstance(transform, str) or transform not in poses:
            raise ValueError('Missing physical collider transform reference')
        if 'owner' not in shape or type(shape.get('owner_is_null')) is not bool:
            raise ValueError('Missing explicit physical collider owner or null evidence')
        owner = shape['owner']
        if shape['owner_is_null']:
            if owner not in (None, ''):
                raise ValueError('Physical collider null ownership evidence conflicts')
        elif not isinstance(owner, str) or owner not in owners:
            raise ValueError('Missing physical collider owner body reference')
        elif owners[owner] not in chains[transform]:
            raise ValueError('Physical collider owner is outside its transform parent chain')


def validate_cargo(snapshot, lifecycle):
    cargo = snapshot.get('cargo')
    if not isinstance(cargo, list):
        raise ValueError('Missing retained cargo owner observations')
    identities = {}
    for item in cargo:
        key = item.get('key')
        rows = item.get('collider_owners')
        if not key or key in identities or not isinstance(rows, list) or type(item.get('ownership_ready')) is not bool:
            raise ValueError('Incomplete retained cargo owner observations')
        retained = []
        for row in rows:
            if (not row.get('key') or type(row.get('instance_id')) is not int or row['instance_id'] == 0 or
                    any(type(row.get(k)) is not bool for k in ('original_owner_is_null', 'present', 'owner_is_null', 'retired'))):
                raise ValueError('Missing retained cargo collider identity or explicit status')
            for prefix in ('original_', ''):
                owner, owner_id, owner_null = (row.get(prefix+k) for k in ('owner', 'owner_instance_id', 'owner_is_null'))
                if (prefix+'owner' not in row or type(owner_id) is not int or
                        (owner_null and (owner_id != 0 or owner)) or
                        (not owner_null and (owner_id == 0 or not isinstance(owner, str) or not owner))):
                    raise ValueError('Incomplete retained cargo native owner identity')
            if not {'retirement_audio', 'removed'} <= row.keys():
                raise ValueError('Missing explicit cargo retirement evidence')
            retained.append((row['key'], row['instance_id'], row['original_owner'], row['original_owner_instance_id'], row['original_owner_is_null']))
            if row['present']:
                if row['retired'] or row['retirement_audio'] != [] or row['removed'] != []:
                    raise ValueError('Live cargo collider conflicts with retirement evidence')
                continue
            evidence = row['retirement_audio']
            audio = evidence[0] if isinstance(evidence, list) and len(evidence) == 1 else None
            matches = [a for a in lifecycle.get('audio', []) if a.get('collider_id') == row['instance_id']]
            if (row['retired'] is not True or not row['owner_is_null'] or not isinstance(audio, dict) or
                    len(matches) != 1 or audio != matches[0] or audio.get('collider_id') != row['instance_id']):
                raise ValueError('Destroyed cargo collider has no exact original audio retirement proof')
            opened = _mark(lifecycle.get('opened'), 'world.entry')
            closed = _mark(lifecycle.get('closed'), 'observer.prepared-held.close-cohort')
            required, removed = validate_audio(audio, opened, closed)
            if (not required or removed is None or row['removed'] != [audio['removed']] or
                    type(snapshot.get('frame')) is not int or not scalar(snapshot.get('time')) or
                    audio['removed']['frame'] > snapshot['frame'] or audio['removed']['time'] > snapshot['time']):
                raise ValueError('Incomplete cargo source audio native removal at observation')
        if len({r[1] for r in retained}) != len(retained):
            raise ValueError('Repeated retained cargo collider identity')
        identities[key] = retained
    return identities


def validate_cargo_input(run, by_phase):
    report = run.get('cargo_input')
    if not isinstance(report, dict) or report.get('applications') != 1 or report.get('phase') != 'source-physical-ready':
        raise ValueError('Declared cargo input must apply exactly once at source-physical-ready')
    declared = report.get('input')
    if (not isinstance(declared, dict) or declared.get('phase') != 'source-physical-ready' or
            any(type(declared.get(k)) is not int or declared[k] < 0 for k in ('frame', 'physics_step', 'input_revision')) or
            not isinstance(declared.get('targets'), list) or report.get('arguments') != declared['targets'] or
            any(run.get(name, {}).get('cargo_input') != declared for name in ('setup_state', 'initial'))):
        raise ValueError('Missing or changed declared cargo input and exact setter arguments')
    phases = [row['phase'] for row in run['geometry']]
    if (phases.count('source-physical-ready') != 1 or phases.count('cargo-input-applied-held') != 1 or
            phases.index('cargo-input-applied-held') != phases.index('source-physical-ready') + 1 or
            phases.index('cargo-input-applied-held') >= phases.index('prepared-held')):
        raise ValueError('Declared cargo application boundary order differs')
    before, after = (by_phase[k] for k in ('source-physical-ready', 'cargo-input-applied-held'))
    if any(row['body'].get('kinematic') is not True for row in (before, after)):
        raise ValueError('Declared cargo input was not held')
    for field in ('frame', 'physics_step'):
        if type(report.get(field)) is not int or report[field] < 0 or any(report[field] != row.get(field) for row in (before, after)):
            raise ValueError('Missing cargo application source stamp')
    for field in ('time', 'fixed_time'):
        if not scalar(report.get(field)) or any(report[field] != row.get(field) for row in (before, after)):
            raise ValueError('Missing cargo application source clock')
    if type(report.get('input_revision')) is not int or report['input_revision'] < 0:
        raise ValueError('Missing cargo application input revision')
    caches = report.get('caches_before')
    if (not isinstance(caches, list) or caches != report.get('caches_after') or
            any(not isinstance(value, str) or not value for value in caches) or before['cargo'] and not caches):
        raise ValueError('Cargo source ownership or cache evidence changed or is missing')
    targets = declared['targets']
    keys = [target.get('key') for target in targets]
    if (any(not isinstance(key, str) or not key for key in keys) or len(set(keys)) != len(keys) or
            keys != sorted(keys) or set(keys) != {row['key'] for row in before['cargo']} or
            set(keys) != {row['key'] for row in after['cargo']}):
        raise ValueError('Declared cargo cohort is missing, duplicate, or extra')
    for snapshot in (before, after):
        if 'mounts' in snapshot:
            stored = [key for mount in snapshot['mounts'] for key in mount['stored']]
            if sorted(stored) != keys:
                raise ValueError('Declared cargo cohort differs from source stored membership')
    identities = dict(key='key', pickup='pickup', save='save', parent='parent', mount='lock_owner',
                      lock_parent='lock_parent', body='car_body', category='category', item='item', lock_id='lock_id')
    expected = deepcopy(physical_inputs(before))
    actual = physical_inputs(after)
    for target in targets:
        key = target['key']
        if (any(not isinstance(target.get(k), str) or not target[k] for k in ('pickup', 'save', 'parent', 'mount', 'lock_parent', 'body')) or
                any(type(target.get(k)) is not int for k in ('category', 'item', 'lock_id')) or
                not coordinates(target.get('local_position')) or not coordinates(target.get('local_scale')) or
                not coordinates(target.get('local_rotation'), ('x', 'y', 'z', 'w'))):
            raise ValueError('Incomplete declared cargo identity or exact local TRS')
        for phase, evidence in ((before, 'before'), (after, 'after')):
            cargo = next(row for row in phase['cargo'] if row['key'] == key)
            rows = report.get(evidence)
            observations = [row for row in rows if row.get('key') == key] if isinstance(rows, list) else []
            if len(observations) != 1 or len(rows) != len(keys):
                raise ValueError('Missing cargo native before or after observation')
            observed = observations[0]
            if (any(target[k] != cargo.get(v) for k, v in identities.items()) or
                    cargo.get('locked') is not True or cargo.get('ownership_ready') is not True or
                    any(cargo.get(k) is not False for k in ('body_present', 'original_body_present', 'deferred_destruction', 'retained_nested_body')) or
                    type(cargo.get('lock_parent_id')) is not int or cargo['lock_parent_id'] != cargo.get('expected_lock_parent_id')):
                raise ValueError('Declared cargo source parent, lock, or retired body differs')
            for field, source in (('root_id', 'root_id'), ('pickup_id', 'pickup_id'), ('save_id', 'save_id'),
                                  ('mount_id', 'mount_id'), ('body_id', 'car_body_id'), ('lock_parent_id', 'lock_parent_id')):
                if type(observed.get(field)) is not int or observed[field] != cargo.get(source) or field != 'lock_parent_id' and observed[field] == 0:
                    raise ValueError('Missing cargo raw native identity evidence')
            live = [row for row in cargo['collider_owners'] if row['present']]
            if (observed.get('owner_ids') != [row['owner_instance_id'] for row in live] or
                    any(row['owner'] != target['body'] or row['owner_instance_id'] != observed['body_id'] for row in live)):
                raise ValueError('Declared cargo native attached body differs')
            for field in ('local_position', 'local_rotation', 'local_scale'):
                if observed.get(field) != cargo.get(field):
                    raise ValueError('Cargo native setter output differs from raw geometry observation')
            poses = [row for row in phase['transforms'] if row['key'] == key]
            if len(poses) != 1 or any(poses[0][field] != cargo[field] for field in ('local_position', 'local_rotation', 'local_scale')):
                raise ValueError('Cargo root raw transform observation is missing or differs')
        # Only declared roots can change local TRS at this boundary. Native output is never replaced in stored records.
        source = next(row for row in expected['transforms'] if row['key'] == key)
        native = next(row for row in actual['transforms'] if row['key'] == key)
        for field in ('local_position', 'local_rotation', 'local_scale'):
            source[field] = native[field]
    if expected != actual:
        raise ValueError('Non-root physical input changed at declared cargo boundary')


def validate_geometry(run):
    snapshots = run.get('geometry', [])
    if run.get('probe_kind') not in ('assembled_lada', 'curve', 'wheel_collider','wheel_angular'):
        raise ValueError('Unknown declared physical capture path')
    isolated = run['probe_kind'] in ('wheel_collider','wheel_angular')
    if run.get('isolated_wheel_probe') is not isolated:
        raise ValueError('Declared physical source-body or isolated-probe path differs')
    required = {'world.entry', 'source-physical-ready', 'cargo-input-applied-held', 'prepared-held', 'backlog-drained-held',
                'before-dynamic', 'switched-dynamic',
                'canonical-transforms-synchronized', 'native-wheels-enabled', 'armed-after-canonical'}
    if not isolated:
        required.update(('before-inertia-reset', 'after-inertia-reset'))
    if not required <= {row['phase'] for row in snapshots}:
        raise ValueError('Missing physical geometry checkpoints')
    if any(not row['complete'] or row['unavailable'] or not row['colliders'] for row in snapshots):
        raise ValueError('Incomplete discovered physical geometry')
    retained_cargo = {}
    for row in snapshots:
        current_cargo = validate_cargo(row, run.get('lifecycle', {}))
        for key, owners in retained_cargo.items():
            if current_cargo.get(key) != owners:
                raise ValueError('Retained cargo collider or original owner record was lost')
        retained_cargo = current_cargo
        if row['phase'] in required - {'world.entry'} and not row['physical_ready']:
            raise ValueError('Incomplete deferred cargo body removal or collider ownership')
        validate_ownership(row)
        friction_fields = {'extremum_slip', 'extremum_value', 'asymptote_slip', 'asymptote_value', 'stiffness'}
        for shape in row['colliders']:
            if not coordinates(shape.get('center')):
                raise ValueError('Incomplete native shape data: explicit center fields are required')
            if shape['type'] == 'BoxCollider' and not coordinates(shape.get('size')):
                raise ValueError('Incomplete native shape data: explicit box size fields are required')
            if shape['type'] in ('SphereCollider', 'CapsuleCollider') and not scalar(shape.get('radius')):
                raise ValueError('Incomplete native shape data: explicit radius is required')
            if shape['type'] == 'CapsuleCollider' and (not scalar(shape.get('height')) or type(shape.get('direction')) is not int):
                raise ValueError('Incomplete native shape data: explicit capsule fields are required')
            if shape['type'] == 'WheelCollider':
                scalars = ('radius', 'mass', 'damping', 'suspension_distance', 'force_application_distance',
                           'spring_n_per_m', 'damper_n_s_per_m', 'spring_target')
                for name in scalars:
                    if not scalar(shape.get(name)):
                        raise ValueError('Incomplete native wheel profile scalar: ' + name)
                for name in ('forward', 'sideways'):
                    friction = shape.get(name)
                    if not isinstance(friction, dict) or not friction_fields <= friction.keys() or any(not scalar(friction[k]) for k in friction_fields):
                        raise ValueError('Incomplete native wheel friction profile: ' + name)
        keys = [shape['key'] for shape in row['colliders']]
        if len(keys) != len(set(keys)):
            raise ValueError('Ambiguous stable collider identity')
        required_shapes = {
            'BoxCollider': {'center', 'size'},
            'SphereCollider': {'center', 'radius'},
            'CapsuleCollider': {'center', 'radius', 'height', 'direction'},
            'WheelCollider': {'center', 'radius', 'mass', 'damping', 'suspension_distance',
                              'force_application_distance', 'spring_n_per_m', 'damper_n_s_per_m',
                              'spring_target', 'forward', 'sideways'},
        }
        if any(not shape['complete'] or shape['type'] not in required_shapes or
               not required_shapes[shape['type']] <= shape.keys() for shape in row['colliders']):
            raise ValueError('Incomplete native shape data')
    by_phase = {row['phase']: row for row in snapshots}
    validate_cargo_input(run, by_phase)
    for phase in ('prepared-held', 'backlog-drained-held', 'native-wheels-enabled'):
        wheels = [shape for shape in by_phase[phase]['colliders'] if shape['type'] == 'WheelCollider']
        if len(wheels) != 5 or sorted(shape['road_slot'] for shape in wheels) != [-1, 0, 1, 2, 3]:
            raise ValueError('Five source native wheels and four separate road roles are required')
        if any(not shape['sprung_mass_available'] for shape in wheels):
            raise ValueError('Native wheel sprungMass evidence is missing')
        if phase == 'native-wheels-enabled' and not isolated:
            if any(not shape['restoration_applied'] or shape['enabled'] != shape['intended_enabled'] or
                   shape['restored_enabled'] != shape['intended_enabled'] for shape in wheels):
                raise ValueError('Source native WheelCollider intended state was not restored')
    for sample in run['samples']:
        if len(sample.get('native_wheels', [])) != 5 or any(not wheel.get('sprung_mass_available') for wheel in sample['native_wheels']):
            raise ValueError('Buffered native helper or sprungMass samples are missing')
    if physical_inputs(by_phase['prepared-held']) != physical_inputs(by_phase['backlog-drained-held']):
        raise ValueError('Physical shape inputs changed across the held boundary')
    return {'checkpoints': len(snapshots), 'native_wheels': 5}
