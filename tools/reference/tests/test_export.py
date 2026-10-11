import math
import struct
import tempfile
import unittest
from pathlib import Path

from tools.reference.export import validate_build
from tools.reference.gltf import convert_point, convert_rotation, convert_triangles, transform_point
from tools.reference.source import normalize_arrays
from tools.reference.collision import bake_points


class ReferenceExportTests(unittest.TestCase):
    def test_required_switch_callbacks_reject_unknown_methods_and_skip_audio(self):
        from types import SimpleNamespace
        from tools.reference.mechanisms import recover_turn
        reader=SimpleNamespace(assets_file=SimpleNamespace(name='level1'),path_id=70733,type=SimpleNamespace(name='MonoBehaviour'))
        target=SimpleNamespace(assets_file=reader.assets_file,path_id=77839,type=SimpleNamespace(name='MonoBehaviour'))
        audio=SimpleNamespace(assets_file=reader.assets_file,path_id=17,type=SimpleNamespace(name='AudioSource'))
        source=SimpleNamespace(resolve=lambda r,p: target if p['m_PathID']==77839 else audio,
            class_name=lambda r:'carscript',read=lambda r:{'m_GameObject':{'m_PathID':20}})
        def call(method,identity=77839,mode=1):
            return {'m_Target':{'m_PathID':identity},'m_MethodName':method,'m_Mode':mode,'m_CallState':2}
        def recover(calls):
            fields={'turnable':True,'maxTurnState':0,'currentTurnState':0,
                'turnedEvent':{'m_PersistentCalls':{'m_Calls':calls}}}
            return recover_turn(source,reader,fields,{1:{}},1,lambda r,p:1)
        with self.assertRaisesRegex(ValueError,r'usablescript level1:70733.*turnedEvent.*m_MethodName.*UnknownLights'):
            recover([call('UnknownLights')])
        with self.assertRaisesRegex(ValueError,r'usablescript level1:70733.*turnedEvent.*m_Mode'):
            recover([call('InteractSwitchLights',mode=2)])
        self.assertIsNone(recover([call('Play',identity=17)]))
        mixed=recover([call('Play',identity=17),call('InteractSwitchLights')])
        self.assertEqual(mixed['action'],{'kind':'cycle_headlights'})
        with self.assertRaisesRegex(ValueError,r'turnedEvent.*m_MethodName'):
            recover([call('InteractSwitchLights'),call('UnknownLights')])

    def test_pickup_recenter_and_restrictions_keep_source_values(self):
        from types import SimpleNamespace
        from tools.reference.actions import recover_pickup
        reader = SimpleNamespace(assets_file=SimpleNamespace(name='fixture',source_container_path='fixture'), path_id=19)
        fields = {'cantPickup': 1, 'cantPickupWhileMount': 1, 'cantPickupWhileSitting': 1,
            'noRot': 1, 'moveWithRot': 0, 'useRecenter': 1,
            'recenterPos': {'x': .2, 'y': .4, 'z': .6}, 'useRecenterRot': 1,
            'recenterRot': {'x': 0., 'y': 90., 'z': 0.}, 'recenterDist': -.5}
        result = recover_pickup(SimpleNamespace(read=lambda r: fields), reader)
        self.assertEqual(result['source'], {'file': 'fixture', 'path_id': 19})
        self.assertEqual(result['recenter_position'], (.2, .4, -.6))
        self.assertEqual(result['recenter_distance'], -.5)
        self.assertAlmostEqual(result['recenter_rotation'][1], -math.sqrt(.5))
        self.assertAlmostEqual(result['recenter_rotation'][3], math.sqrt(.5))
        for field in ('cant_pickup', 'cant_pickup_while_mounted', 'cant_pickup_while_seated',
                'no_rotation', 'use_recenter_position', 'use_recenter_rotation'):
            self.assertTrue(result[field])
        del fields['recenterDist']
        with self.assertRaises(KeyError):
            recover_pickup(SimpleNamespace(read=lambda r: fields), reader)

    def test_source_action_masks(self):
        from tools.reference.actions import recover_masks
        masks = recover_masks({'useLayer': {'m_Bits': 12747265},
            'pickupLayer': {'m_Bits': 8388609}, 'attachLayer': {'m_Bits': 8552449},
            'pushLayer': {'m_Bits': 163841}})
        self.assertEqual(masks, {'use_mask': 12747265, 'pickup_mask': 8388609,
            'attach_mask': 8552449, 'push_mask': 163841})

    def test_cap_binding_follows_pickupable_parent_and_startup_layer_link(self):
        from types import SimpleNamespace
        from tools.reference.actions import recover_bindings
        def reader(identity, name):
            return SimpleNamespace(assets_file=SimpleNamespace(name='fixture',source_container_path='fixture'), path_id=identity,
                type=SimpleNamespace(name=name))
        pick, cap, layer, collider = [reader(identity, name) for identity, name in
            [(1, 'MonoBehaviour'), (2, 'MonoBehaviour'), (3, 'MonoBehaviour'), (4, 'BoxCollider')]]
        null = {'m_FileID': 0, 'm_PathID': 0}
        fields = {1: {'cantPickup': False, 'cols': [{'m_PathID': 3}], 'part': null},
            2: {}, 3: {'col': null, 'p': null}}
        readers = {r.path_id: r for r in (pick, cap, layer, collider)}
        classes = {1: 'pickupable', 2: 'tankcapscript', 3: 'layerScript'}
        source = SimpleNamespace(read=lambda r: fields[r.path_id],
            resolve=lambda r, p: readers.get(p['m_PathID']),
            class_name=lambda r: classes[r.path_id], get=lambda file, identity: readers[identity])
        nodes = [{'id': 1, 'parent': None, 'active': True}, {'id': 2, 'parent': 1, 'active': True}]
        components = {1: {'pickupable': [pick]},
            2: {'tankcapscript': [cap], 'layerScript': [layer], 'BoxCollider': [collider]}}
        bindings = recover_bindings(source, nodes, components, {1: 10, 2: 11},
            [{'id': 1, 'node': 2, 'source': {'file': 'fixture', 'path_id': 4}}], [],
            [{'source': {'file': 'fixture', 'path_id': 2}, 'cap': 11}], [])
        self.assertEqual(bindings, [{'collider': 1, 'use_owner': 11, 'pickup_owner': 10,
            'removal_owner': None, 'push_owner': None}])
        fields[1]['cols'] = [{'m_PathID': 2}]
        with self.assertRaisesRegex(ValueError, 'pickupable.cols'):
            recover_bindings(source, nodes, components, {1: 10, 2: 11}, [], [], [], [])
    def test_validator_filters_only_its_provisional_notice(self):
        import warnings
        from unittest.mock import patch
        import pygltflib
        from pygltflib.validator import InvalidMeshPrimitiveMode
        from tools.reference.gltf import validate_gltf
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / 'scene.gltf'
            pygltflib.GLTF2().save_json(str(path))
            with warnings.catch_warnings(record=True) as caught:
                warnings.simplefilter('always')
                validate_gltf(path)
                self.assertEqual(caught, [])
            document = pygltflib.GLTF2(meshes=[pygltflib.Mesh(primitives=[pygltflib.Primitive(mode=99)])])
            document.save_json(str(path))
            with self.assertRaises(InvalidMeshPrimitiveMode):
                validate_gltf(path)
            pygltflib.GLTF2().save_json(str(path))
            def warning(document):
                warnings.warn('Unrelated validation warning')
            with patch('pygltflib.validator.validate', side_effect=warning):
                with warnings.catch_warnings(record=True) as caught:
                    warnings.simplefilter('always')
                    validate_gltf(path)
                    self.assertEqual(str(caught[0].message), 'Unrelated validation warning')

    def test_camera_layers_exclude_source_helper(self):
        from tools.reference.visibility import normal_view_mask, visible_to_camera
        masks=[0x04d28f33,0x10000000,0x00088001,0x00010000]
        self.assertEqual(normal_view_mask(masks),0x14db8f33)
        self.assertFalse(visible_to_camera(18,normal_view_mask(masks)))
        self.assertTrue(visible_to_camera(0,normal_view_mask(masks)))

    def test_terrain_hole_renderer_is_hidden(self):
        from tools.reference.visibility import renderer_visible
        self.assertFalse(renderer_visible(True, {'digholescript2'}))
        self.assertTrue(renderer_visible(True, set()))
        self.assertFalse(renderer_visible(False, set()))
        self.assertFalse(renderer_visible(True, {'TextMeshPro'}))
        self.assertFalse(renderer_visible(True, {'TMP_SubMesh'}))

    def test_startup_terrain_markers_use_source_references(self):
        from types import SimpleNamespace
        from tools.reference.visibility import recover_startup_inactive_nodes
        component = SimpleNamespace(type=SimpleNamespace(name='MonoBehaviour'))
        marker = SimpleNamespace(assets_file=SimpleNamespace(name='fixture.assets',source_container_path='fixture.assets'), path_id=21)
        source = SimpleNamespace(class_name=lambda _: 'terrainHeightAlignToBuildingScript',
            read=lambda _: {'helpPosCoords': [{'m_FileID': 0, 'm_PathID': 21}]},
            resolve=lambda _owner, _pointer: marker)
        self.assertEqual(recover_startup_inactive_nodes(source, [{'components': [component]}]),
                         {('fixture.assets', 21)})
        source.class_name = lambda _: 'ordinaryScript'
        self.assertEqual(recover_startup_inactive_nodes(source, [{'components': [component]}]), set())

    def test_build_and_conversion(self):
        cache = Path('.cache/reference-test')
        cache.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=cache) as directory:
            root = Path(directory) / 'steamapps' / 'common' / 'The Long Drive'
            root.mkdir(parents=True)
            manifest = root.parents[1] / 'appmanifest_1017180.acf'
            manifest.write_text('"AppState" { "appid" "1017180" "buildid" "11140890" }')
            validate_build(root)
            manifest.write_text('"AppState" { "appid" "1017180" "buildid" "11140891" }')
            with self.assertRaisesRegex(ValueError, 'build'):
                validate_build(root)
        self.assertEqual(convert_point((0.0, 0.0, 1.0)), (0.0, 0.0, -1.0))
        self.assertEqual(convert_point((100.0, 0.0, 0.0)), (100.0, 0.0, 0.0))
        rotation = (0.0, math.sin(0.4), 0.0, math.cos(0.4))
        translation = (10.0, 2.0, 5.0)
        for scale in ((100.0, 100.0, 100.0), (-1.0, 2.0, 1.0)):
            visual = (0.25, 0.5, 1.0)
            expected = convert_point(transform_point(visual, translation, rotation, scale))
            actual = transform_point(convert_point(visual), convert_point(translation), convert_rotation(rotation), scale)
            for a, b in zip(actual, expected):
                self.assertAlmostEqual(a, b)
            chain=[{'m_LocalPosition':dict(zip(('x','y','z'),translation)),
                'm_LocalRotation':dict(zip(('x','y','z','w'),rotation)),
                'm_LocalScale':dict(zip(('x','y','z'),scale))}]
            collider=bake_points([visual],chain,{'translation':[0.,0.,0.],'rotation':[0.,0.,0.,1.]})[0]
            for a,b in zip(actual,collider):self.assertAlmostEqual(a,b)

    def test_normals_and_collision_use_one_conversion(self):
        points = [(0., 0., 0.), (1., 0., 0.), (0., 1., 0.)]
        converted = list(map(convert_point, points))
        a, b, c = [converted[i] for i in convert_triangles([(0, 1, 2)])[0]]
        u = [b[i] - a[i] for i in range(3)]
        v = [c[i] - a[i] for i in range(3)]
        normal = (u[1]*v[2]-u[2]*v[1], u[2]*v[0]-u[0]*v[2], u[0]*v[1]-u[1]*v[0])
        self.assertEqual(normal, convert_point((0., 0., 1.)))
        self.assertEqual(converted, [convert_point(p) for p in points])

    def test_generated_string_array_and_scalar_string(self):
        from UnityPy.helpers.TypeTreeNode import TypeTreeNode
        from UnityPy.helpers import TypeTreeHelper
        from UnityPy.streams import EndianBinaryReader
        from unittest.mock import patch
        def node(level, kind, name, children=()):
            return TypeTreeNode(level,kind,name,-1,1,list(children),m_MetaFlag=0)
        def string(level,name):
            return node(level,'string',name,[node(level+1,'Array','Array',[
                node(level+2,'int','size'),node(level+2,'char','data')])])
        names = node(1,'string','names',[node(2,'Array','Array',[
            node(3,'int','size'),string(3,'data')])])
        root = node(0,'Fixture','Base',[names,string(1,'label')])
        raw = struct.pack('<i',2) + struct.pack('<i',3) + b'car\0' + struct.pack('<i',4) + b'home' + struct.pack('<i',4) + b'fuel'
        with self.assertRaises((ValueError,EOFError)):
            TypeTreeHelper.read_typetree(root,EndianBinaryReader(raw,endian='<'),byte_size=len(raw))
        normalize_arrays(root)
        with patch.object(TypeTreeHelper,'read_typetree_boost',None):
            result = TypeTreeHelper.read_typetree(root,EndianBinaryReader(raw,endian='<'),byte_size=len(raw))
        self.assertEqual(result,{'names':['car','home'],'label':'fuel'})
        self.assertEqual(root.m_Children[1].m_Type,'string')


if __name__ == '__main__':
    unittest.main()
