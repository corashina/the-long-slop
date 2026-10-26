import unittest
from types import SimpleNamespace as NS
from unittest import mock
from tools.reference import wheel_effects


def reader(identity, kind, name=None):
    return NS(assets_file=NS(source_container_path='level1'), path_id=identity, type=NS(name=kind), name=name)


def pointer(r):
    return {'m_FileID': 0, 'm_PathID': r.path_id if r else 0}


class FakeSource:
    def __init__(self, links):
        self.objects = {}; self.data = {}
        self.wheels = [self.add(10 + i, 'WheelCollider') for i in range(4)]
        self.root = self.add(1, 'Transform')
        self.car = self.add(2, 'MonoBehaviour', 'carscript')
        scripts = []
        # Each script links one wheel collider. The scripts are out of wheel order.
        for i, wheel in enumerate(links):
            base = 100 + 10 * i
            node = self.add(base, 'Transform'); go = self.add(base + 1, 'GameObject')
            self.data[go.path_id] = {'m_Component': [{'component': pointer(node)}]}
            dust = self.add(base + 2, 'ParticleSystem'); grass = self.add(base + 3, 'ParticleSystem')
            self.data[grass.path_id] = {'EmissionModule': {'rateOverTime': {'scalar': 7.}}}
            audio = [self.add(base + 4 + k, 'AudioSource') for k in range(3)]
            script = self.add(base + 9, 'MonoBehaviour', 'talajnezo')
            self.data[script.path_id] = {'m_Enabled': 1, 'm_GameObject': pointer(go), 'W': pointer(self.wheels[wheel]),
                'Car': pointer(self.car), 'ps': pointer(dust), 'grasstrail': pointer(None), 'grasss': [pointer(grass)],
                'rateM': 2. + i, 'LM': {'m_Bits': 5},
                **{name: pointer(a) for name, a in zip(('GroundSource', 'SpringSource', 'SlideSource'), audio)}}
            scripts.append(script)
        self.scripts = scripts

    def add(self, identity, kind, name=None):
        r = reader(identity, kind, name); self.objects[identity] = r; self.data.setdefault(identity, {}); return r

    def hierarchy(self, root, activate_root=False):
        return [{'active': True, 'components': self.scripts}]

    def read(self, r): return self.data[r.path_id]
    def resolve(self, owner, p): return self.objects[p['m_PathID']] if p['m_PathID'] else None
    def transform_for(self, go): return self.resolve(go, self.data[go.path_id]['m_Component'][0]['component'])
    def class_name(self, r): return r.name
    def get(self, file, identity): return self.objects[identity]


def fake_particle(source, system, role, rate, output, owner=None):
    return {'source': system.path_id, 'role': role, 'rate': rate}


@mock.patch.object(wheel_effects, 'relative_pose', lambda source, node, root: node.path_id)
@mock.patch.object(wheel_effects, 'particle', fake_particle)
class WheelEffectsTests(unittest.TestCase):
    def test_wheel_order(self):
        source = FakeSource([3, 1, 2, 0])
        records = wheel_effects.recover_wheel_effects(source, source.car, {}, source.root, source.wheels, 0, None)
        # Script i links wheel [3, 1, 2, 0][i], so wheel order is scripts 3, 1, 2, 0.
        self.assertEqual([r['rate'] for r in records], [5., 3., 4., 2.])
        self.assertEqual(records[0]['pose'], 130)
        self.assertEqual(records[0]['dust'], {'source': 132, 'role': 'wheel_0_0_dust', 'rate': 1.})
        self.assertEqual(records[0]['grass'], [{'source': 133, 'role': 'wheel_0_0_grass_0', 'rate': 7.}])
        self.assertEqual([records[0][f] for f in ('ground_voice', 'spring_voice', 'slide_voice')],
            [{'file': 'level1', 'path_id': 134 + k} for k in range(3)])
        self.assertEqual(records[0]['ray_mask'], 5)

    def test_missing_talajnezo_raises(self):
        source = FakeSource([0, 1, 2])
        with self.assertRaisesRegex(ValueError, 'Wheel needs talajnezo'):
            wheel_effects.recover_wheel_effects(source, source.car, {}, source.root, source.wheels, 0, None)


class SkidTests(unittest.TestCase):
    def test_skid_record(self):
        main = reader(64919, 'MonoBehaviour', 'mainscript')
        systems = [reader(200 + i, 'ParticleSystem') for i in range(7)]
        clips = [reader(300, 'AudioClip'), reader(301, 'AudioClip')]
        data = {'Particles': [pointer(s) for s in systems], 'SSkidRoad': pointer(clips[0]), 'SSkidDirt': pointer(clips[1]),
            'SkidRoadVolume': .5, 'SkidRoadPitchMin': .9, 'SkidRoadPitchMax': 1.1, 'SkidDirtVolume': .4,
            'SkidDirtPitchMin': .8, 'SkidDirtPitchMax': 1.2, 'CarMinSpeedDif': 3., 'CarMaxSpeedDif': 20., 'tireSmokeRate': 15.}
        objects = {r.path_id: r for r in [main, *systems, *clips]}
        source = NS(get=lambda file, identity: objects[identity], read=lambda r: data, class_name=lambda r: r.name,
            resolve=lambda owner, p: objects.get(p['m_PathID']))
        with mock.patch.object(wheel_effects, 'managed_evidence'), mock.patch.object(wheel_effects, 'particle', fake_particle):
            record = wheel_effects.recover_skid(source, None)
        self.assertEqual(record['road_clip'], {'file': 'level1', 'path_id': 300})
        self.assertEqual(record['dirt_clip'], {'file': 'level1', 'path_id': 301})
        self.assertEqual(record['road_pitch'], [.9, 1.1])
        self.assertEqual(record['dirt_pitch'], [.8, 1.2])
        self.assertEqual(record['speed_difference'], [3., 20.])
        self.assertEqual(record['smoke_rate'], 15.)
        self.assertEqual(record['particles'], [{'source': 205, 'role': 'skid_tiresmoke', 'rate': 0.},
            {'source': 206, 'role': 'skid_tiresand', 'rate': 0.}])
        wheel_effects.validate_skid(record)
        with self.assertRaises(ValueError):
            wheel_effects.validate_skid({**record, 'road_pitch': [1.1, .9]})


if __name__ == '__main__':
    unittest.main()
