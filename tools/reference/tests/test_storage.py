import unittest

from tools.reference import storage


class StorageTests(unittest.TestCase):
    def test_policy_rejects_changed_default_or_mask(self):
        dropdown = {'useDefInt': 1, 'defInt': 2}
        storage.check_policy(dropdown, True, 8388609)
        for args in ((dropdown, True, 8388608), (dict(dropdown, defInt=1), True, 8388609),
                     (dict(dropdown, useDefInt=0), True, 8388609), (dropdown, False, 8388609)):
            with self.assertRaises(ValueError):
                storage.check_policy(*args)

    def test_box_shape_converts_center_and_halves_size(self):
        shape = storage.box_shape({'m_Center': dict(x=1., y=2., z=3.), 'm_Size': dict(x=2., y=4., z=6.)})
        self.assertEqual(shape, {'center': [1., 2., -3.], 'half_extents': [1., 2., 3.]})

    def test_flags_keep_only_true_entries(self):
        entries = [(1, {'dontMount': 0, 'dontMountTillPickup': 0}), (2, {'dontMount': 1, 'dontMountTillPickup': 0}),
                   (3, {'dontMount': 0, 'dontMountTillPickup': 1})]
        self.assertEqual(storage.flag_entries(entries), [
            {'item': 2, 'dont_mount': True, 'dont_mount_till_pickup': False},
            {'item': 3, 'dont_mount': False, 'dont_mount_till_pickup': True}])

    def test_prototype_flags_use_prototype_key(self):
        entries = [({'file': 'a', 'path_id': 1}, {'dontMount': 0, 'dontMountTillPickup': 1})]
        self.assertEqual(storage.flag_entries(entries, 'prototype'), [
            {'prototype': {'file': 'a', 'path_id': 1}, 'dont_mount': False, 'dont_mount_till_pickup': True}])

    def test_root_box_applies_chain_in_root_frame(self):
        quarter = dict(x=0., y=0.7071067811865476, z=0., w=0.7071067811865476)
        chain = [dict(m_LocalPosition=dict(x=0., y=0., z=1.), m_LocalRotation=dict(x=0., y=0., z=0., w=1.), m_LocalScale=dict(x=2., y=1., z=1.)),
                 dict(m_LocalPosition=dict(x=0., y=1., z=0.), m_LocalRotation=quarter, m_LocalScale=dict(x=1., y=1., z=1.))]
        box = storage.root_box(chain, {'m_Center': dict(x=1., y=0., z=0.), 'm_Size': dict(x=2., y=4., z=6.)})
        # Centre (1,0,0) -> scaled (2,0,1) -> yaw 90: (1,1,-2) in Unity, z flipped for glTF.
        for got, want in zip(box['center'], [1., 1., 2.]):
            self.assertAlmostEqual(got, want)
        self.assertEqual(box['half_extents'], [2., 2., 3.])
        for got, want in zip(box['rotation'], [0., -0.7071067811865476, 0., 0.7071067811865476]):
            self.assertAlmostEqual(got, want)

    def test_prototype_volume_requires_home_and_mechanism_usables(self):
        fields = {'slot': [], 'onlyManual': 0, 'noPhysicsLock': 0}
        home = {'file': 'f', 'path_id': 7}
        usable = {'file': 'f', 'path_id': 9}
        prototypes = [{'source': home, 'nodes': [{'source': {'file': 'f', 'path_id': 1}}, {'source': {'file': 'f', 'path_id': 2}}]}]
        homes = storage.containing(prototypes, {'file': 'f', 'path_id': 2})
        self.assertEqual(homes, [(home, 1)])
        storage.check_prototype_volume('v', fields, True, homes, [usable], {('f', 7, 'f', 9)})
        for args in (('v', fields, True, [], [], set()), ('v', fields, False, homes, [], set()),
                     ('v', dict(fields, noPhysicsLock=1), True, homes, [], set()), ('v', fields, True, homes, [usable], set())):
            with self.assertRaises(ValueError):
                storage.check_prototype_volume(*args)


if __name__ == '__main__':
    unittest.main()
