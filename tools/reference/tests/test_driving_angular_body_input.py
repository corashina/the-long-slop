"""Critical copied-body input ownership checks. Fixtures are not native evidence."""
from copy import deepcopy
from pathlib import Path
import subprocess
import json
import tempfile
import unittest
from unittest.mock import patch
from tools.reference.tests.test_driving_angular import gate_fixture, STUBS
from tools.reference.driving_angular import validate_angular


class BodyInputTests(unittest.TestCase):
    def test_first_and_reused_input_gates(self):
        from tools.reference.driving_angular import validate_angular_body_input
        row, first = gate_fixture()
        canonical = deepcopy(first['angular']['body_input'])
        validate_angular_body_input(first, None, True)
        for corruption in ('missing', 'input', 'setup', 'initial', 'origin', 'origin_missing', 'origin_numeric', 'source'):
            bad = deepcopy(first)
            if corruption == 'missing': del bad['angular']['body_input']
            if corruption == 'input': bad['angular']['body_input']['mass'] += 1
            if corruption == 'setup': del bad['setup_state']['angular_body_input']
            if corruption == 'initial': bad['initial']['angular_body_input']['mass'] += 1
            if corruption == 'origin': bad['angular']['body_input_from_source'] = False
            if corruption == 'origin_missing': del bad['angular']['body_input_from_source']
            if corruption == 'origin_numeric': bad['angular']['body_input_from_source'] = 1
            if corruption == 'source': bad['angular']['source_body']['mass'] += 1
            with self.subTest(corruption=corruption), self.assertRaises(ValueError):
                validate_angular_body_input(bad, None, True)
        reused = deepcopy(first)
        reused['angular']['body_input_from_source'] = False
        reused['angular']['source_body']['mass'] += 10
        validate_angular_body_input(reused, canonical, False)
        self.assertEqual(validate_angular(reused, row)['steps'], 480)
        with self.assertRaises(ValueError): validate_angular_body_input(reused, None, False)
        for corruption in ('canonical', 'origin', 'setup', 'input'):
            bad = deepcopy(reused)
            expected = deepcopy(canonical)
            if corruption == 'canonical': expected['mass'] += 1
            if corruption == 'origin': bad['angular']['body_input_from_source'] = True
            if corruption == 'setup': bad['setup_state']['angular_body_input']['mass'] += 1
            if corruption == 'input': bad['angular']['body_input']['inertia_tensor_rotation']['w'] = .99
            with self.subTest(corruption=corruption), self.assertRaises(ValueError):
                validate_angular_body_input(bad, expected, False)

    def test_quaternion_arguments_stay_separate_from_readbacks(self):
        row, run = gate_fixture()
        run['angular']['source_body'] = deepcopy(run['angular']['source_body'])
        for record in (run['angular']['body_input'], run['setup_state']['angular_body_input'], run['initial']['angular_body_input'], run['angular']['source_body']):
            record['inertia_tensor_rotation']['w'] = .99999
        # Native readbacks remain normalized. Input retains the exact source argument.
        self.assertEqual(validate_angular(run, row)['steps'], 480)
        self.assertEqual(run['angular']['body_input']['inertia_tensor_rotation']['w'], .99999)
        self.assertEqual(run['angular']['copied_held_body']['inertia_tensor_rotation']['w'], 1.)
        run['angular']['initial_state']['body']['mass'] += .0001
        with self.assertRaisesRegex(ValueError, 'copied body setup changed'): validate_angular(run, row)

    def test_capture_and_verify_bind_repeat_input_origin(self):
        from tools.reference import driving_probe as probe
        row, first = gate_fixture()
        for mode in ('capture', 'verify'):
            for corruption in (None, 'missing', 'setup', 'input', 'first_origin', 'reused_origin', 'self_consistent_replacement'):
                with self.subTest(mode=mode, corruption=corruption), tempfile.TemporaryDirectory() as name:
                    project = Path(name).resolve()
                    output = project/'.cache/g11-driving/proof'
                    definitions = project/'scenarios.json'
                    definitions.write_text(json.dumps(dict(scenarios=[row])))
                    launched = []
                    def make_run(repeat):
                        run = deepcopy(first)
                        if repeat > 1:
                            run['angular']['body_input_from_source'] = False
                            run['angular']['source_body']['mass'] += 10
                        if repeat == 1 and corruption == 'first_origin': run['angular']['body_input_from_source'] = False
                        if repeat == 2:
                            if corruption == 'missing': del run['angular']['body_input']
                            if corruption == 'setup': del run['setup_state']['angular_body_input']
                            if corruption == 'input': run['angular']['body_input']['mass'] += 1
                            if corruption == 'reused_origin': run['angular']['body_input_from_source'] = True
                            if corruption == 'self_consistent_replacement':
                                for target in (run['angular']['body_input'], run['setup_state']['angular_body_input'], run['initial']['angular_body_input']): target['mass'] += 1
                        return run
                    def fake_player(arguments, **kwargs):
                        if arguments[arguments.index('-Mode')+1] == 'Run':
                            directory = Path(arguments[arguments.index('-RunRoot')+1])
                            supplied = json.loads((directory/'scenario.json').read_text())
                            launched.append(supplied)
                            repeat = len(launched)
                            self.assertEqual(supplied.get('has_setup_state', False), repeat>1)
                            if repeat>1: self.assertEqual(supplied['setup_state']['angular_body_input'], first['angular']['body_input'])
                            (directory/'runtime.json').write_text(json.dumps(make_run(repeat)))
                        return type('Result', (), dict(returncode=0))()
                    def check_evidence(output, directory, run, scenario): return validate_angular(run, scenario)
                    with patch.object(probe, 'PROJECT', project), patch.object(probe, 'physical_inputs', return_value={}), patch.object(probe, 'validate_evidence', side_effect=check_evidence):
                        if mode == 'capture':
                            with patch.object(probe, 'preflight', return_value=dict(powershell='pwsh',observer_revision={})), patch.object(probe.subprocess, 'run', side_effect=fake_player), patch.object(probe, 'verify', return_value=dict(runs=[1,2,3])):
                                if corruption:
                                    with self.assertRaises(ValueError): probe.capture(project, project, output, definitions, 3)
                                    self.assertEqual(len(launched), 1 if corruption=='first_origin' else 2)
                                    self.assertFalse((output/'receipt.json').exists())
                                else:
                                    self.assertEqual(len(probe.capture(project, project, output, definitions, 3)['runs']), 3)
                                    self.assertEqual(len(launched), 3)
                        else:
                            output.mkdir(parents=True)
                            (output/'scratch').mkdir()
                            (output/'request.json').write_text(json.dumps(dict(repeats=3,scenarios=[row],observer_revision={})))
                            (output/'preflight.json').write_text(json.dumps(dict(app_id=probe.APP_ID,build_id=probe.BUILD_ID,unity_version=probe.UNITY_VERSION,native_engine=dict(sha256='engine'),dependencies={'TheLongDrive_Data/Managed/UnityEngine.PhysicsModule.dll':'physics'})))
                            (output/'scratch/patch-receipt.json').write_text(json.dumps(dict(changed_methods=['checked'],observer_revision={},native_engine_sha256='engine',source_physics_module_sha256='physics')))
                            for repeat in range(1,4):
                                directory=output/row['id']/f'repeat-{repeat}'
                                directory.mkdir(parents=True)
                                (directory/'runtime.json').write_text(json.dumps(make_run(repeat)))
                            if corruption:
                                with self.assertRaises(ValueError): probe.verify(output)
                            else: self.assertEqual(len(probe.verify(output)['runs']), 3)

    def test_actual_setup_methods_have_one_property_owner(self):
        reference = Path(__file__).parents[1]
        capture = (reference/'driving_capture.cs').read_text(encoding='utf-8-sig')
        canonical = capture.split('static void ApplyCanonical(Initial initial)', 1)[1].split('car.Engine.rpm=', 1)[0].strip()[1:]
        load = capture.split('if(scenario.has_setup_state) inputPose=scenario.setup_state.input_pose;', 1)[1].split('if(scenario.setup.surface==', 1)[0].split('{', 1)[1]
        stubs = STUBS.split('public static class AngularChecks {', 1)[0]+'public static class G11ContactProbe {public static bool Active {get{return false;}} public static bool WheelEnabled {get{throw new System.Exception("Unexpected contact execution");}} public static void Arm(){throw new System.Exception("Unexpected contact execution");} public static void RetainInitial(){throw new System.Exception("Unexpected contact execution");}}'
        stubs = stubs.replace('public float mass=100,drag,', 'public int massWrites,comWrites,tensorWrites,rotationWrites; float m=100; public float mass {get{return m;}set{massWrites++;m=value;}} public float drag,')
        stubs = stubs.replace('public Vector3 centerOfMass,inertiaTensor,position;public Quaternion rotation,inertiaTensorRotation;', """public Vector3 position;public Quaternion rotation;
        Vector3 com,tensor;Quaternion tensorRotation;public bool normalizeRotation;
        public Vector3 centerOfMass {get{return com;}set{comWrites++;com=value;}}
        public Vector3 inertiaTensor {get{return tensor;}set{tensorWrites++;tensor=value;}}
        public Quaternion inertiaTensorRotation {get{return tensorRotation;}set{rotationWrites++;tensorRotation=value;if(normalizeRotation)tensorRotation.w=1;}}""")
        source = (reference/'driving_angular.cs').read_text(encoding='utf-8-sig') + stubs + SETUP_CHECKS.replace('CANONICAL', canonical).replace('LOAD', load)
        result = subprocess.run(['pwsh', '-NoProfile', '-Command', "Add-Type -TypeDefinition @'\n"+source+"\n'@\n[BodyInputChecks]::Run()"], capture_output=True, text=True, timeout=60)
        self.assertEqual(result.returncode, 0, result.stdout+result.stderr)
        self.assertIn('G11_BODY_INPUT_OWNER_PASS', result.stdout)


