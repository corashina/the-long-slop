"""Checked authored geometry identities and exact physical-input comparisons."""
import math

from .library.types import ObjectId


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


def validate_geometry(run):
    snapshots = run.get('geometry', [])
    required = {'world.entry', 'source-physical-ready', 'prepared-held', 'backlog-drained-held',
                'before-dynamic', 'switched-dynamic', 'before-inertia-reset', 'after-inertia-reset',
                'canonical-transforms-synchronized', 'native-wheels-enabled', 'armed-after-canonical'}
    if not required <= {row['phase'] for row in snapshots}:
        raise ValueError('Missing physical geometry checkpoints')
    if any(not row['complete'] or row['unavailable'] or not row['colliders'] for row in snapshots):
        raise ValueError('Incomplete discovered physical geometry')
    for row in snapshots:
        if row['phase'] in required - {'world.entry'} and not row['physical_ready']:
            raise ValueError('Incomplete deferred cargo body removal or collider ownership')
        def scalar(value):
            return type(value) in (int, float) and math.isfinite(value)
        def vector(value):
            return isinstance(value, dict) and {'x', 'y', 'z'} <= value.keys() and all(scalar(value[k]) for k in ('x', 'y', 'z'))
        friction_fields = {'extremum_slip', 'extremum_value', 'asymptote_slip', 'asymptote_value', 'stiffness'}
        for shape in row['colliders']:
            if not vector(shape.get('center')):
                raise ValueError('Incomplete native shape data: explicit center fields are required')
            if shape['type'] == 'BoxCollider' and not vector(shape.get('size')):
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
