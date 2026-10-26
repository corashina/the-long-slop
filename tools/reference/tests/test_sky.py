import unittest
from types import SimpleNamespace as NS
from unittest import mock
from tools.reference import sky

ASSETS = 'sharedassets1.assets'


def reader(file, identity, kind, name=None):
    return NS(assets_file=NS(source_container_path=file), path_id=identity, type=NS(name=kind), name=name)


def pointer(r):
    return {'m_FileID': 0, 'm_PathID': r.path_id if r else 0}


class FakeSource:
    """Main with the spawner, an airplane prefab with a contrail and a sound, a UFO prefab with a spin, and RandomBat."""
    def __init__(self):
        self.objects = {}; self.data = {}; self.children = {}
        self.lighting = {}; self.lighting_textures = {}; self.library = None
        main = self.add('level1', 425, 'GameObject')
        self.spawner = self.add('level1', 426, 'MonoBehaviour', 'AirPlaneSpawnScript')
        plane = self.node(6483, 'AirPlane')
        trail = self.node(6486, 'planetrail', plane)
        self.system = self.component(trail, 6488, 'ParticleSystem')
        sound = self.node(6489, 'Audio', plane)
        self.plane_voice = self.component(sound, 6491, 'AudioSource')
        ufo = self.node(38330, 'Fedo01')
        self.ufo_voice = self.component(ufo, 38340, 'AudioSource')
        self.rot = self.node(38350, 'Rot', ufo)
        fedo = self.component(ufo, 38341, 'MonoBehaviour', 'fedoscript')
        self.data[fedo.path_id] = {'rot': pointer(self.objects[(ASSETS, 38351)]), 'rotaxis': {'x': 0., 'y': 1., 'z': 0.}, 'rotspeed': 50.}
        self.data[self.spawner.path_id] = {'m_GameObject': pointer(main), 'AirPlanePrefabs': [pointer(plane), pointer(ufo)],
            'chances': [.9, .1], 'maxPlaneNum': 3, 'minSpawnTime': 50., 'maxSpawnTime': 500., 'spawnDistance': 5000.,
            'height': 600., 'speed': 20., 'maxAngle': 45., 'destroyDistance': 6000., 'rate': 3.}
        self.prefabs = [plane, ufo]
        mainscript = self.add('level1', 64919, 'MonoBehaviour', 'mainscript')
        bat = self.node(32394, 'RandomBat')
        self.node(32400, 'Wing', bat)
        self.animator = self.component(bat, 32396, 'Animator')
        self.flyer = self.component(bat, 32397, 'MonoBehaviour', 'randomFlyer')
        save = self.component(bat, 32398, 'MonoBehaviour', 'tosaveitemscript')
        controller = self.add(ASSETS, 4511, 'AnimatorController')
        clip = self.add(ASSETS, 3992, 'AnimationClip')
        sound = self.add(ASSETS, 4477, 'AudioClip')
        self.data[mainscript.path_id] = {'randomFlyerPrefab': pointer(bat)}
        self.data[self.animator.path_id] = {'m_Controller': pointer(controller)}
        self.data[controller.path_id] = {'m_AnimationClips': [pointer(clip)]}
        self.data[save.path_id] = {'id': 318}
        self.data[self.flyer.path_id] = {'minTime': 0., 'maxTime': 3., 'minRange': 5., 'maxRange': 20., 'maxRangeFromStart': 20.,
            'minSpeed': 1., 'maxSpeed': 4., 'velocityLerp': 1., 'rotLerp': 2., 'soundClips': [pointer(sound)],
            'soundMinTime': 10., 'soundMaxTime': 60., 'soundMinVolume': .3, 'soundMaxVolume': .6, 'soundMinPitch': .8, 'soundMaxPitch': 1.2}

    def add(self, file, identity, kind, name=None):
        r = reader(file, identity, kind, name); self.objects[(file, identity)] = r; self.data.setdefault(identity, {}); return r

    def node(self, identity, name, parent=None):
        go = self.add(ASSETS, identity, 'GameObject'); transform = self.add(ASSETS, identity + 1, 'Transform')
        self.data[identity] = {'m_Name': name, 'components': [transform], 'transform': transform}
        self.data[transform.path_id] = {'m_GameObject': pointer(go)}
        self.children[transform.path_id] = []
        if parent: self.children[self.transform_for(parent).path_id].append(transform)
        return go

    def component(self, go, identity, kind, name=None):
        c = self.add(ASSETS, identity, kind, name); self.data[go.path_id]['components'].append(c); return c

    def hierarchy(self, root, activate_root=False):
        go = self.objects[(ASSETS, root.path_id - 1)]
        result = [{'reader': root, 'active': True, 'components': self.data[go.path_id]['components'][1:]}]
        for child in self.children[root.path_id]: result += self.hierarchy(child)
        return result

    def get(self, file, identity): return self.objects[(file, identity)]
    def components(self, root):
        go = self.objects[(ASSETS, root.path_id - 1)]
        return go, self.data[go.path_id]['components']
    def read(self, r): return self.data[r.path_id]
    def resolve(self, owner, p): return next((r for r in self.objects.values() if r.path_id == p['m_PathID']), None) if p['m_PathID'] else None
    def transform_for(self, go): return self.data[go.path_id]['transform']
    def class_name(self, r): return r.name


