import unittest
from types import SimpleNamespace as NS
from unittest.mock import patch

from tools.reference.source import source_id
from tools.reference.vehicles import part_records, vehicle_record

FILE = NS(name='sharedassets1.assets', source_container_path='sharedassets1.assets')
LEVEL1 = NS(name='level1', source_container_path='level1')
ENGINE = dict(startChance=.9, starterRpm=300., starterRpmLerp=2., alapRpm=800., maxRpm=6000., shutdownRpm=200.,
    maxNm=120., consumptionM=1., OilConsumptionModifier=1., motorBrakeModifier=1., rpmChangeModifier=1.,
    engineStartTimeModifier=1., engineHeatGainMin=1., engineHeatGainMax=2., engineHeatLoss=1., temp=20.,
    engineHeatLossSpeedModifier=1., maxRpmDifference=100., modifier=1., Oilfluid=3, autoShiftClutchRpm=1000.,
    autoShiftClutchRpmRange=200., minOptimalTemp1=80., maxOptimalTemp1=90., minOptimalTemp2=70., maxOptimalTemp2=100.,
    FuelConsumption={'fluids': [{'type': 2, 'amount': 1.}]}, twostroke=0, noOverHeat=0, kickstart=0, defSmokeRate=5.,
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
    data[car.path_id].update(speedBasedKeyboardSens=0, keySteerSensMultiplier=0, RB=ptr(rigidbody),
        WHDriven=[ptr(w) for w in wheels[2:]], WHSteer=[ptr(w) for w in wheels[:2]], WHBrakeOnly=[],
        WHHandBrake=[ptr(w) for w in wheels[2:]], electronics=ptr(None), carType=1, startingTime=1.,
        gear=0, gears=[dict(ratio=-3., freeRun=0), dict(ratio=0., freeRun=1), dict(ratio=3., freeRun=0)],
        forceAutomaticTransmission=0, BhandBrake=1, kerekagyType=0, differentialRatio=4., steerAngle=35.,
        brakePower=1000., maxHandBrake=2000., autoShiftStopSpeed=1., autoShiftMinRpm=1500., autoShiftMaxRpm=4000.,
        dontIgniteWithThorttleInAutomatic=0, collisionSoundM=1., psSmoke=[])
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
    def test_car_prototype_vehicle_and_engine_part(self):
        source, hierarchy, bodies, colliders = car_fixture()
        view = dict(default_pitch_degrees=0., pitch_limits_degrees=[-89., 89.])
        with patch('tools.reference.vehicles.seat_view', return_value=view), \
                patch('tools.reference.vehicles.recover_exhaust', return_value=[]):
            vehicle = vehicle_record(source, hierarchy, bodies, colliders, 5, {}, None)
        self.assertEqual(vehicle['body'], 0)
        self.assertEqual(vehicle['colliders'], [0])
        self.assertEqual(len(vehicle['wheels']), 4)
        self.assertEqual(sum(w['steer'] for w in vehicle['wheels']), 2)
        self.assertEqual(sum(w['drive'] and w['handbrake'] for w in vehicle['wheels']), 2)
        self.assertTrue(all(w['mount'] == w['slot'] for w in vehicle['wheels']))
        self.assertEqual([s['driver'] for s in vehicle['seats']], [True])
        self.assertEqual(len(vehicle['seats'][0]['hit_nodes']), 2)
        self.assertEqual(vehicle['mass_kg'], 820.)
        self.assertEqual(vehicle['fuel_tank'], 0)
        self.assertEqual(vehicle['driving']['initial_gear'], 1)
        self.assertIsNone(vehicle['instruments'])
        engines, radiators = part_records(source, hierarchy)
        self.assertEqual(len(engines), 1)
        engine = engines[0]
        self.assertEqual(hierarchy[engine['node']]['components'][-1].cls, 'enginescript')
        self.assertEqual(engine['oil_tank'], engine['node'] + 1)
        self.assertEqual(engine['fuel_consumption'], [0, 0, 1., 0, 0, 0])
        # A missing radiator is accepted.
        self.assertEqual(radiators, [])

    def test_prototype_without_carscript_has_no_vehicle(self):
        source, hierarchy, bodies, colliders = car_fixture()
        hierarchy[0]['components'] = [c for c in hierarchy[0]['components'] if c.cls != 'carscript']
        self.assertIsNone(vehicle_record(source, hierarchy, bodies, colliders, 5, {}, None))


if __name__ == '__main__':
    unittest.main()
