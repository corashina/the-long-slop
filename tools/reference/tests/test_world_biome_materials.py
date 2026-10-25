import unittest
from types import SimpleNamespace as NS
from tools.reference.world_biome_materials import recover_sets, used_sets, check_exported, export_sets

FILE = NS(name='level1', source_container_path='level1')


def obj(path_id, kind):
    return NS(assets_file=FILE, path_id=path_id, type=NS(name=kind))


def ptr(path_id):
    return {'m_FileID': 0, 'm_PathID': path_id}


class FakeSource:
    def __init__(self, objects, data, classes):
        self.objects = {('level1', o.path_id): o for o in objects}
        self.data, self.classes = data, classes

    def read(self, reader):
        return self.data[reader.path_id]

    def resolve(self, owner, pointer):
        return self.objects.get(('level1', pointer['m_PathID'])) if pointer['m_PathID'] else None

    def class_name(self, reader):
        return self.classes.get(reader.path_id)


class BiomeMaterialTests(unittest.TestCase):
    def scene(self, sets):
        materials = [obj(i, 'Material') for i in (11, 12, 13, 21)]
        pairs = obj(1, 'MonoBehaviour')
        root, child = obj(2, 'Transform'), obj(3, 'Transform')
        swap, renderer = obj(4, 'MonoBehaviour'), obj(5, 'MeshRenderer')
        data = {1: {'materials': [{'m': [ptr(i) for i in row]} for row in sets]},
                4: {'m_Enabled': 1}, 5: {'m_Materials': [ptr(11), ptr(21)]}}
        source = FakeSource([*materials, pairs, root, child, swap, renderer], data, {4: 'materialChangeScript'})
        records = [{'reader': root, 'parent': None, 'components': [swap]},
                   {'reader': child, 'parent': root, 'components': [renderer]}]
        return source, pairs, records

    def test_used_set_is_exported_with_null_variants(self):
        source, pairs, records = self.scene([[21, 0, 0], [11, 0, 13], [11, 12, 12]])
        sets = recover_sets(source, pairs)
        self.assertEqual(len(sets), 2)  # The first set of a desert material wins.
        used = used_sets(source, records, sets)
        self.assertEqual(used, {1})
        check_exported(sets, used, {('level1', 11), ('level1', 13)}, 'fixture')
        with self.assertRaises(ValueError):
            check_exported(sets, used, {('level1', 11)}, 'fixture')
        selected = NS(receipt=lambda m: None)
        self.assertEqual(export_sets(sets, used, selected),
                         [[{'file': 'level1', 'path_id': 11}, None, {'file': 'level1', 'path_id': 13}]])

    def test_malformed_sets_fail(self):
        for rows in ([[11, 12]], [[0, 12, 13]], []):
            source, pairs, _ = self.scene(rows)
            with self.assertRaises(ValueError):
                recover_sets(source, pairs)


if __name__ == '__main__':
    unittest.main()