def fake_particle(source, system, role, rate, output, owner=None):
    return {'source': system.path_id, 'role': role, 'rate': rate, 'owner': owner.path_id}


def manifest(source):
    nodes = [{'id': i + 1, 'source': {'file': ASSETS, 'path_id': pid}, 'asset': 1 if pid < 38000 else 2,
        'parent': None if pid in (6484, 38331) else 1 if pid < 38000 else 4} for i, pid in enumerate([6484, 6487, 6490, 38331, 38351])]
    return {'nodes': nodes, 'assets': [{'id': 1, 'usage': 'sky_traffic', 'root': {'file': ASSETS, 'path_id': 6484}},
        {'id': 2, 'usage': 'sky_traffic', 'root': {'file': ASSETS, 'path_id': 38331}}],
        'lighting': {'textures': []}, 'audio': {'voices': [{'id': 1, 'emitter': 1}, {'id': 2, 'emitter': 2}],
        'emitters': [{'id': 1, 'node': 3}, {'id': 2, 'node': 4}], 'clips': [{'id': 1}]},
        'world': {'prototypes': [{'source': {'file': ASSETS, 'path_id': 32395}, 'asset': 7,
            'nodes': [{'source': {'file': ASSETS, 'path_id': 32395}}, {'source': {'file': ASSETS, 'path_id': 32401}}]}]}}


def fake_clips(source, identities, animator, bind):
    """One looping clip with a quaternion track on the wing bone."""
    assert animator is source.animator
    binding = {'type_id': 4, 'attribute': 2, 'dimension': 4, 'is_pptr': False, 'script': None}
    track = {'node': bind(binding, {'file': ASSETS, 'path_id': 32401}), 'attribute': 2, 'path_hash': 7, 'custom_type': 0, 'curve_offset': 0,
        'storage': 'dense', 'interpolation': 'LINEAR', 'times': [0., .5], 'values': [[0., 0., 0., 1.], [0., .5, 0., .866]],
        'in_tangents': [[0.] * 4] * 2, 'out_tangents': [[0.] * 4] * 2, 'coefficients': []}
    return [{'source': identities[0], 'source_tracks': {'bytes': 1, 'file': 'tracks.json', 'sha256': 'a'}, 'name': 'fly',
        'start_time': 0., 'stop_time': .5, 'sample_rate': 60., 'loop': True, 'cycle_offset': 0., 'tracks': [track], 'events': []}]


