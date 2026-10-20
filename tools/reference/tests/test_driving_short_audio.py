"""Execute actual source audio hooks across live and Unity-null observations."""
from copy import deepcopy
from pathlib import Path
import subprocess
import unittest

from tools.reference.driving import method
from tools.reference.driving_lifecycle import validate_audio
from tools.reference.tests.test_driving_lifecycle import lifecycle_fixture

ROOT = Path(__file__).resolve().parents[1]


def fixture(lifecycle=None):
    lifecycle = (ROOT / 'driving_lifecycle.cs').read_text(encoding='utf-8-sig') if lifecycle is None else lifecycle
    capture = (ROOT / 'driving_capture.cs').read_text(encoding='utf-8-sig')
    declarations = ('public class Mark', 'public class Scope', 'public class Audio', 'class Retained', 'class Owner',
                    'public static Mark At(', 'public static void Begin()', 'public static void Created(',
                    'public static void DestroyRequested(', 'public static void OwnersCompleted(', 'public static void OwnerBoundary(',
                    'static void ObserveScope(', 'public static void CapturedScope(', 'public static void PollAudio(',
                    'public static bool AudioReady()', 'public static bool Ready', 'public static void RequireHeld()')
    return STUBS + '\npublic static class G11Lifecycle {\n' + STATE + '\n'.join(
        method(lifecycle, name)[0] for name in declarations) + '}\npublic static class G11Capture {\n' + \
        'public static int InputRevision=7; public static void Fail(Exception error) { throw error; }\n' + \
        method(capture, 'public static void AudioDestroy(')[0] + '}\n' + CHECKS


