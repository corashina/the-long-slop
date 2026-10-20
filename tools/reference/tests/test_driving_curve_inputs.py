"""Execute actual curve observers and reject changed frozen query boundaries."""
from copy import deepcopy
from decimal import Decimal
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from tools.reference.driving import method
from tools.reference import driving_probe as probe
from tools.reference.driving_curve import (
    SCENARIO_ID, INPUT_FIELD, PROVENANCE_FIELD, INPUT_BITS, PROVENANCE,
    bits, original_inputs, serialized_torque_curve, validate_curve_scenario, validate_curve_run)

ROOT = Path(__file__).resolve().parents[1]
FROZEN_INPUTS = [31.25, 656.25, 687.5, 1437.5, 1468.75, 1984.375, 2390.625,
                 2484.375, 2890.625, 2968.75, 3437.5, 4265.625, 4484.375, 4406.25,
                 4718.75, 4890.625, 4906.25, 5187.5, 5343.75, 5359.375, 5856.25,
                 5875., 5931.25, 6553.125, 6240.625]
PREFIX = [0., -6600., 0., 1000., 1500., 2000., 2500., 3000., 3500., 4000.,
          4500., 5000., 5500., 6100., 6600., 13200., 250., 500., 750.,
          1125., 1250., 1375., 1625., 1750., 1875., 2125., 2250., 2375.,
          2625., 2750., 2875., 3125., 3250., 3375., 3625., 3750., 3875.,
          4125., 4250., 4375., 4625., 4750., 4875., 5125., 5250., 5375.,
          5650., 5800., 5950., 6225., 6350., 6475.]


def scenario():
    document = json.loads((ROOT.parents[1] / 'tests/fixtures/driving/scenarios.json').read_text())
    row = deepcopy(next(row for row in document['scenarios'] if row['id'] == 'curve-boundaries'))
    return dict(row, id=SCENARIO_ID, **{INPUT_FIELD: FROZEN_INPUTS.copy(), PROVENANCE_FIELD: dict(PROVENANCE)})


def torque_row():
    return dict(owner='enginescript', name='torqueCurve', keys=serialized_torque_curve()['keys'],
                pre_wrap=8, post_wrap=8, inputs=PREFIX + FROZEN_INPUTS,
                values=[float(i) + .25 for i in range(77)])


def raw_car_curve_run(token):
    run = dict(scenario=SCENARIO_ID, probe_kind='curve', runtime='original-unity-player',
               curves=[torque_row(), dict(owner='carscript', name='other', inputs=['CAR_INPUT_TOKEN'])])
    return json.dumps(run).replace('"CAR_INPUT_TOKEN"', token)


