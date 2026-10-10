"""Recover road and terrain generator records.

Generator values stay in Unity space. The runtime converts generated geometry once.
"""
import math

from .gltf import GltfExporter
from .source import key, source_id, vector

ROAD_GENERATOR = 64771
TERRAIN_SETTINGS = 64898
GRAPHICS_SETTINGS = 16865
HOME_ALIGNMENT = 79970


def trs(position, rotation, scale):
    """Column-major Unity TRS matrix."""
    x, y, z, w = rotation
    sx, sy, sz = scale
    columns = [
        (1-2*(y*y+z*z), 2*(x*y+z*w), 2*(x*z-y*w)),
        (2*(x*y-z*w), 1-2*(x*x+z*z), 2*(y*z+x*w)),
        (2*(x*z+y*w), 2*(y*z-x*w), 1-2*(x*x+y*y)),
    ]
    result = []
    for column, s in zip(columns, (sx, sy, sz)):
        result.extend([column[0]*s, column[1]*s, column[2]*s, 0.])
    result.extend([position[0], position[1], position[2], 1.])
    return result


def multiply(a, b):
    return [sum(a[k*4+row]*b[column*4+k] for k in range(4)) for column in range(4) for row in range(4)]


def local(data):
    return trs(vector(data['m_LocalPosition']), vector(data['m_LocalRotation'], 4), vector(data['m_LocalScale']))


def relative(source, reader, ancestor):
    """Matrix from an ancestor Transform to a descendant Transform."""
    result = [1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.]
    while key(reader) != key(ancestor):
        data = source.read(reader)
        result = multiply(local(data), result)
        reader = source.resolve(reader, data['m_Father'])
        if reader is None:
            raise ValueError('Transform is outside the required ancestor')
    return result


def world_position(source, reader):
    point = (0., 0., 0.)
    for data in source.chain(reader):
        m = local(data)
        point = tuple(m[i]*point[0]+m[4+i]*point[1]+m[8+i]*point[2]+m[12+i] for i in range(3))
    return point


def draw_distances(source):
    """`settingsscript.RefreshInMain` at the default draw distance slider."""
    reader = source.get('level0', GRAPHICS_SETTINGS)
    if source.class_name(reader) != 'settingsscript':
        raise ValueError('Required graphics settings differ')
    settings = source.read(reader)
    slider = source.read(source.resolve(reader, settings['tabGraphics']['sliders'][0]))
    draw = slider['defValue'] * (slider['valueMultipler'] or 1.)
    low, high = settings['DDSmin'], settings['DDSmax']
    t = min(1., max(0., (draw-low['drawDistance'])/(high['drawDistance']-low['drawDistance'])))
    values = {name: low[name]+(high[name]-low[name])*t for name in low}
    values['drawDistance'] = draw
    return reader, values


def tile_layer(setting, generate, remove):
    """`TerrainSetting.Upd` inputs for the near terrain."""
    if setting['YCoord'] != 0.:
        raise ValueError('Near terrain must sit at the generated height')
    return {'resolution': max(4, int(setting['resolution'])), 'tile_size': float(int(setting['tileSize'])),
            'generate_distance': generate, 'remove_distance': remove}


def mesh_data(source, mesh):
    from UnityPy.helpers.MeshHelper import MeshHandler
    handler = MeshHandler(source.object(mesh))
    handler.process()
    groups = handler.get_triangles()
    submeshes = handler.src.m_SubMeshes
    if len(groups) != 1 or not handler.m_BoneIndices or not handler.m_UV0:
        raise ValueError('Road mesh must have one skinned, mapped submesh')
    triangles = [[a+submeshes[0].baseVertex, b+submeshes[0].baseVertex, c+submeshes[0].baseVertex] for a, b, c in groups[0]]
    # The source mesh binds each vertex to one bone with full weight.
    if handler.m_BoneWeights is not None or any(len(v) != 1 for v in handler.m_BoneIndices):
        raise ValueError('Road mesh must bind each vertex to one bone')
    joints = [int(v[0]) for v in handler.m_BoneIndices]
    poses = [[matrix[f'e{row}{column}'] for column in range(4) for row in range(4)] for matrix in source.read(mesh)['m_BindPose']]
    return {'positions': [list(v[:3]) for v in handler.m_Vertices], 'normals': [list(v[:3]) for v in handler.m_Normals],
            'uvs': [list(v[:2]) for v in handler.m_UV0], 'triangles': triangles,
            'joints': joints, 'bind_poses': poses}


