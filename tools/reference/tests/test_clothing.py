import unittest
from types import SimpleNamespace as NS

from tools.reference.appearance_preview import recover_preview
from tools.reference.library.types import ObjectId

IDENTITY = {'x': 0., 'y': 0., 'z': 0., 'w': 1.}


def link(path_id):
    return {'m_FileID': 0, 'm_PathID': path_id}


def xyz(x, y, z):
    return {'x': x, 'y': y, 'z': z}


class FakeSource:
    """camStuff with a camera, a point light and a figure with one skin joint. The script lists them."""
    def __init__(self):
        self.objects, self.data, self.classes, self.dependencies = {}, {}, {}, {}
        self.library = NS(source_objects=self.dependencies)
        self.add(200, 'MonoScript', {'m_ClassName': 'settingsplayercustomscript'})
        self.stage = self.node(10, 'camStuff', 0, (5., 0., 0.))
        camera = self.node(20, 'Cam', 10, (0., 1., 2.))
        self.add(22, 'Camera', {'m_GameObject': link(camera + 1), 'orthographic': 0, 'field of view': 30.,
            'near clip plane': .1, 'far clip plane': 100.})
        light = self.node(30, 'Light', 10, (1., 2., 3.))
        self.add(32, 'Light', {'m_GameObject': link(light + 1), 'm_Enabled': 1, 'm_Type': 2, 'm_Color': {'r': 1., 'g': .5, 'b': .25, 'a': 1.},
            'm_Intensity': 1.5, 'm_Range': 10., 'm_SpotAngle': 30.})
        figure = self.node(40, 'Figure', 10, (0., 0., 3.))
        self.node(50, 'Hips', 40, (0., 1., 0.))
        self.add(61, 'SkinnedMeshRenderer', {'m_Bones': [link(50)]})
        self.add(60, 'MonoBehaviour', {'ch': {'renderers': [link(61)]}}, 'characterscript')
        self.add(42, 'MonoBehaviour', {'m_GameObject': link(figure + 1), 'selectedCharacter': 0, 'characters': [link(60)]}, 'playermodeloutfitscript')
        self.components = {10: [], 20: [22], 30: [32], 40: [42], 50: []}
        self.script(100)

    def add(self, path_id, kind, data, class_name=None):
        reader = NS(assets_file=NS(source_container_path='level1'), path_id=path_id, type=NS(name=kind))
        self.objects['level1', path_id] = reader
        self.data[path_id] = data
        self.classes[path_id] = class_name
        self.dependencies[ObjectId('level1', path_id)] = {'dependencies': []}
        return reader

    def node(self, path_id, name, parent, position):
        self.add(path_id, 'Transform', {'m_GameObject': link(path_id + 1), 'm_Father': link(parent),
            'm_LocalPosition': xyz(*position), 'm_LocalRotation': IDENTITY, 'm_LocalScale': xyz(1., 1., 1.)})
        self.add(path_id + 1, 'GameObject', {'m_Name': name})
        return path_id

    def script(self, path_id, cam=22):
        self.add(path_id, 'MonoBehaviour', {'camStuff': link(10), 'cam': link(cam), 'outfit': link(42)}, 'settingsplayercustomscript')
        self.dependencies[ObjectId('level1', path_id)] = {'dependencies': [{'file': 'level1', 'path_id': 200}]}

    def read(self, reader):
        return self.data[reader.path_id]

    def class_name(self, reader):
        return self.classes[reader.path_id]

    def resolve(self, owner, pointer):
        return self.objects.get(('level1', pointer['m_PathID'])) if pointer['m_PathID'] else None

    def transform_for(self, game_object):
        return self.objects['level1', game_object.path_id - 1]

    def hierarchy(self, root, activate_root=False):
        return [{'reader': self.objects['level1', t], 'active': True, 'components': [self.objects['level1', c] for c in components]}
            for t, components in self.components.items()]


class PreviewTests(unittest.TestCase):
    def test_found(self):
        preview = recover_preview(FakeSource())
        camera = preview['camera']
        # Poses are local to camStuff. The z axis flips.
        self.assertEqual(camera['pose']['translation'], [0., 1., -2.])
        self.assertEqual((camera['fov'], camera['near'], camera['far']), (30., .1, 100.))
        self.assertEqual([(l['kind'], l['pose']['translation'], l['intensity']) for l in preview['lights']], [('point', [1., 2., -3.], 1.5)])
        figure = preview['figure']
        self.assertEqual((figure['root_pose']['translation'], figure['character']), ([0., 0., -3.], 0))
        self.assertEqual([(j['name'], j['pose']['translation']) for j in figure['joints']], [('Hips', [0., 1., -3.])])

    def test_missing(self):
        source = FakeSource()
        del source.objects['level1', 100]
        with self.assertRaisesRegex(ValueError, 'found 0'):
            recover_preview(source)
        source = FakeSource()
        source.script(100, cam=0)
        with self.assertRaisesRegex(ValueError, 'camera'):
            recover_preview(source)

    def test_two_scripts(self):
        source = FakeSource()
        source.script(101)
        with self.assertRaisesRegex(ValueError, 'found 2'):
            recover_preview(source)


if __name__ == '__main__':
    unittest.main()
