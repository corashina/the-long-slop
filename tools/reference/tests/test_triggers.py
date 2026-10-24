import math
import unittest
from types import SimpleNamespace as NS
from unittest import mock
from tools.reference import triggers


def reader(identity, kind):
    return NS(assets_file=NS(source_container_path='fixture'), path_id=identity, type=NS(name=kind))


class FakeSource:
    def __init__(self, scripts):
        half = math.sqrt(.5)
        self.root = reader(1, 'Transform'); self.child = reader(2, 'Transform'); self.ladder_t = reader(3, 'Transform')
        identity = dict(m_LocalPosition=dict(x=0., y=0., z=0.), m_LocalScale=dict(x=1., y=1., z=1.))
        self.data = {1: dict(identity, m_LocalRotation=dict(x=0., y=0., z=0., w=1.), m_Father=None),
                     2: dict(identity, m_LocalRotation=dict(x=0., y=0., z=0., w=1.), m_Father=self.root),
                     # Unity +90 degrees about x: local up points to Unity +z.
                     3: dict(identity, m_LocalRotation=dict(x=half, y=0., z=0., w=half), m_Father=self.root)}
        self.scripts = scripts

    def read(self, r):
        return self.data[r.path_id] if r.type.name == 'Transform' else r.values

    def resolve(self, owner, pointer):
        return pointer

    def components(self, transform):
        return None, self.scripts.get(transform.path_id, [])

    def class_name(self, r):
        return r.name

    def chain(self, transform):
        result = []
        while transform:
            result.append(self.data[transform.path_id]); transform = result[-1]['m_Father']
        return result


def script(name, values):
    return NS(assets_file=NS(source_container_path='fixture'), path_id=9, type=NS(name='MonoBehaviour'), name=name, values=values)


IDENTITY = {'translation': [0., 0., 0.], 'rotation': [0., 0., 0., 1.], 'scale': [1., 1., 1.]}


@mock.patch.object(triggers, 'managed_evidence')
class TriggerRoleTests(unittest.TestCase):
    def test_ladder_on_parent_gives_converted_up(self, _):
        source = FakeSource({})
        source.scripts[1] = [script('ladderscript', dict(T=source.ladder_t, speed=2.))]
        role = triggers.recover_trigger_role(source, source.child, source.root, IDENTITY)
        self.assertEqual(role['ladder']['speed'], 2.)
        for actual, expected in zip(role['ladder']['up'], [0., 0., -1.]):
            self.assertAlmostEqual(actual, expected)

    def test_hazard_and_plain_trigger(self, _):
        source = FakeSource({})
        self.assertIsNone(triggers.recover_trigger_role(source, source.child, source.root, IDENTITY))
        source.scripts[2] = [script('playerdeathtriggerscript', dict(damagePlayer=50., damageBreakable=10.))]
        self.assertEqual(triggers.recover_trigger_role(source, source.child, source.root, IDENTITY),
                         {'hazard': {'damage_player': 50., 'damage_breakable': 10.}})


if __name__ == '__main__':
    unittest.main()
