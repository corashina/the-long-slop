import unittest
from types import SimpleNamespace as NS
from tools.reference.world import Selection, required_record

class WorldTests(unittest.TestCase):
    def test_renderer_export_keeps_two_sided_source_shadows(self):
        from tools.reference.render_lighting import recover_renderer
        file = NS(name='fixture', source_container_path='fixture')
        renderer = NS(assets_file=file, path_id=1, type=NS(name='MeshRenderer'))
        mesh = object()
        data = dict(m_ProbeAnchor={'m_PathID': 0}, m_CastShadows=2,
                    m_ReceiveShadows=1, m_ReflectionProbeUsage=0, m_MotionVectors=1,
                    m_SortingLayerID=0, m_SortingLayer=0, m_SortingOrder=0)
        bounds = {'m_Center': dict(x=0., y=0., z=0.), 'm_Extent': dict(x=1., y=1., z=1.)}
        source = NS(read=lambda r: data if r is renderer else {'m_LocalAABB': bounds})
        result = recover_renderer(source, renderer, mesh)
        self.assertEqual(result['cast_shadows'], 2)

    def test_generation_counts_are_required_and_checked(self):
        from tools.reference.world import generation_counts
        self.assertEqual(generation_counts({'genAtStart':100,'genInOneFrame':1}), {'genAtStart':100,'genInOneFrame':1})
        for road in ({}, {'genAtStart':99,'genInOneFrame':1}, {'genAtStart':100,'genInOneFrame':2}):
            with self.assertRaises((KeyError,ValueError)): generation_counts(road)

    def test_unslotted_part_spawns_without_part_attachment(self):
        from tools.reference.world import part_scope
        slots = [{'source': 's', 'main_types': [1], 'part_types': [2]}]
        self.assertEqual(part_scope([{'maintipus': [9], 'tipus': [2]}], slots, ['wearable']), ([], ['part_attachment', 'wearable']))
        self.assertEqual(part_scope([{'maintipus': [], 'tipus': [5]}], slots, []), ([], ['part_attachment']))
        self.assertEqual(part_scope([{'maintipus': [1], 'tipus': [2]}], slots, []), (['s'], []))
        self.assertEqual(part_scope([], slots, []), ([], []))

    def test_car_allow_list_keeps_lada_and_excludes_other_cars(self):
        from tools.reference.world import vehicle_exclusion
        lada = NS(assets_file=NS(source_container_path='sharedassets1.assets'), path_id=82556)
        bus = NS(assets_file=NS(source_container_path='sharedassets1.assets'), path_id=1)
        self.assertEqual(vehicle_exclusion(lada, {'carscript'}), '')
        self.assertEqual(vehicle_exclusion(bus, {'carscript', 'partscript'}), 'other_vehicle')
        self.assertEqual(vehicle_exclusion(bus, {'utanfutoscript'}), 'other_vehicle')
        self.assertEqual(vehicle_exclusion(bus, {'partscript'}), '')

    def test_sled_values_stop_on_a_missing_value(self):
        from tools.reference.sled import VALUES, LINKS, sled_values
        data = {**{field: 1. for field in VALUES}, **{field: None for field in LINKS}}
        self.assertEqual(sled_values(data)['slide_above_mass'], 1.)
        for field in ('soundMaxPitch', 'cols'):
            with self.assertRaises(ValueError): sled_values({k: v for k, v in data.items() if k != field})
        with self.assertRaises(ValueError): sled_values({**data, 'maxSpeed': None})


class LightCookieTests(unittest.TestCase):
    def test_world_light_cookie_binds_to_lighting_spot_cookie(self):
        from tools.reference.world import bind_light_cookies
        cookie = {'file': 'sharedassets1.assets', 'path_id': 990}
        other = {'file': 'sharedassets1.assets', 'path_id': 735}
        source = NS(lighting_textures={('sharedassets1.assets', 990): {'id': 4, 'source': cookie},
                                       ('sharedassets1.assets', 735): {'id': 1, 'source': other}},
                    lighting_lights=[{'kind': 'directional', 'cookie': 1}, {'kind': 'spot', 'cookie': 4}])
        def light(link, kind=0):
            return {'source': {'file': 'sharedassets1.assets', 'path_id': 123692}, 'class_name': 'Light',
                    'fields': {'m_Cookie': {'kind': 'link', 'value': link} if link else {'kind': 'empty'},
                               'm_Type': {'kind': 'integer', 'value': kind}}}
        world = {'records': [light(cookie), light(cookie), light(None)]}
        bind_light_cookies(source, world)
        self.assertEqual(world['light_cookies'], [{'source': cookie, 'texture': 4}])
        # A point cookie or a cookie without a lighting spot cookie texture has no renderer binding.
        for record in (light(other), light({'file': 'sharedassets1.assets', 'path_id': 1}), light(cookie, 2)):
            with self.assertRaisesRegex(ValueError, 'lighting spot cookie'):
                bind_light_cookies(source, {'records': [record]})

def _reader(path_id, kind='MonoBehaviour', name=None):
    return NS(assets_file=NS(source_container_path='level1'), path_id=path_id, type=NS(name=kind), name=name)

class RopeFake:
    def __init__(self):
        self.home = _reader(70166, name='buildingscript'); self.rope = _reader(76200, name='ropeScript')
        self.go = _reader(1, 'GameObject'); self.root = _reader(2, 'Transform'); self.segment = _reader(3, 'Transform')
        self.target = _reader(4, 'Transform')
        link = lambda r: {'m_FileID': 0, 'm_PathID': r.path_id}
        self.objects = {r.path_id: r for r in (self.home, self.rope, self.go, self.root, self.segment, self.target)}
        self.data = {70166: {'connectToPole': link(self.rope)},
                     76200: {'m_GameObject': link(self.go), 'ropes': [link(self.segment)], 'ropeTargets': [link(self.target)]}}
    def get(self, file, path_id): return self.objects[path_id]
    def read(self, r): return self.data.get(r.path_id, {})
    def resolve(self, owner, pointer): return self.objects.get(pointer['m_PathID'])
    def class_name(self, r): return r.name
    def transform_for(self, game_object): return self.root
    def hierarchy(self, root, activate_root=False): return [{'reader': root}, {'reader': self.segment}]

class RopeTests(unittest.TestCase):
    def test_rope_rule_record_keeps_rope_links(self):
        from tools.reference.world import RULE_CLASSES
        self.assertIn('ropeScript', RULE_CLASSES)
        source = RopeFake()
        reader = NS(record_files={f'level1/{i}.json': {'sha256': 'x'} for i in source.objects})
        links = {l['field']: l['target']['path_id'] for l in Selection(source, reader).record(source.rope)['links']}
        self.assertEqual(links, {'m_GameObject': 1, 'ropes[0]': 3, 'ropeTargets[0]': 4})

    def test_home_cable_maps_rope_nodes(self):
        from tools.reference.world import recover_home_cable
        nodes = [{'id': 433, 'source': {'file': 'level1', 'path_id': 2}}, {'id': 434, 'source': {'file': 'level1', 'path_id': 3}}]
        self.assertEqual(recover_home_cable(RopeFake(), nodes), {'root': 433, 'ropes': [434]})
        with self.assertRaises(ValueError): recover_home_cable(RopeFake(), nodes[:1])

    def test_home_cable_keeps_inactive_rope_geometry(self):
        from tools.reference.world import prepare_home_cable
        source = RopeFake()
        prepare_home_cable(source)
        self.assertEqual(source.home_cable_geometry_nodes, {('level1', 2), ('level1', 3)})
