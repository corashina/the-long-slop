"""Declared native angular inputs and evidence gates. No response model lives here."""
from copy import deepcopy
import json
import math
import struct
from pathlib import Path

SUPPORTS = ('full-tire', 'worn-tire', 'bare-rim', 'hub')
PROVENANCE = 'declared copied-rig input; native source profile copied without a response equation'
RIG = dict(use_gravity=False, drag=0., angular_drag=0., sleep_threshold=0.,
           velocity_mps=dict(x=10., y=0., z=0.), angular_velocity_rad_s=dict(x=0., y=0., z=0.),
           offset_m=dict(x=10000., y=10000., z=0.))

def events(values):
    return [dict(step=i*60+1, phase=phase, motor_nm=motor, brake_nm=brake)
            for i,(phase,motor,brake) in enumerate(values)]

EXCITATION = events([('positive-high',600.,0.),('positive-low',20.,0.),('coast-positive',0.,0.),
                     ('negative-high',-600.,0.),('negative-low',-20.,0.),('coast-negative',0.,0.),
                     ('positive-revisit',600.,0.),('coast-revisit',0.,0.)])
CONFIRMATION = events([('positive-independent',35.,0.),('coast-independent',0.,0.),
                       ('negative-independent',-45.,0.),('motor-brake-overlap',25.,40.),
                       ('brake-hold',0.,120.),('coast-held',0.,0.),
                       ('negative-restart',-15.,0.),('coast-negative',0.,0.)])
HELPER = events([('positive-high',3.,0.),('positive-low',1.,0.),('coast-positive',0.,0.),
                 ('negative-high',-3.,0.),('negative-low',-1.,0.),('coast-negative',0.,0.),
                 ('positive-revisit',1.,0.),('coast-revisit',0.,0.)])

IDENTIFICATION_INPUTS = (('angular-identification-full-tire-x-below-five-adjacent', 'angular-full-tire-excitation', (4.999999523162842, 0.0, 0.0), None), ('angular-identification-full-tire-x-five', 'angular-full-tire-excitation', (5.0, 0.0, 0.0), None), ('angular-identification-full-tire-x-above-five-adjacent', 'angular-full-tire-excitation', (5.000000476837158, 0.0, 0.0), None), ('angular-identification-full-tire-x-five-z-positive-ten', 'angular-full-tire-excitation', (5.0, 0.0, 10.0), None), ('angular-identification-full-tire-x-five-z-negative-ten', 'angular-full-tire-excitation', (5.0, 0.0, -10.0), None), ('angular-identification-worn-tire-zero-body-speed', 'angular-worn-tire-excitation', (0.0, 0.0, 0.0), None), ('angular-identification-full-tire-zero-low-brake-start', 'angular-full-tire-brake-excitation-zero-body-speed', (0.0, 0.0, 0.0), None), ('angular-identification-full-tire-zero-high-brake-start', 'angular-full-tire-brake-excitation-zero-body-speed', (0.0, 0.0, 0.0), 1800.0))

