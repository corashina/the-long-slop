import math
import unittest

from tools.reference.creatures import ray_direction


class CreatureExportTests(unittest.TestCase):
    def test_ray_direction_uses_unity_euler_order(self):
        # Unity +y yaw turns forward to +x. +x pitch turns it down. The port negates z.
        for euler, expected in (({'x': 0, 'y': 90, 'z': 0}, (1., 0., 0.)),
                                ({'x': 90, 'y': 0, 'z': 0}, (0., -1., 0.)),
                                ({'x': 0, 'y': 0, 'z': 70}, (0., 0., -1.))):
            for a, b in zip(ray_direction(euler), expected):
                self.assertAlmostEqual(a, b)
        self.assertAlmostEqual(math.hypot(*ray_direction({'x': 45, 'y': 30, 'z': 0})), 1.)


if __name__ == '__main__':
    unittest.main()
