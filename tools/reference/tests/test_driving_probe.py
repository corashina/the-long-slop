"""Critical G11 provenance and native evidence gates."""
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from tools.reference.driving_probe import preflight, capture, verify, validate_run
from tools.reference.library.archive import sha256


class DrivingProbeTests(unittest.TestCase):
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
    def sample_run(kind='assembled_lada'):
        return {'schema_version': 1, 'scenario': 'launch-default', 'probe_kind': kind,
                'runtime': 'original-unity-player', 'completed': True,
                'menu_ready': True, 'lada_ready': True, 'control_capture_active': True,
                'physics_steps': 2, 'error': '', 'initial': {'engine_rpm': 0.},
                'samples': [{'phase': phase, 'step': step, 'frame': step, 'sequence': i,
                             'time': step * .02, 'fixed_dt': .02, 'rpm': 0.,
                             'input_sequence': 1, 'engine_sequence': 2, 'car_sequence': 3,
                             'rpm_sequence': 4, 'gear': 0, 'clutch': 0.,
                             'position': [0.,0.,0.], 'rotation': [0.,0.,0.,1.],
                             'velocity': [0.,0.,0.], 'angular_velocity': [0.,0.,0.],
                             'engine_output_nm': 0., 'engine_brake_nm': 0., 'average_rpm': 0., 'torque_divisor': 2,
                             'wheels': [{'rpm': 0., 'motor_torque': 0., 'brake_torque': 0.,
                                'contact': False, 'radius': .3, 'suspension_distance': .3,
                                'suspension_offset_m': 0., 'pose': [0.,0.,0.],
                                'rotation': [0.,0.,0.,1.], 'contact_point': [0.,0.,0.],
                                'contact_normal': [0.,1.,0.], 'force_n': 0.,
                                'forward_slip': 0., 'sideways_slip': 0.} for _ in range(4)]}
                            for i, (step, phase) in enumerate([(1, 'input.entry'), (1, 'input.exit'),
                                (1, 'car.entry'), (1, 'car.exit'), (1, 'engine.entry'),
                                (1, 'engine.exit'), (1, 'control.entry'), (1, 'control.exit'),
                                (1, 'rpm.entry'), (1, 'rpm.exit'), (1, 'physics.entry'), (1, 'physics.exit'),
                                (2, 'physics.entry'), (2, 'physics.exit')], 1)]}

    def envelope(self, output):
        revision = {'driving_probe.py': 'a'*64, 'driving_capture.cs': 'b'*64,
                    'driving_capture.ps1': 'c'*64}
        (output / 'request.json').write_text(json.dumps({'repeats': 3,
            'observer_revision': revision,
            'scenarios': [{'id': 'launch-default', 'probe_kind': 'assembled_lada'}]}))
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

    def test_capture_requires_native_phases(self):
        for kind in ('assembled_lada', 'wheel_collider'):
            run = self.sample_run(kind)
            run['runtime'] = 'formula-only'
            with self.assertRaises(ValueError): validate_run(run, {'id': 'launch-default', 'probe_kind': kind})
        run = self.sample_run()
        run['samples'] = [s for s in run['samples'] if s['phase'] != 'engine.exit']
        with self.assertRaises(ValueError): validate_run(run, {'id': 'launch-default', 'probe_kind': 'assembled_lada'})
        validate_run(self.sample_run(), {'id': 'launch-default', 'probe_kind': 'assembled_lada'})


if __name__ == '__main__':
    unittest.main()