def angular_rows():
    """Generate the committed rows from existing checked setup metadata."""
    base = json.loads((Path(__file__).parents[2]/'tests/fixtures/driving/scenarios.json').read_text())
    template = next(r for r in base['scenarios'] if r['id']=='grip-full-tire-longitudinal')
    rows=[]
    for support,purpose in [(s,p) for s in SUPPORTS for p in ('excitation','confirmation')]+[('auxiliary-helper',p) for p in ('baseline','motor-coast')]:
        row=deepcopy(template)
        row.update(id='angular-'+support+'-'+purpose,probe_kind='wheel_angular', duration_frames=1800,
                   input_mode='keyboard-digital',parking_brake_intent='parked')
        row['setup'].update(support='full-tire' if support=='auxiliary-helper' else support,
                            surface='no copied contact geometry',velocity_mps=deepcopy(RIG['velocity_mps']))
        row['controls']=[dict(frame=0,throttle=0.,brake=0.,clutch=0.,steer=0.,ignition_button_held=False,handbrake_press=False)]
        schedule=CONFIRMATION if purpose=='confirmation' else HELPER if purpose=='motor-coast' else EXCITATION
        if purpose=='baseline': schedule=[dict(step=1,phase='baseline',motor_nm=0.,brake_nm=0.)]
        row['angular']=dict(support=support,purpose=purpose,source_wheel='level1:58655' if support=='auxiliary-helper' else 'level1:58684',
                            duration_steps=480,schedule_unit='native physics step from 1; assign before consuming solve',events=deepcopy(schedule),rig=dict(deepcopy(RIG),constraints=0 if support=='auxiliary-helper' else 116),
                            provenance=PROVENANCE,body_properties='first source records mass, centerOfMass, inertiaTensor, inertiaTensorRotation setter inputs in canonical Initial; later repeats reuse those exact inputs; live source and copied readbacks stay separate; Configure alone assigns copied properties; rig overrides explicit; remaining body settings retain measured constructor defaults')
        row['required_observations']=['one enabled contact-free native wheel','exact consuming physics solves','active signed spin and coast' if purpose!='baseline' else 'free-body baseline']
        row['frame_schedule']['sampling']='fixed-step angular assignment before physics.entry; native physics.exit; original callbacks unchanged'
        rows.append(row)
    zero=deepcopy(rows[0])
    zero['id']='angular-full-tire-excitation-zero-body-speed'
    zero['setup']['velocity_mps']=dict(x=0.,y=0.,z=0.)
    zero['angular']['rig']['velocity_mps']=dict(x=0.,y=0.,z=0.)
    brake=deepcopy(rows[0])
    brake['id']='angular-full-tire-brake-excitation'
    brake['angular']['purpose']='brake-excitation'
    brake['angular']['events']=events([
        ('positive-brake-start',600.,1.),('positive-unbraked-release',600.,0.),
        ('brake-stop-after-positive-release',0.,1800.),('positive-brake-restart',600.,1.),
        ('negative-brake-reversal',-600.,1.),('negative-unbraked-release',-600.,0.),
        ('brake-stop-after-negative-release',0.,1800.),('negative-brake-restart',-600.,1.)])
    brake['required_observations']=[
        'one enabled active contact-free native wheel and awake dynamic body',
        'exact consuming physics solves and actual motor and brake assignments',
        'observed initial zero RPM and terminal strong-brake zero intervals',
        'signed spin in both unbraked release phases',
        'observed low-brake response from qualified zero boundaries']
    rows.extend((zero,brake))
    # Frozen world inputs. Keep each parent's complete setup and observation contract.
    for identity,parent,velocity in (
        ('angular-full-tire-excitation-world-x-positive-below-five',0,(4.999000072479248,0.,0.)),
        ('angular-full-tire-excitation-world-x-positive-above-five',0,(5.000999927520752,0.,0.)),
        ('angular-full-tire-excitation-world-x-negative-below-five',0,(-4.999000072479248,0.,0.)),
        ('angular-full-tire-excitation-world-x-negative-above-five',0,(-5.000999927520752,0.,0.)),
        ('angular-full-tire-excitation-world-z-positive-ten',0,(0.,0.,10.)),
        ('angular-bare-rim-excitation-zero-body-speed',4,(0.,0.,0.)),
        ('angular-hub-excitation-zero-body-speed',6,(0.,0.,0.)),
        ('angular-auxiliary-helper-motor-coast-zero-body-speed',9,(0.,0.,0.)),
        ('angular-full-tire-brake-excitation-zero-body-speed',11,(0.,0.,0.)),
    ):
        clone=deepcopy(rows[parent])
        clone['id']=identity
        clone['setup']['velocity_mps']=dict(zip(('x','y','z'),velocity))
        clone['angular']['rig']['velocity_mps']=dict(zip(('x','y','z'),velocity))
        rows.append(clone)
    for identity,parent,velocity,brake in IDENTIFICATION_INPUTS:
        clone=deepcopy(next(row for row in rows if row['id']==parent))
        clone['id']=identity
        clone['setup']['velocity_mps']=dict(zip(('x','y','z'),velocity))
        clone['angular']['rig']['velocity_mps']=dict(zip(('x','y','z'),velocity))
        if brake is not None:clone['angular']['events'][0]['brake_nm']=brake
        rows.append(clone)
    return rows


def validate_angular_schedule(scenario):
    spec=scenario.get('angular',{})
    if scenario.get('probe_kind')!='wheel_angular' or spec.get('support') not in SUPPORTS+('auxiliary-helper',):
        raise ValueError('Angular support identity is missing')
    purpose=spec.get('purpose')
    if purpose not in ('excitation','confirmation','baseline','motor-coast','brake-excitation') or (spec['support']=='auxiliary-helper') != (purpose in ('baseline','motor-coast')):
        raise ValueError('Angular purpose differs from support')
    if purpose=='brake-excitation' and spec['support']!='full-tire':
        raise ValueError('Angular brake excitation requires full tire')
    if spec.get('source_wheel') != ('level1:58655' if spec['support']=='auxiliary-helper' else 'level1:58684') or spec.get('provenance')!=PROVENANCE:
        raise ValueError('Angular source identity or input provenance differs')
    expected=next((row for row in angular_rows() if row['id']==scenario['id']),None)
    if scenario!=expected:
        raise ValueError('Declared angular inputs changed; review a new row before measurement')
    validate_angular_declaration_words(scenario,expected)
    for event in spec['events']:
        if event['phase'].startswith('coast') and (event['motor_nm']!=0 or event['brake_nm']!=0):
            raise ValueError('Active coast requires zero motor and zero brake')
    return spec


def angular_initial_inputs(run):
    """Exact new setup state. Exclude process-specific IDs, as geometry already does."""
    report=run['angular']
    initial=deepcopy(report['initial_state'])
    del initial['wheel_id']
    del initial['body']['instance_id']
    return dict(initial_state=initial,source_profile=report['source_profile'],copied_profile=report['copied_profile'],
                source_transform=report['source_transform'],copied_transform=report['copied_transform'],
                body_input=report['body_input'],argument_words=report['argument_words'],wheel_input=report['wheel_input'])


BODY_INPUT_FIELDS = ('mass','center_of_mass','inertia_tensor','inertia_tensor_rotation')


