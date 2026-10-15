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

if __name__ == '__main__':
    unittest.main()
