"""Critical driving source contracts from the accepted library."""
import hashlib
import json
import unittest
from copy import deepcopy
from pathlib import Path
from unittest.mock import patch
from tempfile import TemporaryDirectory
from tools.reference.library.reader import LibraryReader
from tools.reference.library.types import LibraryReceipt
from tools.reference.source import LibrarySource, key

class DrivingSourceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        path = Path('web/assets/library/manifest.json')
        content = path.read_bytes(); manifest = json.loads(content)
        cls.source = LibrarySource(LibraryReader(LibraryReceipt(path, hashlib.sha256(content).hexdigest(),
            manifest['catalog_sha256'], manifest['source_manifest_sha256'], ())))
        cls.cars = [cls.source.get('level1', identity) for identity in (77839, 69517)]
        cls.wheels = []
        for car in cls.cars:
            raw = cls.source.read(car)
            identities = {key(cls.source.resolve(car, p)) for name in ('WHDriven','WHSteer','WHBrakeOnly') for p in raw[name]}
            cls.wheels.extend(cls.source.get(*identity) for identity in sorted(identities))

    def test_missing_driving_records_fail_without_promotion(self):
        from tools.reference.driving import recover_driving, car_driving, hub_record
        source = self.source
        original = source.read
        clutch_toggle = source.get('level0',18636)
        def missing(reader):
            value = original(reader)
            if key(reader)==key(clutch_toggle):
                value=deepcopy(value);del value['def']
            return value
        with patch.object(source,'read',side_effect=missing), patch('tools.reference.export.promote_reference') as promote:
            from tools.reference.export import derive_reference
            with TemporaryDirectory() as directory, patch('tools.reference.library.reader.LibraryReader',return_value=source.library), patch('tools.reference.source.LibrarySource',return_value=source), patch('tools.reference.library.storage.check_space'), patch('tools.reference.export._export_selected',side_effect=lambda *_:recover_driving(source)):
                with self.assertRaises((KeyError,ValueError)):
                    derive_reference(source.library.snapshot,source.library.receipt,Path(directory)/'reference')
            promote.assert_not_called()
        car = original(self.cars[0])
        hubs = original(source.get('level1',64919))['kerekagyProperties']
        with self.assertRaises(ValueError):hub_record(hubs,dict(car,kerekagyType=999))
        with self.assertRaises(ValueError):car_driving(dict(car,gear=len(car['gears'])))
        from tools.reference.wheels import wheel_collider_record
        value=deepcopy(original(self.wheels[0]));del value['m_Mass']
        with self.assertRaises(KeyError):wheel_collider_record(value)

        def disabled_clutch(reader):
            value=original(reader)
            if key(reader)==key(clutch_toggle):value=deepcopy(value);value['def']=0
            return value
        with patch.object(source,'read',side_effect=disabled_clutch):
            result=recover_driving(source)
            self.assertEqual(result['global']['default_assists'],{'automatic_clutch':False,'automatic_gears':False})