def validate_angular_body_input(run,canonical,from_source):
    """Bind setter words to source arguments or the actual supplied canonical input."""
    report=run.get('angular',{})
    parsed=require_word_schema(report)
    body_input=report.get('body_input')
    if (type(report.get('body_input_from_source')) is not bool or
        report['body_input_from_source'] is not from_source or
        not isinstance(body_input,dict) or set(body_input)!=set(BODY_INPUT_FIELDS)):
        word_error('body input or origin is missing or differs')
    arguments=report['argument_words']
    if from_source:
        source=report.get('source_body',{})
        if canonical is not None or not set(BODY_INPUT_FIELDS)<=source.keys() or body_input!={key:source[key] for key in BODY_INPUT_FIELDS}:
            word_error('first-source body input provenance differs')
        if arguments!=report['source_argument_words'] or parsed['canonical_supplied'] is not False:
            word_error('first-source arguments or parser origin differ')
    else:
        if canonical is None or arguments!=body_words(canonical):
            word_error('reused canonical setter arguments differ')
        if parsed['canonical_supplied'] is not True or parsed['canonical_is_null'] is not False or parsed['canonical_arguments']!=arguments:
            word_error('parsed canonical arguments or origin differ')
    if any(run.get(name,{}).get('angular_body_input')!=body_input for name in ('setup_state','initial')):
        word_error('stored canonical body input differs from setter record')


WORD_SCHEMA = 2
VECTOR_FIELDS = ('x','y','z')
QUATERNION_FIELDS = (*VECTOR_FIELDS,'w')
BODY_READBACK_FIELDS = ('rotation','inertia_tensor_rotation','inertia_tensor')
TRANSFORM_READBACK_FIELDS = ('rotation','local_rotation','local_scale','lossy_scale')


def word_error(message):
    raise ValueError('Angular word '+message)


def word_shape(value, fields, label):
    if not isinstance(value,dict) or set(value)!=set(fields):
        word_error(label+' shape is missing or differs')
    return value


def binary32_word(value):
    if type(value) not in (int,float) or not math.isfinite(value):
        word_error('value is not finite numeric input')
    try:
        return struct.unpack('<i',struct.pack('<f',value))[0]
    except (OverflowError,struct.error):
        word_error('value is outside binary32')


def binary32_value(word):
    if type(word) is not int or not -2147483648<=word<=2147483647:
        word_error('evidence must be typed signed int32')
    value=struct.unpack('<f',struct.pack('<i',word))[0]
    if not math.isfinite(value):
        word_error('candidate value is not finite')
    return value


def vector_words(value, fields=VECTOR_FIELDS):
    word_shape(value,fields,'vector input')
    return {key:binary32_word(value[key]) for key in fields}


def body_words(value):
    word_shape(value,BODY_INPUT_FIELDS,'body input')
    return dict(mass=binary32_word(value['mass']),
                center_of_mass=vector_words(value['center_of_mass']),
                inertia_tensor=vector_words(value['inertia_tensor']),
                inertia_tensor_rotation=vector_words(value['inertia_tensor_rotation'],(*VECTOR_FIELDS,'w')))


def body_word_values(value):
    word_shape(value,BODY_INPUT_FIELDS,'body arguments')
    result=dict(mass=binary32_value(value['mass']))
    for name,fields in (('center_of_mass',VECTOR_FIELDS),('inertia_tensor',VECTOR_FIELDS),('inertia_tensor_rotation',(*VECTOR_FIELDS,'w'))):
        word_shape(value[name],fields,name)
        result[name]={key:binary32_value(value[name][key]) for key in fields}
    return result


def vector_word_values(value, fields=VECTOR_FIELDS):
    word_shape(value,fields,'observation vector')
    return {key:binary32_value(value[key]) for key in fields}


def body_readback_values(value):
    word_shape(value,BODY_READBACK_FIELDS,'body readback')
    return {key:vector_word_values(value[key],VECTOR_FIELDS if key=='inertia_tensor' else QUATERNION_FIELDS)
            for key in BODY_READBACK_FIELDS}


def transform_readback_values(value):
    word_shape(value,TRANSFORM_READBACK_FIELDS,'transform readback')
    return {key:vector_word_values(value[key],QUATERNION_FIELDS if key in ('rotation','local_rotation') else VECTOR_FIELDS)
            for key in TRANSFORM_READBACK_FIELDS}


def require_word_schema(report):
    if type(report.get('word_schema_version')) is not int or report['word_schema_version']!=WORD_SCHEMA:
        word_error('schema is missing or obsolete')
    for name in ('argument_words','source_argument_words'):
        body_word_values(report.get(name))
    vector_word_values(report.get('source_body_rotation_words'),QUATERNION_FIELDS)
    for name in ('copied_constructor_words','copied_held_words'):
        body_readback_values(report.get(name))
    for name in ('source_transform_words','copied_transform_words'):
        transform_readback_values(report.get(name))
    wheel=word_shape(report.get('wheel_input_words'),('rotation','local_scale'),'wheel setter arguments')
    vector_word_values(wheel['rotation'],QUATERNION_FIELDS)
    vector_word_values(wheel['local_scale'])
    parsed=word_shape(report.get('parsed_words'),('observed','canonical_supplied','canonical_is_null','setup_velocity','rig_velocity','rig_angular_velocity','canonical_arguments'),'parser')
    if parsed['observed'] is not True or any(type(parsed[k]) is not bool for k in ('canonical_supplied','canonical_is_null')):
        word_error('parser observation or origin is missing')
    body_word_values(parsed['canonical_arguments'])
    return parsed


def angular_transport_initial(run):
    """Separate request copy. Preserve all recorded physical Initial fields."""
    result=deepcopy(run['setup_state'])
    result['angular_body_input']=body_word_values(run['angular']['argument_words'])
    return result



