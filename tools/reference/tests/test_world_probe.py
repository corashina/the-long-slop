import json
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory

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

if __name__ == '__main__':
    unittest.main()
