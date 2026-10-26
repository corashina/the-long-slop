import unittest
from types import SimpleNamespace as NS

from tools.reference import breakables


def reader(path_id, type_name='MonoBehaviour'):
    return NS(assets_file=NS(source_container_path='a'), path_id=path_id, type=NS(name=type_name))


def pointer(path_id):
    return {'m_FileID': 0, 'm_PathID': path_id}


def script(**changes):
    d = {'health': 6., 'shootHealth': 0., 'explosionHealth': 0., 'onlyDecreaseAfterAttackIfAboveHealth': 0.,
         'childModifier': 0., 'massModifier': 0., 'childs': [], 'clips': [], 'Gib': pointer(0),
         'reducedGib': pointer(0), 'AI': pointer(0), 'newAI': pointer(0), 'lamp': pointer(0), 'destroyed': 0,
         **{v: 0 for v in breakables.FLAGS.values()}}
    d.update(changes)
    return d


class FakeSource:
    def __init__(self, data):
        self.item_clips = set()
        self.readers = {1: reader(1, 'Transform'), 2: reader(2, 'Transform'), 3: reader(3, 'Transform'),
                        10: reader(10), 11: reader(11), 12: reader(12), 20: reader(20, 'GameObject'),
                        21: reader(21, 'Transform'), 30: reader(30, 'AudioClip')}
        self.data = data
        self.classes = {10: 'flashlightscript', 11: 'breakablescript', 12: 'breakablescript'}

    def read(self, r):
        return self.data[r.path_id]

    def resolve(self, owner, p):
        return self.readers.get(p['m_PathID'])

    def class_name(self, r):
        return self.classes[r.path_id]

    def transform_for(self, go):
        return self.readers[21]


class BreakableTests(unittest.TestCase):
    def test_lamp_trigger_and_bottle(self):
        source = FakeSource({
            10: {'Lamps': [{'breaktrigger': []}, {'breaktrigger': [pointer(11)]}]},
            11: script(health=5., decreaseAfterAttack=1, noTrigger=1, noCollider=1),
            12: script(Gib=pointer(20), clips=[pointer(30)])})
        r = source.readers
        hierarchy = [{'reader': r[1], 'components': [r[1], r[10]]}, {'reader': r[2], 'components': [r[11]]},
                     {'reader': r[3], 'components': [r[12]]}]
        owners = [('world', {'file': 'a', 'path_id': 1}, hierarchy, {('a', 1): 0, ('a', 2): 1, ('a', 3): 2})]
        lamp, bottle = breakables.breakable_records(source, owners, {('a', 10)})
        self.assertEqual(lamp['owners'], [{'kind': 'world', 'prototype': {'file': 'a', 'path_id': 1}, 'node': 1}])
        self.assertEqual(lamp['lamp'], {'control': {'file': 'a', 'path_id': 10}, 'lamps': [1]})
        self.assertEqual((lamp['health'], lamp['decrease_after_attack'], lamp['no_trigger'], lamp['mass_modifier']),
                         (5., True, True, 1.))
        self.assertEqual((bottle['gib'], bottle['clips'], bottle['lamp']),
                         ({'file': 'a', 'path_id': 21}, [{'file': 'a', 'path_id': 30}], None))
        self.assertEqual(source.item_clips, {('a', 30)})
        with self.assertRaisesRegex(ValueError, 'outside the export'):
            breakables.breakable_records(source, owners, set())
        source.data[12] = script(newAI=pointer(20))
        with self.assertRaisesRegex(ValueError, 'AI link'):
            breakables.breakable_records(source, owners, {('a', 10)})


if __name__ == '__main__':
    unittest.main()
