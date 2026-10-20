"""Frozen engine torque queries and exact source observation checks."""
from decimal import Decimal
import hashlib
import json
import math
from pathlib import Path
import struct

SCENARIO_ID = 'curve-coefficient-holdouts'
COMPLETE_SCENARIO_ID = 'curve-complete-identification'
INPUT_FIELD = 'engine_torque_holdout_inputs'
PROVENANCE_FIELD = 'engine_torque_holdout_provenance'
INPUT_BITS = (
    '41fa0000', '44241000', '442be000', '44b3b000', '44b79800',
    '44f80c00', '45156a00', '451b4600', '4534aa00', '45398c00',
    '4556d800', '45854d00', '458c2300', '4589b200', '45937600',
    '4598d500', '45995200', '45a21c00', '45a6fe00', '45a77b00',
    '45b70200', '45b79800', '45b95a00', '45ccc900', '45c30500')
PROVENANCE = {
    'owner': 'enginescript', 'name': 'torqueCurve',
    'holdouts_sha256': '976095da27c1537f328b518640b74d4b419eae00f8e3aaf21fcc2da7ac9ffae2',
    'inputs_binary32_le_sha256': '4e48751084e1efd7557d0659cc301cc05aa6173f2c3e229d3c8c42eafd9d03e2'}
COMPLETE_INPUT_BITS = (
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
COMPLETE_PROVENANCE = {
    'owner': 'enginescript', 'name': 'torqueCurve',
    'design_results_sha256': '4d75b86cbe88acd58f20dda76eeb2c4be809478b1aa29e1e210e8c2002c63e61',
    'inputs_binary32_le_sha256': 'd07975062c25a031100585f2f308cc7c0fc5cfce328c8a788849c1ca171c5a8b'}
BASE_ROW_SHA256 = '611dd801fb756b26726bd81e6d528080958d0813d49b47e20a744f6408fe4411'
SERIALIZED_TORQUE_CURVE_SHA256 = '5586ee5cf414a3d7db1a64f198af209ecd5bfcf95310bf853c5a7af85737ea89'
# Shipped WrapMode.ClampForever, confirmed by qualified focused07 native metadata.
NATIVE_TORQUE_WRAPS = (8, 8)


def definition_hash(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(',', ':'),
                                    ensure_ascii=True, allow_nan=False).encode('ascii')).hexdigest()


def bits(value):
    if type(value) not in (int, float, Decimal):
        raise ValueError('Curve value must be numeric without Booleans')
    try:
        number = float(value)
        if not math.isfinite(number):
            raise ValueError('Curve value must be finite')
        packed = struct.pack('>f', number)
        if not math.isfinite(struct.unpack('>f', packed)[0]):
            raise ValueError('Curve value exceeds binary32')
        return packed.hex()
    except (OverflowError, struct.error) as error:
        raise ValueError('Curve value exceeds binary32') from error


def validate_curve_scenario(scenario):
    present = {INPUT_FIELD, PROVENANCE_FIELD} & scenario.keys()
    if scenario.get('id') == SCENARIO_ID:
        expected_bits, provenance = INPUT_BITS, PROVENANCE
    elif scenario.get('id') == COMPLETE_SCENARIO_ID:
        expected_bits, provenance = COMPLETE_INPUT_BITS, COMPLETE_PROVENANCE
    else:
        if present:
            raise ValueError('Engine torque holdouts are reserved for their exact scenario')
        return
    count = len(expected_bits)
    if scenario.get('probe_kind') != 'curve' or present != {INPUT_FIELD, PROVENANCE_FIELD}:
        raise ValueError('Curve holdout scenario requires both declarations and curve kind')
    inputs = scenario[INPUT_FIELD]
    if not isinstance(inputs, list) or len(inputs) != count:
        raise ValueError(f'Curve holdouts require exactly {count} inputs')
    actual = tuple(bits(value) for value in inputs)
    if (actual != expected_bits or len(set(actual)) != count or
        any(type(value) not in (int, float) or struct.unpack('>f', bytes.fromhex(bit))[0] != value
            for value, bit in zip(inputs, actual))):
        raise ValueError('Curve holdouts differ from the ordered exact binary32 inputs')
    packed = b''.join(bytes.fromhex(bit)[::-1] for bit in actual)
    if hashlib.sha256(packed).hexdigest() != provenance['inputs_binary32_le_sha256']:
        raise ValueError('Curve holdout input hash differs')
    if scenario[PROVENANCE_FIELD] != provenance:
        raise ValueError('Curve holdout provenance differs')
    base = {key: value for key, value in scenario.items()
            if key not in (INPUT_FIELD, PROVENANCE_FIELD)}
    base['id'] = 'curve-boundaries'
    if definition_hash(base) != BASE_ROW_SHA256:
        raise ValueError('Curve holdout base row differs from its bound definition')


