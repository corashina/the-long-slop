"""Critical zero-body-speed and brake gates. Fabricated traces are not native evidence."""
from copy import deepcopy
from pathlib import Path
import hashlib
import json
import subprocess
import unittest
import sys
from unittest.mock import patch
from tools.reference.driving_angular import angular_rows, validate_angular_schedule, validate_angular
from tools.reference.tests import test_driving_angular as angular_tests
from tools.reference.tests import test_driving_angular_body_input as body_tests

BRAKE_EVENTS = [
    (1, 'positive-brake-start', 600., 1.),
    (61, 'positive-unbraked-release', 600., 0.),
    (121, 'brake-stop-after-positive-release', 0., 1800.),
    (181, 'positive-brake-restart', 600., 1.),
    (241, 'negative-brake-reversal', -600., 1.),
    (301, 'negative-unbraked-release', -600., 0.),
    (361, 'brake-stop-after-negative-release', 0., 1800.),
    (421, 'negative-brake-restart', -600., 1.),
]
OBSERVATIONS = [
    'one enabled active contact-free native wheel and awake dynamic body',
    'exact consuming physics solves and actual motor and brake assignments',
    'observed initial zero RPM and terminal strong-brake zero intervals',
    'signed spin in both unbraked release phases',
    'observed low-brake response from qualified zero boundaries',
]
BASE_FIXTURE = angular_tests.gate_fixture


def new_fixture(index=10, spinning=False):
    row, run = BASE_FIXTURE(index)
    run['angular']['initial_state'].update(active_self=True, active_hierarchy=True)
    run['initial']['wheel_state'] = [dict(rpm=0., contact=False, motor_torque=0., brake_torque=0.)]
    if index == 11:
        previous = 0.
        for step in range(1, 481):
            event = (step-1)//60
            tick = (step-1)%60
            # Deliberate gate cases, not values from a response equation or native capture.
            values = (4., 9.+tick, 0. if tick>=3 else 3.,
                      5., -6., -10.-tick, 0. if tick>=3 else -2., -7.)
            rpm = values[event] if spinning or event not in (0,3,4,7) else 0.
            run['samples'][2*(step-1)]['wheels'][0]['rpm'] = previous
            run['samples'][2*(step-1)+1]['wheels'][0]['rpm'] = rpm
            previous = rpm
    return angular_tests.add_word_fixture(row,run)



IDENTIFICATION_BRAKE_IDS = ('angular-identification-full-tire-zero-low-brake-start',
                            'angular-identification-full-tire-zero-high-brake-start')

ORIGINAL_NEW_FIXTURE = new_fixture


def identification_brake_fixture(row,spinning=False):
    parent=next(i for i,row in enumerate(angular_rows()) if row['id']=='angular-full-tire-brake-excitation')
    _,run=ORIGINAL_NEW_FIXTURE(parent,spinning)
    run['angular']['declared']=deepcopy(row['angular'])
    velocity=deepcopy(row['setup']['velocity_mps'])
    run['angular']['initial_state']['body']['velocity']=velocity
    run['angular']['assignments']=[dict(event_revision=i+1,**event) for i,event in enumerate(row['angular']['events'])]
    for sample in run['samples']:
        sample['angular_state']['body']['velocity']=deepcopy(velocity)
        event=next(e for e in reversed(row['angular']['events']) if e['step']<=sample['step'])
        sample['wheels'][0].update(motor_torque=event['motor_nm'],brake_torque=event['brake_nm'])
    return angular_tests.add_word_fixture(row,run)

