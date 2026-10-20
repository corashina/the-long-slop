"""Critical driver lifecycle corruption gates."""
from copy import deepcopy
import unittest

from tools.reference.driving_driver import POSE_FIELDS, STAMPS, validate_driver


def driver_fixture():
    zero = dict.fromkeys('xyz', 0.)
    def pose(field):
        return dict(field=field, key=field, parent='seat' if field == 'Tb' else 'Tb',
                    local_position=zero.copy(), local_rotation=dict(zero, w=1.), local_scale=dict.fromkeys('xyz', 1.))
    source_input = dict(references='checked original refs', parent_body='car-body', poses=[pose(f) for f in POSE_FIELDS],
                        yaw=0., pitch=0., stored_dt=.02, duck_value=1., look_x=0., look_y=0., scroll=0.,
                        lean=False, lean_down=False, lean_up=False, recenter=False, recenter_down=False, recenter_up=False,
                        key_looking=False, input_duck=False, input_duck_down=False, input_duck_up=False, duck=False, last_ducked=False, stand_lean=False,
                        **{k: zero.copy() for k in ('parent_velocity', 'car_velocity_history', 'head_velocity_history', 'head_bob_history')})
    stamps = []
    for i, phase in enumerate(STAMPS):
        update = (0, 0, 1, 1, 2, 2, 3, 4)[i]
        stamps.append(dict(phase=phase, callback='driver.update.exit' if i in (4, 6, 7) else 'observer.LateUpdate',
                           frame=update+10, physics_step=update, update_sequence=update, input_revision=update,
                           time=.02*update, fixed_time=.02*update, delta_time=.02, stored_dt=.02, consumed_dt=.02,
                           original_body_present=i<2, managed_body_present=i==0, owners_transferred=i>=3,
                           consistent=i>=1, collider_owners=['car-body' if i>=3 else 'driver-body']))
    states = []
    for phase, update in (('prepared-held', 3), ('backlog-drained-held', 4)):
        states.append(dict(source_input, phase=phase, poses=deepcopy(source_input['poses']), piss_pose=pose('piss'),
                           look_point=zero.copy(), update_sequence=update, input_revision=update, consumed_dt=.02,
                           drive_head_bob=True, xr=False, mind=True, died=False, cursor_locked=True, driver_update_flag=False, sleeping=False,
                           neutral=True, look_x=0., look_y=0., scroll=0.))
    return dict(driver=dict(original_body='driver-body', original_body_instance_id=123, expected_colliders=['driver-shape'],
                            input_applications=1, input=source_input, stamps=stamps, states=states),
                setup_state=dict(driver_input=deepcopy(source_input)), initial=dict(driver_input=deepcopy(source_input)))


class DrivingDriverTests(unittest.TestCase):
    def test_real_native_body_and_expected_owners_are_required(self):
        for corrupt, message in (('native', 'body removal'), ('owner', 'owner transfer'),
                                 ('missing_transfer', 'transfer set'), ('inconsistent', 'seated driver ownership')):
            with self.subTest(corrupt=corrupt):
                run = driver_fixture()
                self.assertEqual(validate_driver(run)['expected_colliders'], 1)
                if corrupt == 'native': run['driver']['stamps'][2]['original_body_present'] = True
                if corrupt == 'owner': run['driver']['stamps'][3]['collider_owners'] = ['driver-body']
                if corrupt == 'missing_transfer': run['driver']['expected_colliders'] = []
                if corrupt == 'inconsistent': run['driver']['stamps'][4]['consistent'] = False
                with self.assertRaisesRegex(ValueError, message): validate_driver(run)

    def test_native_removal_requires_explicit_false_body_flags(self):
        valid = driver_fixture()
        for stamp in valid['driver']['stamps'][2:]:
            self.assertIs(stamp['original_body_present'], False)
            self.assertIs(stamp['managed_body_present'], False)
        self.assertEqual(validate_driver(valid)['expected_colliders'], 1)
        for missing in (('original_body_present',), ('managed_body_present',),
                        ('original_body_present', 'managed_body_present')):
            for index in range(2, len(STAMPS)):
                with self.subTest(missing=missing, phase=STAMPS[index]):
                    run = driver_fixture()
                    for field in missing:
                        del run['driver']['stamps'][index][field]
                    with self.assertRaisesRegex(ValueError, 'body removal'):
                        validate_driver(run)

    def test_player_update_drive_flag_is_not_seat_ownership(self):
        run = driver_fixture()
        self.assertFalse(run['driver']['states'][0]['driver_update_flag'])
        self.assertEqual(validate_driver(run)['held_update_sequence'], 4)
        run['driver']['states'][0]['sleeping'] = True
        with self.assertRaisesRegex(ValueError, 'non-XR'): validate_driver(run)

    def test_stationary_setup_requires_real_separate_source_update_exits(self):
        for corrupt, message in (('no_update', 'completed original'), ('not_next', 'completed original'),
                                 ('manual', 'completed original'), ('timing', 'timing evidence'),
                                 ('missing_stamp', 'lifecycle boundary')):
            with self.subTest(corrupt=corrupt):
                run = driver_fixture()
                stamps = run['driver']['stamps']
                if corrupt == 'no_update': stamps[4]['update_sequence'] = stamps[3]['update_sequence']
                if corrupt == 'not_next': stamps[7]['update_sequence'] += 1
                if corrupt == 'manual': stamps[6]['callback'] = 'observer.manual-update'
                if corrupt == 'timing': del stamps[6]['consumed_dt']
                if corrupt == 'missing_stamp': del stamps[2]
                with self.assertRaisesRegex(ValueError, message): validate_driver(run)

    def test_canonical_inputs_and_local_source_outputs_are_complete(self):
        for corrupt, message in (('reapply', 'exactly once'), ('changed', 'applied record'),
                                 ('target', 'target chain'), ('pose', 'local pose'), ('dt', 'stored dt'),
                                 ('bob', 'authored zero'), ('input_lean', 'neutral driver input'), ('xr', 'non-XR'), ('neutral', 'non-XR'),
                                 ('look_point', 'output state'), ('scale', 'local pose')):
            with self.subTest(corrupt=corrupt):
                run = driver_fixture()
                report = run['driver']
                if corrupt == 'reapply': report['input_applications'] = 2
                if corrupt == 'changed': run['initial']['driver_input']['yaw'] = 1.
                if corrupt == 'target': report['input']['poses'].pop()
                if corrupt == 'pose': del report['input']['poses'][0]['local_scale']
                if corrupt == 'dt': del report['input']['stored_dt']
                if corrupt == 'bob': report['input']['head_bob_history']['x'] = .1
                if corrupt == 'input_lean': report['input']['lean'] = True
                if corrupt in ('target', 'pose', 'dt', 'bob', 'input_lean'):
                    run['initial']['driver_input'] = deepcopy(report['input'])
                    run['setup_state']['driver_input'] = deepcopy(report['input'])
                if corrupt == 'xr': report['states'][0]['xr'] = True
                if corrupt == 'neutral': report['states'][0]['look_x'] = .1
                if corrupt == 'look_point': del report['states'][0]['look_point']
                if corrupt == 'scale': del report['states'][0]['poses'][0]['local_scale']
                with self.assertRaisesRegex(ValueError, message): validate_driver(run)
