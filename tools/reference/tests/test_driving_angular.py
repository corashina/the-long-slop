from copy import deepcopy
from pathlib import Path
import json
import subprocess
import unittest
from tools.reference.driving_angular import angular_rows, validate_angular_schedule, validate_angular


def gate_fixture(index=0):
    """Fabricated gate input. This is not native evidence or a response model."""
    row=angular_rows()[index];spec=row['angular'];z=dict(x=0.,y=0.,z=0.);q=dict(x=0.,y=0.,z=0.,w=1.)
    profile=dict(center=z,radius=.3,mass=20.,damping=.25,suspension_distance=.3,force_application_distance=0.,spring_n_per_m=25000.,damper_n_s_per_m=1500.,spring_target=.45,forward={},sideways={})
    body=dict(instance_id=2,constraints=spec['rig']['constraints'],mass=100.,center_of_mass=z,inertia_tensor=dict(x=1.,y=2.,z=3.),inertia_tensor_rotation=q,position=z,rotation=q,velocity=spec['rig']['velocity_mps'],angular_velocity=z,gravity=z,use_gravity=False,drag=0.,angular_drag=0.,sleep_threshold=0.,sleeping=False,solver_iterations=6,solver_velocity_iterations=1,max_angular_velocity=7.,detect_collisions=True,kinematic=False,interpolation=0,collision_detection_mode=0)
    native=[dict(key='level1:'+str(k),instance_id=i+10,road_slot=i if i<4 else -1,source_index=i,owner_instance_id=1,enabled=False) for i,k in enumerate((58684,58685,58686,58687,58655))]
    shapes=[dict(type='WheelCollider',**n,**profile,sample=dict(enabled=False)) for n in native]
    chosen=next(n for n in native if n['key']==spec['source_wheel'])
    transform=dict(position=z,rotation=q,local_position=z,local_rotation=q,local_scale=dict(x=1.,y=1.,z=1.),lossy_scale=dict(x=1.,y=1.,z=1.))
    report=dict(declared=deepcopy(spec),source_key=spec['source_wheel'],support=spec['support'],copied_wheel_id=50,source_wheel_id=chosen['instance_id'],original_body_id=1,copied_body_id=2,
                source_profile=deepcopy(profile),copied_profile=deepcopy(profile),source_body=dict(body,instance_id=1),copied_constructor_body=body,copied_held_body=body,
                source_transform=deepcopy(transform),copied_transform=dict(deepcopy(transform),position=deepcopy(spec['rig']['offset_m'])),source_sprung_mass=200.,copied_sprung_mass=100.,assignments=[dict(event_revision=i+1,**e) for i,e in enumerate(spec['events'])])
    report['body_input']={k:deepcopy(body[k]) for k in ('mass','center_of_mass','inertia_tensor','inertia_tensor_rotation')}
    report['wheel_input']=dict(position=deepcopy(spec['rig']['offset_m']),rotation=deepcopy(q),local_scale=dict(x=1.,y=1.,z=1.))
    report['initial_state']=dict(profile,body=deepcopy(body),wheel_id=50,enabled=True,event_revision=0,assignment_step=0,phase='before-first-assignment')
    samples=[];previous=0.
    for step in range(1,481):
        i=max(i for i,e in enumerate(spec['events']) if e['step']<=step);event=spec['events'][i];t=(step-1)%60
        values=(2.*(t+1),120.-t,59.-t,-2.*(t+1),-120.+t,-59.+t,1.+t,59.-t)
        if spec['purpose']=='confirmation': values=(2.*(t+1),119.-2*t,-2.*(t+1),min(0.,-118.+3*t),0.,0.,-1.-t,-59.+t)
        rpm=values[i] if spec['purpose']!='baseline' else 0.
        state=dict(profile,body=deepcopy(body),event_revision=i+1,assignment_step=step,phase=event['phase'],enabled=True,active_self=True,active_hierarchy=True,wheel_id=50,sprung_mass=100.,sprung_mass_available=True)
        for phase,value in [('physics.entry',previous),('physics.exit',rpm)]:
            samples.append(dict(step=step,sequence=len(samples)+1,phase=phase,frame=step,delta_time=.02,time=step*.02,fixed_dt=.02,input_sequence=0,angular_state=deepcopy(state),native_wheels=deepcopy(native),wheels=[dict(rpm=value,motor_torque=event['motor_nm'],brake_torque=event['brake_nm'],contact=False)]))
        previous=rpm
    return row,dict(angular=report,body_trace=dict(retained_instance_id=1,copied_instance_id=2),geometry=[dict(phase='armed-after-canonical',colliders=shapes)],samples=samples,initial=dict(fixed_dt=.02))