class ZeroBrakeTests(unittest.TestCase):
    def test_exact_rows_and_unchanged_prefix(self):
        rows = angular_rows()
        self.assertEqual(len(rows), 29)
        self.assertEqual(hashlib.sha256(json.dumps(rows[:10],sort_keys=True,separators=(',',':')).encode()).hexdigest(), 'e6e7765656c9ca1b945ce7f61f859e1356db9c4e51f044f86fe93802c6cf19b5')
        stored = json.loads((Path(__file__).parents[3]/'tests/fixtures/driving/angular-scenarios.json').read_text())
        self.assertEqual({k:v for k,v in stored.items() if k!='scenarios'}, {'schema_version': 1})
        self.assertEqual(stored['scenarios'], rows)
        zero = deepcopy(rows[0])
        zero['id'] = 'angular-full-tire-excitation-zero-body-speed'
        zero['setup']['velocity_mps'] = dict(x=0.,y=0.,z=0.)
        zero['angular']['rig']['velocity_mps'] = dict(x=0.,y=0.,z=0.)
        self.assertEqual(rows[10], zero)
        brake = deepcopy(rows[0])
        brake['id'] = 'angular-full-tire-brake-excitation'
        brake['angular']['purpose'] = 'brake-excitation'
        brake['angular']['events'] = [dict(step=s,phase=p,motor_nm=m,brake_nm=b) for s,p,m,b in BRAKE_EVENTS]
        brake['required_observations'] = OBSERVATIONS
        self.assertEqual(rows[11], brake)
        for row in rows[10:]: validate_angular_schedule(row)

    def test_new_rows_reject_changed_inputs(self):
        for row in angular_rows()[10:]:
            for case in ('step','phase','motor_nm','brake_nm','missing','repeated','frames','steps','source','rig','setup'):
                with self.subTest(row=row['id'], case=case):
                    bad = deepcopy(row)
                    if case in ('step','motor_nm','brake_nm'): bad['angular']['events'][2][case] += 1
                    if case == 'phase': bad['angular']['events'][2]['phase'] = 'other'
                    if case == 'missing': bad['angular']['events'].pop()
                    if case == 'repeated': bad['angular']['events'].append(deepcopy(bad['angular']['events'][0]))
                    if case == 'frames': bad['duration_frames'] += 1
                    if case == 'steps': bad['angular']['duration_steps'] += 1
                    if case == 'source': bad['angular']['source_wheel'] = 'wrong-source'
                    if case == 'rig': bad['angular']['rig']['velocity_mps']['x'] += 1
                    if case == 'setup': bad['setup']['velocity_mps']['x'] += 1
                    with self.assertRaises(ValueError): validate_angular_schedule(bad)

    def test_zero_speed_retains_existing_evidence_and_tensor_gates(self):
        with patch.object(angular_tests, 'gate_fixture', side_effect=lambda index=0:new_fixture(10)):
            for name in ('test_actual_evidence_gate_and_rejections', 'test_frozen_tensor_changes_at_first_solve_exit', 'test_reject_frozen_tensor_timing_and_partial_zeros'):
                getattr(angular_tests.AngularTests(), name)()
        row, run = new_fixture(10)
        for sample_index in (240,241):
            bad = deepcopy(run)
            sample = bad['samples'][sample_index]
            sample['angular_state']['body']['sleeping'] = True
            with self.assertRaisesRegex(ValueError, 'step 121 '+sample['phase']): validate_angular(bad,row)

    def test_initial_active_awake_gates_for_both_rows(self):
        for index in (10,11):
            row, run = new_fixture(index)
            for case in ('sleeping','kinematic','enabled','active_self','active_hierarchy','velocity'):
                bad = deepcopy(run)
                initial = bad['angular']['initial_state']
                if case in ('sleeping','kinematic'): initial['body'][case] = True
                elif case == 'velocity':
                    initial['body']['velocity']['x'] += 1
                    angular_tests.add_word_fixture(row,bad)
                else: initial[case] = False
                with self.subTest(index=index, case=case), self.assertRaises(ValueError): validate_angular(bad,row)

    def test_brake_accepts_locked_spinning_and_opposite_sign_low_brake(self):
        for row in (row for row in angular_rows() if row['id'] in IDENTIFICATION_BRAKE_IDS):
            with patch.object(sys.modules[__name__],'new_fixture',side_effect=lambda index=11,spinning=False,row=row:identification_brake_fixture(row,spinning)):
                self.check_brake_accepts_locked_spinning_and_opposite_sign_low_brake()
        self.check_brake_accepts_locked_spinning_and_opposite_sign_low_brake()

    def check_brake_accepts_locked_spinning_and_opposite_sign_low_brake(self):
        for spinning in (False, True):
            row, run = new_fixture(11, spinning)
            result = validate_angular(run,row)
            self.assertEqual(result['held_zero_steps'], [179,180,419,420])
            response = result['low_brake_response']
            self.assertEqual([r['phase'] for r in response], [BRAKE_EVENTS[i][1] for i in (0,3,4,7)])
            self.assertEqual([r['step'] for phase in response for r in phase['solves']], list(range(1,61))+list(range(181,301))+list(range(421,481)))
            self.assertEqual(response[0]['zero_to_zero_steps'], [] if spinning else list(range(1,61)))
            self.assertEqual(response[0]['zero_to_nonzero_steps'], [1] if spinning else [])
            self.assertEqual(response[2]['solves'][0]['entry_rpm'], 5. if spinning else 0.)
            self.assertEqual(result['strong_brake_response'][0]['first_observed_zero']['step'],124)
            self.assertEqual(result['strong_brake_response'][1]['first_observed_zero']['step'],364)
        row, run = new_fixture(11)
        run['samples'][1]['wheels'][0]['rpm'] = -3.
        result = validate_angular(run,row)
        self.assertEqual(result['low_brake_response'][0]['solves'][0]['exit_rpm'], -3.)

    def test_brake_rejects_missing_zero_and_release_gates(self):
        for row in (row for row in angular_rows() if row['id'] in IDENTIFICATION_BRAKE_IDS):
            with patch.object(sys.modules[__name__],'new_fixture',side_effect=lambda index=11,spinning=False,row=row:identification_brake_fixture(row,spinning)):
                self.check_brake_rejects_missing_zero_and_release_gates()
        self.check_brake_rejects_missing_zero_and_release_gates()

    def check_brake_rejects_missing_zero_and_release_gates(self):
        row, run = new_fixture(11)
        for case in ('initial_missing','initial_count','initial_rpm','initial_motor','initial_brake','first_entry', 'release_positive','release_negative','release_unchanged',
                     '179_entry','179_exit','180_entry','180_exit','419_entry','419_exit','420_entry','420_exit','181_entry','421_entry'):
            bad = deepcopy(run)
            if case == 'initial_missing': del bad['initial']['wheel_state']
            elif case == 'initial_count': bad['initial']['wheel_state'] *= 2
            elif case.startswith('initial_'):
                field = {'initial_rpm':'rpm','initial_motor':'motor_torque','initial_brake':'brake_torque'}[case]
                bad['initial']['wheel_state'][0][field] = 1.
            elif case == 'first_entry': bad['samples'][0]['wheels'][0]['rpm'] = 1.
            elif case.startswith('release_'):
                event = 5 if case == 'release_negative' else 1
                for sample in bad['samples'][120*event:120*(event+1)]:
                    sample['wheels'][0]['rpm'] = 1. if case in ('release_negative','release_unchanged') else -1.
            else:
                step,phase=case.split('_');bad['samples'][2*(int(step)-1)+(phase=='exit')]['wheels'][0]['rpm'] = .000001
            with self.subTest(case=case), self.assertRaises(ValueError): validate_angular(bad,row)

    def test_brake_retains_boundary_identity_and_actual_inputs(self):
        row, run = new_fixture(11)
        for phase in (0,1):
            for case in ('sleep','kinematic','disabled','inactive_self','inactive_hierarchy','contact','identity','motor','brake'):
                bad = deepcopy(run);sample=bad['samples'][240+phase];state=sample['angular_state']
                if case == 'sleep': state['body']['sleeping'] = True
                if case == 'kinematic': state['body']['kinematic'] = True
                if case == 'disabled': state['enabled'] = False
                if case.startswith('inactive_'): state[case.replace('inactive','active')] = False
                if case == 'contact': sample['wheels'][0]['contact'] = True
                if case == 'identity': state['wheel_id'] += 1
                if case in ('motor','brake'): sample['wheels'][0][case+'_torque'] += 1
                with self.subTest(phase=phase, case=case), self.assertRaises(ValueError): validate_angular(bad,row)

    def test_capture_and_verify_provenance_for_each_new_row(self):
        for index in (10,11):
            with self.subTest(index=index), patch.object(body_tests, 'gate_fixture', side_effect=lambda index=index:new_fixture(index)):
                checks=body_tests.BodyInputTests()
                checks.test_first_and_reused_input_gates()
                checks.test_capture_and_verify_bind_repeat_input_origin()

    def test_actual_helper_full_schedules_and_zero_velocity(self):
        reference=Path(__file__).parents[1]
        helper=(reference/'driving_angular.cs').read_text(encoding='utf-8-sig')
        stubs=angular_tests.STUBS.split('public static class AngularChecks {',1)[0]
        for index in (10,11):
            row=angular_rows()[index];events=row['angular']['events']
            declared=','.join('new G11AngularProbe.Event{step='+str(e['step'])+',phase="'+e['phase']+'",motor_nm='+str(e['motor_nm'])+'f,brake_nm='+str(e['brake_nm'])+'f}' for e in events)
            source=helper+stubs+SCHEDULE_CHECK.replace('EVENTS',declared).replace('VELOCITY',str(row['angular']['rig']['velocity_mps']['x'])+'f')
            self.compile(source,'ScheduleChecks','G11_FULL_SCHEDULE_PASS')

    def test_actual_capture_activation_zero_and_ten(self):
        reference=Path(__file__).parents[1]
        source=angular_tests.capture_arming_source(reference)
        for speed in ('0','10'):
            self.compile(source.replace('x=10}', 'x='+speed+'}').replace('==10', '=='+speed),'CaptureArmingChecks','G11_CAPTURE_ARMING_STUB_PASS')

    def compile(self,source,check,marker):
        result=subprocess.run(['pwsh','-NoProfile','-Command',"Add-Type -TypeDefinition @'\n"+source+"\n'@\n["+check+"]::Run()"],capture_output=True,text=True,timeout=60)
        self.assertEqual(result.returncode,0,result.stdout+result.stderr)
        self.assertIn(marker,result.stdout)


