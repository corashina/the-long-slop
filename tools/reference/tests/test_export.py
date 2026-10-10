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
            return SimpleNamespace(assets_file=SimpleNamespace(name='fixture'), path_id=identity,
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

    def test_camera_layers_exclude_source_helper(self):
        from tools.reference.visibility import normal_view_mask, visible_to_camera
        masks=[0x04d28f33,0x10000000,0x00088001,0x00010000]
        self.assertEqual(normal_view_mask(masks),0x14db8f33)
        self.assertFalse(visible_to_camera(18,normal_view_mask(masks)))
        self.assertTrue(visible_to_camera(0,normal_view_mask(masks)))

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
