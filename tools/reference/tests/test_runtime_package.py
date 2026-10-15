import json
import tempfile
import unittest
from pathlib import Path

from tools.reference.runtime_package import (build_file_catalog, build_prototype_plan,
    file_receipt, package_id, publish_package, verify_package)


class RuntimePackageTests(unittest.TestCase):
    def test_package_paths_preserve_internal_spaces_and_reject_unsafe_paths(self):
        import hashlib
        from tools.reference.runtime_package import canonical
        name = 'reference/lighting-unity default resources-10001.smip'
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / Path(name).name
            payload = b'checked source texture bytes'
            path.write_bytes(payload)
            receipt = file_receipt(path, name)
            self.assertEqual(receipt, {'file': name, 'bytes': len(payload),
                'sha256': hashlib.sha256(payload).hexdigest(), 'image_size': None})
            self.assertEqual(path.read_bytes(), payload)
        for path in (name, 'reference/a_b-0.1', 'reference/internal  spaces.bin', 'shaders/source_mesh.wgsl'):
            self.assertEqual(canonical(path), path)
        for path in (
            '', '/reference/file.bin', 'reference/', 'reference//file.bin',
            'reference/./file.bin', 'reference/../file.bin', '../file.bin', './reference/file.bin',
            'reference\\file.bin', 'C:/reference/file.bin', 'reference/%2e%2e/file.bin',
            'reference/file%20name.bin', 'reference/file.bin?x=1', 'reference/file.bin#x',
            'reference/ file.bin', 'reference/file.bin ', ' reference/file.bin', 'reference /file.bin',
            'reference/\tfile.bin', 'reference/file\tname.bin', 'reference/file.bin\n',
            'reference/file\rname.bin', 'reference/file\0.bin', 'reference/caf\u00e9.bin',
            'reference/file+name.bin', 'reference/file~name.bin',
        ):
            with self.subTest(path=path), self.assertRaisesRegex(ValueError, 'package path'):
                canonical(path)

    def test_world_provenance_accepts_only_checked_known_sidecars(self):
        import hashlib
        from tools.reference.export import verify_world_provenance
        world = {'library': {'schema_version': 1, 'manifest_sha256': 'a' * 64},
                 'source_hashes': [{'file': 'level1', 'sha256': 'b' * 64}]}
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            evidence = output / 'provenance'
            evidence.mkdir()
            def write(kind, **fields):
                value = dict(world, receipts=[], **fields)
                content = json.dumps(value, separators=(',', ':')).encode()
                path = evidence / f'world-{kind}-{hashlib.sha256(content).hexdigest()}.json'
                path.write_bytes(content)
                return path
            receipts = write('receipts')
            records = write('records', records=[], initialization={'nodes': [], 'components': []})
            expected = {str(p.relative_to(output).as_posix()) for p in (receipts, records)}
            self.assertEqual(verify_world_provenance(output, world), expected)
            ancestor = write('records', records=[], initialization={'nodes': [{'source': {'file': 'level1', 'path_id': 1}}], 'components': []})
            self.assertEqual(verify_world_provenance(output, world), expected | {ancestor.relative_to(output).as_posix()})
            ancestor.unlink()
            self.assertEqual(build_file_catalog({'assets': []}, output)['files'], [])
            records.write_bytes(records.read_bytes() + b' ')
            with self.assertRaisesRegex(ValueError, 'hash'):
                verify_world_provenance(output, world)
            records.unlink()
            records = write('records', records=[], initialization={'nodes': [], 'components': []})
            for name in ('unknown.json', 'world-records-not-a-hash.json'):
                extra = evidence / name
                extra.write_bytes(b'{}')
                with self.assertRaisesRegex(ValueError, 'path'):
                    verify_world_provenance(output, world)
                extra.unlink()
            nested = evidence / 'nested'
            nested.mkdir()
            with self.assertRaisesRegex(ValueError, 'path'):
                verify_world_provenance(output, world)
            nested.rmdir()
            extra = write('records', records=[], initialization={'nodes': [], 'components': []}, unknown=True)
            with self.assertRaisesRegex(ValueError, 'shape'):
                verify_world_provenance(output, world)
            extra.unlink()
            for field in ('library', 'source_hashes'):
                changed = dict(world, **{field: {}})
                with self.assertRaisesRegex(ValueError, 'identity'):
                    verify_world_provenance(output, changed)

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
