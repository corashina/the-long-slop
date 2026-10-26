import unittest
from types import SimpleNamespace as NS

from tools.reference.instruments import instrument_record, recover_material_switch


def dashboard_fixture():
    records = {}
    data = {}
    def record(path_id, fields, class_name='', type_name='MonoBehaviour'):
        reader = NS(assets_file=NS(name='level1', source_container_path='level1'), path_id=path_id,
                    type=NS(name=type_name), class_name=class_name)
        records[path_id] = reader
        data[path_id] = {'m_Script': {'m_PathID': 1}, **fields}
        return reader
    def ptr(path_id):
        return {'m_PathID': path_id}
    record(21, {}, type_name='Material')
    record(22, {}, type_name='Material')
    car = record(1, dict(electronics=ptr(0), lowFuelValue=5., lowOilValue=1.,
                         ledOilTestRpm=600., ledBatteryTestRpm=800., meters=[ptr(2)]), 'carscript')
    needles = []
    nodes = []
    for index, signal in enumerate(('speed', 'fuel', 'temp')):
        moving = 100 + index
        renderer = 200 + index
        game_object = 300 + index
        record(moving, {}, type_name='Transform')
        record(renderer, {'m_GameObject': ptr(game_object)}, type_name='MeshRenderer')
        record(game_object, {'transform': ptr(renderer + 200)}, type_name='GameObject')
        record(renderer + 200, {}, type_name='Transform')
        needle = record(10 + index, dict(mutato=ptr(moving), R=ptr(renderer),
            OnM=ptr(21), OffM=ptr(22), X=True, Y=False, Z=False, minValue=0., maxValue=100.,
            minAngle=0., maxAngle=180., x=0., y=0., z=0., inputValueOffset=0., inv=False,
            lerpSpeedMultiplier=1., noValueLerp=False, lerpAngle=False), 'mutatoscript')
        needles.append(dict(mutato=ptr(needle.path_id), tipus=signal))
        nodes.extend(dict(id=node, source=dict(file='level1', path_id=node))
                     for node in (moving, renderer + 200))
    record(500, {'m_GameObject': ptr(501)}, type_name='MeshRenderer')
    record(501, {'transform': ptr(502)}, type_name='GameObject')
    record(502, {}, type_name='Transform')
    nodes.append(dict(id=502, source=dict(file='level1', path_id=502)))
    counter = record(30, dict(szamok=[], szamrendszer=10, tizedesjegyek=1.,
                              useMultiplier=False, multiplier=1.), 'forgoszamlap')
    meter = record(2, dict(R=ptr(0), OnM=ptr(21), OffM=ptr(22), mutatok=needles, leds=[],
                           szamlapok=[dict(forgo=ptr(30), value=0., name='kilometerszamlalo')]), 'meterscript')
    components = [car, meter, counter] + [records[10 + index] for index in range(3)]
    source = NS(hierarchy=lambda root: [dict(components=components)],
                read=lambda reader: data[reader.path_id],
                class_name=lambda reader: reader.class_name,
                resolve=lambda owner, pointer: records.get(pointer['m_PathID']),
                transform_for=lambda reader: records[data[reader.path_id]['transform']['m_PathID']])
    manifest = dict(nodes=nodes, assets=[dict(id=7, path='reference/car.gltf')])
    reports = dict(car=dict(material_indices={'level1:21': 3, 'level1:22': 4}))
    return source, manifest, reports, car


def recover_instruments(source, manifest, reports, car):
    nodes = {(n['source']['file'], n['source']['path_id']): n['id'] for n in manifest['nodes']}
    return instrument_record(source, car, nodes, 7, reports['car']['material_indices'])


