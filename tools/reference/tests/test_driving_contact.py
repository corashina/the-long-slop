"""Critical adapter checks. All fabricated records and stub physics remain unqualified."""
from copy import deepcopy
from pathlib import Path
import json
import subprocess
import tempfile
import unittest
from unittest.mock import patch
from tools.reference.driving_contact import *
from tools.reference.tests.test_driving_angular import gate_fixture


def fixture(index=0):
    row=contact_rows()[index];_,run=gate_fixture();angular=run.pop('angular');spec=row['contact'];z=deepcopy(ZERO);n=dict(x=0.,y=1.,z=0.);f=dict(x=0.,y=0.,z=1.);s=dict(x=1.,y=0.,z=0.);one=dict(x=1.,y=1.,z=1.);q=dict(x=0.,y=0.,z=0.,w=1.)
    source=deepcopy(angular['source_body']);source.update(mass=400.,center_of_mass=dict(x=0.,y=2.,z=0.),kinematic=True)
    body=deepcopy(angular['copied_held_body']);body.update(position=dict(x=10000.,y=f32(.5+f32(.3*.5)),z=0.),constraints=112,center_of_mass=dict(x=0.,y=2.,z=0.),use_gravity=True,gravity=dict(x=0.,y=-10.,z=0.),sleep_threshold=.005,velocity=dict(x=0.,y=0.,z=5.),kinematic=False)
    declared={k:deepcopy(body[k]) for k in BODY_INPUT_FIELDS}
    geometry=dict(normal=n,forward=f,sideways=s,source_center_offset=dict(x=0.,y=2.,z=0.),source_up=n,source_signed_height=2.,source_center=z,source_world_com=dict(x=0.,y=2.,z=0.),copied_center=deepcopy(body['position']),copied_com_offset=dict(x=0.,y=2.,z=0.),copied_held_world_com=dict(x=10000.,y=f32(body['position']['y']+2.),z=0.),query_origin=dict(x=0.,y=.5,z=0.),query_direction=dict(x=0.,y=-1.,z=0.),source_point=z,source_forward=f,source_collider_key='support-source',source_collider_id=98,query_mask=-5,support_position=dict(x=10000.,y=-.5,z=0.),support_rotation=q,support_scale=dict(x=200.,y=1.,z=200.),position_spacing=z,support_id=99,source_material_id=0,support_material_id=0,query_distance=10.,selection_rule='nearest non-trigger static hit outside original car hierarchy')
    obs=dict(body_scale=one,body_lossy_scale=one,body=body,original_body=source,world_center_of_mass=deepcopy(geometry['copied_held_world_com']),wheel_center=deepcopy(body['position']),wheel_up=n,wheel_position=deepcopy(body['position']),wheel_rotation=q,pose_position=deepcopy(body['position']),pose_rotation=q,wheel_id=50,owner_id=2,owner_is_null=False,owner_available=True,enabled=spec['wheel_enabled'],active_self=True,active_hierarchy=True,rpm_available=True,sprung_mass_available=True,contact=spec['wheel_enabled'],hit_available=spec['wheel_enabled'],body_point_velocity_available=spec['wheel_enabled'],attached_body_available=False,attached_point_velocity_available=False,collider_id=99 if spec['wheel_enabled'] else 0,attached_body_id=0,wheel_scale=one,wheel_lossy_scale=one,rpm=0.,sprung_mass=100.,q=0.,motor_nm=0.,brake_nm=0.,steer_degrees=0.,point=z,normal=n,forward_direction=f,sideways_direction=s,body_point_velocity=z,attached_point_velocity=z,force_scalar=0.,forward_slip=0.,sideways_slip=0.)
    profile=angular['source_profile'];profile['radius']=.5
    for snap in run['geometry']:
        for shape in snap['colliders']:shape['radius']=.5
    report=dict(other_external_input_count=0,declared=deepcopy(spec),source_key=spec['source_wheel'],source_index=0,road_role=0,source_wheel_id=10,copied_wheel_id=50,original_body_id=1,copied_body_id=2,source_body=source,body_input=declared,body_input_from_source=True,source_profile=profile,copied_profile=deepcopy(profile),copied_constructor_body=deepcopy(body),copied_held_body=dict(deepcopy(body),kinematic=True),geometry=geometry,source_sprung_mass_available=True,copied_sprung_mass_available=True,source_sprung_mass=100.,copied_sprung_mass=100.,n0=1000.,q0=500.,motor_argument=spec['torque_scale']*500.,wheel_input=dict(position=deepcopy(body['position']),rotation=q,local_scale=one))
    motor=report['motor_argument'];report['assignments']=[dict(step=step,event_revision=i+1,motor_nm=motor if i==1 else 0.,brake_nm=0.,steer_degrees=0.) for i,step in enumerate((1,121,145))]
    report['impulse']=dict(producer_revision=2,consuming_step=121,application_count=1,mode=1,mass=100.,side_speed_mps=spec['side_speed_mps'],pre_velocity=dict(x=0.,y=0.,z=5.),target_velocity=dict(x=spec['side_speed_mps'],y=0.,z=5.),argument=dict(x=100.*spec['side_speed_mps'],y=0.,z=0.),application_point=deepcopy(obs['world_center_of_mass']))
    report['initial_state']=dict(deepcopy(profile),step=0,event_revision=0,assignment_step=0,observed=deepcopy(obs))
    samples=[]
    for step in range(1,241):
        revision=1 if step<=120 else 2 if step<=144 else 3
        if step>1:obs['body']['inertia_tensor']=deepcopy(z)
        pre=dict(step=step,observed_step=step+10,frame=step,time=step*.02,fixed_dt=.02,input_revision=0 if step==1 else 1 if step<=121 else 2 if step<=145 else 3,observed=deepcopy(obs))
        pre['native_wheel_count']=5
        pre.update({'native_wheel'+str(i):deepcopy(w) for i,w in enumerate(run['samples'][0]['native_wheels'])})
        for phase in ('physics.entry','physics.exit'):
            state=dict(deepcopy(profile),step=step,event_revision=revision,assignment_step=step,pre_input=deepcopy(pre),observed=deepcopy(obs));state['observed']['motor_nm']=motor if 121<=step<=144 else 0.
            if step>1 or phase=='physics.exit':state['observed']['body']['inertia_tensor']=deepcopy(z)
            samples.append(dict(step=step,phase=phase,frame=step,time=step*.02,fixed_dt=.02,contact_state=state,wheels=[{}],native_wheels=deepcopy(run['samples'][0]['native_wheels'])))
    run.update(contact=report,samples=samples,initial=dict(fixed_dt=.02,contact_body_input=deepcopy(declared)),setup_state=dict(contact_body_input=deepcopy(declared)),physics_steps=240,probe_kind='single_wheel_contact')
    return row,run


