"""Derive playback rules from accepted records and managed control flow."""
import hashlib
import json
import math
import re
import shutil
from pathlib import Path
from .library.reader import checked
from .library.types import ObjectId
from .source import key, source_id

ASSEMBLY_HASH = '9919a9eeddcce23bb4ff86e890437f32b4a3ef3c10af2efe3b8d889d894dd62e'
NATIVE_HASH = '553c1e7560e1b42bd4638c53d0168dcd1b4b9eb5b9df324b0a44f14b3f18bc86'
PRIORITIES = [10, 25, 40, 55, 70, 80, 85, 100]


def station_records(source, owner, data, clip):
    stations = []
    for band, field in [('fm', 'radioChs'), ('am', 'radioChsAM')]:
        for value in data[field]:
            custom = bool(value['custom'])
            if custom and (value['music'] or value['talk']):
                raise ValueError('Custom station must retain external content')
            station = {'id': len(stations) + 1, 'band': band, 'frequency': value['FR'],
                'name': value['name'], 'custom': custom, 'content': 'external' if custom else 'embedded',
                'initial_music': bool(value['Bmusic']), 'start_time': value['startTime'],
                'initial_time': value['cTime'], 'initial_max_time': value['cMaxTime'],
                'initial_gain': value['currentVolume']}
            for kind in ['music', 'talk']:
                station[kind] = [clip(source.resolve(owner, p)) for p in value[kind]]
                station[kind + '_gains'] = value[kind + 'Volume']
                station[kind + '_holidays'] = value[kind + 'Holiday']
            stations.append(station)
    return stations


def managed_evidence(library):
    files = {r['file']: r for r in library.manifest['managed_analysis']['files']}
    receipt = json.loads(checked(library.records_root, files['managed/decompilation.json']))
    content = checked(library.records_root, files['managed/Assembly-CSharp.cs'])
    if (not receipt['complete'] or receipt['mode'] != 'whole_module'
            or receipt['input']['sha256'] != ASSEMBLY_HASH
            or receipt['output']['sha256'] != hashlib.sha256(content).hexdigest()
            or receipt['decompiler']['version'] != '11.1.0.9782'):
        raise ValueError('Audio requires the accepted complete managed analysis')
    text = content.decode('utf-8-sig')
    def evidence(cls, method):
        declaration = re.search(r'\bpublic class ' + re.escape(cls) + r'\b', text)
        if declaration is None: raise ValueError(f'Missing audio class: {cls}')
        start = declaration.start()
        end = text.find('\npublic class ', start + 1)
        region = text[start:end if end >= 0 else len(text)]
        matches = list(re.finditer(r'\b(?:void|float|bool|GameObject)\s+' + re.escape(method) + r'\(', region))
        if not matches: raise ValueError(f'Missing audio method: {cls}.{method}')
        match = matches[0]
        opening = region.index('{', matches[-1].end())
        depth = 1; closing = opening + 1
        while depth:
            depth += (region[closing] == '{') - (region[closing] == '}')
            closing += 1
        return {'class': cls, 'method': method, 'file': 'managed/Assembly-CSharp.cs',
            'line_start': text[:start + match.start()].count('\n') + 1,
            'line_end': text[:start + closing].count('\n') + 1,
            'sha256': hashlib.sha256(content).hexdigest()}
    if 'new byte[8] { 10, 25, 40, 55, 70, 80, 85, 100 }' not in text:
        raise ValueError('Source audio priority ordering changed')
    # The source has one declaration and no reads of this field.
    if len(re.findall(r'\bmaxAudioCount\b', text)) != 1:
        raise ValueError('Source voice-budget control flow needs a new audit')
    for statement in ('startTime = Time.time;', 'cTime = Time.time - _radio[j].startTime;',
                      'cTime = Time.unscaledTime - _radio[i].startTime;'):
        if statement not in text:
            raise ValueError('Source radio clock needs a new audit')
    return receipt, evidence


def copy_audio(library, identity: ObjectId, clip_id: int, output: Path, usage: str) -> dict:
    path, metadata = library.audio(identity)
    name = f'audio-{identity.file}-{identity.path_id}.wav'
    shutil.copyfile(path, output / name)
    return {'id': clip_id, 'source': {'file':identity.file,'path_id':identity.path_id},
        'path': 'reference/' + name, 'usages':[usage], **metadata}