class AngularTests(unittest.TestCase):
    def test_declared_rows(self):
        rows=angular_rows()
        self.assertEqual(len(rows),10)
        committed=json.loads((Path(__file__).parents[3]/'tests/fixtures/driving/angular-scenarios.json').read_text())
        self.assertEqual(committed['scenarios'],rows)
        self.assertEqual(len({r['id'] for r in rows}),10)
        for row in rows: validate_angular_schedule(row)
        self.assertEqual({r['angular']['support'] for r in rows}, {'full-tire','worn-tire','bare-rim','hub','auxiliary-helper'})

    def test_reject_bad_schedule(self):
        row=angular_rows()[0]
        for field,value in [('motor_nm',1),('brake_nm',1)]:
            bad=deepcopy(row);bad['angular']['events'][2][field]=value
            with self.assertRaises(ValueError): validate_angular_schedule(bad)
        for key,value in [('source_wheel','level1:58655'),('support','hub'),('provenance','guessed'),('duration_steps',481)]:
            bad=deepcopy(row);bad['angular'][key]=value
            with self.assertRaises(ValueError): validate_angular_schedule(bad)

    def test_missing_native_evidence(self):
        with self.assertRaises(ValueError): validate_angular({},angular_rows()[0])

    def test_actual_evidence_gate_and_rejections(self):
        row,run=gate_fixture()
        self.assertEqual(validate_angular(run,row)['steps'],480)
        cases=('contact','sleep','disabled','identity','profile','assignment','coast','missing_event','repeated_event','missing_solve','source_id','source_body','no_spin','no_coast','no_overlap','sprung_mass','fixed_dt','body_change','source_enabled')
        for case in cases:
            with self.subTest(case=case):
                bad=deepcopy(run);sample=bad['samples'][241]
                if case=='contact': sample['wheels'][0]['contact']=True
                if case=='sleep': sample['angular_state']['body']['sleeping']=True
                if case=='disabled': sample['angular_state']['enabled']=False
                if case=='identity': sample['angular_state']['wheel_id']=10
                if case=='profile': sample['angular_state']['radius']=.2
                if case=='assignment': sample['angular_state']['event_revision']=1
                if case=='coast': sample['wheels'][0]['motor_torque']=20.
                if case=='missing_event': bad['angular']['assignments'].pop()
                if case=='repeated_event': bad['angular']['assignments'].append(bad['angular']['assignments'][0])
                if case=='missing_solve': bad['samples'].pop(240)
                if case=='source_id': sample['native_wheels'][4]['key']='level1:58684'
                if case=='source_body': bad['angular']['source_body']['instance_id']=2
                if case=='no_spin':
                    for s in bad['samples']: s['wheels'][0]['rpm']=0.
                if case=='no_coast':
                    for s in bad['samples']:
                        if s['angular_state']['phase'].startswith('coast'): s['wheels'][0]['rpm']=1.
                if case=='no_overlap':
                    for s in bad['samples']:
                        if s['angular_state']['phase']=='positive-low': s['wheels'][0]['rpm']=1000.+s['step']
                        if s['angular_state']['phase']=='positive-revisit': s['wheels'][0]['rpm']=2000.+s['step']
                if case=='sprung_mass': sample['angular_state']['sprung_mass_available']=False
                if case=='fixed_dt': sample['fixed_dt']=.01
                if case=='body_change': sample['angular_state']['body']['inertia_tensor']['x']=4.
                if case=='source_enabled': sample['native_wheels'][0]['enabled']=True
                with self.assertRaises(ValueError): validate_angular(bad,row)

    def test_helper_baseline_keeps_free_body(self):
        row,run=gate_fixture(8)
        self.assertEqual(validate_angular(run,row)['purpose'],'baseline')
        run['samples'][0]['angular_state']['body']['constraints']=116
        with self.assertRaises(ValueError): validate_angular(run,row)

    def test_confirmation_requires_actual_hold(self):
        row,run=gate_fixture(1)
        self.assertEqual(validate_angular(run,row)['purpose'],'confirmation')
        for sample in run['samples']:
            if sample['angular_state']['phase']=='brake-hold': sample['wheels'][0]['rpm']=1.
        with self.assertRaisesRegex(ValueError,'held-zero'): validate_angular(run,row)

    def test_source_events_remain_separate_from_disabled_native_wheels(self):
        from tools.reference.tests.test_driving_controls import scenario_fixture, add_control_fixture
        from tools.reference.tests.test_driving_probe import DrivingProbeTests
        from tools.reference.driving_controls import validate_controls
        scenario=scenario_fixture('wheel_angular')
        run=add_control_fixture(DrivingProbeTests.sample_run(),scenario)
        run['initial']['source_control_wheels']=deepcopy(run['initial']['wheels'])
        run['initial']['wheels']=run['initial']['wheels'][:1]
        for sample in run['samples']: sample['wheels']=[dict(motor_torque=60.,brake_torque=0.)]
        self.assertIn('unavailable',validate_controls(run,scenario)['source_native_torque'])
        run['samples'][next(i for i,s in enumerate(run['samples']) if s['phase']=='input.exit')]['controls']['input_handbrake_press']=True
        with self.assertRaises(ValueError): validate_controls(run,scenario)

    def test_exact_initial_inputs_keep_pose_and_ignore_only_native_ids(self):
        from tools.reference.driving_angular import angular_initial_inputs
        row,run=gate_fixture()
        before=angular_initial_inputs(run)
        run['angular']['initial_state']['wheel_id']=999
        run['angular']['initial_state']['body']['instance_id']=888
        self.assertEqual(before,angular_initial_inputs(run))
        run['angular']['initial_state']['body']['rotation']['x']=.01
        self.assertNotEqual(before,angular_initial_inputs(run))

    def test_actual_role_binding_waits_for_source_initialization(self):
        from tools.reference.driving import method
        actual=method((Path(__file__).parents[1]/'driving_geometry.cs').read_text(encoding='utf-8-sig'),'public static void BindNativeWheelRoles(')[0]
        source=ROLE_STUBS.replace('// ACTUAL',actual)
        result=subprocess.run(['pwsh','-NoProfile','-Command',"Add-Type -TypeDefinition @'\n"+source+"\n'@\n[RoleChecks]::Run()"],capture_output=True,text=True,timeout=60)
        self.assertEqual(result.returncode,0,result.stdout+result.stderr)
        self.assertIn('G11_ROLES_PASS',result.stdout)

    def test_actual_native_helper_with_compiled_stubs(self):
        source=(Path(__file__).parents[1]/'driving_angular.cs').read_text(encoding='utf-8-sig')+STUBS
        result=subprocess.run(['pwsh','-NoProfile','-Command',"Add-Type -TypeDefinition @'\n"+source+"\n'@\n[AngularChecks]::Run()"],capture_output=True,text=True,timeout=60)
        self.assertEqual(result.returncode,0,result.stdout+result.stderr)
        self.assertIn('G11_ANGULAR_STUB_PASS',result.stdout)


