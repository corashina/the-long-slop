"""Export selected Unity hierarchies through pygltflib."""
import hashlib
import math
import struct
from pathlib import Path

from .source import key, source_id, vector


def convert_point(value: tuple[float, float, float]) -> tuple[float, float, float]:
    return value[0], value[1], -value[2]


def convert_rotation(value):
    return -value[0], -value[1], value[2], value[3]


def convert_triangles(values):
    return [(a, c, b) for a, b, c in values]


def convert_bind_poses(matrices):
    """Reflect Unity inverse bind matrices and store glTF column order."""
    signs = (1, 1, -1, 1)
    return [tuple(matrix[f'e{row}{column}'] * signs[row] * signs[column]
                  for column in range(4) for row in range(4)) for matrix in matrices]


def transform_point(point, translation, rotation, scale):
    x, y, z = [point[i] * scale[i] for i in range(3)]
    qx, qy, qz, qw = rotation
    tx, ty, tz = 2*(qy*z-qz*y), 2*(qz*x-qx*z), 2*(qx*y-qy*x)
    return (x+qw*tx+qy*tz-qz*ty+translation[0],
            y+qw*ty+qz*tx-qx*tz+translation[1],
            z+qw*tz+qx*ty-qy*tx+translation[2])


def converted_transform(data):
    return {'translation': list(convert_point(vector(data['m_LocalPosition']))),
            'rotation': list(convert_rotation(vector(data['m_LocalRotation'], 4))),
            'scale': list(vector(data['m_LocalScale']))}


