"""Selected shovel controller and checked normalized source tracks."""
import json
import math
from .source import source_id
from .library.reader import checked
from .survival import finite_record

CLIPS = (3998, 4001, 3999, 3997, 4002, 4003)
STATES = (2008834259, 2530234843, 3863879828, 1326377276, 1368685456, 3895914711)
PARAMETERS = ('AttackLeft', 'AttackRight', 'AttackForward', 'AttackBack', 'AttackStab', 'Speed', 'Hit')


def recover_motion(source, manifest):
    reader = source.get('sharedassets1.assets', 4515)
    controller = source.read(reader)
    slots = [source_id(source.resolve(reader, p)) for p in controller['m_AnimationClips']]
    if [s['path_id'] for s in slots] != list(CLIPS):
        raise ValueError('Shovel clip slots differ')
    node = next(n['id'] for n in manifest['nodes'] if n['source'] == {'file':'level1', 'path_id':23118})
    handler = source.get('level1', 74298)
    hd = source.read(handler)
    animator = source.resolve(handler, hd['anim'])
    if source_id(animator) != {'file':'level1', 'path_id':56797}:
        raise ValueError('Shovel Animator owner differs')
    names = dict(controller['m_TOS'])
    data = controller['m_Controller']
    if len(data['m_StateMachineArray']) != 1 or len(data['m_LayerArray']) != 1:
        raise ValueError('Shovel controller layer count differs')
    machine = data['m_StateMachineArray'][0]['data']
    parameters = []
    for p in data['m_Values']['data']['m_ValueArray']:
        kind = p['m_Type']
        if kind not in (1,9):raise ValueError('Shovel parameter type differs')
        defaults = data['m_DefaultValues']['data']['m_FloatValues' if kind == 1 else 'm_BoolValues']
        parameters.append({'id':p['m_ID'], 'name':names[p['m_ID']], 'kind':'float' if kind==1 else 'trigger',
                           'index':p['m_Index'], 'default_float':defaults[p['m_Index']] if kind==1 else None,
                           'default_bool':defaults[p['m_Index']] if kind==9 else None})
    def transition(d):
        return {'id':d['m_ID'], 'full_path_id':d['m_FullPathID'], 'user_id':d['m_UserID'],
            'destination':d['m_DestinationState'], 'duration':d['m_TransitionDuration'], 'offset':d['m_TransitionOffset'],
            'exit_time':d['m_ExitTime'], 'has_exit_time':d['m_HasExitTime'], 'fixed_duration':d['m_HasFixedDuration'],
            'can_transition_to_self':d['m_CanTransitionToSelf'], 'interruption_source':d['m_InterruptionSource'],
            'ordered_interruption':d['m_OrderedInterruption'], 'conditions':[
                {'mode':c['data']['m_ConditionMode'], 'parameter':c['data']['m_EventID'],
                 'threshold':c['data']['m_EventThreshold'], 'exit_time':c['data']['m_ExitTime']}
                for c in d['m_ConditionConstantArray']]}
    behaviors = [source.resolve(reader, p) for p in controller['m_StateMachineBehaviours']]
    if any(source.class_name(b) != 'animationEventScript' for b in behaviors):
        raise ValueError('Unsupported shovel state callback')
    vector = controller['m_StateMachineBehaviourVectorDescription']
    callbacks = {}
    for owner, r in vector['m_StateMachineBehaviourRanges']:
        if owner['m_LayerIndex'] != 0 or r['m_Count'] != 1:
            raise ValueError('Shovel callback range differs')
        callbacks[owner['m_StateID']] = source_id(behaviors[vector['m_StateMachineBehaviourIndices'][r['m_StartIndex']]])
    states = []
    for state in machine['m_StateConstantArray']:
        d = state['data']
        blends = d['m_BlendTreeConstantArray']
        if len(blends)!=1 or len(blends[0]['data']['m_NodeArray'])!=1 or d['m_BlendTreeConstantIndexArray']!=[0]:
            raise ValueError('Unsupported shovel blend tree')
        blend = blends[0]['data']['m_NodeArray'][0]['data']
        if blend['m_ChildIndices'] or blend['m_BlendEventID']!=2**32-1 or blend['m_BlendEventYID']!=2**32-1:
            raise ValueError('Unsupported shovel blend operation')
        states.append({'id':d['m_FullPathID'], 'name_id':d['m_NameID'], 'name':names[d['m_NameID']],
            'path_id':d['m_PathID'], 'tag_id':d['m_TagID'], 'clip_slot':blend['m_ClipID'],
            'speed':d['m_Speed'], 'speed_parameter':d['m_SpeedParamID'], 'cycle_offset':d['m_CycleOffset'],
            'cycle_offset_parameter':d['m_CycleOffsetParamID'], 'time_parameter':d['m_TimeParamID'],
            'mirror_parameter':d['m_MirrorParamID'], 'mirror':d['m_Mirror'], 'loop':d['m_Loop'],
            'ik_on_feet':d['m_IKOnFeet'], 'write_default_values':d['m_WriteDefaultValues'],
            'clip_duration':blend['m_Duration'], 'clip_cycle_offset':blend['m_CycleOffset'],
            'clip_mirror':blend['m_Mirror'], 'callback':callbacks.get(d['m_FullPathID']),
            'transitions':[transition(t['data']) for t in d['m_TransitionConstantArray']]})
    index = next(i for i in source.library.manifest['category_indexes'] if i['file']=='animations/index.json')
    exports = json.loads(checked(source.library.root,index))['exports']
    clips = []
    for identity in slots:
        export = next(e for e in exports if e['source']==identity)
        receipt = next(f for f in export['files'] if f['file']==export['details']['tracks_file'])
        record = json.loads(checked(source.library.root,receipt))
        tracks = []
        for track in record['tracks']:
            b = track['binding']
            target = next(t for t in b['targets'] if t['owner']==source_id(animator))
            if b['type_id']!=4 or b['attribute'] not in (1,4) or b['dimension']!=3 or b['is_pptr'] or b['script'] is not None or target['source']!={'file':'level1','path_id':23118}:
                raise ValueError('Unsupported shovel track binding')
            tracks.append({'node':node, 'attribute':b['attribute'], 'path_hash':b['path_hash'], 'custom_type':b['custom_type'],
                'curve_offset':b['curve_offset'], 'storage':b['storage'], 'interpolation':track['interpolation'],
                'times':track['times'], 'values':track['values'], 'in_tangents':track['in_tangents'],
                'out_tangents':track['out_tangents'], 'coefficients':track['coefficients']})
        if record['events']:
            raise ValueError('Unexpected shovel clip events; callbacks are state behaviors')
        muscle = record['settings']['muscle']
        clips.append({'source':identity, 'source_tracks':receipt, 'name':record['settings']['m_Name'],
            'start_time':muscle['m_StartTime'], 'stop_time':muscle['m_StopTime'],
            'sample_rate':record['settings']['m_SampleRate'], 'loop':muscle['m_LoopTime'],
            'cycle_offset':muscle['m_CycleOffset'], 'tracks':tracks, 'events':[]})
    layer = data['m_LayerArray'][0]['data']
    return {'controller':source_id(reader), 'handler':source_id(handler), 'animator':source_id(animator),
        'target':node, 'coordinate_space':'unity_parent_local', 'clip_slots':slots, 'parameters':parameters,
        'default_state':machine['m_DefaultState'], 'layer_binding':layer['m_Binding'],
        'layer_weight':layer['m_DefaultWeight'], 'layer_blending_mode':layer['(int&)m_LayerBlendingMode'],
        'layer_ik_pass':layer['m_IKPass'], 'states':states,
        'any_state_transitions':[transition(t['data']) for t in machine['m_AnyStateTransitionConstantArray']],
        'selectors':[{'id':s['data']['m_FullPathID'], 'entry':s['data']['m_IsEntry'],
            'destinations':[t['data']['m_Destination'] for t in s['data']['m_TransitionConstantArray']]}
            for s in machine['m_SelectorStateConstantArray']], 'clips':clips}


