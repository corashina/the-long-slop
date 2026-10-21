"""Critical world-speed clones. Fabricated traces and recording stubs are not native evidence."""
from copy import deepcopy
from pathlib import Path
import hashlib
import json
import struct
import unittest
from unittest.mock import patch
from tools.reference.driving_angular import angular_rows, validate_angular, validate_angular_schedule
from tools.reference.tests import test_driving_angular as angular_tests
from tools.reference.tests import test_driving_angular_body_input as body_tests
from tools.reference.tests import test_driving_angular_zero_brake as zero_tests

# Literal inputs from the frozen speed brief. Labels belong to later analysis only.
CLONES = (
    ('angular-full-tire-excitation-world-x-positive-below-five', 0, (4.999000072479248, 0., 0.), ('409ff7cf','00000000','00000000')),
    ('angular-full-tire-excitation-world-x-positive-above-five', 0, (5.000999927520752, 0., 0.), ('40a00831','00000000','00000000')),
    ('angular-full-tire-excitation-world-x-negative-below-five', 0, (-4.999000072479248, 0., 0.), ('c09ff7cf','00000000','00000000')),
    ('angular-full-tire-excitation-world-x-negative-above-five', 0, (-5.000999927520752, 0., 0.), ('c0a00831','00000000','00000000')),
    ('angular-full-tire-excitation-world-z-positive-ten', 0, (0., 0., 10.), ('00000000','00000000','41200000')),
    ('angular-bare-rim-excitation-zero-body-speed', 4, (0., 0., 0.), ('00000000','00000000','00000000')),
    ('angular-hub-excitation-zero-body-speed', 6, (0., 0., 0.), ('00000000','00000000','00000000')),
    ('angular-auxiliary-helper-motor-coast-zero-body-speed', 9, (0., 0., 0.), ('00000000','00000000','00000000')),
    ('angular-full-tire-brake-excitation-zero-body-speed', 11, (0., 0., 0.), ('00000000','00000000','00000000')),
)


IDENTIFICATION_CLONES = (('angular-identification-full-tire-x-below-five-adjacent', 'angular-full-tire-excitation', (4.999999523162842, 0.0, 0.0), ('409fffff', '00000000', '00000000')), ('angular-identification-full-tire-x-five', 'angular-full-tire-excitation', (5.0, 0.0, 0.0), ('40a00000', '00000000', '00000000')), ('angular-identification-full-tire-x-above-five-adjacent', 'angular-full-tire-excitation', (5.000000476837158, 0.0, 0.0), ('40a00001', '00000000', '00000000')), ('angular-identification-full-tire-x-five-z-positive-ten', 'angular-full-tire-excitation', (5.0, 0.0, 10.0), ('40a00000', '00000000', '41200000')), ('angular-identification-full-tire-x-five-z-negative-ten', 'angular-full-tire-excitation', (5.0, 0.0, -10.0), ('40a00000', '00000000', 'c1200000')), ('angular-identification-worn-tire-zero-body-speed', 'angular-worn-tire-excitation', (0.0, 0.0, 0.0), ('00000000', '00000000', '00000000')), ('angular-identification-full-tire-zero-low-brake-start', 'angular-full-tire-brake-excitation-zero-body-speed', (0.0, 0.0, 0.0), ('00000000', '00000000', '00000000')), ('angular-identification-full-tire-zero-high-brake-start', 'angular-full-tire-brake-excitation-zero-body-speed', (0.0, 0.0, 0.0), ('00000000', '00000000', '00000000')))
ALL_CLONES = CLONES+IDENTIFICATION_CLONES

