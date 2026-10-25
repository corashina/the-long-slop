import copy
import math
import unittest
from tools.reference.hose import chain_order, check_joint, validate_hose

SEGMENTS = [4, 2, 6, 1, 3, 5]


def joint(body, other):
    return {'body': body, 'connected_body': other, 'anchor': [0., 0., .075], 'connected_anchor': [0., 0., -.075]}


def hose():
    opening = {'source': {'file': 'sharedassets1.assets', 'path_id': 1}, 'tank': None, 'flow_liters_per_second': .1, 'flow_radius': .02}
    return {'segments': SEGMENTS, 'main': 6, 't1': 4, 't2': 5,
        'break_force': [500.] * 6, 'connect_anchor': [[0., 0., .075]] * 6,
        'joints': [joint(a, b) for a, b in zip(SEGMENTS, SEGMENTS[1:])],
        'ends': [{'body': body, 'node': 9 + i, 'opening': opening,
            'visual': {'role': f'outlet_hose_{i}'}, 'splash': {'role': f'splash_hose_{i}'}} for i, body in ((1, 4), (2, 5))],
        'skin': {'bones': SEGMENTS, 'bind_poses': [[0.] * 16] * 6}}


PROTOTYPE = {'bodies': [{'id': i} for i in range(1, 7)]}


class HoseTests(unittest.TestCase):
    def test_chain_order(self):
        joints = [joint(a, b) for a, b in zip(SEGMENTS, SEGMENTS[1:])][::-1]
        segments, ordered = chain_order(joints)
        self.assertEqual(segments, SEGMENTS)
        self.assertEqual([j['body'] for j in ordered], SEGMENTS[:5])
        for bad in ([joint(4, 2), joint(4, 6)], [joint(4, 2), joint(2, 6), joint(6, 4)], [joint(4, 2), joint(6, 1), joint(1, 6)]):
            with self.subTest(joints=bad), self.assertRaises(ValueError):
                chain_order(bad)

    def test_unlocked_linear_motion_rejected(self):
        fields = {'m_XMotion': 0, 'm_YMotion': 0, 'm_ZMotion': 0, 'm_AngularXMotion': 2, 'm_AngularYMotion': 2,
            'm_AngularZMotion': 2, 'm_BreakForce': math.inf, 'm_BreakTorque': math.inf}
        check_joint(fields)
        for field, value in (('m_YMotion', 2), ('m_AngularZMotion', 0), ('m_BreakForce', 1000.)):
            with self.subTest(field=field), self.assertRaises(ValueError):
                check_joint({**fields, field: value})

    def test_validate(self):
        validate_hose(hose(), PROTOTYPE)
        broken = []
        missing_end = hose(); missing_end['ends'].pop(); broken.append(missing_end)
        outside = hose(); outside['main'] = 9; broken.append(outside)
        split = hose(); split['joints'][2] = joint(6, 3); broken.append(split)
        endless = copy.deepcopy(hose()); endless['break_force'][0] = math.nan; broken.append(endless)
        for record in broken:
            with self.subTest(record=record), self.assertRaises(ValueError):
                validate_hose(record, PROTOTYPE)


if __name__ == '__main__':
    unittest.main()