def validate_motion(m, manifest):
    from .acquisition import _shape,_source,_integer,_number,_boolean,_list,_numbers
    _shape(m,'controller handler animator target coordinate_space clip_slots parameters default_state layer_binding layer_weight layer_blending_mode layer_ik_pass states any_state_transitions selectors clips')
    finite_record(m)
    for field,pid,file in [('controller',4515,'sharedassets1.assets'),('handler',74298,'level1'),('animator',56797,'level1')]:
        _source(m[field])
        if m[field]!={'file':file,'path_id':pid}:raise ValueError('Shovel motion source differs')
    _integer(m['target'])
    node=next((n for n in manifest['nodes'] if n['id']==m['target']),{})
    if node.get('source')!={'file':'level1','path_id':23118} or m['coordinate_space']!='unity_parent_local':raise ValueError('Shovel motion target or coordinate space differs')
    if m['clip_slots']!=[{'file':'sharedassets1.assets','path_id':pid} for pid in CLIPS]:raise ValueError('Shovel clip slot links differ')
    parameters={}
    for p,name in zip(_list(m['parameters']),PARAMETERS,strict=True):
        _shape(p,'id name kind index default_float default_bool');_integer(p['id']);_integer(p['index'])
        if p['name']!=name or p['id'] in parameters:raise ValueError('Shovel parameter name or identity differs')
        parameters[p['id']]=p
        if p['kind']=='float':
            _number(p['default_float'])
            if name!='Speed' or p['default_bool'] is not None or p['default_float']!=0. or p['index']!=0:raise ValueError('Shovel float parameter differs')
        elif p['kind']=='trigger':
            _boolean(p['default_bool'])
            if name=='Speed' or p['default_float'] is not None or p['default_bool']:raise ValueError('Shovel trigger parameter differs')
        else:raise ValueError('Shovel parameter operation differs')
    if m['parameters']!=MOTION_PARAMETERS:raise ValueError('Shovel parameter source identities differ')
    for f in ('default_state','layer_binding','layer_blending_mode'):_integer(m[f])
    _number(m['layer_weight']);_boolean(m['layer_ik_pass'])
    if m['default_state']!=0 or m['layer_binding']!=756556552 or m['layer_blending_mode']!=0 or m['layer_weight']!=0. or m['layer_ik_pass']:raise ValueError('Shovel layer contract differs')
    def transition(t):
        _shape(t,'id full_path_id user_id destination duration offset exit_time has_exit_time fixed_duration can_transition_to_self interruption_source ordered_interruption conditions')
        for f in ('id','full_path_id','user_id','destination','interruption_source'):_integer(t[f])
        for f in ('duration','offset','exit_time'):_number(t[f])
        for f in ('has_exit_time','fixed_duration','can_transition_to_self','ordered_interruption'):_boolean(t[f])
        if t['destination']>=6 or t['duration']<0 or t['interruption_source']!=0 or t['user_id']!=0:raise ValueError('Shovel transition target differs')
        for c in _list(t['conditions']):
            _shape(c,'mode parameter threshold exit_time');_integer(c['mode']);_integer(c['parameter']);_number(c['threshold']);_number(c['exit_time'])
            if c['mode']!=1 or c['parameter'] not in parameters or parameters[c['parameter']]['kind']!='trigger' or c['threshold']!=0. or c['exit_time']!=0.:
                raise ValueError('Shovel transition condition differs')
    states=_list(m['states'])
    if len(states)!=6 or [s['id'] for s in states]!=list(STATES):raise ValueError('Shovel complete states differ')
    for i,s in enumerate(states):
        _shape(s,'id name_id name path_id tag_id clip_slot speed speed_parameter cycle_offset cycle_offset_parameter time_parameter mirror_parameter mirror loop ik_on_feet write_default_values clip_duration clip_cycle_offset clip_mirror callback transitions')
        for f in ('id','name_id','path_id','tag_id','clip_slot','speed_parameter','cycle_offset_parameter','time_parameter','mirror_parameter'):_integer(s[f])
        for f in ('speed','cycle_offset','clip_duration','clip_cycle_offset'):_number(s[f])
        for f in ('mirror','loop','ik_on_feet','write_default_values','clip_mirror'):_boolean(s[f])
        if type(s['name']) is not str or s['clip_slot']!=(1,0,2,3,4,5)[i] or s['path_id']!=s['id'] or s['speed']!=1. or s['loop']!=(i==0) or s['speed_parameter']!=(0 if i==0 else 3471299058):raise ValueError('Shovel state clip or speed binding differs')
        if s['mirror'] or s['ik_on_feet'] or s['clip_mirror'] or not s['write_default_values'] or any(s[f]!=0 for f in ('tag_id','cycle_offset','cycle_offset_parameter','time_parameter','mirror_parameter','clip_cycle_offset')):
            raise ValueError('Shovel state operation differs')
        if (s['name'],s['name_id'],s['clip_duration'])!=STATE_BINDINGS[i]:raise ValueError('Shovel state source fields differ')
        callback=None if i==0 else {'file':'sharedassets1.assets','path_id':(130091,130154,130160,130158,130156)[i-1]}
        if s['callback']!=callback:raise ValueError('Shovel attack callback target differs')
        if len(_list(s['transitions']))!=(5 if i==0 else 0):raise ValueError('Shovel state transitions incomplete')
        for t in s['transitions']:transition(t)
    if len(_list(m['any_state_transitions']))!=7:raise ValueError('Shovel AnyState transitions incomplete')
    for t in m['any_state_transitions']:transition(t)
    if m['any_state_transitions']!=ANY_TRANSITIONS or m['states'][0]['transitions']!=IDLE_TRANSITIONS:raise ValueError('Shovel checked transitions differ')
    for s in _list(m['selectors']):
        _shape(s,'id entry destinations');_integer(s['id']);_boolean(s['entry'])
        for d in _list(s['destinations']):_integer(d)
    if m['selectors']!=[{'id':756556552,'entry':True,'destinations':[0]},{'id':756556552,'entry':False,'destinations':[30000]}]:raise ValueError('Shovel controller selector differs')
    clips=_list(m['clips'])
    if len(clips)!=6:raise ValueError('Shovel complete clips required')
    for i,c in enumerate(clips):
        _shape(c,'source source_tracks name start_time stop_time sample_rate loop cycle_offset tracks events');_source(c['source'])
        if c['source']!=m['clip_slots'][i] or type(c['name']) is not str:raise ValueError('Shovel clip identity differs')
        receipt=c['source_tracks'];_shape(receipt,'bytes file sha256');_integer(receipt['bytes'])
        if receipt!=TRACK_RECEIPTS[i]:raise ValueError('Shovel checked track receipt differs')
        for f in ('start_time','stop_time','sample_rate','cycle_offset'):_number(c[f])
        _boolean(c['loop'])
        if c['sample_rate']!=60. or c['start_time']!=0. or c['stop_time']<c['start_time'] or c['cycle_offset']!=0. or c['events']!=[]:raise ValueError('Shovel clip timing or events differ')
        if c['stop_time']!=CLIP_TIMES[i] or c['loop']!=(i==1):raise ValueError('Shovel source clip length differs')
        if len(_list(c['tracks']))!=2:raise ValueError('Shovel track count differs')
        for j,t in enumerate(c['tracks']):
            _shape(t,'node attribute path_hash custom_type curve_offset storage interpolation times values in_tangents out_tangents coefficients')
            for f in ('node','attribute','path_hash','custom_type','curve_offset'):_integer(t[f])
            if t['node']!=m['target'] or t['attribute']!=(1,4)[j] or t['path_hash']!=0 or t['curve_offset']!=j*3 or t['custom_type']!=(0,4)[j]:raise ValueError('Shovel track binding differs')
            constant=i==1
            if (t['storage'],t['interpolation'])!=(('constant','CONSTANT') if constant else ('streamed','CUBIC')):raise ValueError('Shovel track interpolation differs')
            times=_list(t['times'])
            if len(times)!=(5,1,5,4,3,3)[i] or times[0]!=(.1666666716337204,0.,.1666666716337204,.1666666716337204,0.,.1666666716337204)[i] or times[-1]!=c['stop_time']:raise ValueError('Shovel source key extent differs')
            if not times or any(b<=a for a,b in zip(times,times[1:])):raise ValueError('Shovel track time order differs')
            for time in times:
                _number(time)
                if time<0 or time>c['stop_time']+1e-6:raise ValueError('Shovel track time out of clip')
            for f in ('values','in_tangents','out_tangents'):
                if len(_list(t[f]))!=len(times):raise ValueError('Shovel track key count differs')
                for row in t[f]:_numbers(row,3)
            coeff=_list(t['coefficients'])
            if len(coeff)!=(0 if constant else len(times)):raise ValueError('Shovel polynomial key count differs')
            for key in coeff:
                if len(_list(key))!=3:raise ValueError('Shovel polynomial dimension differs')
                for row in key:_numbers(row,3)