def road_prefab(source, exporter, generator, pointer):
    game_object = source.resolve(generator, pointer)
    root = source.transform_for(game_object)
    records = source.hierarchy(root, activate_root=True)
    def components(record, name):
        return [c for c in record['components'] if c is not None and
                (source.class_name(c) if c.type.name == 'MonoBehaviour' else c.type.name) == name]
    script_reader = components(records[0], 'roadscript')[0]
    script = source.read(script_reader)
    bones = [source.resolve(script_reader, p) for p in script['bones']]
    renderer = next(c for r in records for c in components(r, 'SkinnedMeshRenderer'))
    renderer_data = source.read(renderer)
    skin = [source.resolve(renderer, p) for p in renderer_data['m_Bones']]
    if [key(b) for b in skin[:len(bones)]] != [key(b) for b in bones]:
        raise ValueError('Road skin must list road bones first')
    armature = source.resolve(bones[0], source.read(bones[0])['m_Father'])
    for index, bone in enumerate(bones):
        data = source.read(bone)
        expected = (0., 0., 0.) if index == 0 else (0., script['boneLength'], 0.)
        if any(abs(a-b) > 1e-4 for a, b in zip(vector(data['m_LocalPosition']), expected)) \
                or any(abs(a-b) > 1e-6 for a, b in zip(vector(data['m_LocalRotation'], 4), (0., 0., 0., 1.))) \
                or vector(data['m_LocalScale']) != (1., 1., 1.):
            raise ValueError('Road bones must form a straight chain')
        if index and key(source.resolve(bone, data['m_Father'])) != key(bones[index-1]):
            raise ValueError('Road bones must form one parent chain')
    mesh = source.resolve(renderer, renderer_data['m_Mesh'])
    material = exporter.material(source.resolve(renderer, renderer_data['m_Materials'][0]))
    pole = source.resolve(script_reader, script['randomPoleRotTransform'])
    if key(source.resolve(pole, source.read(pole)['m_Father'])) != key(bones[0]):
        raise ValueError('Road pole must be a child of the first road bone')
    renderers, colliders = [], []
    pole_records = source.hierarchy(pole)
    hidden = set()
    for record in pole_records:
        for component in record['components']:
            if component is None or component.type.name != 'LODGroup':
                continue
            lods = source.read(component)['m_LODs']
            shown = {key(source.resolve(component, p['renderer'])) for p in lods[0]['renderers']} if lods else set()
            hidden.update(key(source.resolve(component, p['renderer'])) for lod in lods[1:] for p in lod['renderers'])
            hidden.difference_update(shown)
    for record in pole_records:
        if not record['active'] or 'VillanyOszlopRope' in record['path']:
            continue
        parts = {c.type.name: c for c in record['components'] if c is not None}
        matrix = relative(source, record['reader'], pole)
        if 'MeshFilter' in parts and 'MeshRenderer' in parts:
            data = source.read(parts['MeshRenderer'])
            if data['m_Enabled'] and key(parts['MeshRenderer']) not in hidden:
                mesh_reader = source.resolve(parts['MeshFilter'], source.read(parts['MeshFilter'])['m_Mesh'])
                materials = [source.resolve(parts['MeshRenderer'], p) for p in data['m_Materials']]
                index = exporter.mesh(mesh_reader, materials)
                primitives = exporter.document.meshes[index].primitives
                if len(primitives) != 1:
                    raise ValueError('Road pole meshes must have one primitive')
                renderers.append({'matrix': matrix, 'mesh': index, 'material': primitives[0].material})
        for name in ('CapsuleCollider', 'BoxCollider'):
            if name not in parts:
                continue
            data = source.read(parts[name])
            if not data['m_Enabled'] or data['m_IsTrigger']:
                continue
            if name == 'CapsuleCollider':
                shape = {'kind': 'capsule', 'center': list(vector(data['m_Center'])), 'radius': data['m_Radius'],
                         'height': data['m_Height'], 'direction': data['m_Direction']}
            else:
                shape = {'kind': 'box', 'center': list(vector(data['m_Center'])), 'size': list(vector(data['m_Size']))}
            colliders.append({'matrix': matrix, 'shape': shape})
    static = []
    for index in range(len(bones), len(skin)):
        static.append({'joint': index, 'matrix': relative(source, skin[index], armature)})
    return {
        'source': source_id(game_object),
        'material': material,
        'mesh': source_id(mesh),
        'armature': relative(source, armature, root),
        'static_bones': static,
        'pole': {'matrix': relative(source, pole, bones[0]),
                 'chances': [script['randomPoleRotChance1'], script['randomPoleRotChance2']],
                 'angles': [[script[f'randomPoleRotAngle{axis}{i}'] for axis in 'XYZ'] for i in (1, 2)],
                 'renderers': renderers, 'colliders': colliders},
    }, mesh


