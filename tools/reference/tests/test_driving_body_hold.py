"""Execute actual hold ownership and reject incomplete native hold evidence."""
from copy import deepcopy
from pathlib import Path
import subprocess
import unittest

from tools.reference.driving import method
from tools.reference.driving_body_hold import validate_body_hold
from tools.reference.tests.test_driving_body_trace import compiled_source, CHECKS


def add_hold_fixture(run):
    isolated = run.get('isolated_wheel_probe', False)
    original = dict(key='original', instance_id=1, present=True, kinematic=True,
                    velocity=dict(x=0., y=0., z=0.), angular_velocity=dict(x=0., y=0., z=0.))
    copied = dict(original, key='copied', instance_id=2)
    events = []
    oh = ch = False
    def add(kind, callback='', step=0, target=None, requested=False, call=0):
        events.append(dict(kind=kind, callback=callback, physics_step=step, frame=step, time=float(step),
            order=len(events)+1, original_hold=oh, copy_hold=ch, retained=deepcopy(original),
            copied=deepcopy(copied) if isolated else {}, current=deepcopy(original), current_is_retained=True,
            target=deepcopy(target or original), requested=requested, call_id=call))
    oh = True
    add('hold.started', 'observer.original.hold')
    if isolated:
        copied['kinematic'] = False
        add('copy.created', target=copied)
        copied['kinematic'] = ch = True
        add('hold.started', 'observer.copy.hold', target=copied)
    add('hold.solve', step=1)
    for phase in ('prepared-held', 'backlog-drained-held', 'before-dynamic'):
        add('hold.boundary', phase, step=1)
    target = copied if isolated else original
    add('hold.release.before', step=1, target=target)
    if isolated: ch = False
    else: oh = False
    target['kinematic'] = False
    add('hold.release.after', step=1, target=target)
    if isolated:
        add('setter.before', step=1, requested=False, call=1)
        original['kinematic'] = False
        add('setter.after', step=1, requested=False, call=1)
        add('hold.restore.before', step=1, requested=True, call=1)
        original['kinematic'] = True
        add('hold.restore.after', step=1, requested=True, call=1)
    add('hold.solve', step=2)
    add('hold.end', step=2)
    run['body_trace'] = dict(retained_body='original', retained_instance_id=1,
        copied_body='copied' if isolated else None, copied_instance_id=2 if isolated else 0,
        isolated=isolated, ended=True, hold_failure=None, events=events)
    return run


HOLD_CHECKS = r"""
public static class HoldTests {
    static void Need(bool value,string label) { if(!value) throw new Exception(label); }
    static void Reject(Action action,string label) { try { action();throw new Exception(label); } catch(InvalidOperationException) {} }
    static void Ready() { foreach(var phase in new[]{"prepared-held","backlog-drained-held","before-dynamic"}) G11BodyTrace.SetupBoundary(phase); }
    public static void Run(bool isolated,bool violation,bool sourceThrows) {
        var original=new Rigidbody{key="original",id=1,value=false};var car=new carscript{RB=original};
        G11Capture.ObservedBody=original;G11BodyTrace.Bind(car);G11Driver.Bind(car);
        G11BodyTrace.HoldOriginal();Need(original.value && original.writes==1,"original hold");
        var error=new InvalidOperationException("exact original exception");
        if(violation) original.value=false;
        if(sourceThrows) { original.rejected=error;original.forbidRead=true; }
        int writes=original.writes;
        try { G11BodyTrace.Set(original,false,"source",570,"callvirt");Need(!sourceThrows,"source exception swallowed"); }
        catch(InvalidOperationException caught) { Need(sourceThrows && Object.ReferenceEquals(caught,error),"source exception replaced"); }
        if(sourceThrows) { Need(original.writes==writes+1 && G11Capture.failure!=null,"source exception with failed observation");return; }
        Need(original.writes==writes+2 && original.value,"source false and observer true once");
        var events=G11BodyTrace.report.events;int n=events.Count;
        Need(events[n-3].kind=="setter.after" && !events[n-3].target.kinematic &&
             events[n-2].kind=="hold.restore.before" && events[n-1].kind=="hold.restore.after" && events[n-1].target.kinematic,"source result preserved");
        if(violation) { Need(G11Capture.failure!=null && G11BodyTrace.report.hold_failure!=null,"earlier violation hidden");Reject(G11BodyTrace.BeforeSolve,"violated solve accepted");return; }
        var other=new Rigidbody{forbidRead=true};G11BodyTrace.Set(other,false,"unrelated while held",1,"call");
        Need(other.writes==1 && other.reads==0,"unrelated held-body inspection");
        Reject(()=>G11BodyTrace.Bind(car),"ownership rebound");
        writes=original.writes;G11BodyTrace.Set(original,true,"source true",1,"call");Need(original.writes==writes+1,"true restoration added");
        Reject(()=>G11BodyTrace.ReleaseMeasurement(original),"premature release");
        Rigidbody copied=null;
        if(isolated) {
            copied=G11Capture.CreateHeldProbeBody(new GameObject());
            Need(copied.value && copied.writes==1,"constructor hold");G11Capture.ObservedBody=copied;
            G11Capture.ObservedPhysicsStep++;G11BodyTrace.BeforeSolve();Need(copied.writes==1,"solve repaired copy");
        }
        Ready();
        if(isolated) Reject(()=>G11BodyTrace.ReleaseMeasurement(original),"wrong role release");
        G11BodyTrace.ReleaseMeasurement(copied??original);G11Capture.IsArmed=true;
        writes=original.writes;G11BodyTrace.Set(original,false,"active source",570,"callvirt");
        Need(original.writes==writes+(isolated?2:1) && original.value==isolated,"active original policy");
        if(isolated) Need(!copied.value && copied.writes==2,"copied release");
        G11BodyTrace.BeforeSolve();G11BodyTrace.End();Need(G11BodyTrace.report.ended,"end missing");
        G11Capture.TraceOpen=false;original.forbidRead=true;writes=original.writes;
        G11BodyTrace.Set(original,false,"ended",1,"call");Need(original.writes==writes+1,"ended policy");
        Console.WriteLine("G11_HOLD_PASS");
    }
}
"""


