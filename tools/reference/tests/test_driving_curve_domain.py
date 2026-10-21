"""Curve-only qualification uses reported curve inputs. Physical gates stay exact."""
from contextlib import ExitStack
from copy import deepcopy
import json
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

from tools.reference import driving_probe as probe
from tools.reference.driving_curve import bits
from tools.reference.tests.test_driving_curve_inputs import scenario, torque_row, PREFIX
from tools.reference.tests import test_driving_probe as probe_fixtures
from tools.reference.tests.test_driving_controls import scenario_fixture


def curve_scenario(holdouts):
    row = scenario(complete='confirmation' if holdouts == 'confirmation' else holdouts == 'complete')
    if not holdouts:
        row['id'] = 'curve-boundaries'
        del row['engine_torque_holdout_inputs']; del row['engine_torque_holdout_provenance']
    return row


def observed_run(row, repeat):
    # Reuse the complete existing hold/geometry/driver/lifecycle fixture graph.
    run = probe_fixtures.DrivingProbeTests.sample_run(row['probe_kind'])
    run['scenario'] = row['id']
    if row['probe_kind'] == 'curve':
        engine = torque_row(complete='confirmation' if row['id'] == 'curve-predictor-confirmation' else row['id'] == 'curve-complete-identification')
        if row['id'] == 'curve-boundaries':
            engine['inputs'] = PREFIX.copy(); engine['values'] = engine['values'][:52]
        run['curves'] = [engine, dict(owner='carscript', name='other', inputs=[0.], values=[.5])]
    # Six finite later chassis leaves vary. They are not curve Evaluate arguments.
    run['initial'].update(inertia_tensor=dict(x=1., y=2., z=3.+repeat*.001),
        inertia_tensor_rotation=dict(x=.01+repeat*.001, y=.02+repeat*.001, z=.03+repeat*.001, w=1.),
        rotation=dict(x=repeat*1e-10, y=.7, z=repeat*2e-10, w=.7))
    return run


def write_run(directory, output, run, repeat):
    directory.mkdir(parents=True, exist_ok=True)
    stored = deepcopy(run)
    if stored.get('curves') and stored['curves'][0]['values'][0] == .25:
        stored['curves'][0]['values'][0] = 'LOSSLESS_CURVE_TOKEN'
    probe.write_json(directory / 'runtime.json', stored)
    raw = (directory / 'runtime.json').read_text().replace('"LOSSLESS_CURVE_TOKEN"', '0.2500000000000000000000000000001')
    (directory / 'runtime.json').write_text(raw)
    probe.write_json(directory / 'process.json', dict(pid=repeat, executable=str(output/'scratch/game/TheLongDrive.exe')))
    probe.write_json(directory / 'exit.json', dict(exit_code=0))
    probe.write_json(directory / 'completion.json', dict(completed=True, sample_count=len(run['samples']), physics_steps=run['physics_steps']))
    (directory / 'unity.log').write_text('G11_NATIVE_CAPTURE_FLUSHED completed=True')


