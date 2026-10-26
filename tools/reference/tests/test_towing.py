import unittest
from types import SimpleNamespace as NS

from tools.reference.towing import recover_hitches, recover_trailer


def reader(path_id, type_name='MonoBehaviour', class_name=''):
    return NS(assets_file=NS(name='level1', source_container_path='level1'), path_id=path_id,
              type=NS(name=type_name), class_name=class_name)


def link(path_id):
    return {'m_FileID': 0, 'm_PathID': path_id}


class TowingTests(unittest.TestCase):
    def test_only_trailer_targets_with_one_collider_become_hitches(self):
        hitch_node, target_node, plate_node = (reader(n, 'Transform') for n in (10, 11, 12))
        hitch = reader(20, class_name='attachTargetscript')
        plate = reader(21, class_name='attachTargetscript')
        data = {20: {'tipus': ['utanfuto'], 'target': link(11)}, 21: {'tipus': ['licenseplate'], 'target': link(11)}}
        source = NS(read=lambda r: data[r.path_id], class_name=lambda r: r.class_name,
                    resolve=lambda owner, pointer: {11: target_node}.get(pointer['m_PathID']))
        hierarchy = [dict(reader=hitch_node, components=[hitch]), dict(reader=target_node, components=[]),
                     dict(reader=plate_node, components=[plate])]
        mapping = {('level1', 10): 0, ('level1', 11): 1, ('level1', 12): 2}
        self.assertEqual(recover_hitches(source, hierarchy, mapping, [dict(node=0), dict(node=2)]), [
            {'source': {'file': 'level1', 'path_id': 20}, 'node': 0, 'target': 1, 'collider': 0, 'types': ['utanfuto']}])
        with self.assertRaises(ValueError):
            recover_hitches(source, hierarchy, mapping, [dict(node=2)])

    def test_trailer_keeps_grip_mass_points_and_brake_switch(self):
        root, wheel_node, com, attached = (reader(n, 'Transform') for n in (1, 2, 3, 4))
        grip_root = reader(5, 'Transform')
        utanfuto = reader(10, class_name='utanfutoscript')
        grip = reader(11, class_name='attachgripscript')
        brake = reader(12, class_name='flashlightscript')
        attachable = reader(13, class_name='attachablescript')
        rb, wheel, prefab = reader(20, 'Rigidbody'), reader(21, 'WheelCollider'), reader(22, 'GameObject')
        curve = dict(m_ExtremumSlip=0.4, m_ExtremumValue=1., m_AsymptoteSlip=0.8, m_AsymptoteValue=0.5, m_Stiffness=1.)
        data = {
            10: {'RB': link(20), 'wheels': [link(21)], 'brake': link(12), 'brakeForce': 100.,
                 'CenterOfMass': link(3), 'CenterOfMassAttached': link(4)},
            11: {'fixGripPos': 1, 'grips': [], 'targetTosave': link(0), 'GripPrefab': link(22),
                 'VgripPos': {'x': 0., 'y': 0.4, 'z': 1.6}},
            13: {'onlyToPoint': 1, 'tipus': ['utanfuto']},
            21: {'m_GameObject': link(30), 'm_Radius': 0.3, 'm_SuspensionDistance': 0.2, 'm_Center': {'x': 0., 'y': 0., 'z': 0.},
                 'm_SuspensionSpring': {'spring': 1., 'damper': 1., 'targetPosition': 0.5}, 'm_ForwardFriction': curve,
                 'm_SidewaysFriction': curve, 'm_Mass': 1., 'm_WheelDampingRate': 0.25, 'm_ForceAppPointDistance': 0.},
        }
        objects = {20: rb, 21: wheel, 12: brake, 3: com, 4: attached, 22: prefab, 30: reader(30, 'GameObject')}
        transforms = {30: wheel_node, 22: grip_root, 3: com, 4: attached}
        source = NS(read=lambda r: data[r.path_id], class_name=lambda r: r.class_name,
                    resolve=lambda owner, pointer: objects.get(pointer['m_PathID']),
                    transform_for=lambda r: transforms[r.path_id],
                    components=lambda t: (None, [attachable]))
        hierarchy = [dict(reader=root, components=[utanfuto, grip, rb]), dict(reader=wheel_node, components=[wheel, brake])]
        mapping = {('level1', 1): 0, ('level1', 2): 1}
        bodies = [{'source': {'file': 'level1', 'path_id': 20}, 'node': 0}]
        points = {3: [0., 0.4, -0.3], 4: [0., 0.4, -0.75]}
        record = recover_trailer(source, hierarchy, mapping, bodies, lambda t: points[t.path_id], lambda: [{'kind': 'rear'}])
        self.assertEqual((record['body'], record['brake_force'], record['brake']), (0, 100., {'file': 'level1', 'path_id': 12}))
        self.assertEqual((record['center_of_mass'], record['center_of_mass_hitched']), (points[3], points[4]))
        self.assertEqual(record['grip'], {'source': {'file': 'level1', 'path_id': 11}, 'prototype': {'file': 'level1', 'path_id': 5},
                                          'position': [0., 0.4, -1.6], 'types': ['utanfuto']})
        self.assertEqual([w['node'] for w in record['wheels']], [1])
        self.assertIsNone(recover_trailer(source, hierarchy[1:], mapping, bodies, None, None))


if __name__ == '__main__':
    unittest.main()
