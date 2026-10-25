import unittest

from tools.reference import melee


class MeleeTests(unittest.TestCase):
    def test_validate_melee(self):
        particle = {'materials': [1], 'curves': {'InitialModule.startLifetime': {}}}
        record = {'source': {}, 'hit_volumes': list(melee.HIT_VOLUMES), 'particles': [particle] * 5}
        melee.validate_melee(record, {})
        for change in [{'hit_volumes': [0.3] * 6}, {'particles': [particle] * 4}, {'extra': 1}]:
            with self.assertRaises(ValueError): melee.validate_melee({**record, **change}, {})