SCHEDULE_CHECK = r"""
public static class ScheduleChecks {
 static void Need(bool v,string m){if(!v)throw new Exception(m);}
 public static void Run(){
  var source=new UnityEngine.WheelCollider();var original=new UnityEngine.Rigidbody{id=1};var held=new UnityEngine.Rigidbody();
  var input=new G11AngularProbe.Input{source_wheel="level1:58684",support="full-tire",duration_steps=480,
   rig=new G11AngularProbe.Rig{constraints=116,velocity_mps=new UnityEngine.Vector3{x=VELOCITY}},events=new[]{EVENTS}};
  var wheel=G11AngularProbe.Configure(input,source,original,new UnityEngine.GameObject("rig"),held,false,null);
  wheel.enabled=true;held.isKinematic=false;G11AngularProbe.Arm();G11AngularProbe.RetainInitial();
  Need(held.writes==1 && held.angularWrites==1 && held.velocity.x==VELOCITY,"one-time input");
  bool rejected=false;try{G11AngularProbe.Arm();}catch(InvalidOperationException){rejected=true;}Need(rejected,"repeated arm");
  int motors=wheel.motors,brakes=wheel.brakes,revision=0;
  for(int step=1;step<=480;step++){
   G11AngularProbe.Assign(step);
   if(revision<8 && input.events[revision].step==step)revision++;
   var e=input.events[revision-1];var state=new G11AngularProbe.State();G11AngularProbe.Read(state);
   Need(wheel.motorTorque==e.motor_nm && wheel.brakeTorque==e.brake_nm,"actual torque");
   Need(state.event_revision==revision && state.assignment_step==step && state.phase==e.phase && state.enabled && state.active_self && state.active_hierarchy && !state.body.sleeping,"read boundary");
   Need(wheel.motors==motors+revision && wheel.brakes==brakes+revision,"per-solve torque assignment");
   Need(held.writes==1 && held.angularWrites==1 && wheel.rpm==-7,"state correction");
   rejected=false;try{G11AngularProbe.Assign(step);}catch(InvalidOperationException){rejected=true;}Need(rejected,"repeated solve");
  }
  Need(revision==8 && G11AngularProbe.report.assignments.Length==8,"eight assignments");
  for(int i=0;i<8;i++)Need(G11AngularProbe.report.assignments[i].step==input.events[i].step,"assignment clock");
  Console.WriteLine("G11_FULL_SCHEDULE_PASS");
 }
}
"""
