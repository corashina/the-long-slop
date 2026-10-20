"""Critical G11 provenance and native evidence gates."""
import json
from copy import deepcopy
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from tools.reference.driving_probe import preflight, capture, verify, validate_run, observer_revision
from tools.reference.library.archive import sha256
from tools.reference.driving_geometry import validate_geometry, physical_inputs
from tools.reference.tests.test_driving_driver import driver_fixture
from tools.reference.tests.test_driving_lifecycle import lifecycle_fixture


class DrivingProbeTests(unittest.TestCase):
    def test_geometry_helper_provenance(self):
        revision = observer_revision()
        for name in ('driving_geometry.cs', 'driving_geometry.py', 'driving_driver.cs', 'driving_driver.py', 'driving_lifecycle.cs', 'driving_lifecycle.py'):
            self.assertEqual(revision[name], sha256(Path(__file__).parents[1] / name))

    def test_native_helper_restoration_and_physical_readiness(self):
        for change, message in (('helper', 'intended state'), ('cargo', 'deferred cargo'),
                                ('shape', 'shape data'), ('spring', 'profile scalar'), ('empty_spring', 'profile scalar'), ('empty_friction', 'friction profile'), ('missing_friction_scalar', 'friction profile'), ('helper_samples', 'helper or sprungMass'),
                                ('roles', 'four separate road roles'), ('held_geometry', 'held boundary')):
            with self.subTest(change=change):
                run = self.sample_run()
                validate_geometry(run)
                snapshot = next(row for row in run['geometry'] if row['phase'] == 'native-wheels-enabled')
                if change == 'helper': snapshot['colliders'][0]['enabled'] = True
                if change == 'cargo': snapshot['physical_ready'] = False
                if change == 'shape': snapshot['colliders'][0]['complete'] = False
                if change == 'spring': del snapshot['colliders'][0]['spring_n_per_m']
                if change == 'empty_spring': snapshot['colliders'][0]['spring_n_per_m'] = {}
                if change == 'empty_friction': snapshot['colliders'][0]['forward'] = {}
                if change == 'missing_friction_scalar': del snapshot['colliders'][0]['sideways']['stiffness']
                if change == 'roles': snapshot['colliders'][0]['road_slot'] = 0
                if change == 'helper_samples': run['samples'][0]['native_wheels'].pop()
                if change == 'held_geometry':
                    held = next(row for row in run['geometry'] if row['phase'] == 'backlog-drained-held')
                    held['colliders'][0]['radius'] = 1.
                with self.assertRaisesRegex(ValueError, message): validate_geometry(run)

    def test_geometry_outputs_do_not_hide_local_shape_inputs(self):
        run = self.sample_run()
        left, right = run['geometry'][:2]
        body_transform = next(row for row in right['transforms'] if row['key'] == 'body-transform')
        body_transform['local_rotation']['w'] = .99
        body_transform['world_matrix'] = 99.
        self.assertEqual(physical_inputs(left), physical_inputs(right))
        right['colliders'][0]['radius'] = 99.
        self.assertNotEqual(physical_inputs(left), physical_inputs(right))

    def retirement_run(self):
        run = self.sample_run()
        audio = run['lifecycle']['audio'][0]
        row = dict(key='retained-audio-collider', instance_id=audio['collider_id'],
                   original_owner='cargo-body', original_owner_instance_id=123, original_owner_is_null=False,
                   present=False, owner=None, owner_instance_id=0, owner_is_null=True, retired=True,
                   retirement_audio=[deepcopy(audio)], removed=[deepcopy(audio['removed'])])
        for snapshot in run['geometry']:
            snapshot.update(frame=20, time=1., cargo=[dict(key='source-cargo', ownership_ready=True,
                                                        collider_owners=[deepcopy(row)])])
        return run

    def test_exact_retained_source_audio_retirement_preserves_geometry_inputs(self):
        run = self.retirement_run()
        self.assertEqual(validate_geometry(run)['native_wheels'], 5)
        snapshot = run['geometry'][2]
        before = physical_inputs(snapshot)
        # Native identities remain raw evidence. They are not physical comparison inputs.
        owner = snapshot['cargo'][0]['collider_owners'][0]
        owner['instance_id'] = -99
        owner['original_owner_instance_id'] = -100
        self.assertEqual(before, physical_inputs(snapshot))
        with self.assertRaisesRegex(ValueError, 'exact original audio'): validate_geometry(run)

    def test_destroyed_cargo_requires_complete_exact_source_audio_proof(self):
        corruptions = ('unknown', 'missing_proof', 'duplicate', 'persistent_loss', 'lost_record', 'owner_id', 'live_claim',
                       'missing_retired', 'missing_present', 'missing_original_owner_is_null', 'missing_owner_is_null',
                       'missing_removed', 'wrong_removed', 'future_removal')
        audio_fields = ('original_destroy_requested', 'object_null', 'component_null', 'collider_null',
                        'required', 'scope_member', 'created_after_close', 'created', 'destruction_requested',
                        'parent_observed', 'removed', 'source_stack', 'prefab', 'membership')
        for corruption in (*corruptions, *('audio_missing_'+f for f in audio_fields),
                           *('audio_false_'+f for f in ('original_destroy_requested', 'object_null', 'component_null', 'collider_null'))):
            with self.subTest(corruption=corruption):
                run = self.retirement_run()
                self.assertEqual(validate_geometry(run)['native_wheels'], 5)
                snapshot = run['geometry'][2]
                owner = snapshot['cargo'][0]['collider_owners'][0]
                audio = owner['retirement_audio'][0]
                if corruption == 'unknown': owner['instance_id'] = 999
                if corruption == 'live_claim': owner['present'] = True
                if corruption == 'missing_proof': owner['retirement_audio'] = []
                if corruption == 'duplicate': run['lifecycle']['audio'].append(deepcopy(audio))
                if corruption == 'persistent_loss': owner.update(retired=False, retirement_audio=[], removed=[])
                if corruption == 'lost_record': snapshot['cargo'][0]['collider_owners'] = []
                if corruption == 'owner_id': owner['original_owner_instance_id'] = 0
                if corruption.startswith('missing_') and corruption != 'missing_proof': del owner[corruption[8:]]
                if corruption == 'wrong_removed': owner['removed'][0]['frame'] = 99
                if corruption == 'future_removal': snapshot['frame'] = audio['removed']['frame'] - 1
                if corruption.startswith('audio_missing_'): del audio[corruption[14:]]
                if corruption.startswith('audio_false_'): audio[corruption[12:]] = False
                if corruption.startswith('audio_'):
                    # The matching global record must not rescue incomplete retirement proof.
                    run['lifecycle']['audio'][0] = deepcopy(audio)
                with self.assertRaises(ValueError): validate_geometry(run)

    def test_geometry_checkpoints_follow_the_declared_physical_path(self):
        isolated = self.sample_run('wheel_collider')
        self.assertNotIn('before-inertia-reset', {row['phase'] for row in isolated['geometry']})
        self.assertEqual(validate_geometry(isolated)['native_wheels'], 5)
        validate_run(isolated, self.scenario('wheel_collider'))
        assembled = self.sample_run()
        assembled['geometry'] = [row for row in assembled['geometry'] if row['phase'] != 'before-inertia-reset']
        with self.assertRaisesRegex(ValueError, 'Missing physical geometry checkpoints'):
            validate_geometry(assembled)
        isolated['isolated_wheel_probe'] = False
        with self.assertRaisesRegex(ValueError, 'physical source-body or isolated-probe path differs'):
            validate_geometry(isolated)

    def test_geometry_requires_ownership_and_complete_transform_chains(self):
        corruptions = (('missing_owner', 'explicit physical collider owner'),
                       ('missing_null_flag', 'explicit physical collider owner'),
                       ('conflicting_null', 'null ownership evidence conflicts'),
                       ('unknown_owner', 'owner body reference'),
                       ('missing_owner_transform', 'body ownership transform'),
                       ('missing_shape_transform', 'collider transform reference'),
                       ('empty_transforms', 'Missing physical transform records'),
                       ('missing_transform_record', 'collider transform reference'),
                       ('missing_parent', 'Incomplete physical local transform'),
                       ('missing_position', 'Incomplete physical local transform'),
                       ('incomplete_rotation', 'Incomplete physical local transform'),
                       ('missing_scale', 'Incomplete physical local transform'),
                       ('unknown_parent', 'Broken physical transform parent chain'),
                       ('cycle', 'Broken physical transform parent chain'),
                       ('owner_outside_chain', 'owner is outside its transform parent chain'))
        for corruption, message in corruptions:
            with self.subTest(corruption=corruption):
                run = self.sample_run()
                validate_geometry(run)
                snapshot = run['geometry'][0]
                shape = snapshot['colliders'][1]
                pose = next(row for row in snapshot['transforms'] if row['key'] == shape['transform'])
                if corruption == 'missing_owner': del shape['owner']
                if corruption == 'missing_null_flag': del shape['owner_is_null']
                if corruption == 'conflicting_null': shape['owner_is_null'] = True
                if corruption == 'unknown_owner': shape['owner'] = 'missing-body'
                if corruption == 'missing_owner_transform': snapshot['body']['transform_key'] = 'missing-transform'
                if corruption == 'missing_shape_transform': del shape['transform']
                if corruption == 'empty_transforms': snapshot['transforms'] = []
                if corruption == 'missing_transform_record': snapshot['transforms'].remove(pose)
                if corruption == 'missing_parent': del pose['parent']
                if corruption == 'missing_position': del pose['local_position']
                if corruption == 'incomplete_rotation': del pose['local_rotation']['w']
                if corruption == 'missing_scale': del pose['local_scale']
                if corruption == 'unknown_parent': pose['parent'] = 'missing-parent'
                if corruption == 'cycle': pose['parent'] = pose['key']
                if corruption == 'owner_outside_chain': pose['parent'] = 'scene-transform'
                with self.assertRaisesRegex(ValueError, message): validate_geometry(run)

    def test_transitional_native_owners_remain_valid_observations(self):
        run = self.sample_run()
        snapshot = deepcopy(run['geometry'][0])
        snapshot.update(phase='storage.lock.exit', physical_ready=False)
        cargo_pose = deepcopy(snapshot['transforms'][-1])
        cargo_pose.update(key='cargo-transform', parent='body-transform')
        snapshot['transforms'].append(cargo_pose)
        snapshot['nested_bodies'].append({'key': 'cargo-body', 'transform_key': 'cargo-transform', 'parent': 'body-transform'})
        snapshot['colliders'].append({'key': 'cargo-shape', 'type': 'BoxCollider', 'complete': True,
                                      'center': {'x': 0., 'y': 0., 'z': 0.}, 'size': {'x': 1., 'y': 1., 'z': 1.},
                                      'transform': 'cargo-transform', 'owner': 'cargo-body', 'owner_is_null': False})
        run['geometry'].insert(1, snapshot)
        self.assertEqual(validate_geometry(run)['checkpoints'], 12)

    def test_preflight_rejects_changed_source(self):
        with tempfile.TemporaryDirectory() as name:
            project = Path(name)
            stage = project / '.cache/reference-library'
            source = stage / 'source'
            source.mkdir(parents=True)
            original = source / 'unchanged-engine-data.bin'
            original.write_bytes(b'original archive data')
            steam = stage / 'steam-manifest.acf'
            steam.write_text('"appid" "1017180"\n"buildid" "11140890"')
            manifest = {'schema_version': 1, 'app_id': 1017180, 'build_id': 11140890,
                        'unity_version': '2021.3.18f1', 'serialized_versions': [],
                        'directories': [], 'files': [{'file': original.name,
                        'bytes': original.stat().st_size, 'sha256': sha256(original)}],
                        'steam_manifest': {'path': steam.name, 'sha256': sha256(steam)}}
            (stage / 'source-manifest.json').write_text(json.dumps(manifest))
            original.write_bytes(b'corrupt! archive data')
            self.assertEqual(original.stat().st_size, manifest['files'][0]['bytes'])
            self.assertNotEqual(sha256(original), manifest['files'][0]['sha256'])
            with self.assertRaisesRegex(ValueError, 'Snapshot file list, size, or hash changed'):
                preflight(source, project)

    @staticmethod
    def scenario(kind='assembled_lada'):
        return {'id': 'launch-default', 'probe_kind': kind, 'duration_frames': 4,
                'controls': [{'frame': 0}, {'frame': 2}]}

    @staticmethod
    def sample_run(kind='assembled_lada', initial_physics=False):
        run = {'schema_version': 1, 'scenario': 'launch-default', 'probe_kind': kind,
               'runtime': 'original-unity-player', 'completed': True,
               'isolated_wheel_probe': kind == 'wheel_collider',
               'menu_ready': True, 'lada_ready': True, 'control_capture_active': True,
               'physics_steps': 4 if initial_physics else 3, 'error': '', 'initial': {'engine_rpm': 0.}, 'samples': []}
        links = dict(input_sequence=0, engine_sequence=0, car_sequence=0, rpm_sequence=0)
        revisions = {'input.entry': 'input_sequence', 'engine.entry': 'engine_sequence',
                     'car.entry': 'car_sequence', 'rpm.exit': 'rpm_sequence'}
        step = 0
        # Native order: physics consumes preceding render outputs. Some frames have no step;
        # frame 1 has two. Engine order also changes without changing the link contract.
        for frame in range(5):
            phases = (['physics.entry', 'physics.exit'] * (2 if frame == 1 else 1)
                      if frame in (1, 3) or (frame == 0 and initial_physics) else [])
            engine = ['engine.entry', 'engine.exit']
            car = ['car.entry', 'control.entry', 'control.exit', 'autoshift.entry',
                   'autoshift.exit', 'rpm.entry', 'rpm.exit', 'car.exit']
            phases += ['input.entry', 'input.exit'] + (engine + car if frame % 2 else car + engine)
            for phase in phases:
                if phase == 'physics.entry': step += 1
                sequence = len(run['samples']) + 1
                if phase in revisions: links[revisions[phase]] = sequence
                run['samples'].append({'phase': phase, 'step': step, 'frame': frame,
                    'sequence': sequence, 'time': frame * .02, 'fixed_dt': .02,
                    'rpm': 0., **links, 'gear': 0, 'clutch': 0.,
                    'position': [0., 0., 0.], 'rotation': [0., 0., 0., 1.],
                    'velocity': [0., 0., 0.], 'angular_velocity': [0., 0., 0.],
                    'engine_output_nm': 0., 'engine_brake_nm': 0.,
                    'average_rpm': 0., 'torque_divisor': 2,
                    'wheels': [{'rpm': 0., 'motor_torque': 0., 'brake_torque': 0.,
                        'contact': False, 'radius': .3, 'suspension_distance': .3,
                        'suspension_offset_m': 0., 'pose': [0., 0., 0.],
                        'rotation': [0., 0., 0., 1.], 'contact_point': [0., 0., 0.],
                        'contact_normal': [0., 1., 0.], 'force_n': 0.,
                        'forward_slip': 0., 'sideways_slip': 0.} for _ in range(4)]})
        wheels = [{'key': f'wheel-{i}', 'type': 'WheelCollider', 'road_slot': i-1,
                   'complete': True, 'sprung_mass_available': True, 'enabled': i != 0,
                   'transform': f'wheel-transform-{i}', 'owner': 'car-body' if i else None, 'owner_is_null': i == 0,
                   'intended_enabled': i != 0, 'restored_enabled': i != 0,
                   'restoration_applied': True, 'center': {'x': 0., 'y': 0., 'z': 0.}, 'radius': .3, 'mass': 1., 'damping': .25,
                   'suspension_distance': .3, 'force_application_distance': 0., 'spring_n_per_m': 35000.,
                   'damper_n_s_per_m': 4500., 'spring_target': .5,
                   'forward': {'extremum_slip': .4, 'extremum_value': 1., 'asymptote_slip': .8, 'asymptote_value': .5, 'stiffness': 1.},
                   'sideways': {'extremum_slip': .2, 'extremum_value': 1., 'asymptote_slip': .5, 'asymptote_value': .75, 'stiffness': 1.}} for i in range(5)]
        phases = ('world.entry', 'source-physical-ready', 'prepared-held', 'backlog-drained-held',
                  'before-dynamic', 'switched-dynamic', 'before-inertia-reset', 'after-inertia-reset',
                  'canonical-transforms-synchronized', 'native-wheels-enabled', 'armed-after-canonical')
        if kind == 'wheel_collider':
            phases = tuple(phase for phase in phases if phase not in ('before-inertia-reset', 'after-inertia-reset'))
        transforms = [{'key': key, 'parent': parent,
                       'local_position': {'x': 0., 'y': 0., 'z': 0.},
                       'local_rotation': {'x': 0., 'y': 0., 'z': 0., 'w': 1.},
                       'local_scale': {'x': 1., 'y': 1., 'z': 1.},
                       'active_self': True, 'active_hierarchy': True}
                      for key, parent in [('scene-transform', None), ('body-transform', 'scene-transform'),
                                          *[(f'wheel-transform-{i}', 'body-transform') for i in range(5)]]]
        run['geometry'] = [{'phase': phase, 'complete': True, 'physical_ready': True, 'unavailable': [],
                            'colliders': deepcopy(wheels), 'transforms': deepcopy(transforms), 'cargo': [],
                            'body': {'key': 'car-body', 'transform_key': 'body-transform', 'parent': 'scene-transform'}, 'nested_bodies': []}
                           for phase in phases]
        for sample in run['samples']:
            sample['native_wheels'] = [{'sprung_mass_available': True} for _ in range(5)]
        driver = driver_fixture()
        run['initial'].update(driver.pop('initial'))
        run.update(driver)
        return lifecycle_fixture(run)

    @staticmethod
    def remove_sample(run, index):
        del run['samples'][index]
        numbers = {sample['sequence']: i for i, sample in enumerate(run['samples'], 1)}
        for sample in run['samples']:
            sample['sequence'] = numbers[sample['sequence']]
            for link in ('input_sequence', 'engine_sequence', 'car_sequence', 'rpm_sequence'):
                if sample[link]: sample[link] = numbers[sample[link]]

    def envelope(self, output):
        revision = {'driving_probe.py': 'a'*64, 'driving_capture.cs': 'b'*64,
                    'driving_capture.ps1': 'c'*64}
        (output / 'request.json').write_text(json.dumps({'repeats': 3,
            'observer_revision': revision,
            'scenarios': [self.scenario()]}))
        (output / 'preflight.json').write_text(json.dumps({'schema_version': 1,
            'app_id': 1017180, 'build_id': 11140890, 'unity_version': '2021.3.18f1',
            'native_engine': {'sha256': 'd'*64},
            'dependencies': {'TheLongDrive_Data/Managed/UnityEngine.PhysicsModule.dll': 'e'*64}}))
        scratch = output / 'scratch'
        scratch.mkdir()
        (scratch / 'patch-receipt.json').write_text(json.dumps({
            'changed_methods': ['System.Void carscript::Update()'], 'observer_revision': revision,
            'native_engine_sha256': 'd'*64, 'source_physics_module_sha256': 'e'*64}))
        for repeat in range(1, 4):
            directory = output / 'launch-default' / f'repeat-{repeat}'
            directory.mkdir(parents=True)
            run = self.sample_run()
            (directory / 'runtime.json').write_text(json.dumps(run))
            (directory / 'unity.log').write_text('G11_NATIVE_CAPTURE_FLUSHED completed=True')
            (directory / 'process.json').write_text(json.dumps({'pid': repeat,
                'executable': str(output / 'scratch/game/TheLongDrive.exe')}))
            (directory / 'exit.json').write_text(json.dumps({'exit_code': 0}))
            (directory / 'completion.json').write_text(json.dumps({'completed': True,
                'sample_count': len(run['samples']), 'physics_steps': run['physics_steps']}))
        return output

    def test_incomplete_capture_never_accepts(self):
        for change, message in (('missing_samples', 'samples are missing'),
                ('nonfinite', 'Nonfinite native sample'),
                ('missing_completion', 'completion marker'),
                ('missing_repeats', 'Missing or invalid evidence'),
                ('missing_marker_file', 'Missing or invalid evidence'),
                ('changed_initial', 'different initial states'),
                ('missing_wheel_metric', 'Missing native wheel metrics'),
                ('exception_before_flush', 'exception occurred before capture flush'),
                ('other_exception_after_flush', 'Unclassified native exception'),
                ('native_exit', 'Native player process evidence failed'),
                ('dropped_sample', 'Native callback sequence is invalid')):
            with self.subTest(change=change), tempfile.TemporaryDirectory() as name:
                output = self.envelope(Path(name))
                # This exact envelope must pass before each independent corruption.
                self.assertEqual(len(verify(output)['runs']), 3)
                path = output / 'launch-default/repeat-3/runtime.json'
                run = json.loads(path.read_text())
                if change == 'missing_samples': run['samples'] = []
                if change == 'nonfinite': run['samples'][0]['rpm'] = float('nan')
                if change == 'missing_completion': run['completed'] = False
                if change == 'missing_marker_file': path.with_name('completion.json').unlink()
                if change == 'changed_initial': run['initial']['engine_rpm'] = 1.
                if change == 'native_exit': path.with_name('exit.json').write_text(json.dumps({'exit_code': -1073741819}))
                if change == 'dropped_sample': del run['samples'][10]
                if change == 'missing_wheel_metric': del run['samples'][0]['wheels'][0]['rpm']
                if change == 'other_exception_after_flush':
                    path.with_name('unity.log').write_text('G11_NATIVE_CAPTURE_FLUSHED completed=True\nOtherException')
                if change == 'exception_before_flush':
                    path.with_name('unity.log').write_text('NullReferenceException\nG11_NATIVE_CAPTURE_FLUSHED completed=True')
                if change == 'missing_repeats': path.unlink()
                else: path.write_text(json.dumps(run))
                with self.assertRaisesRegex(ValueError, message): verify(output)
                self.assertFalse((output / 'receipt.json').exists())
        with tempfile.TemporaryDirectory() as name, patch('tools.reference.driving_probe.preflight', return_value={}):
            output = Path(name) / 'not-created'
            with self.assertRaisesRegex(ValueError, 'Three fresh native repeats'):
                capture(Path(name), Path(name), output, Path(name) / 'scenarios.json', 2)
            self.assertFalse((output / 'receipt.json').exists())

    def test_capture_reuses_presolve_setup(self):
        with tempfile.TemporaryDirectory() as name:
            project = Path(name).resolve()
            output = project / '.cache/g11-driving/reset'
            definitions = project / 'scenarios.json'
            row = dict(self.scenario(), source_objects={}, setup={}, frame_schedule={})
            definitions.write_text(json.dumps({'scenarios': [row]}))
            setup = {'engine_rpm': 0., 'engine_temperature': 0., 'random_state': 'canonical-before-solve',
                     'driver_input': driver_fixture()['driver']['input']}
            observed = dict(setup, engine_temperature=1.)
            launched = []
            def native(arguments, **kwargs):
                if arguments[arguments.index('-Mode') + 1] == 'Run':
                    directory = Path(arguments[arguments.index('-RunRoot') + 1])
                    launched.append(json.loads((directory / 'scenario.json').read_text()))
                    run = self.sample_run()
                    run.update(setup_state=setup, initial=observed)
                    (directory / 'runtime.json').write_text(json.dumps(run))
                return type('Result', (), {'returncode': 0})()
            with patch('tools.reference.driving_probe.PROJECT', project), \
                 patch('tools.reference.driving_probe.preflight', return_value={'powershell': 'pwsh', 'observer_revision': {}}), \
                 patch('tools.reference.driving_probe.subprocess.run', side_effect=native), \
                 patch('tools.reference.driving_probe.verify', return_value={'runs': [1, 2, 3]}), \
                 patch('tools.reference.driving_probe.validate_evidence', side_effect=lambda output, directory, run, row: validate_run(run, row)):
                capture(project, project, output, definitions, 3)
            self.assertEqual(len(launched), 3)
            self.assertNotIn('setup_state', launched[0])
            for scenario in launched[1:]:
                self.assertEqual(scenario['setup_state'], setup)
                self.assertNotEqual(scenario['setup_state'], observed)
                self.assertTrue(scenario['has_setup_state'])
                self.assertNotIn('initial_state', scenario)

    def test_capture_stops_on_first_invalid_run(self):
        for corruption, expected_runs, message in (('readiness', 1, 'must initialize'),
                ('runtime', 1, 'exception occurred before capture flush'),
                ('initial', 2, 'different initial states')):
            with self.subTest(corruption=corruption), tempfile.TemporaryDirectory() as name:
                project = Path(name).resolve()
                evidence = project / 'evidence'
                evidence.mkdir()
                self.envelope(evidence)
                self.assertEqual(len(verify(evidence)['runs']), 3)
                output = project / '.cache/g11-driving/reset'
                definitions = project / 'scenarios.json'
                row = dict(self.scenario(), source_objects={}, setup={}, frame_schedule={})
                definitions.write_text(json.dumps({'scenarios': [row, dict(row, id='later-scenario')]}))
                launched = []
                def native(arguments, **kwargs):
                    if arguments[arguments.index('-Mode') + 1] == 'Run':
                        directory = Path(arguments[arguments.index('-RunRoot') + 1])
                        launched.append(directory)
                        run = self.sample_run()
                        run['setup_state']['canonical'] = True
                        if corruption == 'readiness': run['lada_ready'] = False
                        if corruption == 'initial' and len(launched) == 2: run['initial']['engine_rpm'] = 1.
                        for path in (evidence / 'launch-default/repeat-1').iterdir():
                            if path.is_file(): (directory / path.name).write_bytes(path.read_bytes())
                        (directory / 'runtime.json').write_text(json.dumps(run))
                        (directory / 'process.json').write_text(json.dumps({'pid': 1,
                            'executable': str(output / 'scratch/game/TheLongDrive.exe')}))
                        if corruption == 'runtime':
                            (directory / 'unity.log').write_text('OtherException\nG11_NATIVE_CAPTURE_FLUSHED completed=True')
                    return type('Result', (), {'returncode': 0})()
                with patch('tools.reference.driving_probe.PROJECT', project), \
                     patch('tools.reference.driving_probe.preflight', return_value={'powershell': 'pwsh', 'observer_revision': {}}), \
                     patch('tools.reference.driving_probe.subprocess.run', side_effect=native):
                    with self.assertRaisesRegex(ValueError, message):
                        capture(project, project, output, definitions, 3)
                self.assertEqual(len(launched), expected_runs)
                self.assertTrue((launched[-1] / 'runtime.json').exists())
                self.assertFalse((output / 'receipt.json').exists())

    def test_capture_requires_declared_interval(self):
        for change, message in (('truncated', 'declared observation interval'),
                ('missing_frame', 'frame coverage'),
                ('rewound_origin', 'declared observation interval'),
                ('final_control_not_consumed', 'control interval'),
                ('event_after_end', 'control schedule'),
                ('event_at_end', 'control schedule')):
            with self.subTest(change=change), tempfile.TemporaryDirectory() as name:
                output = self.envelope(Path(name))
                self.assertEqual(len(verify(output)['runs']), 3)
                request_path = output / 'request.json'
                request = json.loads(request_path.read_text())
                row = request['scenarios'][0]
                path = output / 'launch-default/repeat-3/runtime.json'
                run = json.loads(path.read_text())
                if change == 'truncated': row['duration_frames'] = 240
                if change == 'rewound_origin': run['samples'][0]['frame'] = 1
                if change == 'missing_frame':
                    for sample in run['samples']:
                        if sample['frame'] == 2: sample['frame'] = 1
                if change == 'final_control_not_consumed': row['controls'][-1]['frame'] = 3
                if change == 'event_after_end': row['controls'][-1]['frame'] = 5
                if change == 'event_at_end': row['controls'][-1]['frame'] = 4
                request_path.write_text(json.dumps(request))
                path.write_text(json.dumps(run))
                with self.assertRaisesRegex(ValueError, message): verify(output)
                self.assertFalse((output / 'receipt.json').exists())

    def test_capture_requires_valid_callback_links(self):
        for change, message in (('negative', 'callback link'),
                ('outside_samples', 'callback link'),
                ('wrong_callback', 'callback link'),
                ('future', 'callback link'),
                ('stale', 'callback link'),
                ('missing_revision', 'callback link'),
                ('missing_pair', 'callback pair'),
                ('missing_entire_pair', 'callback pair')):
            with self.subTest(change=change), tempfile.TemporaryDirectory() as name:
                output = self.envelope(Path(name))
                self.assertEqual(len(verify(output)['runs']), 3)
                path = output / 'launch-default/repeat-3/runtime.json'
                run = json.loads(path.read_text())
                physics = [s for s in run['samples'] if s['phase'] == 'physics.entry']
                consumer = physics[0]
                if change == 'negative': consumer['input_sequence'] = -100
                if change == 'outside_samples': consumer['engine_sequence'] = 100000
                if change == 'wrong_callback': consumer['car_sequence'] = consumer['input_sequence']
                if change == 'future':
                    consumer['engine_sequence'] = next(s['sequence'] for s in run['samples']
                        if s['phase'] == 'engine.entry' and s['sequence'] > consumer['sequence'])
                if change == 'stale': physics[-1]['input_sequence'] = physics[0]['input_sequence']
                if change == 'missing_revision': consumer['rpm_sequence'] = 0
                if change == 'missing_pair':
                    self.remove_sample(run, next(i for i, s in enumerate(run['samples'])
                        if s['phase'] == 'engine.exit'))
                    path.with_name('completion.json').write_text(json.dumps({'completed': True,
                        'sample_count': len(run['samples']), 'physics_steps': run['physics_steps']}))
                if change == 'missing_entire_pair':
                    for phase in ('control.entry', 'control.exit'):
                        self.remove_sample(run, next(i for i, s in enumerate(run['samples'])
                            if s['phase'] == phase and s['frame'] == 2))
                    path.with_name('completion.json').write_text(json.dumps({'completed': True,
                        'sample_count': len(run['samples']), 'physics_steps': run['physics_steps']}))
                path.write_text(json.dumps(run))
                with self.assertRaisesRegex(ValueError, message): verify(output)
                self.assertFalse((output / 'receipt.json').exists())

        # A retained native run starts with an explicitly unlinked physics prefix.
        result = validate_run(self.sample_run(initial_physics=True), self.scenario())
        self.assertEqual(result['initial_unlinked_physics_steps'], [1])

    def test_capture_requires_native_phases(self):
        for kind in ('assembled_lada', 'wheel_collider'):
            run = self.sample_run(kind)
            run['runtime'] = 'formula-only'
            with self.assertRaises(ValueError): validate_run(run, self.scenario(kind))
        run = self.sample_run()
        run['samples'] = [s for s in run['samples'] if s['phase'] != 'engine.exit']
        with self.assertRaises(ValueError): validate_run(run, self.scenario())
        validate_run(self.sample_run(), self.scenario())


if __name__ == '__main__':
    unittest.main()
