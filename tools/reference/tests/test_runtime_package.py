import json
import tempfile
import unittest
from pathlib import Path

from tools.reference.runtime_package import (build_file_catalog, build_prototype_plan,
    file_receipt, package_id, publish_package, verify_package)


class RuntimePackageTests(unittest.TestCase):
    def test_receipts_and_selected_plans(self):
        from PIL import Image
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            (output / 'mesh.bin').write_bytes(bytes(36))
            Image.new('RGBA', (2, 3)).save(output / 'image.png')
            document = {'buffers': [{'uri': 'mesh.bin', 'byteLength': 36}],
                        'nodes': [{'mesh': 0}], 'accessors': [{'count': 3, 'type': 'VEC3'}],
                        'meshes': [{'primitives': [{'attributes': {'POSITION': 0},
                                    'extras': {'submesh': 0, 'world_source_material': None}}]}]}
            (output / 'mesh.gltf').write_text(json.dumps(document))
            manifest = {'assets': [{'path': 'reference/mesh.gltf', 'dependencies': [
                {'file': 'reference/mesh.bin'}, {'file': 'reference/image.png'}, {'file': 'reference/mesh.bin'}]}]}
            catalog = build_file_catalog(manifest, output)
            receipts = {r['file']: r for r in catalog['files']}
            self.assertEqual(len(receipts), len(catalog['files']))
            self.assertEqual(receipts['reference/mesh.bin']['bytes'], 36)
            self.assertEqual(receipts['reference/image.png']['image_size'], [2, 3])
            self.assertEqual(catalog['plans'][0]['asset_file'], 'reference/mesh.gltf')
            plan = json.loads((output / Path(catalog['plans'][0]['input']['file']).name).read_bytes())
            self.assertEqual(plan['buffer_bytes'], 36)
            self.assertEqual(plan['mesh_bytes'], 144)
            document['buffers'][0]['byteLength'] = 35
            with self.assertRaisesRegex(ValueError, 'buffer'):
                build_prototype_plan(document, {'document': receipts['reference/mesh.gltf'], 'files': receipts})

    def test_fixed_authored_null_slot_is_not_a_fabricated_material(self):
        document = {'nodes': [{'mesh': 0}], 'accessors': [{'count': 3, 'type': 'VEC3'}],
                    'meshes': [{'primitives': [{'attributes': {'POSITION': 0}, 'extras': {'submesh': 0}}]}]}
        plan = build_prototype_plan(document, {'document': {'file': 'reference/sky.gltf', 'bytes': 20}, 'files': {}})
        self.assertIsNone(plan['primitives'][0]['material'])
        self.assertFalse(plan['primitives'][0]['has_source_material'])
        self.assertIsNone(plan['nodes'][0]['extras'])

    def test_plan_arithmetic_rejects_overflow(self):
        document = {'accessors': [{'count': 2**32, 'type': 'VEC3'}],
                    'meshes': [{'primitives': [{'attributes': {'POSITION': 0},
                                'extras': {'submesh': 0, 'world_source_material': None}}]}]}
        with self.assertRaisesRegex(ValueError, 'overflow'):
            build_prototype_plan(document, {'document': {'file': 'reference/mesh.gltf', 'bytes': 1}, 'files': {}})

    def test_package_identity_changes_with_final_bytes_and_preserves_old_package(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            output = root / 'reference'
            output.mkdir()
            payload = output / 'mesh.bin'
            payload.write_bytes(b'first')
            manifest = {'file_catalog': {'files': [file_receipt(payload, 'reference/mesh.bin')],
                'plans': [], 'dependency_staging_reserve': 5242960}, 'build_id': 11140890}
            first = package_id(manifest)
            manifest['content_base'] = f'assets/runtime/{first}/'
            self.assertEqual(package_id(manifest), first)
            (output / 'manifest.json').write_text(json.dumps(manifest))
            publish_package(output, root)
            verify_package(manifest, root / 'runtime' / first)
            payload.write_bytes(b'other')
            manifest['file_catalog']['files'] = [file_receipt(payload, 'reference/mesh.bin')]
            second = package_id(manifest)
            self.assertNotEqual(first, second)
            manifest['content_base'] = f'assets/runtime/{second}/'
            (output / 'manifest.json').write_text(json.dumps(manifest))
            publish_package(output, root)
            self.assertEqual((root / 'runtime' / first / 'reference/mesh.bin').read_bytes(), b'first')
            self.assertEqual((root / 'runtime' / second / 'reference/mesh.bin').read_bytes(), b'other')


if __name__ == '__main__':
    unittest.main()