def recover_audio(source, roots: dict, manifest: dict, library, output: Path) -> dict:
    receipt, method = managed_evidence(library)
    audio = {'schema_version': 1, 'clips': [], 'emitters': [], 'voices': [], 'mixers': [],
        'bindings': [], 'mechanisms': [], 'fluids': [], 'interiors': [], 'routes': [], 'lamps': [], 'silent_actions': ['pickup', 'detach', 'handbrake'],
        'managed': {'file': receipt['input']['file'], 'sha256': receipt['input']['sha256'],
                    'decompiled_sha256': receipt['output']['sha256']},
        'shared': {'priorities': PRIORITIES, 'random_pitch': [0.85, 1.15],
            'declared_max_audio_count': 32, 'enforced_voice_limit': None,
            'gain_above_one': 'clamp_gain_scale_distances_by_gain_minus_one',
            'destroy_after_source_duration': True, 'method': method('mainscript', 'PlayClipAtPoint')}}
    nodes = {(n['source']['file'], n['source']['path_id']): n for n in manifest['nodes']}
    item_by_node = {i['node']: i['id'] for i in manifest['items']}
    def item_for(node):
        while node:
            if node['id'] in item_by_node: return item_by_node[node['id']]
            parent = node['parent']; node = next((n for n in nodes.values() if n['id'] == parent), None)
        return None
    clips = {}; emitters = {}; voices = {}; groups = {}
    def clip(reader, usage='effect'):
        if reader is None: raise ValueError('Required audio clip pointer is null')
        identity = key(reader)
        if identity not in clips:
            clips[identity] = len(audio['clips']) + 1
            audio['clips'].append(copy_audio(library,ObjectId(*identity),clips[identity],output,usage))
        elif usage not in audio['clips'][clips[identity]-1]['usages']:
            audio['clips'][clips[identity]-1]['usages'].append(usage)
        return clips[identity]
    def emitter(reader):
        go = source.resolve(reader, source.read(reader)['m_GameObject'])
        transform = source.transform_for(go); identity = key(transform)
        if identity not in nodes: raise ValueError(f'Audio emitter outside scene nodes: {identity}')
        if identity not in emitters:
            node = nodes[identity]; emitters[identity] = len(audio['emitters']) + 1
            audio['emitters'].append({'id': emitters[identity], 'node': node['id'],
                'item': item_for(node), 'source': source_id(transform)})
        return emitters[identity]
    def mixer(reader):
        if reader is None: raise ValueError('Required audio mixer pointer is null')
        identity = key(reader)
        if identity not in groups:
            groups[identity] = len(groups) + 1
        return groups[identity]
    components = []
    for readers in roots.values():
        for root in readers:
            for node in source.hierarchy(root): components.extend(node['components'])
    main_r = source.get('level1', 64919); main = source.read(main_r)
    prefab_go = source.resolve(main_r, main['Audio'])
    _, prefab_components = source.components(source.transform_for(prefab_go))
    prefab = next(c for c in prefab_components if c.type.name == 'AudioSource')
    manager = source.get('globalgamemanagers', 4)
    manager_data = source.read(manager)
    audio['shared']['audio_manager'] = {'source': source_id(manager), 'real_voices': manager_data['m_RealVoiceCount'],
        'virtual_voices': manager_data['m_VirtualVoiceCount'], 'virtualize_effects': manager_data['m_VirtualizeEffects'],
        'doppler_factor': manager_data['Doppler Factor'], 'rolloff_scale': manager_data['Rolloff Scale']}
    audio['shared']['source'] = source_id(prefab)
    prefab_data = source.read(prefab)
    def curve(value):
        return {'keys': [{'time': k['time'], 'value': k['value'], 'in_tangent': k['inSlope'],
            'out_tangent': k['outSlope'], 'in_weight': k['inWeight'], 'out_weight': k['outWeight'],
            'weighted_mode': k['weightedMode']} for k in value['m_Curve']],
            'pre_wrap_mode': value['m_PreInfinity'], 'post_wrap_mode': value['m_PostInfinity'],
            'rotation_order': value['m_RotationOrder']}
    def voice(reader, evidence=None):
        identity = key(reader)
        if identity in voices:
            for value in evidence or []:
                if value not in audio['voices'][voices[identity]-1]['methods']:
                    audio['voices'][voices[identity]-1]['methods'].append(value)
            return voices[identity]
        d = source.read(reader); cid = source.resolve(reader, d['m_audioClip'])
        v = {'id': len(audio['voices']) + 1, 'source': source_id(reader), 'emitter': emitter(reader),
            'clip': clip(cid) if cid else None, 'authored_clip':clip(cid) if cid else None,
            'authored_priority':d['Priority'],'authored_loop':bool(d['Loop']), 'mixer': mixer(source.resolve(reader, d['OutputAudioMixerGroup'])),
            'priority': d['Priority'], 'loop': bool(d['Loop']), 'gain': d['m_Volume'], 'pitch': d['m_Pitch'],
            'spatial_blend': d['panLevelCustomCurve']['m_Curve'][0]['value'], 'pan': d['Pan2D'],
            'min_distance': d['MinDistance'], 'max_distance': d['MaxDistance'], 'rolloff_mode': d['rolloffMode'],
            'doppler': d['DopplerLevel'], 'play_on_awake': bool(d['m_PlayOnAwake']),
            'mute': bool(d['Mute']), 'enabled': bool(d['m_Enabled']),
            'curves': {name: curve(d[field]) for name, field in [('rolloff', 'rolloffCustomCurve'),
                ('spatial_blend', 'panLevelCustomCurve'), ('spread', 'spreadCustomCurve'),
                ('reverb_mix', 'reverbZoneMixCustomCurve')]},
            'methods': evidence or []}
        voices[identity] = v['id']; audio['voices'].append(v); return v['id']
    for c in components:
        if c.type.name == 'AudioSource': voice(c)
    audio['shared']['settings'] = {'priority':prefab_data['Priority'],'loop':bool(prefab_data['Loop']),
        'gain':prefab_data['m_Volume'],'pitch':prefab_data['m_Pitch'],'min_distance':prefab_data['MinDistance'],
        'max_distance':prefab_data['MaxDistance'],'rolloff_mode':prefab_data['rolloffMode'],
        'spatial_blend':prefab_data['panLevelCustomCurve']['m_Curve'][0]['value'],
        'spatial_blend_curve':curve(prefab_data['panLevelCustomCurve']),
        'rolloff_curve':curve(prefab_data['rolloffCustomCurve'])}
    audio['shared']['mixer'] = mixer(source.resolve(main_r, main['defGroup']))
    def binding(owner, trigger, pointers, gain, pitch, priority, evidence, route=None, voice_id=None):
        choices = [clip(source.resolve(owner, p)) for p in pointers if p['m_PathID']]
        if not choices: return None
        record = {'id': len(audio['bindings']) + 1, 'source': source_id(owner), 'trigger': trigger,
            'emitter': emitter(owner), 'clips': choices, 'gain': gain, 'pitch_range': pitch,
            'priority': priority, 'mixer': route or audio['shared']['mixer'], 'voice': voice_id, 'method': evidence}
        audio['bindings'].append(record); return record['id']
    mono = [(c, source.class_name(c), source.read(c)) for c in components if c.type.name == 'MonoBehaviour']
    for c, cls, d in mono:
        if cls == 'enginescript':
            idle = voice(source.resolve(c, d['SAlap']), [method(cls, 'Start'), method(cls, 'Update')])
            motor = voice(source.resolve(c, d['SMotor']), [method(cls, 'Start'), method(cls, 'Update')])
            for vid in [idle, motor]: audio['voices'][vid - 1]['priority'] = PRIORITIES[5]
            starter = binding(c, 'starter', [d['SStart']], d['SStartVolume'], [1, 1], PRIORITIES[2], method(cls, 'Update'))
            stop = binding(c, 'engine_stop', [d['SStop']], d['SStopVolume'], [1, 1], PRIORITIES[2], method(cls, 'Update'))
            audio['engine'] = {'source': source_id(c), 'emitter': emitter(c), 'idle_voice': idle,
                'motor_voice': motor, 'starter_binding': starter, 'stop_binding': stop,
                'constants': {k: d[k] for k in ['SMinAlapRpm','SMaxAlapRpm','SMinMotorRpm','SMaxMotorRpm',
                    'SMaxAlapSound','SMaxMotorSound','minAlapPitch','maxAlapPitch','minMotorPitch','maxMotorPitch',
                    'maxRpm','engineStartTimeModifier','SStartVolume','SStopVolume']}, 'method': method(cls, 'Update')}
        elif cls == 'carscript':
            horn = voice(source.resolve(c, d['SHornSound']), [method(cls, 'Update')])
            cycle_r = source.resolve(main_r, main['napszak']); cycle = source.read(cycle_r)
            audio['horn'] = {'source': source_id(c), 'voice': horn, 'attack_release_rate': 10,
                'pitch_min': d['HornPitchMin'], 'pitch_max': d['HornPitchMax'],
                'temperature_min': cycle['tempNights'][0], 'temperature_max': cycle['tempDays'][0],
                'switches': [source_id(source.resolve(c, p)) for p in d['HornSwitch']], 'method': method(cls, 'Update')}
            vr = source.resolve(c, d['SSounds']); vid = voice(vr)
            for trigger, name in [('ignition','Update'), ('light_switch','InteractSwitchLights')]:
                binding(c, trigger, [d['turnedRadioSOund']], 1, [1,1], source.read(vr)['Priority'],
                    method(cls, name), mixer(source.resolve(vr, source.read(vr)['OutputAudioMixerGroup'])), vid)
        elif cls == 'usablescript':
            movement = None
            if d['moveSound'] and d['SSource']['m_PathID'] and d['SMove']['m_PathID']:
                movement = voice(source.resolve(c, d['SSource']), [method(cls, 'RotMoveSound'), method(cls, 'SlideMoveSound')])
                v = audio['voices'][movement-1]; v['clip'] = clip(source.resolve(c,d['SMove']));v['loop'] = False
                v['pitch'] = 1
            entry = {'source': source_id(c), 'emitter': emitter(c), 'movement_voice': movement,
                'audio_multiplier': d['audioM'], 'slide_multiplier': d['audioMSlide'] or 1,
                'rotation_gain': 0.1, 'slide_gain': 2, 'gain_smoothing': 3, 'slide_stop_seconds': 0.2,
                'squeak_chance': d['dontSqueakChanceOverride'] if d['useDontSqueakChanceOverride'] else 0.75,
                'pitch_range': [0.95,1.05], 'endpoints': []}
            for field in ['startPosEnd','endPosEnd','startPosStart','endPosStart']:
                bid = binding(c, field, [d[field]], d['audioM'], [0.85,1.15], PRIORITIES[4], method(cls,'StartEndStuff'))
                if bid: entry['endpoints'].append(bid)
            entry['turn_binding'] = binding(c,'turn', d['turnClips'],d['turnVolume'],[0.85,1.15],PRIORITIES[4],method(cls,'Turn'))
            if movement or entry['endpoints'] or entry['turn_binding']:audio['mechanisms'].append(entry)
        elif cls == 'pickupable' and d['collideSound']['m_PathID']:
            binding(c,'crafted', [d['collideSound']],0.25,[0.85,1.15],PRIORITIES[4],method(cls,'Crafted'))
        elif cls == 'tankcapscript':
            fill_voice = voice(source.resolve(c,d['SSource']), [method(cls,'PlayFillSound'),method(cls,'Update')]) if d['SSource']['m_PathID'] else None
            audio['fluids'].append({'source':source_id(c),'emitter':emitter(c),'fill_voice':fill_voice,
                'max_gain':d['maxVolume'],'tune_speed':d['tuneSpeed'],'fluid_tune_speed':main['fluidTuneSpeed'],
                'pour_speed_min':main['minPourSpeedSound'],'pour_speed_max':main['maxPourSpeedSound'],
                'fill_loop':False,'fill_latch_steps':3, 'open_binding':binding(c,'cap_open',[d['openSound']],d['openVolume'],[1,1],PRIORITIES[4],method(cls,'USE')),
                'close_binding':binding(c,'cap_close',[d['closeSound']],d['closeVolume'],[1,1],PRIORITIES[4],method(cls,'USE')),
                'tune_binding':binding(c,'cap_tune',[d['tekerSound']],1,[1,1],PRIORITIES[4],method(cls,'TUNE')),
                'method':method(cls,'Update')})
        elif cls == 'flashlightscript':
            control=next((v['id'] for v in manifest['lighting']['controllers'] if v['source']==source_id(c)),None)
            lamp_voices=[]
            for lamp in d['Lamps']:
                for pointer in lamp['Sources']:
                    lamp_voices.append(voice(source.resolve(c,pointer),[method(cls,'Refresh')]))
            on=binding(c,'lamp_on',[d['sound']],d['volume'],[0.85,1.15],PRIORITIES[4],method(cls,'TurnOn'))
            off=binding(c,'lamp_off',[d['sound']],d['volume'],[0.85,1.15],PRIORITIES[4],method(cls,'TurnOff'))
            if lamp_voices or on or off:
                if control is None:raise ValueError('Lamp audio lacks selected light controller')
                audio['lamps'].append({'source':source_id(c),'control':control,'voices':lamp_voices,
                    'on_binding':on,'off_binding':off,'method':method(cls,'Refresh')})
        elif cls == 'radioscript':
            rvoices = [voice(source.resolve(c,d[f]),[method(cls,'Start'),method(cls,'SetRadio')]) for f in ['SS','SC','SC2']]
            for vid in rvoices[:2]:audio['voices'][vid-1]['priority']=PRIORITIES[5]
            def control(field):
                if not d[field]['m_PathID']: return None
                reader = source.resolve(c,d[field]); eid = emitter(reader)
                node = audio['emitters'][eid-1]['node']
                values = source.read(reader)
                item = next(i['id'] for i in manifest['items'] if i['node'] == node)
                return {'source':source_id(reader),'node':node,'item':item,
                    'enabled':bool(values['m_Enabled']) and not bool(values['disable']),
                    'tune':bool(values['tuneAble']),'turnable':bool(values['turnable'])}
            controls = {name:control(field) for name,field in [('volume','Bvolume'),('tuning','Btune'),('band','BChangeAMFM')]}
            electronics_reader = source.resolve(c,d['electronics'])
            electronics = source.read(electronics_reader)
            if electronics['jamValue'] != 0:
                raise ValueError('Radio nonzero startup jam requires source rule')
            audio['radio'] = {'source':source_id(c),'emitter':emitter(c),'voices':rvoices,'controls':controls,
                'electronics_source':source_id(electronics_reader),'electronics_output':electronics['maxOutput'],
                'tuning_repeat_seconds':0.2,'holiday_ranges':[[10,15,11,15],[12,15,1,15]],
                'constants':{k:d[k] for k in ['AMmin','AMmax','FMmin','FMmax','AMStep','FMStep','vSpeed','turnVolume']},
                'initial_on':bool(d['ON']),'initial_am':bool(d['amfm']),'initial_tuning':d['currentTeker'],'initial_volume':d['v'],
                'fm_tolerance':main['FRTFM'],'am_tolerance':main['FRTAM'],'electronics_cutoff':0.2,
                'time_correction_seconds':0.5,'late_seek_margin_seconds':0.1,'custom_gain':0.5,
                'clock':{'kind':'scaled','selected':method('mainscript','RADIO'),
                    'music_start':method('radioCH','SelectRandomMusic'),
                    'talk_start':method('radioCH','SelectRandomTalk'),
                    'multiplayer':method('mainscript','RADIO2')},
                'click_binding':binding(c,'radio_power_band',[d['Cturned']],d['turnVolume'],[0.85,1.15],PRIORITIES[4],method(cls,'TurnRadio')),
                'stations':station_records(source,main_r,main,lambda reader:clip(reader,'radio')),'methods':[method(cls,'SetRadio'),method(cls,'Update'),method('mainscript','Update'),
                    method('radioCH','SelectRandomMusic'),method('radioCH','SelectRandomTalk'),method('radioCH','AdvanceRadio'),
                    method('fpscontroller','Update'),method('electronicsscript','Start'),method('electronicsscript','UpdOutPut'),method('mainscript','CheckHoliday')]}
    main=source.get('level1',64919);md=source.read(main)
    player=source.get('level1',80125);pd=source.read(player)
    prefab=next(c for c in source.components(source.transform_for(source.resolve(main,md['Audio'])))[1] if c.type.name=='AudioSource')
    sounds=source.get('level1',70770)
    if source.class_name(sounds)!='soundMainScript':raise ValueError('Toilet source differs')
    survival_groups=[('eat',0,main,md['soundEat'],prefab,[md['eatVolume']]*2,[1.,1.])]
    for role,kind,field,group in zip(('burp','fart','shit'),(1,2,9),('SBurp','SFart','SFart'),source.read(sounds)['clips'],strict=True):
        prefix='Burp' if role=='burp' else 'Fart'
        survival_groups.append((role,kind,sounds,group['clips'],source.resolve(player,pd[field]),[pd['min'+prefix+'Volume'],pd['max'+prefix+'Volume']],[pd['min'+prefix+'Pitch'],pd['max'+prefix+'Pitch']]))
    survival_groups.append(('death',5,main,[md['SdeathBell']],source.resolve(player,pd['SDeathBellSource']),[1.,1.],[.8,1.2]))
    storm=source.get('sharedassets1.assets',158276)
    cycle=source.resolve(main,md['napszak']);cd=source.read(cycle)
    for role,kind,owner,pointer in [('sleep',4,player,pd['SleepCamSource']),('storm',6,storm,source.read(storm)['source']),('storm_inside',10,cycle,cd['sandStormSource']),('ambient_day',7,cycle,cd['DayAmbientSound'][0]),('ambient_night',8,cycle,cd['NightAmbientSound'][0])]:
        selected_source=source.resolve(owner,pointer);ad=source.read(selected_source)
        survival_groups.append((role,kind,selected_source,[ad['m_audioClip']],selected_source,[ad['m_Volume']]*2,[ad['m_Pitch']]*2))

    from .collision import world_pose
    audio['survival']=[]
    for role,kind,owner,pointers,reader,gain,pitch in survival_groups:
        d=source.read(reader)
        route=source.resolve(player,pd['deathbellGroup']) if role=='death' else source.resolve(reader,d['OutputAudioMixerGroup'])
        transform=source.transform_for(source.resolve(reader,d['m_GameObject']))
        if any(c.type.name=='Rigidbody' for c in source.components(transform)[1]):raise ValueError('Selected audio velocity needs a Rigidbody route')
        pose=world_pose(source,transform)
        player_pose=world_pose(source,source.transform_for(source.resolve(player,pd['m_GameObject'])))
        offset=[pose['translation'][i]-player_pose['translation'][i] for i in range(3)] if role in ('burp','fart','shit','death','sleep') else [0.,0.,0.]
        settings={'priority':70 if role=='eat' else d['Priority'],'loop':bool(d['Loop']) if role!='death' else False,
            'gain':d['m_Volume'],'pitch':d['m_Pitch'],'min_distance':d['MinDistance'],'max_distance':d['MaxDistance'],
            'rolloff_mode':d['rolloffMode'],'spatial_blend':1. if role=='eat' else d['panLevelCustomCurve']['m_Curve'][0]['value'],
            'spatial_blend_curve':curve(d['panLevelCustomCurve']),'rolloff_curve':curve(d['rolloffCustomCurve'])}
        if role=='eat':
            for k in settings['spatial_blend_curve']['keys']:k['value']=1.
        cls,name={'eat':('mainscript','PlayClipAtPoint'),'burp':('fpscontroller','Burp'),'fart':('fpscontroller','Fart'),'shit':('fpscontroller','Shit'),'death':('fpscontroller','Death2'),'sleep':('mainscript','Update'),'storm':('sandstormscript','Update')}.get(role,('napszakvaltakozas','FixedUpdate'))
        audio['survival'].append({'role':role,'kind':kind,'source':source_id(owner),'audio_source':source_id(reader),
            'clips':[clip(source.resolve(owner,p)) for p in pointers], 'gain':gain,'pitch':pitch,'offset':offset,
            'settings':settings,'mixer':mixer(route),'doppler':d['DopplerLevel'],'pan':d['Pan2D'],'method':method(cls,name)})
    def control_reader(reader):
        if reader is None: return None
        eid = emitter(reader); node = audio['emitters'][eid-1]['node']
        mid = next((m['id'] for m in manifest['mechanisms'] if m['source']==source_id(reader)),None)
        if mid is None: raise ValueError(f'Interior door lacks mechanism: {key(reader)}')
        return {'source':source_id(reader),'node':node,'mechanism':mid}
    def sensor(reader, player=False):
        d=source.read(reader);go=source.resolve(reader,d['m_GameObject']);tr=source.transform_for(go)
        sd={'source':source_id(reader),'transform_source':source_id(tr),'node':nodes[key(tr)]['id'] if key(tr) in nodes else None,
            'player':player,'initial_sensed':bool(d['sensed']),'initial_count':d['v'],
            'radius':d.get('radius'),'mask':d.get('LM',{}).get('m_Bits'),
            'method':method('sens2' if 'radius' in d else 'sens','FixedUpdate')}
        if player:
            # This is the source sensor's local pose, not another scene hierarchy.
            td=source.read(tr);sd['local_position']=[td['m_LocalPosition'][axis] for axis in ['x','y','z']]
            sd['parent_source']=source_id(source.resolve(tr,td['m_Father']))
        else:sd['local_position']=None;sd['parent_source']=None
        return sd
    interior_readers=[c for c,cls,d in mono if cls=='interiorscript']
    interior_ids={key(r):i+1 for i,r in enumerate(interior_readers)}
    for r in interior_readers:
        d=source.read(r);eid=emitter(r);node=audio['emitters'][eid-1]['node']
        collider=source.resolve(r,d['col'])
        if collider is None:
            go=source.resolve(r,d['m_GameObject']);_,parts=source.components(source.transform_for(go))
            collider=next((c for c in parts if c.type.name in ['BoxCollider','SphereCollider','CapsuleCollider','MeshCollider']),None)
        collider_id=next((c['id'] for c in manifest['colliders'] if c['source']==source_id(collider)),None) if collider else None
        if collider_id is None:raise ValueError(f'Interior sensor collider outside selected gameplay: {key(r)}')
        parent=source.resolve(r,d['parent'])
        if parent and key(parent) not in interior_ids:raise ValueError('Interior parent outside selected roots')
        doors=[control_reader(source.resolve(r,p)) for p in d['doors']]
        slots=[]
        for p in d['doorSlots']:
            sr=source.resolve(r,p);slot=next((s for s in manifest['part_slots'] if s['source']==source_id(sr)),None)
            if slot is None:raise ValueError(f'Interior door slot outside gameplay: {key(sr)}')
            slots.append(slot['id'])
        parent_slot=source.resolve(r,d['parentDoorSlot'])
        parent_slot_id=next((s['id'] for s in manifest['part_slots'] if s['source']==source_id(parent_slot)),None) if parent_slot else None
        audio['interiors'].append({'id':interior_ids[key(r)],'source':source_id(r),'node':node,'collider':collider_id,
            'parent':interior_ids[key(parent)] if parent else None,'invert':bool(d['invert']),
            'minimum_doors_to_max':d['minDoorToMax'],'maximum_transmission':d['maxAtengedes'],
            'removable_doors':bool(d['removableDoor']),'use_value':bool(d['useValue']),'value':d['value'],
            'doors':doors,'door_slots':slots,'invert_doors':[bool(x) for x in d['invertindividual2']] or [False]*len(doors),
            'invert_slots':[bool(x) for x in d['invertindividual']] or [False]*len(slots),
            'parent_door':control_reader(source.resolve(r,d['parentDoor'])), 'parent_slot':parent_slot_id,
            'method':method('interiorscript','GetValue')})
    for r,cls,d in mono:
        if cls=='objSlideOnColliderScript' and d['hasSound']:
            voice(source.resolve(r,d['SSource']),[method(cls,'Update'),method(cls,'Disable')])
        elif cls=='sprayscript':
            voice(source.resolve(r,d['Saudio']),[method(cls,'Update'),method(cls,'Fire')])
    for r,cls,d in mono:
        if cls!='audioBasedOnInteriorsensscript':continue
        route_voices=[voice(source.resolve(r,p),[method(cls,'Update'),method(cls,'SetSources')]) for p in d['Sources']]
        default_interiors=[]
        for p in d['defInterior']:
            target=source.resolve(r,p)
            if key(target) not in interior_ids:raise ValueError('Audio route interior outside selected roots')
            default_interiors.append(interior_ids[key(target)])
        audio['routes'].append({'source':source_id(r),'voices':route_voices,'interiors':default_interiors,
            'has_car':bool(d['def']['m_PathID']),'sensor':sensor(source.resolve(r,d['interiorSens'])),
            'main_mixer':mixer(source.resolve(r,d['GMain'])),'outside_mixer':mixer(source.resolve(r,d['GOutSide'])),
            'car_mixer':mixer(source.resolve(r,d['GCarSounds'])) if d['GCarSounds']['m_PathID'] else None,
            'initial_outside':bool(d['outSide']),'initial_player_inside':bool(d['playerInside']),
            'methods':[method(cls,'Update'),method(cls,'SetSources')]})

    # Preserve the compiled group graph, snapshot parameters, and native effect identity.
    mixer_sources = {key(source.resolve(source.get(*identity),source.read(source.get(*identity))['m_AudioMixer'])) for identity in groups}
    if len(mixer_sources)!=1:raise ValueError('Selected voices must resolve one source mixer')
    mr=source.get(*next(iter(mixer_sources)));md=source.read(mr);constant=md['m_MixerConstant']
    for obj in source.objects.values():
        if obj.type.name=='AudioMixerGroupController' and key(source.resolve(obj,source.read(obj)['m_AudioMixer']))==key(mr):mixer(obj)
    snapshot=source.resolve(mr,md['m_StartSnapshot']);sd=source.read(snapshot)
    si=constant['snapshotGUIDs'].index(sd['m_SnapshotID']);values=constant['snapshots'][si]['values']
    native=library.snapshot.root/'UnityPlayer.dll'
    with native.open('rb') as stream:
        if hashlib.file_digest(stream,'sha256').hexdigest()!=NATIVE_HASH:raise ValueError('Native audio engine hash changed')
    for identity,gid in groups.items():
        r=source.get(*identity);d=source.read(r);i=constant['groupGUIDs'].index(d['m_GroupID']);g=constant['groups'][i]
        parent=g['parentConstantIndex'];parent_id=None
        if parent>=0:
            parent_id=next(groups[k] for k in groups if source.read(source.get(*k))['m_GroupID']==constant['groupGUIDs'][parent])
        effects=[]
        for fx in constant['effects']:
            if fx['groupConstantIndex']!=i:continue
            if fx['type'] not in [-2,10]:raise ValueError(f"Unsupported required mixer effect: {fx['type']}")
            params=[values[j] for j in fx['parameterIndices']]
            effects.append({'type':fx['type'],'name':'attenuation' if fx['type']==-2 else 'parameq',
                'bypass':fx['bypass'],'parameter_indices':fx['parameterIndices'],'parameters':params,
                'previous_effect':fx['prevEffectIndex'],'send_target':fx['sendTargetEffectIndex'],'wet_mix_index':fx['wetMixLevelIndex']})
        audio['mixers'].append({'id':gid,'source':source_id(r),'controller':source_id(mr),'snapshot':source_id(snapshot),
            'name':d['m_Name'],'parent':parent_id,'gain_db':values[g['volumeIndex']],'pitch':values[g['pitchIndex']],
            'mute':g['mute'],'solo':g['solo'],'bypass_effects':g['bypassEffects'],'effects':effects})
    audio['native']={'file':'UnityPlayer.dll','sha256':NATIVE_HASH,'effect_type':10,
        'registration_va':'0x1814436f0','type_write_va':'0x181443798','coefficient_va':'0x181444ff0',
        'parameter_names':['Center freq','Octave range','Frequency gain'],
        'browser_q_rule':'1 / bandwidth','browser_gain_db_rule':'40 * log10(gain)'}
    audio['mixer_controls']={'source':source_id(mr),'parameter_indices':constant['exposedParameterIndices'],
        'parameter_name_hashes':constant['exposedParameterNames'],
        'parameter_names':['CarSounds','PlayerEar','MasterVolume','Outside','InGame'],
        'methods':[method('mainscript','Update'),method('mainscript','SetVolume'),method('fpscontroller','SetOutSideAudio')]}
    player = source.read(source.get('level1',80125))
    audio['mixer_controls']['outside'] = {'default_interior_gain': player['defInteriorM'],
        'gain_smoothing':player['interiorLerpM'], 'car_sound_multiplier':player['FCarSoundM'],
        'initial_gain':player['outsideVolume'], 'sensor':sensor(source.get('level1',80121),True), 'methods':[method('fpscontroller','SetOutSideAudio')]}
    for field in ['engine','horn','radio']:
        if field not in audio:raise ValueError(f'Missing required selected audio: {field}')
    for voice_record in audio['voices']:
        if not voice_record['methods']:raise ValueError(f"Audio voice lacks source control flow: {voice_record['source']}")
    validate_audio_export(audio,output)
    return audio


