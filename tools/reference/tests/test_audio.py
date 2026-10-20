"""Critical selected-audio contract checks."""
import copy
import ast
import hashlib
import inspect
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
    def test_audio_requires_gltf_reports(self):
        from tools.reference.audio import recover_audio
        signature = inspect.signature(recover_audio)
        self.assertEqual(list(signature.parameters), ['source', 'roots', 'manifest', 'library', 'output', 'reports'])
        self.assertIs(signature.parameters['reports'].default, inspect.Parameter.empty)

    def test_audio_exporter_passes_gltf_reports(self):
        from tools.reference.audio import recover_audio
        from tools.reference.export import _export_selected
        tree = ast.parse(inspect.getsource(_export_selected))
        call = next(node for node in ast.walk(tree) if isinstance(node, ast.Call)
                    and isinstance(node.func, ast.Name) and node.func.id == 'recover_audio')
        reports = {'lada': {'material_indices': {'level1:21': 3}}}
        def bind(*args, **kwargs):
            arguments = inspect.signature(recover_audio).bind(*args, **kwargs).arguments
            self.assertIs(arguments['reports'], reports)
            return arguments
        scope = dict(source=object(), roots={}, CARS={}, manifest={}, library=object(),
                     output=Path('.'), report=reports, recover_audio=bind)
        eval(compile(ast.Expression(call), '<audio export caller>', 'eval'), scope)

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
        electronics=add('level1',305,'MonoBehaviour',{'maxOutput':1,'jamValue':0},'electronicsscript')
        volume=add('level1',306,'MonoBehaviour',{'m_GameObject':ptr(go),'m_Enabled':1,'disable':0,'tuneAble':1,'turnable':0},'usablescript')
        radio=add('level1',303,'MonoBehaviour',cls='radioscript')
        records[key(radio)]={'m_GameObject':ptr(go),'electronics':ptr(electronics),'SS':ptr(radio_voices[0]),
            'SC':ptr(radio_voices[1]),'SC2':ptr(radio_voices[2]),
            'Bvolume':ptr(volume),'Btune':null,'BChangeAMFM':null,'Cturned':ptr(clip),
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
            'items':[{'id':1,'node':1}],'mechanisms':[],'colliders':[],'part_slots':[],
            'lighting':{'controllers':[]}}
        receipt={'input':{'file':'Assembly-CSharp.dll','sha256':'a'*64},
            'output':{'sha256':'b'*64}}
        def method(cls,name):return {'class':cls,'method':name,'file':'managed/Assembly-CSharp.cs',
            'line_start':1,'line_end':2,'sha256':'b'*64}
        return library,Source(),{'selected':[transform]},manifest,records[key(engine)],seal_records,native,receipt,method

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

    def test_required_links_and_samples_fail(self):
        with tempfile.TemporaryDirectory() as directory:
            reader, identity, path = self.fixture(directory)
            self.assertTrue(hasattr(reader, 'audio'), 'missing selected WAV interface')
            path.write_bytes(path.read_bytes()[:-2] + b'xx')
            with self.assertRaises(ValueError): reader.audio(identity)