class CurveDomainTests(unittest.TestCase):
    GATES = ('validate_timing', 'validate_controls', 'validate_body_hold', 'validate_geometry', 'validate_driver', 'validate_lifecycle')

    def execute(self, operation, row, output, change=None, fail_gate=None, missing_repeat=None, file_change=None, campaign_change=None):
        project = output.parents[2]
        pre = dict(powershell='pwsh', observer_revision=probe.observer_revision(), app_id=1017180,
            build_id=11140890, unity_version='2021.3.18f1', native_engine={'sha256':'engine'},
            dependencies={'TheLongDrive_Data/Managed/UnityEngine.PhysicsModule.dll':'physics'})
        calls = dict.fromkeys(self.GATES, 0); launched = []
        self.last_calls = calls; self.last_launched = launched
        def setup():
            (output/'scratch').mkdir()
            probe.write_json(output/'scratch/patch-receipt.json', dict(changed_methods=['original'],
                observer_revision=pre['observer_revision'], native_engine_sha256='engine', source_physics_module_sha256='physics'))
        def save(directory, repeat):
            if repeat == missing_repeat: return
            run = observed_run(row, repeat)
            if change is not None: change(run, repeat)
            write_run(directory, output, run, repeat)
            if file_change is not None: file_change(directory, repeat)
            if repeat == 3 and campaign_change is not None: campaign_change(output)
        def native(argv, **kwargs):
            if argv[argv.index('-Mode')+1] == 'Setup': setup()
            else:
                directory = Path(argv[argv.index('-RunRoot')+1]); launched.append(directory)
                save(directory, len(launched))
            return type('Result', (), {'returncode':0})()
        def gate(name, original):
            def checked(*args):
                calls[name] += 1
                if fail_gate == name and calls[name] == 2: raise ValueError('injected '+name)
                # Only timing/control declarations adapt to the existing four-frame fixture.
                # Production scenario files and every other evidence gate remain unchanged.
                if row['probe_kind'] == 'curve' and name in ('validate_timing','validate_controls'):
                    return original(args[0], scenario_fixture('curve'))
                return original(*args)
            return checked
        with ExitStack() as stack:
            for name in self.GATES:
                stack.enter_context(patch.object(probe, name, side_effect=gate(name, getattr(probe,name))))
            if operation == 'capture':
                definitions = project/'scenarios.json'; probe.write_json(definitions, dict(scenarios=[row]))
                stack.enter_context(patch.object(probe,'PROJECT',project))
                stack.enter_context(patch.object(probe,'preflight',return_value=pre))
                stack.enter_context(patch.object(probe.subprocess,'run',side_effect=native))
                result = probe.capture(project, project, output, definitions, 3)
            else:
                output.mkdir(parents=True); setup()
                probe.write_json(output/'request.json', dict(repeats=3, scenarios=[row], observer_revision=pre['observer_revision']))
                probe.write_json(output/'preflight.json', pre)
                for repeat in (1,2,3): save(output/row['id']/f'repeat-{repeat}', repeat)
                result = probe.verify(output)
        return result, calls, launched

    def test_curve_rows_accept_later_body_variation_in_capture_and_verify(self):
        for holdouts in (False, True, 'complete', 'confirmation'):
            for operation in ('capture','verify'):
                with self.subTest(holdouts=holdouts, operation=operation), tempfile.TemporaryDirectory() as name:
                    output = Path(name).resolve()/'.cache/g11-driving/new'; row = curve_scenario(holdouts)
                    result, calls, launched = self.execute(operation,row,output)
                    self.assertEqual(len(result['runs']),3)
                    for run in result['runs']:
                        self.assertEqual(run['qualification_domain']['kind'],'native-engine-curve')
                        self.assertIs(run['qualification_domain']['physical_measurements_qualified'],False)
                        self.assertEqual(run['qualification_domain']['qualified_measurements'], ['enginescript.torqueCurve.Evaluate'])
                    expected_count = 244 if holdouts == 'confirmation' else 366 if holdouts == 'complete' else 77 if holdouts else 52
                    states = [item['engine_curve']['input_state'] for item in result['runs']]
                    self.assertEqual(states, [states[0]]*3)
                    state = states[0]
                    self.assertEqual(set(state), {'owner','name','keys','pre_wrap','post_wrap','input_bits'})
                    self.assertEqual((state['owner'],state['name']), ('enginescript','torqueCurve'))
                    self.assertEqual((state['pre_wrap'],state['post_wrap']), (8,8))
                    self.assertEqual(len(state['keys']),13)
                    self.assertTrue(all(set(key)=={'time','value','in_tangent','out_tangent','in_weight','out_weight','weighted_mode'} for key in state['keys']))
                    self.assertEqual(len(state['input_bits']),expected_count)
                    for item in result['runs']:
                        self.assertEqual(item['engine_curve']['value_bits'], [bits(float(i)+.25) for i in range(expected_count)])
                        self.assertEqual(item['engine_curve']['native_value_decimals'][0], '0.2500000000000000000000000000001')
                        self.assertEqual(item['qualification_domain']['unqualified_context'], ['chassis','driving','suspension'])
                    self.assertEqual(set(calls.values()), {6 if operation=='capture' else 3})
                    self.assertEqual(len(launched),3 if operation=='capture' else 0)
                    raw = [probe.read_json(output/row['id']/f'repeat-{i}/runtime.json') for i in (1,2,3)]
                    self.assertEqual(len({r['initial']['inertia_tensor']['z'] for r in raw}),3)
                    self.assertTrue(all(len(r['curves'])==2 for r in raw))
                    json.dumps(result,allow_nan=False)
                    if operation == 'capture': self.assertEqual(probe.read_json(output/'receipt.json'),result)


    def test_both_curve_rows_reject_changed_inputs_and_output_bits(self):
        for holdouts in (False, True, 'complete', 'confirmation'):
            for operation in ('capture','verify'):
                for change in ('key','wrap','wrap type','input','suffix','output','missing curve','duplicate curve'):
                    with self.subTest(holdouts=holdouts,operation=operation,change=change), tempfile.TemporaryDirectory() as name:
                        row = curve_scenario(holdouts); output = Path(name).resolve()/'.cache/g11-driving/new'
                        def corrupt(run, repeat):
                            if repeat != 2: return
                            engine = run['curves'][0]
                            if change == 'missing curve': run['curves'] = []
                            if change == 'duplicate curve': run['curves'].append(deepcopy(engine))
                            if change == 'key': engine['keys'][0]['out_tangent'] += .001
                            if change == 'wrap': engine['pre_wrap'] = 2
                            if change == 'wrap type': engine['pre_wrap'] = 8.0
                            if change == 'suffix': engine['inputs'][-1] += .25
                            if change == 'input': engine['inputs'][0] = -0.
                            if change == 'output': engine['values'][0] += .5
                        message = {'key':'key bits differ','wrap':'wraps differ','wrap type':'wraps differ',
                                   'suffix':'prefix or suffix bits differ','input':'input|prefix or suffix bits differ',
                                   'output':'different engine torque curve bits',
                                   'missing curve':'identity or count differs','duplicate curve':'identity or count differs'}[change]
                        with self.assertRaisesRegex(ValueError,message): self.execute(operation,row,output,change=corrupt)
                        self.assertFalse((output/'receipt.json').exists())
                        if operation == 'capture': self.assertEqual(len(self.last_launched),2)

    def test_both_curve_rows_require_all_three_repeats(self):
        for holdouts in (False, True, 'complete', 'confirmation'):
            for operation in ('capture','verify'):
                with self.subTest(holdouts=holdouts,operation=operation), tempfile.TemporaryDirectory() as name:
                    row = curve_scenario(holdouts); output = Path(name).resolve()/'.cache/g11-driving/new'
                    with self.assertRaisesRegex(ValueError,'Missing or invalid evidence'):
                        self.execute(operation,row,output,missing_repeat=3)
                    self.assertFalse((output/'receipt.json').exists())

    def test_every_per_run_gate_still_rejects_before_curve_qualification(self):
        for holdouts in (True, 'complete', 'confirmation'):
            for operation in ('capture','verify'):
                for gate in self.GATES:
                    with self.subTest(operation=operation,gate=gate), tempfile.TemporaryDirectory() as name:
                        row=curve_scenario(holdouts); output=Path(name).resolve()/'.cache/g11-driving/new'
                        with self.assertRaisesRegex(ValueError,'injected '+gate):
                            self.execute(operation,row,output,fail_gate=gate)
                        self.assertEqual(self.last_calls[gate],2)
                        self.assertFalse((output/'receipt.json').exists())
                        if operation=='capture': self.assertEqual(len(self.last_launched),2)

    def test_initial_and_raw_log_gates_remain_required_for_curve_rows(self):
        for holdouts in (True, 'complete', 'confirmation'):
            for operation in ('capture','verify'):
                for failure in ('initial','log','completion','exit'):
                    with self.subTest(operation=operation,failure=failure), tempfile.TemporaryDirectory() as name:
                        row=curve_scenario(holdouts); output=Path(name).resolve()/'.cache/g11-driving/new'
                        def change(run,repeat):
                            if repeat==2 and failure=='initial': run['initial']={}
                        def change_file(directory,repeat):
                            if repeat!=2: return
                            if failure=='log': (directory/'unity.log').write_text('NullReferenceException\nG11_NATIVE_CAPTURE_FLUSHED completed=True')
                            if failure=='completion': probe.write_json(directory/'completion.json',dict(completed=False))
                            if failure=='exit': probe.write_json(directory/'exit.json',dict(exit_code=1))
                        message={'initial':'initial state is missing','log':'exception occurred before capture flush',
                                 'completion':'completion marker differs','exit':'player process evidence failed'}[failure]
                        with self.assertRaisesRegex(ValueError,message):
                            self.execute(operation,row,output,change=change,file_change=change_file)
                        self.assertFalse((output/'receipt.json').exists())

    def test_complete_capture_and_verify_keep_request_and_source_provenance_gates(self):
        defects = {
            'request repeats': ('request.json', lambda d: d.update(repeats=2), 'Three fresh native repeats'),
            'request scenario': ('request.json', lambda d: d['scenarios'][0][
                'engine_torque_holdout_inputs'].__setitem__(0, 0.), 'ordered exact binary32'),
            'observer': ('scratch/patch-receipt.json', lambda d: d.update(observer_revision={}), 'patch receipt differs'),
            'app': ('preflight.json', lambda d: d.update(app_id=0), 'build provenance differs'),
            'build': ('preflight.json', lambda d: d.update(build_id=0), 'build provenance differs'),
            'unity': ('preflight.json', lambda d: d.update(unity_version='changed'), 'build provenance differs'),
            'engine': ('preflight.json', lambda d: d['native_engine'].update(sha256='changed'), 'physics dependencies'),
            'physics': ('scratch/patch-receipt.json', lambda d: d.update(
                source_physics_module_sha256='changed'), 'physics dependencies'),
        }
        for holdouts, operation in ((h, op) for h in ('complete', 'confirmation') for op in ('capture', 'verify')):
            for name, (filename, change, message) in defects.items():
                with self.subTest(operation=operation, defect=name), tempfile.TemporaryDirectory() as directory:
                    output = Path(directory).resolve()/'.cache/g11-driving/new'
                    def corrupt(root):
                        path = root/filename
                        data = probe.read_json(path); change(data); probe.write_json(path, data)
                    with self.assertRaisesRegex(ValueError, message):
                        self.execute(operation, curve_scenario(holdouts), output, campaign_change=corrupt)
                    self.assertFalse((output/'receipt.json').exists())

    def test_complete_audio_and_process_gates_stop_at_second_repeat(self):
        for holdouts, operation in ((h, op) for h in ('complete', 'confirmation') for op in ('capture', 'verify')):
            for defect in ('audio', 'process'):
                with self.subTest(operation=operation, defect=defect), tempfile.TemporaryDirectory() as directory:
                    output = Path(directory).resolve()/'.cache/g11-driving/new'
                    def corrupt(run, repeat):
                        if repeat == 2 and defect == 'audio':
                            run['lifecycle']['audio'][0]['original_destroy_requested'] = False
                    def corrupt_file(path, repeat):
                        if repeat == 2 and defect == 'process':
                            probe.write_json(path/'process.json', dict(pid=0, executable='changed'))
                    message = 'audio creation or destruction provenance' if defect == 'audio' else 'process evidence failed'
                    with self.assertRaisesRegex(ValueError, message):
                        self.execute(operation, curve_scenario(holdouts), output, change=corrupt, file_change=corrupt_file)
                    self.assertFalse((output/'receipt.json').exists())
                    if operation == 'capture': self.assertEqual(len(self.last_launched), 2)

    def test_preflight_keeps_native_engine_archive_and_core_module_gates(self):
        # Exercise real preflight dispatch with a minimal checked-source stand-in.
        for defect in ('none', 'archive', 'engine', 'CoreModule'):
            with self.subTest(defect=defect), tempfile.TemporaryDirectory() as directory:
                root = Path(directory).resolve()
                source = root/'source'; source.mkdir()
                required = ('TheLongDrive.exe', 'MonoBleedingEdge/EmbedRuntime/mono-2.0-bdwgc.dll',
                    'TheLongDrive_Data/Managed/Unity.Burst.Cecil.dll',
                    'TheLongDrive_Data/Managed/UnityEngine.PhysicsModule.dll',
                    'TheLongDrive_Data/Managed/UnityEngine.CoreModule.dll',
                    'TheLongDrive_Data/ScriptingAssemblies.json')
                for filename in required:
                    path = source/filename; path.parent.mkdir(parents=True, exist_ok=True); path.write_bytes(b'source')
                compiler = root/'compiler'; compiler.mkdir()
                for filename in ('Microsoft.CodeAnalysis.dll', 'Microsoft.CodeAnalysis.CSharp.dll'):
                    (compiler/filename).write_bytes(b'compiler')
                snapshot = SimpleNamespace(root=source, manifest_path=root/'archive/manifest.json',
                    manifest_sha256='archive', files=[SimpleNamespace(file=name, sha256='checked-'+name, bytes=6)
                    for name in required if not (defect == 'CoreModule' and name.endswith('CoreModule.dll'))])
                def metadata(path):
                    if path.name == 'manifest.json' and path.parent.name == 'library':
                        return dict(source_manifest_sha256='archive', catalog_sha256='catalog')
                    return dict(files=[dict(file='managed/Assembly-CSharp.cs', sha256=probe.DECOMPILED_SHA256)])
                with patch.object(probe, 'load_snapshot', return_value=snapshot), \
                     patch.object(probe, 'verify_snapshot', side_effect=ValueError('changed source archive') if defect == 'archive' else None) as archive, \
                     patch.object(probe, 'native_engine_evidence', side_effect=ValueError('changed native engine') if defect == 'engine' else None,
                                  return_value=dict(sha256='engine')) as engine, \
                     patch.object(probe, 'read_json', side_effect=metadata), \
                     patch.object(probe, 'LibraryReader', side_effect=lambda receipt: SimpleNamespace(receipt=receipt)), \
                     patch.object(probe, 'checked'), patch.object(probe, 'source_geometry_map', return_value={}), \
                     patch.object(probe, 'sha256', side_effect=lambda path: probe.ASSEMBLY_SHA256 if path.name == 'Assembly-CSharp.dll' else 'checked'), \
                     patch.object(probe.shutil, 'which', return_value=str(compiler/'pwsh.exe')):
                    if defect == 'none':
                        result = probe.preflight(source, root)
                        self.assertEqual(result['dependencies']['TheLongDrive_Data/Managed/UnityEngine.CoreModule.dll'],
                                         'checked-TheLongDrive_Data/Managed/UnityEngine.CoreModule.dll')
                        self.assertEqual((result['app_id'], result['build_id'], result['unity_version']),
                                         (1017180, 11140890, '2021.3.18f1'))
                        archive.assert_called_once_with(snapshot); engine.assert_called_once_with(source)
                    else:
                        message = {'archive': 'changed source archive', 'engine': 'changed native engine',
                                   'CoreModule': 'Missing checked native capture dependency.*CoreModule'}[defect]
                        with self.assertRaisesRegex(ValueError, message): probe.preflight(source, root)

    def test_physical_rows_still_reject_the_same_body_variation(self):
        for kind in ('assembled_lada','wheel_collider'):
            for operation in ('capture','verify'):
                with self.subTest(kind=kind,operation=operation), tempfile.TemporaryDirectory() as name:
                    row=dict(scenario_fixture(kind),source_objects={},frame_schedule={})
                    output=Path(name).resolve()/'.cache/g11-driving/new'
                    with self.assertRaisesRegex(ValueError,'different initial states'):
                        self.execute(operation,row,output)
                    self.assertFalse((output/'receipt.json').exists())
                    self.assertEqual(set(self.last_calls.values()),{2})
                    if operation=='capture': self.assertEqual(len(self.last_launched),2)


if __name__ == '__main__': unittest.main()
