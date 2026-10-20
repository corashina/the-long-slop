"""Execute native ownership methods with disabled-association and hierarchy stubs."""
from copy import deepcopy
from pathlib import Path
import subprocess
import unittest
from tools.reference.driving import method


def actual_source():
    geometry=(Path(__file__).parents[1]/'driving_geometry.cs').read_text(encoding='utf-8-sig')
    names=('public struct NativeWheel','sealed class NativeBinding','[Serializable] public class OwnershipCheckpoint',
           'static Exception NativeOwnershipFailure(', 'public static void BindNativeWheelRoles(',
           'public static void NativeOwnershipCheckpoint(', 'public static NativeWheel ReadWheel(')
    return STUBS.replace('// ACTUAL','\n'.join(method(geometry,name)[0] for name in names))


class NativeOwnershipTests(unittest.TestCase):
    def test_actual_binding_read_and_checkpoints(self):
        result=subprocess.run(['pwsh','-NoProfile','-Command',"Add-Type -TypeDefinition @'\n"+actual_source()+"\n'@\n[OwnershipChecks]::Run()"],capture_output=True,text=True,timeout=60)
        self.assertEqual(result.returncode,0,result.stdout+result.stderr)
        self.assertIn('G11_OWNERSHIP_PASS',result.stdout)

    def test_python_ownership_availability_and_missing_fields(self):
        from tools.reference.tests.test_driving_angular import gate_fixture
        from tools.reference.driving_angular import validate_angular
        row,run=gate_fixture()
        validate_angular(run,row)
        for missing in ('native_association_available','owner_is_null','structural_owner_key','structural_owner_instance_id',
                        'structural_owner_bound','structural_links_valid','enabled','active_hierarchy'):
            with self.subTest(missing=missing):
                bad=deepcopy(run);del bad['samples'][0]['native_wheels'][0][missing]
                with self.assertRaises(ValueError):validate_angular(bad,row)
        for change in ('reported_wrong','structural_wrong','enabled_active_null','inconsistent_availability','checkpoint_missing','checkpoint_wrong','geometry_missing_field','checkpoint_time','cohort'):
            with self.subTest(change=change):
                bad=deepcopy(run);wheel=bad['samples'][0]['native_wheels'][0]
                if change=='reported_wrong': wheel.update(owner_instance_id=2,owner_is_null=False,native_association_available=True)
                if change=='structural_wrong': wheel['structural_owner_instance_id']=2
                if change=='enabled_active_null': wheel.update(enabled=True,active_self=True,active_hierarchy=True)
                if change=='inconsistent_availability':wheel['native_association_available']=True
                if change=='checkpoint_missing':bad['wheel_ownership'].pop()
                if change=='checkpoint_wrong':bad['wheel_ownership'][0]['wheels'][0]['structural_owner_key']='wrong'
                if change=='geometry_missing_field':del bad['geometry'][0]['colliders'][0]['sample']['native_association_available']
                if change=='checkpoint_time':del bad['wheel_ownership'][0]['time']
                if change=='cohort':wheel['source_index']=4
                with self.assertRaises(ValueError):validate_angular(bad,row)
        for sample in run['samples']:
            sample['native_wheels'][0].update(owner_instance_id=1,owner_is_null=False,native_association_available=True)
        validate_angular(run,row)