class DrivingCurveInputTests(unittest.TestCase):
    def test_actual_observer_methods_keep_prefix_and_dispatch_only_engine_suffix(self):
        capture = (ROOT / 'driving_capture.cs').read_text()
        methods = ''.join(method(capture, marker)[0] for marker in (
            '[Serializable] public class Key', '[Serializable] public class Curve',
            'static void CaptureCurves(', 'static void CaptureCurve('))
        fields = {'time': 'time', 'value': 'value', 'in_tangent': 'inTangent',
                  'out_tangent': 'outTangent', 'in_weight': 'inWeight', 'out_weight': 'outWeight',
                  'weighted_mode': 'weightedMode'}
        keys = ','.join('new Keyframe{' + ','.join(
            fields[k] + '=' + (str(v) if k == 'weighted_mode' else repr(v) + 'f')
            for k, v in key.items()) + '}' for key in serialized_torque_curve()['keys'])
        arrays = ('static Keyframe[] SourceKeys(){return new[]{' + keys + '};}' +
                  'static float[] Prefix(){return new float[]{' + ','.join(str(v) + 'f' for v in PREFIX) + '};}' +
                  'static float[] Holdouts(){return new float[]{' + ','.join(str(v) + 'f' for v in FROZEN_INPUTS) + '};}' +
                  'static string[] SuffixBits(){return new[]{' + ','.join('"' + bit + '"' for bit in INPUT_BITS) + '};}')
        code = STUBS + 'public static class Capture {' + CAPTURE + methods + arrays + CHECKS + '}'
        command = "Add-Type -TypeDefinition @'\n" + code + "\n'@\n[Capture]::Run()"
        result = subprocess.run(['pwsh', '-NoProfile', '-Command', command], capture_output=True,
                                text=True, timeout=60)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn('G11_CURVE_INPUTS_PASS', result.stdout)

    def test_exact_scenario_and_all_invalid_declarations(self):
        validate_curve_scenario(scenario())
        self.assertEqual(tuple(map(bits, FROZEN_INPUTS)), INPUT_BITS)
        self.assertEqual(original_inputs(serialized_torque_curve()['keys']), PREFIX)
        changes = {
            'missing inputs': lambda r: r.pop(INPUT_FIELD),
            'missing provenance': lambda r: r.pop(PROVENANCE_FIELD),
            'null inputs': lambda r: r.update({INPUT_FIELD: None}),
            'empty inputs': lambda r: r.update({INPUT_FIELD: []}),
            'duplicate input': lambda r: r[INPUT_FIELD].__setitem__(1, r[INPUT_FIELD][0]),
            'missing input': lambda r: r[INPUT_FIELD].pop(),
            'extra input': lambda r: r[INPUT_FIELD].append(1.),
            'reordered input': lambda r: r[INPUT_FIELD].reverse(),
            'nan input': lambda r: r[INPUT_FIELD].__setitem__(0, float('nan')),
            'infinite input': lambda r: r[INPUT_FIELD].__setitem__(0, float('inf')),
            'boolean input': lambda r: r[INPUT_FIELD].__setitem__(0, True),
            'string input': lambda r: r[INPUT_FIELD].__setitem__(0, '31.25'),
            'one bit input': lambda r: r[INPUT_FIELD].__setitem__(0, struct.unpack('>f', bytes.fromhex('41fa0001'))[0]),
            'rounded double input': lambda r: r[INPUT_FIELD].__setitem__(0, 31.25 + 1e-10),
            'wrong kind': lambda r: r.update(probe_kind='assembled_lada'),
            'base row control': lambda r: r['controls'][0].update(throttle=1.),
            'extra row field': lambda r: r.update(extra=True),
            'preset state': lambda r: r.update(setup_state={}),
            'preset state flag': lambda r: r.update(has_setup_state=True),
        }
        for key in PROVENANCE:
            changes['changed provenance ' + key] = lambda r, k=key: r[PROVENANCE_FIELD].update({k: 'changed'})
            changes['missing provenance ' + key] = lambda r, k=key: r[PROVENANCE_FIELD].pop(k)
        changes['extra provenance'] = lambda r: r[PROVENANCE_FIELD].update(extra='changed')
        for name, change in changes.items():
            row = scenario(); change(row)
            with self.subTest(name=name), self.assertRaises(ValueError): validate_curve_scenario(row)
        for field in (INPUT_FIELD, PROVENANCE_FIELD):
            for value in (None, [], {}, FROZEN_INPUTS):
                row = dict(scenario(), id='curve-boundaries'); row.pop(INPUT_FIELD); row.pop(PROVENANCE_FIELD)
                row[field] = value
                with self.subTest(field=field, value=value), self.assertRaises(ValueError): validate_curve_scenario(row)
        old = scenario(); old['id'] = 'curve-boundaries'; old.pop(INPUT_FIELD); old.pop(PROVENANCE_FIELD)
        validate_curve_scenario(old)
        self.assertIsNone(validate_curve_run({}, old))

    def test_serialized_and_native_wrap_domains_are_distinct(self):
        serialized = serialized_torque_curve()
        self.assertEqual((serialized['pre_wrap_mode'], serialized['post_wrap_mode']), (2, 2))
        semantic = json.dumps(serialized, sort_keys=True, separators=(',', ':'), ensure_ascii=True).encode('ascii')
        self.assertEqual(hashlib.sha256(semantic).hexdigest(),
                         '5586ee5cf414a3d7db1a64f198af209ecd5bfcf95310bf853c5a7af85737ea89')
        # Native modes are literal observations from qualified focused07, independent of authoring modes.
        row = torque_row()
        with self.subTest(native=(8, 8)):
            self.assertEqual(len(validate_curve_run({'curves': [row]}, scenario())['value_bits']), 77)
        for pre, post in ((2, 2), (2, 8), (8, 2), (0, 0), (1, 1), (4, 4), (16, 16),
                          (8.0, 8), (8, 8.0), (True, 8), (8, False), ('8', 8), (8, None)):
            wrong = dict(row, pre_wrap=pre, post_wrap=post)
            with self.subTest(native=(pre, post)), self.assertRaisesRegex(ValueError, 'Native engine torque wraps differ'):
                validate_curve_run({'curves': [wrong]}, scenario())

    def test_result_requires_exact_identity_keys_wraps_prefix_suffix_and_lengths(self):
        run = {'curves': [torque_row()]}
        result = validate_curve_run(run, scenario())
        self.assertEqual(result['value_bits'], [bits(float(i) + .25) for i in range(77)])
        changes = {
            'missing': lambda r: r.pop('curves'), 'null': lambda r: r.update(curves=None),
            'empty': lambda r: r.update(curves=[]),
            'null row': lambda r: r.update(curves=[None]),
            'duplicate': lambda r: r['curves'].append(torque_row()),
            'owner': lambda r: r['curves'][0].update(owner='carscript'),
            'name': lambda r: r['curves'][0].update(name='changed'),
            'missing key': lambda r: r['curves'][0]['keys'].pop(),
            'extra key': lambda r: r['curves'][0]['keys'].append(deepcopy(r['curves'][0]['keys'][0])),
            'null key': lambda r: r['curves'][0]['keys'].__setitem__(0, None),
            'key value': lambda r: r['curves'][0]['keys'][1].update(value=25.1),
            'key field': lambda r: r['curves'][0]['keys'][0].pop('in_weight'),
            'key mode': lambda r: r['curves'][0]['keys'][0].update(weighted_mode=True),
            'key order': lambda r: r['curves'][0]['keys'].reverse(),
            'pre wrap': lambda r: r['curves'][0].update(pre_wrap=1),
            'post wrap': lambda r: r['curves'][0].update(post_wrap=1),
            'prefix': lambda r: r['curves'][0]['inputs'].__setitem__(2, -0.),
            'suffix': lambda r: r['curves'][0]['inputs'].__setitem__(52, 31.251),
            'input count': lambda r: r['curves'][0]['inputs'].pop(),
            'value count': lambda r: r['curves'][0]['values'].pop(),
            'extra value': lambda r: r['curves'][0]['values'].append(1.),
            'nonfinite value': lambda r: r['curves'][0]['values'].__setitem__(0, Decimal('NaN')),
            'boolean value': lambda r: r['curves'][0]['values'].__setitem__(0, True),
        }
        for name, change in changes.items():
            bad = deepcopy(run); change(bad)
            with self.subTest(name=name), self.assertRaises(ValueError): validate_curve_run(bad, scenario())

    def test_capture_rejects_invalid_declarations_before_setup(self):
        for change in ('missing', 'wrong order', 'wrong provenance'):
            with self.subTest(change=change), tempfile.TemporaryDirectory() as name:
                project = Path(name).resolve(); output = project / '.cache/g11-driving/new'
                row = scenario()
                if change == 'missing': row.pop(INPUT_FIELD)
                if change == 'wrong order': row[INPUT_FIELD].reverse()
                if change == 'wrong provenance': row[PROVENANCE_FIELD]['owner'] = 'carscript'
                definitions = project / 'scenarios.json'
                definitions.write_text(json.dumps({'scenarios': [row]}))
                with patch.object(probe, 'PROJECT', project), patch.object(probe, 'preflight', return_value={}), \
                     patch.object(probe.subprocess, 'run') as launch:
                    with self.assertRaises(ValueError): probe.capture(project, project, output, definitions, 3)
                    launch.assert_not_called()
                self.assertFalse(output.exists())

    def test_validate_run_uses_strict_curve_gate(self):
        from tools.reference.tests.test_driving_probe import DrivingProbeTests
        run = DrivingProbeTests.sample_run('curve')
        run.update(scenario=SCENARIO_ID, curves=[torque_row()])
        # Other suites check the complete timing and control paths with source protocols.
        with patch.object(probe, 'validate_timing', return_value={}), \
             patch.object(probe, 'validate_controls', return_value={}):
            result = probe.validate_run(run, scenario())
            self.assertEqual(result['curve_holdout']['input_bits'], list(map(bits, PREFIX + FROZEN_INPUTS)))
            run['curves'][0]['inputs'][52] = 0.
            with self.assertRaisesRegex(ValueError, 'prefix or suffix bits differ'):
                probe.validate_run(run, scenario())

    def test_native_decimal_tokens_stay_local_and_receipts_serialize(self):
        with tempfile.TemporaryDirectory() as name:
            path = Path(name) / 'runtime.json'
            data = dict(scenario=SCENARIO_ID, curves=[torque_row()], initial={'rpm': .2})
            text = json.dumps(data).replace('0.25', '0.2500000000000000000000000000000000001', 1)
            path.write_text(text)
            read = probe.read_json(path)
            self.assertIs(type(read['initial']['rpm']), float)
            self.assertIs(type(read['curves'][0]['values'][0]), Decimal)
            result = validate_curve_run(read, scenario())
            self.assertEqual(result['native_value_decimals'][0], '0.2500000000000000000000000000000000001')
            self.assertEqual(result['value_bits'][0], bits(.25))
            probe.write_json(path.with_name('receipt.json'), result)
            self.assertEqual(probe.read_json(path.with_name('receipt.json')), result)

    def test_raw_nonengine_decimal_overflow_fails_validate_run(self):
        with tempfile.TemporaryDirectory() as name:
            path = Path(name) / 'runtime.json'
            path.write_text(raw_car_curve_run('1e1000'))
            run = probe.read_json(path)
            overflow = run['curves'][1]['inputs'][0]
            self.assertIs(type(overflow), Decimal)
            self.assertEqual(overflow, Decimal('1e1000'))
            checked = validate_curve_run(run, scenario())
            self.assertEqual(len(checked['value_bits']), 77)
            self.assertEqual(checked['value_bits'], [bits(float(i) + .25) for i in range(77)])
            with self.assertRaisesRegex(ValueError, 'Nonfinite native sample'):
                probe.validate_run(run, scenario())
            # Match the old float gate's range. Finite binary64 car values remain valid.
            path.write_text(raw_car_curve_run('1e100'))
            probe.finite(probe.read_json(path))

    def test_capture_and_explicit_verify_reject_raw_nonengine_decimal_overflow(self):
        raw = raw_car_curve_run('1e1000')
        for operation in ('capture', 'verify'):
            with self.subTest(operation=operation), tempfile.TemporaryDirectory() as name:
                project = Path(name).resolve(); output = project / '.cache/g11-driving/new'
                definitions = project / 'scenarios.json'
                definitions.write_text(json.dumps({'scenarios': [scenario()]}))
                launched = []
                if operation == 'capture':
                    def native(arguments, **kwargs):
                        mode = arguments[arguments.index('-Mode') + 1]; launched.append(mode)
                        if mode == 'Run':
                            directory = Path(arguments[arguments.index('-RunRoot') + 1])
                            (directory / 'runtime.json').write_text(raw)
                        return type('Result', (), {'returncode': 0})()
                    with patch.object(probe, 'PROJECT', project), \
                         patch.object(probe, 'preflight', return_value={'powershell': 'pwsh', 'observer_revision': {}}), \
                         patch.object(probe.subprocess, 'run', side_effect=native):
                        # Real capture, validate_evidence, and validate_run must stop at finite.
                        with self.assertRaisesRegex(ValueError, 'Nonfinite native sample'):
                            probe.capture(project, project, output, definitions, 3)
                    self.assertEqual(launched, ['Setup', 'Run'])
                else:
                    output.mkdir(parents=True)
                    probe.write_json(output / 'request.json', {'repeats': 3, 'scenarios': [scenario()]})
                    directory = output / SCENARIO_ID / 'repeat-1'; directory.mkdir(parents=True)
                    (directory / 'runtime.json').write_text(raw)
                    # No validation function or evidence gate is mocked for explicit verify.
                    with self.assertRaisesRegex(ValueError, 'Nonfinite native sample'):
                        probe.verify(output)
                self.assertEqual((output / SCENARIO_ID / 'repeat-1/runtime.json').read_text(), raw)
                self.assertFalse((output / 'receipt.json').exists())
                self.assertFalse((output / SCENARIO_ID / 'repeat-2').exists())

    def test_capture_and_verify_preserve_declarations_serialize_and_stop_on_bit_changes(self):
        # Run real orchestration and curve validation with synthetic unrelated telemetry.
        for corrupt in ('none', 'value', 'serialized wraps'):
            with self.subTest(corrupt=corrupt), tempfile.TemporaryDirectory() as name:
                project = Path(name).resolve(); output = project / '.cache/g11-driving/new'
                definitions = project / 'scenarios.json'; definitions.write_text(json.dumps({'scenarios': [scenario()]}))
                launched = []
                pre = dict(powershell='pwsh', observer_revision=probe.observer_revision(), app_id=1017180,
                           build_id=11140890, unity_version='2021.3.18f1', native_engine={'sha256': 'a'},
                           dependencies={'TheLongDrive_Data/Managed/UnityEngine.PhysicsModule.dll': 'b'})
                def native(arguments, **kwargs):
                    if arguments[arguments.index('-Mode') + 1] == 'Setup':
                        scratch = output / 'scratch'; scratch.mkdir()
                        probe.write_json(scratch / 'patch-receipt.json', dict(changed_methods=['original'],
                            observer_revision=pre['observer_revision'], native_engine_sha256='a', source_physics_module_sha256='b'))
                    else:
                        directory = Path(arguments[arguments.index('-RunRoot') + 1]); launched.append(directory)
                        row = torque_row()
                        if corrupt == 'value' and len(launched) == 2: row['values'][52] += .5
                        if corrupt == 'serialized wraps': row.update(pre_wrap=2, post_wrap=2)
                        probe.write_json(directory / 'runtime.json', dict(scenario=SCENARIO_ID, curves=[row],
                            initial={'rpm': .2}, setup_state={'held': True}, geometry=[dict(
                                phase='armed-after-canonical', colliders=[], transforms=[])]))
                    return type('Result', (), {'returncode': 0})()
                def validate(output, directory, run, row):
                    return {'curve_holdout': validate_curve_run(run, row)}
                with patch.object(probe, 'PROJECT', project), patch.object(probe, 'preflight', return_value=pre), \
                     patch.object(probe.subprocess, 'run', side_effect=native), patch.object(probe, 'validate_evidence', side_effect=validate):
                    if corrupt != 'none':
                        message, repeats = ('different engine torque curve bits', 2) if corrupt == 'value' else ('Native engine torque wraps differ', 1)
                        with self.assertRaisesRegex(ValueError, message):
                            probe.capture(project, project, output, definitions, 3)
                        self.assertEqual(len(launched), repeats)
                        self.assertFalse((output / 'receipt.json').exists())
                        continue
                    receipt = probe.capture(project, project, output, definitions, 3)
                    self.assertEqual(probe.verify(output), receipt)
                    self.assertEqual(probe.read_json(output / 'receipt.json'), receipt)
                    request = probe.read_json(output / 'request.json')
                    self.assertEqual(request['repeats'], 3)
                    self.assertEqual(request['scenarios'], [scenario()])
                    self.assertEqual(len(launched), 3)
                    for directory in launched:
                        declared = probe.read_json(directory / 'scenario.json')
                        self.assertEqual(declared[INPUT_FIELD], FROZEN_INPUTS)
                        self.assertEqual(declared[PROVENANCE_FIELD], PROVENANCE)
                    changed = launched[-1] / 'runtime.json'; data = json.loads(changed.read_text())
                    data['curves'][0]['values'][52] += .5; probe.write_json(changed, data)
                    with self.assertRaisesRegex(ValueError, 'different engine torque curve bits'): probe.verify(output)
                    data['curves'][0] = torque_row(); data['curves'][0].update(pre_wrap=2, post_wrap=2)
                    probe.write_json(changed, data)
                    with self.assertRaisesRegex(ValueError, 'Native engine torque wraps differ'): probe.verify(output)


