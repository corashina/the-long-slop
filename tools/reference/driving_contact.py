"""Fixed single-wheel contact rig inputs, boundary validation, and body momentum bookkeeping."""
from copy import deepcopy
import hashlib
import json
import math
from pathlib import Path
from .driving_angular import BODY_INPUT_FIELDS, validate_source_ownership, validate_angular_inertia
from .driving_controls import f32

ZERO=dict(x=0.,y=0.,z=0.)
PROVENANCE='declared copied-rig contact input; body momentum residual is not a native contact impulse'
PROFILE_FIELDS=('center','radius','mass','damping','suspension_distance','force_application_distance','spring_n_per_m','damper_n_s_per_m','spring_target','forward','sideways')

def contact_rows():
    base=json.loads((Path(__file__).parents[2]/'tests/fixtures/driving/scenarios.json').read_text())
    template=next(r for r in base['scenarios'] if r['id']=='grip-full-tire-longitudinal')
    rows=[]
    for suffix,side,scale,enabled in [('baseline',0.,0.,True),('longitudinal',0.,.25,True),('lateral',.5,0.,True),('combined',.5,.25,True),('disabled-combined',.5,.25,False)]:
        row=deepcopy(template)
        row.update(id='contact-full-tire-'+suffix,probe_kind='single_wheel_contact',duration_frames=1800,input_mode='keyboard-digital',parking_brake_intent='parked')
        row['setup'].update(support='full-tire',surface='measured static copied support',load_multiplier=1.,removed_driven_rims=0,velocity_mps=dict(x=0.,y=0.,z=5.))
        row['controls']=[dict(frame=0,throttle=0.,brake=0.,clutch=0.,steer=0.,ignition_button_held=False,handbrake_press=False)]
        row['contact']=dict(source_wheel='level1:58684',support='full-tire',mode='force',duration_steps=240,event_steps=[1,121,145],wheel_enabled=enabled,side_speed_mps=side,torque_scale=scale,offset_m=dict(x=10000.,y=0.,z=0.),schedule_unit='native physics step from 1; assign before consuming solve',provenance=PROVENANCE)
        row['required_observations']=['one copied native wheel and static measured support','exact 240 solves and one declared center-of-mass impulse','body momentum residual with boundary availability']
        row['frame_schedule']['sampling']='fixed-step contact pre-input before physics.entry; native physics.exit; original callbacks unchanged'
        rows.append(row)
    return rows

def validate_contact_schedule(row):
    expected=next((r for r in contact_rows() if r['id']==row.get('id')),None)
    if row!=expected:raise ValueError('Declared contact inputs changed; review a new row before measurement')
    return row['contact']

def dot(a,b):return sum(a[k]*b[k] for k in ZERO)
def fv(value):return {k:f32(v) for k,v in value.items()}
def fdot(a,b):
    a,b=fv(a),fv(b)
    return f32(f32(f32(a['x']*b['x'])+f32(a['y']*b['y']))+f32(a['z']*b['z']))
def vector(value):return isinstance(value,dict) and set(value)==set(ZERO) and all(type(v) in (int,float) and math.isfinite(v) for v in value.values())
def require(condition,message):
    if not condition:raise ValueError(message)

def validate_contact_body_input(run,canonical,from_source):
    report=run.get('contact',{});value=report.get('body_input');geometry=report.get('geometry',{});source=report.get('source_body',{})
    require(type(report.get('body_input_from_source')) is bool and report['body_input_from_source'] is from_source,'Contact input origin differs')
    require(isinstance(value,dict) and set(value)==set(BODY_INPUT_FIELDS),'Contact canonical body input is missing')
    if from_source:
        require(canonical is None and set(BODY_INPUT_FIELDS)<=source.keys(),'Contact first-source body provenance missing')
        require(all(vector(geometry.get(k)) for k in ('source_center_offset','source_up','source_center','source_world_com')),'Contact COM provenance missing')
        offset={k:f32(f32(geometry['source_world_com'][k])-f32(geometry['source_center'][k])) for k in ZERO}
        require(fv(geometry['source_center_offset'])==offset,'Contact source center offset differs')
        height=fdot(offset,geometry['source_up'])
        require(f32(geometry['source_signed_height'])==height,'Contact source COM height differs')
        require(f32(value['mass'])==f32(f32(source['mass'])/4.),'Contact first-source mass division differs')
        expected=dict(mass=value['mass'],center_of_mass=dict(x=0.,y=geometry['source_signed_height'],z=0.),inertia_tensor=source['inertia_tensor'],inertia_tensor_rotation=source['inertia_tensor_rotation'])
    else:
        require(canonical is not None,'Contact reused canonical body input missing')
        expected=canonical
    require(value==expected and all(run.get(n,{}).get('contact_body_input')==expected for n in ('setup_state','initial')),'Contact exact canonical body input differs')
    require(value['mass']>0 and vector(value['center_of_mass']) and vector(value['inertia_tensor']) and all(v>0 for v in value['inertia_tensor'].values()),'Contact declared mass or tensor invalid')

