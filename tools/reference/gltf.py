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
                text_component=next((r for r in record['components'] if r.type.name=='MonoBehaviour' and self.source.class_name(r) in {'TextMeshPro','TMP_SubMesh'}),None)
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
                            converted = []
                            signs = [1,1,-1,1]
                            for matrix in matrices:
                                converted.append(tuple(matrix[f'e{row}{column}']*signs[row]*signs[column] for column in range(4) for row in range(4)))
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
        # Cache all instrument material variants in the same imported glTF.
        for record in records:
            for component in record['components']:
                if component.type.name!='MonoBehaviour' or not component.parse_monobehaviour_head().m_Script.path_id:continue
                cname=self.source.class_name(component)
                if cname not in ('meterscript','mutatoscript','ledkijelzoscript','forgoszamlap'):continue
                data=self.source.read(component)
                pointers=[]
                if cname in ('meterscript','mutatoscript'):pointers=[data['OnM'],data['OffM']]
                elif cname=='ledkijelzoscript':pointers=[v[k] for v in data['leds'] for k in ('on','off')]
                else:pointers=[v[k] for v in data['szamok'] for k in ('MOn','MOff')]
                for ptr in pointers:
                    if ptr['m_PathID']:self.material(self.source.resolve(component,ptr))
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
                'material_count':len(self.document.materials), 'materials':{f'{identity[0][0]}:{identity[0][1]}':index for identity,index in self.materials.items() if identity[1] is None}, 'node_count':len(records), 'mesh_count':len(self.meshes), 'material_adaptations':self.material_report,
                'nodes':[{'source':source_id(r['reader']),'gltf_node':mapping[key(r['reader'])],
                    'parent':source_id(r['parent']) if r['parent'] else None,'hierarchy_path':r['path'],
                    'active':r['active'],'transform':{'translation':self.document.nodes[mapping[key(r['reader'])]].translation,
                        'rotation':self.document.nodes[mapping[key(r['reader'])]].rotation,'scale':self.document.nodes[mapping[key(r['reader'])]].scale}} for r in records]}


def validate_gltf(path):
    import pygltflib as gl
    from pygltflib.validator import validate
    document = gl.GLTF2().load_json(str(path))
    validate(document)
    for buffer in document.buffers:
        if (path.parent / buffer.uri).stat().st_size != buffer.byteLength:
            raise ValueError(f'Buffer size mismatch: {buffer.uri}')
    for image in document.images:
        if not (path.parent / image.uri).is_file():
            raise ValueError(f'Missing texture: {image.uri}')
    for node in document.nodes:
        if node.mesh is not None and not 0 <= node.mesh < len(document.meshes):
            raise ValueError('Invalid node mesh index')
        if any(not 0 <= child < len(document.nodes) for child in node.children):
            raise ValueError('Invalid child index')
    return document


def export_gltf(roots, output: Path) -> dict:
    """Roots are (Source, selected Transform readers)."""
    source, readers = roots
    return GltfExporter(source, output).export(readers)
