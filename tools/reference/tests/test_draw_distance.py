from tools.reference.draw_distance import FIELDS, record, setting
import unittest


def settings():
    lo = {name: 1. for name in FIELDS}
    hi = {name: 2. for name in FIELDS}
    lo.update(drawDistance=500., Cam1FarPlane=150., ObjGenDist=100.)
    hi.update(drawDistance=25000., Cam1FarPlane=450., ObjGenDist=450.)
    return {'DDSmin': lo, 'DDSmax': hi, 'S': {'FDrawDistance': 2500.}}


class DrawDistanceTests(unittest.TestCase):
    def test_source_float32_interpolation_matches_captured_outputs(self):
        self.assertEqual(setting(settings(), 'Cam1FarPlane', 5000.0), 205.10203552246094)
        self.assertEqual(setting(settings(), 'ObjGenDist', 5000.0), 164.2857208251953)

    def test_record_exports_both_source_sets_and_default(self):
        source = {'file': 'level0', 'path_id': 16865}
        result = record(source, settings(), {'drawDistance': 500., 'ObjGenDist': 100.})
        self.assertEqual(result['source'], source)
        self.assertEqual(len(result['minimum']), 28)
        self.assertEqual(len(result['maximum']), 28)
        self.assertEqual(result['minimum']['ObjGenDist'], 100.)
        self.assertEqual(result['maximum']['ObjGenDist'], 450.)
        self.assertEqual(result['default'], 2500.)

    def test_record_rejects_captured_mismatch(self):
        with self.assertRaisesRegex(ValueError, 'distances.ObjGenDist'):
            record({}, settings(), {'drawDistance': 500., 'ObjGenDist': 101.})


if __name__ == '__main__':
    unittest.main()