def validate_angular_declaration_words(scenario,expected):
    """Keep the frozen numeric declarations, including zero signs and types."""
    for actual,wanted in ((scenario['setup']['velocity_mps'],expected['setup']['velocity_mps']),
                          (scenario['angular']['rig']['velocity_mps'],expected['angular']['rig']['velocity_mps']),
                          (scenario['angular']['rig']['angular_velocity_rad_s'],expected['angular']['rig']['angular_velocity_rad_s'])):
        word_shape(actual,VECTOR_FIELDS,'declared vector')
        if any(type(actual[key]) is not type(wanted[key]) for key in VECTOR_FIELDS) or vector_words(actual)!=vector_words(wanted):
            word_error('declared vector differs from registered input')
    for actual,wanted in zip(scenario['angular']['events'],expected['angular']['events']):
        for key in ('motor_nm','brake_nm'):
            if type(actual[key]) is not type(wanted[key]) or binary32_word(actual[key])!=binary32_word(wanted[key]):
                word_error('declared torque/brake differs from registered input')


def validate_angular_supplied(run,scenario,supplied,canonical):
    """Read the actual supplied input, independently of receipts or capture."""
    source=canonical is None
    expected=deepcopy(scenario)
    if not source:
        expected.update(has_setup_state=True,setup_state=canonical)
        actual=supplied.get('setup_state',{}).get('angular_body_input')
        if body_words(actual)!=run['angular']['argument_words']:
            word_error('supplied canonical arguments differ')
    if supplied!=expected:
        word_error('supplied scenario or first/reused origin differs')
    validate_angular_declaration_words(supplied,scenario)
    parsed=require_word_schema(run['angular'])
    if parsed['canonical_supplied'] is not (not source):
        word_error('supplied parser origin differs')


def validate_angular_words(run,scenario):
    """Finite binary32 view. Raw producer floats remain unchanged."""
    report=run.get('angular',{})
    parsed=require_word_schema(report)
    spec=scenario['angular']
    for key,expected in (('setup_velocity',scenario['setup']['velocity_mps']),
                         ('rig_velocity',spec['rig']['velocity_mps']),
                         ('rig_angular_velocity',spec['rig']['angular_velocity_rad_s'])):
        word_shape(parsed[key],VECTOR_FIELDS,key)
        if any(type(parsed[key][axis]) is not int for axis in VECTOR_FIELDS) or parsed[key]!=vector_words(expected):
            word_error('parsed '+key+' differs from frozen declaration')
    for name in ('source_profile_words','copied_profile_words'):
        values=word_shape(report.get(name),('mass','radius','damping'),name)
        for value in values.values():binary32_value(value)
    if report['source_profile_words']!=report['copied_profile_words']:
        word_error('source/copy profile binding differs')
    initial=word_shape(report.get('initial_words'),('observed','wheel_id','fixed_dt','rpm','velocity','angular_velocity','body'),'initial')
    body_readback_values(initial['body'])
    for name,owner in (('source_body','original_body_id'),('copied_constructor_body','copied_body_id'),('copied_held_body','copied_body_id')):
        identity=report.get(name,{}).get('instance_id')
        if type(identity) is not int or type(report.get(owner)) is not int or identity!=report.get(owner):
            word_error(name+' body ownership differs')
    state=report.get('initial_state',{})
    if (type(state.get('wheel_id')) is not int or type(state.get('body',{}).get('instance_id')) is not int or
        state.get('wheel_id')!=report.get('copied_wheel_id') or state.get('body',{}).get('instance_id')!=report.get('copied_body_id')):
        word_error('initial saved State ownership differs')
    if initial['observed'] is not True or type(initial['wheel_id']) is not int or initial['wheel_id']!=report['copied_wheel_id']:
        word_error('initial selected wheel binding differs')
    if binary32_value(initial['fixed_dt'])<=0 or type(initial['rpm']) is not int or initial['rpm']!=0:
        word_error('initial timestep or positive zero RPM differs')
    for key,expected in (('velocity',spec['rig']['velocity_mps']),('angular_velocity',spec['rig']['angular_velocity_rad_s'])):
        word_shape(initial[key],VECTOR_FIELDS,'initial '+key)
        if any(type(initial[key][axis]) is not int for axis in VECTOR_FIELDS) or initial[key]!=vector_words(expected):
            word_error('initial '+key+' differs')
    wheels=run.get('initial',{}).get('wheel_state',[])
    if len(wheels)!=1 or wheels[0].get('rpm')!=0:
        word_error('stored Initial selected wheel zero is missing')
    pairs={}
    previous_sequence=None
    for sample in run.get('samples',[]):
        if sample.get('phase') not in ('physics.entry','physics.exit'):continue
        words=word_shape(sample.get('angular_words'),('observed','wheel_id','sequence','step','phase','fixed_dt','motor_nm','brake_nm','rpm','body','velocity'),'sample')
        body_readback_values(words['body'])
        vector_word_values(words['velocity'])
        state=sample.get('angular_state',{})
        if (type(state.get('wheel_id')) is not int or type(state.get('body',{}).get('instance_id')) is not int or
            state.get('wheel_id')!=report.get('copied_wheel_id') or state.get('body',{}).get('instance_id')!=report.get('copied_body_id')):
            word_error('sample saved State ownership differs')
        if words['observed'] is not True:
            word_error('sample observation is missing')
        for key,expected in (('wheel_id',report['copied_wheel_id']),('sequence',sample['sequence']),('step',sample['step'])):
            if type(words[key]) is not int or words[key]!=expected:
                word_error('sample '+key+' binding differs')
        if previous_sequence is not None and words['sequence']<=previous_sequence:
            word_error('sample sequence order differs')
        previous_sequence=words['sequence']
        pair=pairs.setdefault(words['step'],[])
        pair.append(words['phase'])
        if words['phase']!=sample['phase']:
            word_error('sample phase binding differs')
        for key in ('fixed_dt','motor_nm','brake_nm','rpm'):binary32_value(words[key])
        if words['fixed_dt']!=initial['fixed_dt']:
            word_error('consuming timestep differs from initial')
        event=next((e for e in reversed(spec['events']) if e['step']<=sample['step']),None)
        if event is None or words['motor_nm']!=binary32_word(event['motor_nm']) or words['brake_nm']!=binary32_word(event['brake_nm']):
            word_error('consuming motor/brake binding differs')
    if sorted(pairs)!=list(range(1,spec['duration_steps']+1)) or any(phases!=['physics.entry','physics.exit'] for phases in pairs.values()):
        word_error('complete entry/exit linkage differs')



