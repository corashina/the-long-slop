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
        cls.nodes = {key(n['reader']):i for i,n in enumerate((n for car in cls.cars
            for n in cls.source.hierarchy(cls.source.transform_for(cls.source.resolve(car,cls.source.read(car)['m_GameObject'])))),1)}
        for car in cls.cars:
            raw = cls.source.read(car)
            identities = {key(cls.source.resolve(car, p)) for name in ('WHDriven','WHSteer','WHBrakeOnly') for p in raw[name]}
            cls.wheels.extend(cls.source.get(*identity) for identity in sorted(identities))

    def test_missing_driving_records_fail_without_promotion(self):
        from tools.reference.driving import recover_driving
        source = self.source
        car = self.cars[0]
        original = source.read
        for owner, field in [(car,'gear'),(car,'kerekagyType')]:
            def missing(reader):
                value = original(reader)
                if key(reader)==key(owner):
                    value=deepcopy(value);del value[field]
                return value
            with self.subTest(field=field), patch.object(source,'read',side_effect=missing), patch('tools.reference.export.promote_reference') as promote:
                from tools.reference.export import derive_reference
                with TemporaryDirectory() as directory, patch('tools.reference.library.reader.LibraryReader',return_value=source.library), patch('tools.reference.source.LibrarySource',return_value=source), patch('tools.reference.library.storage.check_space'), patch('tools.reference.export._export_selected',side_effect=lambda *_:recover_driving(source,self.cars,self.wheels,self.nodes)):
                    with self.assertRaises((KeyError,ValueError)):
                        derive_reference(source.library.snapshot,source.library.receipt,Path(directory)/'reference')
                promote.assert_not_called()
        def invalid_hub(reader):
            value=original(reader)
            if key(reader)==key(car):value=deepcopy(value);value['kerekagyType']=999
            return value
        with patch.object(source,'read',side_effect=invalid_hub):
            with self.assertRaises(ValueError):recover_driving(source,self.cars,self.wheels,self.nodes)
        from tools.reference.wheels import wheel_collider_record
        value=deepcopy(original(self.wheels[0]));del value['m_Mass']
        with self.assertRaises(KeyError):wheel_collider_record(value)


        clutch_toggle=source.get('level0',18636)
        def disabled_clutch(reader):
            value=original(reader)
            if key(reader)==key(clutch_toggle):value=deepcopy(value);value['def']=0
            return value
        with patch.object(source,'read',side_effect=disabled_clutch):
            result=recover_driving(source,self.cars,self.wheels,self.nodes)
            self.assertEqual(result['global']['default_assists'],{'automatic_clutch':False,'automatic_gears':False})

        graphic = source.get('level1',77841)
        def unresolved_graphic(reader):
            value=original(reader)
            if key(reader)==key(graphic):
                value=deepcopy(value);value['car']['m_PathID']=0
            return value
        with patch.object(source,'read',side_effect=unresolved_graphic):
            with self.assertRaises(ValueError):recover_driving(source,self.cars,self.wheels,self.nodes)