class DrivingBodyHoldTests(unittest.TestCase):
    def test_actual_policy_and_copy_constructor(self):
        source = compiled_source().replace(CHECKS, HOLD_CHECKS)
        capture = (Path(__file__).parents[1] / 'driving_capture.cs').read_text(encoding='utf-8-sig')
        constructor = method(capture, 'static Rigidbody CreateHeldProbeBody(')[0].replace('static Rigidbody', 'public static Rigidbody')
        source = source.replace('public static class G11Capture {', 'public static class G11Capture {\n' + constructor)
        source += '\npublic class GameObject { public T AddComponent<T>() where T:new() { return new T(); } }'
        for isolated, violation, throws in ((False, False, False), (True, False, False), (True, True, False), (True, True, True)):
            with self.subTest(isolated=isolated, violation=violation, throws=throws):
                args = ','.join('$true' if v else '$false' for v in (isolated, violation, throws))
                command = "Add-Type -TypeDefinition @'\n" + source + "\n'@\n[HoldTests]::Run(" + args + ')'
                result = subprocess.run(['pwsh', '-NoProfile', '-Command', command], capture_output=True, text=True, timeout=60)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertNotIn('.velocity=', source)
        self.assertNotIn('.angularVelocity=', source)
        self.assertLess(capture.index('var rb=CreateHeldProbeBody(probe)'), capture.index('rb.mass=body.mass'))

    def test_hold_gate(self):
        for isolated in (False, True):
            run = add_hold_fixture(dict(isolated_wheel_probe=isolated))
            self.assertEqual(validate_body_hold(run)['solves'], 2)
        for change in ('missing', 'dynamic_original', 'dynamic_copy', 'moving_copy', 'missing_solve', 'wrong_release',
                       'missing_restoration', 'prior_violation', 'identity', 'missing_copy_hold', 'premature', 'missing_end'):
            with self.subTest(change=change):
                run = add_hold_fixture(dict(isolated_wheel_probe=True))
                report = run['body_trace'];events=report['events']
                def row(kind): return next(e for e in events if e['kind']==kind)
                if change == 'missing': run.pop('body_trace')
                if change == 'dynamic_original': row('hold.solve')['retained']['kinematic']=False
                if change == 'dynamic_copy': row('hold.solve')['copied']['kinematic']=False
                if change == 'moving_copy': row('hold.solve')['copied']['velocity']['y']=-.1
                if change == 'missing_solve': events.remove(row('hold.solve'))
                if change == 'wrong_release': row('hold.release.after')['target']['instance_id']=1
                if change == 'missing_restoration': events.remove(row('hold.restore.after'))
                if change == 'prior_violation': report['hold_failure']='prior unknown release'
                if change == 'identity': row('hold.solve')['current_is_retained']=False
                if change == 'missing_copy_hold': events.remove(next(e for e in events if e['callback']=='observer.copy.hold'))
                if change == 'premature': events.remove(next(e for e in events if e['callback']=='backlog-drained-held'))
                if change == 'missing_end': events.pop()
                for index,event in enumerate(events): event['order']=index+1
                with self.assertRaises(ValueError): validate_body_hold(run)
