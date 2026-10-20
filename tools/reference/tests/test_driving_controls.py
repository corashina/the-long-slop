"""Critical explicit source-event and isolated-actuator control gates."""
from copy import deepcopy
import json
from pathlib import Path
import subprocess
import unittest

from tools.reference.driving import method
from tools.reference.driving_controls import (validate_schedule, validate_controls, row_at,
                                               ignition_edges, MODE_FIELDS, GEAR_MASKS)

ROOT = Path(__file__).resolve().parents[1]


def control_row(frame=0, **changes):
    return dict(frame=frame, throttle=0., brake=0., clutch=0., steer=0.,
                ignition_button_held=False, handbrake_press=False, **changes)


def scenario_fixture(kind='assembled_lada'):
    rows = [control_row(0), control_row(2)]
    if kind == 'wheel_collider':
        rows = [dict(frame=f, motor_scale=0., service_brake_fraction=0., handbrake_fraction=0., steer=0.) for f in (0, 2)]
    return dict(id='launch-default', probe_kind=kind, duration_frames=4, controls=rows,
                input_mode='keyboard-digital', parking_brake_intent='parked',
                setup=dict(auto_clutch=True, auto_shift=True))


def add_control_fixture(run, scenario):
    if scenario['probe_kind'] == 'wheel_collider':
        for sample in run['samples']:
            row = row_at(scenario,sample['frame'])
            sample['controls'] = dict(brake_power=1000.,max_handbrake=600.)
            for wheel in sample['wheels']:
                wheel['motor_torque'] = row['motor_scale']*1000.
                wheel['brake_torque'] = row['service_brake_fraction']*1000.+row['handbrake_fraction']*600.
        return run
    mode = dict(MODE_FIELDS, auto_clutch=scenario['setup']['auto_clutch'],
                auto_shift=scenario['setup']['auto_shift'], force_automatic_transmission=False)
    roles = [dict(path='source-wheel-'+str(i), driven=i<2, service_brake=i>=2, handbrake=i>=2) for i in range(4)]
    run['initial'].update(stored_handbrake=True, control_mode=deepcopy(mode), wheels=roles)
    state = dict(input_revision=0, input_gears=0, car_gears=0, mode=mode,
                 input_handbrake_press=False, car_handbrake_press=False, stored_handbrake=True,
                 input_ignition_held=False, input_ignition_down=False, input_ignition_up=False,
                 car_ignition_held=False, car_ignition_down=False,
                 seat_driver0=True, seat_driver1=True, driving=True, driving2=True,
                 player_driving=True, player_driving2=True, selected_ratio=0., differential_ratio=4.,
                 brake_power=1000., max_handbrake=600.)
    state.update({prefix+axis:0. for prefix in ('input_', 'car_') for axis in ('throttle','brake','clutch','steer','handbrake')})
    for sample in run['samples']:
        phase, frame = sample['phase'], sample['frame']
        row = row_at(scenario, frame)
        if phase == 'input.exit':
            state['input_revision'] += 1
            state['input_handbrake_press'] = row['frame'] == frame and row['handbrake_press']
            gear = row.get('gear_press', '') if row['frame'] == frame else ''
            state['input_gears'] = GEAR_MASKS[gear]
            for axis in ('throttle','brake','clutch','steer'):
                state['input_'+axis] = row[axis]
        if phase == 'input2.exit':
            state['input_ignition_held'], state['input_ignition_down'], state['input_ignition_up'] = ignition_edges(scenario, frame)
        if phase == 'control.exit':
            for axis in ('throttle','brake','clutch','steer','handbrake','handbrake_press','gears','ignition_held','ignition_down'):
                state['car_'+axis] = state['input_'+axis]
        if phase == 'actuation.exit' and state['car_handbrake_press']:
            state['stored_handbrake'] = not state['stored_handbrake']
        sample['controls'] = deepcopy(state)
        sample['handbrake'] = float(state['stored_handbrake'])
        sample['brake'] = state['car_brake']
        for role, wheel in zip(roles, sample['wheels']):
            wheel['brake_torque'] = (600.*sample['handbrake'] if role['handbrake'] else 0.) + (1000.*sample['brake'] if role['service_brake'] else 0.)
    return run


