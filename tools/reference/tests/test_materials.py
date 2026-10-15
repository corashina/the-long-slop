import math
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

from PIL import Image

class MaterialTests(unittest.TestCase):
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

    def test_unknown_required_shader_is_rejected(self):
        from tools.reference.materials import recover_material
        source, owner = self.material_source(shader_name='Unknown/Required')
        with self.assertRaisesRegex(ValueError, 'fixture.assets.*10.*Unknown/Required'):
            recover_material(source, owner, None, None)

if __name__ == '__main__':
    unittest.main()