class InstrumentTests(unittest.TestCase):
    def setUp(self):
        self.owner = object()
        self.renderer = object()
        self.on = NS(assets_file=NS(name='level1', source_container_path='level1'), path_id=21)
        self.off = NS(assets_file=NS(name='level1', source_container_path='level1'), path_id=22)
        self.records = {10: self.renderer, 21: self.on, 22: self.off}
        self.source = NS(resolve=lambda owner, pointer: self.records.get(pointer['m_PathID']))
        self.fields = dict(R={'m_PathID': 10}, OnM={'m_PathID': 21}, OffM={'m_PathID': 22})
        self.indices = {'level1:21': 3, 'level1:22': 4}

    def recover(self):
        def node(owner, pointer, renderer=False):
            self.assertIs(owner, self.owner)
            self.assertEqual(pointer, {'m_PathID': 10})
            self.assertTrue(renderer)
            return 99
        return recover_material_switch(self.source, self.owner, self.fields, node, 7, self.indices)

    def test_dashboard_and_needle_material_links(self):
        source, manifest, reports, car = dashboard_fixture()
        result = recover_instruments(source, manifest, reports, car)
        self.assertEqual(result['backlights'], [{'kind': 'absent'}])
        meter = source.resolve(car, {'m_PathID': 2})
        source.read(meter)['R'] = {'m_PathID': 500}
        result = recover_instruments(source, manifest, reports, car)
        self.assertEqual(result['backlights'][0]['node'], 502)
        self.assertEqual(result['backlights'][0]['on']['gltf_material'], 3)
        self.assertEqual(result['backlights'][0]['off']['gltf_material'], 4)
        for index, needle in enumerate(result['needles']):
            self.assertEqual(needle['node'], 100 + index)
            result = needle['backlight']
            self.assertEqual(result['kind'], 'renderer')
            self.assertEqual(result['node'], 400 + index)
            self.assertNotEqual(result['node'], needle['node'])
            for field, source, index in (('on', 21, 3), ('off', 22, 4)):
                self.assertEqual(result[field], dict(source=dict(file='level1', path_id=source),
                                                     asset=7, gltf_material=index))

    def test_built_in_meters_come_from_carscript_meters(self):
        source, manifest, reports, car = dashboard_fixture()
        meter = source.resolve(car, {'m_PathID': 2})
        gauge = NS(assets_file=meter.assets_file, path_id=3, type=meter.type, class_name='meterscript')
        fields = {**source.read(meter), 'mutatok': source.read(meter)['mutatok'][:1], 'szamlapok': []}
        read, resolve = source.read, source.resolve
        source.read = lambda reader: fields if reader is gauge else read(reader)
        source.resolve = lambda owner, pointer: gauge if pointer['m_PathID'] == 3 else resolve(owner, pointer)
        # A meter outside carscript.meters (a gauge part) is not a built-in meter.
        self.assertEqual(len(recover_instruments(source, manifest, reports, car)['needles']), 3)
        source.read(car)['meters'].append({'m_PathID': 3})
        result = recover_instruments(source, manifest, reports, car)
        self.assertEqual(len(result['needles']), 4)
        self.assertEqual(len(result['backlights']), 2)
        self.assertIsNotNone(result['odometer'])
        fields['szamlapok'] = source.read(meter)['szamlapok']
        with self.assertRaisesRegex(ValueError, 'more than one odometer'):
            recover_instruments(source, manifest, reports, car)

    def test_source_null_and_missing_material(self):
        self.fields['R'] = {'m_PathID': 0}
        self.assertEqual(self.recover(), {'kind': 'absent'})
        self.fields['R'] = {'m_PathID': 10}
        for field in ('OnM', 'OffM'):
            original = self.fields[field]
            self.fields[field] = {'m_PathID': 0}
            with self.assertRaisesRegex(ValueError, 'instrument material'):
                self.recover()
            self.fields[field] = original
        self.indices.clear()
        with self.assertRaisesRegex(ValueError, 'instrument material'):
            self.recover()
        self.fields['R'] = {'m_PathID': 123}
        with self.assertRaisesRegex(ValueError, 'instrument renderer'):
            self.recover()