class DrivingControlsTests(unittest.TestCase):
    def observed(self, scenario=None):
        from tools.reference.tests.test_driving_probe import DrivingProbeTests
        scenario = scenario or scenario_fixture()
        run = DrivingProbeTests.sample_run()
        return add_control_fixture(run, scenario), scenario

    def test_every_committed_scenario_has_one_protocol(self):
        rows = json.loads((ROOT.parents[1]/'tests/fixtures/driving/scenarios.json').read_text())['scenarios']
        for row in rows:
            with self.subTest(row=row['id']): validate_schedule(row)

    def test_parked_launch_without_release_is_rejected(self):
        scenario = scenario_fixture()
        scenario['parking_brake_intent'] = 'release-before-acceleration'
        scenario['controls'][1]['throttle'] = 1.
        with self.assertRaisesRegex(ValueError, 'prior parking brake release'): validate_schedule(scenario)
        scenario['controls'][1]['handbrake_press'] = True
        with self.assertRaisesRegex(ValueError, 'prior parking brake release'): validate_schedule(scenario)

    def test_protocol_fields_types_and_modes_are_strict(self):
        changes = [('handbrake', 0.), ('gear', -99), ('ignition', False), ('motor_scale', 0.),
                   ('gear_press', -99), ('handbrake_press', 1), ('throttle', True), ('ignition_button_held', 1)]
        for key, value in changes:
            row = scenario_fixture()
            row['controls'][0][key] = value
            with self.subTest(key=key), self.assertRaises(ValueError): validate_schedule(row)
        for key in ('input_mode', 'parking_brake_intent'):
            row = scenario_fixture(); del row[key]
            with self.assertRaises(ValueError): validate_schedule(row)
        row = scenario_fixture();row['controls'][0]['gear_press'] = 'gear1'
        with self.assertRaisesRegex(ValueError, 'manual input mode'): validate_schedule(row)

    def test_isolated_fractions_do_not_toggle_source_latch(self):
        row = scenario_fixture('wheel_collider')
        row['controls'][0].update(motor_scale=-1., service_brake_fraction=.5, handbrake_fraction=.25)
        from tools.reference.tests.test_driving_probe import DrivingProbeTests
        run=add_control_fixture(DrivingProbeTests.sample_run('wheel_collider'),row)
        self.assertEqual(validate_controls(run, row)['protocol'], 'isolated-native-actuators')
        for field, value in [('handbrake_press', True), ('handbrake_fraction', True), ('service_brake_fraction', 1.1)]:
            bad = deepcopy(row);bad['controls'][0][field] = value
            with self.assertRaises(ValueError):validate_schedule(bad)

    def release_scenario(self):
        scenario = scenario_fixture()
        scenario['parking_brake_intent'] = 'release-before-acceleration'
        scenario['controls'] = [control_row(0), control_row(1), control_row(2), control_row(3)]
        scenario['controls'][0]['ignition_button_held'] = True
        scenario['controls'][1].update(handbrake_press=True, ignition_button_held=True)
        scenario['controls'][2].update(throttle=1.)
        scenario['controls'][3].update(handbrake_press=True, brake=1.)
        return scenario

    def test_exact_event_chain_and_later_independent_engage(self):
        run, scenario = self.observed(self.release_scenario())
        result = validate_controls(run, scenario)
        self.assertEqual([(e['frame'],e['stored_handbrake']) for e in result['handbrake_events']], [(1,False),(3,True)])
        for phase in ('input.exit','control.exit','actuation.exit','torque.exit'):
            sample = next(s for s in run['samples'] if s['phase']==phase and s['frame']==1)
            bad = deepcopy(run)
            target = bad['samples'][sample['sequence']-1]
            if phase=='input.exit':target['controls']['input_handbrake_press']=False
            if phase=='control.exit':target['controls']['car_handbrake_press']=False
            if phase=='actuation.exit':target['controls']['stored_handbrake']=True
            if phase=='torque.exit':target['wheels'][2]['brake_torque']=600.
            with self.subTest(phase=phase), self.assertRaises(ValueError):validate_controls(bad, scenario)

    def test_missed_duplicate_or_replayed_event_fails(self):
        run, scenario = self.observed(self.release_scenario())
        for change in ('missing','duplicate','replay','revision'):
            bad = deepcopy(run)
            target = next(s for s in bad['samples'] if s['phase']=='input.exit' and s['frame']==1)
            if change=='missing':bad['samples'].remove(target)
            if change=='duplicate':bad['samples'].append(deepcopy(target))
            if change=='replay':next(s for s in bad['samples'] if s['phase']=='input.exit' and s['frame']==2)['controls']['input_handbrake_press']=True
            if change=='revision':target['controls']['input_revision'] += 1
            with self.subTest(change=change), self.assertRaises(ValueError):validate_controls(bad, scenario)

    def test_mode_ownership_and_ignition_edges_are_measured(self):
        run, scenario = self.observed(self.release_scenario())
        for change in ('mode','h_shifter','ownership','edge'):
            bad = deepcopy(run)
            if change=='mode':del bad['initial']['control_mode']
            if change=='h_shifter':bad['initial']['control_mode']['h_shifter']=True
            if change=='ownership':bad['samples'][0]['controls']['seat_driver1']=False
            if change=='edge':next(s for s in bad['samples'] if s['phase']=='input2.exit' and s['frame']==1)['controls']['input_ignition_down']=True
            with self.subTest(change=change), self.assertRaises(ValueError):validate_controls(bad, scenario)

    def event_interval_fixture(self):
        scenario = scenario_fixture()
        scenario['parking_brake_intent'] = 'release-before-acceleration'
        scenario['setup']['auto_shift'] = False
        scenario['controls'] = [control_row(0), control_row(1), control_row(2)]
        scenario['controls'][0]['ignition_button_held'] = True
        scenario['controls'][1].update(ignition_button_held=True, handbrake_press=True, gear_press='gear1')
        scenario['controls'][2]['throttle'] = 1.
        return self.observed(scenario)

    def test_validate_run_accepts_events_between_physics_consumers(self):
        from tools.reference.driving_probe import validate_run
        run, scenario = self.event_interval_fixture()
        frames = [run['samples'][s['input_sequence']-1]['frame'] for s in run['samples'] if s['phase']=='physics.entry']
        self.assertEqual(frames,[0,0,2])
        result = validate_run(run, scenario)
        event = result['controls']['handbrake_events'][0]
        self.assertEqual((event['declared_frame'],event['frame'],event['input_revision']),(1,1,2))
        self.assertTrue(event['stored_handbrake_before'])
        self.assertFalse(event['stored_handbrake'])
        self.assertEqual(result['controls']['gear_events'][0]['declared_frame'],1)
        self.assertEqual([c['frame'] for c in scenario['controls']],[0,1,2])
        # Event validation still sees every original row after sustained intervals merge.
        for change in ('missing','duplicate','shifted'):
            bad = deepcopy(run)
            first = next(s for s in bad['samples'] if s['phase']=='input.exit' and s['frame']==1)
            later = next(s for s in bad['samples'] if s['phase']=='input.exit' and s['frame']==2)
            if change in ('missing','shifted'):first['controls']['input_handbrake_press'] = False
            if change in ('duplicate','shifted'):later['controls']['input_handbrake_press'] = True
            with self.subTest(change=change), self.assertRaises(ValueError):validate_run(bad,scenario)

    def test_validate_run_requires_each_real_sustained_transition(self):
        from tools.reference.driving_probe import validate_run
        for name in ('throttle','brake','clutch','steer','ignition_button_held'):
            run, scenario = self.event_interval_fixture()
            if name == 'throttle':
                # Keep the release before the newly earlier acceleration.
                scenario['controls'][0]['handbrake_press'] = True
                scenario['controls'][1]['handbrake_press'] = False
            scenario['controls'][1][name] = False if name=='ignition_button_held' else .25
            add_control_fixture(run,scenario)
            validate_controls(run,scenario)
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, r'control interval \[1, 2\)'):
                validate_run(run,scenario)

    def test_validate_run_requires_isolated_actuator_transitions(self):
        from tools.reference.driving_probe import validate_run
        from tools.reference.tests.test_driving_probe import DrivingProbeTests
        for name in ('motor_scale','service_brake_fraction','handbrake_fraction','steer'):
            scenario = scenario_fixture('wheel_collider')
            scenario['controls'].insert(1,dict(scenario['controls'][0],frame=1))
            scenario['controls'][1][name] = .25
            run = add_control_fixture(DrivingProbeTests.sample_run('wheel_collider'),scenario)
            validate_controls(run,scenario)
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, r'control interval \[1, 2\)'):
                validate_run(run,scenario)

    def test_prior_frame_source_consumption_remains_valid(self):
        from tools.reference.tests.test_driving_probe import DrivingProbeTests
        scenario = self.release_scenario()
        scenario['controls'][3]['handbrake_press'] = False
        run = DrivingProbeTests.sample_run()
        source = {'input.entry', 'input.exit', 'input2.entry', 'input2.exit'}
        reordered = []
        for frame in range(5):
            current = [s for s in run['samples'] if s['frame']==frame]
            reordered += [s for s in current if s['phase'] not in source]
            reordered += [s for s in current if s['phase'] in source]
        links = dict(input_sequence=0,car_sequence=0,engine_sequence=0,rpm_sequence=0)
        revisions = {'input.entry':'input_sequence','car.entry':'car_sequence','engine.entry':'engine_sequence','rpm.exit':'rpm_sequence'}
        for index, sample in enumerate(reordered, 1):
            sample['sequence'] = index
            if sample['phase'] in revisions: links[revisions[sample['phase']]] = index
            sample.update(links)
        run['samples'] = reordered
        add_control_fixture(run, scenario)
        result = validate_controls(run, scenario)
        self.assertEqual([(e['declared_frame'],e['frame']) for e in result['handbrake_events']],[(1,2)])
        self.assertNotEqual(result['handbrake_events'][0]['input_callback_sequence'], result['handbrake_events'][0]['consumed_sequence'])

    def test_direct_gear_consumption_is_required_without_forcing_gear(self):
        scenario = scenario_fixture()
        scenario['setup']['auto_shift'] = False
        scenario['controls'][0]['gear_press'] = 'gear1'
        scenario['setup']['auto_clutch'] = False
        run, scenario = self.observed(scenario)
        # Selected gear stays neutral. The original clutch eligibility may reject a consumed request.
        event=validate_controls(run,scenario)['gear_events'][0]
        self.assertEqual(event['gear'],'gear1')
        self.assertFalse(event['clutch_eligible'])
        self.assertEqual((event['before_gear'],event['after_gear']),(0,0))
        bad = deepcopy(run)
        for sample in bad['samples']:
            if sample['phase'] == 'control.exit' and sample['frame']==0:
                sample['controls']['car_gears'] = 0
        with self.assertRaises(ValueError):validate_controls(bad, scenario)

    def required_gear_fixture(self):
        scenario=scenario_fixture()
        scenario['setup'].update(auto_clutch=False,auto_shift=False)
        scenario['controls']=[control_row(frame) for frame in (0,1,2)]
        changes=[dict(frame=0,before_gear=0,after_gear=1),dict(frame=1,before_gear=1,after_gear=0),dict(frame=2,before_gear=0,after_gear=-1)]
        for row,name in zip(scenario['controls'],('gear1','neutral','reverse')):
            row.update(clutch=1.,gear_press=name)
        scenario['required_gear_changes']=changes
        run,scenario=self.observed(scenario)
        gear=0
        for sample in run['samples']:
            sample['clutch']=1.
            if sample['phase']=='actuation.exit' and sample['frame']<3:
                gear=changes[sample['frame']]['after_gear']
            sample['gear']=gear
        return run,scenario

    def test_required_gear_changes_use_actual_actuation_outcomes(self):
        from tools.reference.driving_probe import validate_run
        run,scenario=self.required_gear_fixture()
        declared=deepcopy(scenario)
        raw=deepcopy(run)
        result=validate_run(run,scenario)
        events=result['controls']['gear_events']
        self.assertEqual([(e['before_gear'],e['after_gear']) for e in events],[(0,1),(1,0),(0,-1)])
        for event in events:
            self.assertTrue(event['clutch_eligible'])
            self.assertEqual(event['clutch'],1.)
            self.assertLess(event['input_callback_sequence'],event['consumed_sequence'])
            self.assertLess(event['consumed_sequence'],event['actuation_entry_sequence'])
            self.assertLess(event['actuation_entry_sequence'],event['actuation_exit_sequence'])
            self.assertEqual(event['input_revision'],event['actuation_input_revision'])
        self.assertEqual(scenario,declared)
        self.assertEqual(run,raw)

    def test_required_gear_changes_reject_ineligible_wrong_and_missing_outcomes(self):
        from tools.reference.driving_probe import validate_run
        from tools.reference.tests.test_driving_probe import DrivingProbeTests
        for frame in (0,1,2):
            for corruption in ('clutch_boundary','clutch_below','before','unchanged','wrong_after','missing','entry_revision','exit_revision'):
                run,scenario=self.required_gear_fixture()
                entry=next(s for s in run['samples'] if s['phase']=='actuation.entry' and s['frame']==frame)
                exit=next(s for s in run['samples'] if s['phase']=='actuation.exit' and s['frame']==frame)
                if corruption=='clutch_boundary':exit['clutch']=.5
                if corruption=='clutch_below':exit['clutch']=.083334505558
                if corruption=='before':entry['gear']=2
                if corruption=='unchanged':exit['gear']=scenario['required_gear_changes'][frame]['before_gear']
                if corruption=='wrong_after':exit['gear']=2
                if corruption=='missing':DrivingProbeTests.remove_sample(run,run['samples'].index(exit))
                if corruption=='entry_revision':entry['controls']['input_revision']+=1
                if corruption=='exit_revision':exit['controls']['input_revision']+=1
                with self.subTest(frame=frame,corruption=corruption),self.assertRaises(ValueError):validate_run(run,scenario)

    def test_required_gear_metadata_is_strict(self):
        _,scenario=self.required_gear_fixture()
        changes=(None,[],[{}],[dict(frame=True,before_gear=0,after_gear=1)],
                 [dict(frame=0,before_gear=True,after_gear=1)],
                 [dict(frame=0,before_gear=0,after_gear=True)],
                 [dict(frame=0,before_gear=1,after_gear=1)],
                 [dict(frame=0,before_gear=0,after_gear=2)],
                 [dict(frame=3,before_gear=0,after_gear=1)],
                 [scenario['required_gear_changes'][0],scenario['required_gear_changes'][0]])
        for metadata in changes:
            bad=deepcopy(scenario);bad['required_gear_changes']=metadata
            with self.subTest(metadata=metadata),self.assertRaises(ValueError):validate_schedule(bad)

    def test_actual_brake_channels_do_not_assume_requested_service_output(self):
        run, scenario = self.observed()
        for sample in run['samples']:
            sample['gear'] = -1
            sample['brake'] = .25
            for index in (2,3):sample['wheels'][index]['brake_torque'] = 850.
        result = validate_controls(run, scenario)
        phase = result['requested_phases'][0]
        self.assertIn('Independent PhysX engine/service/parking brake torque decomposition',result['evidence']['unavailable_native'])
        self.assertEqual(result['evidence']['source_anchors']['service'],'carscript.SetWheel line 8101')
        self.assertEqual(phase['requests']['brake'],0.)
        self.assertEqual(phase['managed_source_brake'],[.25,.25])
        self.assertEqual(phase['wheel_contributors'][2]['derived_service_brake_nm'],[250.,250.])
        self.assertEqual(phase['wheel_contributors'][2]['derived_parking_brake_nm'],[600.,600.])

    def test_required_native_brake_matrix_and_actual_zero_crossings(self):
        from tools.reference.tests.test_driving_probe import DrivingProbeTests
        rows = json.loads((ROOT.parents[1]/'tests/fixtures/driving/scenarios.json').read_text())['scenarios']
        selected = {r['id']:r for r in rows if r['id'].startswith('native-')}
        self.assertEqual(set(selected), {'native-brake-stopped-hold','native-motor-and-service','native-motor-and-parking','native-motor-and-both'})
        for row in selected.values():
            self.assertTrue(any('zero' in note for note in row['required_observations']))
            self.assertTrue(any(c['service_brake_fraction'] or c['handbrake_fraction'] for c in row['controls']))
            if row['id']!='native-brake-stopped-hold':
                self.assertEqual([c['motor_scale'] for c in row['controls']],[-1.,1.,0.])
        row = scenario_fixture('wheel_collider')
        row['controls'][0].update(motor_scale=1.,service_brake_fraction=.25)
        run = add_control_fixture(DrivingProbeTests.sample_run('wheel_collider'),row)
        exits = [s for s in run['samples'] if s['phase']=='physics.exit']
        exits[0]['wheels'][0]['rpm']=-1.
        exits[1]['wheels'][0]['rpm']=1.
        result=validate_controls(run,row)
        self.assertTrue(result['native_actuator_phases'][0]['wheels'][0]['zero_crossing_observed'])
        next(s for s in run['samples'] if s['phase']=='physics.entry')['wheels'][0]['brake_torque']=0.
        with self.assertRaises(ValueError):validate_controls(run,row)

    def test_isolated_brake_phase_rejects_unwanted_or_missing_contributors(self):
        scenario=self.release_scenario()
        scenario['controls'][3]['handbrake_press']=False
        scenario['brake_phase']=dict(start_frame=3,end_frame=4,required_contributors=['service'],excluded_contributors=['engine','motor','parking'],observe_engine_interlock=False)
        run,scenario=self.observed(scenario)
        self.assertTrue(validate_controls(run,scenario)['brake_phase']['qualified_sequences'])
        for change in ('parking','missing'):
            bad=deepcopy(run)
            if change=='parking':
                # A declared later parking engage cannot pass a service-only phase gate.
                bad_scenario=deepcopy(scenario);bad_scenario['controls'][3]['handbrake_press']=True
                add_control_fixture(bad,bad_scenario)
            else:
                bad_scenario=deepcopy(scenario);bad_scenario['controls'][3]['brake']=0.
                add_control_fixture(bad,bad_scenario)
            with self.subTest(change=change), self.assertRaises(ValueError):validate_controls(bad,bad_scenario)

    def test_gear_names_default_absence_and_invalid_declarations(self):
        from tools.reference.driving_probe import validate_run
        for event,mask in GEAR_MASKS.items():
            run,scenario=self.event_interval_fixture()
            scenario['controls'][1]['gear_press']=event
            add_control_fixture(run,scenario)
            result=validate_run(run,scenario)
            emitted=next(s for s in run['samples'] if s['phase']=='input.exit' and s['frame']==1)
            self.assertEqual(emitted['controls']['input_gears'],mask)
            self.assertEqual([e['gear'] for e in result['controls']['gear_events']],[event] if event else [])
        for event in (None,True,0,-99,1.0,{}, {'value':1}, [],'none','noevent','forward','gear0','gear7','Reverse',' neutral','neutral '):
            scenario=scenario_fixture();scenario['setup']['auto_shift']=False
            scenario['controls'][0]['gear_press']=event
            with self.subTest(event=event),self.assertRaises(ValueError):validate_schedule(scenario)
        scenario=scenario_fixture();scenario['controls'][0]['gear_press']=''
        validate_schedule(scenario)

    def test_validate_run_keeps_gear_emission_failures_strict(self):
        from tools.reference.driving_probe import validate_run
        run,scenario=self.event_interval_fixture()
        validate_run(run,scenario)
        for change in ('missing','duplicate','shifted'):
            bad=deepcopy(run)
            first=next(s for s in bad['samples'] if s['phase']=='input.exit' and s['frame']==1)
            later=next(s for s in bad['samples'] if s['phase']=='input.exit' and s['frame']==2)
            if change in ('missing','shifted'):first['controls']['input_gears']=0
            if change in ('duplicate','shifted'):later['controls']['input_gears']=GEAR_MASKS['gear1']
            with self.subTest(change=change),self.assertRaises(ValueError):validate_run(bad,scenario)

    def test_committed_manual_gear_case_requires_prepared_outcomes(self):
        rows=json.loads((ROOT.parents[1]/'tests/fixtures/driving/scenarios.json').read_text())['scenarios']
        case=next(row for row in rows if row['id']=='gear-forward-neutral-reverse')
        self.assertEqual(case['required_gear_changes'],[dict(frame=90,before_gear=0,after_gear=1),dict(frame=180,before_gear=1,after_gear=0),dict(frame=240,before_gear=0,after_gear=-1)])
        self.assertEqual([(row['frame'],row['gear_press']) for row in case['controls'] if row.get('gear_press')],[(90,'gear1'),(180,'neutral'),(240,'reverse')])
        prep=next(row for row in case['controls'] if row['frame']==168)
        self.assertEqual(prep['clutch'],1.)
        self.assertEqual({k:v for k,v in prep.items() if k not in ('frame','clutch')},{k:v for k,v in next(row for row in case['controls'] if row['frame']==110).items() if k not in ('frame','clutch')})

    def test_actual_csharp_event_and_held_semantics(self):
        source = (ROOT/'driving_controls.cs').read_text()
        capture = (ROOT/'driving_capture.cs').read_text()
        bodies = '\n'.join(method(source, signature)[0] for signature in (
            'public static void Initialize(', 'static int Index(', 'public static float Axis(',
            'public static bool Held(', 'public static bool Down(', 'public static bool Up('))
        control = method(capture, '[Serializable] public class Control')[0]
        code = STUBS + 'public class G11Capture {'+control+'}\npublic static class G11Controls {'
        code += 'static G11Capture.Control[] rows;'+bodies+'}'+CHECKS
        command = "Add-Type -TypeDefinition @'\n" + code + "\n'@\n[G11ControlTests]::Run()"
        result = subprocess.run(['pwsh','-NoProfile','-Command',command],capture_output=True,text=True,timeout=60)
        self.assertEqual(result.returncode,0,result.stdout+result.stderr)
        self.assertIn('G11_CONTROL_PROTOCOL_PASS',result.stdout)


