"""Critical audio archive and WAV checks."""
import hashlib
import struct
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from tools.reference.library.audio import export_audio, validate_wav
from tools.reference.library.types import Catalog, ObjectId, ObjectRecord


def wav(format=1, bits=16, channels=1, frequency=8000, frames=8):
    block = channels * bits // 8
    payload = bytes(frames * block)
    fmt = struct.pack('<HHIIHH', format, channels, frequency, frequency * block, block, bits)
    chunks = b'fmt ' + struct.pack('<I', 16) + fmt + b'data' + struct.pack('<I', len(payload)) + payload
    return b'RIFF' + struct.pack('<I', len(chunks) + 4) + b'WAVE' + chunks


class AudioTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        (self.root / 'TheLongDrive_Data').mkdir()
        (self.root / 'TheLongDrive_Data/a.resource').write_bytes(b'FSB5' + bytes(12))
        self.source = SimpleNamespace(root=self.root, resource_streams=('a.resource',))
        self.clip = {'m_Name': 'same', 'm_Channels': 1, 'm_Frequency': 8000,
                     'm_BitsPerSample': 16, 'm_Length': .001, 'm_CompressionFormat': 1,
                     'm_Resource': {'m_Source': 'a.resource', 'm_Offset': 0, 'm_Size': 16}}

    def export(self, entries):
        self.source.get = lambda file, path_id: SimpleNamespace(
            identity=ObjectId(file, path_id), parse_as_object=lambda check_read: entries[ObjectId(file, path_id)][1])
        self.source.read_record = lambda reader: entries[reader.identity][0]
        model = Catalog({i: ObjectRecord(i, 'AudioClip', 83, 100, 200, 'container-hash') for i in entries},
                        {'AudioClip': len(entries)}, {i: () for i in entries})
        return export_audio(self.source, model, self.root / 'output')

    def test_duplicate_clip_names_do_not_overwrite(self):
        raw = wav()
        ids = (ObjectId('a.assets', 1), ObjectId('b.assets', 1))
        exports = self.export({i: (self.clip, SimpleNamespace(samples={'same.wav': raw})) for i in ids})
        self.assertEqual([e.source for e in exports], list(ids))
        self.assertNotEqual(exports[0].files[0].file, exports[1].files[0].file)
        for e in exports:
            file = e.files[0]
            self.assertEqual((self.root / 'output' / file.file).read_bytes(), raw)
            self.assertEqual(file.sha256, hashlib.sha256(raw).hexdigest())
            self.assertEqual(e.details['typed_record'], f'records/{e.source.file}/{e.source.path_id}.json')
            self.assertEqual(e.details['source_audio']['sha256'], hashlib.sha256(b'FSB5' + bytes(12)).hexdigest())
        self.assertTrue((self.root / 'output/audio/index.json').is_file())

    def test_missing_stream_and_truncated_wav_fail(self):
        self.source.resource_streams = ()
        with self.assertRaisesRegex(ValueError, 'Missing stream'):
            self.export({ObjectId('a.assets', 1): (self.clip, SimpleNamespace(samples={'same.wav': wav()}))})
        path = self.root / 'truncated.wav'
        path.write_bytes(wav()[:-1])
        with self.assertRaises(ValueError):
            validate_wav(path, self.clip)
        path.write_bytes(wav() + b'extra')
        with self.assertRaises(ValueError):
            validate_wav(path, self.clip)

    def test_pcm_and_float_headers_match_source(self):
        for format, bits in ((1, 16), (3, 32)):
            with self.subTest(format=format):
                raw = wav(format, bits)
                path = self.root / 'sample.wav'
                path.write_bytes(raw)
                clip = {**self.clip, 'm_BitsPerSample': bits}
                result = validate_wav(path, clip)
                self.assertEqual(result['format'], 'pcm' if format == 1 else 'ieee_float')
                self.assertEqual(result['frames'], 8)
                self.assertEqual(result['duration_seconds'], .001)
                self.assertEqual(path.read_bytes(), raw)
                for field, value in (('m_Channels', 2), ('m_Frequency', 44100), ('m_Length', 1)):
                    with self.assertRaises(ValueError):
                        validate_wav(path, {**clip, field: value})

    def test_long_clip_allows_only_binary32_duration_rounding(self):
        # Sparse samples keep this boundary fixture small in memory.
        frames, frequency = 2 ** 25 + 4, 48000
        path = self.root / 'long.wav'
        header = bytearray(wav(bits=8, frequency=frequency, frames=0))
        struct.pack_into('<I', header, 4, frames + 36)
        struct.pack_into('<I', header, 40, frames)
        with path.open('wb') as stream:
            stream.write(header)
            stream.seek(frames + 43)
            stream.write(b'\0')
        clip = {**self.clip, 'm_BitsPerSample': 8, 'm_Frequency': frequency,
                'm_Length': struct.unpack('<f', struct.pack('<f', frames / frequency))[0]}
        result = validate_wav(path, clip)
        self.assertGreater(abs(result['duration_delta_frames']), 1)
        self.assertGreater(result['duration_tolerance_frames'], 1)
        with self.assertRaisesRegex(ValueError, 'WAV duration mismatch'):
            validate_wav(path, {**clip, 'm_Length': (frames + 5) / frequency})

    def test_empty_samples_and_stream_range_fail(self):
        identity = ObjectId('a.assets', 1)
        with self.assertRaisesRegex(ValueError, 'No decoded samples'):
            self.export({identity: (self.clip, SimpleNamespace(samples={}))})
        bad = {**self.clip, 'm_Resource': {**self.clip['m_Resource'], 'm_Size': 17}}
        with self.assertRaisesRegex(ValueError, 'Stream range'):
            self.export({identity: (bad, SimpleNamespace(samples={'same.wav': wav()}))})
        empty = {**self.clip, 'm_Resource': {**self.clip['m_Resource'], 'm_Size': 0}}
        with self.assertRaisesRegex(ValueError, 'no source samples'):
            self.export({identity: (empty, SimpleNamespace(samples={'same.wav': wav()}))})

    def test_invalid_alignment_and_identity_collision_fail(self):
        raw = bytearray(wav())
        struct.pack_into('<H', raw, 32, 3)
        path = self.root / 'bad.wav'
        path.write_bytes(raw)
        with self.assertRaises(ValueError):
            validate_wav(path, self.clip)
        ids = (ObjectId('A.assets', 1), ObjectId('a.assets', 1))
        with self.assertRaisesRegex(ValueError, 'identity collision'):
            self.export({i: (self.clip, SimpleNamespace(samples={'same.wav': wav()})) for i in ids})


if __name__ == '__main__':
    unittest.main()