ROLE_STUBS=r'''
using System;using System.Collections.Generic;using System.Linq;
public class WheelCollider { public int id;public object attachedRigidbody;public int GetInstanceID(){return id;} }
public class Car {public object RB=new object();public List<WheelCollider> allWH=new List<WheelCollider>();}
public static class G11Geometry {
 public struct NativeWheel {public int road_slot,instance_id,source_index;public string key;}
 public static Car car=new Car();public static WheelCollider[] nativeWheels;
 public static Dictionary<WheelCollider,NativeWheel> nativeIdentities=new Dictionary<WheelCollider,NativeWheel>();
 static bool nativeRolesBound;public static string Key(WheelCollider w){return "source-"+w.id;}
 // ACTUAL
}
public static class RoleChecks {
 static void Need(bool v,string m){if(!v)throw new Exception(m);}
 public static void Run(){
  G11Geometry.nativeWheels=Enumerable.Range(0,5).Select(i=>new WheelCollider{id=i+10,attachedRigidbody=G11Geometry.car.RB}).ToArray();
  foreach(var w in G11Geometry.nativeWheels) G11Geometry.nativeIdentities.Add(w,new G11Geometry.NativeWheel{key="source-"+w.id,instance_id=w.id,source_index=w.id-10,road_slot=-1});
  bool failed=false;try{G11Geometry.BindNativeWheelRoles();}catch(InvalidOperationException){failed=true;}Need(failed,"empty source roles accepted");
  G11Geometry.car.allWH.AddRange(G11Geometry.nativeWheels.Take(4).Reverse());G11Geometry.BindNativeWheelRoles();
  Need(G11Geometry.nativeIdentities[G11Geometry.nativeWheels[4]].road_slot==-1,"helper became road slot");
  for(int i=0;i<4;i++){var w=G11Geometry.car.allWH[i];var n=G11Geometry.nativeIdentities[w];Need(n.road_slot==i && n.instance_id==w.id && n.key=="source-"+w.id,"binding changed");}
  failed=false;try{G11Geometry.BindNativeWheelRoles();}catch(InvalidOperationException){failed=true;}Need(failed,"roles rebound");Console.WriteLine("G11_ROLES_PASS");
 }
}
'''

