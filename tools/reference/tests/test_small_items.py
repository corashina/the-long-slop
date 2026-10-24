import unittest

from tools.reference import small_items


class SmallItemTests(unittest.TestCase):
    def test_animation_curve_keeps_unity_keys(self):
        data = {'m_Curve': [{'time': 0., 'value': 0., 'inSlope': 1., 'outSlope': 2., 'inWeight': .3,
                             'outWeight': .4, 'weightedMode': 0}], 'm_PreInfinity': 2, 'm_PostInfinity': 2}
        self.assertEqual(small_items.animation_curve(data), {
            'keys': [{'time': 0., 'value': 0., 'in_tangent': 1., 'out_tangent': 2., 'in_weight': .3,
                      'out_weight': .4, 'weighted_mode': 0}], 'pre_wrap_mode': 2, 'post_wrap_mode': 2})

    def test_curves_reject_unordered_or_infinite_keys(self):
        key = {'value': 0., 'in_tangent': 0., 'out_tangent': 0., 'in_weight': 0., 'out_weight': 0.}
        curve = {'keys': [dict(key, time=0.), dict(key, time=1.)], 'pre_wrap_mode': 2, 'post_wrap_mode': 2}
        small_items._curve(curve)
        for keys in ([dict(key, time=1.), dict(key, time=0.)], [dict(key, time=0., value=float('inf'))], []):
            with self.assertRaises(ValueError):
                small_items._curve(dict(curve, keys=keys))

    def test_record_rejects_extra_fields(self):
        with self.assertRaises(ValueError):
            small_items.validate_small_items({'spirit_level': {}, 'extra': {}}, {'audio': {'clips': []}})


if __name__ == '__main__':
    unittest.main()
