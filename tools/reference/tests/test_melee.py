import copy
import unittest
from unittest.mock import patch

from tools.reference import melee


def sid(path_id, file='sharedassets1.assets'):
    return {'file': file, 'path_id': path_id}


def weapon(prototype, source, controller, **values):
    return {'prototype': sid(prototype), 'source': sid(source), 'pickup': sid(source + 1),
            'target': sid(prototype + 1), 'trigger': sid(prototype + 2), 'contact_mask': 8552961,
            'speed': .699999988079071, 'shovel_speed': .5, 'attack_volume': .5, 'attack_clips': [sid(5)],
            'controller': sid(controller), 'right_hand_poses': [], 'left_hand_poses': [],
            'contact': dict(melee.SHOVEL_CONTACT), **values}


class MeleeTests(unittest.TestCase):
    def setUp(self):
        particle = {'materials': [1], 'curves': {'InitialModule.startLifetime': {}}}
        shovel = weapon(61587, 132938, 4515)
        katana = weapon(100, 132935, 4600, speed=1., right_hand_poses=[sid(101)])
        self.record = {'source': {}, 'hit_volumes': list(melee.HIT_VOLUMES), 'particles': [particle] * 5,
                       'hit_clips': [[sid(6)]] * 6, 'recent_recovery_seconds': 1.,
                       'weapons': [katana, shovel],
                       'motions': [{'controller': sid(4515)}, {'controller': sid(4600)}]}
        self.manifest = {
            'audio': {'clips': [{'source': sid(5)}, {'source': sid(6)}]},
            'world': {'prototypes': [{'source': sid(p), 'nodes': [{'source': sid(p + i)} for i in range(3)]}
                                     for p in (100, 61587)]},
            'item_prototypes': [{'item_id': 216, 'id': sid(61587), 'pickups': [{'pickup': {'source': sid(132939)}}],
                                 'behavior': {'melee': sid(132938), 'attack_target': sid(61588), 'attack_trigger': sid(61589),
                                              'clips': [sid(5)], 'contact_clips': [[sid(6)]] * 6}}]}

    def check(self, record):
        with patch.object(melee, 'validate_motion'):
            melee.validate_melee(record, self.manifest)

    def test_validate_melee(self):
        self.check(self.record)
        for change in [{'hit_volumes': [0.3] * 6}, {'particles': self.record['particles'][:4]}, {'extra': 1},
                       {'weapons': self.record['weapons'][::-1]}, {'motions': self.record['motions'][:1]}]:
            with self.subTest(change=list(change)), self.assertRaises(ValueError):
                self.check({**self.record, **change})

    def test_weapon_errors_name_the_weapon(self):
        for index, field, value, text in [(0, 'right_hand_poses', [sid(999)], '132935'),
                                          (0, 'attack_clips', [sid(7)], '132935'),
                                          (1, 'speed', 1., '132938')]:
            record = copy.deepcopy(self.record)
            record['weapons'][index][field] = value
            with self.subTest(field=field), self.assertRaisesRegex(ValueError, text):
                self.check(record)
