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
from tools.reference import driving_curve as curve
from tools.reference.driving_curve import (
    SCENARIO_ID, INPUT_FIELD, PROVENANCE_FIELD, INPUT_BITS, PROVENANCE,
    COMPLETE_SCENARIO_ID, COMPLETE_INPUT_BITS, COMPLETE_PROVENANCE,
    bits, original_inputs, serialized_torque_curve, validate_curve_scenario, validate_curve_run)

ROOT = Path(__file__).resolve().parents[1]
FROZEN_INPUTS = [31.25, 656.25, 687.5, 1437.5, 1468.75, 1984.375, 2390.625,
                 2484.375, 2890.625, 2968.75, 3437.5, 4265.625, 4484.375, 4406.25,
                 4718.75, 4890.625, 4906.25, 5187.5, 5343.75, 5359.375, 5856.25,
                 5875., 5931.25, 6553.125, 6240.625]
FROZEN_COMPLETE_BITS = (
    '4104d000', '41a02800', '448b86c0', '44b61d18', '44b78c48', '448bcd10', '448d01a8', '44ad4378',
    '448fc4b0', '44885a40', '447fbcc0', '448462a0', '44b1e6f8', '44b60d78', '44b4aa00', '44a37b90',
    '44b31b90', '4487a690', '448a0fc0', '44ad0110', '448c9c18', '448b9660', '4496b608', '449e9990',
    '44d11370', '44f79ab8', '44f58f48', '44d8ef28', '44ceb5f8', '44f6eed8', '44c87850', '44e32858',
    '44ef1320', '44f6c7c8', '44ed1368', '44ccba28', '44d97010', '44d75cd0', '44dab060', '44d7ca30',
    '44e80670', '44dad770', '44e7a0e0', '44f1a748', '44f0f780', '44df01d8', '44e64920', '44e85c60',
    '44e9db30', '44f45ab0', '44cc7f90', '44d32a98', '44d5ca78', '44da52a0', '44ea8710', '44f5ae88',
    '44f69118', '44cdf690', '44dfec38', '44e3d438', '44e69740', '44e69b28', '44e8be08', '44e8d578',
    '4505d2db', '45061ea0', '451548cc', '450256b1', '4504f3a9', '4506a749', '45174e60', '4500d38e',
    '4503d7f0', '45054148', '450673ff', '4506996d', '451189d0', '4518a814', '451be04c', '44fdd844',
    '4501a0ac', '45031bfd', '450329a1', '450356a0', '4503ad45', '4503cd19', '4503f6db', '450442e1',
    '4506a5cb', '4506a99d', '450b30d0', '450bf22c', '450ee414', '45109d7c', '451a80bc', '45009fbc',
    '4500fe1d', '45013c7f', '4501e3d2', '4502ae3f', '4502c466', '45039780', '4503bce9', '45042237',
    '45044735', '4504616f', '4504a225', '4504b363', '4504c060', '4504c33f', '45055474', '45055f83',
    '45062a85', '4506b029', '45071bcb', '45075328', '4507d11f', '451289ac', '4512e384', '451402a0',
    '45155c54', '451575b8', '4516eeac', '4518c754', '451a3490', '451a6758', '451a6d34', '452d4b48',
    '453910f4', '4531e310', '4524f09c', '45274644', '4539f384', '45382c70', '45386708', '45275fa8',
    '45352ed0', '453aef78', '4525d520', '452fbc48', '453d703b', '4550b5b0', '4551a3f8', '4543a6dc',
    '4549d010', '454324eb', '4552f7d0', '455474ac', '455a681c', '45402b48', '4540dafc', '45427bec',
    '45468a2c', '45485334', '454faff8', '453f55ff', '453f6f4b', '4541189a', '4548012c', '454e2188',
    '454edb14', '454fb9bc', '455126f8', '45596458', '455a6df8', '45400e77', '45411dbf', '4541521d',
    '454365e2', '45572df0', '45575ec4', '453e22c2', '453fa712', '453fc2d9', '453ff9a3', '45408956',
    '454180ac', '454194a4', '45425fa9', '45426dda', '4542e06c', '45432508', '45438861', '45438a98',
    '4543bfcb', '4544104d', '45446037', '454522cc', '45453c30', '454a3f64', '454f7f24', '45508ea0',
    '455101dc', '4552c120', '45563db4', '4556d60c', '455706e0', '454161a3', '4541f1a6', '4542c595',
    '45443c54', '454617fa', '4549511c', '45497e08', '4549dfb0', '454d391c', '455063a8', '45521d10',
    '45525d84', '4552bf2c', '4552d0c0', '45530770', '45530d4c', '4553b544', '45543244', '4554ab5c',
    '45557a64', '45562e14', '4556419c', '45564390', '45565cf4', '4557dbc4', '45588f74', '4558ba6c',
    '4558dba0', '455a0e44', '455a587c', '455a6a10', '455aa0c0', '45764eb0', '4560d88c', '456b9880',
    '45786bb4', '4577bde0', '455cd534', '45623434', '4568e130', '4571340c', '4574a6dc', '4576c7c8',
    '457990ac', '45780818', '45622e58', '456f2c84', '456b774c', '4577a670', '4578cd5c', '4570e9d4',
    '45776dcc', '457233e8', '45684108', '45662dc8', '4576d380', '456844f0', '456848d8', '456ff7a4',
    '45652810', '456b2b20', '456acf54', '456c6f58', '456775e8', '4569df18', '456a13d4', '456cf234',
    '456e72f8', '456eaf84', '456f9408', '4570b900', '45715f04', '457302f0', '4580cf96', '458c1f18',
    '458a512e', '458b9d36', '4583205c', '458024b0', '458148ae', '458bb988', '457d3450', '458bf902',
    '458bffd8', '458b5ace', '457f7670', '45931840', '459b1914', '459ab866', '45941df8', '458ed956',
    '459a3490', '4599936e', '459288b2', '4593711e', '4599f322', '45998aa4', '459012d0', '45a7137c',
    '45a1953c', '45a93562', '45a67642', '45a16850', '45a8cce4', '45a99516', '45a5c968', '45a1f108',
    '45b56328', '45bddea4', '45b10c38', '45bd34b8', '45bd78b0', '45b0a068', '45b0e208', '45b7efe4',
    '45b7c488', '45b1ee64',
)
FROZEN_CONFIRMATION_BITS = (
    '04072d2d', '080e5a5a', '0c158787', '101cb4b4', '1423e1e1', '182b0f0f', '1c323c3c', '20396969',
    '24409696', '2847c3c3', '2c4ef0f0', '30561e1e', '345d4b4b', '38647878', '3c6ba5a5', '4072d2d2',
    '447dda5a', '4481b4b4', '44858f0f', '44896969', '448d43c3', '44911e1e', '4494f878', '4498d2d2',
    '449cad2d', '44a08787', '44a461e1', '44a83c3c', '44ac1696', '44aff0f0', '44b3cb4b', '44b7a5a5',
    '44bf2d2d', '44c2da5a', '44c68787', '44ca34b4', '44cde1e1', '44d18f0f', '44d53c3c', '44d8e969',
    '44dc9696', '44e043c3', '44e3f0f0', '44e79e1e', '44eb4b4b', '44eef878', '44f2a5a5', '44f652d2',
    '44fc03c3', '44fe0787', '45000b4b', '45020f0f', '450412d2', '45061696', '45081a5a', '450a1e1e',
    '450c21e1', '450e25a5', '45102969', '45122d2d', '451430f0', '451634b4', '45183878', '451a3c3c',
    '451e1696', '451fed2d', '4521c3c3', '45239a5a', '452570f0', '45274787', '45291e1e', '452af4b4',
    '452ccb4b', '452ea1e1', '45307878', '45324f0f', '453425a5', '4535fc3c', '4537d2d2', '4539a969',
    '453d5696', '453f2d2d', '454103c3', '4542da5a', '4544b0f0', '45468787', '45485e1e', '454a34b4',
    '454c0b4b', '454de1e1', '454fb878', '45518f0f', '455365a5', '45553c3c', '455712d2', '4558e969',
    '455c9696', '455e6d2d', '456043c3', '45621a5a', '4563f0f0', '4565c787', '45679e1e', '456974b4',
    '456b4b4b', '456d21e1', '456ef878', '4570cf0f', '4572a5a5', '45747c3c', '457652d2', '45782969',
    '457b1878', '457c30f0', '457d4969', '457e61e1', '457f7a5a', '458092d2', '4581ab4b', '4582c3c3',
    '4583dc3c', '4584f4b4', '45860d2d', '458725a5', '45883e1e', '45895696', '458a6f0f', '458b8787',
    '458d8b4b', '458e7696', '458f61e1', '45904d2d', '45913878', '459223c3', '45930f0f', '4593fa5a',
    '4594e5a5', '4595d0f0', '4596bc3c', '4597a787', '459892d2', '45997e1e', '459a6969', '459b54b4',
    '459d2b4b', '459e1696', '459f01e1', '459fed2d', '45a0d878', '45a1c3c3', '45a2af0f', '45a39a5a',
    '45a485a5', '45a570f0', '45a65c3c', '45a74787', '45a832d2', '45a91e1e', '45aa0969', '45aaf4b4',
    '45acfa5a', '45ae14b4', '45af2f0f', '45b04969', '45b163c3', '45b27e1e', '45b39878', '45b4b2d2',
    '45b5cd2d', '45b6e787', '45b801e1', '45b91c3c', '45ba3696', '45bb50f0', '45bc6b4b', '45bd85a5',
    '45bf8b4b', '45c07696', '45c161e1', '45c24d2d', '45c33878', '45c423c3', '45c50f0f', '45c5fa5a',
    '45c6e5a5', '45c7d0f0', '45c8bc3c', '45c9a787', '45ca92d2', '45cb7e1e', '45cc6969', '45cd54b4',
)
CONFIRMATION_INPUTS = [struct.unpack('>f', bytes.fromhex(word))[0] for word in FROZEN_CONFIRMATION_BITS]
CONFIRMATION_PROVENANCE = {
    'owner': 'enginescript', 'name': 'torqueCurve',
    'experiment_freeze_sha256': '18c05918d03b43f570eb69b9ee5785a8ab6b6779cb3358cd7a5a3646b77d0a4a',
    'inputs_binary32_le_sha256': 'c7d1dff06cc0404a73a8b91cb691986af6b6ada272c8eeb0eb3ccfe8f03496d9'}