STUBS = r'''
using System;
using System.Collections.Generic;
using System.Linq;
using UnityEngine;
namespace UnityEngine {
    public class Object {
        public int id; public bool dead; public static int calls; public static Object target; public static float delay;
        public int GetInstanceID() { if(dead) throw new Exception("dead native ID read"); return id; }
        public static bool operator ==(Object a,Object b) { return ReferenceEquals(a,b) || ((ReferenceEquals(a,null)||a.dead)&&(ReferenceEquals(b,null)||b.dead)); }
        public static bool operator !=(Object a,Object b) { return !(a==b); }
        public override bool Equals(object b) { return ReferenceEquals(this,b); }
        public override int GetHashCode() { return id; }
        public static void Destroy(Object value,float seconds) { calls++;target=value;delay=seconds; }
    }
    public class Component : Object { public string key; }
    public class Transform : Component { public GameObject obj; public Transform value;
        public Transform parent { get { if(obj.dead) throw new Exception("dead parent read"); G11AudioTests.ParentRead(); return value; } } }
    public class AudioClip : Object { public string name;public float length; }
    public class AudioSource : Component { public AudioClip clip; }
    public class Rigidbody : Component {}
    public class Collider : Component { public GameObject obj;public Rigidbody attachedRigidbody; }
    public class GameObject : Object {
        public Transform transform;public AudioSource source;public Collider collider;
        public T GetComponent<T>() where T:Component { return (T)(Component)(typeof(T)==typeof(AudioSource)?(Component)source:collider); }
    }
    public static class Time { public static int frameCount=11;public static float time=.5f; }
    public static class JsonUtility { public static string ToJson(object value) { return "stub"; } }
}
public class mainscript { public static mainscript M=new mainscript();public GameObject Audio=new GameObject{id=100}; }
public static class G11Geometry {
    public static string Key(Component value) { return value==null?"":value.key; }
    public static bool InCaptureScope(Collider c) { return c!=null && c.obj.transform.value!=null && c.obj.transform.value.key=="selected"; }
}
'''
STATE = r'''
    public class Update { public bool dependency_complete=true,rest_equal=true;public Mark exit=new Mark{order=1000}; }
    public class Report { public bool begun,armed,cohort_closed,owners_complete,ready;public Mark opened,owners_completed;
        public List<Mark> owner_callbacks=new List<Mark>();public string owner_completion_evidence;public List<Audio> audio=new List<Audio>(),new_after_close=new List<Audio>(); }
    static readonly List<Retained> retained=new List<Retained>();static readonly List<Owner> owners=new List<Owner>();
    public static Report report=new Report();static int order;static Update last=new Update();
    public static string Pending() { return "pending"; }
    public static void Reset() { report=new Report();retained.Clear();owners.Clear();order=0; }
'''
CHECKS = r'''
public static class G11AudioTests {
    static int reads;static bool checkRead;
    static void Require(bool condition,string label) { if(!condition) throw new Exception(label); }
    public static void ParentRead() {
        reads++;
        if(checkRead) {
            // This actual native read must occur after the creation mark, before the parent mark.
            var type=typeof(G11Lifecycle);var field=type.GetField("order",System.Reflection.BindingFlags.Static|System.Reflection.BindingFlags.NonPublic);
            Require((int)field.GetValue(null)==2,"live creation parent read must follow created mark");
            checkRead=false;
        }
    }
    static GameObject NewAudio(int id) {
        var clip=new AudioClip{id=id+3,name="blood01",length=.367f};
        var obj=new GameObject{id=id,source=new AudioSource{id=id+1,clip=clip},collider=new Collider{id=id+2}};
        obj.transform=new Transform{obj=obj};obj.collider.obj=obj;return obj;
    }
    static G11Lifecycle.Audio Create(GameObject obj) { G11Lifecycle.Created(obj,obj.source.clip);return G11Lifecycle.report.audio.Last(); }
    static void Die(GameObject obj) { obj.dead=true;obj.source.dead=true;obj.collider.dead=true; }
    static void Throws(Action action) { try { action(); } catch(InvalidOperationException) { return; } throw new Exception("missing strict rejection"); }
    public static void Run() {
        G11Lifecycle.Reset();G11Lifecycle.Begin();checkRead=true;reads=0;
        var outside=NewAudio(200);var a=Create(outside);
        Require(!checkRead && reads>=1,"missing live creation read");
        Require(a.created.order==2 && a.parent_observed!=null && a.parent_observed.order==3 &&
            a.parent_observed.callback=="mainscript.PlayClipAtPoint.Instantiate.parent","missing genuine creation mark");
        Require(a.created.frame==a.parent_observed.frame && a.created.time==a.parent_observed.time &&
            a.created.input_revision==a.parent_observed.input_revision && a.created.update_sequence==a.parent_observed.update_sequence,"creation clock changed");
        Require(a.creation_parent=="" && a.parent=="" && !a.required && a.membership.Count==1 && !a.membership[0].member,"outside creation scope changed");
        G11Capture.AudioDestroy(outside,.367f);
        Require(UnityEngine.Object.calls==1 && ReferenceEquals(UnityEngine.Object.target,outside) && UnityEngine.Object.delay==.367f &&
            a.object_id==200 && a.component_id==201 && a.collider_id==202 && a.clip_id==203 && a.destroy_delay==a.clip_length,"original identity/delay forwarding changed");
        var parentMark=a.parent_observed;Die(outside);int before=reads;G11Lifecycle.PollAudio("observer.LateUpdate");
        Require(reads==before && ReferenceEquals(a.parent_observed,parentMark) && a.removed.order>a.destruction_requested.order &&
            a.object_null && a.component_null && a.collider_null && G11Lifecycle.report.audio.Count==1 && !a.required,"short lived global observation lost");
        G11Lifecycle.OwnersCompleted(true,"actual completion");Require(!G11Lifecycle.AudioReady(),"required cohort absent");
        var member=NewAudio(300);var b=Create(member);var initial=b.parent_observed;
        member.transform.value=new Transform{key="selected"};G11Lifecycle.PollAudio("driver.update.exit");
        Require(b.required && b.scope_member && b.parent=="selected" && b.creation_parent=="" &&
            b.parent_observed.callback=="driver.update.exit.parent" && b.parent_observed.order>initial.order,"live membership poll lost");
        member.transform.value=new Transform{key="outside"};G11Lifecycle.PollAudio("observer.LateUpdate");
        Require(b.required && !b.scope_member && b.parent=="outside" && b.parent_observed.callback=="observer.LateUpdate.parent","required membership not sticky");
        G11Capture.AudioDestroy(member,.367f);member.dead=true;member.source.dead=true;G11Lifecycle.PollAudio("driver.update.exit");
        Require(!G11Lifecycle.AudioReady() && !G11Lifecycle.Ready,"required collider drain skipped");
        member.collider.dead=true;G11Lifecycle.PollAudio("driver.update.exit");Require(G11Lifecycle.AudioReady() && G11Lifecycle.Ready,"required drain not recognized");
        var geometry=NewAudio(400);var c=Create(geometry);G11Lifecycle.CapturedScope(new HashSet<Collider>{geometry.collider},"checkpoint");
        Require(c.required && c.membership.Last().observed.callback=="geometry.checkpoint.scope","geometry membership lost");
        G11Capture.AudioDestroy(geometry,.367f);Die(geometry);G11Lifecycle.PollAudio("driver.update.exit");
        G11Lifecycle.report.cohort_closed=true;G11Lifecycle.report.ready=true;G11Lifecycle.RequireHeld();
        var late=NewAudio(500);var d=Create(late);Require(d.created_after_close && !d.required,"outside after-close record lost");
        late.transform.value=new Transform{key="selected"};Throws(()=>G11Lifecycle.PollAudio("observer.LateUpdate"));
        Require(d.required && G11Lifecycle.report.new_after_close.Contains(d),"late membership rejection lost");Throws(()=>G11Lifecycle.RequireHeld());
        Console.WriteLine("G11_SHORT_AUDIO_PASS creation_read creation_mark short_lived global_records poll_parent scope sticky_required drain late_rejection forwarding");
    }
}
'''