def serialized_torque_curve():
    path = Path(__file__).resolve().parents[2] / 'tests/fixtures/driving/scenarios.json'
    curve = json.loads(path.read_text(encoding='utf-8'))['source_initial_parameters']['engine']['torque_curve']
    if definition_hash(curve) != SERIALIZED_TORQUE_CURVE_SHA256:
        raise ValueError('Bound serialized torque keys or authoring wraps differ')
    return curve


def original_inputs(keys):
    times = [key['time'] for key in keys]
    interval = times[-1] - times[0]
    return [0., times[0] - interval, *times, times[-1] + interval] + [
        left + (right - left) * quarter * .25
        for left, right in zip(times, times[1:]) for quarter in (1, 2, 3)]


def validate_curve_run(run, scenario):
    validate_curve_scenario(scenario)
    if scenario.get('probe_kind') != 'curve':
        return None
    curves = run.get('curves')
    if not isinstance(curves, list):
        raise ValueError('Native engine torque curve is missing')
    if any(not isinstance(row, dict) for row in curves):
        raise ValueError('Native curve record must be an object')
    rows = [row for row in curves if row.get('owner') == 'enginescript' or row.get('name') == 'torqueCurve']
    if len(rows) != 1 or (rows[0].get('owner'), rows[0].get('name')) != ('enginescript', 'torqueCurve'):
        raise ValueError('Native engine torque curve identity or count differs')
    row = rows[0]
    serialized = serialized_torque_curve()
    keys = row.get('keys')
    if not isinstance(keys, list) or len(keys) != 13:
        raise ValueError('Native engine torque key count differs')
    for actual, expected in zip(keys, serialized['keys']):
        if not isinstance(actual, dict) or actual.keys() != expected.keys() or type(actual['weighted_mode']) is not int:
            raise ValueError('Native engine torque key fields differ')
        if (actual['weighted_mode'] != expected['weighted_mode'] or
            any(bits(actual[name]) != bits(value) for name, value in expected.items() if name != 'weighted_mode')):
            raise ValueError('Native engine torque key bits differ')
    if any(type(row.get(field)) is not int or row[field] != expected
           for field, expected in zip(('pre_wrap', 'post_wrap'), NATIVE_TORQUE_WRAPS)):
        raise ValueError('Native engine torque wraps differ')
    inputs, values = row.get('inputs'), row.get('values')
    expected = tuple(bits(value) for value in original_inputs(serialized['keys']))
    if scenario['id'] == SCENARIO_ID:
        expected += INPUT_BITS
    elif scenario['id'] == COMPLETE_SCENARIO_ID:
        expected += COMPLETE_INPUT_BITS
    count = len(expected)
    if not isinstance(inputs, list) or not isinstance(values, list) or len(inputs) != count or len(values) != count:
        raise ValueError(f'Native engine torque requires {count} inputs and values')
    input_bits = [bits(value) for value in inputs]
    if tuple(input_bits) != expected:
        raise ValueError('Native engine torque prefix or suffix bits differ')
    state = dict(owner=row['owner'], name=row['name'],
        keys=[{name: value if name == 'weighted_mode' else bits(value)
               for name, value in key.items()} for key in keys],
        pre_wrap=row['pre_wrap'], post_wrap=row['post_wrap'], input_bits=input_bits)
    # These are reported curve inputs. Later chassis observations stay raw context.
    return {'input_state': state, 'value_bits': [bits(value) for value in values],
            'native_value_decimals': [str(value) for value in values]}


def require_same_curve(first, current):
    if first is not None and first['value_bits'] != current['value_bits']:
        raise ValueError('Fresh native repeats have different engine torque curve bits')
