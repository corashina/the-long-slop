"""Critical source lifetime and original callback dependency corruption gates."""
from copy import deepcopy
import json
import unittest

from tools.reference.driving_lifecycle import validate_lifecycle
from tools.reference.tests.test_driving_driver import driver_fixture, body_fixture


def lifecycle_fixture(run=None):
    run = driver_fixture() if run is None else run
    zero = dict.fromkeys('xyz', 0.)
    rotation = dict(zero, w=1.)
    def mark(callback, order, sequence=0):
        return dict(callback=callback, order=order, frame=10+sequence, time=.02*sequence,
                    update_sequence=sequence, input_revision=sequence)
    updates = []
    for sequence in (3, 4, 5):
        start = 20 + (sequence-3)*10
        update = dict(sequence=sequence, setup_revision=2, look_producer=sequence-1 if sequence>3 else 0,
                      midpoint_producer=sequence-1 if sequence>3 else 0, dependency_complete=sequence>3,
                      rest_equal=sequence>3, first_change=None if sequence>3 else 'first completed Update',
                      rays=[dict(origin=zero.copy(), direction=zero.copy(), point=zero.copy(), distance=10., mask=1,
                                 hit=False, collider='explicit miss', collider_id=0) for _ in range(2)],
                      rest_fields=[dict(key='piss-transform', value=json.dumps(dict(local_rotation=rotation)))],
                      **{k: zero.copy() for k in ('entry_look', 'entry_piss', 'entry_piss_forward', 'entry_head', 'entry_head_forward',
                           'consumed_look', 'consumed_piss', 'produced_look', 'midpoint_position')},
                      forward_rotation=rotation.copy(), look_rotation=rotation.copy())
        for offset, (field, callback) in enumerate((('entry', 'driver.update.entry'), ('forward', 'driver.forward.before'),
              ('look_entry', 'driver.look-point.entry'), ('look_exit', 'driver.look-point.exit'),
              ('look_output', 'driver.look.exit'), ('midpoint', 'driver.midpoint.after'), ('exit', 'driver.update.exit'))):
            update[field] = mark(callback, start+offset, sequence)
        for boundary in ('entry', 'exit'):
            update[boundary+'_body'] = body_fixture('driver.update.'+boundary, sequence)
            update[boundary+'_stored_dt'] = .02
            for name in ('parent_velocity', 'car_velocity_history', 'head_velocity_history', 'head_bob_history'):
                update[boundary+'_'+name] = zero.copy()
        update['delta_time'] = .02
        updates.append(update)
    for phase, sequence in (('stationary-update-exit', 4), ('held-update-exit', 5)):
        stamp = next(s for s in run['driver']['stamps'] if s['phase'] == phase)
        stamp['update_sequence'] = sequence
    for state, sequence in zip(run['driver']['states'], (4, 5)): state['update_sequence'] = sequence
    run['lifecycle'] = dict(begun=True, owners_complete=True, cohort_closed=True, ready=True, armed=True,
        owner_completion_evidence='original owners completed', new_after_close=[], opened=mark('world.entry', 1),
        owners_completed=mark('observer.Initialize.explicit-setup-exit', 9),
        closed=mark('observer.prepared-held.close-cohort', 37, 4), prepared=mark('observer.prepared-held', 38, 4),
        owner_callbacks=[mark('slot.ready:source-slot', 8)], updates=updates,
        audio=[dict(prefab='sharedassets1.assets:21250', clip='source-clip', creation_parent=None, parent='source-part',
                    source_owner='source-part', source_stack='pickupable.Crafted -> PlayClipAtPoint',
                    component='source-audio/AudioSource', collider='source-audio/SphereCollider',
                    object_id=11, component_id=12, collider_id=13, prefab_id=14, clip_id=15, clip_length=.367, destroy_delay=.367,
                    original_destroy_requested=True, object_null=True, component_null=True, collider_null=True,
                    required=True, scope_member=False, created_after_close=False,
                    membership=[dict(observed=mark('observer.LateUpdate.scope', 5), member=True, parent='source-part', owner='car-body', owner_id=456, owner_is_null=False),
                                dict(observed=mark('driver.update.exit.scope', 9), member=False, parent='source-part', owner=None, owner_id=0, owner_is_null=True)],
                    created=mark('setup.crafted.entry', 2),
                    destruction_requested=mark('mainscript.PlayClipAtPoint.Destroy(Object,clip.length)', 3),
                    parent_observed=mark('observer.LateUpdate.parent', 4), removed=mark('driver.update.exit.Unity-null', 10))])
    return run


