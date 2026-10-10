"""Critical image preservation checks."""
import hashlib
import io
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch
from PIL import Image
from tools.reference.library.images import export_images
from tools.reference.library.types import Catalog, ObjectId, ObjectRecord


class FakeSource:
    def __init__(self, entries):
        self.entries = entries
        self.root = Path('.')
    def get(self, file, path_id):
        return ObjectId(file, path_id)
    def read_record(self, reader):
        return self.entries[reader][1]
    def object(self, reader):
        return self.entries[reader][2]


def catalog(entries):
    return Catalog({key: ObjectRecord(key, value[0], 0, 0, 0, '') for key, value in entries.items()}, {},
                   {key: () for key in entries})


class ImageTests(unittest.TestCase):
    def export(self, entries, dependencies=None):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        output = Path(self.temp.name)
        model = catalog(entries)
        if dependencies:
            model.dependencies.update(dependencies)
        return output, export_images(FakeSource(entries), model, output)

    def test_zero_payload_font_texture(self):
        key = ObjectId('sharedassets0.assets', 352)
        data = {'m_Name': 'font', 'm_Width': 0, 'm_Height': 0, 'image data': b'',
                'm_StreamData': {'size': 0, 'offset': 0, 'path': ''}}
        output, exports = self.export({key: ('Texture2D', data, None)})
        self.assertEqual(exports[0].status, 'source_empty')
        self.assertEqual(exports[0].details['dimensions'], [0, 0])
        self.assertFalse(list(output.rglob('*.png')))

    def test_shared_texture_roles_distinct(self):
        key = ObjectId('a.assets', 4)
        pixels = Image.new('RGBA', (2, 3), (60, 80, 100, 120))
        obj = SimpleNamespace(image=pixels, get_image_data=lambda: b'pixels')
        data = {'m_Name': 'shared', 'm_Width': 2, 'm_Height': 3, 'm_TextureFormat': 4}
        with patch('tools.reference.library.images.get_image_from_texture2d', return_value=pixels):
            output, exports = self.export({key: ('Texture2D', data, obj)})
        file = exports[0].files[0]
        self.assertEqual(file.file, 'images/a.assets/4/source.png')
        with Image.open(output / file.file) as image:
            self.assertEqual(image.getpixel((0, 0)), (60, 80, 100, 120))
        self.assertEqual(exports[0].details['conversion']['role'], 'source_pixels')

    def test_sprite_rotation_border_and_cubemap_faces(self):
        sprite = ObjectId('a.assets', 2)
        cube = ObjectId('a.assets', 3)
        data = {'m_Name': 'slice', 'm_Border': {'x': 1, 'y': 2, 'z': 3, 'w': 4},
                'm_Pivot': {'x': .2, 'y': .3}, 'm_RD': {'settingsRaw': 17},
                'm_SpriteAtlas': {'m_FileID': 0, 'm_PathID': 10}}
        payload = b''.join(bytes([i]) * 16 for i in range(6))
        cubedata = {'m_Name': 'hdr', 'm_Width': 4, 'm_Height': 4, 'm_TextureFormat': 24,
                    'm_ImageCount': 6, 'm_MipCount': 1, 'm_CompleteImageSize': 16}
        entries = {sprite: ('Sprite', data, SimpleNamespace(image=Image.new('RGBA', (3, 2)))),
                   cube: ('Cubemap', cubedata, SimpleNamespace(get_image_data=lambda: payload))}
        output, exports = self.export(entries)
        self.assertEqual(exports[0].details['packing_rotation'], 4)
        self.assertEqual(exports[0].details['border'], data['m_Border'])
        faces = exports[1].details['faces']
        self.assertEqual([face['label'] for face in faces], ['+X', '-X', '+Y', '-Y', '+Z', '-Z'])
        for i, face in enumerate(faces):
            raw = (output / face['file']).read_bytes()
            self.assertEqual(raw[148:], bytes([i]) * 16)
            with Image.open(io.BytesIO(raw)) as image:
                self.assertEqual(image.size, (4, 4))
                self.assertEqual(image.pixel_format, 'BC6H')

    def test_embedded_font_bytes_unchanged(self):
        key = ObjectId('a.assets', 5)
        raw = b'\x00\x01\x00\x00' + bytes(80)
        face = SimpleNamespace(getname=lambda: ('Liberation Sans', 'Regular'))
        with patch('PIL.ImageFont.truetype', return_value=face):
            output, exports = self.export({key: ('Font', {'m_Name': 'LiberationSans', 'm_FontData': raw}, None)})
        record = exports[0].files[0]
        self.assertEqual((output / record.file).read_bytes(), raw)
        self.assertEqual(record.sha256, hashlib.sha256(raw).hexdigest())
        self.assertEqual(exports[0].details['face'], ['Liberation Sans', 'Regular'])

    def test_empty_font_keeps_dependencies(self):
        key = ObjectId('a.assets', 5)
        texture = ObjectId('a.assets', 6)
        _, exports = self.export({key: ('Font', {'m_Name': 'Arial', 'm_FontData': b''}, None)},
                                 {key: (texture,)})
        self.assertEqual(exports[0].status, 'native_record')
        self.assertIn('typed_record', exports[0].details)
        self.assertEqual(exports[0].dependencies, (texture,))


if __name__ == '__main__':
    unittest.main()
