"""Critical selected-audio contract checks."""
import copy
import hashlib
import json
import tempfile
import unittest
import wave
from pathlib import Path
from types import SimpleNamespace as NS
from unittest.mock import patch
from tools.reference.library.reader import LibraryReader
from tools.reference.library.types import ObjectId
from tools.reference.source import key, source_id

class SelectedAudio(unittest.TestCase):
    def accepted_source_fixture(self, directory):
        root=Path(directory)
        library,identity,_=self.fixture(directory)
        library.snapshot=NS(root=root)
        library.records_root=root/'accepted-records'
        library.records_root.mkdir()
        library.record_files={}
        library.selected_records=set()
        library.record=LibraryReader.record.__get__(library,LibraryReader)
        native=root/'UnityPlayer.dll'
        native.write_bytes(b'accepted native fixture')
        records={};objects={};classes={}
        def add(file,path_id,type_name,data=None,cls=None):
            obj=NS(assets_file=NS(source_container_path=file),path_id=path_id,type=NS(name=type_name))
            objects[file,path_id]=obj;records[file,path_id]=data if data is not None else {}
            if cls:classes[file,path_id]=cls
            return obj
        def ptr(obj):
            return {'file':obj.assets_file.source_container_path,'m_PathID':obj.path_id}
        null={'m_PathID':0}
        go=add('level1',100,'GameObject')
        parent=add('level1',39811,'Transform')
        transform=add('level1',39826,'Transform',{'m_LocalPosition':{'x':0,'y':0,'z':0},'m_Father':ptr(parent)})
        prefab_go=add('level1',101,'GameObject')
        clip=add(identity.file,identity.path_id,'AudioClip')
        records[key(clip)]={'m_Channels':1,'m_Frequency':8000,'m_BitsPerSample':16,'m_Length':0.01}
        mixer=add('sharedassets0.assets',810,'AudioMixerController')
        group=add('sharedassets0.assets',811,'AudioMixerGroupController',
            {'m_AudioMixer':ptr(mixer),'m_GroupID':'main','m_Name':'Main'})
        snapshot=add('sharedassets0.assets',821,'AudioMixerSnapshotController',{'m_SnapshotID':'active'})
        records[key(mixer)]={'m_StartSnapshot':ptr(snapshot),'m_MixerConstant':{
            'groupGUIDs':['main'],'groups':[{'parentConstantIndex':-1,'volumeIndex':0,
                'pitchIndex':1,'mute':False,'solo':False,'bypassEffects':False}],
            'snapshotGUIDs':['active'],'snapshots':[{'values':[0.0,1.0]}],
            'effects':[],'exposedParameterIndices':[],'exposedParameterNames':[]}}
        curve={'m_Curve':[{'time':0.0,'value':1.0,'inSlope':0.0,'outSlope':0.0,
            'inWeight':0.0,'outWeight':0.0,'weightedMode':0}],
            'm_PreInfinity':2,'m_PostInfinity':2,'m_RotationOrder':4}
        def source_data():
            return {'m_GameObject':ptr(go),'m_audioClip':ptr(clip),'OutputAudioMixerGroup':ptr(group),
                'Priority':128,'Loop':True,'m_Volume':1.0,'m_Pitch':1.0,
                'panLevelCustomCurve':curve,'Pan2D':0.0,'MinDistance':1.0,'MaxDistance':100.0,
                'rolloffMode':0,'DopplerLevel':1.0,'m_PlayOnAwake':False,'Mute':False,
                'm_Enabled':True,'rolloffCustomCurve':curve,'spreadCustomCurve':curve,
                'reverbZoneMixCustomCurve':curve}
        prefab=add('level1',102,'AudioSource',source_data())
        records[key(prefab)].update({'Loop':False,'MinDistance':0.5,'MaxDistance':200.0})
        idle=add('level1',201,'AudioSource',source_data())
        motor=add('level1',202,'AudioSource',source_data())
        horn=add('level1',203,'AudioSource',source_data())
        radio_voices=[add('level1',204+i,'AudioSource',source_data()) for i in range(3)]
        engine=add('level1',301,'MonoBehaviour',cls='enginescript')
        records[key(engine)]={'m_GameObject':ptr(go),'SAlap':ptr(idle),'SMotor':ptr(motor),
            'SStart':ptr(clip),'SStop':ptr(clip),
            **{name:value for name,value in [('SMinAlapRpm',600),('SMaxAlapRpm',1400),
                ('SMinMotorRpm',500),('SMaxMotorRpm',1300),('SMaxAlapSound',0.7),
                ('SMaxMotorSound',0.7),('minAlapPitch',0.9),('maxAlapPitch',1.2),
                ('minMotorPitch',0.8),('maxMotorPitch',1.5),('maxRpm',6600),
                ('engineStartTimeModifier',1),('SStartVolume',0.6),('SStopVolume',0.6)]}}
        car=add('level1',302,'MonoBehaviour',{'m_GameObject':ptr(go),'SHornSound':ptr(horn),
            'SSounds':ptr(horn),'turnedRadioSOund':ptr(clip),'HornPitchMin':0.9,
            'HornPitchMax':1.0,'HornSwitch':[]},'carscript')
        radio=add('level1',303,'MonoBehaviour',cls='radioscript')
        records[key(radio)]={'m_GameObject':ptr(go),'SS':ptr(radio_voices[0]),
            'SC':ptr(radio_voices[1]),'SC2':ptr(radio_voices[2]),
            'Bvolume':null,'Btune':null,'BChangeAMFM':null,'Cturned':ptr(clip),
            'AMmin':540,'AMmax':1600,'FMmin':88,'FMmax':108,'AMStep':5,'FMStep':0.1,
            'vSpeed':0.1,'turnVolume':1,'ON':False,'amfm':False,'currentTeker':0.325,'v':0}
        cycle=add('level1',304,'MonoBehaviour',{'tempNights':[-9],'tempDays':[35]})
        add('globalgamemanagers',4,'AudioManager',{'m_RealVoiceCount':128,
            'm_VirtualVoiceCount':512,'m_VirtualizeEffects':True,'Doppler Factor':1,
            'Rolloff Scale':1})
        main=add('level1',64919,'MonoBehaviour',{'Audio':ptr(prefab_go),'defGroup':ptr(group),
            'napszak':ptr(cycle),'FRTFM':0.4,'FRTAM':20,'fluidTuneSpeed':1,
            'minPourSpeedSound':0,'maxPourSpeedSound':1,'radioChs':[],
            'radioChsAM':[{'FR':1080,'name':'custom','custom':1,'music':[],'talk':[],
                'musicHoliday':[],'talkHoliday':[],'musicVolume':[],'talkVolume':[],
                'Bmusic':0,'startTime':0,'cTime':0,'cMaxTime':0,'currentVolume':0}]})
        sensor=add('level1',80121,'MonoBehaviour',{'m_GameObject':ptr(go),'sensed':False,
            'v':0,'radius':0.05,'LM':{'m_Bits':8192}})
        add('level1',80125,'MonoBehaviour',{'defInteriorM':0.5,'interiorLerpM':2,
            'FCarSoundM':1.4,'outsideVolume':0})
        components=[idle,motor,horn,*radio_voices,engine,car,radio]
        def seal_records():
            for (file,path_id),data in records.items():
                name=f'{file}/{path_id}.json'
                path=library.records_root/name
                path.parent.mkdir(parents=True,exist_ok=True)
                content=json.dumps({'schema_version':1,'source':{'file':file,'path_id':path_id},
                    'data':data},allow_nan=False).encode()
                path.write_bytes(content)
                library.record_files[name]={'file':name,'bytes':len(content),
                    'sha256':hashlib.sha256(content).hexdigest()}
        seal_records()
        class Source:
            def __init__(self):
                self.root=root
                self.objects=objects
            def get(self,file,path_id):return objects[file,path_id]
            def read(self,obj):return library.record(ObjectId(*key(obj)))
            def resolve(self,owner,pointer):
                if not pointer['m_PathID']:return None
                return self.get(pointer.get('file',owner.assets_file.source_container_path),pointer['m_PathID'])
            def transform_for(self,game_object):return transform
            def hierarchy(self,selected):return [{'components':components}]
            def components(self,selected):return None,[prefab]
            def class_name(self,obj):return classes[key(obj)]
        manifest={'nodes':[{'id':1,'source':source_id(transform),'parent':None}],
            'items':[],'mechanisms':[],'colliders':[],'part_slots':[],
            'lighting':{'controllers':[]}}
        receipt={'input':{'file':'Assembly-CSharp.dll','sha256':'a'*64},
            'output':{'sha256':'b'*64}}
        def method(cls,name):return {'class':cls,'method':name,'file':'managed/Assembly-CSharp.cs',
            'line_start':1,'line_end':2,'sha256':'b'*64}
        return library,Source(),{'selected':[transform]},manifest,records[key(engine)],seal_records,native,receipt,method

    def test_recover_audio_follows_accepted_pointers_and_overrides(self):
        from tools.reference.audio import recover_audio
        with tempfile.TemporaryDirectory() as directory:
            library,source,roots,manifest,engine,seal_records,native,receipt,method=self.accepted_source_fixture(directory)
            with patch('tools.reference.audio.managed_evidence',return_value=(receipt,method)), \
                    patch('tools.reference.audio.NATIVE_HASH',hashlib.sha256(native.read_bytes()).hexdigest()):
                audio=recover_audio(source,roots,manifest,library,Path(directory))
                idle=audio['voices'][audio['engine']['idle_voice']-1]
                self.assertEqual(idle['source'],{'file':'level1','path_id':201})
                self.assertEqual(audio['emitters'][idle['emitter']-1]['node'],1)
                self.assertEqual(idle['clip'],1)
                self.assertEqual(audio['clips'][0]['source'],{'file':'sharedassets1.assets','path_id':4423})
                self.assertEqual(idle['mixer'],audio['shared']['mixer'])
                self.assertEqual(idle['curves']['rolloff']['pre_wrap_mode'],2)
                self.assertEqual(idle['authored_priority'],128)
                self.assertEqual(idle['priority'],80)
                self.assertEqual(idle['methods'][0]['method'],'Start')
                self.assertEqual(audio['radio']['stations'][0]['frequency'],1080)
                self.assertIn(ObjectId('level1',301),library.selected_records)
                self.assertIn(ObjectId('sharedassets1.assets',4423),library.selected_records)
                engine['SAlap']={'file':'level1','m_PathID':999999}
                seal_records()
                with self.assertRaises(KeyError):
                    recover_audio(source,roots,manifest,library,Path(directory))

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
            for field,value in [('gain',-1),('pitch',0),('min_distance',-1),
                                ('max_distance',0),('spatial_blend',2),('rolloff_mode',3)]:
                broken=copy.deepcopy(audio);broken['shared']['settings'][field]=value
                with self.subTest(shared=field), self.assertRaises(ValueError):validate_audio_export(broken,reader.root)
            broken=copy.deepcopy(audio)
            broken['shared']['settings']['rolloff_curve']['keys']=[]
            with self.assertRaises(ValueError):validate_audio_export(broken,reader.root)
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