class GltfExporter:
    def __init__(self, source, output):
        import pygltflib as gl
        self.gl = gl
        self.source = source
        self.output = output
        self.document = gl.GLTF2(asset=gl.Asset(generator='My Summer Slop reference exporter'))
        self.binary = bytearray()
        self.meshes = {}
        self.materials = {}
        self.textures = {}
        self.mesh_data = {}
        self.material_report = []
        self.samplers = {}

    def accessor(self, values, shape, component=5126, target=34962):
        gl = self.gl
        width = {'SCALAR': 1, 'VEC2': 2, 'VEC3': 3, 'VEC4': 4, 'MAT4':16}[shape]
        rows = [tuple(value) if width > 1 else (value,) for value in values]
        if not rows or any(len(row) != width or not all(math.isfinite(v) for v in row) for row in rows):
            raise ValueError(f'Invalid {shape} mesh attribute')
        self.binary.extend(b'\0' * (-len(self.binary) % 4))
        offset = len(self.binary)
        flat = [v for row in rows for v in row]
        code = {5126:'f',5125:'I',5123:'H'}[component]
        self.binary.extend(struct.pack(f'<{len(flat)}{code}', *flat))
        view = len(self.document.bufferViews)
        self.document.bufferViews.append(gl.BufferView(buffer=0, byteOffset=offset, byteLength=len(self.binary)-offset, target=target))
        accessor = len(self.document.accessors)
        self.document.accessors.append(gl.Accessor(bufferView=view, componentType=component, count=len(rows), type=shape,
            min=[min(row[i] for row in rows) for i in range(width)], max=[max(row[i] for row in rows) for i in range(width)]))
        return accessor

    def texture(self, owner, environment, color_space):
        from PIL import Image
        gl = self.gl
        reader = self.source.resolve(owner, environment['m_Texture'])
        if reader is None:
            return None
        if reader.type.name == 'RenderTexture':
            self.material_report.append({'source':source_id(reader),'adaptation':'Runtime render texture omitted; authored material factors retained.'})
            return None
        identity = key(reader), color_space
        if identity not in self.textures:
            texture = self.source.object(reader)
            settings=texture.m_TextureSettings
            sampler_key=(settings.m_FilterMode,settings.m_WrapU,settings.m_WrapV,texture.m_MipCount>1,settings.m_Aniso,settings.m_MipBias)
            if sampler_key not in self.samplers:
                wrapping={0:10497,1:33071,2:33648}
                if settings.m_WrapU not in wrapping or settings.m_WrapV not in wrapping:
                    raise ValueError(f'Unsupported texture wrap mode: {key(reader)}')
                filtering={0:(9728,9984),1:(9729,9985),2:(9729,9987)}
                mag,minimum=filtering[settings.m_FilterMode]
                if texture.m_MipCount<=1:minimum=mag
                self.samplers[sampler_key]=len(self.document.samplers)
                self.document.samplers.append(gl.Sampler(wrapS=wrapping[settings.m_WrapU],wrapT=wrapping[settings.m_WrapV],
                    magFilter=mag,minFilter=minimum,extras={'unity_aniso':settings.m_Aniso,'unity_mip_bias':settings.m_MipBias}))
            name = f'texture-{reader.assets_file.name}-{reader.path_id}-{color_space}.png'
            texture.image.transpose(Image.Transpose.FLIP_TOP_BOTTOM).save(self.output.parent / name)
            image = len(self.document.images)
            self.document.images.append(gl.Image(uri=name, extras={'source': source_id(reader), 'color_space': color_space,'unity_color_space':texture.m_ColorSpace}))
            self.textures[identity] = len(self.document.textures)
            self.document.textures.append(gl.Texture(source=image, sampler=self.samplers[sampler_key]))
        scale, offset = environment['m_Scale'], environment['m_Offset']
        transform = {'offset': [offset['x'], offset['y']], 'scale': [scale['x'], scale['y']]}
        if 'KHR_texture_transform' not in self.document.extensionsUsed:
            self.document.extensionsUsed.append('KHR_texture_transform')
        return {'index': self.textures[identity], 'extensions': {'KHR_texture_transform': transform}}

    def material(self, reader, color_override=None):
        if reader is None:
            raise ValueError('Required renderer material has no source')
        identity = key(reader), tuple(color_override.values()) if color_override else None
        if identity in self.materials:
            return self.materials[identity]
        gl = self.gl
        data = self.source.read(reader)
        properties = data['m_SavedProperties']
        floats = dict(properties.get('m_Floats', []))
        colors = dict(properties.get('m_Colors', []))
        environments = dict(properties.get('m_TexEnvs', []))
        color = colors.get('_Color', colors.get('_BaseColor', {'r': 1., 'g': 1., 'b': 1., 'a': 1.}))
        if color_override:color=color_override
        mode = int(floats.get('_Mode', 0))
        shader = self.source.resolve(reader, data['m_Shader'])
        shader_name = (self.source.object(shader).m_Name or self.source.object(shader).m_ParsedForm.m_Name) if shader else 'Unity built-in shader'
        if mode == 0 and data.get('m_CustomRenderQueue', -1) >= 3000:
            mode = 2
        material = gl.Material(name=data['m_Name'], alphaMode={0:'OPAQUE',1:'MASK',2:'BLEND',3:'BLEND'}.get(mode,'OPAQUE'),
            alphaCutoff=floats.get('_Cutoff', 0.5) if mode == 1 else None,
            doubleSided=floats.get('_Cull',2) == 0,
            pbrMetallicRoughness=gl.PbrMetallicRoughness(baseColorFactor=[color[n] for n in ('r','g','b','a')],
                metallicFactor=floats.get('_Metallic',0.), roughnessFactor=1.-floats.get('_Glossiness',0.5)),
            extras={'source': source_id(reader), 'unity_shader': shader_name, 'unity_properties': properties})
        for name in ('_MainTex','_BaseMap'):
            if name in environments:
                info = self.texture(reader, environments[name], 'srgb')
                if info:
                    material.pbrMetallicRoughness.baseColorTexture = gl.TextureInfo(**info)
                break
        if '_BumpMap' in environments:
            info = self.texture(reader, environments['_BumpMap'], 'linear')
            if info:
                material.normalTexture = gl.NormalMaterialTexture(scale=floats.get('_BumpScale',1.), **info)
        emission = colors.get('_EmissionColor')
        if emission:
            material.emissiveFactor = [emission[n] for n in ('r','g','b')]
            if '_EmissionMap' in environments:
                info = self.texture(reader, environments['_EmissionMap'], 'srgb')
                if info:
                    material.emissiveTexture = gl.TextureInfo(**info)
        self.materials[identity] = len(self.document.materials)
        self.document.materials.append(material)
        self.material_report.append({'source': source_id(reader), 'shader': shader_name,
            'adaptation': 'Unity shader properties retained in extras; base color, normal, emission, alpha mapped to glTF PBR.'})
        return self.materials[identity]

    def geometry(self, reader):
        if key(reader) not in self.mesh_data:
            from UnityPy.helpers.MeshHelper import MeshHandler
            handler = MeshHandler(self.source.object(reader))
            handler.process()
            if not handler.m_Vertices:
                raise ValueError(f'Mesh has no vertices: {key(reader)}')
            triangles = handler.get_triangles()
            triangles=[[(a+submesh.baseVertex,b+submesh.baseVertex,c+submesh.baseVertex) for a,b,c in group]
                for submesh,group in zip(handler.src.m_SubMeshes,triangles)]
            count = len(handler.m_Vertices)
            if any(i < 0 or i >= count for group in triangles for triangle in group for i in triangle):
                raise ValueError(f'Mesh index outside vertices: {key(reader)}')
            self.mesh_data[key(reader)] = handler, list(map(convert_point, handler.m_Vertices)), [convert_triangles(group) for group in triangles]
        return self.mesh_data[key(reader)]

    def mesh(self, mesh_reader, material_readers, color_override=None):
        identity = key(mesh_reader), tuple(key(r) if r else None for r in material_readers), tuple(color_override.values()) if color_override else None
        if identity in self.meshes:
            return self.meshes[identity]
        gl = self.gl
        handler, points, submeshes = self.geometry(mesh_reader)
        position = self.accessor(points,'VEC3')
        normal = self.accessor([convert_point(v[:3]) for v in handler.m_Normals], 'VEC3') if handler.m_Normals else None
        tangent = self.accessor([(*convert_point(v[:3]),-v[3]) for v in handler.m_Tangents], 'VEC4') if handler.m_Tangents else None
        uvs={f'TEXCOORD_{index}':self.accessor(values,'VEC2') for index in range(8) if (values:=getattr(handler,f'm_UV{index}'))}
        colors=self.accessor(handler.m_Colors,'VEC4') if handler.m_Colors else None
        joints = self.accessor(handler.m_BoneIndices, 'VEC4',5123) if handler.m_BoneIndices else None
        weights = self.accessor(handler.m_BoneWeights, 'VEC4') if handler.m_BoneWeights else None
        primitives = []
        if len(material_readers) < len(submeshes):
            raise ValueError(f'Missing submesh material: {key(mesh_reader)}')
        for index, triangles in enumerate(submeshes):
            if not triangles:
                continue
            material = self.material(material_readers[index],color_override) if material_readers[index] else None
            if material is None:
                self.material_report.append({'source':source_id(mesh_reader), 'submesh':index,
                    'unresolved':'Serialized material is null. Original startup creates the material.'})
            attributes = gl.Attributes(POSITION=position, NORMAL=normal, TANGENT=tangent, COLOR_0=colors, JOINTS_0=joints, WEIGHTS_0=weights, **uvs)
            primitives.append(gl.Primitive(attributes=attributes, indices=self.accessor([i for t in triangles for i in t], 'SCALAR',5125,34963),
                material=material, extras={'submesh': index, 'source': source_id(mesh_reader), 'material_resolved':material is not None}))
        mesh_id = len(self.document.meshes)
        self.document.meshes.append(gl.Mesh(primitives=primitives, name=self.source.object(mesh_reader).m_Name))
        self.meshes[identity] = mesh_id
        return mesh_id

    def export(self, roots):
        gl = self.gl
        self.output.parent.mkdir(parents=True, exist_ok=True)
        records = []
        mapping = {}
        for root in roots:
            records.extend(self.source.hierarchy(root, activate_root=True))
        hidden_lods=set()
        for record in records:
            for component in record['components']:
                if component.type.name != 'LODGroup':continue
                lods=self.source.read(component)['m_LODs']
                if not lods:continue
                highest={key(self.source.resolve(component,p['renderer'])) for p in lods[0]['renderers']}
                for lod in lods[1:]:
                    hidden_lods.update(key(self.source.resolve(component,p['renderer'])) for p in lod['renderers'])
                hidden_lods.difference_update(highest)
        for index, record in enumerate(records):
            identity = key(record['reader'])
            if identity in mapping:
                raise ValueError(f'Duplicate export root: {identity}')
            mapping[identity] = index
        for record in records:
            reader = record['reader']
            data = self.source.read(reader)
            transform = converted_transform(data)
            if record['parent'] is None:
                from .collision import world_pose
                transform=world_pose(self.source,reader)
            node = gl.Node(name=self.source.read(record['game_object'])['m_Name'],
                extras={'reference_id': source_id(reader), 'active':record['active'], 'hierarchy_path':record['path'],
                    'layer':self.source.read(record['game_object'])['m_Layer']}, **transform)
            components = {r.type.name:r for r in record['components']}
            renderer = components.get('MeshRenderer') or components.get('SkinnedMeshRenderer')
            mesh_filter = components.get('MeshFilter')
            if renderer and (mesh_filter or renderer.type.name == 'SkinnedMeshRenderer'):
                renderer_data = self.source.read(renderer)
                render_enabled=bool(renderer_data['m_Enabled']) and key(renderer) not in hidden_lods
                text_component=next((r for r in record['components'] if r.type.name=='MonoBehaviour' and self.source.read(r)['m_Script']['m_PathID'] and self.source.class_name(r) in {'TextMeshPro','TMP_SubMesh'}),None)
                if text_component:
                    render_enabled=False
                    self.material_report.append({'source':source_id(renderer),
                        'adaptation':'Omitted obsolete editor TMP glyphs. Seeded plate text requires glyph regeneration.'})
                node.extras['renderer_enabled'] = render_enabled
                owner = mesh_filter or renderer
                mesh_reader = self.source.resolve(owner, self.source.read(owner)['m_Mesh'])
                if mesh_reader and record['active'] and render_enabled:
                    materials=[self.source.resolve(renderer,p) for p in renderer_data['m_Materials']]
                    selected=getattr(self.source,'condition_materials',{}).get(key(renderer))
                    if selected:materials=[selected for _ in materials]
                    color=getattr(self.source,'condition_colors',{}).get(key(renderer))
                    condition=getattr(self.source,'condition_renderers',{}).get(key(renderer))
                    if condition:node.extras['condition']=condition
                    if any(r is None for r in materials) and key(mesh_reader)==('sharedassets0.assets',564):
                        materials=[self.source.get('sharedassets1.assets',624) if r is None else r for r in materials]
                        self.material_report.append({'source':source_id(renderer),'adaptation':'Authored mirror material 624. Live camera reflection omitted.'})
                    node.mesh = self.mesh(mesh_reader,materials,color)
                    if renderer.type.name == 'SkinnedMeshRenderer':
                        bones = [self.source.resolve(renderer, pointer) for pointer in renderer_data['m_Bones']]
                        if bones:
                            if any(key(bone) not in mapping for bone in bones):
                                raise ValueError(f'Skin joint outside selected hierarchy: {key(reader)}')
                            matrices = self.source.read(mesh_reader)['m_BindPose']
                            if len(matrices) != len(bones):
                                raise ValueError(f'Skin bind pose count: {key(reader)}')
                            converted = convert_bind_poses(matrices)
                            node.skin = len(self.document.skins)
                            root_bone = self.source.resolve(renderer,renderer_data['m_RootBone'])
                            self.document.skins.append(gl.Skin(joints=[mapping[key(bone)] for bone in bones],
                                inverseBindMatrices=self.accessor(converted,'MAT4',target=None),
                                skeleton=mapping[key(root_bone)] if root_bone else None))
            self.document.nodes.append(node)
        root_nodes = []
        for record in records:
            index = mapping[key(record['reader'])]
            if record['parent'] is None:
                root_nodes.append(index)
            else:
                self.document.nodes[mapping[key(record['parent'])]].children.append(index)
        self.document.scenes.append(gl.Scene(nodes=root_nodes))
        self.document.scene = 0
        self.document.buffers.append(gl.Buffer(uri=self.output.with_suffix('.bin').name,byteLength=len(self.binary)))
        self.output.with_suffix('.bin').write_bytes(self.binary)
        self.document.save_json(str(self.output))
        validate_gltf(self.output)
        paths={buffer.uri for buffer in self.document.buffers}|{image.uri for image in self.document.images}
        dependencies=[{'file':f'reference/{path}','sha256':hashlib.sha256((self.output.parent/path).read_bytes()).hexdigest()} for path in sorted(paths)]
        return {'path': self.output.name, 'sha256': hashlib.sha256(self.output.read_bytes()).hexdigest(),
                'dependencies':dependencies,
                'node_count':len(records), 'mesh_count':len(self.meshes), 'material_adaptations':self.material_report,
                'nodes':[{'source':source_id(r['reader']),'gltf_node':mapping[key(r['reader'])],
                    'parent':source_id(r['parent']) if r['parent'] else None,'hierarchy_path':r['path'],
                    'active':r['active'],'transform':{'translation':self.document.nodes[mapping[key(r['reader'])]].translation,
                        'rotation':self.document.nodes[mapping[key(r['reader'])]].rotation,'scale':self.document.nodes[mapping[key(r['reader'])]].scale}} for r in records]}


