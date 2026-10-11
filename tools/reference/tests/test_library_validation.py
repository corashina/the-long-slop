import hashlib
import json
import tempfile
import unittest
from dataclasses import asdict, replace
from pathlib import Path
from unittest.mock import patch

from tools.reference.library.records import _file, _json
from tools.reference.library.types import AssetExport, Catalog, ObjectId, ObjectRecord, Snapshot
from tools.reference.library.validation import validate_library, publish_library, verify_library


class LibraryValidationTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.work = self.root / 'cache/work/library'
        self.work.mkdir(parents=True)
        self.cache = self.root / 'cache'
        self.source = ObjectId('objects', 1)
        self.snapshot = Snapshot(self.cache / 'source', (), self.cache / 'source-manifest.json', 'source-hash')
        (self.cache / 'source-manifest.json').write_text('{}')
        (self.cache / 'catalog.json').write_text('{}')
        (self.cache / 'records').mkdir()
        (self.cache / 'records/manifest.json').write_text('{}')
        self.catalog = Catalog({self.source: ObjectRecord(self.source, 'AnimationClip', 74, 0, 1, 'x')},
                               {'AnimationClip': 1}, {self.source: ()})
        track = {'times': [0, 1], 'values': [[0], [1]], 'binding': {'type_id': 95}, 'interpolation': 'LINEAR'}
        f = _file(self.work, 'animations/clips/objects/1/tracks.json', _json({
            'source': asdict(self.source), 'tracks': [track], 'events': [], 'settings': {'curve_counts': {'streamed': 1, 'dense': 0, 'constant': 0}}}))
        self.export = AssetExport(self.source, 'animations', 'native_record', 'engine_dependent',
            'Normalized tracks need an adapter.', (f,), (), {'type': 'AnimationClip',
            'typed_record': 'records/objects/1.json', 'tracks_file': f.file, 'track_count': 1,
            'native_track_count': 1, 'curve_counts': {'streamed': 1, 'dense': 0, 'constant': 0}, 'packages': []})
        _file(self.cache / 'records', 'objects/1.json', _json({'data': {}}))
        self.inputs = patch('tools.reference.library.validation._verify_inputs')
        self.inputs.start()
        self.addCleanup(self.inputs.stop)
        self.index()

    def index(self):
        ownership = _file(self.work, 'animations/ownership.json', _json([]))
        _file(self.work, 'animations/index.json', _json({'schema_version': 1,
              'exports': [asdict(self.export)], 'ownership': asdict(ownership)}))

    def check(self):
        return validate_library(self.snapshot, self.catalog, (self.export,), self.work)

    def test_unconverted_clip_blocks_library(self):
        self.export = replace(self.export, files=(), details={**self.export.details, 'tracks_file': None})
        self.index()
        with self.assertRaisesRegex(ValueError, 'normalized tracks'):
            self.check()

    def test_hash_or_required_pointer_failure_blocks_library(self):
        (self.work / self.export.files[0].file).write_text('corrupt')
        with self.assertRaisesRegex(ValueError, 'hash'):
            self.check()
        self.catalog.dependencies[self.source] = (ObjectId('missing', 3),)
        with self.assertRaisesRegex(ValueError, 'dependency'):
            self.check()

    def test_native_reason_cannot_hide_required_decode_failure(self):
        self.catalog.objects[self.source] = replace(self.catalog.objects[self.source], type='AudioClip')
        self.catalog.counts.clear(); self.catalog.counts['AudioClip'] = 1
        self.export = replace(self.export, category='native', details={'type': 'AudioClip',
            'typed_record': 'records/objects/1.json'}, reason='Native bytes retained')
        with self.assertRaisesRegex(ValueError, 'required conversion'):
            self.check()

    def test_unknown_status_or_schema_blocks_library(self):
        with self.assertRaises(ValueError):
            replace(self.export, status='unsupported')
        data = json.loads((self.work / 'animations/index.json').read_bytes())
        data['schema_version'] = 2
        (self.work / 'animations/index.json').write_bytes(_json(data))
        with self.assertRaisesRegex(ValueError, 'schema'):
            self.check()

    def test_source_empty_requires_measured_source_evidence(self):
        self.catalog.objects[self.source] = replace(self.catalog.objects[self.source], type='Texture2D')
        self.catalog.counts.clear(); self.catalog.counts['Texture2D'] = 1
        self.export = replace(self.export, category='images', status='source_empty', files=(),
            details={'type': 'Texture2D', 'typed_record': 'records/objects/1.json', 'dimensions': [0, 0]})
        _file(self.cache / 'records', 'objects/1.json', _json({'data': {
            'm_Width': 8, 'm_Height': 8, 'image data': [1], 'm_StreamData': {'size': 0}}}))
        _file(self.work, 'images/index.json', _json([asdict(self.export)]))
        with self.assertRaisesRegex(ValueError, 'source-empty'):
            self.check()

    def test_late_failure_preserves_accepted_manifest(self):
        output = self.root / 'web/assets/library'
        output.mkdir(parents=True)
        (output / 'manifest.json').write_bytes(b'accepted')
        runtime = self.root / 'web/assets/reference/manifest.json'
        runtime.parent.mkdir(); runtime.write_bytes(b'runtime')
        with patch('tools.reference.library.validation._publish_directory', side_effect=OSError('late failure')):
            with self.assertRaisesRegex(OSError, 'late failure'):
                publish_library(self.snapshot, self.catalog, (self.export,), self.work, output)
        self.assertEqual((output / 'manifest.json').read_bytes(), b'accepted')
        self.assertEqual(runtime.read_bytes(), b'runtime')

    def test_repeat_export_is_deterministic(self):
        import shutil
        output = self.root / 'web/assets/library'
        backup = self.root / 'template'; shutil.copytree(self.work, backup)
        a = publish_library(self.snapshot, self.catalog, (self.export,), self.work, output)
        first = {p.relative_to(output).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
                 for p in output.rglob('*') if p.is_file()}
        shutil.copytree(backup, self.work)
        b = publish_library(self.snapshot, self.catalog, (self.export,), self.work, output)
        self.assertEqual(a.manifest_sha256, b.manifest_sha256)
        self.assertEqual(first, {p.relative_to(output).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
                                for p in output.rglob('*') if p.is_file()})

    def test_verify_is_read_only_and_binds_shared_record_root(self):
        output = self.root / 'web/assets/library'
        publish_library(self.snapshot, self.catalog, (self.export,), self.work, output)
        before = {p: (p.stat().st_mtime_ns, p.read_bytes()) for p in self.root.rglob('*') if p.is_file()}
        with patch('tools.reference.library.validation.load_snapshot', return_value=self.snapshot), patch(
                'tools.reference.library.validation.load_catalog', return_value=self.catalog):
            verified = verify_library(output, self.snapshot.root)
            self.assertEqual(verified.exports, (self.export,))
            self.assertEqual(before, {p: (p.stat().st_mtime_ns, p.read_bytes()) for p in before})
            manifest = json.loads((output / 'manifest.json').read_bytes())
            manifest['records_root'] = '../../../foreign/records'
            (output / 'manifest.json').write_bytes(_json(manifest))
            with self.assertRaisesRegex(ValueError, 'records root'):
                verify_library(output, self.snapshot.root)

    def test_duplicate_ownership_and_conflicting_files_fail(self):
        with self.assertRaisesRegex(ValueError, 'Duplicate'):
            validate_library(self.snapshot, self.catalog, (self.export, self.export), self.work)
        self.export = replace(self.export, files=(*self.export.files, replace(self.export.files[0], bytes=1)))
        self.index()
        with self.assertRaisesRegex(ValueError, 'Conflicting'):
            self.check()

    def test_directory_swap_failure_restores_accepted_output(self):
        from tools.reference.library.validation import _publish_directory
        output = self.root / 'records/managed'
        output.mkdir(parents=True); (output / 'manifest.json').write_bytes(b'accepted')
        original = Path.replace
        def replace_path(path, target):
            if path == self.work:
                raise OSError('injected rename failure')
            return original(path, target)
        with patch.object(Path, 'replace', replace_path):
            with self.assertRaisesRegex(OSError, 'injected rename failure'):
                _publish_directory(self.work, output)
        self.assertEqual((output / 'manifest.json').read_bytes(), b'accepted')
        self.assertTrue(self.work.is_dir())

    def test_backup_cleanup_failure_keeps_committed_output(self):
        from tools.reference.library.validation import _publish_directory
        output = self.root / 'web/assets/library'
        output.mkdir(parents=True)
        (output / 'manifest.json').write_bytes(b'previous')
        (self.work / 'manifest.json').write_bytes(b'committed')
        with patch('tools.reference.library.validation.shutil.rmtree', side_effect=PermissionError('locked backup')):
            with self.assertLogs('tools.reference.library.validation', level='WARNING') as logs:
                _publish_directory(self.work, output)
        self.assertEqual((output / 'manifest.json').read_bytes(), b'committed')
        backups = list(self.work.parent.glob('previous-library-*'))
        self.assertEqual(len(backups), 1)
        self.assertEqual((backups[0] / 'manifest.json').read_bytes(), b'previous')
        self.assertIn(str(backups[0]), logs.output[0])
        self.assertIn('locked backup', logs.output[0])

    def test_partial_backup_cleanup_does_not_block_next_publication(self):
        from tools.reference.library.validation import _publish_directory
        output = self.root / 'web/assets/library'
        output.mkdir(parents=True)
        (output / 'manifest.json').write_bytes(b'previous')
        (output / 'retained.bin').write_bytes(b'recovery data')
        (self.work / 'manifest.json').write_bytes(b'first commit')
        def partial_cleanup(backup):
            (backup / 'manifest.json').unlink()
            raise PermissionError('partial cleanup')
        with patch('tools.reference.library.validation.shutil.rmtree', side_effect=partial_cleanup):
            with self.assertLogs('tools.reference.library.validation', level='WARNING'):
                _publish_directory(self.work, output)
        self.assertEqual((output / 'manifest.json').read_bytes(), b'first commit')
        retained = list(self.work.parent.glob('previous-library-*'))
        self.assertEqual(len(retained), 1)
        self.work.mkdir()
        (self.work / 'manifest.json').write_bytes(b'second commit')
        _publish_directory(self.work, output)
        self.assertEqual((output / 'manifest.json').read_bytes(), b'second commit')
        self.assertEqual((retained[0] / 'retained.bin').read_bytes(), b'recovery data')
        self.assertEqual(list(self.work.parent.glob('previous-library-*')), retained)

    def test_native_text_requires_exact_payload_file(self):
        from tools.reference.library.validation import _required
        native = replace(self.export, category='native', files=(), details={
            'type': 'TextAsset', 'typed_record': 'records/objects/1.json'})
        with self.assertRaisesRegex(ValueError, 'TextAsset source payload'):
            _required(native, 'TextAsset', self.cache / 'records', self.work)

    def test_failed_fresh_managed_stage_preserves_shared_records(self):
        from types import SimpleNamespace
        from unittest.mock import Mock
        from tools.reference.export import _category_stages
        records = self.cache / 'records'
        (records / 'managed').mkdir()
        (records / 'managed/previous-diagnostic.json').write_bytes(b'previous')
        before = {p.relative_to(records): p.read_bytes() for p in records.rglob('*') if p.is_file()}
        staged = []
        def native(source, catalog, root, claimed):
            self.assertTrue(root.is_relative_to(self.cache / 'work'))
            self.assertEqual(claimed, frozenset((self.source,)))
            self.assertEqual((root / 'manifest.json').read_bytes(), (records / 'manifest.json').read_bytes())
            staged.append(root)
            return ()
        def managed(snapshot, root):
            (root / 'managed').mkdir()
            (root / 'managed/diagnostics.json').write_bytes(b'failed analysis')
            raise ValueError('managed analysis failed')
        replacements = {name: Mock(return_value=()) for name in (
            'export_images', 'export_ui', 'export_audio', 'export_models', 'export_animations', 'index_source_text')}
        replacements['export_images'] = Mock(return_value=(self.export,))
        replacements['export_native_assets'] = Mock(side_effect=native)
        replacements['index_managed'] = Mock(side_effect=managed)
        with patch('tools.reference.source.Source', return_value=SimpleNamespace()), patch.multiple(
                'tools.reference.library', **replacements):
            with self.assertRaisesRegex(ValueError, 'managed analysis failed'):
                _category_stages(self.snapshot, Catalog({}, {}, {}))
        self.assertEqual(before, {p.relative_to(records): p.read_bytes() for p in records.rglob('*') if p.is_file()})
        self.assertEqual((staged[0] / 'managed/diagnostics.json').read_bytes(), b'failed analysis')
        self.assertFalse((self.cache / 'category-stages.json').exists())