def creation_audio():
    a = deepcopy(lifecycle_fixture()['lifecycle']['audio'][0])
    base = dict(a['created'], update_sequence=0)
    def mark(callback, order): return dict(base, callback=callback, order=order)
    a.update(creation_parent='', parent='', required=False, scope_member=False, created_after_close=True,
             created=mark('mainscript.PlayClipAtPoint.Instantiate', 40),
             parent_observed=mark('mainscript.PlayClipAtPoint.Instantiate.parent', 41),
             membership=[dict(observed=mark('mainscript.PlayClipAtPoint.Instantiate.scope', 42),
                              member=False, parent='', owner='', owner_id=0, owner_is_null=True)],
             destruction_requested=mark('mainscript.PlayClipAtPoint.Destroy(Object,clip.length)', 43),
             removed=mark('observer.LateUpdate.Unity-null', 45))
    return a


class DrivingShortAudioTests(unittest.TestCase):
    def test_actual_short_lived_audio_and_later_membership(self):
        command = "Add-Type -TypeDefinition @'\n" + fixture() + "\n'@\n[G11AudioTests]::Run()"
        result = subprocess.run(['pwsh', '-NoProfile', '-Command', command], capture_output=True, text=True, timeout=60)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn('G11_SHORT_AUDIO_PASS', result.stdout)

    def test_exact_creation_observation_validates(self):
        a = creation_audio()
        self.assertEqual(validate_audio(a, 1, 37), (False, 45))
        for callback in ('setup.crafted.entry', 'setup.slot.entry'):
            a['created']['callback']=callback
            self.assertEqual(validate_audio(a, 1, 37), (False, 45))
        # Later actual polls can change the last parent, without rewriting creation_parent.
        a.update(parent='selected', required=True, created_after_close=False)
        a['created']['order']=2
        a['destruction_requested']['order']=5
        a['parent_observed'].update(callback='driver.update.exit.parent', order=6)
        a['membership']=[dict(a['membership'][0], member=True, parent='selected', observed=dict(a['membership'][0]['observed'], order=7))]
        a['removed']['order']=10
        a['scope_member']=True
        self.assertEqual(validate_audio(a, 1, 37), (True, 10))

    def test_creation_observation_corruption_rejected(self):
        cases = ('absent', 'default', 'callback', 'created_callback', 'equal_order', 'gap_order', 'after_request',
                 'scope_order', 'scope_callback', 'creation_parent', 'scope_parent', 'null_parent', 'nonzero_sequence', 'missing_removal', 'removal_before_parent')
        for case in cases:
            a = creation_audio()
            if case=='absent': del a['parent_observed']
            if case=='default': a['parent_observed']=dict.fromkeys(a['parent_observed'], 0)
            if case=='callback': a['parent_observed']['callback']='invented.parent'
            if case=='created_callback': a['created']['callback']='invented.Instantiate'
            if case=='equal_order': a['parent_observed']['order']=40
            if case=='gap_order': a['parent_observed']['order']=42
            if case=='after_request': a['parent_observed']['order']=44
            if case=='scope_order': a['membership'][0]['observed']['order']=41
            if case=='scope_callback': a['membership'][0]['observed']['callback']='invented.scope'
            if case=='creation_parent': a['parent']='unobserved'
            if case=='scope_parent': a['membership'][0]['parent']='unobserved'
            if case=='null_parent': a.update(parent=None, creation_parent=None); a['membership'][0]['parent']=None
            if case=='nonzero_sequence':
                for stamp in (a['created'], a['parent_observed'], a['destruction_requested'], a['membership'][0]['observed']): stamp['update_sequence']=1
            if case=='missing_removal': del a['removed']
            if case=='removal_before_parent': a['removed']['order']=41
            with self.subTest(case=case), self.assertRaises(ValueError): validate_audio(a, 1, 37)
        for name in ('parent_observed', 'destruction_requested', 'scope'):
            for clock in ('frame', 'time', 'update_sequence', 'input_revision'):
                a=creation_audio()
                stamp=a['membership'][0]['observed'] if name=='scope' else a[name]
                stamp[clock]+=1
                with self.subTest(name=name, clock=clock), self.assertRaises(ValueError): validate_audio(a, 1, 37)