TRACK_RECEIPTS = [{'bytes': 9870,
  'file': 'animations/clips/sharedassets1.assets/3998/tracks.json',
  'sha256': '8851780dbb8517494a90bac266283eedb8acae717148c979e0f6e7fb34c1d239'},
 {'bytes': 6835,
  'file': 'animations/clips/sharedassets1.assets/4001/tracks.json',
  'sha256': '7035332bc89364a4a1f52db9a6c4455a813f4922ac6f600773930a9703363239'},
 {'bytes': 9832,
  'file': 'animations/clips/sharedassets1.assets/3999/tracks.json',
  'sha256': 'a70bbc9445f598dcf3a9cb55f482a00e17cfa232cd7dd5bb471c4016090efc89'},
 {'bytes': 9084,
  'file': 'animations/clips/sharedassets1.assets/3997/tracks.json',
  'sha256': '3eb3053001e6920fd0912df821ed2504996cddfc3891bbf2626ab8d544beeaa6'},
 {'bytes': 8314,
  'file': 'animations/clips/sharedassets1.assets/4002/tracks.json',
  'sha256': '3adf64bf7bfa84c12d5a54a42b9735424bb0744e28ed9caa21ccc27f360f3704'},
 {'bytes': 8344,
  'file': 'animations/clips/sharedassets1.assets/4003/tracks.json',
  'sha256': '7f94cbd4c86427963acbb8e5e08d7d5d3b26444e4fca2369fdea340c5449eb31'}]

