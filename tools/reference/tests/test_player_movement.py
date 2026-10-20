"""Critical movement records from the accepted installed library."""
import hashlib
import json
import os
import unittest
from copy import deepcopy
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import patch
from tools.reference.library.reader import LibraryReader
from tools.reference.library.types import LibraryReceipt
from tools.reference.source import LibrarySource, key


class PlayerMovementTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        path = Path(os.environ['G09_LIBRARY_ROOT']) / 'manifest.json'
        raw = path.read_bytes()
        manifest = json.loads(raw)
        cls.source = LibrarySource(LibraryReader(LibraryReceipt(
            path, hashlib.sha256(raw).hexdigest(), manifest['catalog_sha256'],
            manifest['source_manifest_sha256'], ())))

    def recover(self):
        from tools.reference.player_movement import recover_player_movement
        return recover_player_movement(self.source)

    def test_recover_fov_defaults(self):
        value = self.recover()
        self.assertEqual(value['view']['foot_fov_degrees'], 65.0)
        self.assertEqual(value['view']['car_fov_degrees'], 65.0)
        from tools.reference.player_movement import recover_seat_view
        for identity in (77843, 76381, 65840, 69296, 66466):
            seat = recover_seat_view(self.source, self.source.get('level1', identity))
            self.assertEqual(seat['default_yaw_degrees'], 0.0)
            self.assertEqual(seat['pitch_limits_degrees'], [-89.0, 89.0])

    def test_ordered_clearance_spheres(self):
        value = self.recover()
        self.assertEqual(len(value['posture']['clearance']), 10)
        self.assertEqual(value['posture']['sensor_hold_steps'], 3)
        self.assertEqual(value['posture']['clearance'][0]['sphere']['filter'], 4037970431)
        for index, sensor in enumerate(value['posture']['clearance'], 1):
            self.assertAlmostEqual(sensor['posture'], index / 10)
            self.assertEqual(sensor['sphere']['source'], {'file': 'sharedassets1.assets', 'path_id': 124758})
            self.assertAlmostEqual(sensor['sphere']['radius'], 0.15)
            self.assertEqual(sensor['sphere']['membership'], 1 << 14)
            self.assertAlmostEqual(sensor['sphere']['center'][1], -0.685 + 1.335 * index / 10)

    def test_feet_parent_chain(self):
        feet = self.recover()['posture']['feet']
        self.assertEqual(feet['source'], {'file': 'level1', 'path_id': 57258})
        self.assertEqual(feet['filter'], 4172190207)
        self.assertEqual(feet['membership'], 1 << 12)
        self.assertAlmostEqual(feet['radius'], 0.3)
        self.assertAlmostEqual(feet['center'][1], -0.573)
        original = self.source.read
        def shifted(reader):
            value = original(reader)
            if key(reader) == ('level1', 39842):
                value = deepcopy(value)
                value['m_LocalPosition']['y'] += 0.1
            return value
        with patch.object(self.source, 'read', side_effect=shifted):
            self.assertAlmostEqual(self.recover()['posture']['feet']['center'][1], -0.473)

    def test_missing_data_prevents_promotion(self):
        from tools.reference.player_movement import validate_player_movement
        from tools.reference.export import derive_reference
        source = self.source
        original = source.read
        for identity, field in [(('level0', 17625), 'defValue'),
                                (('level1', 80125), 'Feet2'),
                                (('sharedassets1.assets', 124758), 'm_Radius')]:
            def missing(reader):
                value = original(reader)
                if key(reader) == identity:
                    value = deepcopy(value)
                    del value[field]
                return value
            with self.subTest(missing=field), patch.object(source, 'read', side_effect=missing), \
                    patch('tools.reference.export.promote_reference') as promote:
                with TemporaryDirectory() as directory, \
                        patch('tools.reference.library.reader.LibraryReader', return_value=source.library), \
                        patch('tools.reference.source.LibrarySource', return_value=source), \
                        patch('tools.reference.library.storage.check_space'), \
                        patch('tools.reference.export._export_selected', side_effect=lambda *_: self.recover()):
                    with self.assertRaises((KeyError, ValueError)):
                        derive_reference(source.library.snapshot, source.library.receipt, Path(directory) / 'reference')
                promote.assert_not_called()
        value = self.recover()
        mutations = [
            lambda v: v['posture']['clearance'].reverse(),
            lambda v: v['posture']['clearance'][0]['sphere'].update(radius=0.2),
            lambda v: v['posture']['clearance'][0]['sphere'].update(filter=1),
            lambda v: v.pop('view'),
            lambda v: v['view'].pop('head_lerp'),
            lambda v: v['posture'].update(sensor_hold_steps=2),
            lambda v: v['posture'].update(sensor_hold_steps=3.0),
            lambda v: v['posture']['standing_head']['translation'].__setitem__(0, float('nan')),
            lambda v: v['posture']['feet']['center'].__setitem__(0, float('inf')),
            lambda v: v['view'].update(tap_seconds=0.3),
        ]
        for mutate in mutations:
            invalid = deepcopy(value)
            mutate(invalid)
            with self.subTest(mutation=mutate), self.assertRaises((KeyError, ValueError)):
                validate_player_movement(invalid)
        from tools.reference.player_movement import recover_seat_view
        for field, bad in [('lookInsteadOfLean', 1), ('rotLimit', 90.0), ('defYrot', 1.0),
                           ('minGetUpAnlge', 60.0), ('BcantSitAngle', 1)]:
            def wrong_seat(reader):
                value = original(reader)
                if key(reader) == ('level1', 69296):
                    value = deepcopy(value)
                    value[field] = bad
                return value
            with self.subTest(seat_field=field), patch.object(source, 'read', side_effect=wrong_seat):
                with self.assertRaises(ValueError):
                    recover_seat_view(source, source.get('level1', 69296))