STUBS=r'''
namespace UnityEngine {
 public struct Vector3 { public float x,y,z;public static Vector3 operator+(Vector3 a,Vector3 b){return new Vector3{x=a.x+b.x,y=a.y+b.y,z=a.z+b.z};} }
 public struct Quaternion { public float x,y,z,w; }
 public struct WheelFrictionCurve { public float extremumSlip,extremumValue,asymptoteSlip,asymptoteValue,stiffness; }
 public struct JointSpring { public float spring,damper,targetPosition; }
 public enum RigidbodyConstraints { None=0,Freeze=116 }
 public class Transform {public Transform parent;public Vector3 position,localPosition,localScale,lossyScale;public Quaternion rotation,localRotation;}
 public class GameObject {public Transform transform=new Transform();public bool activeSelf=true,activeInHierarchy=true;public GameObject(string n){} public T AddComponent<T>() where T:new(){return new T();}}
 public class Rigidbody {
  public int id=2,solverIterations=6,solverVelocityIterations=1,interpolation,collisionDetectionMode,writes;
  public float mass=100,drag,angularDrag,sleepThreshold,maxAngularVelocity=7;public bool useGravity,isKinematic=true,detectCollisions=true;
  public RigidbodyConstraints constraints;public Vector3 centerOfMass,inertiaTensor,position,angularVelocity;public Quaternion rotation,inertiaTensorRotation;
  Vector3 speed;public Vector3 velocity {get{return speed;}set{writes++;speed=value;}}
  public bool IsSleeping(){return false;} public int GetInstanceID(){return id;}
 }
 public class WheelCollider {
  public GameObject gameObject=new GameObject("wheel"); public Transform transform {get{return gameObject.transform;}}
  public bool enabled=true; public Vector3 center;public float radius=.3f,mass=20,wheelDampingRate=.25f,suspensionDistance=.3f,forceAppPointDistance,sprungMass=123,rpm=-7,steerAngle;
  public WheelFrictionCurve forwardFriction,sidewaysFriction;public JointSpring suspensionSpring;
  float motor,brake;public int motors,brakes;
  public float motorTorque{get{return motor;}set{motors++;motor=value;}}
  public float brakeTorque{get{return brake;}set{brakes++;brake=value;}}
  public int GetInstanceID(){return 50;}
 }
 public static class Physics {public static Vector3 gravity;}
}
public static class G11Geometry {public static string Key(UnityEngine.WheelCollider value){return "level1:58684";}}
public static class AngularChecks {
 static void Need(bool value,string message){if(!value)throw new Exception(message);}
 public static void Run(){
  var source=new UnityEngine.WheelCollider();var original=new UnityEngine.Rigidbody{id=1};var held=new UnityEngine.Rigidbody();
  var input=new G11AngularProbe.Input{source_wheel="level1:58684",support="full-tire",duration_steps=5,
   rig=new G11AngularProbe.Rig{constraints=116,velocity_mps=new UnityEngine.Vector3{x=10}},
   events=new[]{new G11AngularProbe.Event{step=1,phase="positive",motor_nm=60},new G11AngularProbe.Event{step=3,phase="coast",motor_nm=0,brake_nm=0}}};
  var wheel=G11AngularProbe.Configure(input,source,original,new UnityEngine.GameObject("rig"),held);
  Need(held.isKinematic && held.writes==0,"hold changed or setup velocity correction");
  Need(wheel.radius==source.radius && wheel.mass==source.mass && wheel.wheelDampingRate==source.wheelDampingRate,"profile copy");
  Need(G11AngularProbe.report.source_sprung_mass==123 && G11AngularProbe.report.copied_sprung_mass==123,"separate sprung mass");
  held.isKinematic=false;G11AngularProbe.Arm();Need(held.writes==1 && held.velocity.x==10,"one-time velocity");
  wheel.enabled=true;G11AngularProbe.RetainInitial();Need(G11AngularProbe.report.initial_state.body.velocity.x==10,"initial state");
  G11AngularProbe.Assign(1);int writes=wheel.motors;G11AngularProbe.Assign(2);Need(wheel.motors==writes,"repeated event setter");
  var state=new G11AngularProbe.State();G11AngularProbe.Read(state);Need(state.event_revision==1 && state.assignment_step==2 && state.body.instance_id==2,"revision and body");
  G11AngularProbe.Assign(3);Need(wheel.motorTorque==0 && wheel.brakeTorque==0,"active coast torque");
  Need(wheel.rpm==-7 && held.writes==1,"angular or velocity correction");
  bool rejected=false;try{G11AngularProbe.Assign(3);}catch(InvalidOperationException){rejected=true;}Need(rejected,"repeated solve accepted");
  rejected=false;try{G11AngularProbe.Assign(5);}catch(InvalidOperationException){rejected=true;}Need(rejected,"missing solve accepted");
  G11AngularProbe.Read(state);Need(state.phase=="coast" && state.event_revision==2 && state.enabled==true,"coast observation");
  Need(G11AngularProbe.report.assignments[1].step==3,"assignment evidence");Console.WriteLine("G11_ANGULAR_STUB_PASS");
 }
}
'''

if __name__=='__main__': unittest.main()