def contact_initial_inputs(run):
    report=run['contact'];state=deepcopy(report['initial_state']);observed=state['observed']
    # Raw source properties and origin remain provenance outside compared copied Initial inputs.
    del observed['original_body']
    for key in ('wheel_id','owner_id','collider_id','attached_body_id'):del observed[key]
    del observed['body']['instance_id']
    geometry=report['geometry']
    return dict(initial_state=state,body_input=report['body_input'],source_profile=report['source_profile'],copied_profile=report['copied_profile'],wheel_input=report['wheel_input'],geometry={k:geometry[k] for k in ('normal','forward','sideways','copied_center','support_position','support_rotation','support_scale','position_spacing')})

def validate_observation(value,report,enabled):
    require(value.get('wheel_id')==report['copied_wheel_id'] and value.get('body',{}).get('instance_id')==report['copied_body_id'],'Contact copied identity differs')
    require(value.get('original_body',{}).get('instance_id')==report['original_body_id'] and value['original_body'].get('kinematic') is True,'Contact original hold differs')
    body_fields={'instance_id','constraints','mass','center_of_mass','inertia_tensor','inertia_tensor_rotation','position','rotation','velocity','angular_velocity','gravity','use_gravity','drag','angular_drag','sleeping','sleep_threshold','solver_iterations','solver_velocity_iterations','max_angular_velocity','detect_collisions','kinematic','interpolation','collision_detection_mode'}
    require(body_fields<=value['body'].keys() and body_fields<=value['original_body'].keys(),'Contact body observation fields missing')
    require(all(vector(value.get(k)) for k in ('body_scale','body_lossy_scale','world_center_of_mass','wheel_center','wheel_up','wheel_position','wheel_scale','wheel_lossy_scale','pose_position','point','normal','forward_direction','sideways_direction','body_point_velocity','attached_point_velocity')),'Contact geometric observation fields missing')
    require(all(type(value.get(k)) in (float,int) and math.isfinite(value[k]) for k in ('rpm','sprung_mass','q','motor_nm','brake_nm','steer_degrees','force_scalar','forward_slip','sideways_slip')),'Contact scalar observation missing')
    flags=('owner_is_null','owner_available','enabled','active_self','active_hierarchy','rpm_available','sprung_mass_available','contact','hit_available','body_point_velocity_available','attached_body_available','attached_point_velocity_available')
    require(all(type(value.get(k)) is bool for k in flags),'Contact availability missing')
    require(value['enabled'] is enabled and value['active_self'] and value['active_hierarchy'],'Contact copied wheel activation differs')
    require(value['owner_available']==(value['owner_id']!=0) and value['owner_is_null'] is not value['owner_available'],'Contact raw owner availability differs')
    require(not value['owner_available'] or value['owner_id']==report['copied_body_id'],'Contact reported wrong native owner')
    require(not enabled or value['owner_available'],'Contact active copied wheel owner missing')
    require(value['rpm_available'] and value['sprung_mass_available'],'Contact native scalar availability missing')
    require(value['hit_available'] is value['contact'] and value['body_point_velocity_available'] is value['contact'],'Contact hit availability differs')
    if value['contact']:
        require(value['collider_id']==report['geometry']['support_id'],'Contact hit wrong support')
        require(not value['attached_body_available'] and value['attached_body_id']==0 and not value['attached_point_velocity_available'],'Contact static support has attached body')
    else:
        require(value['collider_id']==0 and value['attached_body_id']==0 and not value['attached_body_available'] and not value['attached_point_velocity_available'],'Contact failed hit contains available data')
    require(value['body_scale']==dict(x=1.,y=1.,z=1.) and value['body_lossy_scale']==dict(x=1.,y=1.,z=1.),'Contact copied body scale differs')
    require(value['wheel_scale']==dict(x=1.,y=1.,z=1.) and value['wheel_lossy_scale']==dict(x=1.,y=1.,z=1.),'Contact copied scale differs')
    body=value['body'];held=report['copied_held_body']
    require(all(body.get(k)==held.get(k) for k in ('mass','center_of_mass','drag','angular_drag','sleep_threshold','max_angular_velocity','solver_iterations','solver_velocity_iterations','interpolation','collision_detection_mode','detect_collisions','use_gravity','constraints','inertia_tensor_rotation')),'Contact copied body settings changed')
    require(body.get('kinematic') is False and type(body.get('sleeping')) is bool,'Contact copied body mode missing')
    require(body['constraints']==112 and body['use_gravity'] is True and body['drag']==0 and body['angular_drag']==0,'Contact force mode differs')
    require(body['angular_velocity']==ZERO and value['brake_nm']==0 and value['steer_degrees']==0,'Contact frozen rotation or wheel assignment differs')

