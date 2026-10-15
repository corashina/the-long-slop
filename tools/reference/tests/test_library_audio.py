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

if __name__ == '__main__':
    unittest.main()