def export_world(source, output):
    generator = source.get('level1', ROAD_GENERATOR)
    settings_reader = source.get('level1', TERRAIN_SETTINGS)
    if source.class_name(generator) != 'roadGenScript' or source.class_name(settings_reader) != 'TerrainGenerationSettings':
        raise ValueError('Required world generator links differ')
    road = source.read(generator)
    settings = source.read(settings_reader)
    if [p['m_PathID'] for p in settings['roadGens']] != [ROAD_GENERATOR]:
        raise ValueError('World generation requires one road generator')
    for name in ('world',):
        data = source.read(source.resolve(generator, road[name]))
        if vector(data['m_LocalPosition']) != (0., 0., 0.) or vector(data['m_LocalRotation'], 4) != (0., 0., 0., 1.):
            raise ValueError('Road world must start at the origin and face +Z')
    if road['aliignRoad'] or road['alignBuildings'] or not road['alignEdgeOfTheWorld']:
        raise ValueError('Road height alignment flags differ')
    graphics, dds = draw_distances(source)
    exporter = GltfExporter(source, output)
    biomes, meshes = [], []
    for pointer in road['prefabs']:
        record, mesh = road_prefab(source, exporter, generator, pointer)
        biomes.append(record)
        meshes.append(mesh)
    if len({key(m) for m in meshes}) != 1:
        raise ValueError('Road biomes must share one skinned mesh')
    segment = mesh_data(source, meshes[0])
    terrain = source.resolve(settings_reader, settings['MaterialTerrain'])
    terrain_data = source.read(terrain)
    sand = dict(terrain_data['m_SavedProperties']['m_TexEnvs'])['_SandTexture']
    gl = exporter.gl
    terrain_material = len(exporter.document.materials)
    exporter.document.materials.append(gl.Material(name=terrain_data['m_Name'],
        pbrMetallicRoughness=gl.PbrMetallicRoughness(baseColorTexture=gl.TextureInfo(**exporter.texture(terrain, sand, 'srgb')),
            metallicFactor=0., roughnessFactor=1.),
        extras={'source': source_id(terrain), 'adaptation': 'TerrainBiome sand layer only. Biome blending is not ported.'}))
    alignment = source.get('level1', HOME_ALIGNMENT)
    if source.class_name(alignment) != 'terrainHeightAlignToBuildingScript':
        raise ValueError('Home terrain alignment link differs')
    alignment_data = source.read(alignment)
    if alignment_data['range2']:
        raise ValueError('Home terrain alignment uses default range only')
    zones = []
    for pointer in alignment_data['helpPosCoords']:
        reader = source.resolve(alignment, pointer)
        half = source.read(reader)['m_LocalScale']['x']*0.5
        zones.append({'center': list(world_position(source, reader)), 'inner_sqr': half*half, 'outer_sqr': (half*4.)**2})
    anchor = source.resolve(settings_reader, settings['StartObjs'][0])
    asset = exporter.save([])
    noises = [{name: n[name] for name in ('amplitude', 'frequency', 'octaves', 'persistence', 'lacunarity')} for n in road['noises']]
    return {
        'source': source_id(settings_reader),
        'graphics_source': source_id(graphics),
        'asset': {'path': f'reference/{asset["path"]}', 'sha256': asset['sha256'], 'dependencies': asset['dependencies']},
        'seed': settings['seed'],
        'edge_of_world': settings['edgeOfWorld'],
        'edge_gradient': settings['edgeOfWorldGradient'],
        'terrain': tile_layer(settings['Terrain'], dds['TerrainGenDist'], dds['TerrainRemoveDist']),
        'terrain_material': terrain_material,
        'flat_zones': zones,
        'height_anchor': list(world_position(source, anchor)),
        'road': {
            'source': source_id(generator),
            'length': road['length'], 'width': road['width'], 'scale': road['scale'], 'noises': noises,
            'segment_length': road['roadSegmentLength'], 'bone_length': road['roadBoneLength'], 'bone_count': road['roadBoneNum'],
            'max_pitch_degrees': road['XmaxAngle'], 'max_yaw_degrees': road['YmaxAngle'], 'above_terrain': road['roadAboveTerrain'],
            'start_straight': road['startStraight'], 'straight_modifier': road['straightModifier'],
            'point_range': road['defPointRange'], 'generate_distance': dds['RoadGenDist'],
            'segment': segment, 'biomes': biomes,
        },
    }
