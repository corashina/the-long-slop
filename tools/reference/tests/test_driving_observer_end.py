"""Execute observer completion adapters with strict source-component stubs."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

from tools.reference.driving import method

ROOT = Path(__file__).resolve().parents[1]


def fixture():
    capture = (ROOT / 'driving_capture.cs').read_text(encoding='utf-8-sig')
    geometry = (ROOT / 'driving_geometry.cs').read_text(encoding='utf-8-sig')
    adapters = '\n'.join(method(capture, name)[0] for name in (
        'public static void SetSourceActive(', 'public static void SetColliderEnabled(',
        'static void Flush()', 'static void FlushLocked()', 'static void Fail(Exception error)'))
    completion = method(capture, 'if(active && !finished && startFrame>=0')[0]
    forwarding = '\n'.join(method(geometry, name)[0] for name in (
        'public static void SetEnabled(', 'public static void SetActive('))
    return STUBS + '\npublic static class G11Geometry {\n' + GEOMETRY + forwarding + '}\n' + \
        'public static class G11Capture {\n' + CAPTURE + adapters + \
        'public static void Complete() {' + completion + '}\n}\n' + CHECKS


STUBS = r'''
using System;
using System.Collections.Generic;
using System.IO;
using System.Threading;
public class GameObject {
    public bool value, reject, rejectRead; public int calls, reads;
    public bool activeSelf { get { reads++; if(rejectRead) throw new InvalidOperationException("source read"); return value; } }
    public void SetActive(bool requested) { calls++; if(reject) throw new InvalidOperationException("source activation"); value=requested; }
}
public class Collider {
    public bool value, reject; public int writes;
    public bool enabled { get { return value; } set { writes++; if(reject) throw new InvalidOperationException("source collider"); this.value=value; } }
}
public class WheelCollider : Collider {
    public GameObject target=new GameObject(); public bool destroyed;
    public GameObject gameObject { get { if(destroyed) throw new InvalidOperationException("destroyed tracked wheel"); return target; } }
}
public static class Time { public static int frameCount=1; }
public static class Application {
    public static event Action<string,string,int> logMessageReceived;
    public static Action shutdown; public static int exit;
    public static void Quit(int code) { exit=code; shutdown(); }
    public static void Emit() { if(logMessageReceived!=null) logMessageReceived("","",0); }
}
public static class Debug { public static void Log(string text) {} public static void LogException(Exception error) {} }
public static class JsonUtility { public static string ToJson(object value,bool pretty) { return "{}"; } }
'''
GEOMETRY = r'''
    public class Activation { public string wheel,caller; public int frame; public bool previous,requested,observed; }
    public class Hold { public bool intended; }
    public static bool holding=true;
    public static WheelCollider[] nativeWheels;
    public static readonly Dictionary<WheelCollider,Hold> held=new Dictionary<WheelCollider,Hold>();
    public static Activation[] activation_events=new Activation[4096]; public static int activation_count;
    static string Key(WheelCollider wheel) { return "tracked-wheel"; }
'''
CAPTURE = r'''
    public class Sample {}
    public class Result { public bool completed; public string error; public int physics_steps;
        public G11Geometry.Activation[] wheel_activation_events; public Sample[] samples; }
    public class Scenario { public int duration_frames=1; }
    static Scenario scenario=new Scenario(); public static Result result;
    static bool active,finished; static int startFrame,used; static Sample[] buffer;
    static readonly object bufferGate=new object(); static Timer timeoutTimer=null;
    static string Output { get { return Environment.GetEnvironmentVariable("G11_OBSERVER_TEST_ROOT"); } }
    static void CaptureLog(string message,string stack,int type) {}
    public static void Reset() { active=true;finished=false;startFrame=0;used=1;buffer=new Sample[1];result=new Result(); }
    public static void Reject() { Fail(new InvalidOperationException("capture failure")); }
'''
CHECKS = r'''
public static class G11ObserverEndTests {
    static void Require(bool condition,string label) { if(!condition) throw new Exception(label); }
    static void Throws(Action callback,string message) {
        try { callback(); } catch(InvalidOperationException error) { Require(error.Message==message,"exception changed"); return; }
        throw new Exception("exception swallowed: "+message);
    }
    public static void Run() {
        G11Capture.Reset();
        var wheel=new WheelCollider(); G11Geometry.nativeWheels=new[]{wheel};
        var hold=new G11Geometry.Hold(); G11Geometry.held.Add(wheel,hold);
        G11Capture.SetColliderEnabled(wheel,true);
        Require(hold.intended && !wheel.value && wheel.writes==1,"active hold changed");
        var plain=new Collider(); G11Capture.SetColliderEnabled(plain,true);
        Require(plain.value && plain.writes==1,"active plain collider forwarding");
        G11Capture.SetSourceActive(wheel.target,true,"source-callback");
        var evidence=G11Geometry.activation_events[0];
        Require(wheel.target.calls==1 && evidence.requested && !evidence.previous && evidence.observed &&
            evidence.caller=="source-callback" && evidence.wheel=="tracked-wheel","active evidence changed");
        wheel.destroyed=true;
        var unrelated=new GameObject();
        Throws(()=>G11Capture.SetSourceActive(unrelated,false,"active"),"destroyed tracked wheel");
        Require(unrelated.calls==1,"active original call missing");
        foreach(bool failed in new[]{false,true}) {
            G11Capture.Reset();
            int count=G11Geometry.activation_count;
            Application.shutdown=delegate {
                foreach(bool value in new[]{false,true}) {
                    var target=new GameObject{rejectRead=true};
                    G11Capture.SetSourceActive(target,value,"ended");
                    Require(target.calls==1 && target.value==value && target.reads==0,"ended activation forwarding");
                    int before=wheel.writes; G11Capture.SetColliderEnabled(wheel,value);
                    Require(wheel.writes==before+1 && wheel.value==value && hold.intended,"ended collider forwarding");
                }
                var rejected=new GameObject{reject=true};
                Throws(()=>G11Capture.SetSourceActive(rejected,true,"ended"),"source activation");
                Require(rejected.calls==1,"ended rejected original call count");
                var collider=new Collider{reject=true};
                Throws(()=>G11Capture.SetColliderEnabled(collider,true),"source collider");
                Require(collider.writes==1,"ended rejected collider call count");
                Require(G11Geometry.activation_count==count,"ended observation added");
            };
            if(failed) G11Capture.Reject(); else G11Capture.Complete();
            Require(G11Capture.result.completed==!failed && Application.exit==(failed?2:0),"completion status changed");
            Require(G11Capture.result.samples.Length==1 && G11Capture.result.wheel_activation_events.Length==count,"flush evidence changed");
            Require(File.ReadAllText(Path.Combine(Environment.GetEnvironmentVariable("G11_OBSERVER_TEST_ROOT"),"completion.json"))
                .Contains("\"completed\":"+(failed?"false":"true")),"completion marker changed");
        }
        Console.WriteLine("G11_OBSERVER_END_PASS active_hold active_evidence active_failure success_end failed_end original_exceptions");
    }
}
'''


class DrivingObserverEndTests(unittest.TestCase):
    def test_active_and_ended_original_forwarding(self):
        with tempfile.TemporaryDirectory() as directory:
            env = dict(os.environ, G11_OBSERVER_TEST_ROOT=directory)
            command = "Add-Type -TypeDefinition @'\n" + fixture() + "\n'@\n[G11ObserverEndTests]::Run()"
            result = subprocess.run(['pwsh', '-NoProfile', '-Command', command],
                                    capture_output=True, text=True, env=env, timeout=60)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn('G11_OBSERVER_END_PASS', result.stdout)