def selected_inputs(complete):
    if complete == 'confirmation': return CONFIRMATION_INPUTS
    return COMPLETE_INPUTS if complete else FROZEN_INPUTS


def selected_id(complete):
    if complete == 'confirmation': return 'curve-predictor-confirmation'
    return COMPLETE_SCENARIO_ID if complete else SCENARIO_ID


def selected_provenance(complete):
    if complete == 'confirmation': return CONFIRMATION_PROVENANCE
    return COMPLETE_PROVENANCE if complete else PROVENANCE


COMPLETE_INPUTS = [struct.unpack('>f', bytes.fromhex(word))[0] for word in FROZEN_COMPLETE_BITS]
PREFIX = [0., -6600., 0., 1000., 1500., 2000., 2500., 3000., 3500., 4000.,
          4500., 5000., 5500., 6100., 6600., 13200., 250., 500., 750.,
          1125., 1250., 1375., 1625., 1750., 1875., 2125., 2250., 2375.,
          2625., 2750., 2875., 3125., 3250., 3375., 3625., 3750., 3875.,
          4125., 4250., 4375., 4625., 4750., 4875., 5125., 5250., 5375.,
          5650., 5800., 5950., 6225., 6350., 6475.]


def scenario(complete=False):
    document = json.loads((ROOT.parents[1] / 'tests/fixtures/driving/scenarios.json').read_text())
    row = deepcopy(next(row for row in document['scenarios'] if row['id'] == 'curve-boundaries'))
    return dict(row, id=selected_id(complete), **{
        INPUT_FIELD: (selected_inputs(complete)).copy(),
        PROVENANCE_FIELD: dict(selected_provenance(complete))})


