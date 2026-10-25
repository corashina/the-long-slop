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



HIT_SOURCE_TEXT = '''public class mainscript
{
    public static byte[] AudioPriorities = new byte[8] { 10, 25, 40, 55, 70, 80, 85, 100 };
    public int maxAudioCount;
    private void RADIO() { startTime = Time.time; cTime = Time.time - _radio[j].startTime; cTime = Time.unscaledTime - _radio[i].startTime; }
}
public class pickupable
{
    private void OnCollisionEnter(Collision c)
    {
        float num = c.relativeVelocity.magnitude * collisionSoundM * ((!pickedUp) ? 0.3f : 1f);
        if (num > 0.075f) { mainscript.PlayClipAtPoint(collideSound, p, num - 0.075f, mainscript.AudioPriorities[7]); }
    }
    private void FixedUpdate()
    {
        if (Time.time - lastInteriorSensed > 10f && !pickedUp) { Physics.OverlapSphere(transform.position, 0.1f, m.interiorLayer); }
    }
}
public class carscript
{
    private void OnCollisionEnter(Collision c)
    {
        magnitude *= collisionSoundM * 0.3f;
        if (magnitude > 0.7f) { } else if (magnitude > 0.5f) { } else if (magnitude > 0.2f) { }
        else { if (!(magnitude > 0.05f)) { return; } magnitude = 1f; }
    }
}
public class soundplayscript
{
    public void PlaySound()
    {
        if (!(UnityEngine.Random.Range(0f, 1f) > chance)) { mainscript.PlayClipAtPoint(c, p, v, mainscript.AudioPriorities[7]); }
    }
}
public class stepSoundScript
{
    public void Step(float _modifier)
    {
        if (Physics.Raycast(transform.position, -transform.up, out hit, rayDist, mainscript.M.floorLayerMask)) {
        otherStep = hit.collider.GetComponent<otherStepSoundScript>();
        if (hit.collider.gameObject.layer == mainscript.sandLayer || hit.collider.gameObject.layer == mainscript.sandLayer2 || hit.collider.CompareTag("Sand")) { }
        else if (hit.collider.CompareTag("Wood")) { }
        mainscript.PlayClipAtPoint(StepSoundClip, transform.position, StepSoundVolume * _modifier * stepSound, randomPitch: true, mainscript.AudioPriorities[3]); }
    }
}
public class interiorscript
{
    private void Start()
    {
        if (col == null) { col = GetComponent<Collider>(); }
    }
}
public class steps
{
    public static int sandLayer = 17;
    public static int sandLayer2 = 19;
    public enum walkSoundType
    {
        road,
        desert,
        winter,
        grass,
        wood
    }
}
public class fpscontroller
{
\tprivate void FixedUpdate()
\t{
\t\tif (ws <= 0f - walkbobSoundF) { stepScript.Step(duckValue + (input.run ? 0.15f : 0f)); }
\t}

\tprivate void Grounded(bool a)
\t{
\t\tif (grounded) { stepScript.Step(duckValue + (input.run ? 0.15f : 0f)); }
\t}
}
'''

ROCK = {'SSource': {'m_PathID': 0}, 'findNap': 1, 'same': 1, 'clipsReggel': ['a', 'b'],
    'reggeledik': 0, 'nappal': 0, 'esteledik': 1, 'este': 1, 'minLerpReggel': 0., 'maxLerpReggel': 0.,
    'minLerpNappal': 0., 'maxLerpNappal': 0., 'minLerpEsteledik': .5, 'maxLerpEsteledik': 1.,
    'minLerpEste': .5, 'maxLerpEste': 1., 'minTime': 10., 'maxTime': 30., 'chance': .5,
    'minVolume': .5, 'maxVolume': 1.}


