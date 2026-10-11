import hashlib
import json
import struct
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch, Mock
from dataclasses import FrozenInstanceError, fields

from tools.reference.library.archive import snapshot_source, verify_snapshot, scan_files
from tools.reference.library.inventory import inventory_source
from tools.reference.library.types import ObjectId, FileRecord, Snapshot, ObjectRecord, Catalog, AssetExport, LibraryReceipt
from tools.reference.source import Source, discover_source, source_id, key


def serialized_bytes(version='2021.3.18f1'):
    header = struct.pack('>4I', 0, 0, 22, 0) + bytes(4)
    header += struct.pack('>Iqqq', 64, 160, 128, 0)
    return (header + version.encode('ascii') + b'\0').ljust(160, b'\0')


class LibrarySourceTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.root = self.base / 'steamapps/common/The Long Drive'
        self.data = self.root / 'TheLongDrive_Data'
        self.data.mkdir(parents=True)
        self.manifest = self.root.parents[1] / 'appmanifest_1017180.acf'
        self.manifest.write_text('"AppState" { "appid" "1017180" "buildid" "11140890" }')
        (self.data / 'level0').write_bytes(serialized_bytes())
        self.work = self.base / 'library'

    def test_relative_archive_paths_verify(self):
        from tools.reference.library.archive import load_snapshot
        with patch('os.getcwd', return_value=str(self.base)):
            relative_root = self.root.relative_to(self.base)
            self.assertEqual(scan_files(relative_root), scan_files(self.root))
            self.assertEqual(discover_source(relative_root / 'TheLongDrive_Data'), discover_source(self.data))
            snapshot_source(relative_root, Path('library'))
            verify_snapshot(load_snapshot(Path('library')))

    def test_shared_records_and_cache_layout(self):
        expected = {
            ObjectId: ('file', 'path_id'), FileRecord: ('file', 'bytes', 'sha256'),
            Snapshot: ('root', 'files', 'manifest_path', 'manifest_sha256'),
            ObjectRecord: ('source', 'type', 'class_id', 'offset', 'bytes', 'container_sha256'),
            Catalog: ('objects', 'counts', 'dependencies'),
            AssetExport: ('source', 'category', 'status', 'runtime', 'reason', 'files', 'dependencies', 'details'),
            LibraryReceipt: ('manifest_path', 'manifest_sha256', 'catalog_sha256', 'source_manifest_sha256', 'exports'),
        }
        for record, names in expected.items():
            self.assertEqual(tuple(field.name for field in fields(record)), names)
            self.assertTrue(record.__dataclass_params__.frozen)
        snapshot = snapshot_source(self.root, self.work)
        self.assertEqual(snapshot.root, self.work / 'source')
        self.assertEqual(snapshot.manifest_path, self.work / 'source-manifest.json')
        self.assertTrue((self.work / 'steam-manifest.acf').is_file())
        self.assertFalse((snapshot.root / 'steam-manifest.acf').exists())
        self.assertEqual(snapshot.manifest_sha256, hashlib.sha256(snapshot.manifest_path.read_bytes()).hexdigest())
        self.assertTrue(all(isinstance(file, FileRecord) for file in snapshot.files))
        with self.assertRaises(FrozenInstanceError):
            snapshot.root = self.root
        with patch('UnityPy.Environment', return_value=FakeEnvironment()), patch('UnityPy.helpers.TypeTreeGenerator.TypeTreeGenerator'):
            source = Source(snapshot.root)
        catalog = inventory_source(source, snapshot)
        identity = ObjectId('level0', 1)
        self.assertEqual(set(catalog.objects), {identity})
        self.assertEqual(catalog.counts, {'GameObject': 1})
        self.assertEqual(catalog.dependencies, {identity: ()})
        record = catalog.objects[identity]
        self.assertIsInstance(record, ObjectRecord)
        container = next(file for file in snapshot.files if file.file == 'TheLongDrive_Data/level0')
        self.assertEqual(record.container_sha256, container.sha256)

    def test_cli_runtime_uses_library_prerequisite(self):
        from tools.reference.export import main
        accepted = Mock(spec=Snapshot)
        calls = Mock()
        calls.attach_mock(Mock(), 'library')
        calls.attach_mock(Mock(), 'runtime')
        with patch('tools.reference.export.stage_library', calls.library), patch('tools.reference.export.derive_reference', calls.runtime), patch('tools.reference.library.archive.load_snapshot', return_value=accepted):
            main(['--steam-root', str(self.root), '--output', str(self.base / 'runtime')])
        self.assertEqual([call[0] for call in calls.mock_calls], ['library', 'runtime'])
        calls.library.assert_called_once_with(self.root, self.base / 'runtime', verify=False)
        calls.runtime.assert_called_once_with(accepted, calls.library.return_value, self.base / 'runtime')

        with patch('sys.stderr'):
            with self.assertRaises(SystemExit):
                main(['--stage', 'source', '--work-root', str(self.work)])

    def test_catalog_source_catalog_verification_preserves_acceptance(self):
        from tools.reference.export import stage_catalog, stage_source
        class InstalledObjects(dict):
            def __len__(self):
                return 262784
        a, b = ObjectId('owner', 1), ObjectId('owner', 2)
        objects = InstalledObjects({identity: ObjectRecord(identity, 'Material', 21, 0, 32, 'container') for identity in (a, b)})
        inventory = Catalog(objects, {'Material': 2}, {a: (), b: ()})
        complete = Catalog(objects, {'Material': 2}, {a: (b,), b: (a,)})
        snapshot = Snapshot(self.work / 'source', (), self.work / 'source-manifest.json', 'source')
        self.work.mkdir()
        snapshot.manifest_path.write_text('accepted source')
        parsed = SimpleNamespace(serialized_files=dict.fromkeys(range(9)), resource_streams=tuple(range(7)))
        def data(catalog, source, accepted):
            return {'schema_version': 1, 'source_manifest_sha256': 'source', 'objects': [
                {'source': {'file': identity.file, 'path_id': identity.path_id},
                 'dependencies': [{'file': target.file, 'path_id': target.path_id} for target in catalog.dependencies[identity]]}
                for identity in (a, b)]}
        def build(source, accepted, root):
            (root / 'manifest.json').write_text(json.dumps({'schema_version': 1, 'source_manifest_sha256': 'source'}))
            return complete
        def verify(source, accepted, catalog_path, records_root):
            receipt = json.loads((records_root / 'manifest.json').read_bytes())
            raw = catalog_path.read_bytes()
            if hashlib.sha256(raw).hexdigest() != receipt['catalog_sha256']:
                raise ValueError('Catalog hash mismatch')
            self.assertEqual(json.loads(raw), data(complete, parsed, snapshot))
            return complete
        with patch('tools.reference.export.SOURCE_WORK_ROOT', self.work), patch('tools.reference.source.Source', return_value=parsed), \
             patch('tools.reference.library.archive.load_snapshot', return_value=snapshot), patch('tools.reference.library.archive.verify_snapshot'), \
             patch('tools.reference.library.archive.snapshot_source', return_value=snapshot), patch('tools.reference.library.inventory.inventory_source', return_value=inventory), \
             patch('tools.reference.library.inventory.catalog_data', side_effect=data), patch('tools.reference.library.records.build_catalog', side_effect=build), \
             patch('tools.reference.library.records.verify_catalog', side_effect=verify):
            stage_catalog(None)
            before = {path: (path.stat().st_mtime_ns, path.read_bytes()) for path in (self.work / 'catalog.json', self.work / 'records/manifest.json')}
            managed = self.work / 'records/managed/analysis.json'
            managed.parent.mkdir()
            managed.write_bytes(b'accepted managed analysis')
            before[managed] = (managed.stat().st_mtime_ns, managed.read_bytes())
            stage_source(self.root)
            self.assertEqual(stage_catalog(None), complete)
            self.assertEqual(stage_catalog(None, verify=True), complete)
            stage_source(None, verify=True)
            after = {path: (path.stat().st_mtime_ns, path.read_bytes()) for path in before}
            self.assertEqual(before, after)
            obsolete = json.loads((self.work / 'catalog.json').read_bytes())
            obsolete['schema_version'] = 0
            (self.work / 'catalog.json').write_text(json.dumps(obsolete))
            with self.assertRaisesRegex(ValueError, 'complete inventory'):
                stage_source(self.root)

    def test_cli_rejects_live_installation_output_before_staging(self):
        from tools.reference.export import main, _export_selected
        before = {path.relative_to(self.root).as_posix(): (path.stat().st_mtime_ns, path.read_bytes())
                  for path in self.root.rglob('*') if path.is_file()}
        for output in (self.root, self.root / 'generated', self.data / '..' / 'generated'):
            with self.subTest(output=output):
                with patch('tools.reference.export.stage_source') as source, patch('tools.reference.export.derive_reference') as runtime, patch('sys.stderr'):
                    with self.assertRaises(SystemExit) as rejected:
                        main(['--steam-root', str(self.root), '--output', str(output)])
                self.assertEqual(rejected.exception.code, 2)
                source.assert_not_called()
                runtime.assert_not_called()
        after = {path.relative_to(self.root).as_posix(): (path.stat().st_mtime_ns, path.read_bytes())
                 for path in self.root.rglob('*') if path.is_file()}
        self.assertEqual(before, after)
        self.assertFalse((self.root / 'generated').exists())
        archive_root = self.work / 'source'
        with self.assertRaisesRegex(ValueError, 'outside the source installation'):
            _export_selected(SimpleNamespace(root=archive_root), archive_root / 'generated', None, None)
        self.assertFalse((archive_root / 'generated').exists())

    def test_changed_source_rejects_snapshot(self):
        runtime = self.base / 'reference/manifest.json'
        runtime.parent.mkdir()
        runtime.write_bytes(b'accepted runtime')
        before = hashlib.sha256(runtime.read_bytes()).hexdigest()
        snapshot = snapshot_source(self.root, self.work)
        original = (snapshot.root / 'TheLongDrive_Data/level0').read_bytes()
        from tools.reference.library import archive
        copy = archive.shutil.copy2
        def mutate(src, dst):
            result = copy(src, dst)
            Path(src).write_bytes(Path(src).read_bytes() + b'changed')
            return result
        fresh = self.base / 'second-library'
        with patch.object(archive.shutil, 'copy2', side_effect=mutate):
            with self.assertRaisesRegex(ValueError, 'changed'):
                snapshot_source(self.root, fresh)
        self.assertFalse((fresh / 'source').exists())
        self.assertEqual((snapshot.root / 'TheLongDrive_Data/level0').read_bytes(), original)
        self.assertEqual(hashlib.sha256(runtime.read_bytes()).hexdigest(), before)

    def test_streams_and_builtins_discovered(self):
        for name in ('Resources/unity default resources', 'Resources/unity_builtin_extra'):
            path = self.data / name
            path.parent.mkdir(exist_ok=True)
            path.write_bytes(serialized_bytes('2021.3.13f1' if name.endswith('unity default resources') else '2021.3.18f1'))
        for name in ('level0.resS', 'sharedassets0.resource'):
            (self.data / name).write_bytes(b'audio or texture stream')
        assets, streams = discover_source(self.data)
        self.assertEqual(assets, ('Resources/unity default resources', 'Resources/unity_builtin_extra', 'level0'))
        self.assertEqual(streams, ('level0.resS', 'sharedassets0.resource'))
        fake = FakeEnvironment()
        with patch('UnityPy.Environment', return_value=fake), patch('UnityPy.helpers.TypeTreeGenerator.TypeTreeGenerator'):
            source = Source(self.root)
        self.assertEqual(fake.loaded[:2], list(streams))
        self.assertEqual(set(source.serialized_files), set(assets))

    def test_escape_and_case_collision_rejected(self):
        outside = self.base / 'outside'
        outside.mkdir()
        (outside / 'file').write_bytes(b'outside')
        linked = self.data / 'escape'
        try:
            linked.symlink_to(outside, target_is_directory=True)
        except OSError:
            import subprocess
            subprocess.run(['cmd', '/c', 'mklink', '/J', str(linked), str(outside)], check=True, capture_output=True)
        try:
            with self.assertRaisesRegex(ValueError, 'link|escape'):
                scan_files(self.root)
        finally:
            if linked.is_symlink():
                linked.unlink()
            else:
                linked.rmdir()
        from tools.reference.library.archive import validate_paths
        with self.assertRaisesRegex(ValueError, 'collision'):
            validate_paths(['TheLongDrive_Data/A.assets', 'TheLongDrive_Data/a.assets'])
        with self.assertRaisesRegex(ValueError, 'path'):
            validate_paths(['../outside'])

    def test_wrong_builtin_version_rejects_snapshot(self):
        builtin = self.data / 'Resources/unity default resources'
        builtin.parent.mkdir()
        builtin.write_bytes(serialized_bytes())
        with self.assertRaisesRegex(ValueError, 'Unity identity'):
            snapshot_source(self.root, self.work)
        self.assertFalse((self.work / 'source').exists())
        builtin.write_bytes(serialized_bytes('2021.3.13f1'))
        verify_snapshot(snapshot_source(self.root, self.work))

    def test_duplicate_path_id_in_distinct_containers(self):
        (self.data / 'Nested').mkdir()
        (self.data / 'Nested/level0').write_bytes(serialized_bytes())
        fake = FakeEnvironment()
        with patch('UnityPy.Environment', return_value=fake), patch('UnityPy.helpers.TypeTreeGenerator.TypeTreeGenerator'):
            source = Source(self.root)
        objects = list(source.objects.values())
        self.assertEqual(len(objects), 2)
        self.assertNotEqual(key(objects[0]), key(objects[1]))
        self.assertNotEqual(source_id(objects[0]), source_id(objects[1]))
        snapshot = snapshot_source(self.root, self.work)
        # Readers refer to the archive root during inventory.
        source.root = snapshot.root
        catalog = inventory_source(source, snapshot)
        self.assertEqual(len(catalog.objects), 2)
        self.assertEqual({item.source.file for item in catalog.objects.values()}, {'level0', 'Nested/level0'})
        self.assertTrue(all(item.offset == 128 and item.bytes == 8 for item in catalog.objects.values()))

    def test_verify_rejects_changed_hash_list_and_identity_without_writes(self):
        snapshot = snapshot_source(self.root, self.work)
        before = {p.relative_to(self.work).as_posix(): (p.stat().st_mtime_ns, p.read_bytes())
                  for p in self.work.rglob('*') if p.is_file()}
        verify_snapshot(snapshot)
        after = {p.relative_to(self.work).as_posix(): (p.stat().st_mtime_ns, p.read_bytes())
                 for p in self.work.rglob('*') if p.is_file()}
        self.assertEqual(before, after)
        path = snapshot.root / 'TheLongDrive_Data/level0'
        path.write_bytes(path.read_bytes()[:-1] + b'x')
        with self.assertRaisesRegex(ValueError, 'hash|changed'):
            verify_snapshot(snapshot)
        path.write_bytes(serialized_bytes())
        extra = snapshot.root / 'unexpected'
        extra.write_bytes(b'extra')
        with self.assertRaisesRegex(ValueError, 'list|changed'):
            verify_snapshot(snapshot)
        extra.unlink()
        manifest = json.loads(snapshot.manifest_path.read_text())
        manifest['build_id'] += 1
        snapshot.manifest_path.write_text(json.dumps(manifest))
        with self.assertRaisesRegex(ValueError, 'manifest|identity|build'):
            verify_snapshot(snapshot)


class FakeEnvironment:
    def __init__(self):
        self.loaded = []
        self.files = {}
        self.assets = []
        self.objects = []
    def load_file(self, path):
        marker = 'TheLongDrive_Data/'
        name = Path(path).as_posix().split(marker)[1]
        self.loaded.append(name)
        if name.endswith(('.resS', '.resource')):
            return SimpleNamespace()
        asset = SimpleNamespace(name=Path(path).name, unity_version='2021.3.13f1' if name.endswith('unity default resources') else '2021.3.18f1', externals=[])
        reader = SimpleNamespace(assets_file=asset, path_id=1, byte_start=128, byte_size=8,
                                 class_id=1, type=SimpleNamespace(name='GameObject'))
        asset.objects = {1: reader}
        self.assets.append(asset)
        self.objects.append(reader)
        return asset


if __name__ == '__main__':
    unittest.main()
