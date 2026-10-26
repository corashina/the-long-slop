import unittest
from types import SimpleNamespace as NS
from unittest.mock import patch
from tools.reference.source import key, source_id
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

    def test_car_allow_list_keeps_listed_cars_and_excludes_other_cars(self):
        from tools.reference.world import vehicle_exclusion, LADA
        lada = NS(assets_file=NS(source_container_path='sharedassets1.assets'), path_id=82556)
        trailer = NS(assets_file=NS(source_container_path='sharedassets1.assets'), path_id=1)
        cars = {LADA}
        self.assertEqual(vehicle_exclusion(lada, {'carscript'}, cars), '')
        self.assertEqual(vehicle_exclusion(trailer, {'carscript', 'partscript'}, cars), 'other_vehicle')
        self.assertEqual(vehicle_exclusion(trailer, {'utanfutoscript'}, cars), '')
        self.assertEqual(vehicle_exclusion(trailer, {'partscript'}, cars), '')

    def test_bus_pool_items_follow_child_categories(self):
        from tools.reference.world import category_items
        catalog = {'categories': [
            {'index': 0, 'name': 'bus', 'items': [105, 106], 'categories': [1]},
            {'index': 1, 'name': 'child', 'items': [194], 'categories': [0]}]}
        self.assertEqual(category_items(catalog, 'bus'), [105, 106, 194])
        with self.assertRaises(ValueError):
            category_items(catalog, 'utanfuto')

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


class CarAllowListTests(unittest.TestCase):
    """Car items export by name; start-only items stay out of pools; each start car field gives one placement."""
    def fixture(self, missing='', roots_at=None):
        from tools.reference.world import CAR_ITEMS, START_CARS, START_ROOTS, LADA
        objects, data, trees = {}, {}, {}
        def add(file, pid, type_name, fields=None, cls=''):
            reader = NS(assets_file=NS(source_container_path=file), path_id=pid, type=NS(name=type_name), cls=cls)
            objects[file, pid] = reader; data[file, pid] = {'m_Script': {'m_PathID': 1}, **(fields or {})}
            return reader
        def ptr(reader):
            return {'file': reader.assets_file.source_container_path, 'm_PathID': reader.path_id}
        def node(file, pid, classes):
            transform = add(file, pid, 'Transform')
            game_object = add(file, pid + 1, 'GameObject', {'transform': transform})
            components = [add(file, pid + 2 + i, 'MonoBehaviour', cls=c) for i, c in enumerate(classes)]
            return {'reader': transform, 'game_object': game_object, 'components': components}
        def prefab(pid, classes=('carscript',)):
            root = node('sharedassets1.assets', pid, classes)
            trees[key(root['reader'])] = [root]
            return root['reader']
        lada = prefab(LADA[1])
        items = [{'index': 0, 'name': 'bus01', 'source': source_id(prefab(1000))},
                 {'index': 1, 'name': 'car06p', 'source': source_id(prefab(1010))}]
        roots = {}
        for name in CAR_ITEMS:
            roots[name] = prefab(2000 + 10 * len(roots))
            if name != missing:
                items.append({'index': len(items), 'name': name, 'source': source_id(roots[name])})
        catalog = {'items': items, 'categories': [{'index': 0, 'name': 'bus', 'items': [0], 'categories': None}, {'index': 1, 'name': 'bike', 'items': [], 'categories': None}]}
        settings = add('level1', 64898, 'MonoBehaviour')
        cars = {}
        for i, field in enumerate(START_CARS):
            group = node('level1', 3000 + 100 * i, ())
            # A scene GameObject id is its transform id + 1.
            car = node('level1', (roots_at or START_ROOTS).get(field, 3051 + 100 * i) - 1, ('carscript',))
            trees[key(group['reader'])] = [group, car]
            data['level1', 64898][field] = ptr(group['game_object'])
            cars[field] = car['reader']
        source = NS(get=lambda f, p: objects[f, p], read=lambda r: data[key(r)], class_name=lambda r: r.cls,
                    resolve=lambda owner, p: objects[p['file'], p['m_PathID']] if p['m_PathID'] else None,
                    transform_for=lambda go: data[key(go)]['transform'], hierarchy=lambda root, **_: trees[key(root)])
        return source, catalog, settings, lada, roots, cars

    def test_car_items_export_and_stay_out_of_pools(self):
        from tools.reference.world import car_prototypes, pool_exclusion, CAR_ITEMS
        source, catalog, _, lada, roots, _ = self.fixture()
        cars, start_only = car_prototypes(source, catalog)
        bus = source.get('sharedassets1.assets', 1000)
        self.assertEqual(cars, {key(lada), key(bus), *(key(r) for r in roots.values())})
        self.assertNotIn(('sharedassets1.assets', 1010), cars)
        self.assertEqual(start_only, {key(roots[name]) for name, pool in CAR_ITEMS.items() if not pool})
        self.assertEqual(pool_exclusion(roots['car01full'], start_only), 'other_vehicle')
        self.assertEqual(pool_exclusion(bus, start_only), '')
        source, catalog, *_ = self.fixture(missing='car04full')
        with self.assertRaisesRegex(ValueError, 'level1:66024 item car04full'):
            car_prototypes(source, catalog)

    def test_start_placements_use_scene_pose_and_prefab(self):
        from tools.reference.world import car_prototypes, start_placements, START_CARS
        source, catalog, settings, lada, roots, scene = self.fixture()
        cars, _ = car_prototypes(source, catalog)
        with patch('tools.reference.world.world_pose', lambda s, r: {'node': r.path_id}):
            rows = start_placements(source, settings, source.read(settings), catalog, cars)
        self.assertEqual([r['group'] for r in rows], list(START_CARS))
        self.assertTrue(all(r['start_car'] for r in rows))
        by_field = {r['group']: r for r in rows}
        self.assertEqual(by_field['StartCar02']['prototype'], source_id(lada))
        self.assertEqual(by_field['StartCar01']['prototype'], source_id(roots['car01full']))
        self.assertEqual(by_field['StartCar06']['source'], source_id(scene['StartCar06']))
        self.assertEqual(by_field['StartCar06']['pose'], {'node': scene['StartCar06'].path_id})
        # Car09 starts at one of two roots: the start pick needs both.
        self.assertEqual(by_field['StartCar09']['prototype'], source_id(roots['car09full']))
        self.assertEqual(by_field['StartCar09S']['prototype'], source_id(roots['car09fullS']))
        self.assertEqual(by_field['StartCar09S']['source'], {'file': 'level1', 'path_id': 12381})
        source, catalog, settings, *_ = self.fixture(roots_at={'StartCar02': 17687})
        with patch('tools.reference.world.world_pose', lambda s, r: {}), \
                self.assertRaisesRegex(ValueError, 'StartCar09S: the car root is not level1 GO 12382'):
            start_placements(source, settings, source.read(settings), catalog, cars)
