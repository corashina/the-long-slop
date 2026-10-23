"""Critical source transition schedule and evidence gates."""
from copy import deepcopy
import json
import math
from pathlib import Path
import unittest

from tools.reference.driving_probe import validate_transition_evidence, validate_transition_schedule
from tools.reference.driving_controls import validate_schedule


ROOT = Path(__file__).resolve().parents[3]
SCENARIO = json.loads((ROOT / 'tests/fixtures/driving/transition-scenarios.json').read_text(encoding='utf-8'))['scenarios'][0]
LAUNCH_DEFAULT_TIMING = {
    'source': '.cache/g11-driving/instrumentation-05/launch-default/repeat-1/runtime.json',
    'sha256': 'b4b5a27aa02931c81b4b2e2c1615220cf65ffe67c704e50f65e96adb7eea0d54',
    'frame': 239,
    'phase': 'input.exit',
    'throttle': 1.0,
    'gear': 1,
    'native_wheel_index': 1,
    'wheel_rpm': 14.66994571685791,
}
METHODS = {
    'refresh_tire_condition': 'partconditionscript.Refresh(int)',
    'remove_tire': 'partslotscript.UnCraft(false)',
    'remove_cover': 'partslotscript.UnCraft(false)',
    'remove_other_cover': 'partslotscript.UnCraft(false)',
    'remove_driven_rim': 'partslotscript.UnCraft(false)',
    'remove_other_driven_rim': 'partslotscript.UnCraft(false)',
    'remount_driven_rim': 'partslotscript.CanCraft(partscript)+Craft(partscript)',
    'remount_other_driven_rim': 'partslotscript.CanCraft(partscript)+Craft(partscript)',
    'remount_tire': 'partslotscript.CanCraft(partscript)+Craft(partscript)',
    'remount_cover': 'partslotscript.CanCraft(partscript)+Craft(partscript)',
    'remount_other_cover': 'partslotscript.CanCraft(partscript)+Craft(partscript)',
}


def transition_run(scenario=SCENARIO):
    slots = [dict(slot_key=f'slot-{index}', retained_rim_key=f'rim-{index}',
                  retained_tire_key=f'tire-{index}', retained_cover_key=f'cover-{index}',
                  rim_instance_id=100 + index, tire_instance_id=200 + index, cover_instance_id=300 + index,
                  native_wheel_index=index if index < 2 else -1, driven=index < 2,
                  rim_present=True, tire_present=True, cover_present=True, condition_available=True,
                  condition_state=2, rim_data_available=True, tire_slip1=0.8, tire_slip2=0.9,
                  rim_felni_radius=0.2, rim_data_radius=0.3, rim_data_slip1=1.0,
                  rim_data_slip2=1.0, rim_data_mass=10.0,
                  rim_active=True, tire_active=True, cover_active=True)
            for index in range(4)]
    native = [dict(index=index, key=f'wheel-{index}', instance_id=400 + index,
                   body_key=f'body-{index}', body_instance_id=500 + index,
                   enabled=True, active_self=True, active_hierarchy=True, contact=True,
                   rpm=12.0, radius=0.3, mass=1.0, damping=0.25,
                   forward_stiffness=1.0, sideways_stiffness=1.0,
                   pose=dict(x=0.0, y=0.0, z=0.0), rotation=dict(x=0.0, y=0.0, z=0.0, w=1.0))
              for index in range(4)]
    events = []
    step = 10

    def snapshot(phase, frame, physics_step, current_slots):
        installed = sum(slot['rim_present'] for slot in current_slots if slot['driven'])
        return dict(phase=phase, frame=frame, physics_step=physics_step,
                    installed_driven_rims=installed, driven_wheels_num=max(installed, 1),
                    source_lock_events=3, wheel_activation_events=0,
                    slots=deepcopy(current_slots), native_wheels=deepcopy(native))

    for declaration in scenario['transitions']:
        target_index = 0 if declaration['target'] == 'primary-driven' else 1
        before_slots = deepcopy(slots)
        immediate_slots = deepcopy(slots)
        target = immediate_slots[target_index]
        action = declaration['action']
        if action == 'refresh_tire_condition':
            target['condition_state'] = declaration['condition_state']
        elif action.startswith('remove_'):
            channel = 'tire_present' if action == 'remove_tire' else 'cover_present' if action.endswith('cover') else 'rim_present'
            target[channel] = False
        elif action.startswith('remount_'):
            channel = 'tire_present' if action == 'remount_tire' else 'cover_present' if action.endswith('cover') else 'rim_present'
            target[channel] = True
        for state in (before_slots, immediate_slots):
            for slot in state:
                for part in ('rim', 'tire', 'cover'):
                    slot[part + '_key'] = slot['retained_' + part + '_key'] if slot[part + '_present'] else None
        target_native_index = target['native_wheel_index']
        identity_values = sorted([slot[key] for slot in before_slots for key in
                                  ('slot_key', 'retained_rim_key', 'retained_tire_key', 'retained_cover_key')])
        later_frame = declaration['frame'] + declaration['observe_frames']
        events.append(dict(
            id=declaration['id'], action=action, target=declaration['target'],
            source_context=declaration['source_context'], source_method=METHODS[action],
            api_result='void-return' if action == 'refresh_tire_condition' else 'True',
            scheduled_frame=declaration['frame'], before_physics_step=step, later_physics_step=step + 1,
            rpm_wheel_index=target_native_index, rpm_before=native[target_native_index]['rpm'],
            later_after_frame=later_frame, condition_state=declaration.get('condition_state', 0),
            target_slot_key=f'slot-{target_index}', can_craft=action.startswith('remount_'), api_success=True,
            original_identities=identity_values, identities_after=identity_values,
            before=snapshot('before', declaration['frame'], step, before_slots),
            immediate=snapshot('immediate', declaration['frame'], step, immediate_slots),
            later=snapshot('later', later_frame, step + 1, immediate_slots)))
        slots = immediate_slots
        step += 2
    return {'transitions': events}