def analyze_contact(run,scenario):
    report=run['contact'];spec=scenario['contact'];pairs={}
    for sample in run['samples']:
        if sample['phase'] in ('physics.entry','physics.exit'):
            key=(sample['step'],sample['phase']);require(key not in pairs,'Contact duplicated solve boundary');pairs[key]=sample
    require(set(pairs)=={(s,p) for s in range(1,241) for p in ('physics.entry','physics.exit')},'Contact solve pairs missing')
    output=[]
    for step in range(1,241):
        entry=pairs[step,'physics.entry'];exit=pairs[step,'physics.exit'];state=entry['contact_state'];pre=state['pre_input']
        require(pre==exit['contact_state']['pre_input'] and pre['step']==step and state['step']==step and exit['contact_state']['step']==step,'Contact pre-input linkage differs')
        require(pre['fixed_dt']==entry['fixed_dt']==exit['fixed_dt']==run['initial']['fixed_dt'] and pre['time']==entry['time'] and pre['frame']==entry['frame'],'Contact pre-input timing differs')
        require(pre['observed_step']>0 and (step==1 or pre['observed_step']==previous_observed+1),'Contact observed step linkage differs');previous_observed=pre['observed_step']
        before=pre['observed']['body'];after=exit['contact_state']['observed']['body'];start=entry['contact_state']['observed']['body']
        require(all(b['mass']==report['body_input']['mass'] and b['drag']==0 and b['constraints']==112 for b in (before,start,after)),'Contact momentum mass, drag, or free axes differ')
        impulse=report['impulse']['argument'] if step==121 else ZERO
        if step==121:
            require(report['impulse']['pre_velocity']==before['velocity'] and report['impulse']['application_point']==pre['observed']['world_center_of_mass'],'Contact impulse pre-input operands differ')
        # Decode the retained Single values exactly. Raw JSON records remain unchanged.
        mass=f32(before['mass']);dt=f32(pre['fixed_dt'])
        residual={k:mass*(f32(after['velocity'][k])-f32(before['velocity'][k]))-(mass*f32(before['gravity'][k])*dt if before['use_gravity'] else 0.)-f32(impulse[k]) for k in ZERO}
        awake=not any(b['sleeping'] or b['kinematic'] for b in (start,after))
        active=awake and spec['wheel_enabled'] and entry['contact_state']['observed']['contact'] and exit['contact_state']['observed']['contact']
        output.append(dict(step=step,residual=residual,declared_impulse=impulse,pre_sleeping=before['sleeping'],entry_sleeping=start['sleeping'],exit_sleeping=after['sleeping'],entry_kinematic=start['kinematic'],exit_kinematic=after['kinematic'],awake_dynamic_boundaries=awake,active_response_available=active,projections={k:dot(residual,report['geometry'][k]) for k in ('normal','forward','sideways')},step_average={k:dot(residual,report['geometry'][k])/dt for k in ('normal','forward','sideways')}))
    return output

