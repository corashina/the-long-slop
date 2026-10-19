from tools.reference.draw_distance import setting
import unittest


class DrawDistanceTests(unittest.TestCase):
    def test_source_float32_interpolation_matches_captured_outputs(self):
        settings = {
            'DDSmin': {
                'drawDistance': 500.0,
                'Cam1FarPlane': 150.0,
                'ObjGenDist': 100.0,
            },
            'DDSmax': {
                'drawDistance': 25000.0,
                'Cam1FarPlane': 450.0,
                'ObjGenDist': 450.0,
            },
        }
        self.assertEqual(setting(settings, 'Cam1FarPlane', 5000.0), 205.10203552246094)
        self.assertEqual(setting(settings, 'ObjGenDist', 5000.0), 164.2857208251953)


if __name__ == '__main__':
    unittest.main()
