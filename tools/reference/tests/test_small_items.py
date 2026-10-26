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

    def test_washes_need_checked_cobwebs(self):
        clip = {'file': 'sharedassets1.assets', 'path_id': 4349}
        effect = {'source': {'file': 'sharedassets1.assets', 'path_id': 45340}, 'particle': {}, 'clip': clip, 'volume': 1.}
        def wash(i):
            return {'source': {'file': 'level1', 'path_id': i}, 'prototype': {'file': 'sharedassets1.assets', 'path_id': 1},
                    'node': {'file': 'sharedassets1.assets', 'path_id': i}, 'colliders': [{'file': 'sharedassets1.assets', 'path_id': i}],
                    'permission': 2, 'never_repair': False, 'destroy_on_repair': True, 'objects': [],
                    'chance_to_start_disabled': .3, 'particle_on_destroy': effect['source']}
        washes = [wash(i) for i in range(small_items.COBWEBS)]
        clips = {('sharedassets1.assets', 4349)}
        small_items._washes(washes, [effect], clips)
        for change in ({'never_repair': True}, {'destroy_on_repair': False}, {'chance_to_start_disabled': 1.}, {'colliders': []}):
            with self.assertRaises(ValueError):
                small_items._washes([dict(washes[0], **change)] + washes[1:], [effect], clips)
        with self.assertRaises(ValueError):
            small_items._washes(washes[1:], [effect], clips)


if __name__ == '__main__':
    unittest.main()