STUBS = r"""
using System;
public static class Mathf { public static float Max(float a,float b) { return Math.Max(a,b); } }
public class inputscript {public enum IN {gas,brake,clutch,steerleft,steerright,handbrake,ignition,gearr,gearn,gear1,gear2,gear3,gear4,gear5,gear6}}
"""
CHECKS = r"""
public static class G11ControlTests {
    static void Require(bool ok,string name) {if(!ok)throw new Exception(name);}
    public static void Run() {
        Require(typeof(G11Capture.Control).GetField("gear_press").FieldType==typeof(string),"gear field must be text");
        var actions=new[]{inputscript.IN.gearr,inputscript.IN.gearn,inputscript.IN.gear1,inputscript.IN.gear2,inputscript.IN.gear3,inputscript.IN.gear4,inputscript.IN.gear5,inputscript.IN.gear6};
        foreach(var absent in new[]{new G11Capture.Control{frame=0},new G11Capture.Control{frame=0,gear_press=String.Empty}})
        {
            G11Controls.Initialize(new[]{absent});
            foreach(var action in actions) Require(!G11Controls.Down(action,0),"native default omitted gear emitted an event");
        }
        var names=new[]{"reverse","neutral","gear1","gear2","gear3","gear4","gear5","gear6"};
        for(int index=0;index<names.Length;index++)
        {
            G11Controls.Initialize(new[]{new G11Capture.Control{frame=7,gear_press=names[index]}});
            for(int target=0;target<actions.Length;target++)
            {
                Require(G11Controls.Down(actions[target],7)==(target==index),"wrong exact gear event");
                Require(!G11Controls.Down(actions[target],8) && !G11Controls.Held(actions[target],7),"gear event replayed or held");
            }
        }
        var rows=new[]{new G11Capture.Control{frame=0,ignition_button_held=true},
            new G11Capture.Control{frame=89,ignition_button_held=true,handbrake_press=true},
            new G11Capture.Control{frame=90,throttle=1,gear_press="gear1"},
            new G11Capture.Control{frame=210,handbrake_press=true}};
        G11Controls.Initialize(rows);
        Require(G11Controls.Down(inputscript.IN.ignition,0),"initial ignition down");
        Require(G11Controls.Held(inputscript.IN.ignition,89) && !G11Controls.Down(inputscript.IN.ignition,89),"held row created a false down");
        Require(G11Controls.Up(inputscript.IN.ignition,90) && !G11Controls.Held(inputscript.IN.ignition,90),"ignition release");
        Require(G11Controls.Down(inputscript.IN.handbrake,89) && !G11Controls.Held(inputscript.IN.handbrake,89),"event became held input");
        Require(!G11Controls.Down(inputscript.IN.handbrake,90) && !G11Controls.Down(inputscript.IN.handbrake,209) && G11Controls.Down(inputscript.IN.handbrake,210),"event moved or replayed");
        Require(G11Controls.Down(inputscript.IN.gear1,90) && !G11Controls.Held(inputscript.IN.gear1,90) && !G11Controls.Down(inputscript.IN.gear1,91),"digital gear event");
        Require(!G11Controls.Down(inputscript.IN.gearn,0) && !G11Controls.Down(inputscript.IN.gearr,0),"absent gear event");
        Require(G11Controls.Axis(inputscript.IN.gas,90)==1 && G11Controls.Axis(inputscript.IN.gas,89)==0,"source axis timing");
        Console.WriteLine("G11_CONTROL_PROTOCOL_PASS");
    }
}
"""

if __name__ == '__main__':
    unittest.main()
