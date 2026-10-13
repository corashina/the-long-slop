import math
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

from PIL import Image



class MaterialTests(unittest.TestCase):
    def test_material_identity_includes_color_and_condition(self):
        from unittest.mock import patch
        from tools.reference.gltf import GltfExporter
        source, owner = self.material_source()
        cache = Path('.cache/reference-test'); cache.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=cache) as directory, patch('tools.reference.gltf.material_source_evidence'):
            exporter = GltfExporter(source, Path(directory) / 'fixture.gltf', None, 1)
            white = dict(r=1., g=1., b=1., a=1.)
            first = exporter.material(owner)
            self.assertEqual(first, exporter.material(owner, white))
            exporter.material_indices.clear()
            self.assertEqual(first, exporter.material(owner))
            self.assertEqual(exporter.material_indices['fixture.assets:10'], first)
            red = exporter.material(owner, dict(r=1., g=0., b=0., a=1.))
            self.assertNotEqual(first, red)
            condition = dict(source=dict(file='fixture.assets', path_id=99), state=0, quality=1., range=None)
            conditioned = exporter.material(owner, white, condition)
            self.assertNotEqual(first, conditioned)
            self.assertEqual(conditioned, exporter.material(owner, white, dict(condition)))

    def test_material_instancing_uses_required_source_flag(self):
        from tools.reference.materials import recover_material
        source,owner=self.material_source()
        self.assertFalse(recover_material(source,owner)['enable_instancing'])
        source.read(owner)['m_EnableInstancingVariants']=True
        self.assertTrue(recover_material(source,owner)['enable_instancing'])
        del source.read(owner)['m_EnableInstancingVariants']
        with self.assertRaises(KeyError): recover_material(source,owner)

    def test_source_channel_samples(self):
        from tools.reference.materials import SOURCE_CHANNEL_SAMPLES
        detail, smoothness, linear = SOURCE_CHANNEL_SAMPLES
        self.assertEqual(detail['detail'], .2)
        self.assertEqual(detail['mask_alpha'], .5)
        self.assertAlmostEqual(detail['albedo_multiplier'], .7)
        self.assertEqual(smoothness['main_alpha'], .75)
        self.assertEqual(smoothness['gloss_map_scale'], .4)
        self.assertAlmostEqual(smoothness['smoothness'], .3)
        self.assertEqual(smoothness['keyword'], '_SMOOTHNESS_TEXTURE_ALBEDO_CHANNEL_A')
        self.assertAlmostEqual(linear['albedo_multiplier'], .5 + .5 * .2 * 4.59479380)

    def test_packed_normal_uses_alpha_green(self):
        from tools.reference.materials import decode_normal_rgba
        x, y, z = decode_normal_rgba((255, 126, 126, 127), 'ag')
        self.assertAlmostEqual(x, -1 / 255, places=6)
        self.assertAlmostEqual(y, -3 / 255, places=6)
        self.assertAlmostEqual(z, math.sqrt(1 - x*x - y*y), places=6)

    def test_normal_encodings_and_unknown_encoding(self):
        from tools.reference.materials import decode_normal_rgba
        self.assertEqual(decode_normal_rgba((255, 0, 99, 0), 'rg'), (1., -1., 0.))
        self.assertEqual(decode_normal_rgba((255, 0, 255, 0), 'rgb'), (1., -1., 1.))
        with self.assertRaisesRegex(ValueError, 'encoding'):
            decode_normal_rgba((255, 126, 126, 127), 'unknown')

    def test_source_normal_encoding_requires_shader_and_format_evidence(self):
        from tools.reference.materials import normal_encoding
        from UnityPy.enums import TextureFormat
        reader = SimpleNamespace(assets_file=SimpleNamespace(name='fixture.assets',source_container_path='fixture.assets'), path_id=12)
        texture = {'m_TextureFormat':TextureFormat.DXT5Crunched}
        shader = {'platforms':[4]}
        self.assertEqual(normal_encoding(texture, shader, 'Standard', {'file':'fixture.assets','path_id':12}), 'ag')
        texture['m_TextureFormat'] = TextureFormat.BC5
        self.assertEqual(normal_encoding(texture, shader, 'Standard', {'file':'fixture.assets','path_id':12}), 'rg')
        texture['m_TextureFormat'] = TextureFormat.RGBA32
        with self.assertRaisesRegex(ValueError, 'fixture.assets.*12.*_BumpMap'):
            normal_encoding(texture, shader, 'Standard', {'file':'fixture.assets','path_id':12})
        with self.assertRaisesRegex(ValueError, 'encoding'):
            normal_encoding(texture, None, 'unknown', {'file':'fixture.assets','path_id':12})

    def test_texture_roles_do_not_share_converted_pixels(self):
        from tools.reference.materials import recover_material, export_material_textures
        from UnityPy.enums import TextureFormat
        source, owner = self.material_source(texture_pointer=12)
        data = source.read(owner)
        data['m_ShaderKeywords'] += ' _NORMALMAP'
        data['m_SavedProperties']['m_TexEnvs'].append(('_BumpMap', data['m_SavedProperties']['m_TexEnvs'][0][1]))
        texture = source.object(owner)
        texture.m_TextureFormat = TextureFormat.DXT5
        texture.m_Width, texture.m_Height = 1, 2
        texture.image = Image.new('RGBA', (1, 2))
        texture.image.putdata([(255, 126, 126, 127), (255, 191, 126, 191)])
        record = recover_material(source, owner)
        cache = Path('.cache/reference-test'); cache.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=cache) as directory:
            export_material_textures(source, owner, record, Path(directory))
            entries = {t['slot']: t for t in record['properties']['textures']}
            self.assertNotEqual(entries['_BumpMap']['file'], entries['_DetailMask']['file'])
            with Image.open(Path(directory) / Path(entries['_DetailMask']['file']).name) as image:
                self.assertEqual(image.getpixel((0, 1)), (255, 126, 126, 127))
                self.assertEqual(image.getpixel((0, 0)), (255, 191, 126, 191))
            with Image.open(Path(directory) / Path(entries['_BumpMap']['file']).name) as image:
                self.assertGreater(image.getpixel((0, 1))[2] / 255 * 2 - 1, .99)
                self.assertGreater(image.getpixel((0, 0))[0], image.getpixel((0, 1))[0])

    def material_source(self, shader_name='Standard', texture_pointer=0):
        owner = SimpleNamespace(assets_file=SimpleNamespace(name='fixture.assets',source_container_path='fixture.assets'), path_id=10)
        shader_reader = SimpleNamespace(assets_file=owner.assets_file, path_id=11)
        texture_reader = SimpleNamespace(assets_file=owner.assets_file, path_id=12,
                                         type=SimpleNamespace(name='Texture2D'))
        shader = SimpleNamespace(m_Name=shader_name, platforms=[4])
        texture = SimpleNamespace(image=Image.new('RGBA', (1, 1), (20, 30, 40, 128)),
            m_Width=1, m_Height=1, m_MipCount=1, m_ColorSpace=0,
            m_TextureSettings=SimpleNamespace(m_FilterMode=1, m_WrapU=0, m_WrapV=0,
                                              m_Aniso=1, m_MipBias=0))
        env = {'m_Texture': {'m_FileID': 0, 'm_PathID': texture_pointer},
               'm_Scale': {'x': 3., 'y': 4.}, 'm_Offset': {'x': .2, 'y': .3}}
        data = {'m_Name': 'fixture', 'm_EnableInstancingVariants': False, 'm_Shader': {'m_PathID': 11},
            'm_ShaderKeywords': '_DETAIL_MULX2', 'm_CustomRenderQueue': -1,
            'm_SavedProperties': {'m_Floats': [], 'm_Colors': [],
                                 'm_TexEnvs': [('_DetailMask', env)]}}
        def resolve(_, pointer):
            return {0: None, 11: shader_reader, 12: texture_reader}.get(pointer['m_PathID'])
        source = SimpleNamespace(resolve=resolve,
            read=lambda r: vars(shader) if r.path_id == 11 else dict(vars(texture),m_TextureSettings=vars(texture.m_TextureSettings)) if r.path_id == 12 else data,
            object=lambda r: shader if r.path_id == 11 else texture, texture_object=lambda r:texture)
        return source, owner

    def test_material_keeps_detail_mask_alpha(self):
        from tools.reference.materials import recover_material, export_material_textures
        source, owner = self.material_source(texture_pointer=12)
        record = recover_material(source, owner, None, None)
        cache = Path('.cache/reference-test'); cache.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=cache) as directory:
            export_material_textures(source, owner, record, Path(directory))
            mask = next(t for t in record['properties']['textures'] if t['slot'] == '_DetailMask')
            self.assertEqual(mask['scale'], [1., 1.])
            self.assertEqual(mask['offset'], [0., 0.])
            with Image.open(Path(directory) / Path(mask['file']).name) as image:
                self.assertEqual(image.getpixel((0, 0))[3], 128)

    def test_missing_active_texture_names_source(self):
        from tools.reference.materials import recover_material
        source, owner = self.material_source(texture_pointer=99)
        with self.assertRaisesRegex(ValueError, 'fixture.assets.*10.*_DetailMask'):
            recover_material(source, owner, None, None)

    def test_known_rope_keeps_existing_imported_pbr(self):
        from unittest.mock import patch
        from tools.reference.gltf import GltfExporter
        source,owner=self.material_source(shader_name='AmplifyRope')
        data=source.read(owner);data['m_ShaderKeywords']=''
        data['m_SavedProperties']={'m_Floats':[('_Sag',1.5),('_SwaySpeed',.1),('_SwayStrength',1.5)],'m_Colors':[],'m_TexEnvs':[]}
        with tempfile.TemporaryDirectory() as directory, patch('tools.reference.gltf.material_source_evidence'):
            writer=GltfExporter(source,Path(directory)/'rope.gltf',None,4)
            material=writer.document.materials[writer.material(owner)]
            record=writer.material_records[0]
            self.assertEqual(record['shader_family'],'amplify_rope')
            self.assertEqual(record['properties']['colors'],{})
            self.assertEqual(record['properties']['textures'],[])
            self.assertEqual(record['properties']['floats']['_Sag'],1.5)
            self.assertEqual(material.pbrMetallicRoughness.baseColorFactor,[1.,1.,1.,1.])
            self.assertEqual(material.pbrMetallicRoughness.metallicFactor,0.)
            self.assertEqual(material.pbrMetallicRoughness.roughnessFactor,.5)

    def test_unknown_required_shader_is_rejected(self):
        from tools.reference.materials import recover_material
        source, owner = self.material_source(shader_name='Unknown/Required')
        with self.assertRaisesRegex(ValueError, 'fixture.assets.*10.*Unknown/Required'):
            recover_material(source, owner, None, None)

    def test_stored_texture_color_space_overrides_material_role(self):
        from tools.reference.materials import recover_material
        source, owner = self.material_source(texture_pointer=12)
        texture = source.object(owner)
        for stored, expected in ((0, 'linear'), (1, 'srgb')):
            texture.m_ColorSpace = stored
            record = recover_material(source, owner)
            entry = next(t for t in record['properties']['textures'] if t['slot'] == '_DetailMask')
            self.assertEqual(entry['color_space'], expected)
            self.assertIn('-' + expected + '.', entry['file'])
        texture.m_ColorSpace = 2
        with self.assertRaisesRegex(ValueError, 'stored color space'):
            recover_material(source, owner)

    def test_null_texture_uses_shader_default(self):
        from tools.reference.materials import recover_material
        source, owner = self.material_source()
        record = recover_material(source, owner, None, None)
        self.assertEqual(record['properties']['textures'], [])
        self.assertEqual(record['properties']['defaults']['_DetailMask'], 'white')

    def test_condition_old_color_uses_rusty_color(self):
        from tools.reference.conditions import condition_material
        source, owner = self.material_source()
        rusty = {'r': .2, 'g': .3, 'b': .4, 'a': 1.}
        source.read = lambda _: {'m_SavedProperties': {'m_Colors': [('_Color', rusty)]}}
        data = {'DontUseWorn': False, 'UseMatList': False, 'disableColor': False,
            'forceColor': False, 'defOld': {'m_PathID': 11}, 'defRusty': {'m_PathID': 12}}
        for state in (3, 4):
            _, color = condition_material(source, owner, data, None, owner,
                                         {'state': state, 'color': {}})
            self.assertEqual(color, rusty)
        data['disableColor'] = True
        self.assertIsNone(condition_material(source, owner, data, None, owner,
                         {'state': 3, 'color': {}})[1])

    def test_condition_force_color_uses_part_color(self):
        from tools.reference.conditions import condition_material
        source, owner = self.material_source()
        color = {'r': .4, 'g': .5, 'b': .6, 'a': 1.}
        data = {'DontUseWorn': False, 'UseMatList': False, 'disableColor': False,
            'forceColor': True, 'defOld': {'m_PathID': 11}, 'defRusty': {'m_PathID': 12}}
        for state in (3, 4):
            self.assertEqual(condition_material(source, owner, data, None, owner,
                             {'state': state, 'color': color})[1], color)

    def test_authored_mips_keep_mask_alpha(self):
        from tools.reference.materials import source_mip_images
        from UnityPy.enums import TextureFormat, BuildTarget
        # Two authored levels have distinct alpha. A resized base level cannot pass.
        texture = SimpleNamespace(m_MipCount=2, m_Width=2, m_Height=2,
            m_TextureFormat=TextureFormat.RGBA32, get_image_data=lambda: bytes([10, 20, 30, 128]*4 + [40, 50, 60, 64]),
            object_reader=SimpleNamespace(version=(2021, 3, 18, 1), platform=BuildTarget.StandaloneWindows64))
        levels = list(source_mip_images(texture))
        self.assertEqual([level.size for level in levels], [(2, 2), (1, 1)])
        self.assertEqual(levels[0].getpixel((0, 0))[3], 128)
        self.assertEqual(levels[1].getpixel((0, 0))[3], 64)

    def test_detail_normal_uses_detail_albedo_transform(self):
        from tools.reference.materials import recover_material
        from UnityPy.enums import TextureFormat
        source, owner = self.material_source(texture_pointer=12)
        data = source.read(owner)
        data['m_ShaderKeywords'] += ' _NORMALMAP'
        data['m_SavedProperties']['m_TexEnvs'].append(('_DetailNormalMap',
            data['m_SavedProperties']['m_TexEnvs'][0][1]))
        data['m_SavedProperties']['m_TexEnvs'].append(('_DetailAlbedoMap',
            {'m_Texture': {'m_PathID': 0}, 'm_Scale': {'x': 7., 'y': 8.}, 'm_Offset': {'x': .4, 'y': .5}}))
        data['m_SavedProperties']['m_Floats'] = [('_UVSec', 1.)]
        source.object(owner).m_TextureFormat = TextureFormat.DXT5
        normal = next(t for t in recover_material(source, owner)['properties']['textures']
                      if t['slot'] == '_DetailNormalMap')
        self.assertEqual(normal['scale'], [7., 8.])
        self.assertEqual(normal['offset'], [.4, .5])
        self.assertEqual(normal['uv_channel'], 1)
        self.assertEqual(normal['encoding'], 'rgb')
        self.assertEqual(normal['source_encoding'], 'ag')

    def test_child_inherits_state_without_parent_paint(self):
        from tools.reference.conditions import prepare_conditions
        def reader(pid):
            return SimpleNamespace(assets_file=SimpleNamespace(name='fixture.assets',source_container_path='fixture.assets'), path_id=pid,
                type=SimpleNamespace(name='MonoBehaviour'))
        parent, child, renderer, material = [reader(i) for i in range(1, 5)]
        pointer = lambda pid: {'m_PathID': pid}
        red = dict(r=1., g=0., b=0., a=1.)
        black = dict(r=0., g=0., b=0., a=1.)
        common = {name: False for name in ['useRandomTipus', 'setRandom', 'useOnlyMaterialTipusForMaterial',
            'enableOverrideMin', 'enableOverrideMax', 'noInheritState', 'loaded', 'sameStateAsParent',
            'isChildnewColor', 'randomStartState3InFStart', 'UseMesList', 'UseGoList', 'UseMatList',
            'DontUseWorn', 'disableColor', 'forceColor']}
        common.update(state=0, state2=0, state3=0, tipus='paint', colors=[], MesList=[], GoList=[],
                      renderers=[], childs=[], randomMin=1, randomMax=1)
        parent_data = dict(common, color=red, randomStart=True, isChild=False, childs=[pointer(2)])
        child_data = dict(common, color=black, randomStart=False, isChild=True, renderers=[pointer(3)])
        group = {'tipus': 'paint', 'colors': [red], 'New': pointer(4), 'Used': pointer(4),
                 'Middle': pointer(4), 'Old': pointer(4), 'Rusty': pointer(4)}
        data = {1: parent_data, 2: child_data, 3: {}, 4: {}, 99: {'conditionmaterials': [group]}}
        objects = {r.path_id: r for r in [parent, child, renderer, material]}
        source = SimpleNamespace(get=lambda *_: reader(99), read=lambda r: data[r.path_id],
            resolve=lambda _, p: objects.get(p['m_PathID']), class_name=lambda _: 'partconditionscript',
            hierarchy=lambda *_args, **_kwargs: [{'active': True, 'components': [parent, child]}])
        prepare_conditions(source, [parent])
        self.assertEqual(source.condition_renderers[('fixture.assets', 3)]['state'], 1)
        self.assertEqual(source.condition_colors[('fixture.assets', 3)], black)


if __name__ == '__main__':
    unittest.main()
