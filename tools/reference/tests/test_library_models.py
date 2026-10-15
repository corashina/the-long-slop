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
