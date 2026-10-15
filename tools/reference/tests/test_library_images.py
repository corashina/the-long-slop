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

if __name__ == '__main__':
    unittest.main()
