"""Mandatory capture-input identity proof with small generated files."""
import hashlib
import json
import tempfile
import unittest
from copy import deepcopy
from pathlib import Path
from types import SimpleNamespace as NS
from unittest.mock import patch

from tools.reference import world_probe
from tools.reference.world_capture import verify_capture_inputs


class CaptureTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        root = Path(self.temporary.name)
        self.probe = root / 'probe'
        self.library = root / 'library'
        self.records = root / 'records'
        for directory in (self.probe, self.library, self.records):
            directory.mkdir()
        self.managed = self.write(self.records, 'managed/Assembly-CSharp.cs', b'checked world initializer')
        typed = self.write(self.records, 'manifest.json', b'checked typed record receipts')
        coverage = self.write(self.library, 'coverage.json', b'new evidence')
        indexes = [self.write(self.library, f'{category}/index.json', category.encode())
                   for category in ('animations', 'audio', 'images', 'models', 'native', 'ui')]
        standalone = self.write(self.library, 'standalone/source.bin', b'converted input')
        self.current = dict(schema_version=1, records_root='../records',
            source_manifest_sha256='1' * 64, catalog_sha256='2' * 64,
            records_manifest=typed, category_indexes=indexes, standalone_files=[standalone],
            converter_versions={'converter': 'checked'}, coverage_totals={'complete': 7},
            coverage=coverage, files=[coverage, *indexes, standalone],
            managed_analysis={'root': 'records', 'files': [self.managed], 'evidence': 'new'})
        self.original = deepcopy(self.current)
        self.original['records_root'] = 'UNTRUSTED-OLD-PATH/records'
        self.original['managed_analysis']['evidence'] = 'old'
        old_coverage = dict(coverage, bytes=12, sha256='3' * 64)
        self.original['coverage'] = old_coverage
        self.original['files'][0] = old_coverage
        self.reader = NS(root=self.library, records_root=self.records)
        self.save_current()
        self.save_original()
        self.verify = self.enter(patch.object(world_probe, 'verify'))
        self.enter(patch.object(world_probe, 'PROBE_ROOT', self.probe))
        self.enter(patch('tools.reference.world_capture.MANAGED_TEXT_SHA256', self.managed['sha256']))

    def enter(self, context):
        value = context.__enter__()
        self.addCleanup(context.__exit__, None, None, None)
        return value

    @staticmethod
    def write(root, name, content):
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(content)
        return dict(file=name, bytes=len(content), sha256=hashlib.sha256(content).hexdigest())

    def save_current(self):
        data = json.dumps(self.current, sort_keys=True).encode()
        record = self.write(self.library, 'manifest.json', data)
        self.reader.receipt = NS(manifest_path=self.library / 'manifest.json',
            manifest_sha256=record['sha256'], catalog_sha256=self.current['catalog_sha256'],
            source_manifest_sha256=self.current['source_manifest_sha256'])

    def save_original(self):
        data = json.dumps(self.original, sort_keys=True).encode()
        record = self.write(self.probe, 'capture-library-manifest.json', data)
        self.capture_hash = record['sha256']
        self.enter(patch.object(world_probe, 'LIBRARY_SHA256', self.capture_hash))
        sample = dict(library=dict(manifest_sha256=self.capture_hash,
            catalog_sha256=self.original['catalog_sha256'],
            source_manifest_sha256=self.original['source_manifest_sha256']))
        self.write(self.probe, 'samples.json', json.dumps(sample).encode())

    def test_accepts_only_evidence_and_storage_differences(self):
        expected = dict(schema_version=1, manifest_sha256=self.capture_hash,
            catalog_sha256='2' * 64, source_manifest_sha256='1' * 64)
        self.assertEqual(verify_capture_inputs(self.reader), expected)
        self.verify.assert_called_once_with(self.probe)

    def test_original_raw_hash_precedes_json_parse(self):
        (self.probe / 'capture-library-manifest.json').write_bytes(b'not JSON')
        with self.assertRaisesRegex(ValueError, 'capture manifest hash'):
            verify_capture_inputs(self.reader)

    def test_rejects_every_changed_input_field_and_key_set(self):
        original = deepcopy(self.current)
        mutations = [
            lambda m: m.update(source_manifest_sha256='4' * 64),
            lambda m: m.update(catalog_sha256='4' * 64),
            lambda m: m['records_manifest'].update(sha256='4' * 64),
            lambda m: m['category_indexes'][0].update(sha256='4' * 64),
            lambda m: m['converter_versions'].update(converter='changed'),
            lambda m: m['standalone_files'][0].update(sha256='4' * 64),
            lambda m: m['coverage_totals'].update(complete=8),
            lambda m: m.update(schema_version=2),
            lambda m: m.update(extra='changed'),
            lambda m: m.pop('standalone_files'),
        ]
        for mutate in mutations:
            self.current = deepcopy(original)
            mutate(self.current)
            self.save_current()
            with self.subTest(mutate=mutate), self.assertRaises(ValueError):
                verify_capture_inputs(self.reader)

    def test_rejects_file_map_changes_duplicates_and_normalized_collisions(self):
        original = deepcopy(self.current)
        mutations = [
            lambda m: m['files'].append(dict(m['files'][-1], file='extra.bin')),
            lambda m: m['files'].pop(),
            lambda m: m['files'].append(deepcopy(m['files'][-1])),
            lambda m: m['files'].append(dict(m['files'][-1], file='Standalone/source.bin')),
            lambda m: m['files'][-1].update(sha256='4' * 64),
            lambda m: m['files'][-1].update(bytes=999),
            lambda m: m['files'][-1].update(file='../escape.bin'),
        ]
        for mutate in mutations:
            self.current = deepcopy(original)
            mutate(self.current)
            self.save_current()
            with self.subTest(mutate=mutate), self.assertRaises(ValueError):
                verify_capture_inputs(self.reader)

    def test_coverage_receipt_cannot_hide_payload_or_differ_from_file_map(self):
        for side in ('current', 'original'):
            for fake_path in (True, False):
                with self.subTest(side=side, fake_path=fake_path):
                    old = deepcopy(getattr(self, side))
                    getattr(self, side)['coverage'] = dict(getattr(self, side)['coverage'])
                    receipt = getattr(self, side)['coverage']
                    receipt['file' if fake_path else 'sha256'] = 'standalone/source.bin' if fake_path else '9' * 64
                    if side == 'current': self.save_current()
                    else: self.save_original()
                    with self.assertRaises(ValueError): verify_capture_inputs(self.reader)
                    setattr(self, side, old)
                    if side == 'current': self.save_current()
                    else: self.save_original()

    def test_managed_receipt_changes_are_rejected_on_either_side(self):
        for side in ('original', 'current'):
            original = deepcopy(getattr(self, side))
            getattr(self, side)['managed_analysis']['files'][0]['sha256'] = '8' * 64
            if side == 'current': self.save_current()
            else: self.save_original()
            with self.subTest(side=side), self.assertRaises(ValueError): verify_capture_inputs(self.reader)
            setattr(self, side, original)
            if side == 'current': self.save_current()
            else: self.save_original()

    def test_current_manifest_must_match_verified_receipt(self):
        (self.library / 'manifest.json').write_bytes(b'changed')
        with self.assertRaisesRegex(ValueError, 'current manifest hash'):
            verify_capture_inputs(self.reader)

    def test_current_provenance_must_match_verified_receipt(self):
        for name in ('catalog_sha256', 'source_manifest_sha256'):
            original = getattr(self.reader.receipt, name)
            setattr(self.reader.receipt, name, 'e' * 64)
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, 'current verified provenance'):
                verify_capture_inputs(self.reader)
            setattr(self.reader.receipt, name, original)

    def test_sample_provenance_must_equal_original_manifest(self):
        path = self.probe / 'samples.json'
        original = json.loads(path.read_bytes())
        for name in ('manifest_sha256', 'catalog_sha256', 'source_manifest_sha256'):
            sample = deepcopy(original)
            sample['library'][name] = 'e' * 64
            path.write_text(json.dumps(sample))
            with self.subTest(name=name), self.assertRaises(ValueError):
                verify_capture_inputs(self.reader)

    def test_managed_text_and_typed_manifest_bytes_are_checked(self):
        for name in ('managed/Assembly-CSharp.cs', 'manifest.json'):
            path = self.records / name
            original = path.read_bytes()
            path.write_bytes(b'changed')
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, 'hash mismatch'):
                verify_capture_inputs(self.reader)
            path.write_bytes(original)

    def test_equal_library_hashes_still_require_proof(self):
        self.current = deepcopy(self.original)
        self.save_current()
        self.assertEqual(self.reader.receipt.manifest_sha256, self.capture_hash)
        (self.probe / 'capture-library-manifest.json').unlink()
        with self.assertRaises(OSError): verify_capture_inputs(self.reader)
        self.verify.assert_called_once_with(self.probe)

    def test_sample_verifier_failure_is_not_bypassed(self):
        self.verify.side_effect = ValueError('Source provenance receipt is missing or changed')
        with self.assertRaisesRegex(ValueError, 'Source provenance'):
            verify_capture_inputs(self.reader)

    def test_capture_manifest_link_is_rejected(self):
        target = self.probe / 'capture-library-manifest.json'
        original = Path.is_symlink
        with patch.object(Path, 'is_symlink', lambda path: path == target or original(path)):
            with self.assertRaisesRegex(ValueError, 'Filesystem link'):
                verify_capture_inputs(self.reader)

    def test_run_pointer_cannot_leave_private_probe_root(self):
        (self.probe / 'last-run.txt').write_text(str(self.library))
        with self.assertRaisesRegex(ValueError, 'leaves ignored probe storage'):
            world_probe._capture_root(self.probe)

    def test_export_requires_proof_before_source_selection(self):
        from tools.reference.world import recover_world
        with patch('tools.reference.world_capture.verify_capture_inputs', side_effect=ValueError('required proof')) as proof:
            with self.assertRaisesRegex(ValueError, 'required proof'):
                recover_world(None, self.reader, self.probe, 1)
        proof.assert_called_once_with(self.reader)