class ContactTests(unittest.TestCase):
    def test_five_exact_rows(self):
        rows=contact_rows();self.assertEqual(len(rows),5)
        self.assertEqual(json.loads((Path(__file__).parents[3]/'tests/fixtures/driving/contact-probe-scenarios.json').read_text())['scenarios'],rows)
        for row in rows:validate_contact_schedule(row)
        for key,value in [('duration_steps',241),('side_speed_mps',1.),('event_steps',[1,121,146])]:
            bad=deepcopy(rows[0]);bad['contact'][key]=value
            with self.assertRaises(ValueError):validate_contact_schedule(bad)

    def test_actual_record_gates_and_disabled_control(self):
        for i in range(5):
            row,run=fixture(i);validate_contact_body_input(run,None,True);self.assertEqual(validate_contact(run,row)['steps'],240)
            if i==4:self.assertFalse(any(x['active_response_available'] for x in analyze_contact(run,row)))
        row,run=fixture(3)
        for case in ('support','owner','profile','availability','count','missing_pair','pre_link','impulse','origin','canonical','tensor','source_owner','revision','external','query','geometry','checkpoint'):
            bad=deepcopy(run);sample=bad['samples'][241]
            if case=='support':sample['contact_state']['observed']['collider_id']=98
            if case=='owner':sample['contact_state']['observed']['owner_id']=98
            if case=='profile':sample['contact_state']['radius']=.6
            if case=='availability':del sample['contact_state']['observed']['rpm_available']
            if case=='count':sample['wheels'].append({})
            if case=='missing_pair':bad['samples'].pop()
            if case=='pre_link':sample['contact_state']['pre_input']['step']=122
            if case=='impulse':bad['contact']['impulse']['application_count']=2
            if case=='origin':bad['contact']['body_input_from_source']=False
            if case=='canonical':bad['initial']['contact_body_input']['mass']+=1
            if case=='tensor':sample['contact_state']['observed']['body']['inertia_tensor']['x']=1.
            if case=='source_owner':sample['native_wheels'][0]['owner_instance_id']=50
            if case=='external':bad['contact']['other_external_input_count']=1
            if case=='query':bad['contact']['geometry']['query_origin']['x']=1
            if case=='geometry':bad['contact']['geometry']['support_position']['x']=0
            if case=='checkpoint':bad['wheel_ownership'].pop()
            if case=='revision':sample['contact_state']['event_revision']=3
            with self.subTest(case=case),self.assertRaises(ValueError):
                validate_contact_body_input(bad,None,True);validate_contact(bad,row)

    def test_exact_input_origin_and_later_source_variation(self):
        row,run=fixture();canonical=deepcopy(run['contact']['body_input']);validate_contact_body_input(run,None,True)
        run['contact']['body_input_from_source']=False;run['contact']['source_body']['mass']=999.;validate_contact_body_input(run,canonical,False)
        for record in (run['contact']['body_input'],run['initial']['contact_body_input'],run['setup_state']['contact_body_input']):record['mass']+=1
        with self.assertRaises(ValueError):validate_contact_body_input(run,canonical,False)
        row,run=fixture();run['contact']['geometry']['source_signed_height']=3.
        with self.assertRaises(ValueError):validate_contact_body_input(run,None,True)

    def test_non_axis_projections_and_speed_decode_single_operands(self):
        import struct
        row,run=fixture()
        axes=('x','y','z')
        def decode(value):return struct.unpack('<f',struct.pack('<f',value))[0]
        run['contact']['geometry']['forward']=dict(x=.6,y=0.,z=.8)
        run['contact']['geometry']['sideways']=dict(x=.8,y=0.,z=-.6)
        for sample in run['samples']:sample['contact_state']['observed']['body']['velocity']=dict(x=.6,y=0.,z=.8)
        run['samples'][1]['contact_state']['observed']['body']['velocity']=dict(x=.01,y=.02,z=.03)
        original=deepcopy(run);rows=analyze_contact(run,row);first=rows[0]
        pre=run['samples'][0]['contact_state']['pre_input'];before=pre['observed']['body'];after=run['samples'][1]['contact_state']['observed']['body']
        mass=decode(before['mass']);dt=decode(pre['fixed_dt'])
        expected_residual={axis:mass*(decode(after['velocity'][axis])-decode(before['velocity'][axis]))-mass*decode(before['gravity'][axis])*dt for axis in axes}
        self.assertEqual(first['residual'],expected_residual)
        self.assertTrue(any(value!=0 for value in expected_residual.values()))
        expected={name:sum(expected_residual[axis]*decode(run['contact']['geometry'][name][axis]) for axis in axes) for name in ('normal','forward','sideways')}
        raw_forward=sum(expected_residual[axis]*run['contact']['geometry']['forward'][axis] for axis in axes)
        self.assertNotEqual(raw_forward,expected['forward'])
        self.assertEqual(first['projections'],expected)
        self.assertEqual(first['step_average'],{name:value/dt for name,value in expected.items()})
        speeds=[math.sqrt(sum(decode(sample['contact_state']['observed']['body']['velocity'][axis])**2 for axis in axes)) for sample in run['samples']]
        self.assertNotEqual(max(speeds),math.sqrt(.6**2+.8**2))
        self.assertEqual(contact_summary(run,rows)['speed'],dict(min=min(speeds),max=max(speeds)))
        self.assertEqual(run,original)

    def test_momentum_gravity_impulse_and_boundary_sleep(self):
        row,run=fixture(3);i=240
        run['initial']['fixed_dt']=.125
        for sample in run['samples']:
            sample['fixed_dt']=.125;sample['time']=sample['step']*.125;pre=sample['contact_state']['pre_input'];pre['fixed_dt']=.125;pre['time']=sample['time']
            for body in (pre['observed']['body'],sample['contact_state']['observed']['body']):body['gravity']=dict(x=0.,y=-8.,z=0.)
        pre=run['samples'][i]['contact_state']['pre_input'];pre['observed']['body']['sleeping']=True
        run['samples'][i+1]['contact_state']['pre_input']=deepcopy(pre)
        run['samples'][i+1]['contact_state']['observed']['body']['velocity']=dict(x=.5,y=-1.,z=5.)
        value=analyze_contact(run,row)[120];self.assertEqual(value['residual'],ZERO);self.assertTrue(value['active_response_available']);self.assertTrue(value['pre_sleeping'])
        for boundary in (i,i+1):
            bad=deepcopy(run);bad['samples'][boundary]['contact_state']['observed']['body']['sleeping']=True
            self.assertFalse(analyze_contact(bad,row)[120]['active_response_available'])
            bad=deepcopy(run);bad['samples'][boundary]['contact_state']['observed']['body']['kinematic']=True
            self.assertFalse(analyze_contact(bad,row)[120]['active_response_available'])
        run['samples'][i]['contact_state']['pre_input']['observed']['body']['constraints']=116
        with self.assertRaises(ValueError):analyze_contact(run,row)

    def test_initial_exact_pose_and_origin_separation(self):
        _,run=fixture();first=contact_initial_inputs(run);run['contact']['body_input_from_source']=False
        self.assertEqual(contact_initial_inputs(run),first)
        self.assertNotIn('contact_body_input_from_source',run['initial'])
        run['contact']['initial_state']['observed']['body']['rotation']['w']=.99
        self.assertNotEqual(contact_initial_inputs(run),first)

    def test_actual_capture_and_verify_group_transport(self):
        from tools.reference import driving_probe as probe
        rows=contact_rows();base=fixture()[1];canonical=deepcopy(base['contact']['body_input'])
        for mode in ('capture','verify'):
            for corruption in (None,'first_origin','later_origin','missing','replacement','repeat_replacement','physical_initial'):
                with self.subTest(mode=mode,corruption=corruption),tempfile.TemporaryDirectory() as name:
                    project=Path(name).resolve();output=project/'.cache/g11-driving/proof';definitions=project/'rows.json';definitions.write_text(json.dumps(dict(scenarios=rows)));launched=[]
                    def make_run(index,repeat):
                        run=deepcopy(base);run['row_only']=index;run['initial']['row_only']=index;run['setup_state']['row_only']=index
                        first=index==0 and repeat==1;run['contact']['body_input_from_source']=first
                        if not first:run['contact']['source_body']['mass']=999.+index+repeat
                        bad=index==1 and repeat==1
                        if corruption=='first_origin' and first:run['contact']['body_input_from_source']=False
                        if corruption=='later_origin' and bad:run['contact']['body_input_from_source']=True
                        if corruption=='missing' and bad:del run['initial']['contact_body_input']
                        if corruption=='physical_initial' and index==0 and repeat==2:run['initial']['velocity']=dict(x=1.,y=0.,z=0.)
                        if (corruption=='replacement' and bad) or (corruption=='repeat_replacement' and index==0 and repeat==2):
                            for rec in (run['contact']['body_input'],run['initial']['contact_body_input'],run['setup_state']['contact_body_input']):rec['mass']+=1
                        return run
                    def fake_player(args,**kwargs):
                        if args[args.index('-Mode')+1]=='Run':
                            d=Path(args[args.index('-RunRoot')+1]);supplied=json.loads((d/'scenario.json').read_text());index=len(launched)//3;repeat=len(launched)%3+1;launched.append(supplied)
                            self.assertIs(supplied['contact_body_input_from_source'],index==0 and repeat==1)
                            self.assertEqual(supplied['contact_body_input'],None if index==0 and repeat==1 else canonical)
                            self.assertEqual(supplied.get('has_setup_state',False),repeat>1)
                            if repeat>1:self.assertEqual(supplied['setup_state']['row_only'],index)
                            (d/'runtime.json').write_text(json.dumps(make_run(index,repeat)))
                        return type('Result',(),dict(returncode=0))()
                    # Unrelated provenance, geometry, and full per-run gates have their own existing critical suites.
                    with patch.object(probe,'PROJECT',project),patch.object(probe,'physical_inputs',return_value={}),patch.object(probe,'validate_evidence',return_value={}):
                        if mode=='capture':
                            with patch.object(probe,'preflight',return_value=dict(powershell='pwsh',observer_revision={})),patch.object(probe.subprocess,'run',side_effect=fake_player),patch.object(probe,'verify',return_value={'runs':list(range(15))}):
                                if corruption:
                                    with self.assertRaises(ValueError):probe.capture(project,project,output,definitions,3)
                                else:self.assertEqual(len(probe.capture(project,project,output,definitions,3)['runs']),15)
                        else:
                            output.mkdir(parents=True);(output/'scratch').mkdir();(output/'request.json').write_text(json.dumps(dict(repeats=3,scenarios=rows,observer_revision={})))
                            (output/'preflight.json').write_text(json.dumps(dict(app_id=probe.APP_ID,build_id=probe.BUILD_ID,unity_version=probe.UNITY_VERSION,native_engine=dict(sha256='engine'),dependencies={'TheLongDrive_Data/Managed/UnityEngine.PhysicsModule.dll':'physics'})))
                            (output/'scratch/patch-receipt.json').write_text(json.dumps(dict(changed_methods=['checked'],observer_revision={},native_engine_sha256='engine',source_physics_module_sha256='physics')))
                            for index,row in enumerate(rows):
                                for repeat in range(1,4):
                                    d=output/row['id']/f'repeat-{repeat}';d.mkdir(parents=True);first=index==0 and repeat==1
                                    (d/'runtime.json').write_text(json.dumps(make_run(index,repeat)))
                                    (d/'scenario.json').write_text(json.dumps(dict(contact_body_input_from_source=first,contact_body_input=None if first else canonical)))
                            if corruption:
                                with self.assertRaises(ValueError):probe.verify(output)
                            else:self.assertEqual(len(probe.verify(output)['runs']),15)

    def test_exact_single_round_trip_and_one_bit_rejection(self):
        import struct
        row,run=fixture(3)
        run['contact']['source_body']['mass']=400.000001
        run['contact']['impulse']['argument']['x']=50.0000001
        run=json.loads(json.dumps(run));saved=deepcopy(run)
        self.assertEqual(f32(run['contact']['source_body']['mass']),400.)
        self.assertEqual(f32(run['contact']['impulse']['argument']['x']),50.)
        validate_contact_body_input(run,None,True);validate_contact(run,row);self.assertEqual(run,saved)
        changed=deepcopy(run);bits=struct.unpack('<I',struct.pack('<f',50.))[0]
        changed['contact']['impulse']['argument']['x']=struct.unpack('<f',struct.pack('<I',bits+1))[0]
        with self.assertRaises(ValueError):validate_contact(changed,row)
        canonical=deepcopy(run['contact']['body_input']);run['contact']['body_input_from_source']=False
        for target in (run['contact']['body_input'],run['initial']['contact_body_input'],run['setup_state']['contact_body_input']):target['mass']=100.000001
        self.assertEqual(f32(run['contact']['body_input']['mass']),f32(canonical['mass']))
        with self.assertRaises(ValueError):validate_contact_body_input(run,canonical,False)

    def test_actual_noncontact_canonical_owner_stays_per_row(self):
        from tools.reference import driving_probe as probe
        pairs=[gate_fixture(i) for i in (0,2)]
        for index,(_,run) in enumerate(pairs):
            for target in (run['angular']['body_input'],run['setup_state']['angular_body_input'],run['initial']['angular_body_input'],run['angular']['source_body']):target['mass']=100.+index
        for mode in ('capture','verify'):
            with self.subTest(mode=mode),tempfile.TemporaryDirectory() as name:
                project=Path(name).resolve();output=project/'.cache/g11-driving/proof';definitions=project/'rows.json';rows=[p[0] for p in pairs];definitions.write_text(json.dumps(dict(scenarios=rows)));launched=[]
                def make_run(index,repeat):
                    run=deepcopy(pairs[index][1]);run['angular']['body_input_from_source']=repeat==1
                    if repeat>1:run['angular']['source_body']['mass']+=5.
                    return run
                def fake_player(args,**kwargs):
                    if args[args.index('-Mode')+1]=='Run':
                        d=Path(args[args.index('-RunRoot')+1]);supplied=json.loads((d/'scenario.json').read_text());index=len(launched)//3;repeat=len(launched)%3+1;launched.append(supplied)
                        self.assertNotIn('contact_body_input',supplied);self.assertEqual(supplied.get('has_setup_state',False),repeat>1)
                        if repeat>1:self.assertEqual(supplied['setup_state']['angular_body_input']['mass'],100.+index)
                        (d/'runtime.json').write_text(json.dumps(make_run(index,repeat)))
                    return type('Result',(),dict(returncode=0))()
                with patch.object(probe,'PROJECT',project),patch.object(probe,'physical_inputs',return_value={}),patch.object(probe,'validate_evidence',return_value={}):
                    if mode=='capture':
                        with patch.object(probe,'preflight',return_value=dict(powershell='pwsh',observer_revision={})),patch.object(probe.subprocess,'run',side_effect=fake_player),patch.object(probe,'verify',return_value=dict(runs=list(range(6)))):
                            self.assertEqual(len(probe.capture(project,project,output,definitions,3)['runs']),6)
                    else:
                        output.mkdir(parents=True);(output/'scratch').mkdir();(output/'request.json').write_text(json.dumps(dict(repeats=3,scenarios=rows,observer_revision={})))
                        (output/'preflight.json').write_text(json.dumps(dict(app_id=probe.APP_ID,build_id=probe.BUILD_ID,unity_version=probe.UNITY_VERSION,native_engine=dict(sha256='engine'),dependencies={'TheLongDrive_Data/Managed/UnityEngine.PhysicsModule.dll':'physics'})))
                        (output/'scratch/patch-receipt.json').write_text(json.dumps(dict(changed_methods=['checked'],observer_revision={},native_engine_sha256='engine',source_physics_module_sha256='physics')))
                        for index,row in enumerate(rows):
                            for repeat in range(1,4):
                                d=output/row['id']/f'repeat-{repeat}';d.mkdir(parents=True);(d/'runtime.json').write_text(json.dumps(make_run(index,repeat)))
                        self.assertEqual(len(probe.verify(output)['runs']),6)

    def test_actual_csharp_methods_and_capture_owners(self):
        reference=Path(__file__).parents[1];capture=(reference/'driving_capture.cs').read_text(encoding='utf-8-sig')
        held='static Rigidbody CreateHeldProbeBody'+capture.split('static Rigidbody CreateHeldProbeBody',1)[1].split('static void CreateContactProbe',1)[0]
        canonical=capture.split('static void ApplyCanonical(Initial initial)',1)[1].split('car.Engine.rpm=',1)[0].strip()[1:]
        load=capture.split('if(scenario.has_setup_state) inputPose=scenario.setup_state.input_pose;',1)[1].split('if(scenario.setup.surface==',1)[0].split('{',1)[1]
        activation=capture.split('                ApplyCanonical(result.setup_state);',1)[1].split('                result.initial=InitialState();',1)[0]
        source=(reference/'driving_angular.cs').read_text(encoding='utf-8-sig')+(reference/'driving_contact.cs').read_text(encoding='utf-8-sig').replace('using System;','').replace('using System.Linq;','').replace('using UnityEngine;','')+CONTACT_STUBS+CONTACT_CHECKS.replace('HELD_METHOD',held).replace('CANONICAL_METHOD',canonical).replace('LOAD_METHOD',load).replace('ACTIVATION_METHOD',activation)
        with tempfile.TemporaryDirectory() as name:
            source_path=Path(name)/'contact.cs';script_path=Path(name)/'check.ps1'
            source_path.write_text(source)
            script_path.write_text("param($SourcePath)\n$ErrorActionPreference='Stop'\nAdd-Type -Path $SourcePath\n[ContactChecks]::Run()\n")
            result=subprocess.run(['pwsh','-NoProfile','-File',str(script_path),str(source_path)],capture_output=True,text=True,timeout=60)
        self.assertEqual(result.returncode,0,result.stdout+result.stderr);self.assertIn('G11_CONTACT_METHODS_PASS',result.stdout)

