"""Critical model conversion checks."""
import json
import struct
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace as NS
from unittest.mock import patch
from urllib.parse import unquote

from tools.reference.library.types import Catalog, ObjectId, ObjectRecord
from tools.reference.library.models import export_models, collect_model_hierarchy
from tools.reference.gltf import validate_gltf


def ptr(identity=0):
    return {'m_FileID': 0, 'm_PathID': identity}


class Fixture:
    def __init__(self, file='test.assets'):
        self.file=file
        self.readers = {}
        self.data = {}
        self.handlers = {}
        self.add(1, 'GameObject', {'m_Name': 'root', 'm_IsActive': False, 'm_Layer': 0,
                                 'm_Component': [{'component': ptr(2)}, {'component': ptr(3)}]})
        self.add(2, 'Transform', self.transform(1))
        self.add(3, 'SkinnedMeshRenderer', {'m_Enabled': False, 'm_Mesh': ptr(4), 'm_Bones': [ptr(2)],
            'm_RootBone': ptr(2), 'm_Materials': [], 'm_BlendShapeWeights': [50.]})
        matrix = {f'e{r}{c}': float(r == c) for r in range(4) for c in range(4)}
        matrix['e23'] = 3.
        self.add(4, 'Mesh', {'m_Name': 'mesh', 'm_VertexData': {'m_VertexCount': 3},
            'm_BindPose': [matrix], 'm_LocalAABB': {'m_Center': {'x': 0, 'y': 0, 'z': 1}},
            'm_Shapes': {'vertices': [{'index': 1, 'vertex': {'x': 1., 'y': 0., 'z': 2.},
            'normal': {'x': 0., 'y': 0., 'z': 1.}, 'tangent': {'x': 0., 'y': 0., 'z': 1.}}],
            'shapes': [{'firstVertex': 0, 'vertexCount': 1, 'hasNormals': True, 'hasTangents': True}],
            'channels': [{'name': 'bend', 'nameHash': 1, 'frameIndex': 0, 'frameCount': 1}],
            'fullWeights': [100.]}})
        sub = NS(firstByte=0, indexCount=3, topology=0, baseVertex=0)
        self.handlers[4] = NS(src=NS(m_SubMeshes=[sub]), m_Vertices=[(0,0,1),(1,0,1),(0,1,1)],
            m_Normals=[(0,0,1)]*3, m_Tangents=[(1,0,1,1)]*3, m_Colors=[(1,0,0,1)]*3,
            m_BoneIndices=[(0,0,0,0)]*3, m_BoneWeights=[(1.,0.,0.,0.)]*3,
            m_IndexBuffer=[0,1,2], m_Use16BitIndices=True, process=lambda: None,
            **{f'm_UV{i}': [(0.,0.)]*3 for i in range(8)})

    def transform(self, go, father=0, children=()):
        return {'m_GameObject': ptr(go), 'm_Father': ptr(father), 'm_Children': [ptr(c) for c in children],
                'm_LocalPosition': {'x': 1, 'y': 2, 'z': 3},
                'm_LocalRotation': {'x': 0, 'y': 0, 'z': 0, 'w': 1},
                'm_LocalScale': {'x': -1, 'y': 2, 'z': 1}}

    def add(self, identity, kind, data):
        self.readers[identity] = NS(path_id=identity, type=NS(name=kind),
            assets_file=NS(source_container_path=self.file))
        self.data[identity] = data

    def get(self, file, identity):
        return self.readers[identity]

    def resolve(self, owner, pointer):
        if not pointer['m_PathID']: return None
        if pointer['m_PathID'] not in self.readers: raise ValueError('Missing source test.assets:99')
        return self.readers[pointer['m_PathID']]

    def read(self, reader): return self.data[reader.path_id]
    read_record = read
    def object(self, reader): return reader
    def components(self, reader):
        go = self.resolve(reader, self.read(reader)['m_GameObject'])
        return go, [self.resolve(go, c['component']) for c in self.read(go)['m_Component']]
    def catalog(self):
        objects = {ObjectId(self.file, i): ObjectRecord(ObjectId(self.file, i), r.type.name, 0, 0, 0, '')
                   for i,r in self.readers.items()}
        return Catalog(objects, {}, {i: () for i in objects})


