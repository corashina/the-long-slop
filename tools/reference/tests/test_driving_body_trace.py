"""Execute the actual selected-body adapter and pre-input driver boundaries."""
from pathlib import Path
import subprocess
import unittest

from tools.reference.driving import method

ROOT = Path(__file__).resolve().parents[1]


def compiled_source():
    driver = (ROOT / 'driving_driver.cs').read_text(encoding='utf-8-sig')
    actual = '\n'.join(method(driver, name)[0] for name in (
        '[Serializable] public class BodyState', 'public static BodyState ReadBody(',
        'public static void Observe(', 'public static bool Tick(', 'static bool TickCore('))
    trace = (ROOT / 'driving_body_trace.cs').read_text(encoding='utf-8-sig')
    return trace + STUBS + '\npublic static class G11Driver {\n' + DRIVER + actual + '\n}\n' + CHECKS


STUBS = r"""
namespace UnityEngine {
    public struct Vector3 { public float x,y,z; public static Vector3 zero { get { return new Vector3(); } } }
    public class Rigidbody {
        public string key; public int id,writes,reads; public bool value,forbidRead,destroyed;
        public Exception rejected; public Vector3 speed,spin;
        public bool isKinematic {
            get { reads++;if(forbidRead) throw new Exception("body inspected");return value; }
            set { writes++;if(rejected!=null) throw rejected;this.value=value; }
        }
        public Vector3 velocity { get { reads++;if(forbidRead) throw new Exception("velocity inspected");return speed; } }
        public Vector3 angularVelocity { get { reads++;if(forbidRead) throw new Exception("spin inspected");return spin; } }
        public int GetInstanceID() { reads++;if(forbidRead) throw new Exception("id inspected");return id; }
    }
    public static class Time { public static int frameCount=10;public static float time=1.25f,fixedTime=1.24f,deltaTime=.02f; }
    public static class Debug { public static void LogException(Exception error) {} }
    public static class JsonUtility { public static string ToJson(object value) { return "stub"; } }
}
public class carscript { public Rigidbody RB; }
public class fpscontroller { public float dt=.03f; }
public static class G11Geometry {
    public static string Key(Rigidbody target) { if(target==null) return null;if(target.forbidRead) throw new Exception("key inspected");return target.key; }
}
public static class G11Capture {
    public static bool TraceOpen=true,IsArmed=false;
    public static int ObservedPhysicsStep=63,InputRevision=12;
    public static Rigidbody ObservedBody;public static Exception failure;public static int rejections;public static bool rejectReporting;
    public static void TraceFailure(Exception error) { failure=error;if(rejectReporting) throw new Exception("reporting failure"); }
    public static void CaptureDriverRejection(string callback) { rejections++; }
}
public static class G11Lifecycle {
    public static bool Ready=false;public static int calls;
    public static void Player(fpscontroller player,string phase,int sequence,int applied) { calls++; }
    public static string Pending() { return "pending"; }
}
"""
DRIVER = r"""
    public class Input {}
    public class Stamp {}
    public class State {}
    public class Report {
        public int input_applications;
        public List<BodyState> rejected_bodies=new List<BodyState>();
        public List<State> states=new List<State>();
    }
    public static Report report=new Report();
    public static fpscontroller player=new fpscontroller();
    public static carscript car;
    static Rigidbody originalBody=null,retainedCarBody;
    static string retainedCarBodyKey;static int retainedCarBodyId;
    static bool getInExited,removed,transferred,applied,prepared=false,armed=false;
    static int ownershipUpdate,appliedUpdate=0,startedFrame=10;
    static Stamp readyStamp;static State readyState;
    public static int UpdateSequence { get; private set; }
    public static float ConsumedDt { get; private set; }
    public static int InputApplications { get { return report.input_applications; } }
    public static int canonical_reads,stamp_count,local_checks;
    static void StampAt(string phase,string callback,int step) { stamp_count++; }
    static Stamp ReadStamp(string phase,string callback,int step) { return new Stamp(); }
    static State ReadState(string phase) { return new State(); }
    static void RequireLocal(string phase="observer.LateUpdate") { local_checks++; }
    static bool OwnersTransferred() { return transferred; }
    static bool Neutral() { return true; }
    static Input MakeInput() { canonical_reads++;throw new Exception("canonical read before readiness"); }
    static void Apply(Input input) { report.input_applications++;applied=true; }
    public static void Bind(carscript target) { car=target;retainedCarBody=target.RB;
        retainedCarBodyKey=target.RB.key;retainedCarBodyId=target.RB.id; }
    public static void ReadyForGuard() { getInExited=true;removed=true;transferred=true;ownershipUpdate=0;UpdateSequence=2; }
"""
CHECKS = r"""
public static class BodyTraceTests {
    static void Require(bool value,string label) { if(!value) throw new Exception(label); }
    static void CheckPair(int index,bool value,int call) {
        var before=G11BodyTrace.report.events[index];var after=G11BodyTrace.report.events[index+1];
        Require(before.kind=="setter.before" && after.kind=="setter.after" && before.order<after.order,"setter pair order");
        foreach(var e in new[]{before,after}) {
            Require(e.caller=="System.Void itemPlaceRemoveScript::RemoveItems()" && e.il_offset==0x2af && e.original_opcode=="callvirt","caller metadata");
            Require(e.call_id==call && e.requested==value && e.callback=="Rigidbody.set_isKinematic","exact request");
            Require(e.frame==10 && e.physics_step==63 && e.input_revision==12 && e.update_sequence==0 && e.input_applications==0,"exact stamps");
            Require(e.time==1.25f && e.fixed_time==1.24f && e.delta_time==.02f && !e.armed,"exact clocks and arm");
            Require(e.target.key=="level1:50613" && e.target.instance_id==465454 && e.retained.instance_id==465454,"retained identity");
            Require(e.current.instance_id==465454 && e.observer.instance_id==465454 && e.current_is_retained && e.current_is_observer,"separate identities");
        }
        Require(after.target.kinematic==value,"actual after value");
    }
    public static void Run() {
        foreach(bool value in new[]{false,true}) {
            var plain=new Rigidbody{forbidRead=true};G11BodyTrace.Set(plain,value,"unbound",7,"call");
            Require(plain.writes==1 && plain.value==value && plain.reads==0,"unbound plain forward");
        }
        var body=new Rigidbody{key="level1:50613",id=465454,value=true};var car=new carscript{RB=body};
        G11Capture.ObservedBody=body;G11BodyTrace.Bind(car);G11Driver.Bind(car);
        foreach(bool value in new[]{false,true}) {
            var other=new Rigidbody{forbidRead=true};G11BodyTrace.Set(other,value,"unrelated",7,"callvirt");
            Require(other.writes==1 && other.value==value && other.reads==0,"unrelated body inspection");
        }
        var unrelatedError=new InvalidOperationException("unrelated original");var rejectedOther=new Rigidbody{forbidRead=true,rejected=unrelatedError};
        try { G11BodyTrace.Set(rejectedOther,true,"unrelated",0,"callvirt");throw new Exception("unrelated swallowed"); }
        catch(InvalidOperationException error) { Require(Object.ReferenceEquals(error,unrelatedError) && rejectedOther.writes==1 && rejectedOther.reads==0,"unrelated exception"); }
        Require(G11BodyTrace.report.events.Count==0,"unrelated trace");
        G11BodyTrace.Set(body,false,"System.Void itemPlaceRemoveScript::RemoveItems()",0x2af,"callvirt");CheckPair(0,false,1);
        G11BodyTrace.Set(body,true,"System.Void itemPlaceRemoveScript::RemoveItems()",0x2af,"callvirt");CheckPair(2,true,2);
        Require(body.writes==2 && G11BodyTrace.report.events[0].target.kinematic && !G11BodyTrace.report.events[2].target.kinematic,"before state");
        var original=new InvalidOperationException("original setter exception");body.rejected=original;
        try { G11BodyTrace.Set(body,false,"throwing",42,"call");throw new Exception("swallowed"); }
        catch(InvalidOperationException error) { Require(Object.ReferenceEquals(error,original),"original exception replaced"); }
        Require(body.writes==3 && body.value && G11BodyTrace.report.events[5].kind=="setter.exception" &&
            G11BodyTrace.report.events[5].original_exception.Contains("original setter exception"),"exception observation");
        body.rejected=null;
        // Use the actual driver Observe and Tick methods before Apply. Canonical targets must remain unread.
        int start=G11BodyTrace.report.events.Count;
        G11Driver.Observe(G11Driver.player,"driver.get-in.exit",63);
        G11Driver.Observe(G11Driver.player,"driver.update.entry",63);
        G11Driver.Observe(G11Driver.player,"driver.forward.before",63);
        G11Driver.Observe(G11Driver.player,"driver.update.exit",63);
        Require(!G11Driver.Tick(null,63),"early readiness changed");
        Require(G11Driver.InputApplications==0 && G11Driver.canonical_reads==0 && G11Lifecycle.calls==0 && G11Driver.UpdateSequence==1,"pre-input state changed");
        string[] callbacks={"driver.get-in.exit","driver.update.entry","driver.forward.before","driver.update.exit","driver.Tick.entry","driver.Tick.exit"};
        for(int i=0;i<callbacks.Length;i++) {
            var e=G11BodyTrace.report.events[start+i];Require(e.callback==callbacks[i] && e.input_applications==0,"callback order before Apply");
        }
        G11Driver.ReadyForGuard();body.value=false;
        try { G11Driver.Tick(null,63);throw new Exception("guard omitted"); }
        catch(InvalidOperationException error) { Require(error.Message.StartsWith("Stationary driver setup requires"),"guard exception changed"); }
        Require(G11Driver.report.rejected_bodies.Count==1 && G11Capture.rejections==1 && G11Driver.InputApplications==0 && G11Driver.canonical_reads==0,"guard decision changed");
        Require(G11BodyTrace.report.events[G11BodyTrace.report.events.Count-1].callback=="driver.Tick.exit","rejected Tick exit absent");
        // A later copied observer body cannot replace the retained source identity.
        var copied=new Rigidbody{key="probe",id=999,value=false};G11Capture.ObservedBody=copied;
        G11BodyTrace.Boundary("copied-probe");var probe=G11BodyTrace.report.events[G11BodyTrace.report.events.Count-1];
        Require(probe.retained.instance_id==465454 && probe.current.instance_id==465454 && probe.observer.instance_id==999 &&
            probe.current_is_retained && !probe.current_is_observer,"probe identity conflated");
        int previous=0;foreach(var e in G11BodyTrace.report.events) { Require(e.order>previous,"global order");previous=e.order; }
        int count=G11BodyTrace.report.events.Count;G11Capture.TraceOpen=false;body.forbidRead=true;
        foreach(bool value in new[]{false,true}) { int writes=body.writes;G11BodyTrace.Set(body,value,"ended",17,"call");
            Require(body.writes==writes+1 && body.value==value,"ended forward"); }
        body.rejected=original;try { G11BodyTrace.Set(body,false,"ended",0,"callvirt");throw new Exception("ended swallowed"); }
        catch(InvalidOperationException error) { Require(Object.ReferenceEquals(error,original),"ended original exception"); }
        G11BodyTrace.Boundary("ended");Require(G11BodyTrace.report.events.Count==count && G11Capture.failure==null,"ended observer access");
        G11Capture.TraceOpen=true;body.rejected=null;int beforeFailedObservation=body.writes;
        G11BodyTrace.Set(body,true,"observation failure",0,"callvirt");
        Require(body.writes==beforeFailedObservation+1 && body.value && G11Capture.failure!=null,"observation failure suppressed source setter");
        body.rejected=original;G11Capture.rejectReporting=true;
        try { G11BodyTrace.Set(body,false,"reporting failure",0,"call");throw new Exception("original swallowed by reporting"); }
        catch(InvalidOperationException error) { Require(Object.ReferenceEquals(error,original) && body.writes==beforeFailedObservation+2,"reporting replaced original exception"); }
        Console.WriteLine("G11_BODY_TRACE_PASS unbound selected unrelated exact_forward exceptions metadata order pre_apply guard probe ended");
    }
}
"""


class DrivingBodyTraceTests(unittest.TestCase):
    def test_actual_adapter_and_pre_apply_driver_boundaries(self):
        command = "Add-Type -TypeDefinition @'\n" + compiled_source() + "\n'@\n[BodyTraceTests]::Run()"
        result = subprocess.run(['pwsh', '-NoProfile', '-Command', command], capture_output=True, text=True, timeout=60)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn('G11_BODY_TRACE_PASS', result.stdout)


if __name__ == '__main__':
    unittest.main()