class DrivingLifecycleTests(unittest.TestCase):
    def test_absent_lifecycle_fields_fail(self):
        valid = lifecycle_fixture()
        self.assertEqual(validate_lifecycle(valid)['audio_objects'], 1)
        for field in tuple(valid['lifecycle']):
            with self.subTest(field=field):
                run = deepcopy(valid)
                del run['lifecycle'][field]
                with self.assertRaises(ValueError): validate_lifecycle(run)

    def test_audio_removal_requires_all_actual_references_and_source_provenance(self):
        valid = lifecycle_fixture()
        for field in tuple(valid['lifecycle']['audio'][0]):
            with self.subTest(field=field):
                run = deepcopy(valid)
                del run['lifecycle']['audio'][0][field]
                with self.assertRaises(ValueError): validate_lifecycle(run)
        for field in ('object_null', 'component_null', 'collider_null', 'original_destroy_requested'):
            run = deepcopy(valid)
            run['lifecycle']['audio'][0][field] = False
            with self.assertRaises(ValueError): validate_lifecycle(run)
        run = deepcopy(valid)
        run['lifecycle']['new_after_close'] = [run['lifecycle']['audio'][0]]
        with self.assertRaisesRegex(ValueError, 'cohort closure'): validate_lifecycle(run)

    def test_original_dependency_and_natural_rest_evidence_are_required(self):
        corruptions = ('look_producer', 'midpoint_producer', 'consumed_look', 'consumed_piss', 'ray', 'ray_hit',
                       'look_output', 'writer_order', 'geometry', 'rest_claim', 'setup_revision', 'prepared', 'held',
                       'entry_body', 'exit_body', 'entry_piss_forward', 'exit_parent_velocity', 'entry_head_bob_history', 'entry_stored_dt')
        for corruption in corruptions:
            with self.subTest(corruption=corruption):
                run = lifecycle_fixture()
                report = run['lifecycle']
                update = report['updates'][1]
                if corruption in ('look_producer', 'midpoint_producer'): update[corruption] = 0
                if corruption in ('consumed_look', 'consumed_piss'): update[corruption]['x'] = 1.
                if corruption == 'ray': del update['rays'][0]['mask']
                if corruption == 'ray_hit': update['rays'][0]['hit'] = True
                if corruption == 'look_output': del update['look_output']
                if corruption == 'writer_order': update['midpoint']['order'] = update['forward']['order']
                if corruption == 'geometry': update['rest_fields'][0]['value'] = '{}'
                if corruption == 'rest_claim': update['rest_equal'] = False
                if corruption == 'setup_revision': update['setup_revision'] = 1
                if corruption == 'prepared': report['prepared']['order'] = update['entry']['order']
                if corruption == 'held': report['updates'].pop()
                if corruption in ('entry_body', 'exit_body', 'entry_piss_forward', 'exit_parent_velocity', 'entry_head_bob_history', 'entry_stored_dt'):
                    del update[corruption]
                with self.assertRaises(ValueError): validate_lifecycle(run)

    def test_audio_creation_remains_allowed_while_source_cohort_drains(self):
        run = lifecycle_fixture()
        audio = deepcopy(run['lifecycle']['audio'][0])
        audio.update(object_id=21, component_id=22, collider_id=23,
                     created=dict(audio['created'], order=11),
                     destruction_requested=dict(audio['destruction_requested'], order=12),
                     parent_observed=dict(audio['parent_observed'], order=13),
                     removed=dict(audio['removed'], order=16),
                     membership=[dict(observed=dict(audio['membership'][0]['observed'], order=14), member=True, parent='source-part', owner='car-body', owner_id=456, owner_is_null=False),
                                 dict(observed=dict(audio['membership'][1]['observed'], order=15), member=False, parent='source-part', owner=None, owner_id=0, owner_is_null=True)])
        run['lifecycle']['audio'].append(audio)
        self.assertGreater(audio['created']['order'], run['lifecycle']['owners_completed']['order'])
        self.assertEqual(validate_lifecycle(run)['audio_objects'], 2)
        run['lifecycle']['new_after_close'].append(deepcopy(audio))
        with self.assertRaisesRegex(ValueError, 'cohort closure'): validate_lifecycle(run)
        run['lifecycle']['new_after_close'].clear()
        audio['created']['order'] = run['lifecycle']['closed']['order'] + 1
        with self.assertRaises(ValueError): validate_lifecycle(run)

    def test_closure_requires_actual_audio_removal_and_completed_driver_rest(self):
        for corruption in ('before_removal', 'before_driver_rest', 'missing_parent', 'missing_owner_completion'):
            with self.subTest(corruption=corruption):
                run = lifecycle_fixture()
                report = run['lifecycle']
                if corruption == 'before_removal': report['audio'][0]['removed']['order'] = report['closed']['order'] + 1
                if corruption == 'before_driver_rest': report['closed']['order'] = report['updates'][1]['exit']['order'] - 1
                if corruption == 'missing_parent': del report['audio'][0]['parent_observed']
                if corruption == 'missing_owner_completion': del report['owners_completed']
                with self.assertRaises(ValueError): validate_lifecycle(run)

    def test_live_outside_audio_does_not_block_exact_car_scope(self):
        run = lifecycle_fixture()
        outside = deepcopy(run['lifecycle']['audio'][0])
        outside.update(object_id=21, component_id=22, collider_id=23, required=False, scope_member=False,
                       object_null=False, component_null=False, collider_null=False, removed=None,
                       parent='outside-source',
                       membership=[dict(observed=dict(outside['membership'][0]['observed']), member=False,
                                        parent='outside-source', owner=None, owner_id=0, owner_is_null=True)])
        run['lifecycle']['audio'].append(outside)
        self.assertEqual(validate_lifecycle(run)['required_audio_objects'], 1)
        required = run['lifecycle']['audio'][0]
        required.update(object_null=False, component_null=False, collider_null=False, removed=None)
        with self.assertRaisesRegex(ValueError, 'captured-geometry source audio removal'): validate_lifecycle(run)

    def test_actual_late_scope_membership_cannot_escape_required_drain(self):
        run = lifecycle_fixture()
        audio = run['lifecycle']['audio'][0]
        audio['required'] = False
        with self.assertRaisesRegex(ValueError, 'actual geometry membership'): validate_lifecycle(run)
        audio['required'] = True
        audio['membership'][0]['observed']['order'] = run['lifecycle']['closed']['order'] + 1
        with self.assertRaisesRegex(ValueError, 'joined after readiness closure'): validate_lifecycle(run)

    def test_missing_scope_fields_cannot_classify_audio_as_outside(self):
        valid = lifecycle_fixture()
        for field in ('required', 'scope_member', 'created_after_close', 'membership'):
            run = deepcopy(valid)
            del run['lifecycle']['audio'][0][field]
            with self.subTest(field=field), self.assertRaises(ValueError): validate_lifecycle(run)
        for field in tuple(valid['lifecycle']['audio'][0]['membership'][0]):
            run = deepcopy(valid)
            del run['lifecycle']['audio'][0]['membership'][0][field]
            with self.subTest(scope_field=field), self.assertRaises(ValueError): validate_lifecycle(run)
        for field in ('required', 'scope_member', 'created_after_close'):
            run = deepcopy(valid)
            run['lifecycle']['audio'][0][field] = None
            with self.subTest(null=field), self.assertRaises(ValueError): validate_lifecycle(run)
