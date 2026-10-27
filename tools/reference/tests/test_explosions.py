import copy
import unittest

from tools.reference import explosions


def sid(path_id, file='sharedassets1.assets'):
    return {'file': file, 'path_id': path_id}


def kind(index):
    particle = {'role': f'explosion_{index}', 'materials': [{'textures': [{'texture': 1}]}],
                'curves': {'InitialModule.startLifetime': {}}}
    return {'prefab': sid(100 + index), 'source': sid(110 + index), 'explosion_force': 10., 'multiplier': 1.,
            'explosion_dist': 5., 'particle': particle, 'sound': {'source': sid(120 + index), 'clip': sid(5), 'volume': 1.}}


class ExplosionsTests(unittest.TestCase):
    def setUp(self):
        hole = {'prefab': sid(200), 'source': sid(201), **{k: 1. for k in explosions.HOLE_FIELDS}}
        hole.update({k: False for k in explosions.HOLE_FLAGS})
        player = {'source': sid(80125, 'level1'), **{k: 1. for k in explosions.PLAYER_FIELDS},
                  'ear_ring': {'source': sid(300), 'clip': sid(6), 'volume': 1.}}
        self.record = {'source': sid(64919, 'level1'), 'types': [kind(i) for i in range(4)], 'hole': hole,
                       'player': player, 'explosive_fluids': [0, 1, 5]}
        self.manifest = {'audio': {'clips': [{'source': sid(5)}, {'source': sid(6)}]},
                         'lighting': {'textures': [{'id': 1}]}}

    def test_validate_explosions(self):
        explosions.validate_explosions(self.record, self.manifest)
        for change in [{'types': self.record['types'][:3]}, {'extra': 1}, {'explosive_fluids': [0, 6]},
                       {'explosive_fluids': [0, 0]}, {'explosive_fluids': []}]:
            with self.subTest(change=list(change)), self.assertRaises(ValueError):
                explosions.validate_explosions({**self.record, **change}, self.manifest)

    def test_link_errors(self):
        for path, value, text in [(('types', 2, 'sound', 'clip'), sid(7), 'type 2'),
                                  (('types', 1, 'particle', 'role'), 'explosion_0', 'type 1'),
                                  (('player', 'ear_ring', 'clip'), sid(7), 'missing from audio'),
                                  (('hole', 'valve'), True, 'float')]:
            record = copy.deepcopy(self.record)
            target = record
            for field in path[:-1]:
                target = target[field]
            target[path[-1]] = value
            with self.subTest(path=path), self.assertRaisesRegex(ValueError, text):
                explosions.validate_explosions(record, self.manifest)