def clone_fixture(index, spinning=False):
    identity, parent, values, _ = ALL_CLONES[index]
    if isinstance(parent,str):
        original=next(r for r in angular_rows() if r['id']==parent)
        if original['angular']['purpose']=='brake-excitation':row,run=zero_tests.identification_brake_fixture(original,spinning)
        else:row,run=zero_tests.new_fixture(next(i for i,r in enumerate(angular_rows()) if r['id']==parent),spinning)
    else:row,run=zero_tests.new_fixture(parent,spinning)
    row = deepcopy(row)
    row['id'] = identity
    velocity = dict(zip(('x','y','z'), values))
    row['setup']['velocity_mps'] = deepcopy(velocity)
    row['angular']['rig']['velocity_mps'] = deepcopy(velocity)
    run['angular']['declared'] = deepcopy(row['angular'])
    for name in ('source_body','copied_constructor_body','copied_held_body'):
        run['angular'][name]['velocity'] = deepcopy(velocity)
    run['angular']['initial_state']['body']['velocity'] = deepcopy(velocity)
    for sample in run['samples']:
        sample['angular_state']['body']['velocity'] = deepcopy(velocity)
    if identity=='angular-identification-full-tire-zero-high-brake-start':
        row['angular']['events'][0]['brake_nm']=1800.
        run['angular']['declared']=deepcopy(row['angular'])
        run['angular']['assignments'][0]['brake_nm']=1800.
        for sample in run['samples']:
            if sample['step']<=60:sample['wheels'][0]['brake_torque']=1800.
    return angular_tests.add_word_fixture(row,run)