SETUP_CHECKS = r"""
public static class BodyInputChecks {
 static UnityEngine.Rigidbody body;
 class Initial {public float mass=1061.3505859375f;public UnityEngine.Vector3 center_of_mass=new UnityEngine.Vector3();}
 class Setup {public float load_multiplier=1f;}
 class Scenario {public bool has_setup_state;public Setup setup=new Setup();}
 static Scenario scenario=new Scenario();
 static void ApplyCanonical(Initial initial){CANONICAL}
 static void ApplyLoad(){if(!scenario.has_setup_state){LOAD}}
 static void Need(bool v,string message){if(!v)throw new Exception(message);}
 static void Reset(){typeof(G11AngularProbe).GetField("input",System.Reflection.BindingFlags.Static|System.Reflection.BindingFlags.NonPublic).SetValue(null,null);}
 public static void Run(){
  var original=new UnityEngine.Rigidbody{id=1,mass=1061.3505859375f,inertiaTensorRotation=new UnityEngine.Quaternion{w=.99999f}};
  var input=new G11AngularProbe.Input{source_wheel="level1:58684",rig=new G11AngularProbe.Rig(),events=new G11AngularProbe.Event[0]};
  var source=new UnityEngine.WheelCollider();var probe=new UnityEngine.GameObject("rig");
  body=new UnityEngine.Rigidbody{normalizeRotation=true};
  G11AngularProbe.Configure(input,source,original,probe,body,false,null);
  var canonical=G11AngularProbe.report.body_input;
  Need(G11AngularProbe.report.body_input_from_source && canonical.mass==original.mass,"first-source record");
  Need(canonical.inertia_tensor_rotation.w==.99999f && body.inertiaTensorRotation.w==1,"setter quaternion replaced by normalized readback");
  ApplyLoad();ApplyCanonical(new Initial());
  Need(body.massWrites==1 && body.comWrites==1 && body.tensorWrites==1 && body.rotationWrites==1,"first run has competing property owner");
  Reset();original.mass=1061.3509521484375f;original.centerOfMass=new UnityEngine.Vector3{x=9};original.inertiaTensor=new UnityEngine.Vector3{x=8};original.inertiaTensorRotation=new UnityEngine.Quaternion{w=.5f};scenario.has_setup_state=true;body=new UnityEngine.Rigidbody{normalizeRotation=true};
  G11AngularProbe.Configure(input,source,original,probe,body,true,canonical);
  Need(!G11AngularProbe.report.body_input_from_source && body.mass==1061.3505859375f && G11AngularProbe.report.source_body.mass==1061.3509521484375f,"canonical reuse changed or source readback hidden");
  Need(body.centerOfMass.x==0 && body.inertiaTensor.x==0 && G11AngularProbe.report.body_input.inertia_tensor_rotation.w==.99999f && G11AngularProbe.report.source_body.inertia_tensor_rotation.w==.5f,"all canonical setter arguments reused");
  ApplyLoad();ApplyCanonical(new Initial());
  Need(body.massWrites==1 && body.comWrites==1 && body.tensorWrites==1 && body.rotationWrites==1,"reused run has competing property owner");
  Reset();bool rejected=false;try{G11AngularProbe.Configure(input,source,original,probe,body,true,null);}catch(InvalidOperationException){rejected=true;}
  Need(rejected,"missing canonical input accepted");
  Reset();rejected=false;try{G11AngularProbe.Configure(input,source,original,probe,body,false,canonical);}catch(InvalidOperationException){rejected=true;}
  Need(rejected,"unexpected first canonical input accepted");
  Reset();scenario.has_setup_state=false;body=new UnityEngine.Rigidbody();body.mass=100;body.massWrites=0;ApplyLoad();ApplyCanonical(new Initial());
  Need(body.massWrites==2 && body.comWrites==1,"other scenario setup changed");
  Console.WriteLine("G11_BODY_INPUT_OWNER_PASS");
 }
}
"""