def angular_observation_view(run):
    """Decode saved observation words. Preserve producer text and argument records."""
    report=run['angular']
    require_word_schema(report)
    source=body_word_values(report['source_argument_words'])
    initial=report['initial_words']
    wheel=report['wheel_input_words']
    return dict(source_body=dict(rotation=vector_word_values(report['source_body_rotation_words'],QUATERNION_FIELDS),
                                inertia_tensor=source['inertia_tensor'],inertia_tensor_rotation=source['inertia_tensor_rotation']),
                copied_constructor_body=body_readback_values(report['copied_constructor_words']),
                copied_held_body=body_readback_values(report['copied_held_words']),
                source_transform=transform_readback_values(report['source_transform_words']),
                copied_transform=transform_readback_values(report['copied_transform_words']),
                wheel_input=dict(rotation=vector_word_values(wheel['rotation'],QUATERNION_FIELDS),local_scale=vector_word_values(wheel['local_scale'])),
                initial=dict(wheel_id=initial['wheel_id'],body=body_readback_values(initial['body']),velocity=vector_word_values(initial['velocity'])),
                samples=[dict(sequence=s['angular_words']['sequence'],step=s['angular_words']['step'],phase=s['angular_words']['phase'],
                              wheel_id=s['angular_words']['wheel_id'],body=body_readback_values(s['angular_words']['body']),
                              velocity=vector_word_values(s['angular_words']['velocity']))
                         for s in run['samples'] if s['phase'] in ('physics.entry','physics.exit')])


def angular_candidate_view(run):
    """Decode only the finite candidate fields. Do not alter native records."""
    report=run['angular']
    require_word_schema(report)
    profile=word_shape(report['copied_profile_words'],('mass','radius','damping'),'candidate profile')
    return dict(profile={key:binary32_value(value) for key,value in profile.items()},
                fixed_dt=binary32_value(report['initial_words']['fixed_dt']),
                samples=[dict(sequence=s['sequence'],step=s['step'],phase=s['phase'],wheel_id=s['angular_words']['wheel_id'],
                              motor_nm=binary32_value(s['angular_words']['motor_nm']),
                              brake_nm=binary32_value(s['angular_words']['brake_nm']),
                              rpm=binary32_value(s['angular_words']['rpm']))
                         for s in run['samples'] if s['phase']=='physics.exit'])


def validate_source_ownership(native,expected,original_id,original_key):
    fields={'key','instance_id','source_index','road_slot','owner_instance_id','owner_is_null','native_association_available',
            'structural_owner_key','structural_owner_instance_id','structural_owner_bound','structural_links_valid',
            'enabled','active_self','active_hierarchy'}
    if len(native)!=5 or any(not fields<=wheel.keys() for wheel in native):
        raise ValueError('Angular source ownership evidence is missing')
    if ([w['source_index'] for w in native]!=list(range(5)) or len({w['key'] for w in native})!=5 or
        {w['key']:(w['instance_id'],w['road_slot']) for w in native}!=expected):
        raise ValueError('Angular source native wheel identity or cohort differs')
    for wheel in native:
        booleans=('owner_is_null','native_association_available','structural_owner_bound','structural_links_valid','enabled','active_self','active_hierarchy')
        if any(type(wheel[k]) is not bool for k in booleans) or type(wheel['owner_instance_id']) is not int:
            raise ValueError('Angular native association availability is missing or invalid')
        if (wheel['structural_owner_bound'] is not True or wheel['structural_links_valid'] is not True or
            wheel['structural_owner_key']!=original_key or wheel['structural_owner_instance_id']!=original_id):
            raise ValueError('Angular bound structural owner or parent links changed')
        available=wheel['native_association_available']
        if available!= (wheel['owner_instance_id']!=0) or wheel['owner_is_null']==available:
            raise ValueError('Angular raw native association and availability differ')
        if available and wheel['owner_instance_id']!=original_id:
            raise ValueError('Angular reported native owner differs')
        if not available and wheel['enabled'] and wheel['active_hierarchy']:
            raise ValueError('Angular enabled active native association is missing')
        if wheel['enabled']:
            raise ValueError('Angular original source wheel isolation changed')