class AngularSpeedTests(unittest.TestCase):
    def test_exact_nine_clones_and_original_document(self):
        rows = angular_rows()
        self.assertEqual(len(rows), 29)
        committed = json.loads((Path(__file__).parents[3]/'tests/fixtures/driving/angular-scenarios.json').read_text())
        self.assertEqual(committed['scenarios'], rows)
        original = dict(committed, scenarios=committed['scenarios'][:12])
        self.assertEqual(hashlib.sha256(json.dumps(original,sort_keys=True,separators=(',',':')).encode()).hexdigest(),
                         'aee81f342939e4df4075e0872b772b4b90111884fadc7b6cd6dd55959ced3c13')
        for row, (identity, parent, values, bits) in zip(rows[12:21], CLONES):
            expected = deepcopy(rows[parent])
            expected['id'] = identity
            velocity = dict(zip(('x','y','z'), values))
            expected['setup']['velocity_mps'] = deepcopy(velocity)
            expected['angular']['rig']['velocity_mps'] = deepcopy(velocity)
            self.assertEqual(row, expected)
            for actual in (row['setup']['velocity_mps'], row['angular']['rig']['velocity_mps']):
                self.assertEqual(tuple(struct.pack('>f',actual[a]).hex() for a in ('x','y','z')),bits)
            validate_angular_schedule(row)
        self.assertEqual(hashlib.sha256(json.dumps(dict(committed,scenarios=rows[:21]),sort_keys=True,separators=(',',':')).encode()).hexdigest(),'b2b5dd69d5eec8e988f7a1243a498ad1ee58dc76171c0e7787d4f7414ce0f1ab')
        self.assertEqual(hashlib.sha256(json.dumps(rows[21:],sort_keys=True,separators=(',',':')).encode()).hexdigest(),'7842163b871acbad8a6dca7baa2281d0b7edec3d2464f04806969aaf1e89ba2e')
        for row,(identity,parent,values,bits) in zip(rows[21:],IDENTIFICATION_CLONES):
            original=next(r for r in rows[:21] if r['id']==parent)
            expected=deepcopy(original);expected['id']=identity
            expected['setup']['velocity_mps']=dict(zip(('x','y','z'),values));expected['angular']['rig']['velocity_mps']=dict(zip(('x','y','z'),values))
            if identity=='angular-identification-full-tire-zero-high-brake-start':expected['angular']['events'][0]['brake_nm']=1800.
            self.assertEqual(row,expected)
            self.assertEqual(tuple(struct.pack('>f',row['setup']['velocity_mps'][a]).hex() for a in ('x','y','z')),bits)
            validate_angular_schedule(row)

    def test_new_rows_require_initial_and_consuming_activity(self):
        for index, (identity, _, _, _) in enumerate(ALL_CLONES):
            row, run = clone_fixture(index)
            with self.subTest(row=identity):
                result = validate_angular(run,row)
                self.assertEqual((result['steps'],result['events']), (480,8))
            for boundary in ('initial',0,1):
                for case in ('velocity','sleeping','kinematic','enabled','active_self','active_hierarchy','missing_activity'):
                    # Only the initial velocity is fixed. Consuming velocity may drift.
                    if case=='velocity' and boundary!='initial': continue
                    bad = deepcopy(run)
                    state = bad['angular']['initial_state'] if boundary=='initial' else bad['samples'][boundary]['angular_state']
                    if case=='velocity':
                        state['body']['velocity']['z'] += 1.
                        angular_tests.add_word_fixture(row,bad)
                    elif case in ('sleeping','kinematic'): state['body'][case] = True
                    elif case=='missing_activity': del state['active_hierarchy']
                    else: state[case] = False
                    with self.subTest(row=identity,boundary=boundary,case=case), self.assertRaises(ValueError):
                        validate_angular(bad,row)
            for sample in run['samples']:
                sample['angular_state']['body']['velocity'] = dict(x=1.,y=2.,z=3.)
            self.assertEqual(validate_angular(run,row)['steps'],480)

    def test_clones_reject_changed_schedule_and_missing_terminal_zero(self):
        for index, (identity, _, _, _) in enumerate(ALL_CLONES):
            row, run = clone_fixture(index)
            for target in ('setup','rig','schedule','assignments','solve'):
                bad_row, bad = deepcopy(row), deepcopy(run)
                if target=='setup': bad_row['setup']['velocity_mps']['x'] += 1
                if target=='rig': bad_row['angular']['rig']['velocity_mps']['z'] += 1
                if target=='schedule': bad_row['angular']['events'][2]['motor_nm'] += 1
                if target=='assignments': bad['angular']['assignments'].pop()
                if target=='solve': bad['samples'].pop()
                with self.subTest(row=identity,target=target), self.assertRaises(ValueError): validate_angular(bad,bad_row)
        for spinning in (False,True):
            row, run = clone_fixture(8,spinning)
            self.assertEqual(validate_angular(run,row)['held_zero_steps'],[179,180,419,420])
            for step in (179,180,419,420):
                for offset in (0,1):
                    bad = deepcopy(run)
                    bad['samples'][2*(step-1)+offset]['wheels'][0]['rpm'] = .000001
                    with self.subTest(spinning=spinning,step=step,offset=offset), self.assertRaisesRegex(ValueError,'terminal zero'):
                        validate_angular(bad,row)

    def test_capture_and_verify_body_inputs_for_each_clone(self):
        for index, (identity, _, _, _) in enumerate(ALL_CLONES):
            with self.subTest(row=identity), patch.object(body_tests,'gate_fixture',side_effect=lambda index=index:clone_fixture(index)):
                checks = body_tests.BodyInputTests()
                checks.test_first_and_reused_input_gates()
                checks.test_capture_and_verify_bind_repeat_input_origin()

    def test_actual_arm_inputs_and_full_schedules_for_each_clone(self):
        reference = Path(__file__).parents[1]
        helper = (reference/'driving_angular.cs').read_text(encoding='utf-8-sig')
        stubs = angular_tests.STUBS.split('public static class AngularChecks {',1)[0]
        template = zero_tests.SCHEDULE_CHECK
        template = template.replace('new UnityEngine.Vector3{x=VELOCITY}', 'new UnityEngine.Vector3{VECTOR}')
        template = template.replace('held.velocity.x==VELOCITY','VECTOR_EQUAL')
        template = template.replace('source_wheel="level1:58684",support="full-tire"','source_wheel="SOURCE",support="SUPPORT"')
        template = template.replace('constraints=116','constraints=CONSTRAINTS')
        for index, (identity, _, _, _) in enumerate(ALL_CLONES):
            row, _ = clone_fixture(index)
            spec = row['angular']
            velocity = spec['rig']['velocity_mps']
            vector = ','.join(a+'='+str(velocity[a])+'f' for a in ('x','y','z'))
            equal = ' && '.join('held.velocity.'+a+'=='+str(velocity[a])+'f' for a in ('x','y','z'))
            declared = ','.join('new G11AngularProbe.Event{step='+str(e['step'])+',phase="'+e['phase']+'",motor_nm='+str(e['motor_nm'])+'f,brake_nm='+str(e['brake_nm'])+'f}' for e in spec['events'])
            source_stubs = stubs.replace('return "level1:58684";', 'return "'+spec['source_wheel']+'";')
            source = helper+source_stubs+template.replace('EVENTS',declared).replace('VECTOR_EQUAL',equal).replace('VECTOR',vector).replace('SOURCE',spec['source_wheel']).replace('SUPPORT',spec['support']).replace('CONSTRAINTS',str(spec['rig']['constraints']))
            with self.subTest(row=identity):
                zero_tests.ZeroBrakeTests().compile(source,'ScheduleChecks','G11_FULL_SCHEDULE_PASS')

    def test_angular_default_producer_boundary_contract(self):
        from tools.reference.driving_angular import angular_candidate_view, binary32_word
        for index in range(9,len(ALL_CLONES)):
            row,run=clone_fixture(index)
            original=deepcopy(run)
            self.assertEqual(validate_angular(run,row)['steps'],480)
            view=angular_candidate_view(run)
            self.assertEqual(len(view['samples']),480)
            self.assertTrue(all(sample['phase']=='physics.exit' for sample in view['samples']))
            self.assertEqual(view['samples'][-1]['rpm'],struct.unpack('<f',struct.pack('<f',run['samples'][-1]['wheels'][0]['rpm']))[0])
            self.assertEqual(run,original)
            cases=('missing_schema','old_schema','bool_schema','parser_adjacent','parser_zero','initial_velocity','initial_zero',
                   'profile','swapped_profile','dt','motor','brake','rpm_nonfinite','sample_wheel','sample_sequence','sample_step','sample_phase',
                   'word_bool','word_float','word_string','word_low','word_high','unknown_shape')
            for case in cases:
                bad=deepcopy(run);report=bad['angular'];sample=bad['samples'][0]['angular_words']
                if case=='missing_schema':del report['word_schema_version']
                if case=='old_schema':report['word_schema_version']=0
                if case=='bool_schema':report['word_schema_version']=True
                if case=='parser_adjacent':report['parsed_words']['setup_velocity']['x']+=1
                if case=='parser_zero':report['parsed_words']['rig_angular_velocity']['y']=-2147483648
                if case=='initial_velocity':report['initial_words']['velocity']['x']+=1
                if case=='initial_zero':report['initial_words']['rpm']=-2147483648
                if case=='profile':report['copied_profile_words']['mass']+=1
                if case=='swapped_profile':
                    words=report['copied_profile_words'];words['mass'],words['radius']=words['radius'],words['mass']
                if case=='dt':sample['fixed_dt']+=1
                if case=='motor':sample['motor_nm']+=1
                if case=='brake':sample['brake_nm']=-2147483648
                if case=='rpm_nonfinite':sample['rpm']=2139095040
                if case=='sample_wheel':sample['wheel_id']+=1
                if case=='sample_sequence':sample['sequence']+=1
                if case=='sample_step':sample['step']+=1
                if case=='sample_phase':sample['phase']='physics.exit'
                if case.startswith('word_'):
                    sample['rpm']={'word_bool':True,'word_float':0.,'word_string':'0','word_low':-2147483649,'word_high':2147483648}[case]
                if case=='unknown_shape':sample['extra']=0
                with self.subTest(row=row['id'],case=case),self.assertRaisesRegex(ValueError,'Angular word'):
                    validate_angular(bad,row)

    def test_actual_word_union_and_capture_wiring(self):
        reference=Path(__file__).parents[1]
        helper=(reference/'driving_angular.cs').read_text(encoding='utf-8-sig')
        capture=(reference/'driving_capture.cs').read_text(encoding='utf-8-sig')
        parsed='scenario=JsonUtility.FromJson<Scenario>'+capture.split('                scenario=JsonUtility.FromJson<Scenario>',1)[1].split('                result.scenario=',1)[0]
        initial=capture.split('        if(G11AngularProbe.Active)\n            for(int index=0;index<wheels.Length;index++)\n                G11AngularProbe.ObserveInitial',1)[1].split('        return initial;',1)[0]
        initial='if(G11AngularProbe.Active)\n for(int index=0;index<wheels.Length;index++)\n G11AngularProbe.ObserveInitial'+initial
        sample='if(G11AngularProbe.Active) {'+capture.split('static void RecordLocked(string phase)',1)[1].split('        if(G11AngularProbe.Active) {',1)[1].split('        if(G11ContactProbe.Active) G11ContactProbe.Read(sample.contact_state);',1)[0]
        stubs=angular_tests.STUBS.split('public static class AngularChecks {',1)[0]
        stubs=stubs.replace('public float mass=100,drag,','public int massReads,massWrites,comWrites,tensorWrites,rotationWrites;public List<string> propertyOrder=new List<string>();float m=100;public float mass {get{massReads++;return m;}set{massWrites++;propertyOrder.Add("mass");m=value;}}public float drag,')
        stubs=stubs.replace('public Vector3 centerOfMass,inertiaTensor,position;public Quaternion rotation,inertiaTensorRotation;', 'public Vector3 position;public Quaternion rotation;Vector3 com,tensor;Quaternion tensorRotation;public Vector3 centerOfMass {get{return com;}set{comWrites++;propertyOrder.Add("center");com=value;}}public Vector3 inertiaTensor {get{return tensor;}set{tensorWrites++;propertyOrder.Add("tensor");tensor=value;}}public Quaternion inertiaTensorRotation {get{return tensorRotation;}set{rotationWrites++;propertyOrder.Add("rotation");tensorRotation=value;if(id!=1)tensorRotation.w=1;}}')
        stubs=stubs.replace('public float radius=.3f,mass=20,wheelDampingRate=.25f,suspensionDistance=.3f,forceAppPointDistance,sprungMass=123,rpm=-7,steerAngle;', 'public int massReads,radiusReads,dampingReads,rpmReads;float m=20,r=.3f,d=.25f;public float mass{get{massReads++;return m;}set{m=value;}}public float radius{get{radiusReads++;return r;}set{r=value;}}public float wheelDampingRate{get{dampingReads++;return d;}set{d=value;}}public float rpm{get{rpmReads++;return -7;}}public float suspensionDistance=.3f,forceAppPointDistance,sprungMass=123,steerAngle;')
        source='using System.IO;\n'+helper+stubs+WORD_CHECKS.replace('PARSED',parsed).replace('INITIAL',initial).replace('SAMPLE',sample)
        zero_tests.ZeroBrakeTests().compile(source,'WordChecks','G11_ANGULAR_EXACT_WORD_PASS')

    def test_frozen_declaration_words_and_measured_negative_zero(self):
        from tools.reference.driving_angular import angular_candidate_view
        for index in range(9,len(ALL_CLONES)):
            row,run=clone_fixture(index)
            for target in ('setup','rig','angular_velocity','motor','brake'):
                for value in (-0.0,False,0):
                    bad=deepcopy(row)
                    if target=='setup':bad['setup']['velocity_mps']['y']=value
                    if target=='rig':bad['angular']['rig']['velocity_mps']['y']=value
                    if target=='angular_velocity':bad['angular']['rig']['angular_velocity_rad_s']['y']=value
                    if target in ('motor','brake'):
                        key=target+'_nm';next(e for e in bad['angular']['events'] if e[key]==0)[key]=value
                    with self.subTest(row=row['id'],target=target,value=repr(value)),self.assertRaisesRegex(ValueError,'Angular word'):
                        validate_angular_schedule(bad)
            if row['angular']['purpose']=='brake-excitation':
                for sample in run['samples']:
                    if sample['step'] in (179,180,419,420):
                        sample['wheels'][0]['rpm']=-0.0
                        sample['angular_words']['rpm']=-2147483648
                self.assertEqual(validate_angular(run,row)['held_zero_steps'],[179,180,419,420])
                exits=[s for s in angular_candidate_view(run)['samples'] if s['step'] in (179,180,419,420) and s['phase']=='physics.exit']
                self.assertTrue(all(struct.pack('<f',s['rpm'])==b'\x00\x00\x00\x80' for s in exits))