def validate_contact(run,scenario):
    spec=validate_contact_schedule(scenario);report=run.get('contact',{})
    require(report.get('declared')==spec and report.get('source_key')==spec['source_wheel'],'Contact declared source input differs')
    origin=report.get('body_input_from_source')
    validate_contact_body_input(run,None if origin else run.get('setup_state',{}).get('contact_body_input'),origin)
    require(report.get('other_external_input_count')==0,'Contact undeclared external input interval')
    trace=run.get('body_trace',{})
    require(report.get('original_body_id')==trace.get('retained_instance_id') and report.get('copied_body_id')==trace.get('copied_instance_id') and report.get('copied_wheel_id') and report['copied_wheel_id']!=report.get('source_wheel_id'),'Contact source/copy identities differ')
    shapes=[s for snap in run.get('geometry',[]) if snap['phase']=='armed-after-canonical' for s in snap['colliders'] if s['key']==spec['source_wheel']]
    require(len(shapes)==1 and shapes[0]['instance_id']==report.get('source_wheel_id') and shapes[0]['sample']['enabled'] is False,'Contact source profile or isolation missing')
    require(report.get('source_index')==shapes[0].get('sample',{}).get('source_index') and report.get('road_role')==shapes[0].get('road_slot'),'Contact source index or road role differs')
    profile=report.get('source_profile');require(isinstance(profile,dict) and all(profile.get(k)==shapes[0].get(k) for k in PROFILE_FIELDS) and report.get('copied_profile')==profile,'Contact copied live profile differs')
    require(report.get('source_body',{}).get('instance_id')==report['original_body_id'] and report['source_body'].get('kinematic') is True,'Contact raw source body provenance differs')
    constructor=report.get('copied_constructor_body',{});held=report['copied_held_body']
    retained=('sleep_threshold','max_angular_velocity','solver_iterations','solver_velocity_iterations','interpolation','collision_detection_mode','detect_collisions')
    require(all(k in constructor and held.get(k)==constructor[k] for k in retained),'Contact constructor settings changed')
    require(held['kinematic'] is True and held['mass']==report['body_input']['mass'] and held['center_of_mass']==report['body_input']['center_of_mass'],'Contact held copied setter input differs')
    validate_angular_inertia(held,report['body_input']['inertia_tensor'],112)
    geometry=report['geometry'];require(geometry['support_scale']==dict(x=200.,y=1.,z=200.) and geometry['support_id'] and geometry['query_distance']==10. and geometry['selection_rule']=='nearest non-trigger static hit outside original car hierarchy','Contact support geometry differs')
    require(geometry['source_material_id']==geometry['support_material_id'],'Contact copied support material differs')
    required_vectors=('query_origin','query_direction','source_point','normal','forward','sideways','source_forward','source_up','source_center','source_world_com','source_center_offset','copied_center','copied_com_offset','copied_held_world_com','support_position','support_scale','position_spacing')
    require(all(vector(geometry.get(k)) for k in required_vectors),'Contact geometry provenance missing')
    require(geometry.get('source_collider_key') and geometry.get('source_collider_id') and geometry.get('query_mask')==-5,'Contact source support query identity missing')
    require(fv(geometry['query_direction'])=={k:-f32(geometry['source_up'][k]) for k in ZERO} and fv(geometry['query_origin'])=={k:f32(f32(geometry['source_center'][k])+f32(f32(geometry['source_up'][k])*f32(profile['radius']))) for k in ZERO},'Contact selected-wheel query operands differ')
    require(dot(held['gravity'],geometry['normal'])<0 and dot(geometry['forward'],geometry['forward'])>0 and dot(geometry['sideways'],geometry['sideways'])>0,'Contact basis or opposing gravity missing')
    center={k:f32(f32(f32(geometry['source_point'][k])+f32(spec['offset_m'][k]))+f32(f32(geometry['normal'][k])*f32(f32(profile['radius'])+f32(f32(profile['suspension_distance'])*.5)))) for k in ZERO}
    plane={k:f32(f32(f32(geometry['source_point'][k])+f32(spec['offset_m'][k]))-f32(f32(geometry['normal'][k])*.5)) for k in ZERO}
    require(fv(geometry['copied_center'])==center and fv(geometry['support_position'])==plane,'Contact declared copied geometry differs')
    require(report.get('source_sprung_mass_available') is True and report.get('copied_sprung_mass_available') is True,'Contact source/copied sprung mass availability missing')
    n0=f32(f32(report['body_input']['mass'])*abs(fdot(held['gravity'],geometry['normal'])));q0=f32(n0*f32(profile['radius']));motor=f32(f32(spec['torque_scale'])*q0)
    require(f32(report['n0'])==n0 and f32(report['q0'])==q0 and f32(report['motor_argument'])==motor,'Contact declared torque operands differ')
    expected=[dict(step=step,event_revision=i+1,motor_nm=report['motor_argument'] if i==1 else 0.,brake_nm=0.,steer_degrees=0.) for i,step in enumerate((1,121,145))]
    require(report.get('assignments')==expected,'Contact event assignment missing or duplicated')
    impulse=report.get('impulse',{});require(impulse.get('application_count')==1 and impulse.get('consuming_step')==121 and impulse.get('producer_revision')==2 and impulse.get('mode')==1 and impulse.get('mass')==report['body_input']['mass'] and impulse.get('side_speed_mps')==spec['side_speed_mps'],'Contact impulse consumption differs')
    v=impulse['pre_velocity'];target={k:f32(f32(f32(5.*f32(geometry['forward'][k]))+f32(f32(spec['side_speed_mps'])*f32(geometry['sideways'][k])))+f32(fdot(v,geometry['normal'])*f32(geometry['normal'][k]))) for k in ZERO}
    require(fv(impulse['target_velocity'])==target and fv(impulse['argument'])=={k:f32(f32(report['body_input']['mass'])*f32(target[k]-f32(v[k]))) for k in ZERO},'Contact impulse argument differs')
    initial=report['initial_state'];validate_observation(initial['observed'],report,spec['wheel_enabled'])
    require(initial['step']==0 and initial['event_revision']==0 and fv(initial['observed']['body']['velocity'])=={k:f32(5.*f32(geometry['forward'][k])) for k in ZERO} and initial['observed']['body']['position']==geometry['copied_center'] and initial['observed']['body']['rotation']==report['wheel_input']['rotation'] and initial['observed']['body']['sleeping'] is False,'Contact initial input proof differs')
    validate_angular_inertia(initial['observed']['body'],report['body_input']['inertia_tensor'],112)
    native_expected={s['key']:(s['instance_id'],s['road_slot']) for s in run['geometry'][-1]['colliders'] if s['type']=='WheelCollider'}
    require(len(run.get('wheel_ownership',[]))>=9,'Contact structural ownership checkpoints missing')
    phases=[snap['phase'] for snap in run['geometry']]
    require('prepared-held' in phases,'Contact prepared structural checkpoint missing')
    checked=run['geometry'][phases.index('prepared-held'):]
    require([c.get('phase') for c in run['wheel_ownership']]==['source-role-bound',*[snap['phase'] for snap in checked],'measurement-complete'],'Contact structural checkpoints missing or reordered')
    for snap in checked:
        native=[shape.get('sample',{}) for shape in snap['colliders'] if shape['type']=='WheelCollider']
        validate_source_ownership(sorted(native,key=lambda w:w.get('source_index',-1)),native_expected,report['original_body_id'],trace['retained_body'])
    for i,checkpoint in enumerate(run['wheel_ownership']):
        require(checkpoint.get('order')==i+1 and all(type(checkpoint.get(k)) is int for k in ('frame','physics_step')) and type(checkpoint.get('time')) in (int,float),'Contact structural checkpoint order or clocks missing')
        validate_source_ownership(checkpoint['wheels'],native_expected,report['original_body_id'],trace['retained_body'])
    for sample in run['samples']:
        validate_source_ownership(sample.get('native_wheels',[]),native_expected,report['original_body_id'],trace['retained_body'])
        if sample['phase'] not in ('physics.entry','physics.exit'):continue
        state=sample.get('contact_state',{});step=sample['step'];revision=1 if step<=120 else 2 if step<=144 else 3
        require(len(sample.get('wheels',[]))==1,'Contact copied wheel count differs')
        require(all(state.get(k)==profile[k] for k in PROFILE_FIELDS),'Contact measured profile changed')
        require(state.get('event_revision')==revision and state.get('assignment_step')==step and state['pre_input']['input_revision']==(0 if step==1 else 1 if step<=121 else 2 if step<=145 else 3),'Contact event revision differs')
        value=state['observed'];validate_observation(value,report,spec['wheel_enabled'])
        require(value['motor_nm']==(report['motor_argument'] if 121<=step<=144 else 0),'Contact motor assignment differs')
        validate_angular_inertia(value['body'],report['body_input']['inertia_tensor'],112,step,sample['phase'])
        pre=state['pre_input'];require(pre.get('native_wheel_count')==5,'Contact pre-input source cohort missing')
        validate_source_ownership([pre.get('native_wheel'+str(i),{}) for i in range(5)],native_expected,report['original_body_id'],trace['retained_body'])
        validate_observation(state['pre_input']['observed'],report,spec['wheel_enabled'])
        pre_body=state['pre_input']['observed']['body']
        require(not pre_body['kinematic'],'Contact pre-input body is held')
        validate_angular_inertia(pre_body,report['body_input']['inertia_tensor'],112,step,'physics.entry')
    residuals=analyze_contact(run,scenario)
    return dict(steps=240,scope='single copied wheel; static support; FreezeRotation; body momentum residual',active_intervals=sum(r['active_response_available'] for r in residuals),disabled_control=not spec['wheel_enabled'],native_contact_impulse_available=False)