def validate_angular_inertia(body,declared,constraints,step=0,phase='initial'):
    """Keep copied input separate from the observed frozen-axis solve state."""
    expected=declared
    # These checked rows freeze all rotation axes or none. PositionY adds no tensor rule.
    if constraints & 112 == 112 and (step>1 or phase=='physics.exit'):
        expected=dict(x=0.,y=0.,z=0.)
    if body.get('constraints')!=constraints or body.get('inertia_tensor')!=expected:
        raise ValueError('Angular constrained body inertia tensor or constraints differ')


def validate_angular(run,scenario):
    spec=validate_angular_schedule(scenario)
    report=run.get('angular',{})
    validate_angular_words(run,scenario)
    declared=deepcopy(report.get('declared',{}));expected=deepcopy(spec)
    for value in (declared,expected):
        if isinstance(value.get('rig'),dict):
            value['rig'].pop('velocity_mps',None);value['rig'].pop('angular_velocity_rad_s',None)
    if declared!=expected or report.get('source_key')!=spec['source_wheel'] or report.get('support')!=spec['support']:
        raise ValueError('Angular profile identity or provenance differs')
    trace=run.get('body_trace',{})
    if (not report.get('copied_wheel_id') or report.get('source_wheel_id')==report.get('copied_wheel_id') or
        report.get('original_body_id')!=trace.get('retained_instance_id') or report.get('copied_body_id')!=trace.get('copied_instance_id')):
        raise ValueError('Angular original/copy identities differ')
    shapes=[s for snap in run.get('geometry',[]) if snap['phase']=='armed-after-canonical' for s in snap['colliders'] if s['key']==spec['source_wheel']]
    if len(shapes)!=1 or shapes[0]['instance_id']!=report.get('source_wheel_id') or shapes[0]['sample']['enabled']:
        raise ValueError('Angular live source profile or source isolation is missing')
    profile=report.get('source_profile')
    fields=('center','radius','mass','damping','suspension_distance','force_application_distance','spring_n_per_m','damper_n_s_per_m','spring_target','forward','sideways')
    if not isinstance(profile,dict) or any(profile.get(k)!=shapes[0].get(k) for k in fields) or report.get('copied_profile')!=profile:
        raise ValueError('Angular copied profile differs from live source')
    native_expected={shape['key']:(shape['instance_id'],shape['road_slot']) for shape in run['geometry'][-1]['colliders'] if shape['type']=='WheelCollider'}
    original_key=trace.get('retained_body')
    if not original_key: raise ValueError('Angular retained structural body key is missing')
    for sample in run.get('samples',[]):
        validate_source_ownership(sample.get('native_wheels',[]),native_expected,report['original_body_id'],original_key)
    checkpoints=run.get('wheel_ownership',[])
    geometry=run.get('geometry',[])
    phases=[row['phase'] for row in geometry]
    if 'prepared-held' not in phases: raise ValueError('Angular prepared structural checkpoint is missing')
    checked_geometry=geometry[phases.index('prepared-held'):]
    expected_phases=[row['phase'] for row in checked_geometry]
    for snapshot in checked_geometry:
        native=[shape.get('sample',{}) for shape in snapshot['colliders'] if shape['type']=='WheelCollider']
        validate_source_ownership(sorted(native,key=lambda wheel:wheel.get('source_index',-1)),native_expected,report['original_body_id'],original_key)
    if [row.get('phase') for row in checkpoints]!=['source-role-bound',*expected_phases,'measurement-complete']:
        raise ValueError('Angular structural ownership checkpoints are missing or reordered')
    for index,checkpoint in enumerate(checkpoints):
        if checkpoint.get('order')!=index+1 or any(type(checkpoint.get(k)) is not int for k in ('frame','physics_step')) or type(checkpoint.get('time')) not in (int,float):
            raise ValueError('Angular structural checkpoint order or clocks are missing')
        validate_source_ownership(checkpoint.get('wheels',[]),native_expected,report['original_body_id'],original_key)
    for name in ('source_sprung_mass','copied_sprung_mass'):
        if type(report.get(name)) not in (int,float): raise ValueError('Separate source/copy sprung mass is missing')
    source_body=report.get('source_body',{})
    if not source_body or source_body.get('instance_id')!=report['original_body_id']:
        raise ValueError('Angular source body provenance is missing')
    constructor=report.get('copied_constructor_body',{})
    held=report.get('copied_held_body',{})
    body_input=report.get('body_input',{})
    from_source=report.get('body_input_from_source')
    validate_angular_body_input(run,None if from_source else body_word_values(report['argument_words']),from_source)
    if not constructor or not held or any(held.get(k)!=body_input[k] for k in ('mass','center_of_mass','inertia_tensor')):
        raise ValueError('Copied body source-property provenance differs')
    for name in ('source_transform','copied_transform'):
        if not {'position','rotation','local_position','local_rotation','local_scale','lossy_scale'}<=report.get(name,{}).keys():
            raise ValueError('Angular component transform provenance is missing')
    from .driving_controls import f32
    source_pose=report['source_transform'];wheel_input=report.get('wheel_input',{})
    if (wheel_input.get('rotation')!=source_pose['rotation'] or wheel_input.get('local_scale')!=source_pose['lossy_scale'] or
        wheel_input.get('position')!={axis:f32(source_pose['position'][axis]+spec['rig']['offset_m'][axis]) for axis in ('x','y','z')}):
        raise ValueError('Angular copied component transform differs from declared offset')
    body_fields={'instance_id','constraints','mass','center_of_mass','inertia_tensor','inertia_tensor_rotation','position','rotation','velocity','angular_velocity','gravity','use_gravity','drag','angular_drag','sleeping','sleep_threshold','solver_iterations','solver_velocity_iterations','max_angular_velocity','detect_collisions','kinematic','interpolation','collision_detection_mode'}
    initial_state=report.get('initial_state',{})
    initial_body=initial_state.get('body',{})
    rig=spec['rig']
    setup_fields=('mass','center_of_mass','inertia_tensor_rotation','solver_iterations','solver_velocity_iterations','max_angular_velocity','detect_collisions','interpolation','collision_detection_mode')
    if any(initial_body.get(k)!=held.get(k) for k in setup_fields):
        raise ValueError('Angular copied body setup changed')
    validate_angular_inertia(held,body_input['inertia_tensor'],rig['constraints'])
    validate_angular_inertia(initial_body,body_input['inertia_tensor'],rig['constraints'])
    if (not body_fields<=initial_body.keys() or initial_state.get('wheel_id')!=report['copied_wheel_id'] or not initial_state.get('enabled') or
        initial_state.get('event_revision')!=0 or initial_state.get('assignment_step')!=0 or
        initial_state.get('phase')!='before-first-assignment' or
        initial_body.get('instance_id')!=report['copied_body_id'] or initial_body.get('kinematic') is not False):
        raise ValueError('Angular one-time initial body input is missing or differs')
    new_row=any(row['id']==scenario['id'] for row in angular_rows()[10:])
    if new_row and (initial_body['sleeping'] is not False or
                    initial_state.get('active_self') is not True or initial_state.get('active_hierarchy') is not True):
        raise ValueError('Angular initial boundary requires an awake dynamic body and active wheel')
    if spec['purpose']=='brake-excitation':
        initial_wheels=run.get('initial',{}).get('wheel_state',[])
        if (len(initial_wheels)!=1 or initial_wheels[0].get('rpm')!=0 or
            initial_wheels[0].get('motor_torque')!=0 or initial_wheels[0].get('brake_torque')!=0 or
            initial_wheels[0].get('contact') is not False):
            raise ValueError('Angular brake initial wheel requires observed zero RPM and zero inputs')
    pairs={}
    for sample in run.get('samples',[]):
        if sample['phase'] in ('physics.entry','physics.exit'):
            pairs.setdefault(sample['step'],{})[sample['phase']]=sample
    if sorted(pairs)!=list(range(1,spec['duration_steps']+1)):
        raise ValueError('Angular consuming solve coverage differs')
    revisions=[]; by_event={}; crossings=[]; held_zero=[]
    for step,pair in pairs.items():
        if set(pair)!={'physics.entry','physics.exit'}: raise ValueError('Angular solve pair is missing')
        event_index=max(i for i,event in enumerate(spec['events']) if event['step']<=step)
        event=spec['events'][event_index]
        for sample in pair.values():
            observation=sample.get('angular_state',{})
            body=observation.get('body',{})
            wheel=sample.get('wheels',[])
            if len(wheel)!=1 or not body_fields<=body.keys() or body['instance_id']!=report['copied_body_id'] or body['kinematic']:
                raise ValueError('Angular body state or one-wheel evidence is missing')
            if (observation.get('event_revision')!=event_index+1 or observation.get('assignment_step')!=step or
                observation.get('phase')!=event['phase'] or wheel[0]['motor_torque']!=event['motor_nm'] or wheel[0]['brake_torque']!=event['brake_nm']):
                raise ValueError('Angular assigned input or consuming solve differs')
            if wheel[0].get('contact') is not False or not observation.get('enabled') or not observation.get('active_self') or not observation.get('active_hierarchy'):
                raise ValueError('Angular contact-free enabled evidence is missing')
            if observation.get('wheel_id')!=report['copied_wheel_id'] or any(observation.get(k)!=profile.get(k) for k in fields):
                raise ValueError('Angular measured wheel profile changed')
            if observation.get('sprung_mass_available') is not True or type(observation.get('sprung_mass')) not in (float,int):
                raise ValueError('Angular native sprung mass is missing')
            validate_angular_inertia(body,body_input['inertia_tensor'],rig['constraints'],step,sample['phase'])
            if any(body[k]!=held[k] for k in setup_fields):
                raise ValueError('Angular copied body setup changed')
            rig=spec['rig']
            if any(body[k]!=v for k,v in [('constraints',rig['constraints']),('use_gravity',rig['use_gravity']),('drag',rig['drag']),('angular_drag',rig['angular_drag']),('sleep_threshold',rig['sleep_threshold'])]):
                raise ValueError('Angular declared body input differs')
            if spec['purpose']!='baseline' and body['sleeping']:
                if new_row:
                    raise ValueError(f"Inactive sleeping getter at step {step} {sample['phase']} cannot establish angular response")
                raise ValueError('Inactive sleeping getter cannot establish angular response')
            if sample.get('fixed_dt')!=run['initial']['fixed_dt'] or not {'frame','delta_time','time','input_sequence','phase'}<=sample.keys():
                raise ValueError('Angular clock or input provenance is missing')
        before,after=pair['physics.entry'],pair['physics.exit']
        if before['sequence']>=after['sequence']: raise ValueError('Angular solve exit precedes entry')
        revisions.append(before['angular_state']['event_revision'])
        by_event.setdefault(event_index,[]).append((before['wheels'][0]['rpm'],after['wheels'][0]['rpm'],after['angular_state']['body']['sleeping']))
        left,right=before['wheels'][0]['rpm'],after['wheels'][0]['rpm']
        if left*right<0 or left!=0 and right==0: crossings.append(dict(step=step,phase=event['phase'],before_rpm=left,after_rpm=right))
        if left==right==0 and event['phase']=='brake-hold': held_zero.append(step)
    if sorted(set(revisions))!=list(range(1,len(spec['events'])+1)):
        raise ValueError('Angular one-time event is missing')
    actual=report.get('assignments',[])
    if actual!=[dict(event_revision=i+1,**event) for i,event in enumerate(spec['events'])]:
        raise ValueError('Angular one-time event was missed or repeated')
    if spec['purpose']!='baseline':
        values=[value for rows in by_event.values() for pair in rows for value in pair[:2]]
        if not min(values)<0<max(values): raise ValueError('Angular actual signed spin is missing')
        for i,event in enumerate(spec['events']):
            rows=by_event[i]
            if event['phase'].startswith('coast') and event['phase']!='coast-held' and not any(left!=0 and right!=left for left,right,_ in rows):
                raise ValueError('Angular active nonzero coast response is missing')
            if event['motor_nm'] and event['brake_nm']==0 and not any(right*event['motor_nm']>0 and left!=right for left,right,_ in rows):
                raise ValueError('Angular actual excitation is missing')
        if spec['purpose'] in ('excitation','motor-coast'):
            for sign in (-1,1):
                groups=[]
                for i,event in enumerate(spec['events']):
                    if event['motor_nm']*sign>0 and event['brake_nm']==0:
                        speeds=[v*sign for row in by_event[i] for v in row[:2] if v*sign>0]
                        groups.append((event['motor_nm'],min(speeds),max(speeds)))
                if not any(a[0]!=b[0] and max(a[1],b[1])<min(a[2],b[2]) for a in groups for b in groups):
                    raise ValueError('Distinct torque levels lack overlapping observed signed RPM')
        if spec['purpose']=='confirmation':
            overlap=by_event[next(i for i,e in enumerate(spec['events']) if e['phase']=='motor-brake-overlap')]
            if not any(left!=0 and right!=left for left,right,_ in overlap):
                raise ValueError('Angular actual motor/brake overlap response is missing')
            if len(held_zero)<2 or not any(b==a+1 for a,b in zip(held_zero,held_zero[1:])) or not any(c['phase'] in ('motor-brake-overlap','brake-hold') for c in crossings):
                raise ValueError('Angular observed crossing and held-zero interval are missing')
    brake_response={}
    if spec['purpose']=='brake-excitation':
        brake_response=validate_brake_excitation(spec,by_event,pairs,crossings)
        held_zero=[179,180,419,420]
    return dict(**brake_response,steps=len(pairs),events=len(actual),support=spec['support'],purpose=spec['purpose'],crossings=crossings,held_zero_steps=held_zero,
                native_internal_inertia='unavailable',native_solver_impulses='unavailable')


