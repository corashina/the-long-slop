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

    def test_driving_source_links_and_defaults(self):
        from tools.reference.driving import recover_driving
        result = recover_driving(self.source, self.cars, self.wheels)
        driving = result['global']
        self.assertEqual(driving['pedal_rate'], 5.0)
        self.assertEqual(driving['min_motor_torque_nm'], 0.0)
        self.assertEqual(driving['default_assists'], {'automatic_clutch':True, 'automatic_gears':True})
        self.assertFalse(driving['force_manual'])
        self.assertEqual(result['keyboard_steering_rate'], 1.25)
        self.assertEqual([r['source']['path_id'] for r in result['vehicles']], [77839, 69517])
        self.assertEqual(len(result['slots']), 8)
        bindings = {b['action']:b['codes'] for b in driving['bindings']}
        self.assertEqual(bindings['clutch'], ['ControlLeft','Numpad0','ControlRight'])
        self.assertEqual(bindings['reverse'], ['KeyR','NumpadAdd'])
        self.assertEqual(bindings['neutral'], ['Digit0','NumpadSubtract','KeyN'])
        self.assertEqual(bindings['shift_up'], ['ArrowUp'])
        self.assertEqual(bindings['shift_down'], ['ArrowDown'])
        for number in range(1,7):self.assertEqual(bindings[f'gear{number}'], [f'Digit{number}',f'Numpad{number}'])
        for car, record in zip(self.cars, result['vehicles']):
            raw = self.source.read(car)
            self.assertEqual(record['driving'], {'initial_gear':raw['gear']+1, 'force_automatic':bool(raw['forceAutomaticTransmission']), 'play_shift_sounds_in_automatic':False})
            self.assertEqual(sum(slot['vehicle_source']==record['source'] for slot in result['slots']),4)
        for wheel, record in zip(self.wheels, result['slots']):
            raw = self.source.read(wheel)
            for output, field in [('wheel_mass_kg','m_Mass'),('rotational_damping','m_WheelDampingRate'),('force_application_distance_m','m_ForceAppPointDistance')]:
                self.assertEqual(record[output],raw[field])
            for output, field in [('forward_friction','m_ForwardFriction'),('sideways_friction','m_SidewaysFriction')]:
                self.assertEqual(record[output], {name:raw[field][source] for name,source in [('extremum_slip','m_ExtremumSlip'),('extremum_value','m_ExtremumValue'),('asymptote_slip','m_AsymptoteSlip'),('asymptote_value','m_AsymptoteValue'),('stiffness','m_Stiffness')]})
            hub = self.source.read(self.source.get('level1',64919))['kerekagyProperties'][0]
            self.assertEqual(record['hub'],dict(radius_m=hub['radius'],mass_kg=hub['mass'],forward_stiffness=hub['slip1'],sideways_stiffness=hub['slip2']))
            self.assertEqual(record['sideways_stiffness_modifier'],0.0)
        evidence = result['evidence']
        self.assertGreater(next(m['first_line'] for m in evidence['methods'] if m['declaration']=='private void LoadSettings()'),55000)
        self.assertEqual(evidence['assembly_sha256'],'9919a9eeddcce23bb4ff86e890437f32b4a3ef3c10af2efe3b8d889d894dd62e')
        self.assertEqual(evidence['assist_toggles'][0]['child_sources'],[evidence['assist_toggles'][1]['source']])

    def test_missing_driving_records_fail_without_promotion(self):
        from tools.reference.driving import recover_driving
        source = self.source
        car = self.cars[0]
        original = source.read
        for owner, field in [(car,'gear'),(car,'kerekagyType'),(self.wheels[0],'m_Mass')]:
            def missing(reader):
                value = original(reader)
                if key(reader)==key(owner):
                    value=deepcopy(value);del value[field]
                return value
            with self.subTest(field=field), patch.object(source,'read',side_effect=missing), patch('tools.reference.export.promote_reference') as promote:
                from tools.reference.export import derive_reference
                with TemporaryDirectory() as directory, patch('tools.reference.library.reader.LibraryReader',return_value=source.library), patch('tools.reference.source.LibrarySource',return_value=source), patch('tools.reference.library.storage.check_space'), patch('tools.reference.export._export_selected',side_effect=lambda *_:recover_driving(source,self.cars,self.wheels)):
                    with self.assertRaises((KeyError,ValueError)):
                        derive_reference(source.library.snapshot,source.library.receipt,Path(directory)/'reference')
                promote.assert_not_called()
        def invalid_hub(reader):
            value=original(reader)
            if key(reader)==key(car):value=deepcopy(value);value['kerekagyType']=999
            return value
        with patch.object(source,'read',side_effect=invalid_hub):
            with self.assertRaises(ValueError):recover_driving(source,self.cars,self.wheels)


        clutch_toggle=source.get('level0',18636)
        def disabled_clutch(reader):
            value=original(reader)
            if key(reader)==key(clutch_toggle):value=deepcopy(value);value['def']=0
            return value
        with patch.object(source,'read',side_effect=disabled_clutch):
            result=recover_driving(source,self.cars,self.wheels)
            self.assertEqual(result['global']['default_assists'],{'automatic_clutch':False,'automatic_gears':False})

        graphic = source.get('level1',77841)
        def unresolved_graphic(reader):
            value=original(reader)
            if key(reader)==key(graphic):
                value=deepcopy(value);value['car']['m_PathID']=0
            return value
        with patch.object(source,'read',side_effect=unresolved_graphic):
            with self.assertRaises(ValueError):recover_driving(source,self.cars,self.wheels)


class SuspensionContractTests(unittest.TestCase):
    def test_exporter_uses_game_suspension_contract_for_current_cars(self):
        from tools.reference.export import validate_runtime_manifest
        content = Path('web/assets/reference/manifest.json').read_bytes()
        validate_runtime_manifest(content)
        manifest = json.loads(content)
        self.assertEqual(len(manifest['vehicles']), 2)
        for vehicle in manifest['vehicles']:
            invalid = deepcopy(manifest)
            invalid['wheel_slots'][vehicle['wheel_slots'][0]]['target_position'] = -0.01
            with self.assertRaisesRegex(ValueError, 'wheel parameters'):
                validate_runtime_manifest(json.dumps(invalid).encode())
