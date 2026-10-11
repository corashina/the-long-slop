import math
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

from PIL import Image

from tools.reference.gltf import GltfExporter


class MaterialTests(unittest.TestCase):
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
        identity = {'file':'fixture.assets','path_id':12}
        self.assertEqual(normal_encoding(texture, shader, 'Standard',identity), 'ag')
        texture['m_TextureFormat'] = TextureFormat.BC5
        self.assertEqual(normal_encoding(texture, shader, 'Standard',identity), 'rg')
        texture['m_TextureFormat'] = TextureFormat.RGBA32
        with self.assertRaisesRegex(ValueError, 'fixture.assets.*12.*_BumpMap'):
            normal_encoding(texture, shader, 'Standard',identity)
        with self.assertRaisesRegex(ValueError, 'encoding'):
            normal_encoding(texture, None, 'unknown',identity)

    def test_texture_roles_do_not_share_converted_pixels(self):
        reader = SimpleNamespace(assets_file=SimpleNamespace(name='fixture.assets',source_container_path='fixture.assets'), path_id=1,
                                 type=SimpleNamespace(name='Texture2D'))
        image = Image.new('RGBA', (1, 2))
        image.putdata([(255, 126, 126, 127), (255, 191, 126, 191)])
        texture = SimpleNamespace(image=image, m_ColorSpace=0, m_MipCount=1,
            m_TextureSettings=SimpleNamespace(m_FilterMode=1, m_WrapU=0, m_WrapV=0, m_Aniso=1, m_MipBias=0))
        source = SimpleNamespace(resolve=lambda *_: reader, read=lambda _: {'m_ColorSpace':0,'m_MipCount':1,'m_TextureSettings':vars(texture.m_TextureSettings)})
        environment = {'m_Texture': {}, 'm_Scale': {'x': 1., 'y': 1.}, 'm_Offset': {'x': 0., 'y': 0.}}
        cache = Path('.cache/reference-test')
        cache.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=cache) as directory:
            stored=Path(directory)/'source.png';image.save(stored)
            library=SimpleNamespace(texture=lambda _:stored)
            exporter = GltfExporter(source, Path(directory) / 'fixture.gltf',library)
            color = exporter.texture(reader, environment, 'linear')
            normal = exporter.texture(reader, environment, 'linear', normal_encoding='ag')
            self.assertNotEqual(color['index'], normal['index'])
            color_image = exporter.document.textures[color['index']].source
            normal_image = exporter.document.textures[normal['index']].source
            self.assertNotEqual(color_image, normal_image)
            with Image.open(Path(directory) / exporter.document.images[color_image].uri) as result:
                self.assertEqual(result.getpixel((0, 1)), (255, 126, 126, 127))
                self.assertEqual(result.getpixel((0, 0)), (255, 191, 126, 191))
            with Image.open(Path(directory) / exporter.document.images[normal_image].uri) as result:
                self.assertGreater(result.getpixel((0, 1))[2] / 255 * 2 - 1, 0.99)
                self.assertGreater(result.getpixel((0, 0))[0], result.getpixel((0, 1))[0])


if __name__ == '__main__':
    unittest.main()
