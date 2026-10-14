import copy
import hashlib
import json
import os
import unittest
from pathlib import Path
from unittest.mock import patch
from tools.reference.library.reader import LibraryReader
from tools.reference.library.types import LibraryReceipt, ObjectId
from tools.reference.source import LibrarySource
from tools.reference.materials import recover_material, material_source_evidence


class PlayerMaterialTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        path = Path(os.environ['REFERENCE_LIBRARY_ROOT']) / 'manifest.json'
        raw = path.read_bytes(); metadata = json.loads(raw)
        cls.source = LibrarySource(LibraryReader(LibraryReceipt(path, hashlib.sha256(raw).hexdigest(),
            metadata['catalog_sha256'], metadata['source_manifest_sha256'], ())))

    def test_checked_player_materials_and_resources(self):
        for identity, distance, main, normal in [(57,.25,346,168),(59,.25,284,168),
                (61,0.3499999940395355,219,168),(75,0.3499999940395355,212,225)]:
            with self.subTest(material=identity):
                reader = self.source.get('sharedassets0.assets', identity)
                record = recover_material(self.source, reader)
                fixture=json.loads(Path('tests/common/player_materials.json').read_bytes())
                expected=next(r for r in fixture['materials'] if r['source']['path_id']==identity)
                self.assertEqual(record,{k:v for k,v in expected.items() if k not in ('asset','gltf_material')})
                self.assertEqual(record['shader_family'], 'player_model')
                self.assertEqual(record['properties']['floats']['_Dist'], distance)
                self.assertEqual(record['keywords'], [])
                self.assertEqual(record['state'], dict(alpha_mode='mask',cull=2,z_write=True,
                    z_test=4,src_blend=1,dst_blend=0,render_queue=-1))
                self.assertEqual(record['properties']['defaults'], {})
                self.assertEqual([(t['slot'],t['source']['path_id']) for t in record['properties']['textures']],
                    [('_MainTex',main),('_BumpMap',normal)])
                for texture in record['properties']['textures']:
                    self.source.library.texture(ObjectId(**texture['source']))
                material_source_evidence(self.source, reader, record)
                evidence = self.source.material_evidence[f'sharedassets0.assets:{identity}']
                self.assertEqual(evidence['shader'],dict(file='sharedassets0.assets',path_id=719))
                self.assertEqual(len(evidence['passes']),5)
                self.assertEqual(evidence['required_operations'][0]['operation'],'camera_distance_clip')
                self.assertIn('_BumpStrength',evidence['channels']['_BumpMap'])

    def test_rejects_missing_or_unknown_required_player_fields(self):
        reader = self.source.get('sharedassets0.assets',57)
        original = self.source.read(reader)
        shader_reader = self.source.get('sharedassets0.assets',719)
        for mutate, label in [
            (lambda d:d['m_SavedProperties']['m_Floats'].__setitem__(slice(None),
                [p for p in d['m_SavedProperties']['m_Floats'] if p[0]!='_Dist']), '_Dist'),
            (lambda d:dict(d['m_SavedProperties']['m_TexEnvs'])['_BumpMap']['m_Texture'].update(m_PathID=0),'_BumpMap'),
            (lambda d:dict(d['m_SavedProperties']['m_TexEnvs'])['_MainTex']['m_Texture'].update(m_PathID=999999999),'_MainTex'),
            (lambda d:d['m_ValidKeywords'].append('_NORMALMAP'),'keyword'),
            (lambda d:d['m_SavedProperties']['m_Floats'].append(('_UnknownRequired',1.)),'properties'),
        ]:
            data=copy.deepcopy(original); mutate(data)
            read=self.source.read
            with self.subTest(field=label),patch.object(self.source,'read',side_effect=lambda r: data if r is reader else read(r)):
                with self.assertRaisesRegex(ValueError,label):recover_material(self.source,reader)
        shader=copy.deepcopy(self.source.read(shader_reader))
        shader['m_ParsedForm']['m_PropInfo']['m_Props'][0]['m_Name']='_UnknownRequired'
        read=self.source.read
        with patch.object(self.source,'read',side_effect=lambda r: shader if r is shader_reader else read(r)):
            with self.assertRaisesRegex(ValueError,'properties'):recover_material(self.source,reader)

    def test_player_normal_multiplies_red_and_alpha(self):
        from PIL import Image
        from tools.reference.materials import decode_normal_rgba, decode_normal_image
        pixel=(128,160,255,192)
        normal=decode_normal_rgba(pixel,'rag')
        self.assertAlmostEqual(normal[0],2*(128/255)*(192/255)-1)
        self.assertNotEqual(normal,decode_normal_rgba(pixel,'ag'))
        image=decode_normal_image(Image.new('RGBA',(1,1),pixel),'rag')
        self.assertLess(abs(image.getpixel((0,0))[0] / 127.5 - 1 - normal[0]),2/255)

    def test_player_bump_uv_is_independent(self):
        reader=self.source.get('sharedassets0.assets',57)
        data=copy.deepcopy(self.source.read(reader))
        env=dict(data['m_SavedProperties']['m_TexEnvs'])['_BumpMap']
        env['m_Scale']={'x':2.,'y':3.};env['m_Offset']={'x':.1,'y':.2}
        read=self.source.read
        with patch.object(self.source,'read',side_effect=lambda r:data if r is reader else read(r)):
            textures=recover_material(self.source,reader)['properties']['textures']
        self.assertEqual(textures[0]['scale'],[1.,1.])
        self.assertEqual(textures[1]['scale'],[2.,3.])
        self.assertEqual(textures[1]['offset'],[.1,.2])

    def test_rejects_wrong_source_shader_and_unsupported_normal(self):
        reader=self.source.get('sharedassets0.assets',57)
        for identity, mutate, label in [
            (719, lambda d:d.update(m_Name='Standard'), 'family'),
            (168, lambda d:d.update(m_TextureFormat=4), 'encoding'),
        ]:
            target=self.source.get('sharedassets0.assets',identity)
            data=copy.deepcopy(self.source.read(target));mutate(data);read=self.source.read
            with self.subTest(source=identity),patch.object(self.source,'read',side_effect=lambda r:data if r is target else read(r)):
                with self.assertRaisesRegex(ValueError,label):recover_material(self.source,reader)
