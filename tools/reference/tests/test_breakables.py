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

    def test_gib_links(self):
        gib = {'file': 'a', 'path_id': 21}
        records = [{'gib': gib, 'reduced_gib': None}]
        breakables.check_gibs(records, [{'id': gib, 'item_id': None}, {'id': {'file': 'a', 'path_id': 1}, 'item_id': 62}])
        with self.assertRaisesRegex(ValueError, 'not item_prototypes.GIBS'):
            breakables.check_gibs(records, [])
        with self.assertRaisesRegex(ValueError, 'without a breakable'):
            breakables.check_gibs([], [{'id': gib, 'item_id': None}])


class GibTests(unittest.TestCase):
    def source(self, gib):
        source = FakeSource({11: gib, 12: {}})
        source.classes = {11: 'gibscript', 12: 'scaleDestroyScript'}
        r = source.readers
        return source, r

    def test_gib_root(self):
        from tools.reference.item_prototypes import recover_gib
        gib = {'gibMain': 1, 'gibMainDontRot': 0, 'gibMainAddSmallPos': 1, 'distance': 0, 'life': 30.,
               'colsOff': 0, 'childs': [pointer(2), pointer(3)]}
        source, r = self.source(gib)
        nodes = {('a', 1): 1, ('a', 2): 2, ('a', 3): 3}
        records = [{'reader': r[1], 'parent': None, 'components': [r[1], r[11]]},
                   {'reader': r[2], 'parent': r[1], 'components': [r[12]]},
                   {'reader': r[3], 'parent': r[2], 'components': [r[12]]}]
        self.assertEqual(recover_gib(source, records, nodes), (
            {'source': {'file': 'a', 'path_id': 11}, 'gib_main': True, 'dont_rot': False, 'add_small_pos': True,
             'distance': False, 'life': 30., 'children': [2, 3]}, [], [2, 3]))
        # A gib without gibscript keeps only its scaleDestroyScript nodes.
        self.assertEqual(recover_gib(source, records[1:], nodes), (None, [], [2, 3]))
        # A gib piece keeps its own life over its subtree. A main gibscript on a piece is not audited.
        gib['gibMain'] = 0
        piece = dict(records[1], components=[r[11]])
        self.assertEqual(recover_gib(source, [records[0], piece, records[2]], nodes)[1], [
            {'source': {'file': 'a', 'path_id': 11}, 'node': 2, 'distance': False, 'life': 30., 'nodes': [2, 3]}])
        gib['gibMain'] = 1
        with self.assertRaisesRegex(ValueError, 'piece gibscript start state'):
            recover_gib(source, [records[0], piece, records[2]], nodes)
        with self.assertRaisesRegex(ValueError, 'outside the gib'):
            recover_gib(source, records, {('a', 1): 1, ('a', 2): 2})
        gib['colsOff'] = 1
        with self.assertRaisesRegex(ValueError, 'start state'):
            recover_gib(source, records, nodes)


if __name__ == '__main__':
    unittest.main()
