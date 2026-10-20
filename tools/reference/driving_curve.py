"""Frozen engine torque queries and exact source observation checks."""
from decimal import Decimal
import hashlib
import json
import math
from pathlib import Path
import struct

SCENARIO_ID = 'curve-coefficient-holdouts'
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
    if scenario.get('id') != SCENARIO_ID:
        if present:
            raise ValueError('Engine torque holdouts are reserved for their exact scenario')
        return
    if scenario.get('probe_kind') != 'curve' or present != {INPUT_FIELD, PROVENANCE_FIELD}:
        raise ValueError('Curve holdout scenario requires both declarations and curve kind')
    inputs = scenario[INPUT_FIELD]
    if not isinstance(inputs, list) or len(inputs) != 25:
        raise ValueError('Curve holdouts require exactly 25 inputs')
    actual = tuple(bits(value) for value in inputs)
    if (actual != INPUT_BITS or len(set(actual)) != 25 or
        any(type(value) not in (int, float) or struct.unpack('>f', bytes.fromhex(bit))[0] != value
            for value, bit in zip(inputs, actual))):
        raise ValueError('Curve holdouts differ from the ordered exact binary32 inputs')
    packed = b''.join(bytes.fromhex(bit)[::-1] for bit in actual)
    if hashlib.sha256(packed).hexdigest() != PROVENANCE['inputs_binary32_le_sha256']:
        raise ValueError('Curve holdout input hash differs')
    if scenario[PROVENANCE_FIELD] != PROVENANCE:
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
    if scenario.get('id') != SCENARIO_ID:
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
    if not isinstance(inputs, list) or not isinstance(values, list) or len(inputs) != 77 or len(values) != 77:
        raise ValueError('Native engine torque requires 77 inputs and values')
    expected = tuple(bits(value) for value in original_inputs(serialized['keys'])) + INPUT_BITS
    if tuple(bits(value) for value in inputs) != expected:
        raise ValueError('Native engine torque prefix or suffix bits differ')
    # Keep native decimal tokens separately. Qualification compares the binary32 readouts only.
    return {'input_bits': list(expected), 'value_bits': [bits(value) for value in values],
            'native_value_decimals': [str(value) for value in values]}


def require_same_curve(first, current):
    if first is not None and any(first[key] != current[key] for key in ('input_bits', 'value_bits')):
        raise ValueError('Fresh native repeats have different engine torque curve bits')
