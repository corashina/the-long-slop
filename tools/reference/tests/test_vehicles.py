import unittest
from types import SimpleNamespace as NS
from unittest.mock import patch

from tools.reference.source import source_id
from tools.reference.vehicles import balance_record, part_records, vehicle_globals, vehicle_record

FILE = NS(name='sharedassets1.assets', source_container_path='sharedassets1.assets')
LEVEL1 = NS(name='level1', source_container_path='level1')
ENGINE = dict(startChance=.9, starterRpm=300., starterRpmLerp=2., alapRpm=800., maxRpm=6000., shutdownRpm=200.,
    maxNm=120., consumptionM=1., OilConsumptionModifier=1., motorBrakeModifier=1., rpmChangeModifier=1.,
    engineStartTimeModifier=1., engineHeatGainMin=1., engineHeatGainMax=2., engineHeatLoss=1., temp=20.,
    engineHeatLossSpeedModifier=1., maxRpmDifference=100., modifier=1., Oilfluid=3, autoShiftClutchRpm=1000.,
    autoShiftClutchRpmRange=200., minOptimalTemp1=80., maxOptimalTemp1=90., minOptimalTemp2=70., maxOptimalTemp2=100.,
    FuelConsumption={'fluids': [{'type': 2, 'amount': 1.}]}, twostroke=0, oilTolerationMin=.02, oilTolerationMax=.3, noOverHeat=0, kickstart=0, defSmokeRate=5.,
    torqueCurve={'m_Curve': [], 'm_PreInfinity': 2, 'm_PostInfinity': 2})
CURVE = dict(m_ExtremumSlip=.4, m_ExtremumValue=1., m_AsymptoteSlip=.8, m_AsymptoteValue=.5, m_Stiffness=1.)