def validate_audio_export(audio: dict, output: Path) -> None:
    clock = audio['radio']['clock']
    if (clock['kind'] != 'scaled'
            or [(clock[name]['class'], clock[name]['method']) for name in
                ('selected', 'music_start', 'talk_start', 'multiplayer')]
            != [('mainscript', 'RADIO'), ('radioCH', 'SelectRandomMusic'),
                ('radioCH', 'SelectRandomTalk'), ('mainscript', 'RADIO2')]):
        raise ValueError('Source radio clock differs from selected single-player clock')
    ids=set()
    for c in audio['clips']:
        if c['id'] in ids:raise ValueError('Duplicate audio clip')
        ids.add(c['id']);name=c['path'].removeprefix('reference/')
        checked(output,{'file':name,'bytes':c['bytes'],'sha256':c['sha256']})
        from .library.audio import validate_wav
        actual=validate_wav(output/name,{'m_Channels':c['channels'],'m_Frequency':c['frequency'],
            'm_BitsPerSample':c['bits_per_sample'],'m_Length':c['source_duration_seconds']})
        if any(c[k]!=v for k,v in actual.items()):raise ValueError('Runtime WAV metadata mismatch')
    emitters = {v['id'] for v in audio['emitters']}
    mixers = {v['id'] for v in audio['mixers']}
    for v in audio['voices']:
        if v['clip'] is not None and v['clip'] not in ids:raise ValueError('Unresolved voice clip')
        if v['authored_clip'] is not None and v['authored_clip'] not in ids:raise ValueError('Unresolved authored voice clip')
        if v['emitter'] not in emitters or v['mixer'] not in mixers:raise ValueError('Unresolved voice owner or mixer')
        if not v['methods'] or any(not m['sha256'] or m['line_start']<=0 for m in v['methods']):raise ValueError('Missing voice method evidence')
        if set(v['curves'])!={'rolloff','spatial_blend','spread','reverb_mix'}:raise ValueError('Missing voice curves')
        if not math.isfinite(v['min_distance']) or not math.isfinite(v['max_distance']) or not 0<v['min_distance']<=v['max_distance']:
            raise ValueError('Invalid audio distance')
    for b in audio['bindings']:
        if not b['clips'] or any(c not in ids for c in b['clips']):raise ValueError('Unresolved event clip')
        if b['emitter'] not in emitters or b['mixer'] not in mixers:raise ValueError('Unresolved event owner or mixer')
    settings=audio['shared']['settings']
    if (not all(math.isfinite(settings[k]) for k in ('gain','pitch','min_distance','max_distance','spatial_blend'))
            or settings['gain']<0 or settings['pitch']<=0
            or not 0<settings['min_distance']<=settings['max_distance']
            or not 0<=settings['spatial_blend']<=1
            or not 0<=settings['rolloff_mode']<=2):
        raise ValueError('Invalid shared audio source settings')
    for name in ('rolloff_curve','spatial_blend_curve'):
        keys=settings[name]['keys']
        if (not keys or any(not all(math.isfinite(key[field]) for field in
                ('time','value','in_tangent','out_tangent','in_weight','out_weight'))
                or not 0<=key['weighted_mode']<=3 for key in keys)
                or any(a['time']>=b['time'] for a,b in zip(keys,keys[1:]))):
            raise ValueError(f'Invalid shared audio curve: {name}')