CONTACT_STUBS=r'''
namespace UnityEngine {
 public struct Vector3 {public float x,y,z;public Vector3(float a,float b,float c){x=a;y=b;z=c;}public static Vector3 zero {get{return new Vector3();}}public static Vector3 one {get{return new Vector3(1,1,1);}}public static Vector3 up {get{return new Vector3(0,1,0);}} public float sqrMagnitude {get{return x*x+y*y+z*z;}}public Vector3 normalized {get{return this*(1f/(float)Math.Sqrt(sqrMagnitude));}}
 public static float Dot(Vector3 a,Vector3 b){return a.x*b.x+a.y*b.y+a.z*b.z;}public static Vector3 Cross(Vector3 a,Vector3 b){return new Vector3(a.y*b.z-a.z*b.y,a.z*b.x-a.x*b.z,a.x*b.y-a.y*b.x);}
 public static Vector3 operator+(Vector3 a,Vector3 b){return new Vector3(a.x+b.x,a.y+b.y,a.z+b.z);}public static Vector3 operator-(Vector3 a,Vector3 b){return new Vector3(a.x-b.x,a.y-b.y,a.z-b.z);}public static Vector3 operator-(Vector3 a){return new Vector3(-a.x,-a.y,-a.z);}public static Vector3 operator*(Vector3 a,float b){return new Vector3(a.x*b,a.y*b,a.z*b);}public static Vector3 operator*(float b,Vector3 a){return a*b;}}
 public struct Quaternion {public float x,y,z,w;public static Quaternion FromToRotation(Vector3 a,Vector3 b){return new Quaternion{w=1};}public static Quaternion LookRotation(Vector3 f,Vector3 n){return new Quaternion{w=1};}}
 public struct WheelFrictionCurve {public float extremumSlip,extremumValue,asymptoteSlip,asymptoteValue,stiffness;}
 public struct JointSpring {public float spring,damper,targetPosition;}
 public enum RigidbodyConstraints {None=0,FreezeRotation=112}public enum PrimitiveType {Cube}public enum QueryTriggerInteraction {Ignore}public enum ForceMode {Impulse=1}
 public class Material {public int GetInstanceID(){return 70;}}
 public class Transform {public Transform parent;public Vector3 position,localPosition,localScale=Vector3.one,lossyScale=Vector3.one;public Quaternion rotation,localRotation;
 public Vector3 up {get{return Vector3.up;}}public Vector3 forward {get{return new Vector3(0,0,1);}}public bool IsChildOf(Transform t){return this==t || parent==t;}public Vector3 TransformPoint(Vector3 p){return position+p;}public Vector3 TransformVector(Vector3 p){return p;}}
 public class GameObject {public string name;public Transform transform=new Transform();public bool activeSelf=true,activeInHierarchy=true;public static int ordinaryColliders;public GameObject(string n){}public static GameObject CreatePrimitive(PrimitiveType p){ordinaryColliders++;return new GameObject("support");}
 public T AddComponent<T>() where T:new(){object v=new T();if(v is WheelCollider){var w=(WheelCollider)v;w.gameObject=this;w.attachedRigidbody=Rigidbody.latest;WheelCollider.created++;}if(v is Rigidbody){var b=(Rigidbody)v;b.transform=transform;Rigidbody.latest=b;}return (T)v;}public T GetComponent<T>() where T:new(){return new T();}}
 public class Rigidbody {public static Rigidbody latest;public Transform transform=new Transform();public int id=2,solverIterations=6,solverVelocityIterations=1,interpolation,collisionDetectionMode,writes,angularWrites,wakes,impulses,massWrites,comWrites,tensorWrites,rotationWrites;public bool sleeping,useGravity,isKinematic,detectCollisions=true;
 float m=100;public float mass {get{return m;}set{massWrites++;m=value;}}public float drag,angularDrag,sleepThreshold=.005f,maxAngularVelocity=7;public RigidbodyConstraints constraints;
 Vector3 com,tensor;Quaternion it;public Vector3 centerOfMass {get{return com;}set{comWrites++;com=value;}}public Vector3 inertiaTensor {get{return tensor;}set{tensorWrites++;tensor=value;}}public Quaternion inertiaTensorRotation {get{return it;}set{rotationWrites++;it=value;it.w=1;}}
 public Vector3 position {get{return transform.position;}set{transform.position=value;}}public Quaternion rotation {get{return transform.rotation;}set{transform.rotation=value;}}public Vector3 worldCenterOfMass {get{return position+centerOfMass;}}
 Vector3 speed,spin;public Vector3 velocity {get{return speed;}set{writes++;speed=value;}}public Vector3 angularVelocity {get{return spin;}set{angularWrites++;spin=value;}}public bool IsSleeping(){return sleeping;}public int GetInstanceID(){return id;}public void WakeUp(){wakes++;sleeping=false;}public Vector3 GetPointVelocity(Vector3 p){return velocity;}public void AddForce(Vector3 j,ForceMode m){impulses++;sleeping=false;}}
 public class Collider {public Transform transform=new Transform();public bool isTrigger;public Rigidbody attachedRigidbody;public Material sharedMaterial;public int GetInstanceID(){return 99;}}
 public class WheelCollider {public static int created;public GameObject gameObject=new GameObject("wheel");public Transform transform {get{return gameObject.transform;}}public Rigidbody attachedRigidbody;public bool enabled=true;
 public Vector3 center=new Vector3(0,-.1f,0);public float radius=.5f,mass=20,wheelDampingRate=.25f,suspensionDistance=.3f,forceAppPointDistance,sprungMass=123,rpm=-7,steerAngle;public WheelFrictionCurve forwardFriction,sidewaysFriction;public JointSpring suspensionSpring;public float motorTorque,brakeTorque;public int GetInstanceID(){return 50;}public void GetWorldPose(out Vector3 p,out Quaternion q){p=transform.position;q=transform.rotation;}public bool GetGroundHit(out WheelHit h){h=new WheelHit();return false;}}
 public struct WheelHit {public Collider collider;public Vector3 point,normal,forwardDir,sidewaysDir;public float forwardSlip,sidewaysSlip,force;}
 public struct RaycastHit {public Collider collider;public Vector3 point,normal;public float distance;}
 public static class Object {public static T[] FindObjectsOfType<T>(){return new T[0];}}
 public static class Physics {public static int syncs;public static void SyncTransforms(){syncs++;}public static Vector3 gravity=new Vector3(0,-10,0);public static int DefaultRaycastLayers=-5;public static RaycastHit[] RaycastAll(Vector3 o,Vector3 d,float l,int m,QueryTriggerInteraction q){return new[]{new RaycastHit{collider=new Collider(),normal=Vector3.up,point=Vector3.zero,distance=1}};}}
}
public static class G11Geometry {public static UnityEngine.WheelCollider[] nativeWheels=new UnityEngine.WheelCollider[0];public struct NativeWheel {public int road_slot,read_revision;}static int reads;public static NativeWheel ReadWheel(UnityEngine.WheelCollider w){return new NativeWheel{read_revision=++reads};}public static void RestoreWheels(){throw new Exception("unexpected source wheel restore");}public static string Key(UnityEngine.WheelCollider w){return "level1:58684";}public static string Key(UnityEngine.Collider c){return "support";}}
public static class G11BodyTrace {public static int holds;public static void HoldCopy(UnityEngine.Rigidbody b){b.isKinematic=true;holds++;}}
'''