def car_fixture():
    """Car prototype: four wheels in slots, a driver seat with a second hit seat, an engine part, no radiator."""
    records = {}; data = {}; hierarchy = []
    def ptr(reader):
        return {'m_FileID': 0, 'm_PathID': reader.path_id if reader else 0}
    def record(type_name, fields, cls='', file=FILE):
        reader = NS(assets_file=file, path_id=len(records) + 1, type=NS(name=type_name), cls=cls)
        records[(file.source_container_path, reader.path_id)] = reader
        data[reader.path_id] = {'m_Script': {'m_PathID': 1}, **fields}
        return reader
    def node(parent, active=True):
        transform = record('Transform', {})
        game_object = record('GameObject', {'transform': ptr(transform)})
        hierarchy.append({'reader': transform, 'game_object': game_object, 'components': [],
                          'parent': parent['reader'] if parent else None, 'active': active})
        return hierarchy[-1]
    def component(owner, type_name, fields, cls=''):
        reader = record(type_name, {'m_GameObject': ptr(owner['game_object']), **fields}, cls)
        owner['components'].append(reader)
        return reader
    main = record('MonoBehaviour', dict(kerekagyProperties=[dict(radius=.3, mass=10., slip1=1., slip2=1.)],
        WheelSlip1Min=0., WheelSlip1Max=1., WheelSlip2Min=0., WheelSlip2Max=1., WheelSlip1=1., WheelSlip2=1.), file=LEVEL1)
    records[('level1', 64919)] = main
    root = node(None)
    rigidbody = component(root, 'Rigidbody', {'m_Mass': 900.})
    component(root, 'MonoBehaviour', {'F': {'fluids': [{'type': 2, 'amount': 20.}]}}, 'tankscript')
    component(root, 'MonoBehaviour', {'mass': 800.}, 'massScript')
    car = component(root, 'MonoBehaviour', {}, 'carscript')
    wheels = []
    for index in range(4):
        wheel = component(node(root), 'WheelCollider', dict(m_Radius=.3, m_SuspensionDistance=.2,
            m_Center=dict(x=0., y=0., z=0.), m_SuspensionSpring=dict(spring=1., damper=1., targetPosition=.5),
            m_ForwardFriction=CURVE, m_SidewaysFriction=CURVE, m_Mass=20., m_WheelDampingRate=.25, m_ForceAppPointDistance=0.))
        slot = component(node(root), 'MonoBehaviour', {'T': ptr(None)}, 'partslotscript')
        component(node(root), 'MonoBehaviour', dict(W=ptr(wheel), car=ptr(car), slot=ptr(slot), sideStiffnessModifier=1.), 'wheelgraphicsscript')
        wheels.append(wheel)
    seat_node = node(root)
    seat = component(seat_node, 'MonoBehaviour', dict(mainseat=ptr(None), sitPos=ptr(None), Car=ptr(car), driverSeat0=1,
        driverSeat1=1, rotLimit=179., minGetUpAnlge=65., getOutHeight=1., showSleepText=0), 'seatscript')
    component(node(root), 'MonoBehaviour', dict(mainseat=ptr(seat), sitPos=ptr(None), Car=ptr(car)), 'seatscript')
    engine_node = node(root)
    oil = node(engine_node)
    oil_tank = component(oil, 'MonoBehaviour', {'F': {'fluids': []}}, 'tankscript')
    component(engine_node, 'MonoBehaviour', {**ENGINE, 'T': ptr(oil_tank)}, 'enginescript')
    # Two plates: the second label links the first with sameAsThis.
    layout = dict(m_fontSize=10., m_fontSizeBase=10., m_fontSizeMin=1., m_fontSizeMax=20., m_fontStyle=1, m_fontWeight=400,
        m_HorizontalAlignment=2, m_VerticalAlignment=512, m_enableAutoSizing=0, m_enableKerning=1, m_enableWordWrapping=0,
        m_enableExtraPadding=0, m_isOrthographic=1, m_isRichText=0, m_overflowMode=0, m_useMaxVisibleDescender=1,
        m_characterSpacing=0., m_wordSpacing=0., m_lineSpacing=0., m_paragraphSpacing=0.,
        m_margin=dict(x=0., y=0., z=0., w=0.), m_fontColor=dict(r=1., g=1., b=1., a=1.))
    font = record('MonoBehaviour', {}); material = record('Material', {})
    plates = []
    for same in (None, 0):
        plate = node(root); text = node(plate)
        data[text['reader'].path_id].update(m_SizeDelta=dict(x=1., y=.2), m_Pivot=dict(x=.5, y=.5),
            m_AnchorMin=dict(x=.5, y=.5), m_AnchorMax=dict(x=.5, y=.5), m_LocalPosition=dict(x=0., y=0., z=0.),
            m_LocalRotation=dict(x=0., y=0., z=0., w=1.), m_LocalScale=dict(x=1., y=1., z=1.))
        tmp = component(text, 'MonoBehaviour', {**layout, 'm_fontAsset': ptr(font), 'm_sharedMaterial': ptr(material)}, 'TextMeshPro')
        component(text, 'MeshRenderer', dict(m_SortingLayer=0, m_SortingOrder=3))
        plates.append(component(plate, 'MonoBehaviour', dict(text=ptr(tmp), S='', same=0,
            sameAsThis=ptr(plates[same] if same is not None else None)), 'rendszamscript'))
    component(node(root), 'ReflectionProbe', dict(m_Mode=1, m_RefreshMode=1, m_TimeSlicingMode=1, m_Resolution=64,
        m_HDR=True, m_BoxProjection=False, m_UseOcclusionCulling=False, m_BakedTexture=ptr(None),
        m_CustomBakedTexture=ptr(None), m_Enabled=1, m_BoxSize=dict(x=6., y=2.5, z=6.), m_BoxOffset=dict(x=0., y=0., z=1.),
        m_BlendDistance=1., m_IntensityMultiplier=1., m_Importance=1, m_NearClip=.3, m_FarClip=100.,
        m_ShadowDistance=50., m_CullingMask={'m_Bits': 1}))
    # Only the right hand has a drive frame, as a node under the root.
    hand = node(root)
    data[car.path_id].update(LHDrivePos=ptr(None), RHDrivePos=ptr(hand['reader']), LLDrivePos=ptr(None), RLDrivePos=ptr(None),
        speedBasedKeyboardSens=0, keySteerSensMultiplier=0, Transmission=ptr(None), RB=ptr(rigidbody),
        WHDriven=[ptr(w) for w in wheels[2:]], WHSteer=[ptr(w) for w in wheels[:2]], WHBrakeOnly=[],
        WHHandBrake=[ptr(w) for w in wheels[2:]], electronics=ptr(None), tankslot=ptr(None), balanced=0, carType=1, startingTime=1.,
        bikepedal=ptr(None), noGasWhilePedaling=0,
        gear=0, gears=[dict(ratio=-3., freeRun=0), dict(ratio=0., freeRun=1), dict(ratio=3., freeRun=0)],
        forceAutomaticTransmission=0, BhandBrake=1, kerekagyType=0, differentialRatio=4., steerAngle=35.,
        brakePower=1000., maxHandBrake=2000., autoShiftStopSpeed=1., autoShiftMinRpm=1500., autoShiftMaxRpm=4000.,
        dontIgniteWithThorttleInAutomatic=0, collisionSoundM=1., psSmoke=[], groundSoundM=.5,
        springSoundM=2., springPitchM=3., slideAngle=20., maxSlideVolumeSpeed=10.,
        meters=[], kmhmeters=[], lowFuelValue=5., lowOilValue=1., ledOilTestRpm=600., ledBatteryTestRpm=800., ledOilTestTwoStrokeDefault=1)
    # A bike pedal script: two pedals, one feet holder, rear wheel driven. The car links it only in the pedal test.
    pedal_node = node(root); axis = node(pedal_node); crank = node(axis); foot = node(axis)
    component(pedal_node, 'MonoBehaviour', dict(WHDriven=[ptr(wheels[3])], DrivenWheelSlots=[ptr(slot)], seat=ptr(seat),
        pedals=[ptr(crank['reader'])], pedalsInverse=[ptr(foot['game_object'])], pedalFeetHolder=[ptr(foot['reader'])],
        pedalAxisIndex=[0], Axis=[ptr(axis['reader'])], ratio=2., pedalingSpeedModifier=6., pedalInputDebounce=.5,
        gas_tap=.8, playerUse=1), 'bikepedalscript')
    def transform_for(reader):
        return records[(reader.assets_file.source_container_path, data[reader.path_id]['transform']['m_PathID'])]
    source = NS(read=lambda reader: data[reader.path_id], class_name=lambda reader: reader.cls,
                resolve=lambda owner, pointer: records.get((owner.assets_file.source_container_path, pointer['m_PathID'])),
                get=lambda file, path_id: records[(file, path_id)], transform_for=transform_for,
                hierarchy=lambda root, activate_root=False: hierarchy)
    bodies = [{'source': source_id(rigidbody)}]
    colliders = [{'body': 0, 'enabled': True}, {'body': None, 'enabled': True}, {'body': 0, 'enabled': False}]
    return source, hierarchy, bodies, colliders