WORD_CHECKS = '\npublic static class WordChecks {\n class Setup {public UnityEngine.Vector3 velocity_mps;}\n class InitialInput {public G11AngularProbe.BodyInput angular_body_input;}\n class Scenario {public string probe_kind;public Setup setup;public G11AngularProbe.Input angular;public bool has_setup_state;public InitialInput setup_state;public bool contact_body_input_from_source=false;public G11AngularProbe.BodyInput contact_body_input=null;}\n class Result {public object contact_configure_readout;}static Result result=new Result();\n static int ObservedPhysicsStep=3,InputRevision=2;static Scenario scenario;static Scenario supplied;static string Output;\n struct SavedWheel {public float rpm,motor_torque,brake_torque;}\n class SavedInitial {public float fixed_dt;public SavedWheel[] wheel_state;}\n class SavedSample {public G11AngularProbe.State angular_state=new G11AngularProbe.State();public G11AngularProbe.SampleWords angular_words;public int sequence,step;public string phase;public float fixed_dt;public SavedWheel[] wheels;}\n static UnityEngine.WheelCollider[] wheels;\n static void Parsed(){PARSED}\n static void Initial(SavedInitial initial){INITIAL}\n static void Sample(SavedSample sample){SAMPLE}\n static void Need(bool value,string message){if(!value)throw new Exception(message);}\n public static object Deserialize(){return supplied;}\n public static void Run(){\n  Need(System.Runtime.InteropServices.Marshal.SizeOf(typeof(G11AngularProbe.FloatWord))==4,"union size");\n  Need(System.Runtime.InteropServices.Marshal.OffsetOf(typeof(G11AngularProbe.FloatWord),"value").ToInt32()==0 && System.Runtime.InteropServices.Marshal.OffsetOf(typeof(G11AngularProbe.FloatWord),"word").ToInt32()==0,"union offsets");\n  float minusZero=new G11AngularProbe.FloatWord{word=int.MinValue}.value;\n  Need(G11AngularProbe.Word(minusZero)==int.MinValue && G11AngularProbe.Word(0f)==0 && G11AngularProbe.Word(-1f)==-1082130432,"signed words");\n  var original=new UnityEngine.Rigidbody{id=1,mass=101.25f,centerOfMass=new UnityEngine.Vector3{x=1,y=2,z=3},inertiaTensor=new UnityEngine.Vector3{x=4,y=5,z=6},inertiaTensorRotation=new UnityEngine.Quaternion{x=.1f,y=.2f,z=.3f,w=.99999f}};\n  var held=new UnityEngine.Rigidbody();\n  var source=new UnityEngine.WheelCollider{mass=21.25f,radius=.375f,wheelDampingRate=.5f};\n  var input=new G11AngularProbe.Input{source_wheel="level1:58684",rig=new G11AngularProbe.Rig{velocity_mps=new UnityEngine.Vector3{x=new G11AngularProbe.FloatWord{word=1084227583}.value,z=-10},angular_velocity_rad_s=new UnityEngine.Vector3{z=2}},events=new G11AngularProbe.Event[0]};\n  Output=System.IO.Path.Combine(System.IO.Path.GetTempPath(),"g11-angular-words-"+Guid.NewGuid().ToString("N"));\n  System.IO.Directory.CreateDirectory(Output);System.IO.File.WriteAllText(System.IO.Path.Combine(Output,"scenario.json"),"stub actual FromJson");\n  supplied=new Scenario{probe_kind="wheel_angular",setup=new Setup{velocity_mps=input.rig.velocity_mps},angular=input,has_setup_state=false,setup_state=new InitialInput{angular_body_input=new G11AngularProbe.BodyInput{mass=999}}};\n  Parsed();Need(UnityEngine.JsonUtility.calls==1,"actual FromJson call count");\n  original.massReads=held.massReads=source.massReads=source.radiusReads=source.dampingReads=0;\n  var selected=G11AngularProbe.Configure(input,source,original,new UnityEngine.GameObject("rig"),held,false,null);\n  var report=G11AngularProbe.report;\n  Need(original.massReads==1 && held.massReads==2 && held.massWrites==1 && held.comWrites==1 && held.tensorWrites==1 && held.rotationWrites==1,"body getter/setter counts");\n  Need(string.Join(",",held.propertyOrder)=="mass,center,tensor,rotation","body setter order");\n  Need(source.massReads==2 && source.radiusReads==2 && source.dampingReads==2 && selected.massReads==1 && selected.radiusReads==1 && selected.dampingReads==1,"profile getter counts");\n  Need(report.parsed_words.observed && !report.parsed_words.canonical_supplied && !report.parsed_words.canonical_is_null && report.parsed_words.canonical_arguments.mass==0,"source placeholder became canonical");\n  Need(report.parsed_words.setup_velocity.x==1084227583 && report.parsed_words.rig_velocity.z==G11AngularProbe.Word(-10f) && report.parsed_words.rig_angular_velocity.z==G11AngularProbe.Word(2f),"parsed field wiring");\n  Need(report.argument_words.mass==G11AngularProbe.Word(101.25f) && report.argument_words.center_of_mass.y==G11AngularProbe.Word(2f) && report.argument_words.inertia_tensor.z==G11AngularProbe.Word(6f),"setter field wiring");\n  Need(report.argument_words.inertia_tensor_rotation.w==G11AngularProbe.Word(.99999f) && report.copied_held_body.inertia_tensor_rotation.w==1f,"argument/readback quaternion separation");\n  Need(report.source_profile_words.mass==G11AngularProbe.Word(21.25f) && report.copied_profile_words.radius==G11AngularProbe.Word(.375f) && report.copied_profile_words.damping==G11AngularProbe.Word(.5f),"profile word wiring");\n  held.isKinematic=false;selected.enabled=true;G11AngularProbe.Arm();G11AngularProbe.RetainInitial();\n  var other=new UnityEngine.WheelCollider();\n  wheels=new[]{other,selected};var initial=new SavedInitial{fixed_dt=.02f,wheel_state=new[]{new SavedWheel{rpm=7},new SavedWheel{rpm=0}}};\n  int reads=selected.massReads+selected.radiusReads+selected.dampingReads+selected.rpmReads+held.massReads;\n  Initial(initial);\n  Need(reads==selected.massReads+selected.radiusReads+selected.dampingReads+selected.rpmReads+held.massReads,"initial added native getter");\n  Need(report.initial_words.observed && report.initial_words.fixed_dt==G11AngularProbe.Word(.02f) && report.initial_words.rpm==0 && report.initial_words.wheel_id==report.initial_state.wheel_id && report.initial_words.velocity.x==1084227583 && report.initial_words.angular_velocity.z==G11AngularProbe.Word(2f),"saved Initial selected field wiring");\n  var sample=new SavedSample{sequence=71,step=23,phase="physics.exit",fixed_dt=.02f,wheels=new[]{new SavedWheel{rpm=999,motor_torque=998,brake_torque=997},new SavedWheel{rpm=-17.25f,motor_torque=31.5f,brake_torque=4.25f}}};\n  int massReads=selected.massReads,radiusReads=selected.radiusReads,dampingReads=selected.dampingReads,rpmReads=selected.rpmReads,bodyReads=held.massReads;\n  Sample(sample);\n  Need(selected.massReads==massReads+1 && selected.radiusReads==radiusReads+1 && selected.dampingReads==dampingReads+1 && selected.rpmReads==rpmReads && held.massReads==bodyReads+1,"sample added native getter");\n  Need(sample.angular_words.observed && sample.angular_words.sequence==71 && sample.angular_words.step==23 && sample.angular_words.phase=="physics.exit" && sample.angular_words.wheel_id==sample.angular_state.wheel_id,"sample owner links");\n  Need(sample.angular_words.fixed_dt==G11AngularProbe.Word(.02f) && sample.angular_words.motor_nm==G11AngularProbe.Word(31.5f) && sample.angular_words.brake_nm==G11AngularProbe.Word(4.25f) && sample.angular_words.rpm==G11AngularProbe.Word(-17.25f),"saved sample selected field wiring");\n  typeof(G11AngularProbe).GetField("input",System.Reflection.BindingFlags.Static|System.Reflection.BindingFlags.NonPublic).SetValue(null,null);\n  supplied.has_setup_state=true;supplied.setup_state.angular_body_input=report.body_input;Parsed();\n  var reused=new UnityEngine.Rigidbody();G11AngularProbe.Configure(input,source,original,new UnityEngine.GameObject("reused"),reused,true,report.body_input);\n  Need(G11AngularProbe.report.parsed_words.canonical_supplied && !G11AngularProbe.report.body_input_from_source && G11AngularProbe.report.parsed_words.canonical_arguments.inertia_tensor_rotation.w==report.argument_words.inertia_tensor_rotation.w,"actual reused parser/setter words");\n  System.IO.Directory.Delete(Output,true);\n  Console.WriteLine("G11_ANGULAR_EXACT_WORD_PASS");\n }\n}\nnamespace UnityEngine {\n public static class Time {public static int frameCount=5;}\n public static class JsonUtility {public static int calls;public static T FromJson<T>(string text){if(text!="stub actual FromJson")throw new Exception("parser input differs");calls++;return (T)WordChecks.Deserialize();}}\n}\npublic static class G11ContactProbe {\n public struct SetupMark {public int frame,physics_step,input_revision;}\n public static object readout;\n public static void ObserveAfterFromJson(bool source,G11AngularProbe.BodyInput input,SetupMark mark){throw new Exception("unrelated contact dispatch");}\n}\n'
