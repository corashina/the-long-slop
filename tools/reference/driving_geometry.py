"""Checked authored geometry identities and exact physical-input comparisons."""
import math

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


def validate_geometry(run):
    snapshots = run.get('geometry', [])
    if run.get('probe_kind') not in ('assembled_lada', 'curve', 'wheel_collider'):
        raise ValueError('Unknown declared physical capture path')
    isolated = run['probe_kind'] == 'wheel_collider'
    if run.get('isolated_wheel_probe') is not isolated:
        raise ValueError('Declared physical source-body or isolated-probe path differs')
    required = {'world.entry', 'source-physical-ready', 'prepared-held', 'backlog-drained-held',
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
    for phase in ('prepared-held', 'backlog-drained-held', 'native-wheels-enabled'):
        wheels = [shape for shape in by_phase[phase]['colliders'] if shape['type'] == 'WheelCollider']
        if len(wheels) != 5 or sorted(shape['road_slot'] for shape in wheels) != [-1, 0, 1, 2, 3]:
            raise ValueError('Five source native wheels and four separate road roles are required')
        if any(not shape['sprung_mass_available'] for shape in wheels):
            raise ValueError('Native wheel sprungMass evidence is missing')
        if phase == 'native-wheels-enabled' and run['probe_kind'] != 'wheel_collider':
            if any(not shape['restoration_applied'] or shape['enabled'] != shape['intended_enabled'] or
                   shape['restored_enabled'] != shape['intended_enabled'] for shape in wheels):
                raise ValueError('Source native WheelCollider intended state was not restored')
    for sample in run['samples']:
        if len(sample.get('native_wheels', [])) != 5 or any(not wheel.get('sprung_mass_available') for wheel in sample['native_wheels']):
            raise ValueError('Buffered native helper or sprungMass samples are missing')
    if physical_inputs(by_phase['prepared-held']) != physical_inputs(by_phase['backlog-drained-held']):
        raise ValueError('Physical shape inputs changed across the held boundary')
    return {'checkpoints': len(snapshots), 'native_wheels': 5}
