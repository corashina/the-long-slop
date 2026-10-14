"""Recover source fuel station prototypes and their roadside rules."""
from .source import key, source_id, vector
from .collision import bake_points, world_pose, geometry
from .gltf import GltfExporter

# Runtime asset name: itemdatabase.b value, itemdatabase field, and prefab GameObject in sharedassets1.assets.
STATIONS = {'station_benzinkut01': (4, 'bbenzinkut01', 26573),
            'station_legjobbbenzinkut': (6, 'blegjobbbenzinkut', 27344),
            'station_nagybenzinkut': (8, 'bnagybenzinkut', 41123)}
DATABASE = ('level1', 66024)
ROAD_BUILDINGS = ('level1', 64750)
COLLIDERS = ('BoxCollider', 'SphereCollider', 'CapsuleCollider', 'MeshCollider')


def station_roots(source):
    database = source.get(*DATABASE); data = source.read(database)
    roots = {}
    for name, (_, field, game_object) in STATIONS.items():
        prefab = source.resolve(database, data[field])
        if key(prefab) != ('sharedassets1.assets', game_object):
            raise ValueError(f'Station prefab link differs: {name}')
        roots[name] = [source.transform_for(prefab)]
    return roots


def _component(source, transform, name):
    for reader in source.components(transform)[1]:
        if reader.type.name == 'MonoBehaviour' and source.class_name(reader) == name:
            return reader
    raise ValueError(f'{key(transform)} requires {name}')


def _ground(source, root, pose):
    """terrainHeightAlignToBuildingScript.FStart: zero range2 means 4, negative means 0."""
    align = _component(source, root, 'terrainHeightAlignToBuildingScript'); data = source.read(align)
    result = []
    for index, pointer in enumerate(data['helpPosCoords']):
        marker = source.resolve(align, pointer)
        range2 = data['range2'][index] if index < len(data['range2']) else 0.
        range2 = 4. if range2 == 0 else max(range2, 0.)
        scale = vector(source.read(marker)['m_LocalScale'])[0]
        result.append({'source': source_id(marker), 'center': bake_points([(0., 0., 0.)], source.chain(marker), pose)[0],
                       'radius': scale * .5, 'blend_radius': scale * range2 * .5})
    if not result:
        raise ValueError(f'{key(root)} has no ground markers')
    return result


def _colliders(source, root, pose):
    """Static solid shapes. Pushable rigid bodies and triggers stay out of the fixed station."""
    exporter = GltfExporter(source, None, source.library)
    dynamic = set(); result = []
    for node in source.hierarchy(root, activate_root=True):
        identity = key(node['reader'])
        if (node['parent'] and key(node['parent']) in dynamic) or any(c.type.name == 'Rigidbody' for c in node['components']):
            dynamic.add(identity); continue
        if not node['active']:
            continue
        for reader in node['components']:
            if reader.type.name not in COLLIDERS:
                continue
            data = source.read(reader)
            if data['m_Enabled'] and not data['m_IsTrigger']:
                result.append({'source': source_id(reader), 'geometry': geometry(source, exporter, reader, node['reader'], pose, False)})
    return result


def recover_stations(source, roots, manifest):
    road = source.get(*ROAD_BUILDINGS); rules = source.read(road)
    if source.class_name(road) != 'roadBuildingScript':
        raise ValueError('Road building generator link differs')
    types = {t['type']: t for group in rules['groups'] for t in group['types']}
    nodes = {(n['source']['file'], n['source']['path_id']): n for n in manifest['nodes']}
    stations = []
    for name, (kind, _, _) in STATIONS.items():
        rule = types[kind]
        if rule['dontUseDefault'] or not rule['useModifier'] or rule['sides']:
            raise ValueError(f'Station side rule differs: {name}')
        root = roots[name][0]; node = nodes[key(root)]
        pose = world_pose(source, root)
        if pose['translation'] != [0., 0., 0.] or pose['rotation'] != [0., 0., 0., 1.] or pose['scale'] != [1., 1., 1.]:
            raise ValueError(f'Station prefab root must use the identity pose: {name}')
        modifier = rule['xDistModifier']
        stations.append({'source': source_id(source.resolve(root, source.read(root)['m_GameObject'])), 'kind': kind,
            'asset': node['asset'], 'root': node['id'],
            'sides': [{'lateral_meters': [side['minXpos'] * modifier, side['maxXpos'] * modifier],
                       'yaw_degrees': [side['minYAngle'], side['maxYAngle']]} for side in rules['defaultSides']],
            'ground': _ground(source, root, pose), 'colliders': _colliders(source, root, pose)})
    manifest['stations'] = {'source': source_id(road), 'types': stations}
