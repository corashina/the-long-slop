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

    def test_player_source_pass_states(self):
        for index, name in enumerate(('FORWARDBASE', 'FORWARDADD', 'DEFERRED', 'SHADOWCASTER', 'META')):
            for field in ('culling', 'zWrite', 'zTest', 'srcBlend', 'destBlend', 'colMask'):
                for change in ('value', 'missing', 'binding'):
                    def mutate(shader):
                        state = shader['m_ParsedForm']['m_SubShaders'][0]['m_Passes'][index]['m_State']
                        owner = state['rtBlend0'] if field in ('srcBlend', 'destBlend', 'colMask') else state
                        if change == 'missing':
                            del owner[field]
                        elif change == 'binding':
                            owner[field]['name'] = '_ZWrite'
                        else:
                            owner[field]['val'] += 1
                    with self.subTest(render_pass=name, field=field, change=change), self.assertRaises(ValueError):
                        self.recover_with_mutation(57, 719, mutate)

    def test_player_source_pass_structure(self):
        for label, mutate in [
            ('missing subshaders', lambda s: s.pop('m_SubShaders')),
            ('empty subshaders', lambda s: s.update(m_SubShaders=[])),
            ('extra subshader', lambda s: s['m_SubShaders'].append(copy.deepcopy(s['m_SubShaders'][0]))),
            ('subshader LOD', lambda s: s['m_SubShaders'][0].update(m_LOD=100)),
            ('missing subshader tags', lambda s: s['m_SubShaders'][0].pop('m_Tags')),
            ('queue tag', lambda s: s['m_SubShaders'][0]['m_Tags']['tags'].__setitem__(1, ('QUEUE', 'Geometry'))),
            ('render tag', lambda s: s['m_SubShaders'][0]['m_Tags']['tags'].__setitem__(2, ('RenderType', 'Opaque'))),
            ('missing passes', lambda s: s['m_SubShaders'][0].pop('m_Passes')),
            ('empty passes', lambda s: s['m_SubShaders'][0].update(m_Passes=[])),
            ('missing pass', lambda s: s['m_SubShaders'][0]['m_Passes'].pop()),
            ('extra pass', lambda s: s['m_SubShaders'][0]['m_Passes'].append(copy.deepcopy(s['m_SubShaders'][0]['m_Passes'][0]))),
            ('pass order', lambda s: s['m_SubShaders'][0]['m_Passes'].reverse()),
        ]:
            with self.subTest(structure=label), self.assertRaises(ValueError):
                self.recover_with_mutation(57, 719, lambda shader: mutate(shader['m_ParsedForm']))
        for index in range(5):
            for label, mutate in [
                ('type', lambda p: p.update(m_Type=1)),
                ('missing state', lambda p: p.pop('m_State')),
                ('name', lambda p: p['m_State'].update(m_Name='OTHER')),
                ('LOD', lambda p: p['m_State'].update(m_LOD=0)),
                ('identity', lambda p: p['m_State']['m_Tags']['tags'].__setitem__(1, ('LIGHTMODE', 'OTHER'))),
                ('missing identity', lambda p: p['m_State']['m_Tags']['tags'].pop(1)),
            ]:
                with self.subTest(render_pass=index, structure=label), self.assertRaises(ValueError):
                    self.recover_with_mutation(57, 719,
                        lambda shader: mutate(shader['m_ParsedForm']['m_SubShaders'][0]['m_Passes'][index]))

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

    def test_complete_selected_player_material_variants(self):
        from tools.reference.materials import PLAYER_TEXTURES
        for pid,(main,bump) in PLAYER_TEXTURES.items():
            with self.subTest(material=pid):
                reader=self.source.get('sharedassets0.assets',pid)
                record=recover_material(self.source,reader)
                self.assertEqual(record['properties']['defaults'],{'_BumpMap':'bump'} if bump is None else {})
                self.assertEqual([(t['slot'],t['source']['path_id']) for t in record['properties']['textures']],
                    [('_MainTex',main)]+([('_BumpMap',bump)] if bump else []))
                for texture in record['properties']['textures']:self.source.library.texture(ObjectId(**texture['source']))
                self.assertEqual('_EmissionColor' in record['properties']['colors'],pid not in (63,64))
                self.assertEqual('_Red' in record['properties']['floats'],pid in (61,69))
                material_source_evidence(self.source,reader,record)
                self.assertEqual(self.source.material_evidence[f'sharedassets0.assets:{pid}']['authored_null_textures'],['_BumpMap'] if bump is None else [])
        for pid,mutate in [(63,lambda d:d['m_SavedProperties']['m_Floats'].pop()),
                (64,lambda d:d['m_SavedProperties']['m_Colors'].append(('_EmissionColor',dict(r=0.,g=0.,b=0.,a=0.)))),
                (69,lambda d:d['m_SavedProperties']['m_Floats'].__setitem__(slice(None),[v for v in d['m_SavedProperties']['m_Floats'] if v[0]!='_Red'])),
                (67,lambda d:dict(d['m_SavedProperties']['m_TexEnvs'])['_BumpMap']['m_Texture'].update(m_PathID=168,m_FileID=0)),
                (70,lambda d:dict(d['m_SavedProperties']['m_TexEnvs'])['_BumpMap']['m_Texture'].update(m_PathID=168,m_FileID=0)),
                (71,lambda d:dict(d['m_SavedProperties']['m_TexEnvs'])['_BumpMap']['m_Texture'].update(m_PathID=0))]:
            with self.subTest(malformed=pid),self.assertRaises(ValueError):self.recover_with_mutation(pid,pid,mutate)