class LeverSourceTests(unittest.TestCase):
    """carscript.Transmission with a fake source: no library needed."""
    def setUp(self):
        from types import SimpleNamespace
        file = SimpleNamespace(source_container_path='level1')
        def transform(father, x, s=1.):
            return {'m_LocalPosition':{'x':x,'y':0.,'z':0.}, 'm_LocalRotation':{'x':0.,'y':0.,'z':0.,'w':1.},
                'm_LocalScale':{'x':s,'y':s,'z':s}, 'm_Father':{'m_FileID':0,'m_PathID':father}}
        # 1 root, 2 lever parent (scale 2), 3 lever, 4-7 gear poses, 8 outside the car.
        self.data = {1:transform(0,0.), 2:transform(1,1.,2.), 3:transform(2,0.), 4:transform(2,.5), 5:transform(2,.25),
            6:transform(2,-.5), 7:transform(2,-.25), 8:transform(0,9.)}
        readers = {i:SimpleNamespace(assets_file=file, path_id=i) for i in self.data}
        self.source = SimpleNamespace(read=lambda r:self.data[r.path_id],
            resolve=lambda owner,p:readers.get(p['m_PathID']) if p['m_PathID'] else None,
            chain=lambda r:[self.data[i] for i in self.ancestors(r.path_id)])
        self.car_reader = SimpleNamespace(assets_file=file, path_id=100)
        pointer = lambda i:{'m_FileID':0,'m_PathID':i}
        self.car = {'Transmission':pointer(3), 'gears':[{'Path':pointer(5),'Pos':pointer(4)}, {'Path':pointer(7),'Pos':pointer(6)}]}
        self.root = readers[1]; self.pointer = pointer

    def ancestors(self, i):
        while i:yield i;i = self.data[i]['m_Father']['m_PathID']

    def recover(self):
        from tools.reference.driving import recover_lever
        return recover_lever(self.source, self.car_reader, self.car, self.root, {('level1',3):42},
            {'seconds':.25, 'stage1':.33333, 'stage2':.66666})

    def test_lever_poses_follow_gears_in_parent_space(self):
        lever = self.recover()
        self.assertEqual(lever['node'], 42)
        self.assertEqual(len(lever['gears']), len(self.car['gears']))
        self.assertEqual([round(g[k]['translation'][0], 6) for g in lever['gears'] for k in ('path','pos')], [.25, .5, -.25, -.5])
        self.assertEqual(lever['stages'], [.33333, .66666])

    def test_lever_absent_or_outside_car(self):
        self.car['Transmission'] = self.pointer(0)
        self.assertIsNone(self.recover())
        self.car['Transmission'] = self.pointer(3)
        self.car['gears'][1]['Pos'] = self.pointer(8)
        with self.assertRaises(ValueError):self.recover()

    def test_lever_constants_must_exist(self):
        from tools.reference.driving import lever_constants, LEVER_LERPS
        text = 'private float TransmissionLerp = 0.25f;\nprivate float transmissionStage1 = 0.33333f;\n' + '\n'.join(LEVER_LERPS)
        with self.assertRaises(ValueError):lever_constants(text)
        values = lever_constants(text + '\nprivate float transmissionStage2 = 0.66666f;')
        self.assertEqual(values, {'seconds':.25, 'stage1':.33333, 'stage2':.66666})


class MouseDriveSourceTests(unittest.TestCase):
    """Mouse driving sliders and toggle binding with a fake source: no library needed."""
    def test_slider_range_applies_multiplier(self):
        from types import SimpleNamespace
        from tools.reference.driving import slider_range
        sliders = {8:{'defValue':1.,'minValue':.1,'maxValue':4.,'valueMultipler':0.},
            9:{'defValue':2.,'minValue':.5,'maxValue':3.,'valueMultipler':2.}}
        source = SimpleNamespace(resolve=lambda owner,p:p['m_PathID'] or None, read=lambda r:sliders[r])
        settings = {'tabControls':{'sliders':[{'m_PathID':i if i in sliders else 0} for i in range(10)]}}
        self.assertEqual(slider_range(source, None, settings, 8)[0], {'default':1.,'minimum':.1,'maximum':4.})
        self.assertEqual(slider_range(source, None, settings, 9)[0], {'default':4.,'minimum':1.,'maximum':6.})
        sliders[9]['defValue'] = 5.
        with self.assertRaises(ValueError):slider_range(source, None, settings, 9)

    def test_toggle_binding_is_key_u(self):
        from tools.reference.driving import ACTIONS, recover_bindings
        names = ['look'] + list(ACTIONS.values())
        text = 'public enum IN\n{\n' + ',\n'.join(names) + '\n}'
        defaults = '\n'.join(f'defaultInput[{i}].keys[0].Set(KeyCode.{"U" if n=="togglemousedrive" else "R"});'
            for i,n in enumerate(names))
        bindings, evidence = recover_bindings(text, defaults)
        self.assertIn({'action':'mouse_drive','codes':['KeyU']}, bindings)
        self.assertEqual(evidence[-1]['source_input'], 'togglemousedrive')
        with self.assertRaises(ValueError):recover_bindings(text, defaults.replace('KeyCode.U', 'KeyCode.Q').replace(f'defaultInput[{len(names)-1}]', 'x'))