CONTACT_CHECKS=r'''
public static class ContactChecks {
 static UnityEngine.Rigidbody body;static UnityEngine.WheelCollider[] wheels;class Result {public bool isolated_wheel_probe=true;}static Result result=new Result();static Dictionary<int,string> surfacePaths=new Dictionary<int,string>();static string PathOf(UnityEngine.Transform t){return "";}static void RecordSetupBoundary(string phase){}static void RunActivation(){ACTIVATION_METHOD}
 class Initial {public float mass=999;public UnityEngine.Vector3 center_of_mass=new UnityEngine.Vector3();}
 class Setup {public float load_multiplier=1;}class Scenario {public bool has_setup_state;public Setup setup=new Setup();}static Scenario scenario=new Scenario();
 HELD_METHOD
 static void ApplyCanonical(Initial initial){CANONICAL_METHOD}
 static void ApplyLoad(){if(!scenario.has_setup_state){LOAD_METHOD}}
 static void Need(bool b,string m){if(!b)throw new Exception(m);}static void Reset(){typeof(G11ContactProbe).GetField("input",System.Reflection.BindingFlags.Static|System.Reflection.BindingFlags.NonPublic).SetValue(null,null);typeof(G11ContactProbe).GetField("armed",System.Reflection.BindingFlags.Static|System.Reflection.BindingFlags.NonPublic).SetValue(null,false);typeof(G11ContactProbe).GetField("assignedStep",System.Reflection.BindingFlags.Static|System.Reflection.BindingFlags.NonPublic).SetValue(null,0);typeof(G11ContactProbe).GetField("revision",System.Reflection.BindingFlags.Static|System.Reflection.BindingFlags.NonPublic).SetValue(null,0);}
 static void Reject(Action action){bool rejected=false;try{action();}catch(InvalidOperationException){rejected=true;}Need(rejected,"invalid call accepted");}
 public static void Run(){
  var original=new UnityEngine.Rigidbody{id=1,isKinematic=true,mass=400,centerOfMass=new UnityEngine.Vector3(0,2,0),inertiaTensor=new UnityEngine.Vector3(1,2,3)};
  var input=new G11ContactProbe.Input{source_wheel="level1:58684",support="full-tire",mode="force",duration_steps=240,event_steps=new[]{1,121,145},wheel_enabled=true,side_speed_mps=.5f,torque_scale=.25f,offset_m=new UnityEngine.Vector3(10000,0,0)};
  var source=new UnityEngine.WheelCollider();G11Geometry.nativeWheels=new[]{source,source,source,source,source};var probe=new UnityEngine.GameObject("rig");body=CreateHeldProbeBody(probe);
  Need(body.isKinematic && G11BodyTrace.holds==1,"reviewed constructor not held");var wheel=G11ContactProbe.Configure(input,source,original,probe,body,true,null);
  Need(UnityEngine.WheelCollider.created==1 && UnityEngine.GameObject.ordinaryColliders==1,"copied geometry count");Need(body.writes==0 && body.angularWrites==0,"setup velocity write");
  Need(body.mass==100 && body.centerOfMass.y==2.1f && body.inertiaTensor.x==1 && body.constraints==(UnityEngine.RigidbodyConstraints)112,"declared body inputs");
  Need(wheel.radius==source.radius && wheel.center.y==source.center.y && wheel.forceAppPointDistance==0,"source profile copy");Need(wheel.transform.position.y+wheel.center.y==.65f,"wheel center placement");
  var canonical=G11ContactProbe.report.body_input;ApplyLoad();ApplyCanonical(new Initial());Need(body.massWrites==1 && body.comWrites==1 && body.tensorWrites==1 && body.rotationWrites==1,"competing property owner");
  Reject(()=>G11ContactProbe.Arm());body.isKinematic=false;wheels=new[]{wheel};RunActivation();Need(body.writes==1 && body.angularWrites==1 && body.wakes==1 && body.velocity.z==5,"arming velocity and wake count");Reject(()=>G11ContactProbe.Arm());
  var entry=new G11ContactProbe.State();G11ContactProbe.Assign(1,10,0,0,.02f);G11ContactProbe.Read(entry);var saved=entry.pre_input;
  Reject(()=>G11ContactProbe.Assign(1,10,0,0,.02f));Reject(()=>G11ContactProbe.Assign(3,12,0,0,.02f));
  for(int step=2;step<=240;step++){if(step==121)body.sleeping=true;G11ContactProbe.Assign(step,step+9,step,step*.02f,.02f);if(step==121){Need(body.impulses==1 && !body.sleeping && G11ContactProbe.report.impulse.argument.x==50,"declared impulse argument");Need(wheel.motorTorque==125,"declared motor torque");}if(step==145)Need(wheel.motorTorque==0,"release torque");}
  Need(entry.pre_input.step==1 && entry.pre_input.native_wheel0.read_revision==saved.native_wheel0.read_revision && entry.pre_input.native_wheel0.read_revision<G11Geometry.ReadWheel(source).read_revision && entry.pre_input.observed.body.velocity.z==saved.observed.body.velocity.z,"retained pre-input aliased later mutable snapshot");
  Need(body.writes==1 && body.angularWrites==1 && body.wakes==1 && body.impulses==1,"repeated input write");Need(G11ContactProbe.report.assignments[2].step==145,"third assignment missing");Reject(()=>G11ContactProbe.Assign(241,250,241,5,.02f));
  Reset();original.mass=999;original.centerOfMass=new UnityEngine.Vector3(0,9,0);original.inertiaTensor=new UnityEngine.Vector3(9,8,7);scenario.has_setup_state=true;body=CreateHeldProbeBody(new UnityEngine.GameObject("reused"));
  input.wheel_enabled=false;wheel=G11ContactProbe.Configure(input,source,original,probe,body,false,canonical);ApplyLoad();ApplyCanonical(new Initial());Need(body.mass==100 && body.centerOfMass.y==2.1f && body.inertiaTensor.x==1 && G11ContactProbe.report.source_body.mass==999,"canonical reuse replaced by later source readback");Need(body.massWrites==1 && body.comWrites==1 && body.tensorWrites==1 && body.rotationWrites==1,"reused property owner");body.isKinematic=false;G11ContactProbe.Arm();Need(!wheel.enabled && body.wakes==1,"disabled control arming");
  Console.WriteLine("G11_CONTACT_METHODS_PASS");
 }
}
'''
