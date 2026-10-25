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

    def test_volume_rejects_changed_owner_type_or_slots(self):
        volume = {'source': {}, 'storage_type': 3, 'usables': [], 'only_manual': False,
                  'no_physics_lock': False, 'active': True}
        storage.check_volume(volume, 73405, (77859,), (3, (77859,)))
        for args in ((volume, 1, (77859,), (3, (77859,))), (volume, 73405, (), (3, (77859,))),
                     (dict(volume, storage_type=5), 73405, (77859,), (3, (77859,))),
                     (dict(volume, active=False), 73405, (77859,), (3, (77859,)))):
            with self.assertRaises(ValueError):
                storage.check_volume(*args)

    def test_box_shape_converts_center_and_halves_size(self):
        shape = storage.box_shape({'m_Center': dict(x=1., y=2., z=3.), 'm_Size': dict(x=2., y=4., z=6.)})
        self.assertEqual(shape, {'center': [1., 2., -3.], 'half_extents': [1., 2., 3.]})

    def test_flags_keep_only_true_entries(self):
        entries = [(1, {'dontMount': 0, 'dontMountTillPickup': 0}), (2, {'dontMount': 1, 'dontMountTillPickup': 0}),
                   (3, {'dontMount': 0, 'dontMountTillPickup': 1})]
        self.assertEqual(storage.flag_entries(entries), [
            {'item': 2, 'dont_mount': True, 'dont_mount_till_pickup': False},
            {'item': 3, 'dont_mount': False, 'dont_mount_till_pickup': True}])


if __name__ == '__main__':
    unittest.main()
