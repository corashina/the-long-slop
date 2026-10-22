"""Keep the native G11 comparison groups explicit and complete."""
from copy import deepcopy
import json
from pathlib import Path
import unittest

SCENARIOS = Path(__file__).resolve().parents[3] / 'tests/fixtures/driving/scenarios.json'
EXPECTED = {
    'wheel': frozenset({
        'grip-full-tire-longitudinal', 'grip-full-tire-lateral', 'grip-full-tire-combined',
        'grip-worn-tire-longitudinal', 'grip-worn-tire-lateral', 'grip-worn-tire-combined',
        'grip-bare-rim-longitudinal', 'grip-bare-rim-lateral', 'grip-bare-rim-combined',
        'grip-hub-longitudinal', 'grip-hub-lateral', 'grip-hub-combined',
        'native-brake-stopped-hold', 'native-motor-and-service',
        'native-motor-and-parking', 'native-motor-and-both',
    }),
    'drivetrain': frozenset({'curve-boundaries'}),
    'assembled_lada': frozenset({
        'launch-default', 'launch-manual', 'gear-forward-neutral-reverse',
        'remove-driven-rims-1', 'remove-driven-rims-2', 'wheel-spin-source-startup',
        'wheel-spin-wheels-clear', 'suspension-settle', 'suspension-full-travel',
        'suspension-load', 'suspension-drop', 'braking-service', 'braking-engine',
        'braking-handbrake', 'braking-combined', 'motor-and-brake-service',
        'motor-and-brake-handbrake', 'motor-and-brake-engine', 'motor-and-brake-all',
        'coast-clutch-stop-slope-hold', 'braking-service-automatic',
        'braking-engine-automatic',
    }),
}


def validate_declared_groups(scenarios):
    allowed = EXPECTED.keys()
    actual = {name: set() for name in allowed}
    seen = set()
    for row in scenarios:
        identity = row.get('id')
        group = row.get('comparison_group')
        if identity in seen or group not in allowed:
            raise ValueError('Scenario identity or comparison group is invalid')
        seen.add(identity)
        actual[group].add(identity)
    if {name: frozenset(ids) for name, ids in actual.items()} != EXPECTED:
        raise ValueError('Scenario comparison groups differ from the reviewed map')


class ScenarioComparisonGroupTests(unittest.TestCase):
    def test_scenarios_have_complete_explicit_comparison_groups(self):
        scenarios = json.loads(SCENARIOS.read_text(encoding='utf-8'))['scenarios']
        self.assertEqual(sum(map(len, EXPECTED.values())), len(scenarios))
        validate_declared_groups(scenarios)

        for corruption in ('missing', 'invalid', 'changed'):
            with self.subTest(corruption=corruption):
                changed = deepcopy(scenarios)
                if corruption == 'missing':
                    del changed[0]['comparison_group']
                elif corruption == 'invalid':
                    changed[0]['comparison_group'] = 'unknown'
                else:
                    changed[0]['comparison_group'] = 'wheel'
                with self.assertRaises(ValueError):
                    validate_declared_groups(changed)


if __name__ == '__main__':
    unittest.main()