class DrivingTransitionTests(unittest.TestCase):
    def test_declared_scenario_has_sequential_nonoverlapping_direct_api_events(self):
        self.assertEqual(validate_schedule(SCENARIO)[0], 1840)
        self.assertEqual(len(validate_transition_schedule(SCENARIO)), 11)
        frames = [event['frame'] for event in SCENARIO['transitions']]
        self.assertEqual(frames[0], LAUNCH_DEFAULT_TIMING['frame'])
        self.assertTrue(all(later - earlier == 150 for earlier, later in zip(frames, frames[1:])))
        final_event = SCENARIO['transitions'][-1]
        self.assertEqual(SCENARIO['duration_frames'],
                         final_event['frame'] + final_event['observe_frames'] + 1)
        self.assertEqual([event['action'] for event in SCENARIO['transitions'][3:6]],
                         ['remove_driven_rim', 'remove_other_cover', 'remove_other_driven_rim'])
        self.assertEqual([event['action'] for event in SCENARIO['transitions'][-5:]],
                         ['remount_driven_rim', 'remount_other_driven_rim', 'remount_tire',
                          'remount_cover', 'remount_other_cover'])

    def test_start_frame_matches_measured_launch_default_evidence(self):
        self.assertTrue(LAUNCH_DEFAULT_TIMING['source'].endswith('/runtime.json'))
        self.assertEqual(len(LAUNCH_DEFAULT_TIMING['sha256']), 64)
        self.assertEqual(LAUNCH_DEFAULT_TIMING['phase'], 'input.exit')
        self.assertEqual(LAUNCH_DEFAULT_TIMING['throttle'], 1.0)
        self.assertEqual(LAUNCH_DEFAULT_TIMING['gear'], 1)
        self.assertGreater(LAUNCH_DEFAULT_TIMING['wheel_rpm'], 0.0)
        self.assertEqual(SCENARIO['transitions'][0]['frame'], LAUNCH_DEFAULT_TIMING['frame'])

    def test_evidence_binds_declarations_frames_channels_and_driven_divisor(self):
        run = transition_run()
        validate_transition_evidence(run, SCENARIO)
        corruptions = (
            'missing_event', 'failed_api', 'zero_rpm', 'missing_slot', 'lost_identity',
            'wrong_id', 'wrong_action', 'wrong_target', 'wrong_scheduled_frame',
            'wrong_before_frame', 'wrong_immediate_frame', 'early_later_frame', 'wrong_later_window',
            'missing_physics', 'wrong_later_physics_step', 'wrong_method', 'wrong_context',
            'wrong_immediate_state', 'wrong_later_state', 'lock_overlap', 'lost_instance_id',
            'native_identity_changed', 'activation_overlap', 'condition_unchanged',
            'remount_rejected', 'missing_count', 'wrong_count', 'missing_tire_cache',
            'missing_tire_slip2', 'missing_felni_radius', 'missing_rim_data',
            'missing_rim_data_slip', 'missing_rim_data_mass', 'wrong_stiffness_type',
            'missing_sideways_stiffness', 'nonfinite_radius', 'nonfinite_mass',
            'missing_damping', 'missing_rpm', 'wrong_pose', 'missing_pose_component',
            'missing_rotation', 'missing_contact', 'wrong_divisor',
        )
        for corruption in corruptions:
            with self.subTest(corruption=corruption):
                broken = deepcopy(run)
                event = broken['transitions'][0]
                if corruption == 'missing_event': broken['transitions'].pop()
                elif corruption == 'failed_api': event['api_success'] = False
                elif corruption == 'zero_rpm': event['rpm_before'] = 0.
                elif corruption == 'missing_slot': event['immediate']['slots'].pop()
                elif corruption == 'lost_identity': event['identities_after'].pop()
                elif corruption == 'wrong_id': event['id'] = 'other'
                elif corruption == 'wrong_action': event['action'] = 'remove_tire'
                elif corruption == 'wrong_target': event['target'] = 'other-driven'
                elif corruption == 'wrong_scheduled_frame': event['scheduled_frame'] += 1
                elif corruption == 'wrong_before_frame': event['before']['frame'] += 1
                elif corruption == 'wrong_immediate_frame': event['immediate']['frame'] += 1
                elif corruption == 'early_later_frame': event['later']['frame'] -= 1
                elif corruption == 'wrong_later_window': event['later_after_frame'] += 1
                elif corruption == 'missing_physics': event['later_physics_step'] = event['before_physics_step']
                elif corruption == 'wrong_later_physics_step': event['later']['physics_step'] += 1
                elif corruption == 'wrong_method': event['source_method'] = 'other API'
                elif corruption == 'wrong_context': event['source_context'] = 'player-interaction'
                elif corruption == 'wrong_immediate_state': event['immediate']['slots'][0]['condition_state'] = 2
                elif corruption == 'wrong_later_state':
                    event = broken['transitions'][1]
                    event['later']['slots'][0]['tire_present'] = True
                elif corruption == 'lock_overlap': event['later']['source_lock_events'] += 1
                elif corruption == 'lost_instance_id': event['later']['slots'][0]['cover_instance_id'] = 0
                elif corruption == 'native_identity_changed': event['immediate']['native_wheels'][0]['instance_id'] = 0
                elif corruption == 'activation_overlap': event['immediate']['wheel_activation_events'] += 1
                elif corruption == 'condition_unchanged': event['before']['slots'][0]['condition_state'] = 4
                elif corruption == 'remount_rejected': broken['transitions'][6]['can_craft'] = False
                elif corruption == 'missing_count': del event['before']['installed_driven_rims']
                elif corruption == 'wrong_count': event['before']['installed_driven_rims'] = 99
                elif corruption == 'missing_tire_cache': del event['before']['slots'][0]['tire_slip1']
                elif corruption == 'missing_tire_slip2': del event['before']['slots'][0]['tire_slip2']
                elif corruption == 'missing_felni_radius': del event['before']['slots'][0]['rim_felni_radius']
                elif corruption == 'missing_rim_data': del event['before']['slots'][0]['rim_data_radius']
                elif corruption == 'missing_rim_data_slip': del event['before']['slots'][0]['rim_data_slip2']
                elif corruption == 'missing_rim_data_mass': del event['before']['slots'][0]['rim_data_mass']
                elif corruption == 'wrong_stiffness_type': event['before']['native_wheels'][0]['forward_stiffness'] = '1'
                elif corruption == 'missing_sideways_stiffness': del event['before']['native_wheels'][0]['sideways_stiffness']
                elif corruption == 'nonfinite_radius': event['before']['native_wheels'][0]['radius'] = math.inf
                elif corruption == 'nonfinite_mass': event['before']['native_wheels'][0]['mass'] = math.nan
                elif corruption == 'missing_damping': del event['before']['native_wheels'][0]['damping']
                elif corruption == 'missing_rpm': del event['before']['native_wheels'][0]['rpm']
                elif corruption == 'wrong_pose': event['before']['native_wheels'][0]['pose'] = [0, 0, 0]
                elif corruption == 'missing_pose_component': del event['before']['native_wheels'][0]['pose']['z']
                elif corruption == 'missing_rotation': del event['before']['native_wheels'][0]['rotation']
                elif corruption == 'missing_contact': del event['before']['native_wheels'][0]['contact']
                elif corruption == 'wrong_divisor': broken['transitions'][5]['immediate']['driven_wheels_num'] = 0
                with self.assertRaises(ValueError):
                    validate_transition_evidence(broken, SCENARIO)

    def test_schedule_rejects_overlap_and_missing_source_context(self):
        overlap = deepcopy(SCENARIO)
        overlap['transitions'][1]['frame'] = overlap['transitions'][0]['frame'] + 20
        with self.assertRaisesRegex(ValueError, 'frame'):
            validate_transition_schedule(overlap)
        missing = deepcopy(SCENARIO)
        del missing['transitions'][0]['source_context']
        with self.assertRaisesRegex(ValueError, 'API'):
            validate_transition_schedule(missing)

    def test_capture_uses_ordinary_source_apis_without_resets(self):
        capture = (ROOT / 'tools/reference/driving_capture.cs').read_text(encoding='utf-8-sig')
        for source_api in ('target.tire_slot.UnCraft(false)', 'target.cover_slot.UnCraft(false)',
                           'target.slot.UnCraft(false)', 'target.slot.CanCraft(target.rim)',
                           'target.tire_slot.CanCraft(target.tire)', 'target.cover_slot.CanCraft(target.cover)'):
            with self.subTest(source_api=source_api):
                self.assertIn(source_api, capture)
        method = capture.split('static void RunSourceTransition(', 1)[1].split('\n    static void ObserveLaterTransition()', 1)[0]
        self.assertIn('input.source_context!="direct-component-api"', method)
        self.assertIn('case "remove_cover": case "remove_other_cover":', method)
        self.assertIn('case "remount_cover": case "remount_other_cover":', method)
        for reset in ('Time.timeScale=', '.rpm=', 'transform.position=', 'transform.rotation=', 'Physics.SyncTransforms'):
            with self.subTest(reset=reset):
                self.assertNotIn(reset, method)


if __name__ == '__main__':
    unittest.main()