def contact_summary(run,residuals):
    values=[s['contact_state']['observed'] for s in run['samples'] if s['phase'] in ('physics.entry','physics.exit')]
    def bounds(items):return dict(min=min(items),max=max(items)) if items else None
    return dict(numeric_precision='raw Unity JSON remains retained; formula operands decode exactly to binary32, then offline momentum arithmetic uses binary64',contact_boundaries=sum(v['contact'] for v in values),nonzero_residual_intervals=sum(any(x!=0 for x in r['residual'].values()) for r in residuals),rpm=bounds([v['rpm'] for v in values]),projected_offset_q=bounds([v['q'] for v in values]),speed=bounds([math.sqrt(dot(v['body']['velocity'],v['body']['velocity'])) for v in values]),native_sprung_mass=bounds([v['sprung_mass'] for v in values if v['sprung_mass_available']]),hit_force_scalar=bounds([v['force_scalar'] for v in values if v['contact']]),forward_slip=bounds([v['forward_slip'] for v in values if v['contact']]),sideways_slip=bounds([v['sideways_slip'] for v in values if v['contact']]),overlap_limit='compare these observed ranges across independently qualified rows; absent overlap supplies no combined-slip identification')

def main():
    import argparse
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--runtime',required=True,type=Path);p.add_argument('--request',required=True,type=Path);p.add_argument('--output',required=True,type=Path);args=p.parse_args()
    run=json.loads(args.runtime.read_text(encoding='utf-8-sig'));request=json.loads(args.request.read_text(encoding='utf-8-sig'));declared=request.get('scenarios')
    require(isinstance(declared,list) and declared and all(isinstance(r,dict) and isinstance(r.get('id'),str) and r['id'] for r in declared),'Contact analysis request row identities missing')
    require(len({r['id'] for r in declared})==len(declared),'Contact analysis request has duplicate row identities')
    rows=[r for r in declared if r['id']==run.get('scenario')]
    require(len(rows)==1,'Contact analysis request has no unique declared row');row=rows[0]
    result=dict(input_sha256={str(path.resolve()):hashlib.sha256(path.read_bytes()).hexdigest() for path in (args.runtime,args.request)},validation=validate_contact(run,row),residuals=analyze_contact(run,row),interpretation='body momentum residual, not native contact impulse; boundary observations do not prove interior state')
    result['summary']=contact_summary(run,result['residuals'])
    args.output.write_text(json.dumps(result,indent=2,allow_nan=False)+'\n')
if __name__=='__main__':main()
