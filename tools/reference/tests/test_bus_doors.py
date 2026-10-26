import unittest
from types import SimpleNamespace as NS
from unittest import mock
from tools.reference import bus_doors


class Reader(NS):
    pass


def reader(file, path_id, type_name='MonoBehaviour'):
    return Reader(assets_file=NS(source_container_path=file), path_id=path_id, type=NS(name=type_name))


class FakeSource:
    def __init__(self):
        self.car = reader('a', 1)
        self.body = reader('a', 2, 'Rigidbody')
        self.chassis = reader('a', 3, 'Transform')
        self.leaf = reader('a', 4, 'Transform')
        self.door = reader('a', 5)
        self.other = reader('a', 6)
        self.data = {1: {'RB': 'rb'}, 2: {'m_GameObject': 'go'}, 5: {'sounds': ['s1', 's2']}, 6: {}}

    def read(self, r):
        return self.data[r.path_id]

    def resolve(self, owner, pointer):
        return {'rb': self.body, 'go': 'chassis-go'}[pointer]

    def transform_for(self, go):
        return self.chassis

    def class_name(self, c):
        return {5: 'busdooropenscript', 6: 'usablescript'}[c.path_id]

    def hierarchy(self, root, activate_root=False):
        return [{'reader': self.chassis, 'components': [self.other]},
                {'reader': self.leaf, 'components': [self.door]}]


class BusDoorAudioTests(unittest.TestCase):
    def test_leaf_sounds_play_at_the_leaf(self):
        source = FakeSource()
        shots = []

        def shot(owner, pointer):
            shots.append((owner.path_id, pointer))
            return {'voice': 1, 'clip': len(shots), 'gain': 1., 'position': [0., 0., 0.]}

        pose = {'translation': [1., 2., 3.]}
        with mock.patch.object(bus_doors, 'relative_pose', return_value=pose) as relative:
            records = bus_doors.door_audio(source, source.chassis, source.car, shot)
        relative.assert_called_once_with(source, source.leaf, source.chassis)
        self.assertEqual(shots, [(5, 's1'), (5, 's2')])
        self.assertEqual(records, [{'source': {'file': 'a', 'path_id': 5}, 'sounds': [
            {'voice': 1, 'clip': 1, 'gain': 1., 'position': [1., 2., 3.]},
            {'voice': 1, 'clip': 2, 'gain': 1., 'position': [1., 2., 3.]}]}])

    def test_null_sound_is_rejected(self):
        source = FakeSource()
        with mock.patch.object(bus_doors, 'relative_pose', return_value={'translation': [0., 0., 0.]}):
            with self.assertRaisesRegex(ValueError, 'sound is null'):
                bus_doors.door_audio(source, source.chassis, source.car, lambda owner, pointer: None)


if __name__ == '__main__':
    unittest.main()
