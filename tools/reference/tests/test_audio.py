"""Critical selected-audio contract checks."""
import copy
import hashlib
import json
import tempfile
import unittest
import wave
from pathlib import Path
from unittest.mock import patch
from tools.reference.library.reader import LibraryReader
from tools.reference.library.types import ObjectId

class SelectedAudio(unittest.TestCase):
    def test_runtime_stage_stays_local_to_checkout(self):
        from tools.reference import export
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            borrowed = root / 'borrowed'
            borrowed.mkdir()
            local = root / 'local'
            local.mkdir()
            with patch.object(export, 'SOURCE_WORK_ROOT', borrowed), patch.object(export, 'RUNTIME_WORK_ROOT', local):
                self.assertEqual(export.runtime_stage_root().resolve(), (local / 'work').resolve())
            with patch.object(export, 'SOURCE_WORK_ROOT', borrowed), patch.object(export, 'RUNTIME_WORK_ROOT', borrowed):
                with self.assertRaisesRegex(ValueError, 'overlaps accepted source'):
                    export.runtime_stage_root()

    def fixture(self, directory):
        root = Path(directory)
        path = root / 'audio/sharedassets1.assets/4423/sample-000.wav'
        path.parent.mkdir(parents=True)
        with wave.open(str(path), 'wb') as stream:
            stream.setparams((1, 2, 8000, 80, 'NONE', 'not compressed'))
            stream.writeframes(b'\0' * 160)
        record = {'m_Channels': 1, 'm_Frequency': 8000, 'm_BitsPerSample': 16, 'm_Length': 0.01}
        from tools.reference.library.audio import validate_wav
        samples = validate_wav(path, record)
        identity = ObjectId('sharedassets1.assets', 4423)
        reader = LibraryReader.__new__(LibraryReader)
        reader.root = root
        reader.selected_audio = set()
        reader.record = lambda _: record
        reader.exports = {identity: {'status': 'converted', 'details': {'type': 'AudioClip',
            'samples': [{'file': path.relative_to(root).as_posix(), **samples}]},
            'files': [{'file': path.relative_to(root).as_posix(), 'bytes': path.stat().st_size,
                'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}]}}
        return reader, identity, path

    def test_audio_uses_accepted_samples(self):
        with tempfile.TemporaryDirectory() as directory:
            reader, identity, original = self.fixture(directory)
            self.assertTrue(hasattr(reader, 'audio'), 'missing selected WAV interface')
            from tools.reference.audio import copy_audio
            metadata = copy_audio(reader,identity,1,reader.root,'effect')
            path = reader.root / metadata['path'].removeprefix('reference/')
            self.assertNotEqual(path, original)
            self.assertEqual(path.read_bytes(), original.read_bytes())
            self.assertEqual(metadata['sha256'], hashlib.sha256(original.read_bytes()).hexdigest())
            self.assertEqual(reader.selected_audio, {identity})

    def test_required_links_and_samples_fail(self):
        with tempfile.TemporaryDirectory() as directory:
            reader, identity, path = self.fixture(directory)
            self.assertTrue(hasattr(reader, 'audio'), 'missing selected WAV interface')
            path.write_bytes(path.read_bytes()[:-2] + b'xx')
            with self.assertRaises(ValueError): reader.audio(identity)

    def test_audio_closure_and_overrides(self):
        from tools.reference.audio import validate_audio_export
        with tempfile.TemporaryDirectory() as directory:
            reader, identity, path = self.fixture(directory)
            from tools.reference.audio import copy_audio
            metadata = copy_audio(reader,identity,1,reader.root,'effect')
            audio = copy.deepcopy(json.loads(Path('tests/common/reference.json').read_text())['audio'])
            audio['clips'] = [metadata]
            for v in audio['voices']:
                if v['clip'] is not None: v['clip'] = 1
                if v['authored_clip'] is not None: v['authored_clip'] = 1
            for b in audio['bindings']: b['clips'] = [1]
            validate_audio_export(audio,reader.root)
            idle=next(v for v in audio['voices'] if v['id']==audio['engine']['idle_voice'])
            self.assertEqual(idle['authored_priority'],128)
            self.assertEqual(idle['priority'],80)
            self.assertEqual(idle['curves']['rolloff']['pre_wrap_mode'],2)
            self.assertTrue(idle['methods'])
            for field,value in [('clip',2),('emitter',999999),('mixer',999999),('max_distance',0)]:
                broken=copy.deepcopy(audio);broken['voices'][0][field]=value
                with self.subTest(field=field), self.assertRaises(ValueError):validate_audio_export(broken,reader.root)
            with self.assertRaises(ValueError):reader.audio(ObjectId('missing.assets',1))

    def test_custom_content_is_external(self):
        from tools.reference.audio import station_records
        class Source:
            def resolve(self, owner, pointer): raise AssertionError('custom content must stay external')
        value = station_records(Source(), None, {'radioChs': [], 'radioChsAM': [
            {'FR': 1080, 'name': 'custom', 'custom': 1, 'music': [], 'talk': [],
             'musicHoliday': [], 'talkHoliday': [], 'musicVolume': [], 'talkVolume': [],
             'Bmusic': 0, 'startTime': 0, 'cTime': 0, 'cMaxTime': 0, 'currentVolume': 0}]}, lambda _: 0)
        self.assertEqual(value[0]['frequency'], 1080)
        self.assertEqual(value[0]['content'], 'external')
        self.assertEqual(value[0]['music'], [])
        self.assertEqual(value[0]['talk'], [])

    def test_radio_clock_receipts_select_scaled_source(self):
        radio = json.loads(Path('tests/common/reference.json').read_text())['audio']['radio']
        self.assertEqual(radio['clock']['kind'], 'scaled')
        self.assertEqual(radio['clock']['selected']['method'], 'RADIO')
        self.assertEqual(radio['clock']['multiplayer']['method'], 'RADIO2')