def values(doc, root, index):
    a = doc.accessors[index]
    v = doc.bufferViews[a.bufferView]
    raw = (root / doc.buffers[v.buffer].uri).read_bytes()
    widths = {'SCALAR':1, 'VEC2':2, 'VEC3':3, 'VEC4':4, 'MAT4':16}
    code = {5126:'f',5125:'I',5123:'H'}[a.componentType]
    return struct.unpack_from('<'+str(a.count*widths[a.type])+code, raw, (v.byteOffset or 0)+(a.byteOffset or 0))


class ModelTests(unittest.TestCase):
    def run_export(self, source, root):
        with patch('tools.reference.library.models.MeshHandler', side_effect=lambda r: source.handlers[r.path_id]):
            return export_models(source, source.catalog(), root)

    def test_signed_scale_skin_and_winding(self):
        source = Fixture()
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            exports = self.run_export(source, root)
            package = next(e for e in exports if e.source.path_id == 1)
            path = root / next(f.file for f in package.files if f.file.endswith('.gltf'))
            doc = validate_gltf(path)
            node = doc.nodes[0]
            primitive = doc.meshes[node.mesh].primitives[0]
            self.assertEqual(node.translation, [1,2,-3])
            self.assertEqual(node.scale, [-1,2,1])
            self.assertEqual(values(doc,path.parent,primitive.indices), (0,2,1))
            self.assertEqual(values(doc,path.parent,primitive.attributes.TANGENT)[:4], (1.,0.,-1.,-1.))
            self.assertEqual(values(doc,path.parent,doc.skins[node.skin].inverseBindMatrices)[14], -3.)
            self.assertEqual(doc.skins[node.skin].joints, [0])
            self.assertEqual(values(doc,path.parent,primitive.attributes.JOINTS_0)[:4], (0,0,0,0))
            self.assertEqual(len(doc.buffers), 1)

    def test_inactive_nodes_and_all_lods_retained(self):
        source = Fixture()
        source.add(5, 'LODGroup', {'m_LODs': [{'renderers':[{'renderer':ptr(3)}]}, {'renderers':[{'renderer':ptr(8)}]}]})
        source.add(6,'Transform',source.transform(7,2))
        source.add(7,'GameObject',{'m_Name':'low','m_IsActive':False,'m_Layer':0,'m_Component':[{'component':ptr(i)} for i in (6,8,9)]})
        source.add(8,'MeshRenderer',{'m_Enabled':False,'m_Materials':[]})
        source.add(9,'MeshFilter',{'m_Mesh':ptr(4)})
        source.data[2]['m_Children'] = [ptr(6)]
        source.data[1]['m_Component'].append({'component':ptr(5)})
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            exports = self.run_export(source, root)
            package = next(e for e in exports if e.source.path_id == 1)
            doc = validate_gltf(root / next(f.file for f in package.files if f.file.endswith('.gltf')))
            self.assertIsNotNone(doc.nodes[0].mesh)
            self.assertFalse(doc.nodes[0].extras['active'])
            self.assertFalse(doc.nodes[0].extras['renderers'][0]['enabled'])
            self.assertEqual(len(doc.nodes[0].extras['lod_groups'][0]['lods']), 2)
            self.assertIsNotNone(doc.nodes[1].mesh)
            self.assertEqual(doc.nodes[0].mesh,doc.nodes[1].mesh)
            self.assertEqual(len(doc.buffers),1)

    def test_empty_mesh_has_no_fabricated_geometry(self):
        source = Fixture()
        source.handlers[4].m_Vertices = []
        source.data[4]['m_VertexData']['m_VertexCount'] = 0
        source.data[3]['m_Bones'] = []
        with tempfile.TemporaryDirectory() as temp:
            exports = self.run_export(source, Path(temp))
            mesh = next(e for e in exports if e.source.path_id == 4)
            self.assertEqual(mesh.status, 'source_empty')
            self.assertEqual(mesh.files, ())

    def test_external_bone_and_morph_links(self):
        source = Fixture()
        for go,t,parent in [(10,11,0),(12,13,11)]:
            source.add(go,'GameObject',{'m_Name':'bone','m_IsActive':True,'m_Layer':0,'m_Component':[{'component':ptr(t)}]})
            source.add(t,'Transform',source.transform(go,parent))
        source.data[3]['m_Bones'] = [ptr(13)]
        source.data[3]['m_RootBone'] = ptr(11)
        nodes = collect_model_hierarchy(source,ObjectId('test.assets',2),source.catalog())
        self.assertEqual({n.path_id for n in nodes}, {2,11,13})
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            exports = self.run_export(source,root)
            package = next(e for e in exports if e.source.path_id == 1)
            path = root / next(f.file for f in package.files if f.file.endswith('.gltf'))
            doc = validate_gltf(path)
            primitive = doc.meshes[doc.nodes[0].mesh].primitives[0]
            target = primitive.targets[0]
            self.assertEqual(values(doc,path.parent,target['POSITION'])[3:6], (1.,0.,-2.))
            self.assertEqual(doc.nodes[0].weights, [.5])
            self.assertEqual(doc.nodes[doc.skins[doc.nodes[0].skin].joints[0]].extras['source']['path_id'], 13)

    def test_missing_bone_and_bad_bind_pose_fail(self):
        for invalid in ('bone','bind'):
            source = Fixture()
            if invalid == 'bone': source.data[3]['m_Bones'] = [ptr(99)]
            else: source.data[4]['m_BindPose'] = []
            with tempfile.TemporaryDirectory() as temp:
                with self.assertRaisesRegex(ValueError, 'test.assets'):
                    self.run_export(source,Path(temp))

    def test_validator_rejects_ranges_skin_and_morph_links(self):
        source = Fixture()
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            exports = self.run_export(source, root)
            package = next(e for e in exports if e.source.path_id == 1)
            path = root / next(f.file for f in package.files if f.file.endswith('.gltf'))
            raw = path.read_text()
            for kind in ('range','skin','morph','topology','node'):
                data = json.loads(raw)
                if kind == 'range': data['accessors'][0]['count'] = 999999
                elif kind == 'skin': data['skins'][0]['joints'] = [99999]
                elif kind == 'morph': data['meshes'][0]['primitives'][0]['targets'][0]['POSITION'] = 99999
                elif kind == 'topology': data['meshes'][0]['primitives'][0]['mode'] = 1
                elif kind == 'node': data['nodes'][0]['children'] = [0]
                path.write_text(json.dumps(data))
                with self.assertRaises(ValueError, msg=kind): validate_gltf(path)
            path.write_text(raw)

    def test_texture_binding_uses_shared_canonical_files(self):
        from PIL import Image
        source = Fixture()
        source.add(20,'Material',{'m_Name':'paint','m_Shader':ptr(),
            'm_SavedProperties':{'m_Floats':[], 'm_Colors':[], 'm_TexEnvs':[
                ['_MainTex',{'m_Texture':ptr(21),'m_Scale':{'x':2.,'y':3.},'m_Offset':{'x':.1,'y':.2}}]]}})
        source.add(21,'Texture2D',{'m_CompleteImageSize':8,'m_MipCount':1,
            'm_TextureSettings':{'m_WrapU':0,'m_WrapV':1,'m_FilterMode':1,'m_Aniso':1,'m_MipBias':0}})
        source.data[3]['m_Materials'] = [ptr(20)]
        image = Image.new('RGBA',(1,2)); image.putdata([(255,0,0,255),(0,0,255,255)])
        source_object = source.object
        source.object = lambda reader: NS(image=image) if reader.path_id == 21 else source_object(reader)
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            exports = self.run_export(source,root)
            package = next(e for e in exports if e.source.path_id == 1)
            path = root / next(f.file for f in package.files if f.file.endswith('.gltf'))
            doc = validate_gltf(path)
            self.assertEqual(doc.materials[0].pbrMetallicRoughness.baseColorTexture.index,0)
            self.assertNotIn('task-3',doc.images[0].uri)
            with Image.open(root/'images/test.assets/21/source.png') as original:
                self.assertEqual(original.getpixel((0,0)),(255,0,0,255))
            with Image.open(path.parent/doc.images[0].uri) as derived:
                self.assertEqual(derived.getpixel((0,0)),(0,0,255,255))

    def test_single_influence_skin_retains_source_weights(self):
        source = Fixture()
        source.handlers[4].m_BoneIndices = [(0,)]*3
        source.handlers[4].m_BoneWeights = [(1.,)]*3
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp)
            exports=self.run_export(source,root)
            package=next(e for e in exports if e.source.path_id==1)
            path=root/next(f.file for f in package.files if f.file.endswith('.gltf'))
            doc=validate_gltf(path)
            attribute=doc.meshes[0].primitives[0].attributes.WEIGHTS_0
            self.assertEqual(values(doc,path.parent,attribute)[:4],(1.,0.,0.,0.))

    def test_implicit_single_bone_weight_layout(self):
        source=Fixture()
        source.handlers[4].m_BoneIndices=[(0,)]*3
        source.handlers[4].m_BoneWeights=None
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp)
            exports=self.run_export(source,root)
            mesh=next(e for e in exports if e.source.path_id==4)
            self.assertFalse(mesh.details['skin_weights'])
            self.assertEqual(mesh.details['skin_weight_encoding'],'implicit_one_bone')
            doc=validate_gltf(root/next(f.file for f in mesh.files if f.file.endswith('.gltf')))
            self.assertEqual(values(doc,(root/mesh.files[0].file).parent,doc.meshes[0].primitives[0].attributes.WEIGHTS_0)[:4],(1.,0.,0.,0.))

    def test_null_mesh_renderer_retains_null_bone_slots(self):
        source=Fixture()
        source.data[3]['m_Mesh']=ptr()
        source.data[3]['m_Bones']=[ptr()]*5
        source.data[3]['m_RootBone']=ptr()
        with tempfile.TemporaryDirectory() as temp:
            exports=self.run_export(source,Path(temp))
            package=next(e for e in exports if e.source.path_id==1)
            renderer=package.details['renderers'][0]
            self.assertEqual(renderer['rig_status'],'source_mesh_null')
            self.assertEqual(renderer['bones'],[None]*5)
            self.assertEqual(package.details['skin_count'],0)

    def test_static_mesh_without_bind_poses(self):
        source=Fixture()
        source.readers[3].type.name='MeshRenderer'
        source.add(24,'MeshFilter',{'m_Mesh':ptr(4)})
        source.data[1]['m_Component'].append({'component':ptr(24)})
        source.data[4]['m_BindPose']=[]
        source.handlers[4].m_BoneWeights=None
        source.handlers[4].m_BoneIndices=None
        with tempfile.TemporaryDirectory() as temp:
            exports=self.run_export(source,Path(temp))
            package=next(e for e in exports if e.source.path_id==1)
            self.assertEqual(package.details['skin_count'],0)
            self.assertEqual(package.details['mesh_count'],1)

    def test_authored_mesh_with_unbound_skin_keeps_null_slots(self):
        source=Fixture()
        source.data[3]['m_Bones']=[ptr()]
        source.data[3]['m_RootBone']=ptr()
        with tempfile.TemporaryDirectory() as temp:
            exports=self.run_export(source,Path(temp))
            package=next(e for e in exports if e.source.path_id==1)
            self.assertEqual(package.runtime,'engine_dependent')
            self.assertEqual(package.details['skin_count'],0)
            self.assertEqual(package.details['renderers'][0]['rig_status'],'source_unbound_skin')
            self.assertEqual(package.details['renderers'][0]['bones'],[None])
            self.assertEqual(package.details['renderers'][0]['bind_pose_count'],1)
        source.data[3]['m_RootBone']=ptr(2)
        with tempfile.TemporaryDirectory() as temp:
            with self.assertRaisesRegex(ValueError,'test.assets'):
                self.run_export(source,Path(temp))

    def test_shared_buffer_uri_encodes_source_path(self):
        source=Fixture('test assets')
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp)
            exports=self.run_export(source,root)
            package=next(e for e in exports if e.source.path_id==1)
            path=root/next(f.file for f in package.files if f.file.endswith('.gltf'))
            doc=validate_gltf(path)
            self.assertIn('test%20assets',doc.buffers[0].uri)
            self.assertNotIn(' ',doc.buffers[0].uri)


    def material_fixture(self, shader='Standard'):
        from PIL import Image
        source = Fixture()
        source.add(27, 'Shader', {'m_ParsedForm': {'m_Name': shader}})
        maps = []
        for name in ('_MainTex', '_BumpMap', '_MetallicGlossMap', '_OcclusionMap', '_EmissionMap'):
            maps.append([name, {'m_Texture': ptr(21), 'm_Scale': {'x': 3., 'y': 4.} if name == '_MainTex' else {'x': 1., 'y': 2.},
                                'm_Offset': {'x': .2, 'y': .3} if name == '_MainTex' else {'x': .7, 'y': .8}}])
        source.add(20, 'Material', {'m_Name': 'paint', 'm_Shader': ptr(27),
            'm_SavedProperties': {'m_Floats': [['_GlossMapScale', 1.]], 'm_Colors': [], 'm_TexEnvs': maps}})
        source.add(21, 'Texture2D', {'m_CompleteImageSize': 8, 'm_MipCount': 1, 'm_TextureFormat': 29,
            'm_TextureSettings': {'m_WrapU': 0, 'm_WrapV': 1, 'm_FilterMode': 1, 'm_Aniso': 1, 'm_MipBias': 0}})
        source.data[3]['m_Materials'] = [ptr(20)]
        image = Image.new('RGBA', (1, 2))
        image.putdata([(192, 64, 37, 255), (255, 64, 37, 192)])
        original = source.object
        source.object = lambda r: NS(image=image) if r.path_id == 21 else original(r)
        return source

    def material_document(self, source, root):
        exports = self.run_export(source, root)
        package = next(e for e in exports if e.source.path_id == 1)
        path = root / next(f.file for f in package.files if f.file.endswith('.gltf'))
        return exports, path, validate_gltf(path)

    def test_standard_packed_normal_reconstructs_rgb(self):
        from PIL import Image
        import math
        source = self.material_fixture()
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            _, path, doc = self.material_document(source, root)
            normal = doc.images[doc.textures[doc.materials[0].normalTexture.index].source]
            with Image.open(path.parent / unquote(normal.uri)) as image:
                z = math.sqrt(1 - (2*192/255-1)**2 - (2*64/255-1)**2)
                expected = (192, 64, round((z+1)*127.5))
                self.assertEqual(image.getpixel((0, 0)), expected)
                self.assertEqual(image.getpixel((0, 1)), expected)
            with Image.open(root / 'images/test.assets/21/source.png') as image:
                self.assertEqual(image.getpixel((0, 1)), (255, 64, 37, 192))

    def test_standard_occlusion_uses_green(self):
        from PIL import Image
        source = self.material_fixture()
        with tempfile.TemporaryDirectory() as temp:
            _, path, doc = self.material_document(source, Path(temp))
            occlusion = doc.images[doc.textures[doc.materials[0].occlusionTexture.index].source]
            with Image.open(path.parent / unquote(occlusion.uri)) as image:
                self.assertEqual(image.convert('RGB').getpixel((0, 0))[0], 64)

    def test_standard_primary_maps_use_main_uv_and_retain_native_uv(self):
        source = self.material_fixture()
        with tempfile.TemporaryDirectory() as temp:
            _, _, doc = self.material_document(source, Path(temp))
            material = doc.materials[0]
            infos = [material.pbrMetallicRoughness.baseColorTexture, material.normalTexture,
                     material.pbrMetallicRoughness.metallicRoughnessTexture, material.occlusionTexture, material.emissiveTexture]
            for info in infos:
                self.assertEqual(info.extensions['KHR_texture_transform'], {'scale': [3., 4.], 'offset': [.2, .3]})
            native = next(t for t in material.extras['textures'] if t['property'] == '_BumpMap')
            self.assertEqual(native['scale'], {'x': 1., 'y': 2.})
            self.assertEqual(native['offset'], {'x': .7, 'y': .8})

    def test_unrelated_shader_keeps_map_uv_and_role_receipts_are_deterministic(self):
        source = self.material_fixture('Custom/Other')
        with tempfile.TemporaryDirectory() as first, tempfile.TemporaryDirectory() as second:
            exports, _, doc = self.material_document(source, Path(first))
            material = doc.materials[0]
            self.assertEqual(material.emissiveTexture.extensions['KHR_texture_transform'], {'scale': [1., 2.], 'offset': [.7, .8]})
            self.assertIsNone(material.normalTexture)
            package = next(e for e in exports if e.source.path_id == 1)
            self.assertEqual(package.details['texture_limits'][0]['role'], 'normal')
            self.assertEqual(next(t for t in material.extras['textures'] if t['property'] == '_BumpMap')['source']['path_id'], 21)
            repeated, _, _ = self.material_document(source, Path(second))
            self.assertEqual([e.files for e in exports], [e.files for e in repeated])


    def test_standard_role_files_and_receipts_are_deterministic(self):
        source = self.material_fixture()
        with tempfile.TemporaryDirectory() as first, tempfile.TemporaryDirectory() as second:
            exports, _, doc = self.material_document(source, Path(first))
            self.assertEqual(len({image.uri for image in doc.images}), 5)
            self.assertEqual({image.extras['role'] for image in doc.images}, {'base_color', 'normal', 'metallic_roughness', 'occlusion', 'emission'})
            repeated, _, _ = self.material_document(source, Path(second))
            self.assertEqual([e.files for e in exports], [e.files for e in repeated])