class HitAudio(unittest.TestCase):
    def evidence(self, text):
        from tools.reference.audio import ASSEMBLY_HASH, managed_evidence
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); (root / 'managed').mkdir()
            content = text.encode()
            receipt = json.dumps({'complete': True, 'mode': 'whole_module', 'input': {'file': 'a.dll', 'sha256': ASSEMBLY_HASH},
                'output': {'sha256': hashlib.sha256(content).hexdigest()}, 'decompiler': {'version': '11.1.0.9782'}}).encode()
            files = []
            for name, data in [('managed/decompilation.json', receipt), ('managed/Assembly-CSharp.cs', content)]:
                (root / name).write_bytes(data)
                files.append({'file': name, 'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()})
            library = NS(records_root=root, manifest={'managed_analysis': {'files': files}})
            return managed_evidence(library)

    def test_hit_source_statements_are_required(self):
        _, _, footsteps = self.evidence(HIT_SOURCE_TEXT)
        self.assertEqual([(m['class'], m['method']) for m in footsteps],
            [('stepSoundScript', 'Step'), ('fpscontroller', 'FixedUpdate'), ('fpscontroller', 'Grounded')])
        for old, new in [('num - 0.075f', 'num - 0.08f'), ('> 10f', '> 5f'), ('> 0.5f', '> 0.4f'), ('> chance', '>= chance'),
                ('AudioPriorities[3]', 'AudioPriorities[4]'), ('CompareTag("Wood")', 'CompareTag("Plank")'), ('sandLayer2 = 19', 'sandLayer2 = 18'),
                ('winter,\n        grass', 'grass,\n        winter'), ('if (grounded) { stepScript.Step', 'if (grounded) { stepScript.Jump'),
                ('col = GetComponent<Collider>();', 'col = GetComponentInChildren<Collider>();')]:
            with self.assertRaises(ValueError): self.evidence(HIT_SOURCE_TEXT.replace(old, new))

    def test_world_interiors(self):
        from tools.reference.audio import world_interiors
        objects = {}
        def add(path_id, type_name, data=None):
            objects[path_id] = NS(assets_file=NS(source_container_path='f'), path_id=path_id, type=NS(name=type_name), data=data)
            return {'m_FileID': 0, 'm_PathID': path_id}
        empty = {'m_FileID': 0, 'm_PathID': 0}
        door, outside = add(1, 'MonoBehaviour'), add(2, 'MonoBehaviour')
        box, capsule = add(3, 'BoxCollider'), add(4, 'CapsuleCollider')
        go = add(5, 'GameObject')
        def interior(**change):
            return {'col': box, 'm_GameObject': go, 'parent': empty, 'invert': 0, 'minDoorToMax': 1, 'maxAtengedes': 1.,
                'useValue': 0, 'value': 0., 'doors': [door], 'invertindividual2': [], 'parentDoor': empty,
                'removableDoor': 0, 'doorSlots': [], 'parentDoorSlot': empty, **change}
        hall = add(6, 'MonoBehaviour', interior())
        room = add(7, 'MonoBehaviour', interior(col=empty, parent=hall, parentDoor=door, doors=[], invertindividual2=[]))
        source = NS(read=lambda o: o.data, resolve=lambda _, p: objects.get(p['m_PathID']),
            transform_for=lambda o: o, components=lambda _: (None, [objects[1], objects[4], objects[3]]))
        f = lambda i: {'file': 'f', 'path_id': i}
        prototype = {'source': f(9), 'colliders': [{'source': f(3)}, {'source': f(4)}]}
        mechanisms = [{'prototype': f(9), 'recipe': {'source': f(1)}}, {'prototype': f(8), 'recipe': {'source': f(2)}}]
        readers = [objects[6], objects[7]]
        self.assertEqual(world_interiors(source, prototype, readers, mechanisms, 'm'), [
            {'collider': f(3), 'parent': None, 'invert': False, 'minimum_doors_to_max': 1, 'maximum_transmission': 1.,
                'use_value': False, 'value': 0., 'doors': [f(1)], 'invert_doors': [False], 'parent_door': None, 'method': 'm'},
            {'collider': f(4), 'parent': 0, 'invert': False, 'minimum_doors_to_max': 1, 'maximum_transmission': 1.,
                'use_value': False, 'value': 0., 'doors': [], 'invert_doors': [], 'parent_door': f(1), 'method': 'm'}])
        for change in [{'removableDoor': 1}, {'doorSlots': [door]}, {'parentDoorSlot': door}, {'doors': [outside]},
                {'parentDoor': outside}, {'parent': outside}, {'invertindividual2': [1, 0]}, {'col': add(10, 'BoxCollider')}]:
            objects[6].data = interior(**change)
            with self.assertRaises(ValueError): world_interiors(source, prototype, readers, mechanisms, 'm')

    def test_footstep_record_and_errors(self):
        from tools.reference.audio import footstep_record, validate_footsteps
        main = {'walkSounds': [{'volume': v, 'clips': [i]} for i, v in enumerate([.2, .2, .2, .2, 2.])], 'floorLayerMask': {'m_Bits': 164353}}
        step = {'feetSens': [], 'stepSound': 1., 'rayDist': .25}
        record = footstep_record(main, step, [0., -.8343, 0.], lambda pointers: [p + 1 for p in pointers], ['m'])
        self.assertEqual(record, {'surfaces': [{'name': n, 'volume': v, 'clips': [i + 1]} for i, (n, v) in
            enumerate(zip(['road', 'desert', 'winter', 'grass', 'wood'], [.2, .2, .2, .2, 2.]))],
            'step_gain': 1., 'ray_distance': .25, 'ray_mask': 164353, 'local_position': [0., -.8343, 0.], 'sand_layers': [17, 19], 'method': ['m']})
        with self.assertRaises(ValueError): footstep_record(main, {**step, 'feetSens': [{}]}, [0., 0., 0.], lambda p: p, [])
        with self.assertRaises(ValueError): footstep_record({**main, 'walkSounds': main['walkSounds'][:4]}, step, [0., 0., 0.], lambda p: p, [])
        manifest = {'colliders': [{'step_surface': 'sand'}], 'item_prototypes': [{'colliders': [{'step_surface': None}]}],
            'world': {'prototypes': [{'colliders': [{'step_surface': 'wood'}]}]}, 'survival': {'foods': [{'colliders': [{'step_surface': 'wood_tag'}]}]}}
        audio = {'footsteps': record, 'survival': [{'role': 'footstep', 'clips': [1, 2, 3, 4, 5]}]}
        validate_footsteps(audio, set(range(1, 6)), manifest)
        for change in [lambda a, m: a['survival'][0]['clips'].pop(), lambda a, m: a['footsteps']['surfaces'].reverse(),
                lambda a, m: a['footsteps'].update(ray_distance=0.), lambda a, m: m['survival']['foods'][0]['colliders'][0].update(step_surface='mud'), lambda a, m: a['footsteps'].update(method={})]:
            a, m = copy.deepcopy(audio), copy.deepcopy(manifest); change(a, m)
            with self.assertRaises(ValueError): validate_footsteps(a, set(range(1, 6)), m)

    def test_collider_step_surface(self):
        from tools.reference.collision import step_surface
        objects = {}
        def add(path_id, type_name, data, cls=None):
            objects['f', path_id] = NS(assets_file=NS(source_container_path='f'), path_id=path_id, type=NS(name=type_name), data=data, cls=cls)
            return {'m_FileID': 0, 'm_PathID': path_id}
        add(1, 'TagManager', {'tags': ['Sand', 'Wood']})
        other = add(2, 'MonoBehaviour', {'tip': 3}, 'otherStepSoundScript')
        box = add(3, 'BoxCollider', {})
        source = NS(objects=objects, read=lambda o: o.data, class_name=lambda o: o.cls, resolve=lambda _, p: objects['f', p['m_PathID']])
        def surface(tag, parts=(box,)):
            return step_surface(source, NS(data={'m_Tag': tag, 'm_Component': [{'component': p} for p in parts]}))
        self.assertEqual([surface(20001, (box, other)), surface(20001), surface(20000), surface(0)], ['grass', 'wood_tag', 'sand', None])

    def test_car_tiers_follow_source(self):
        from tools.reference.audio import collision_record
        record = collision_record([[1], [2, 3], [4], [5]], ['m'])
        self.assertEqual(record['car_tiers'], [{'threshold': .05, 'clips': [1], 'fixed_gain': 1.},
            {'threshold': .2, 'clips': [2, 3], 'fixed_gain': None}, {'threshold': .5, 'clips': [4], 'fixed_gain': None},
            {'threshold': .7, 'clips': [5], 'fixed_gain': None}])
        self.assertEqual([record[k] for k in ('interior_check_seconds', 'interior_radius', 'loose_multiplier', 'item_minimum', 'car_multiplier')],
            [10., .1, .3, .075, .3])
        with self.assertRaises(ValueError): collision_record([[1], [2], [3]], [])

    def test_rock_ambience_record_and_errors(self):
        from tools.reference.audio import ambience_record
        prototype = {'file': 'sharedassets1.assets', 'path_id': 149742}
        clips = lambda pointers: [{'a': 7, 'b': 8}[p] for p in pointers]
        record = ambience_record(prototype, ROCK, clips, 'm')
        self.assertEqual(record, {'prototype': prototype, 'clips': [7, 8], 'phases': {'dusk': [.5, 1.], 'night': [.5, 1.]},
            'interval': [10., 30.], 'chance': .5, 'gain': [.5, 1.], 'pitch': [1., 1.], 'priority': 100, 'method': 'm'})
        for change in [{'SSource': {'m_PathID': 5}}, {'findNap': 0}, {'same': 0}]:
            with self.assertRaises(ValueError): ambience_record(prototype, {**ROCK, **change}, clips, 'm')

    def test_one_shot_flags(self):
        from tools.reference.audio import ambience_record, collision_record, validate_hit_audio
        def record(role, kind, one_shot, clips, priority=100):
            return {'role': role, 'kind': kind, 'one_shot': one_shot, 'clips': clips,
                'settings': {'priority': priority, 'spatial_blend': 1.}}
        audio = {'collisions': collision_record([[1], [2], [3], [4]], []),
            'ambience': [ambience_record({}, ROCK, lambda _: [5], 'm')],
            'survival': [record('eat', 0, True, [6], 70), record('burp', 1, False, [6], 128),
                record('item_hit', 11, True, [6]), record('car_hit', 12, True, [1, 2, 3, 4]), record('rock_heat', 13, True, [5]), record('eat_crunchy', 10, True, [6], 70), record('music_box', 14, True, [6], 128),
                record('footstep', 15, True, [6], 55)]}
        validate_hit_audio(audio, set(range(1, 7)), [])
        for index, field, value in [(0, 'one_shot', False), (1, 'one_shot', True), (3, 'clips', [1, 2, 3]), (2, 'kind', 14), (7, 'settings', {'priority': 100, 'spatial_blend': 1.})]:
            changed = copy.deepcopy(audio); changed['survival'][index][field] = value
            with self.assertRaises(ValueError): validate_hit_audio(changed, set(range(1, 7)), [])
        changed = copy.deepcopy(audio); del changed['survival'][4]
        with self.assertRaises(ValueError): validate_hit_audio(changed, set(range(1, 7)), [])
        validate_hit_audio(audio, set(range(1, 7)), [{'collision_clip': 6, 'collision_gain': .5}])
        for hit in [{'collision_clip': 5, 'collision_gain': .5}, {'collision_clip': 6, 'collision_gain': float('nan')}]:
            with self.assertRaises(ValueError): validate_hit_audio(audio, set(range(1, 7)), [hit])

    def test_world_pickup_hit_sources(self):
        from tools.reference.audio import pickup_hit_sources
        def record(path_id, clip):
            return {'source': {'file': 'f', 'path_id': path_id}, 'class_name': 'pickupable', 'fields': [
                {'name': 'collideSound', 'value': {'kind': 'link', 'value': clip} if clip else {'kind': 'empty'}},
                {'name': 'collisionSoundM', 'value': {'kind': 'number', 'value': .5}}]}
        records = [record(1, {'file': 'f', 'path_id': 9}), record(2, None), {**record(3, {'file': 'f', 'path_id': 9}), 'class_name': 'massScript'}]
        self.assertEqual(list(pickup_hit_sources(records)), [({'file': 'f', 'path_id': 1}, {'file': 'f', 'path_id': 9}, .5)])
