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


def clone_fixture(index, spinning=False):
    identity, parent, values, _ = CLONES[index]
    row, run = zero_tests.new_fixture(parent, spinning)
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
    return row, run


class AngularSpeedTests(unittest.TestCase):
    def test_exact_nine_clones_and_original_document(self):
        rows = angular_rows()
        self.assertEqual(len(rows), 21)
        committed = json.loads((Path(__file__).parents[3]/'tests/fixtures/driving/angular-scenarios.json').read_text())
        self.assertEqual(committed['scenarios'], rows)
        original = dict(committed, scenarios=committed['scenarios'][:12])
        self.assertEqual(hashlib.sha256(json.dumps(original,sort_keys=True,separators=(',',':')).encode()).hexdigest(),
                         'aee81f342939e4df4075e0872b772b4b90111884fadc7b6cd6dd55959ced3c13')
        for row, (identity, parent, values, bits) in zip(rows[12:], CLONES):
            expected = deepcopy(rows[parent])
            expected['id'] = identity
            velocity = dict(zip(('x','y','z'), values))
            expected['setup']['velocity_mps'] = deepcopy(velocity)
            expected['angular']['rig']['velocity_mps'] = deepcopy(velocity)
            self.assertEqual(row, expected)
            for actual in (row['setup']['velocity_mps'], row['angular']['rig']['velocity_mps']):
                self.assertEqual(tuple(struct.pack('>f',actual[a]).hex() for a in ('x','y','z')),bits)
            validate_angular_schedule(row)

    def test_new_rows_require_initial_and_consuming_activity(self):
        for index, (identity, _, _, _) in enumerate(CLONES):
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
                    if case=='velocity': state['body']['velocity']['z'] += 1.
                    elif case in ('sleeping','kinematic'): state['body'][case] = True
                    elif case=='missing_activity': del state['active_hierarchy']
                    else: state[case] = False
                    with self.subTest(row=identity,boundary=boundary,case=case), self.assertRaises(ValueError):
                        validate_angular(bad,row)
            for sample in run['samples']:
                sample['angular_state']['body']['velocity'] = dict(x=1.,y=2.,z=3.)
            self.assertEqual(validate_angular(run,row)['steps'],480)

    def test_clones_reject_changed_schedule_and_missing_terminal_zero(self):
        for index, (identity, _, _, _) in enumerate(CLONES):
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
        for index, (identity, _, _, _) in enumerate(CLONES):
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
        for index, (identity, _, _, _) in enumerate(CLONES):
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