MOTION_PARAMETERS = [{'default_bool': False,
  'default_float': None,
  'id': 2717246184,
  'index': 0,
  'kind': 'trigger',
  'name': 'AttackLeft'},
 {'default_bool': False,
  'default_float': None,
  'id': 1504273457,
  'index': 1,
  'kind': 'trigger',
  'name': 'AttackRight'},
 {'default_bool': False,
  'default_float': None,
  'id': 1708274416,
  'index': 2,
  'kind': 'trigger',
  'name': 'AttackForward'},
 {'default_bool': False,
  'default_float': None,
  'id': 3059532983,
  'index': 3,
  'kind': 'trigger',
  'name': 'AttackBack'},
 {'default_bool': False,
  'default_float': None,
  'id': 268324848,
  'index': 4,
  'kind': 'trigger',
  'name': 'AttackStab'},
 {'default_bool': None, 'default_float': 0.0, 'id': 3471299058, 'index': 0, 'kind': 'float', 'name': 'Speed'},
 {'default_bool': False,
  'default_float': None,
  'id': 1654612129,
  'index': 5,
  'kind': 'trigger',
  'name': 'Hit'}]

ANY_TRANSITIONS = [{'can_transition_to_self': True,
  'conditions': [{'exit_time': 0.0, 'mode': 1, 'parameter': 1504273457, 'threshold': 0.0}],
  'destination': 2,
  'duration': 0.10000000149011612,
  'exit_time': 0.800000011920929,
  'fixed_duration': True,
  'full_path_id': 666786194,
  'has_exit_time': True,
  'id': 1699225119,
  'interruption_source': 0,
  'offset': 0.0,
  'ordered_interruption': True,
  'user_id': 0},
 {'can_transition_to_self': True,
  'conditions': [{'exit_time': 0.0, 'mode': 1, 'parameter': 1708274416, 'threshold': 0.0}],
  'destination': 3,
  'duration': 0.10000000149011612,
  'exit_time': 0.800000011920929,
  'fixed_duration': True,
  'full_path_id': 834792901,
  'has_exit_time': True,
  'id': 3233637805,
  'interruption_source': 0,
  'offset': 0.0,
  'ordered_interruption': True,
  'user_id': 0},
 {'can_transition_to_self': True,
  'conditions': [{'exit_time': 0.0, 'mode': 1, 'parameter': 2717246184, 'threshold': 0.0}],
  'destination': 1,
  'duration': 0.10000000149011612,
  'exit_time': 0.800000011920929,
  'fixed_duration': True,
  'full_path_id': 129301733,
  'has_exit_time': True,
  'id': 1131093452,
  'interruption_source': 0,
  'offset': 0.0,
  'ordered_interruption': True,
  'user_id': 0},
 {'can_transition_to_self': False,
  'conditions': [{'exit_time': 0.0, 'mode': 1, 'parameter': 1654612129, 'threshold': 0.0}],
  'destination': 0,
  'duration': 0.10000000149011612,
  'exit_time': 9.427516545201797e-09,
  'fixed_duration': True,
  'full_path_id': 3058440661,
  'has_exit_time': False,
  'id': 4105854552,
  'interruption_source': 0,
  'offset': 0.0,
  'ordered_interruption': True,
  'user_id': 0},
 {'can_transition_to_self': True,
  'conditions': [{'exit_time': 0.0, 'mode': 1, 'parameter': 3059532983, 'threshold': 0.0}],
  'destination': 4,
  'duration': 0.10000000149011612,
  'exit_time': 0.800000011920929,
  'fixed_duration': True,
  'full_path_id': 1694822627,
  'has_exit_time': True,
  'id': 4259996521,
  'interruption_source': 0,
  'offset': 0.0,
  'ordered_interruption': True,
  'user_id': 0},
 {'can_transition_to_self': True,
  'conditions': [{'exit_time': 0.0, 'mode': 1, 'parameter': 268324848, 'threshold': 0.0}],
  'destination': 5,
  'duration': 0.10000000149011612,
  'exit_time': 0.800000011920929,
  'fixed_duration': True,
  'full_path_id': 3701898148,
  'has_exit_time': True,
  'id': 1145628718,
  'interruption_source': 0,
  'offset': 0.0,
  'ordered_interruption': True,
  'user_id': 0},
 {'can_transition_to_self': False,
  'conditions': [],
  'destination': 0,
  'duration': 0.10000000149011612,
  'exit_time': 0.800000011920929,
  'fixed_duration': True,
  'full_path_id': 3058440661,
  'has_exit_time': True,
  'id': 4105854552,
  'interruption_source': 0,
  'offset': 0.0,
  'ordered_interruption': True,
  'user_id': 0}]