def torque_row(complete=False):
    suffix = selected_inputs(complete)
    return dict(owner='enginescript', name='torqueCurve', keys=serialized_torque_curve()['keys'],
                pre_wrap=8, post_wrap=8, inputs=PREFIX + suffix,
                values=[float(i) + .25 for i in range(52 + len(suffix))])


def raw_car_curve_run(token, complete=False):
    run = dict(scenario=selected_id(complete), probe_kind='curve', runtime='original-unity-player',
               curves=[torque_row(complete), dict(owner='carscript', name='other', inputs=['CAR_INPUT_TOKEN'])])
    return json.dumps(run).replace('"CAR_INPUT_TOKEN"', token)


class DrivingCurveInputTests(unittest.TestCase):

    def test_confirmation_inputs_and_complete_stream_match_frozen_hashes(self):
        self.assertEqual(curve.CONFIRMATION_SCENARIO_ID, 'curve-predictor-confirmation')
        self.assertEqual(curve.CONFIRMATION_INPUT_BITS, FROZEN_CONFIRMATION_BITS)
        self.assertEqual(curve.CONFIRMATION_PROVENANCE, CONFIRMATION_PROVENANCE)
        suffix = b''.join(bytes.fromhex(word)[::-1] for word in FROZEN_CONFIRMATION_BITS)
        full = b''.join(struct.pack('<f', value) for value in PREFIX) + suffix
        self.assertEqual((len(FROZEN_CONFIRMATION_BITS), len(suffix), len(full)), (192, 768, 976))
        self.assertEqual(hashlib.sha256(suffix).hexdigest(),
                         'c7d1dff06cc0404a73a8b91cb691986af6b6ada272c8eeb0eb3ccfe8f03496d9')
        self.assertEqual(hashlib.sha256(full).hexdigest(),
                         'a9cd597666f039b86aad17004b59a2a1f2a5e7873951680b709db6be4abbbabc')
        self.assertEqual(PREFIX[0], PREFIX[2])
        row = scenario('confirmation')
        validate_curve_scenario(row)
        base = {k: v for k, v in row.items() if k not in (INPUT_FIELD, PROVENANCE_FIELD)}
        base['id'] = 'curve-boundaries'
        self.assertEqual(curve.definition_hash(base),
                         '611dd801fb756b26726bd81e6d528080958d0813d49b47e20a744f6408fe4411')
        declared = deepcopy(row)
        declared[INPUT_FIELD][-1] += 1e-9
        self.assertEqual(bits(declared[INPUT_FIELD][-1]), FROZEN_CONFIRMATION_BITS[-1])
        with self.assertRaisesRegex(ValueError, 'ordered exact binary32'):
            validate_curve_scenario(declared)
        native = torque_row('confirmation')
        native['inputs'][-1] = Decimal(str(CONFIRMATION_INPUTS[-1])) + Decimal('0.000000000001')
        self.assertEqual(len(validate_curve_run({'curves': [native]}, row)['value_bits']), 244)

    def test_complete_inputs_match_independently_declared_words_and_provenance(self):
        self.assertEqual(COMPLETE_SCENARIO_ID, 'curve-complete-identification')
        self.assertEqual(COMPLETE_INPUT_BITS, FROZEN_COMPLETE_BITS)
        self.assertEqual(len(COMPLETE_INPUT_BITS), 314)
        packed = b''.join(bytes.fromhex(word)[::-1] for word in FROZEN_COMPLETE_BITS)
        self.assertEqual(hashlib.sha256(packed).hexdigest(),
                         'd07975062c25a031100585f2f308cc7c0fc5cfce328c8a788849c1ca171c5a8b')
        self.assertEqual(COMPLETE_PROVENANCE, {
            'owner': 'enginescript', 'name': 'torqueCurve',
            'design_results_sha256': '4d75b86cbe88acd58f20dda76eeb2c4be809478b1aa29e1e210e8c2002c63e61',
            'inputs_binary32_le_sha256': 'd07975062c25a031100585f2f308cc7c0fc5cfce328c8a788849c1ca171c5a8b'})
        for complete in (False, True, 'confirmation'):
            row = scenario(complete)
            row[PROVENANCE_FIELD] = dict(PROVENANCE if complete else COMPLETE_PROVENANCE)
            with self.subTest(complete=complete), self.assertRaisesRegex(ValueError, 'provenance differs'):
                validate_curve_scenario(row)
        for identifier in ('curve-boundaries', 'unrecognized-curve'):
            for field in (INPUT_FIELD, PROVENANCE_FIELD):
                for value in (None, [], {}):
                    row = dict(scenario(), id=identifier)
                    del row[INPUT_FIELD]; del row[PROVENANCE_FIELD]
                    row[field] = value
                    with self.subTest(identifier=identifier, field=field, value=value), self.assertRaises(ValueError):
                        validate_curve_scenario(row)

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
                  'static string[] SuffixBits(){return new[]{' + ','.join('"' + bit + '"' for bit in INPUT_BITS) + '};}' +
                  'static float[] Complete(){return new float[]{' + ','.join(str(v) + 'f' for v in COMPLETE_INPUTS) + '};}' +
                  'static float[] Confirmation(){return new float[]{' + ','.join(str(v) + 'f' for v in CONFIRMATION_INPUTS) + '};}' +
                  'static string[] ConfirmationBits(){return new[]{' + ','.join(json.dumps(bit) for bit in FROZEN_CONFIRMATION_BITS) + '};}' +
                  'static string[] CompleteBits(){return new[]{' + ','.join('"' + bit + '"' for bit in FROZEN_COMPLETE_BITS) + '};}')
        code = STUBS + 'public static class Capture {' + CAPTURE + methods + arrays + CHECKS + '}'
        command = "Add-Type -TypeDefinition @'\n" + code + "\n'@\n[Capture]::Run()"
        result = subprocess.run(['pwsh', '-NoProfile', '-Command', command], capture_output=True,
                                text=True, timeout=60)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn('G11_CURVE_INPUTS_PASS', result.stdout)

    def test_exact_scenario_and_all_invalid_declarations(self):
        for complete in (False, True, 'confirmation'):
            validate_curve_scenario(scenario(complete))
            expected_bits = FROZEN_CONFIRMATION_BITS if complete == 'confirmation' else FROZEN_COMPLETE_BITS if complete else INPUT_BITS
            self.assertEqual(tuple(map(bits, selected_inputs(complete))), expected_bits)
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
                'one bit input': lambda r: r[INPUT_FIELD].__setitem__(0, struct.unpack('>f', struct.pack('>I', int(
                    (tuple(map(bits, selected_inputs(complete))))[0], 16) ^ 1))[0]),
                'rounded double input': lambda r: r[INPUT_FIELD].__setitem__(0, (selected_inputs(complete))[0] + 1e-10),
                'wrong kind': lambda r: r.update(probe_kind='assembled_lada'),
                'base row control': lambda r: r['controls'][0].update(throttle=1.),
                'extra row field': lambda r: r.update(extra=True),
                'preset state': lambda r: r.update(setup_state={}),
                'preset state flag': lambda r: r.update(has_setup_state=True),
            }
            for key in (selected_provenance(complete)):
                changes['changed provenance ' + key] = lambda r, k=key: r[PROVENANCE_FIELD].update({k: 'changed'})
                changes['missing provenance ' + key] = lambda r, k=key: r[PROVENANCE_FIELD].pop(k)
            changes['extra provenance'] = lambda r: r[PROVENANCE_FIELD].update(extra='changed')
            for name, change in changes.items():
                row = scenario(complete); change(row)
                with self.subTest(name=name), self.assertRaises(ValueError): validate_curve_scenario(row)
            for field in (INPUT_FIELD, PROVENANCE_FIELD):
                for value in (None, [], {}, (selected_inputs(complete))):
                    row = dict(scenario(complete), id='curve-boundaries'); row.pop(INPUT_FIELD); row.pop(PROVENANCE_FIELD)
                    row[field] = value
                    with self.subTest(field=field, value=value), self.assertRaises(ValueError): validate_curve_scenario(row)
            old = scenario(complete); old['id'] = 'curve-boundaries'; old.pop(INPUT_FIELD); old.pop(PROVENANCE_FIELD)
            validate_curve_scenario(old)
            with self.assertRaisesRegex(ValueError, 'Native engine torque curve is missing'): validate_curve_run({}, old)

    def test_serialized_and_native_wrap_domains_are_distinct(self):
        for complete in (False, True, 'confirmation'):
            serialized = serialized_torque_curve()
            self.assertEqual((serialized['pre_wrap_mode'], serialized['post_wrap_mode']), (2, 2))
            semantic = json.dumps(serialized, sort_keys=True, separators=(',', ':'), ensure_ascii=True).encode('ascii')
            self.assertEqual(hashlib.sha256(semantic).hexdigest(),
                             '5586ee5cf414a3d7db1a64f198af209ecd5bfcf95310bf853c5a7af85737ea89')
            # Native modes are literal observations from qualified focused07, independent of authoring modes.
            row = torque_row(complete)
            with self.subTest(native=(8, 8)):
                self.assertEqual(len(validate_curve_run({'curves': [row]}, scenario(complete))['value_bits']), (52 + len(selected_inputs(complete))))
            for pre, post in ((2, 2), (2, 8), (8, 2), (0, 0), (1, 1), (4, 4), (16, 16),
                              (8.0, 8), (8, 8.0), (True, 8), (8, False), ('8', 8), (8, None)):
                wrong = dict(row, pre_wrap=pre, post_wrap=post)
                with self.subTest(native=(pre, post)), self.assertRaisesRegex(ValueError, 'Native engine torque wraps differ'):
                    validate_curve_run({'curves': [wrong]}, scenario(complete))

    def test_result_requires_exact_identity_keys_wraps_prefix_suffix_and_lengths(self):
        for complete in (False, True, 'confirmation'):
            run = {'curves': [torque_row(complete)]}
            result = validate_curve_run(run, scenario(complete))
            self.assertEqual(result['value_bits'], [bits(float(i) + .25) for i in range((52 + len(selected_inputs(complete))))])
            changes = {
                'missing': lambda r: r.pop('curves'), 'null': lambda r: r.update(curves=None),
                'empty': lambda r: r.update(curves=[]),
                'null row': lambda r: r.update(curves=[None]),
                'duplicate': lambda r: r['curves'].append(torque_row(complete)),
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
                with self.subTest(name=name), self.assertRaises(ValueError): validate_curve_run(bad, scenario(complete))

    def test_capture_rejects_invalid_declarations_before_setup(self):
        for complete in (False, True, 'confirmation'):
            for change in ('missing', 'wrong order', 'wrong provenance'):
                with self.subTest(change=change), tempfile.TemporaryDirectory() as name:
                    project = Path(name).resolve(); output = project / '.cache/g11-driving/new'
                    row = scenario(complete)
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
        for complete in (False, True, 'confirmation'):
            from tools.reference.tests.test_driving_probe import DrivingProbeTests
            run = DrivingProbeTests.sample_run('curve')
            run.update(scenario=(selected_id(complete)), curves=[torque_row(complete)])
            # Other suites check the complete timing and control paths with source protocols.
            with patch.object(probe, 'validate_timing', return_value={}), \
                 patch.object(probe, 'validate_controls', return_value={}):
                result = probe.validate_run(run, scenario(complete))
                self.assertEqual(result['engine_curve']['input_state']['input_bits'], list(map(bits, PREFIX + (selected_inputs(complete)))))
                run['curves'][0]['inputs'][52] = 0.
                with self.assertRaisesRegex(ValueError, 'prefix or suffix bits differ'):
                    probe.validate_run(run, scenario(complete))

    def test_native_decimal_tokens_stay_local_and_receipts_serialize(self):
        for complete in (False, True, 'confirmation'):
            with tempfile.TemporaryDirectory() as name:
                path = Path(name) / 'runtime.json'
                data = dict(scenario=(selected_id(complete)), probe_kind='curve', curves=[torque_row(complete)], initial={'rpm': .2})
                data['curves'][0]['values'][0] = 'LOSSLESS_OUTPUT_TOKEN'
                text = json.dumps(data).replace('"LOSSLESS_OUTPUT_TOKEN"',
                                                '0.2500000000000000000000000000000000001')
                path.write_text(text)
                read = probe.read_json(path)
                self.assertIs(type(read['initial']['rpm']), float)
                self.assertIs(type(read['curves'][0]['values'][0]), Decimal)
                result = validate_curve_run(read, scenario(complete))
                self.assertEqual(result['native_value_decimals'][0], '0.2500000000000000000000000000000000001')
                self.assertEqual(result['value_bits'][0], bits(.25))
                probe.write_json(path.with_name('receipt.json'), result)
                self.assertEqual(probe.read_json(path.with_name('receipt.json')), result)

    def test_raw_nonengine_decimal_overflow_fails_validate_run(self):
        for complete in (False, True, 'confirmation'):
            with tempfile.TemporaryDirectory() as name:
                path = Path(name) / 'runtime.json'
                path.write_text(raw_car_curve_run('1e1000', complete))
                run = probe.read_json(path)
                overflow = run['curves'][1]['inputs'][0]
                self.assertIs(type(overflow), Decimal)
                self.assertEqual(overflow, Decimal('1e1000'))
                checked = validate_curve_run(run, scenario(complete))
                self.assertEqual(len(checked['value_bits']), (52 + len(selected_inputs(complete))))
                self.assertEqual(checked['value_bits'], [bits(float(i) + .25) for i in range((52 + len(selected_inputs(complete))))])
                with self.assertRaisesRegex(ValueError, 'Nonfinite native sample'):
                    probe.validate_run(run, scenario(complete))
                # Match the old float gate's range. Finite binary64 car values remain valid.
                path.write_text(raw_car_curve_run('1e100', complete))
                probe.finite(probe.read_json(path))

    def test_capture_and_explicit_verify_reject_raw_nonengine_decimal_overflow(self):
        for complete in (False, True, 'confirmation'):
            raw = raw_car_curve_run('1e1000', complete)
            for operation in ('capture', 'verify'):
                with self.subTest(operation=operation), tempfile.TemporaryDirectory() as name:
                    project = Path(name).resolve(); output = project / '.cache/g11-driving/new'
                    definitions = project / 'scenarios.json'
                    definitions.write_text(json.dumps({'scenarios': [scenario(complete)]}))
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
                        probe.write_json(output / 'request.json', {'repeats': 3, 'scenarios': [scenario(complete)]})
                        directory = output / (selected_id(complete)) / 'repeat-1'; directory.mkdir(parents=True)
                        (directory / 'runtime.json').write_text(raw)
                        # No validation function or evidence gate is mocked for explicit verify.
                        with self.assertRaisesRegex(ValueError, 'Nonfinite native sample'):
                            probe.verify(output)
                    self.assertEqual((output / (selected_id(complete)) / 'repeat-1/runtime.json').read_text(), raw)
                    self.assertFalse((output / 'receipt.json').exists())
                    self.assertFalse((output / (selected_id(complete)) / 'repeat-2').exists())

    def test_capture_and_verify_preserve_declarations_serialize_and_stop_on_bit_changes(self):
        for complete in (False, True, 'confirmation'):
            # Run real orchestration and curve validation with synthetic unrelated telemetry.
            for corrupt in ('none', 'value', 'serialized wraps'):
                with self.subTest(corrupt=corrupt), tempfile.TemporaryDirectory() as name:
                    project = Path(name).resolve(); output = project / '.cache/g11-driving/new'
                    definitions = project / 'scenarios.json'; definitions.write_text(json.dumps({'scenarios': [scenario(complete)]}))
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
                            row = torque_row(complete)
                            if corrupt == 'value' and len(launched) == 2: row['values'][52] += .5
                            if corrupt == 'serialized wraps': row.update(pre_wrap=2, post_wrap=2)
                            probe.write_json(directory / 'runtime.json', dict(scenario=(selected_id(complete)), probe_kind='curve', curves=[row],
                                initial={'rpm': .2}, setup_state={'held': True}, geometry=[dict(
                                    phase='armed-after-canonical', colliders=[], transforms=[])]))
                        return type('Result', (), {'returncode': 0})()
                    def validate(output, directory, run, row):
                        return {'engine_curve': validate_curve_run(run, row)}
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
                        self.assertEqual(request['scenarios'], [scenario(complete)])
                        self.assertEqual(len(launched), 3)
                        for directory in launched:
                            declared = probe.read_json(directory / 'scenario.json')
                            self.assertEqual(declared[INPUT_FIELD], (selected_inputs(complete)))
                            self.assertEqual(declared[PROVENANCE_FIELD], (selected_provenance(complete)))
                        changed = launched[-1] / 'runtime.json'; data = json.loads(changed.read_text())
                        data['curves'][0]['values'][52] += .5; probe.write_json(changed, data)
                        with self.assertRaisesRegex(ValueError, 'different engine torque curve bits'): probe.verify(output)
                        data['curves'][0] = torque_row(complete); data['curves'][0].update(pre_wrap=2, post_wrap=2)
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
        foreach(int count in new[]{0,25,314,192}){
            var suffix=count==192?Confirmation():count==314?Complete():count==25?Holdouts():null;
            result=new Result();scenario=new Scenario{engine_torque_holdout_inputs=suffix};
            var first=new AnimationCurve{keys=SourceKeys()};var second=new AnimationCurve{keys=SourceKeys()};
            var engine=new AnimationCurve{keys=SourceKeys()};
            car=new carscript{first=first,second=second,Engine=new enginescript{torqueCurve=engine}};
            CaptureCurves();
            Need(result.curves.Count==3,"capture call count");
            Need(result.curves.Select(x=>x.owner+":"+x.name).SequenceEqual(new[]{"carscript:first","carscript:second","enginescript:torqueCurve"}),"capture call order and identity");
            for(int index=0;index<3;index++){
                var row=result.curves[index];var source=new[]{first,second,engine}[index];
                var expected=count>0 && index==2?Prefix().Concat(suffix).ToArray():Prefix();
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
            Need(engine.evaluated.Count==52+count,"engine query count, including confirmation244");
            Need(engine.evaluated[0]==0 && engine.evaluated[2]==0,"duplicate zero retained");
            if(count>0)Need(engine.evaluated.Skip(52).Select(Bits).SequenceEqual(count==192?ConfirmationBits():count==314?CompleteBits():SuffixBits()),"frozen suffix bits");
        }
        result=new Result();CaptureCurve("enginescript","torqueCurve",null,Holdouts());
        CaptureCurve("enginescript","torqueCurve",new AnimationCurve{keys=new Keyframe[0]},Holdouts());
        Need(result.curves.Count==0,"native null/empty remains absent for strict validator");
        Console.WriteLine("G11_CURVE_INPUTS_PASS");
    }
"""

if __name__ == '__main__': unittest.main()
