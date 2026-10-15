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

    def recover_with_mutation(self, material, target, mutate, **kwargs):
        reader = self.source.get('sharedassets0.assets', material)
        target_reader = self.source.get('sharedassets0.assets', target)
        data = copy.deepcopy(self.source.read(target_reader))
        mutate(data)
        read = self.source.read
        with patch.object(self.source, 'read', side_effect=lambda r: data if r is target_reader else read(r)):
            return recover_material(self.source, reader, **kwargs)

    def test_player_float_bounds(self):
        for field, maximum in [('_Dist', 10.), ('_Cutoff', 1.), ('_BumpStrength', 5.),
                ('_Glossiness', 1.), ('_Metallic', 1.)]:
            for value in [-1., maximum + 1., float('nan'), float('inf'), 0., maximum]:
                def mutate(data):
                    floats = data['m_SavedProperties']['m_Floats']
                    floats[:] = [(name, value if name == field else old) for name, old in floats]
                with self.subTest(field=field, value=value):
                    if 0 <= value <= maximum:
                        record = self.recover_with_mutation(57, 57, mutate)
                        self.assertEqual(record['properties']['floats'][field], value)
                    else:
                        with self.assertRaises(ValueError):
                            self.recover_with_mutation(57, 57, mutate)

    def test_player_saved_colors_and_metadata(self):
        for label, mutate in [
            ('unknown color', lambda d: d['m_SavedProperties']['m_Colors'].append(
                ('_UnknownRequired', dict(r=0., g=0., b=0., a=1.)))),
            ('missing emission', lambda d: d['m_SavedProperties']['m_Colors'].__setitem__(slice(None),
                [p for p in d['m_SavedProperties']['m_Colors'] if p[0] != '_EmissionColor'])),
            ('missing color', lambda d: d['m_SavedProperties']['m_Colors'].__setitem__(slice(None),
                [p for p in d['m_SavedProperties']['m_Colors'] if p[0] != '_Color'])),
            ('nonfinite color', lambda d: dict(d['m_SavedProperties']['m_Colors'])['_Color'].update(r=float('nan'))),
            ('unexpected red', lambda d: d['m_SavedProperties']['m_Floats'].append(('_Red', 0.))),
            ('instancing', lambda d: d.update(m_EnableInstancingVariants=True)),
            ('queue', lambda d: d.update(m_CustomRenderQueue=2000)),
            ('disabled pass', lambda d: d['disabledShaderPasses'].append('FORWARDBASE')),
        ]:
            with self.subTest(field=label), self.assertRaises(ValueError):
                self.recover_with_mutation(57, 57, mutate)
        with self.subTest(field='missing skin red'), self.assertRaises(ValueError):
            self.recover_with_mutation(61, 61, lambda d: d['m_SavedProperties']['m_Floats'].__setitem__(slice(None),
                [p for p in d['m_SavedProperties']['m_Floats'] if p[0] != '_Red']))
        with self.subTest(field='condition'), self.assertRaises(ValueError):
            recover_material(self.source, self.source.get('sharedassets0.assets', 57),
                condition=dict(source=dict(file='sharedassets0.assets', path_id=57), state=0, quality=1., range=None))

    def test_player_texture_contract(self):
        for material in (57, 59, 61, 75):
            for slot, wrong in [('_MainTex', 219 if material != 61 else 346),
                    ('_BumpMap', 168 if material == 75 else 225)]:
                with self.subTest(material=material, slot=slot), self.assertRaises(ValueError):
                    self.recover_with_mutation(material, material,
                        lambda d: dict(d['m_SavedProperties']['m_TexEnvs'])[slot]['m_Texture'].update(m_PathID=wrong))
        for target, space in [(346, 0), (168, 1)]:
            with self.subTest(texture=target, field='color space'), self.assertRaises(ValueError):
                self.recover_with_mutation(57, target, lambda d: d.update(m_ColorSpace=space))
        for field, value in [('m_Width', 0), ('m_Height', -1), ('m_MipCount', 0), ('m_MipCount', 11)]:
            with self.subTest(field=field, value=value), self.assertRaises(ValueError):
                self.recover_with_mutation(57, 346, lambda d: d.update({field: value}))
        for field, value in [('m_Aniso', -1), ('m_Aniso', 17), ('m_MipBias', float('nan')),
                ('m_FilterMode', -1), ('m_WrapU', 4), ('m_WrapV', -1)]:
            with self.subTest(field=field, value=value), self.assertRaises(ValueError):
                self.recover_with_mutation(57, 346, lambda d: d['m_TextureSettings'].update({field: value}))

    def test_player_forward_base_state(self):
        for field, value in [('culling', 0), ('zWrite', 0), ('zTest', 2),
                ('srcBlend', 5), ('destBlend', 1)]:
            def mutate(shader):
                state = shader['m_ParsedForm']['m_SubShaders'][0]['m_Passes'][0]['m_State']
                (state['rtBlend0'] if field in ('srcBlend', 'destBlend') else state)[field]['val'] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                self.recover_with_mutation(57, 719, mutate)

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