def validate_gltf(path):
    """Check generated glTF references, byte ranges, geometry, and rigs."""
    import json
    from urllib.parse import unquote
    import pygltflib as gl
    from pygltflib.validator import validate
    document = gl.GLTF2().load_json(str(path))
    validate(document)
    raw_document = json.loads(path.read_text())
    def index(value, collection, label):
        if not isinstance(value, int) or not 0 <= value < len(collection):
            raise ValueError(f'Invalid {label} index: {value}')
        return collection[value]
    buffers = []
    for buffer in document.buffers:
        content = (path.parent / unquote(buffer.uri)).read_bytes()
        if buffer.byteLength <= 0 or len(content) != buffer.byteLength:
            raise ValueError(f'Buffer size mismatch: {buffer.uri}')
        buffers.append(content)
    for view in document.bufferViews:
        content = index(view.buffer, buffers, 'buffer')
        if view.byteLength < 0 or (view.byteOffset or 0) < 0 or (view.byteOffset or 0) + view.byteLength > len(content):
            raise ValueError('Buffer view range exceeds buffer')
    shapes = {'SCALAR':1, 'VEC2':2, 'VEC3':3, 'VEC4':4, 'MAT2':4, 'MAT3':9, 'MAT4':16}
    components = {5120:('b',1),5121:('B',1),5122:('h',2),5123:('H',2),5125:('I',4),5126:('f',4)}
    for accessor in document.accessors:
        view = index(accessor.bufferView, document.bufferViews, 'accessor buffer view')
        if accessor.sparse: raise ValueError('Generated accessors must be dense')
        if accessor.type not in shapes or accessor.componentType not in components:
            raise ValueError('Invalid accessor format')
        width = shapes[accessor.type]
        size = components[accessor.componentType][1]
        stride = view.byteStride or width * size
        offset = accessor.byteOffset or 0
        if accessor.count <= 0 or offset < 0 or offset % size or ((view.byteOffset or 0) + offset) % size:
            raise ValueError('Invalid accessor count or alignment')
        if stride < width * size or stride % size:
            raise ValueError('Invalid accessor stride')
        if offset + (accessor.count-1)*stride + width*size > view.byteLength:
            raise ValueError('Accessor range exceeds buffer view')
        if (accessor.min and len(accessor.min) != width) or (accessor.max and len(accessor.max) != width):
            raise ValueError('Invalid accessor bounds width')
    def rows(accessor_index):
        accessor = index(accessor_index,document.accessors,'accessor')
        view = document.bufferViews[accessor.bufferView]
        code,size = components[accessor.componentType]
        width = shapes[accessor.type]
        stride = view.byteStride or width*size
        content = buffers[view.buffer]
        start = (view.byteOffset or 0)+(accessor.byteOffset or 0)
        for row in range(accessor.count):
            values = struct.unpack_from('<'+str(width)+code,content,start+row*stride)
            if any(not math.isfinite(value) for value in values): raise ValueError('Non-finite accessor value')
            yield values
    # Check float payloads, including mesh attributes and inverse bind matrices.
    for i,accessor in enumerate(document.accessors):
        if accessor.componentType == 5126:
            for row in rows(i): pass
    for image in document.images:
        if image.uri is not None and not (path.parent / unquote(image.uri)).is_file():
            raise ValueError(f'Missing texture: {image.uri}')
        if image.bufferView is not None: index(image.bufferView,document.bufferViews,'image buffer view')
    for texture in document.textures:
        index(texture.source,document.images,'texture image')
        if texture.sampler is not None: index(texture.sampler,document.samplers,'texture sampler')
    for material in document.materials:
        if material.pbrMetallicRoughness:
            pbr = material.pbrMetallicRoughness
            if len(pbr.baseColorFactor) != 4 or any(not math.isfinite(v) or not 0 <= v <= 1 for v in (*pbr.baseColorFactor,pbr.metallicFactor,pbr.roughnessFactor)):
                raise ValueError('Invalid PBR material factor')
        if len(material.emissiveFactor) != 3 or any(not math.isfinite(v) or not 0 <= v <= 1 for v in material.emissiveFactor): raise ValueError('Invalid emissive factor')
        infos = [material.normalTexture,material.occlusionTexture,material.emissiveTexture]
        if material.pbrMetallicRoughness:
            infos.extend((material.pbrMetallicRoughness.baseColorTexture,material.pbrMetallicRoughness.metallicRoughnessTexture))
        for info in infos:
            if info is not None: index(info.index,document.textures,'material texture')
    for mesh in document.meshes:
        if not mesh.primitives: raise ValueError('Mesh has no primitives')
        target_counts = set()
        for primitive in mesh.primitives:
            attributes = {name:value for name,value in vars(primitive.attributes).items() if isinstance(value,int)}
            position = index(attributes.get('POSITION'),document.accessors,'position accessor')
            if position.type != 'VEC3' or position.componentType != 5126: raise ValueError('Invalid position accessor')
            for name,value in attributes.items():
                attribute = index(value,document.accessors,'attribute accessor')
                if attribute.count != position.count: raise ValueError('Attribute count mismatch')
                expected = 'VEC2' if name.startswith('TEXCOORD_') else {'NORMAL':'VEC3','TANGENT':'VEC4','JOINTS_0':'VEC4','WEIGHTS_0':'VEC4'}.get(name)
                if expected and attribute.type != expected: raise ValueError(f'Invalid {name} shape')
                if name.startswith('JOINTS_') and attribute.componentType not in (5121,5123): raise ValueError('Invalid joint component type')
            count = position.count
            if primitive.indices is not None:
                indices = index(primitive.indices,document.accessors,'primitive indices')
                if indices.type != 'SCALAR' or indices.componentType not in (5121,5123,5125): raise ValueError('Invalid index accessor')
                count = indices.count
                if any(row[0] >= position.count for row in rows(primitive.indices)): raise ValueError('Primitive vertex index exceeds vertex count')
            mode = primitive.mode if primitive.mode is not None else 4
            if mode not in range(7) or (mode == 1 and count % 2) or (mode == 4 and count % 3): raise ValueError('Invalid primitive topology')
            if (mode in (2,3) and count < 2) or (mode in (5,6) and count < 3): raise ValueError('Insufficient primitive vertices')
            if primitive.material is not None: index(primitive.material,document.materials,'primitive material')
            target_counts.add(len(primitive.targets or []))
            for target in primitive.targets or []:
                for semantic,value in target.items():
                    accessor = index(value,document.accessors,'morph accessor')
                    if semantic not in ('POSITION','NORMAL','TANGENT') or accessor.type != 'VEC3' or accessor.componentType != 5126 or accessor.count != position.count:
                        raise ValueError('Invalid morph target')
        if len(target_counts) > 1: raise ValueError('Morph count differs between primitives')
        if mesh.weights and target_counts and len(mesh.weights) != next(iter(target_counts)): raise ValueError('Mesh morph weight count mismatch')
    parents = {}
    for i,node in enumerate(document.nodes):
        if node.mesh is not None: index(node.mesh,document.meshes,'node mesh')
        for child in node.children:
            index(child,document.nodes,'child node')
            if child in parents: raise ValueError('Node has multiple parents')
            parents[child] = i
        weights = raw_document['nodes'][i].get('weights',[])
        node.weights = weights
        if weights:
            mesh = index(node.mesh,document.meshes,'morph node mesh')
            if len(weights) != len(mesh.weights) or any(not math.isfinite(v) for v in weights): raise ValueError('Node morph weight count mismatch')
        if node.skin is not None:
            skin = index(node.skin,document.skins,'node skin')
            if node.mesh is None: raise ValueError('Skinned node has no mesh')
            for primitive in document.meshes[node.mesh].primitives:
                joints = primitive.attributes.JOINTS_0
                weights_attribute = primitive.attributes.WEIGHTS_0
                if joints is None or weights_attribute is None: raise ValueError('Skinned primitive lacks joints or weights')
                if any(j >= len(skin.joints) for row in rows(joints) for j in row): raise ValueError('Skin joint attribute exceeds joint count')
    for i in range(len(document.nodes)):
        seen = set()
        parent = i
        while parent in parents:
            if parent in seen: raise ValueError('Node hierarchy cycle')
            seen.add(parent)
            parent = parents[parent]
    for skin in document.skins:
        if not skin.joints: raise ValueError('Skin has no authored joints')
        for joint in skin.joints: index(joint,document.nodes,'skin joint')
        if skin.skeleton is not None: index(skin.skeleton,document.nodes,'skin skeleton')
        if skin.inverseBindMatrices is not None:
            accessor = index(skin.inverseBindMatrices,document.accessors,'inverse bind accessor')
            if accessor.type != 'MAT4' or accessor.componentType != 5126 or accessor.count != len(skin.joints): raise ValueError('Skin bind pose count mismatch')
    for scene in document.scenes:
        for root in scene.nodes:
            index(root,document.nodes,'scene root')
            if root in parents: raise ValueError('Scene root has a parent')
    if document.scene is not None: index(document.scene,document.scenes,'default scene')
    return document


def export_gltf(roots, output: Path) -> dict:
    """Roots are (Source, selected Transform readers)."""
    source, readers = roots
    return GltfExporter(source, output).export(readers)
