import unittest
from types import SimpleNamespace as NS

from tools.reference.ragdoll import recover_joints, recover_pose_map


def reader(path_id, type_name='MonoBehaviour', class_name=''):
    return NS(assets_file=NS(name='level1', source_container_path='level1'), path_id=path_id,
              type=NS(name=type_name), class_name=class_name)


def link(path_id):
    return {'m_FileID': 0, 'm_PathID': path_id}


def xyz(x, y, z):
    return {'x': x, 'y': y, 'z': z}


def limit(value):
    return {'limit': value, 'bounciness': 0., 'contactDistance': 0.}


def transform(position, father):
    return {'m_LocalPosition': xyz(*position), 'm_LocalRotation': {'x': 0., 'y': 0., 'z': 0., 'w': 1.},
            'm_LocalScale': xyz(1., 1., 1.), 'm_Father': link(father)}


class RagdollTests(unittest.TestCase):
    def setUp(self):
        # Root 1, pelvis 2 at the root, spine 3 one metre up. The spine joint connects to the pelvis.
        self.objects = {i: reader(i, 'Transform') for i in (1, 2, 3)}
        self.objects.update({20: reader(20, 'Rigidbody'), 30: reader(30, 'Rigidbody')})
        self.data = {1: transform((0., 0., 0.), 0), 2: transform((0., 0., 0.), 1), 3: transform((0., 1., 0.), 2),
            40: {'m_ConnectedBody': link(20), 'm_Anchor': xyz(0., 0., 0.), 'm_AutoConfigureConnectedAnchor': 1,
                 'm_ConnectedAnchor': xyz(9., 9., 9.), 'm_Axis': xyz(2., 0., 0.), 'm_SwingAxis': xyz(0., 0., 1.),
                 'm_LowTwistLimit': limit(-20.), 'm_HighTwistLimit': limit(70.), 'm_Swing1Limit': limit(30.),
                 'm_Swing2Limit': limit(15.), 'm_TwistLimitSpring': {'spring': 0., 'damper': 0.},
                 'm_SwingLimitSpring': {'spring': 0., 'damper': 0.}, 'm_EnableProjection': 0,
                 'm_ProjectionDistance': .1, 'm_ProjectionAngle': 180., 'm_BreakForce': float('inf'),
                 'm_BreakTorque': float('inf'), 'm_EnableCollision': 0}}
        self.source = NS(read=lambda r: self.data[r.path_id], class_name=lambda r: r.class_name,
            resolve=lambda owner, pointer: self.objects.get(pointer['m_PathID']),
            chain=lambda t: [self.data[i] for i in {1: (1,), 2: (2, 1), 3: (3, 2, 1)}[t.path_id]])
        identity = {'rotation': [0., 0., 0., 1.], 'scale': [1., 1., 1.]}
        self.bodies = [{'id': 1, 'node': 2, 'source': {'file': 'level1', 'path_id': 20}},
                       {'id': 2, 'node': 3, 'source': {'file': 'level1', 'path_id': 30}}]
        self.poses = {2: {'translation': [0., 0., 0.], **identity}, 3: {'translation': [0., 1., 0.], **identity}}

    def by_node(self, joint):
        return {2: {'reader': self.objects[2], 'components': [self.objects[20]]},
                3: {'reader': self.objects[3], 'components': [self.objects[30], joint]}}

    def test_joints_and_pose_map(self):
        joints = recover_joints(self.source, self.by_node(reader(40, 'CharacterJoint')), self.bodies, self.poses)
        self.assertEqual(len(joints), 1)
        j = joints[0]
        self.assertEqual((j['kind'], j['body'], j['connected_body']), ('character', 2, 1))
        self.assertEqual((j['anchor'], j['connected_anchor']), ([0., 0., 0.], [0., 1., 0.]))
        # Axial conversion: x and y flip, z stays.
        self.assertEqual((j['axis'], j['swing_axis']), ([-1., 0., 0.], [0., 0., 1.]))
        self.assertEqual((j['low_twist']['limit'], j['high_twist']['limit'], j['swing1']['limit']), (-20., 70., 30.))
        with self.assertRaises(ValueError):
            recover_joints(self.source, self.by_node(reader(40, 'HingeJoint')), self.bodies, self.poses)
        self.data[40]['m_BreakForce'] = 100.
        with self.assertRaises(ValueError):
            recover_joints(self.source, self.by_node(reader(40, 'CharacterJoint')), self.bodies, self.poses)

        own, live = reader(50, class_name='ragdolPosScript'), reader(51, class_name='ragdolPosScript')
        self.data[50] = {'head': link(3), 'ragdolT': [link(2), link(3)]}
        self.data[51] = {'head': link(0), 'ragdolT': [link(102), link(103)]}
        node = lambda r: r.path_id
        self.objects.update({102: reader(102, 'Transform'), 103: reader(103, 'Transform')})
        pose = recover_pose_map(self.source, own, live, node)
        self.assertEqual(pose['head'], 3)
        self.assertEqual(pose['pairs'], [{'ragdoll': 2, 'live': 102}, {'ragdoll': 3, 'live': 103}])
        self.data[51]['ragdolT'].pop()
        with self.assertRaises(ValueError):
            recover_pose_map(self.source, own, live, node)


if __name__ == '__main__':
    unittest.main()
