"""Critical copied-body input ownership checks. Fixtures are not native evidence."""
from copy import deepcopy
from pathlib import Path
import subprocess
import json
import tempfile
import unittest
from unittest.mock import patch
from tools.reference.tests.test_driving_angular import gate_fixture, STUBS
from tools.reference.tests import test_driving_angular as angular_tests
from tools.reference.driving_angular import validate_angular


TRANSPORT_CASES = (None,'missing','setup','input','first_origin','reused_origin','self_consistent_replacement','supplied','consuming_word')

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
        angular_tests.add_word_fixture(row,reused)
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
        angular_tests.add_word_fixture(row,run)
        # Native readbacks remain normalized. Input retains the exact source argument.
        self.assertEqual(validate_angular(run, row)['steps'], 480)
        self.assertEqual(run['angular']['body_input']['inertia_tensor_rotation']['w'], .99999)
        self.assertEqual(run['angular']['copied_held_body']['inertia_tensor_rotation']['w'], 1.)
        run['angular']['initial_state']['body']['mass'] += .0001
        with self.assertRaisesRegex(ValueError, 'copied body setup changed'): validate_angular(run, row)

    def test_capture_and_verify_bind_repeat_input_origin(self):
        for mode in ('capture','verify'):
            for corruption in TRANSPORT_CASES:
                with self.subTest(mode=mode,corruption=corruption):
                    self.transport_case(mode,corruption)

    def transport_case(self,mode,corruption):
        from contextlib import ExitStack
        from tools.reference import driving_probe as probe
        from tools.reference.driving_angular import body_word_values, body_words
        row,first=gate_fixture()
        # A producer decimal that does not reconstruct the actual setter argument.
        exact=1061.3505859375
        raw_mass=1061.351
        for target in (first['angular']['source_body'],first['angular']['body_input'],first['setup_state']['angular_body_input'],
                       first['initial']['angular_body_input'],first['angular']['copied_held_body'],first['angular']['initial_state']['body']):
            target['mass']=raw_mass
        for sample in first['samples']:sample['angular_state']['body']['mass']=raw_mass
        angular_tests.add_word_fixture(row,first)
        actual_words=deepcopy(first['angular']['argument_words'])
        from tools.reference.driving_angular import binary32_word
        actual_words['mass']=binary32_word(exact)
        first['angular']['argument_words']=deepcopy(actual_words)
        first['angular']['source_argument_words']=deepcopy(actual_words)
        # Keep the actual raw producer numbers and their number lexemes.
        original=deepcopy(first)
        with tempfile.TemporaryDirectory() as name:
            project=Path(name).resolve();output=project/'.cache/g11-driving/proof'
            definitions=project/'scenarios.json'
            definitions.write_text(json.dumps(dict(scenarios=[row])))
            launched=[]
            canonical=deepcopy(first['setup_state']);canonical['angular_body_input']=body_word_values(actual_words)
            def scenario_input(repeat):
                value=deepcopy(row)
                if repeat>1:value.update(has_setup_state=True,setup_state=deepcopy(canonical))
                if repeat==2 and corruption=='supplied':
                    value['setup_state']['angular_body_input']['mass']=exact+1.
                if repeat==2 and corruption=='supplied_zero':value['setup']['velocity_mps']['y']=-0.0
                return value
            def make_run(repeat):
                run=deepcopy(first)
                run.update(runtime='original-unity-player',scenario=row['id'],probe_kind='wheel_angular',completed=True,
                    menu_ready=True,lada_ready=True,control_capture_active=True,isolated_wheel_probe=True,physics_steps=480)
                if repeat>1:
                    run['angular']['body_input_from_source']=False
                    run['angular']['source_body']['mass']+=10.
                    run['angular']['parsed_words'].update(canonical_supplied=True,canonical_is_null=False,canonical_arguments=deepcopy(actual_words))
                # Populate unrelated lower capture fields. The angular dispatch stays real.
                prefix=[]
                for phase in sorted(probe.PHASES-{'physics.entry','physics.exit'}):
                    value=deepcopy(run['samples'][0]);value.update(phase=phase,frame=0,step=0);prefix.append(value)
                run['samples']=prefix+run['samples']
                for sequence,sample in enumerate(run['samples'],1):
                    sample['sequence']=sequence
                    for key in ('rpm','gear','clutch','engine_sequence','car_sequence','rpm_sequence','engine_output_nm','engine_brake_nm','average_rpm','torque_divisor'):
                        sample.setdefault(key,0)
                    for key in ('position','rotation','velocity','angular_velocity'):sample.setdefault(key,{})
                    for wheel in sample['wheels']:
                        for key in ('radius','suspension_distance','suspension_offset_m','pose','rotation','contact_point','contact_normal','force_n','forward_slip','sideways_slip'):
                            wheel.setdefault(key,0)
                    if sample['phase'] in ('physics.entry','physics.exit'):
                        sample['angular_words']['sequence']=sequence
                if repeat==1 and corruption=='first_origin':run['angular']['body_input_from_source']=False
                if repeat==2:
                    if corruption=='missing':del run['angular']['body_input']
                    if corruption=='setup':del run['setup_state']['angular_body_input']
                    if corruption=='input':run['angular']['body_input']['mass']+=1.
                    if corruption=='reused_origin':run['angular']['body_input_from_source']=True
                    if corruption=='self_consistent_replacement':
                        for target in (run['angular']['body_input'],run['setup_state']['angular_body_input'],run['initial']['angular_body_input']):target['mass']+=1.
                        run['angular']['argument_words']['mass']=binary32_word(exact+1.)
                        run['angular']['parsed_words']['canonical_arguments']['mass']=binary32_word(exact+1.)
                    if corruption=='consuming_word':run['samples'][-1]['angular_words']['fixed_dt']+=1
                return run
            def write_run(directory,repeat):
                directory.mkdir(parents=True,exist_ok=True)
                supplied=scenario_input(repeat)
                if mode=='verify' or corruption in ('supplied','supplied_zero'):(directory/'scenario.json').write_text(json.dumps(supplied))
                run=make_run(repeat)
                (directory/'runtime.json').write_text(json.dumps(run))
                (directory/'completion.json').write_text(json.dumps(dict(completed=True,sample_count=len(run['samples']),physics_steps=480)))
                (directory/'process.json').write_text(json.dumps(dict(pid=123,executable=str(output/'scratch/game/TheLongDrive.exe'))))
                (directory/'exit.json').write_text(json.dumps(dict(exit_code=0)))
                (directory/'unity.log').write_text('G11_NATIVE_CAPTURE_FLUSHED completed=True\n')
            def fake_player(arguments,**kwargs):
                kind=arguments[arguments.index('-Mode')+1]
                if kind=='Setup':
                    (output/'scratch').mkdir()
                    (output/'scratch/patch-receipt.json').write_text(json.dumps(dict(changed_methods=['checked'],observer_revision={},native_engine_sha256='engine',source_physics_module_sha256='physics')))
                if kind=='Run':
                    directory=Path(arguments[arguments.index('-RunRoot')+1]);repeat=len(launched)+1
                    supplied=json.loads((directory/'scenario.json').read_text())
                    self.assertEqual(supplied,scenario_input(repeat) if corruption!='supplied' else dict(row,has_setup_state=True,setup_state=canonical) if repeat>1 else row)
                    if repeat>1:self.assertEqual(supplied['setup_state']['angular_body_input']['mass'],exact)
                    launched.append(supplied);write_run(directory,repeat)
                return type('Result',(),dict(returncode=0))()
            pre=dict(powershell='pwsh',observer_revision={},app_id=probe.APP_ID,build_id=probe.BUILD_ID,unity_version=probe.UNITY_VERSION,native_engine=dict(sha256='engine'),dependencies={'TheLongDrive_Data/Managed/UnityEngine.PhysicsModule.dll':'physics'})
            with ExitStack() as patches:
                patches.enter_context(patch.object(probe,'PROJECT',project))
                for key,value in (('physical_inputs',{}),('validate_timing',{}),('validate_controls',{}),('validate_body_hold',{}),
                                  ('validate_geometry',{}),('validate_driver',{}),('validate_lifecycle',{})):
                    patches.enter_context(patch.object(probe,key,return_value=value))
                if mode=='capture':
                    patches.enter_context(patch.object(probe,'preflight',return_value=pre))
                    patches.enter_context(patch.object(probe.subprocess,'run',side_effect=fake_player))
                    invoke=lambda:probe.capture(project,project,output,definitions,3)
                else:
                    output.mkdir(parents=True);(output/'scratch').mkdir()
                    (output/'request.json').write_text(json.dumps(dict(repeats=3,scenarios=[row],observer_revision={})))
                    (output/'preflight.json').write_text(json.dumps(pre))
                    (output/'scratch/patch-receipt.json').write_text(json.dumps(dict(changed_methods=['checked'],observer_revision={},native_engine_sha256='engine',source_physics_module_sha256='physics')))
                    for repeat in range(1,4):write_run(output/row['id']/f'repeat-{repeat}',repeat)
                    invoke=lambda:probe.verify(output)
                if corruption:
                    with self.assertRaisesRegex(ValueError,'Angular word'):
                        invoke()
                else:
                    result=invoke()
                    self.assertEqual(len(result['runs']),3)
                    self.assertTrue(all(r['angular']['steps']==480 for r in result['runs']))
                    first_runtime=(output/row['id']/'repeat-1/runtime.json').read_text()
                    self.assertIn('"mass": 1061.351',first_runtime)
                    self.assertEqual(json.loads(first_runtime)['angular']['body_input']['mass'],raw_mass)
            self.assertEqual(first,original)

    def test_supplied_declaration_words_are_checked(self):
        for mode in ('capture','verify'):
            with self.subTest(mode=mode):self.transport_case(mode,'supplied_zero')

    def test_production_word_dispatch_is_required(self):
        import inspect
        from tools.reference import driving_probe as probe
        text=inspect.getsource(probe.validate_run)
        original="timing['angular'] = validate_angular(run, scenario)"
        self.assertIn(original,text)
        namespace=dict(probe.__dict__)
        exec(compile(text.replace(original,"timing['angular'] = {}"),'<angular dispatch removed>','exec'),namespace)
        with patch.object(probe,'validate_run',namespace['validate_run']):
            for mode in ('capture','verify'):
                with self.subTest(mode=mode),self.assertRaises(AssertionError):
                    self.transport_case(mode,'consuming_word')

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
