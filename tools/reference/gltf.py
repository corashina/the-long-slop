"""Export selected Unity hierarchies through pygltflib."""
import hashlib
import math
import struct
from pathlib import Path

from .source import key, source_id, vector
from .materials import recover_material, export_material_textures, material_source_evidence
from .visibility import renderer_visible, recover_startup_inactive_nodes


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
        self.material_records = []
        self.material_dependencies = {}

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

    def record_texture(self, entry):
        gl = self.gl
        settings = entry['sampler']
        identity = entry['file']
        if identity not in self.textures:
            sampler_key = tuple(settings.values())
            if sampler_key not in self.samplers:
                wrapping = {0: 10497, 1: 33071, 2: 33648, 3: 33071}
                mag, minimum = {0: (9728, 9984), 1: (9729, 9985), 2: (9729, 9987)}[settings['filter']]
                if settings['mip_count'] <= 1: minimum = mag
                self.samplers[sampler_key] = len(self.document.samplers)
                self.document.samplers.append(gl.Sampler(wrapS=wrapping[settings['wrap_u']],
                    wrapT=wrapping[settings['wrap_v']], magFilter=mag, minFilter=minimum,
                    extras={'unity_aniso': settings['aniso'], 'unity_mip_bias': settings['mip_bias'],
                            'source_mips': entry['mips']}))
            image = len(self.document.images)
            self.document.images.append(gl.Image(uri=entry['file'].removeprefix('reference/'),
                extras={'source': entry['source'], 'color_space': entry['color_space'],
                        'normal_encoding': entry['encoding']}))
            self.textures[identity] = len(self.document.textures)
            self.document.textures.append(gl.Texture(source=image, sampler=self.samplers[sampler_key]))
        if 'KHR_texture_transform' not in self.document.extensionsUsed:
            self.document.extensionsUsed.append('KHR_texture_transform')
        return {'index': self.textures[identity], 'texCoord': entry['uv_channel'],
            'extensions': {'KHR_texture_transform': {'scale': entry['scale'], 'offset': entry['offset']}}}

    def material(self, reader, color_override=None, condition=None):
        if reader is None: raise ValueError('Required renderer material has no source')
        import json
        identity = key(reader), tuple(color_override[c] for c in ('r', 'g', 'b', 'a')) if color_override is not None else None, json.dumps(condition, sort_keys=True)
        if identity in self.materials: return self.materials[identity]
        record = recover_material(self.source, reader, color_override, condition)
        material_source_evidence(self.source, reader, record)
        dependencies = export_material_textures(self.source, reader, record, self.output.parent)
        self.material_dependencies.update({entry['file']: entry for entry in dependencies})
        gl = self.gl; properties = record['properties']; floats = properties['floats']
        mode = record['state']['alpha_mode']
        material = gl.Material(name=self.source.read(reader)['m_Name'],
            alphaMode={'opaque': 'OPAQUE', 'mask': 'MASK', 'blend': 'BLEND',
                       'premultiply': 'BLEND', 'refract': 'BLEND'}[mode],
            alphaCutoff=floats['_Cutoff'] if mode == 'mask' else None,
            doubleSided=record['state']['cull'] == 0,
            pbrMetallicRoughness=gl.PbrMetallicRoughness(baseColorFactor=record['color'],
                metallicFactor=floats.get('_Metallic', 0.), roughnessFactor=1.-floats.get('_Glossiness', .5)),
            extras={'source': record['source'], 'source_material_record': len(self.material_records)})
        for entry in properties['textures']:
            info = self.record_texture(entry)
            if entry['slot'] == '_MainTex': material.pbrMetallicRoughness.baseColorTexture = gl.TextureInfo(**info)
            elif entry['slot'] == '_BumpMap': material.normalTexture = gl.NormalMaterialTexture(scale=floats.get('_BumpScale', 1.), **info)
            elif entry['slot'] == '_EmissionMap': material.emissiveTexture = gl.TextureInfo(**info)
            elif entry['slot'] == '_OcclusionMap': material.occlusionTexture = gl.OcclusionTextureInfo(strength=floats.get('_OcclusionStrength', 1.), **info)
        if '_EMISSION' in record['keywords']:
            material.emissiveFactor = properties['colors']['_EmissionColor'][:3]
        if record['shader_family'] == 'unlit_color':
            material.extensions['KHR_materials_unlit'] = {}
            if 'KHR_materials_unlit' not in self.document.extensionsUsed: self.document.extensionsUsed.append('KHR_materials_unlit')
        index = len(self.document.materials)
        self.materials[identity] = index; record['gltf_material'] = index
        self.material_records.append(record); self.document.materials.append(material)
        self.material_report.append({'source': record['source'], 'shader': record['shader_family'],
            'keywords': record['keywords'], 'rule': 'Source material record controls final binding.'})
        return index

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

    def mesh(self, mesh_reader, material_readers, color_override=None, condition=None):
        import json
        identity = key(mesh_reader), tuple(key(r) if r else None for r in material_readers), tuple(color_override[c] for c in ('r', 'g', 'b', 'a')) if color_override is not None else None, json.dumps(condition, sort_keys=True)
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
            material = self.material(material_readers[index],color_override,condition)
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
        condition_active = getattr(self.source, 'condition_active', {})
        effective_active = {}
        for record in records:
            identity = key(record['reader'])
            local_active = condition_active.get(identity, bool(self.source.read(record['game_object'])['m_IsActive']))
            parent_active = effective_active[key(record['parent'])] if record['parent'] else True
            record['active'] = parent_active and (local_active or record['parent'] is None)
            effective_active[identity] = record['active']
        inactive_nodes = recover_startup_inactive_nodes(self.source, records)
        for record in records:
            identity = key(record['reader'])
            if identity in inactive_nodes or (record['parent'] and key(record['parent']) in inactive_nodes):
                inactive_nodes.add(identity)
                record['active'] = False
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
                component_classes={self.source.class_name(r) for r in record['components'] if r.type.name=='MonoBehaviour'}
                render_enabled=renderer_visible(bool(renderer_data['m_Enabled']), component_classes) and key(renderer) not in hidden_lods and key(reader) not in inactive_nodes
                if component_classes.intersection({'TextMeshPro','TMP_SubMesh'}):
                    self.material_report.append({'source':source_id(renderer),
                        'adaptation':'Omitted obsolete editor TMP glyphs. Seeded plate text requires glyph regeneration.'})
                node.extras['renderer_enabled'] = render_enabled
                owner = mesh_filter or renderer
                mesh_reader = getattr(self.source, 'condition_meshes', {}).get(key(renderer)) or self.source.resolve(owner, self.source.read(owner)['m_Mesh'])
                if mesh_reader and record['active'] and render_enabled:
                    materials=[self.source.resolve(renderer,p) for p in renderer_data['m_Materials']]
                    selected=getattr(self.source,'condition_materials',{}).get(key(renderer))
                    if selected:materials=[selected for _ in materials]
                    color=getattr(self.source,'condition_colors',{}).get(key(renderer))
                    condition=getattr(self.source,'condition_renderers',{}).get(key(renderer))
                    if condition:node.extras['condition']=condition
                    mirror = getattr(self.source, 'live_mirror_renderers', {}).get(key(renderer))
                    if mirror:
                        materials = [mirror]
                    if not materials or any(r is None for r in materials):
                        raise ValueError(f'{source_id(renderer)} m_Materials: unresolved startup material')
                    if len(materials) < len(self.geometry(mesh_reader)[2]):
                        # Unity reuses the last renderer material for remaining submeshes.
                        materials += [materials[-1]] * (len(self.geometry(mesh_reader)[2]) - len(materials))
                    node.mesh = self.mesh(mesh_reader,materials,color,condition)
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
        self.document.scenes.append(gl.Scene(nodes=root_nodes))
        self.document.scene = 0
        self.document.buffers.append(gl.Buffer(uri=self.output.with_suffix('.bin').name,byteLength=len(self.binary)))
        self.output.with_suffix('.bin').write_bytes(self.binary)
        self.document.save_json(str(self.output))
        validate_gltf(self.output)
        paths={buffer.uri for buffer in self.document.buffers}|{image.uri for image in self.document.images}
        dependencies=[{'file':f'reference/{path}','sha256':hashlib.sha256((self.output.parent/path).read_bytes()).hexdigest()} for path in sorted(paths)]
        dependencies = list({entry['file']: entry for entry in [*dependencies, *self.material_dependencies.values()]}.values())
        return {'path': self.output.name, 'sha256': hashlib.sha256(self.output.read_bytes()).hexdigest(),
                'dependencies':dependencies, 'materials': self.material_records,
                'node_count':len(records), 'mesh_count':len(self.meshes), 'material_adaptations':self.material_report,
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