STUBS=r'''
using System;using System.Collections.Generic;using System.Linq;
public struct Vector3 {public static Vector3 zero {get{return new Vector3();}}}
public struct WheelHit {public Vector3 point,normal;public float force;}
public class Transform {public Transform parent;public Rigidbody body;}
public class Rigidbody {public int id;public Transform transform=new Transform();public int GetInstanceID(){return id;}}
public class GameObject {public bool activeSelf=true,activeInHierarchy=true;}
public class WheelCollider {
 public int id;public Rigidbody attachedRigidbody;public Transform transform=new Transform();public GameObject gameObject=new GameObject();
 public bool enabled=false;public float motorTorque,brakeTorque,rpm,sprungMass;
 public static int hierarchyQueries;
 public int GetInstanceID(){return id;}
 public bool GetGroundHit(out WheelHit hit){hit=new WheelHit();return false;}
 public T GetComponentInParent<T>(bool includeInactive) where T:class {
  hierarchyQueries++;if(!includeInactive)throw new Exception("inactive hierarchy excluded");
  for(var node=transform;node!=null;node=node.parent) if(node.body!=null)return node.body as T;
  return null;
 }
}
public class Car {public Rigidbody RB;public List<WheelCollider> allWH=new List<WheelCollider>();}
public static class Time {public static int frameCount=1;public static float time=1;}
public static class G11Capture {public static int ObservedPhysicsStep=5;}
public static class G11Geometry {
 static bool nativeRolesBound;static Car car;static WheelCollider[] nativeWheels;
 static Dictionary<WheelCollider,NativeBinding> nativeIdentities=new Dictionary<WheelCollider,NativeBinding>();
 public static List<OwnershipCheckpoint> ownershipCheckpoints=new List<OwnershipCheckpoint>();
 static Transform group;
 static string Key(WheelCollider w){return "source-"+w.id;}
 static string Key(Rigidbody b){return b==null?null:"source-body-"+b.id;}
 // ACTUAL
 public static void Seed(){
  nativeRolesBound=false;nativeIdentities.Clear();ownershipCheckpoints.Clear();WheelCollider.hierarchyQueries=0;
  var rb=new Rigidbody{id=1};rb.transform.body=rb;car=new Car{RB=rb};group=new Transform{parent=rb.transform};
  nativeWheels=Enumerable.Range(0,5).Select(i=>new WheelCollider{id=i+10,transform=new Transform{parent=i==4?rb.transform:group}}).ToArray();
  nativeWheels[4].gameObject.activeSelf=false;nativeWheels[4].gameObject.activeInHierarchy=false;
  foreach(var w in nativeWheels)nativeIdentities.Add(w,new NativeBinding{identity=new NativeWheel{key=Key(w),instance_id=w.id,source_index=w.id-10,road_slot=-1}});
  car.allWH.AddRange(nativeWheels.Take(4).Reverse());
 }
 static void Need(bool v,string m){if(!v)throw new Exception(m);}
 static void Reject(Action action,string predicate){try{action();}catch(InvalidOperationException e){Need(e.Message.Contains(predicate),"wrong failure: "+e.Message);return;}throw new Exception("accepted "+predicate);}
 public static void Check(){
  Seed();BindNativeWheelRoles();int queries=WheelCollider.hierarchyQueries;
  for(int i=0;i<5;i++){var w=nativeWheels[i];var state=ReadWheel(w);
   Need(state.instance_id==w.id && state.key==Key(w) && state.source_index==i,"identity changed");
   Need(state.owner_is_null && !state.native_association_available && state.owner_instance_id==0,"raw null replaced");
   Need(state.structural_owner_bound && state.structural_links_valid && state.structural_owner_instance_id==1 && state.structural_owner_key=="source-body-1","structural binding");
   Need(state.road_slot==(i==4?-1:3-i),"helper or road role changed");}
  Need(WheelCollider.hierarchyQueries==queries,"per-step hierarchy query");
  Need(ownershipCheckpoints.Count==1,"setup checkpoint missing");NativeOwnershipCheckpoint("held",6);
  Need(ownershipCheckpoints.Count==2,"held checkpoint missing");
  Seed();group.body=new Rigidbody{id=2};Reject(BindNativeWheelRoles,"structural-owner");
  Seed();nativeWheels[0].attachedRigidbody=new Rigidbody{id=2};Reject(BindNativeWheelRoles,"reported-native-owner");
  Seed();nativeWheels[0].enabled=true;Reject(BindNativeWheelRoles,"enabled-active-native-owner");
  Seed();nativeWheels[4].enabled=true;BindNativeWheelRoles();Need(!ReadWheel(nativeWheels[4]).native_association_available,"inactive null rejected");
  Seed();var swap=nativeWheels[0];nativeWheels[0]=nativeWheels[1];nativeWheels[1]=swap;Reject(BindNativeWheelRoles,"source-index");
  Seed();BindNativeWheelRoles();nativeWheels[0].transform.parent=new Transform();Reject(()=>ReadWheel(nativeWheels[0]),"cached-parent-link");
  Seed();BindNativeWheelRoles();car.allWH.Reverse();Reject(()=>NativeOwnershipCheckpoint("roles",7),"checkpoint-road-role");
  Seed();BindNativeWheelRoles();group.body=new Rigidbody{id=2};ReadWheel(nativeWheels[0]);Reject(()=>NativeOwnershipCheckpoint("end",7),"checkpoint-structural-owner");
  Seed();BindNativeWheelRoles();nativeWheels[0].attachedRigidbody=new Rigidbody{id=2};Reject(()=>ReadWheel(nativeWheels[0]),"reported-native-owner");
  Seed();BindNativeWheelRoles();var missing=nativeWheels[0];nativeWheels[0]=nativeWheels[1];Reject(()=>ReadWheel(missing),"sample-cohort");
  Seed();foreach(var w in nativeWheels)w.attachedRigidbody=car.RB;BindNativeWheelRoles();Need(ReadWheel(nativeWheels[0]).native_association_available,"reported owner lost");
  Reject(BindNativeWheelRoles,"cohort-count");Console.WriteLine("G11_OWNERSHIP_PASS disabled_native_null inactive_native_null raw_identity structural_wrong native_wrong enabled_null cohort parent_link checkpoint");
 }
}
public static class OwnershipChecks {public static void Run(){G11Geometry.Check();}}
'''

if __name__=='__main__':unittest.main()
