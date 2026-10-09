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
