import copy
import json
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import patch

from tools.reference import world_probe


class WorldProbeTests(unittest.TestCase):
    def test_probe_rejects_changed_engine_and_missing_native_results(self):
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            changed_hash_path = root / 'changed'
            missing_native_rows_path = root / 'missing'
            changed_hash_path.mkdir()
            missing_native_rows_path.mkdir()
            changed = {'native_engine': {'sha256': '0' * 64}, 'native': []}
            missing = {'native_engine': {'sha256': world_probe.NATIVE_ENGINE_SHA256}, 'native': []}
            (changed_hash_path / 'samples.json').write_text(json.dumps(changed))
            (missing_native_rows_path / 'samples.json').write_text(json.dumps(missing))
            with self.assertRaisesRegex(ValueError, 'engine hash'):
                world_probe.verify(changed_hash_path)
            with self.assertRaisesRegex(ValueError, 'native.*rows'):
                world_probe.verify(missing_native_rows_path)

    def test_native_noise_samples_cover_both_coordinates(self):
        rows = world_probe.sample_native_noise()
        coordinates = {(row['x'], row['y']) for row in rows}
        self.assertTrue(any(x < 0 and y > 0 for x, y in coordinates))
        self.assertTrue(any(x > 0 and y < 0 for x, y in coordinates))
        self.assertGreater(len({row['bits'] for row in rows}), 3)

    def test_capture_logs_reject_sampling_and_unknown_shutdown_errors(self):
        with TemporaryDirectory(dir=world_probe.PROBE_ROOT) as temporary:
            root = Path(temporary)
            log = root / 'first' / 'seed-42' / 'unity.log'
            log.parent.mkdir(parents=True)
            marker = 'WORLD_SOURCE_CAPTURE_FLUSHED error=none'
            for content in ('NullReferenceException\n' + marker,
                            marker + '\nInvalidOperationException', marker + '\nERROR: unexpected source failure'):
                log.write_text(content)
                with self.assertRaisesRegex(ValueError, 'Source (exception outside checked shutdown|log error)'):
                    world_probe._verify_runtime_logs(root)

    def test_complete_capture_and_replay_are_checked(self):
        with patch.object(world_probe, '_run', side_effect=AssertionError('Verification must not run samplers')):
            world_probe.verify(world_probe.PROBE_ROOT)
        envelope = json.loads((world_probe.PROBE_ROOT / 'samples.json').read_text())
        self.assertEqual(envelope['payload_sha256'], envelope['replay_sha256'])
        with TemporaryDirectory() as temporary:
            root = Path(temporary)
            envelope['assembled'][0]['roads'].pop()
            (root / 'samples.json').write_text(json.dumps(envelope))
            with self.assertRaisesRegex(ValueError, 'payload hash'):
                world_probe.verify(root)
            envelope['payload_sha256'] = world_probe.payload_digest(envelope)
            envelope['replay_sha256'] = envelope['payload_sha256']
            (root / 'samples.json').write_text(json.dumps(envelope))
            with self.assertRaisesRegex(ValueError, 'Missing assembled road rows'):
                world_probe.verify(root)

    def test_required_source_inputs_and_receipts_cannot_be_deleted(self):
        original = json.loads((world_probe.PROBE_ROOT / 'samples.json').read_text())
        def terrain_inputs(envelope):
            for group in ('terrain', 'terrainDirect'):
                for tile in envelope['assembled'][0][group]:
                    for key in ('roadCoords', 'buildingCoords', 'buildingRanges', 'holeCoords'):
                        del tile[key]
        mutations = {
            'startup': lambda e: e['assembled'][0].pop('startObjects'),
            'terrain inputs': terrain_inputs,
            'frame': lambda e: e['assembled'][0]['roads'][0]['roadFrame'].pop('position'),
            'quaternion': lambda e: e['assembled'][0]['numeric'][0].pop('quaternion'),
            'native range': lambda e: e['native']['ranges'].pop(),
            'catalog receipt': lambda e: e['library'].pop('catalog_sha256'),
            'source receipt': lambda e: e['library'].pop('source_manifest_sha256'),
        }
        for name, mutate in mutations.items():
            with self.subTest(name=name):
                envelope = copy.deepcopy(original)
                mutate(envelope)
                self.assert_rehashed_envelope_rejected(envelope)

    def test_changed_output_cannot_reuse_retained_replay(self):
        envelope = json.loads((world_probe.PROBE_ROOT / 'samples.json').read_text())
        envelope['assembled'][0]['roads'][0]['px'] = 123456789
        self.assert_rehashed_envelope_rejected(envelope)

    def assert_rehashed_envelope_rejected(self, envelope):
        envelope['payload_sha256'] = world_probe.payload_digest(envelope)
        envelope['replay_sha256'] = envelope['payload_sha256']
        serialized = json.dumps(envelope)
        original_read = Path.read_text
        target = world_probe.PROBE_ROOT / 'samples.json'
        def read(path, *args, **kwargs):
            return serialized if path == target else original_read(path, *args, **kwargs)
        with patch.object(Path, 'read_text', read), self.assertRaises(ValueError):
            world_probe.verify(world_probe.PROBE_ROOT)

    def test_missing_retained_output_is_rejected(self):
        run = Path((world_probe.PROBE_ROOT / 'last-run.txt').read_text())
        original_open = Path.open
        for relative in ('first/native.json', 'first/placement.csv', 'replay/seed-42/runtime.json'):
            with self.subTest(artifact=relative):
                target = run / relative
                def open_file(path, *args, **kwargs):
                    if path == target:
                        raise FileNotFoundError(target)
                    return original_open(path, *args, **kwargs)
                with patch.object(Path, 'open', open_file), self.assertRaisesRegex(ValueError, 'retained source artifact'):
                    world_probe.verify(world_probe.PROBE_ROOT)


if __name__ == '__main__':
    unittest.main()