IDLE_TRANSITIONS = [{'can_transition_to_self': True,
  'conditions': [{'exit_time': 0.0, 'mode': 1, 'parameter': 2717246184, 'threshold': 0.0}],
  'destination': 1,
  'duration': 0.10000000149011612,
  'exit_time': 0.0,
  'fixed_duration': True,
  'full_path_id': 156787784,
  'has_exit_time': False,
  'id': 3869188192,
  'interruption_source': 0,
  'offset': 0.0,
  'ordered_interruption': True,
  'user_id': 0},
 {'can_transition_to_self': True,
  'conditions': [{'exit_time': 0.0, 'mode': 1, 'parameter': 1504273457, 'threshold': 0.0}],
  'destination': 2,
  'duration': 0.10000000149011612,
  'exit_time': 0.0,
  'fixed_duration': True,
  'full_path_id': 2413259359,
  'has_exit_time': False,
  'id': 3129804257,
  'interruption_source': 0,
  'offset': 0.0,
  'ordered_interruption': True,
  'user_id': 0},
 {'can_transition_to_self': True,
  'conditions': [{'exit_time': 0.0, 'mode': 1, 'parameter': 1708274416, 'threshold': 0.0}],
  'destination': 3,
  'duration': 0.10000000149011612,
  'exit_time': 0.0,
  'fixed_duration': True,
  'full_path_id': 2557134833,
  'has_exit_time': False,
  'id': 3256176142,
  'interruption_source': 0,
  'offset': 0.0,
  'ordered_interruption': True,
  'user_id': 0},
 {'can_transition_to_self': True,
  'conditions': [{'exit_time': 0.0, 'mode': 1, 'parameter': 3059532983, 'threshold': 0.0}],
  'destination': 4,
  'duration': 0.10000000149011612,
  'exit_time': 0.0,
  'fixed_duration': True,
  'full_path_id': 3216411534,
  'has_exit_time': False,
  'id': 33870250,
  'interruption_source': 0,
  'offset': 0.0,
  'ordered_interruption': True,
  'user_id': 0},
 {'can_transition_to_self': True,
  'conditions': [{'exit_time': 0.0, 'mode': 1, 'parameter': 268324848, 'threshold': 0.0}],
  'destination': 5,
  'duration': 0.10000000149011612,
  'exit_time': 4.299437961208241e-08,
  'fixed_duration': True,
  'full_path_id': 101974217,
  'has_exit_time': False,
  'id': 3148241645,
  'interruption_source': 0,
  'offset': 0.0,
  'ordered_interruption': True,
  'user_id': 0}]

STATE_BINDINGS = [('katana_idle', 3361416310, 1.0),
 ('AttackLeft', 2717246184, 1.0),
 ('AttackRight', 1504273457, 1.0),
 ('katana_AttackForward', 1634851761, 1.0),
 ('shovel_AttackBack', 815580648, 1.0),
 ('shovel_AttackStab', 2302561967, 1.0)]

CLIP_TIMES = [0.6666666865348816, 0.0, 0.6666666865348816, 0.6666666865348816, 0.5833333134651184, 0.6666666865348816]