class VehicleRecordTests(unittest.TestCase):
    def test_balance_record(self):
        key = dict(time=0., value=1., inSlope=0., outSlope=0., inWeight=0., outWeight=0., weightedMode=0)
        car = dict(balanced=1, balanceLean=1, balanceLeanMinSpeed=1., balanceLeanMaxSpeed=10., balanceLeanMinAngle=2.,
            balanceLeanMaxAngle=20., balanceLeanAngleLerp=3., balanceLeanCurve={'m_Curve': [key], 'm_PreInfinity': 2,
            'm_PostInfinity': 2}, balanceSteerAngleMin=4., balanceSteerAngleMax=5., balanceSteerAngleSpeedMin=6.,
            balanceSteerAngleSpeedMax=7., balanceSteerAngleLerp=8., balanceP=9., balanceI=.1, balanceD=.2,
            balanceSetAngle=0., balanceUseAngleLimit=1, balanceMinAngle=-45., balanceMaxAngle=45., balanceMaxForce=500.)
        self.assertIsNone(balance_record({**car, 'balanced': 0}))
        balance = balance_record(car)
        self.assertIs(balance['lean'], True)
        self.assertIs(balance['use_angle_limit'], True)
        self.assertEqual(balance['lean_curve']['keys'][0]['value'], 1.)
        self.assertEqual((balance['lean_max_angle'], balance['steer_angle_speed_max'], balance['i'], balance['min_angle'],
            balance['max_force']), (20., 7., .1, -45., 500.))
        self.assertEqual(len(balance), 20)

    def test_car_prototype_vehicle_and_engine_part(self):
        source, hierarchy, bodies, colliders = car_fixture()
        with patch('tools.reference.vehicles.recover_exhaust', return_value=[]), \
                patch('tools.reference.vehicles.recover_wheel_effects',
                      lambda source, car_reader, car, root, wheels, index, output: [w.path_id for w in wheels]):
            vehicle = vehicle_record(source, hierarchy, bodies, colliders, 5, {}, None)
        # One talajnezo record per wheel, in wheel record order.
        self.assertEqual(vehicle['wheel_effects'], [w['source']['path_id'] for w in vehicle['wheels']])
        self.assertEqual(vehicle['effect_gains']['slide_angle'], 20.)
        self.assertEqual(vehicle['body'], 0)
        self.assertEqual(vehicle['colliders'], [0])
        self.assertEqual(len(vehicle['wheels']), 4)
        self.assertEqual(sum(w['steer'] for w in vehicle['wheels']), 2)
        self.assertEqual(sum(w['drive'] and w['handbrake'] for w in vehicle['wheels']), 2)
        self.assertTrue(all(w['mount'] == w['slot'] for w in vehicle['wheels']))
        self.assertEqual([s['driver'] for s in vehicle['seats']], [True])
        self.assertEqual(set(vehicle['seats'][0]), {'source', 'driver', 'driving2'})
        frames = vehicle['drive_frames']
        car = next(c for c in hierarchy[0]['components'] if c.cls == 'carscript')
        self.assertEqual(hierarchy[frames['right_hand']]['reader'].path_id, source.read(car)['RHDrivePos']['m_PathID'])
        self.assertEqual({k: v for k, v in frames.items() if k != 'right_hand'}, dict(left_hand=None, left_leg=None, right_leg=None))
        # Dry massScript mass. The runtime adds the 20 L of tank fluid.
        self.assertEqual(vehicle['mass_kg'], 800.)
        self.assertEqual(vehicle['fuel_tank'], 0)
        self.assertIsNone(vehicle['fuel_tank_slot'])
        self.assertIsNone(vehicle['balance'])
        self.assertIsNone(vehicle['pedals'])
        self.assertIs(vehicle['no_gas_while_pedaling'], False)
        self.assertEqual(vehicle['driving']['initial_gear'], 1)
        # A car without carscript.Transmission has no lever.
        self.assertIsNone(vehicle['lever'])
        self.assertEqual(vehicle['key_steer_multiplier'], 0)
        self.assertIsNone(vehicle['key_steer_speed'])
        self.assertIs(vehicle['oil_led_two_stroke'], True)
        self.assertIsNone(vehicle['instruments'])
        self.assertEqual(vehicle['kmh_meters'], [])
        self.assertEqual(vehicle['gauge_limits']['electronics_output'], 1.)
        # Plate labels come from the car prototype, with prototype node ids.
        front, rear = vehicle['plates']
        self.assertIsNone(front['same_as'])
        self.assertEqual(rear['same_as'], front['node'])
        self.assertEqual(hierarchy[rear['text_node']]['parent'], hierarchy[rear['node']]['reader'])
        self.assertEqual(rear['sorting'], [0, 3])
        self.assertEqual(rear['layout']['bounds'], [1., .2])
        self.assertEqual(front['font_source'], rear['font_source'])
        # The reflection probe record is on the car prototype.
        probe = vehicle['probe']
        self.assertEqual(hierarchy[probe['node']]['components'][0].type.name, 'ReflectionProbe')
        self.assertEqual((probe['bounds_size'], probe['near'], probe['mask']), ([6., 2.5, 6.], .3, 1))
        engines, radiators, gauges = part_records(source, hierarchy, 5, {})
        self.assertEqual(len(engines), 1)
        engine = engines[0]
        self.assertEqual(hierarchy[engine['node']]['components'][-1].cls, 'enginescript')
        self.assertEqual(engine['oil_tank'], engine['node'] + 1)
        self.assertEqual(engine['fuel_consumption'], [0, 0, 1., 0, 0, 0])
        # Two-stroke mix tolerances (Car03 engine values).
        self.assertEqual((engine['oil_toleration_min'], engine['oil_toleration_max']), (.02, .3))
        # A missing radiator is accepted.
        self.assertEqual(radiators, [])
        self.assertEqual(gauges, [])

    def test_bike_pedals_link_wheels_slots_seat_and_nodes(self):
        source, hierarchy, bodies, colliders = car_fixture()
        car = next(c for c in hierarchy[0]['components'] if c.cls == 'carscript')
        pedal_reader = next(c for n in hierarchy for c in n['components'] if c.cls == 'bikepedalscript')
        source.read(car).update(bikepedal={'m_FileID': 0, 'm_PathID': pedal_reader.path_id}, noGasWhilePedaling=1)
        with patch('tools.reference.vehicles.recover_exhaust', return_value=[]), \
                patch('tools.reference.vehicles.recover_wheel_effects', return_value=[]):
            vehicle = vehicle_record(source, hierarchy, bodies, colliders, 5, {}, None)
        pedals = vehicle['pedals']
        self.assertIs(vehicle['no_gas_while_pedaling'], True)
        pedal = source.read(pedal_reader)
        wheel = vehicle['wheels'][pedals['driven_wheels'][0]]
        self.assertEqual(wheel['source']['path_id'], pedal['WHDriven'][0]['m_PathID'])
        self.assertEqual(pedals['driven_slots'], [vehicle['wheels'][3]['slot']])
        self.assertEqual(pedals['seat'], 0)
        node = lambda field: hierarchy[pedals[field][0]]['reader'].path_id
        self.assertEqual(node('pedals'), pedal['pedals'][0]['m_PathID'])
        self.assertEqual(node('axes'), pedal['Axis'][0]['m_PathID'])
        # A GameObject link resolves to its node.
        self.assertEqual(pedals['pedals_inverse'], pedals['feet_holders'])
        self.assertEqual((pedals['axis_index'], pedals['ratio'], pedals['speed_modifier'], pedals['input_debounce'],
            pedals['gas_tap'], pedals['player_use']), ([0], 2., 6., .5, .8, True))
        pedal['seat'] = {'m_FileID': 0, 'm_PathID': 0}
        with patch('tools.reference.vehicles.recover_exhaust', return_value=[]), \
                patch('tools.reference.vehicles.recover_wheel_effects', return_value=[]), \
                self.assertRaisesRegex(ValueError, 'seat'):
            vehicle_record(source, hierarchy, bodies, colliders, 5, {}, None)

    def test_collider_bake_runs_only_in_the_editor(self):
        source, hierarchy, bodies, colliders = car_fixture()
        bake = NS(assets_file=FILE, path_id=999, type=NS(name='MonoBehaviour'), cls='colliderscaleapplyscript')
        hierarchy[0]['components'].append(bake)
        read = source.read
        for run in (0, 1):
            source.read = lambda reader, run=run: {'m_Script': {'m_PathID': 1}, 'run': run} if reader is bake else read(reader)
            with patch('tools.reference.vehicles.recover_exhaust', return_value=[]), \
                    patch('tools.reference.vehicles.recover_wheel_effects', return_value=[]):
                if run:
                    with self.assertRaisesRegex(ValueError, 'colliderscaleapplyscript.run'):
                        vehicle_record(source, hierarchy, bodies, colliders, 5, {}, None)
                else:
                    self.assertIsNotNone(vehicle_record(source, hierarchy, bodies, colliders, 5, {}, None))

    def test_prototype_without_carscript_has_no_vehicle(self):
        source, hierarchy, bodies, colliders = car_fixture()
        hierarchy[0]['components'] = [c for c in hierarchy[0]['components'] if c.cls != 'carscript']
        self.assertIsNone(vehicle_record(source, hierarchy, bodies, colliders, 5, {}, None))


    def test_vehicle_globals_come_once_from_mainscript(self):
        main = NS(assets_file=LEVEL1, path_id=64919)
        values = dict(defConsumption=1., defOilConsumption=.1, minCarTemp=-20., maxCarTemp=60., coolantEvaporateRate=.01,
            tempDifferenceModifier=.5, tempChangeMoveModifier=.2, maxCarVelocity=40., outsideTempM=.3, minSteamTemp=110.)
        source = NS(get=lambda file, path_id: main, read=lambda reader: values)
        result = vehicle_globals(source)
        self.assertEqual(result['source'], source_id(main))
        self.assertEqual(result['car_temperature_bounds'], [-20., 60.])
        self.assertEqual(result['min_steam_temperature'], 110.)