@mock.patch.object(sky, 'managed_evidence', lambda source, lines: {'assembly_sha256': 'a', 'methods': lines})
@mock.patch.object(sky, 'particle', fake_particle)
@mock.patch.object(sky, 'motion_clips', fake_clips)
class SkyTests(unittest.TestCase):
    def recover(self, source, records=None):
        with mock.patch.object(sky, 'scene_component', lambda s, library, name: source.spawner):
            record = sky.recover_sky(source, records or manifest(source), None)
        record['flyer'] = sky.recover_flyer(source, records or manifest(source))
        # recover_audio replaces the AudioClip identities with clip ids.
        record['flyer']['clips'] = [1]
        return record

    def test_record(self):
        source = FakeSource()
        record = self.recover(source)
        self.assertEqual({k: v for k, v in record.items() if k not in ('prefabs', 'evidence', 'flyer')}, {
            'source': {'file': 'level1', 'path_id': 426}, 'chances': [.9, .1], 'max_planes': 3, 'spawn_time_range': [50., 500.],
            'spawn_distance': 5000., 'height': 600., 'speed': 20., 'max_angle': 45., 'destroy_distance': 6000., 'contrail_rate': 3.})
        plane, ufo = record['prefabs']
        self.assertEqual(plane, {'source': {'file': ASSETS, 'path_id': 6484}, 'asset': 1, 'root': 1,
            'contrail': {'source': 6488, 'role': 'sky_airplane_contrail', 'rate': 3., 'owner': 6484},
            'voices': [{'file': ASSETS, 'path_id': 6491}], 'spin': None})
        self.assertEqual(ufo['contrail'], None)
        self.assertEqual(ufo['voices'], [{'file': ASSETS, 'path_id': 38340}])
        self.assertEqual(ufo['spin'], {'node': 5, 'axis': [-0., -1., 0.], 'degrees_per_second': 50.})
        self.assertEqual(record['evidence']['methods'], sky.SKY_EVIDENCE)
        # recover_audio replaces the AudioSource identities with voice ids.
        plane['voices'] = [1]; ufo['voices'] = [2]
        with mock.patch.object(sky, 'validate_particle'):
            sky.validate_sky(record, manifest(source))
            for change in [lambda r: r['chances'].pop(), lambda r: r.update(max_planes=0), lambda r: r.update(spawn_time_range=[500., 50.]),
                    lambda r: r.update(speed=float('nan')), lambda r: r['prefabs'][0].update(voices=[2]),
                    lambda r: r['flyer'].update(clips=[2]), lambda r: r['flyer'].update(speed_range=[4., 1.]),
                    lambda r: r['flyer']['wing'].update(loop=False), lambda r: r['flyer']['wing']['tracks'][0].update(node=2),
                    lambda r: r['flyer']['wing']['tracks'][0]['values'].pop()]:
                broken = self.recover(source); broken['prefabs'][0]['voices'] = [1]; broken['prefabs'][1]['voices'] = [2]
                change(broken)
                with self.subTest(change=change), self.assertRaises(ValueError):
                    sky.validate_sky(broken, manifest(source))
            records = manifest(source); records['assets'][1]['usage'] = 'sky_prototype'
            with self.assertRaisesRegex(ValueError, 'sky prefab root'):
                sky.validate_sky(record, records)

    def test_flyer(self):
        source = FakeSource()
        flyer = self.recover(source)['flyer']
        self.assertEqual({k: v for k, v in flyer.items() if k != 'wing'}, {
            'source': {'file': ASSETS, 'path_id': 32397}, 'prefab': {'file': ASSETS, 'path_id': 32395}, 'asset': 7, 'item_id': 318,
            'time_range': [0., 3.], 'range': [5., 20.], 'max_range_from_start': 20., 'speed_range': [1., 4.], 'velocity_lerp': 1.,
            'rot_lerp': 2., 'sound_time_range': [10., 60.], 'volume_range': [.3, .6], 'pitch_range': [.8, 1.2], 'clips': [1]})
        self.assertEqual(flyer['wing']['source'], {'file': ASSETS, 'path_id': 3992})
        self.assertEqual(flyer['wing']['tracks'][0]['node'], 1)

    def test_missing_flyer_raises(self):
        source = FakeSource()
        source.data[32394]['components'].remove(source.flyer)
        with self.assertRaisesRegex(ValueError, 'requires one randomFlyer'):
            sky.recover_flyer(source, manifest(source))
        source = FakeSource()
        records = manifest(source); records['world']['prototypes'] = []
        with self.assertRaisesRegex(ValueError, 'not a world prototype'):
            sky.recover_flyer(source, records)

    def test_missing_prefab_raises(self):
        source = FakeSource()
        source.data[source.spawner.path_id]['AirPlanePrefabs'][1] = pointer(None)
        with self.assertRaisesRegex(ValueError, 'Sky prefab links differ'):
            self.recover(source)


if __name__ == '__main__':
    unittest.main()