STUBS = r"""
using System;
using System.Collections.Generic;
using System.Linq;
public struct Keyframe {public float time,value,inTangent,outTangent,inWeight,outWeight;public int weightedMode;}
public class AnimationCurve {
    public Keyframe[] keys; public int length {get{return keys.Length;}}
    public int preWrapMode=8,postWrapMode=8;
    public List<float> evaluated=new List<float>();
    public float Evaluate(float input){evaluated.Add(input);return evaluated.Count+1234.25f;}
}
public class enginescript {public AnimationCurve torqueCurve;}
public class carscript {public AnimationCurve first,second; public enginescript Engine;}
"""
CAPTURE = r"""
    public class Result {public List<Curve> curves=new List<Curve>();}
    public class Scenario {public float[] engine_torque_holdout_inputs;}
    static Result result; static carscript car;static Scenario scenario;
"""
CHECKS = r"""
    static void Need(bool value,string name){if(!value)throw new Exception(name);}
    static string Bits(float x){return BitConverter.SingleToInt32Bits(x).ToString("x8");}
    public static void Run(){
        foreach(bool holdout in new[]{false,true}){
            result=new Result();scenario=new Scenario{engine_torque_holdout_inputs=holdout?Holdouts():null};
            var first=new AnimationCurve{keys=SourceKeys()};var second=new AnimationCurve{keys=SourceKeys()};
            var engine=new AnimationCurve{keys=SourceKeys()};
            car=new carscript{first=first,second=second,Engine=new enginescript{torqueCurve=engine}};
            CaptureCurves();
            Need(result.curves.Count==3,"capture call count");
            Need(result.curves.Select(x=>x.owner+":"+x.name).SequenceEqual(new[]{"carscript:first","carscript:second","enginescript:torqueCurve"}),"capture call order and identity");
            for(int index=0;index<3;index++){
                var row=result.curves[index];var source=new[]{first,second,engine}[index];
                var expected=holdout && index==2?Prefix().Concat(Holdouts()).ToArray():Prefix();
                Need(row.inputs.Select(Bits).SequenceEqual(expected.Select(Bits)),"ordered input bits");
                Need(source.evaluated.Select(Bits).SequenceEqual(expected.Select(Bits)),"native Evaluate call order");
                Need(row.values.SequenceEqual(Enumerable.Range(1,expected.Length).Select(x=>x+1234.25f)),"distinct native Evaluate return values");
                Need(row.pre_wrap==8 && row.post_wrap==8,"source wraps");
                Need(row.keys.Length==13,"source key count");
                for(int k=0;k<13;k++){
                    var a=row.keys[k];var b=source.keys[k];
                    Need(Bits(a.time)==Bits(b.time) && Bits(a.value)==Bits(b.value) && Bits(a.in_tangent)==Bits(b.inTangent) && Bits(a.out_tangent)==Bits(b.outTangent) && Bits(a.in_weight)==Bits(b.inWeight) && Bits(a.out_weight)==Bits(b.outWeight) && a.weighted_mode==b.weightedMode,"source key fields");
                }
            }
            Need(engine.evaluated.Count==(holdout?77:52),"engine query count");
            Need(engine.evaluated[0]==0 && engine.evaluated[2]==0,"duplicate zero retained");
            if(holdout)Need(engine.evaluated.Skip(52).Select(Bits).SequenceEqual(SuffixBits()),"frozen suffix bits");
        }
        result=new Result();CaptureCurve("enginescript","torqueCurve",null,Holdouts());
        CaptureCurve("enginescript","torqueCurve",new AnimationCurve{keys=new Keyframe[0]},Holdouts());
        Need(result.curves.Count==0,"native null/empty remains absent for strict validator");
        Console.WriteLine("G11_CURVE_INPUTS_PASS");
    }
"""

if __name__ == '__main__': unittest.main()