if __name__ == '__main__':
    unittest.main()


class WorldPushTests(unittest.TestCase):
    def test_world_push_record_holds_the_car_grip(self):
        from tools.reference.actions import recover_world_pushes
        records = {}; data = {}; hierarchy = []
        def ptr(reader):
            return {'m_FileID': 0, 'm_PathID': reader.path_id if reader else 0}
        def record(type_name, fields, cls=''):
            reader = NS(assets_file=FILE, path_id=len(records) + 1, type=NS(name=type_name), cls=cls)
            records[('sharedassets1.assets', reader.path_id)] = reader
            data[reader.path_id] = fields
            return reader
        def node(parent, position):
            transform = record('Transform', dict(m_LocalPosition=dict(zip('xyz', position)),
                m_LocalRotation=dict(x=0., y=0., z=0., w=1.), m_LocalScale=dict(x=1., y=1., z=1.),
                m_Father=ptr(parent['reader'] if parent else None)))
            game_object = record('GameObject', {'transform': ptr(transform)})
            hierarchy.append({'reader': transform, 'game_object': game_object, 'components': [], 'active': True})
            return hierarchy[-1]
        def component(owner, fields, cls):
            reader = record('MonoBehaviour', {'m_GameObject': ptr(owner['game_object']), **fields}, cls)
            owner['components'].append(reader)
            return reader
        root = node(None, (0., 0., 0.)); handle = node(root, (0., 1., 2.)); hand = node(handle, (0., 0., 0.))
        car = component(root, {}, 'carscript')
        component(root, dict(m_Enabled=1, modifier=2., onlyProjected=1, both=0, pushEvenIfSameParent=0), 'pushablescript')
        grip = component(handle, dict(car=ptr(car), HT=ptr(hand['reader']), inuse=0), 'pushgripable')
        def chain(transform):
            result = []
            while transform:
                result.append(data[transform.path_id])
                transform = records.get(('sharedassets1.assets', result[-1]['m_Father']['m_PathID']))
            return result
        source = NS(read=lambda reader: data[reader.path_id], class_name=lambda reader: reader.cls,
            resolve=lambda owner, pointer: records.get(('sharedassets1.assets', pointer['m_PathID'])),
            transform_for=lambda go: records[('sharedassets1.assets', data[go.path_id]['transform']['m_PathID'])], chain=chain)
        mapping = {('sharedassets1.assets', n['reader'].path_id): i for i, n in enumerate(hierarchy)}
        bodies = [{'node': 0, 'pose': {'translation': [0., 0., 0.], 'rotation': [0., 0., 0., 1.]}}]
        colliders = [{'node': 0}, {'node': 1}]
        [push] = recover_world_pushes(source, hierarchy, mapping, bodies, colliders)
        self.assertEqual((push['node'], push['body'], push['modifier'], push['only_projected'], push['both']), (0, 0, 2., True, False))
        self.assertEqual(push['grip']['source'], source_id(grip))
        self.assertEqual((push['grip']['node'], push['grip']['hand_node'], push['grip']['colliders']), (1, 2, [1]))
        self.assertEqual(push['grip']['local_pose']['translation'], [0., 1., -2.])
        self.assertIs(push['grip']['in_use'], False)
