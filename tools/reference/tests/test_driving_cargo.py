"""Critical declared cargo setup input checks."""
from pathlib import Path
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[1]

class DrivingCargoTests(unittest.TestCase):
    def test_source_boundary_and_later_setup_order(self):
        source = (ROOT / 'driving_capture.cs').read_text(encoding='utf-8-sig')
        calls = ('G11Geometry.Capture("source-physical-ready"', 'G11Geometry.ApplyCargoInput(',
                 'main.PressedContinue();', 'ApplyAssemblySetup(car,assemblySetup);', 'main.player.GetIn(driverSeat);')
        positions = [source.index(call) for call in calls]
        self.assertEqual(positions, sorted(positions))
        self.assertEqual(source.count('G11Geometry.ApplyCargoInput('), 1)
        self.assertNotIn('G11Cargo.Apply(', source)
        geometry = (ROOT / 'driving_geometry.cs').read_text(encoding='utf-8-sig')
        self.assertEqual(geometry.count('G11Cargo.Apply('), 1)
        self.assertIn('cargo.GetComponentsInChildren<Rigidbody>(true).Length!=0', geometry)
        self.assertIn('cargo.physLockID!=mount.id', geometry)
        self.assertIn('G11Cargo.report.caches_before.SequenceEqual(G11Cargo.report.caches_after)', geometry)

    def test_actual_cargo_application(self):
        source = (ROOT / 'driving_cargo.cs').read_text(encoding='utf-8-sig')
        command = "Add-Type -TypeDefinition @'\n" + source + STUBS + CHECKS + "\n'@\n[CargoTests]::Run()"
        result = subprocess.run(['pwsh', '-NoProfile', '-Command', command], capture_output=True, text=True, timeout=60)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn('G11_CARGO_PASS', result.stdout)

STUBS = r"""
namespace UnityEngine {
 public struct Vector3 { public float x,y,z; }
 public struct Quaternion { public float x,y,z,w; }
 public class Transform {
  public string key; public int writes;public Transform parent;public object cache=new object();
  Vector3 pos,scale;Quaternion rot;
  public Vector3 localPosition { get { return pos; } set { writes++;pos=value; } }
  public Quaternion localRotation { get { return rot; } set { writes++;rot=value;rot.w+=.125f; } }
  public Vector3 localScale { get { return scale; } set { writes++;scale=value; } }
 }
 public static class Time { public static int frameCount=10;public static float time=1.25f,fixedTime=1.24f; }
}
public static class G11Geometry { public static string Key(UnityEngine.Transform value) { return value==null?null:value.key; } }
"""
CHECKS = r"""
public static class CargoTests {
 static void Require(bool value,string label) { if(!value) throw new Exception(label); }
 static G11Cargo.Binding Make(string key) {
  var target=new G11Cargo.Target{key=key,pickup=key+"/pickup",save=key+"/save",parent="mount-root",mount="mount",lock_parent="save-owner",body="body",category=0,item=74,lock_id=1,
   local_position=new Vector3{x=1.25f,y=2,z=3},local_rotation=new Quaternion{x=.1f,y=.2f,z=.3f,w=.4f},local_scale=new Vector3{x=100,y=99,z=98}};
  return new G11Cargo.Binding{state=target,root=new Transform{key=key,parent=new Transform{key="mount-root"}},root_id=1,pickup_id=2,save_id=3,mount_id=4,body_id=5,
    locked=true,body_retired=true,ownership_ready=true,lock_parent_id=7,expected_lock_parent_id=7,owner_ids=new[]{5}};
 }
 public static void Run() {
  foreach(string corruption in new[]{"missing","duplicate","extra","parent","mount","lock","body","owner","unsupported","identity","nan"}) {
   var a=Make("a");var b=Make("b");var cohort=new[]{a,b};var input=G11Cargo.Declare(cohort,4,2);var saved=a.root.cache;
   switch(corruption) {
    case "missing":input.targets=new[]{input.targets[0]};break;
    case "duplicate":input.targets[1]=input.targets[0];break;
    case "extra":input.targets=new[]{input.targets[0],input.targets[1],input.targets[0]};break;
    case "parent":b.root.parent.key="wrong";break;
    case "mount":input.targets[1].mount="wrong";break;
    case "lock":b.lock_parent_id=8;break;
    case "body":b.body_retired=false;break;
    case "owner":b.owner_ids[0]=8;break;
    case "unsupported":b.unsupported=true;break;
    case "identity":input.targets[1].pickup="wrong";break;
    case "nan":input.targets[1].local_rotation.x=float.NaN;break;
   }
   var report=new G11Cargo.Report();bool rejected=false;
   try { G11Cargo.Apply(input,cohort,report,4,2,true); } catch(InvalidOperationException) { rejected=true; }
   Require(rejected && a.root.writes==0 && b.root.writes==0 && report.applications==0,"reject before all writes: "+corruption);
   Require(Object.ReferenceEquals(saved,a.root.cache),"cache changed");
  }
  var bindings=new[]{Make("a"),Make("b")};var declared=G11Cargo.Declare(bindings,4,2);var result=new G11Cargo.Report();
  G11Cargo.Apply(declared,bindings,result,4,2,true);
  Require(result.applications==1 && result.before.Length==2 && result.after.Length==2,"one complete application");
  Require(bindings[0].root.writes==3 && bindings[1].root.writes==3,"once per setter");
  Require(result.arguments[0].local_rotation.w==.4f && result.after[0].local_rotation.w==.525f,"exact arguments separate from native output");
  Require(result.arguments[0].local_scale.x==100 && bindings[0].root.localScale.z==98,"non-unit scale");
  Require(result.frame==10 && result.physics_step==4 && result.input_revision==2,"application stamps");
  bool second=false;try { G11Cargo.Apply(declared,bindings,result,5,3,true); } catch(InvalidOperationException) { second=true; }
  Require(second && bindings[0].root.writes==3 && bindings[1].root.writes==3,"second application rejected");
  var first=Make("held");bool dynamic=false;try { G11Cargo.Apply(G11Cargo.Declare(new[]{first},4,2),new[]{first},new G11Cargo.Report(),4,2,false); } catch(InvalidOperationException) { dynamic=true; }
  Require(dynamic && first.root.writes==0,"held boundary");
  Console.WriteLine("G11_CARGO_PASS exact_arguments native_output once cohort validation_before_writes ownership caches scale");
 }
}
"""