def validate_brake_excitation(spec,by_event,pairs,crossings):
    """Qualify observed zero boundaries without choosing the low-brake response."""
    def rpm(step,phase):return pairs[step][phase]['wheels'][0]['rpm']
    for step in (1,181,421):
        if rpm(step,'physics.entry')!=0:
            raise ValueError(f'Angular brake zero entry is missing at step {step}')
    for step in (179,180,419,420):
        for phase in ('physics.entry','physics.exit'):
            if rpm(step,phase)!=0:
                raise ValueError(f'Angular brake terminal zero is missing at step {step} {phase}')
    low=[];strong=[]
    for index,event in enumerate(spec['events']):
        if event['brake_nm']==0:continue
        solves=[dict(step=event['step']+offset,entry_rpm=left,exit_rpm=right)
                for offset,(left,right,_) in enumerate(by_event[index])]
        if event['motor_nm']!=0:
            low.append(dict(phase=event['phase'],solves=solves,
                            zero_to_zero_steps=[s['step'] for s in solves if s['entry_rpm']==s['exit_rpm']==0],
                            zero_to_nonzero_steps=[s['step'] for s in solves if s['entry_rpm']==0 and s['exit_rpm']!=0]))
        else:
            zeros=[dict(step=s['step'],boundary=boundary) for s in solves
                   for boundary,key in (('physics.entry','entry_rpm'),('physics.exit','exit_rpm')) if s[key]==0]
            strong.append(dict(phase=event['phase'],solves=solves,first_observed_zero=zeros[0],
                               crossings=[c for c in crossings if c['phase']==event['phase']],
                               terminal_hold_steps=[s['step'] for s in solves[-2:]]))
    return dict(low_brake_response=low,strong_brake_response=strong)

if __name__=='__main__':
    print(json.dumps(dict(schema_version=1,scenarios=angular_rows()),indent=2))
