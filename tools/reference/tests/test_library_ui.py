"""Critical canvas coordinate checks."""
import unittest
import tempfile
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch
from tools.reference.library.types import Catalog, ObjectId, ObjectRecord
from tools.reference.library.ui import convert_layout, export_ui


def rect(x=100, y=50):
    return {'m_AnchorMin': {'x': 0, 'y': 0}, 'm_AnchorMax': {'x': 0, 'y': 0},
            'm_Pivot': {'x': 0, 'y': 0}, 'm_AnchoredPosition': {'x': x, 'y': y},
            'm_SizeDelta': {'x': 200, 'y': 100}, 'm_LocalScale': {'x': 1, 'y': 1, 'z': 1}}


class LayoutTests(unittest.TestCase):
    def test_canvas_specific_scaling(self):
        record = rect()
        first = {'canvas': {'m_RenderMode': 0}, 'scaler': {'m_UiScaleMode': 1,
                 'm_ReferenceResolution': {'x': 1920, 'y': 1080}, 'm_ScreenMatchMode': 1},
                 'viewport': [960, 540]}
        second = {'canvas': {'m_RenderMode': 0}, 'scaler': {'m_UiScaleMode': 0, 'm_ScaleFactor': 2},
                  'viewport': [960, 540]}
        a = convert_layout(record, (first,))
        b = convert_layout(record, (second,))
        self.assertEqual(a['browser']['scale_factor'], .5)
        self.assertEqual(b['browser']['scale_factor'], 2)
        self.assertEqual(a['browser']['rect'], [100, 930, 200, 100])
        self.assertEqual(b['browser']['rect'], [100, 120, 200, 100])
        self.assertEqual(record['m_AnchoredPosition']['y'], 50)
        self.assertEqual(convert_layout(record, (first,)), a)

    def test_stretched_nested_rect(self):
        context = {'canvas': {'m_RenderMode': 0}, 'scaler': {'m_UiScaleMode': 0, 'm_ScaleFactor': 1},
                   'viewport': [800, 600]}
        parent = rect(20, 30)
        child = rect(0, 0)
        child['m_AnchorMax'] = {'x': 1, 'y': 1}
        child['m_SizeDelta'] = {'x': -10, 'y': -20}
        child['m_Pivot'] = {'x': .5, 'y': .5}
        result = convert_layout(child, (context, parent))
        self.assertEqual(result['browser']['rect'], [25, 480, 190, 80])

    def test_ordinary_parent_rejects_stale_anchors(self):
        context = {'canvas': {'m_RenderMode': 0}, 'scaler': {'m_UiScaleMode': 0, 'm_ScaleFactor': 1},
                   'viewport': [800, 600]}
        earlier_rectangle = rect(20, 30)
        for translation in ((0, 0), (200, 150)):
            ordinary_parent = {'m_LocalPosition': {'x': translation[0], 'y': translation[1], 'z': 0},
                               'm_LocalScale': {'x': 1, 'y': 1, 'z': 1},
                               'm_LocalRotation': {'x': 0, 'y': 0, 'z': 0, 'w': 1}}
            for stale_y in (.8510000109672546, 10000):
                with self.subTest(translation=translation, stale_y=stale_y):
                    child = rect(0, stale_y)
                    child['m_LocalPosition'] = {'x': 0, 'y': 0, 'z': 0}
                    result = convert_layout(child, (context, earlier_rectangle, ordinary_parent))
                    self.assertEqual(result['browser']['space'], 'unresolved')
                    self.assertIsNone(result['browser']['rect'])
                    self.assertEqual(result['browser']['reason'], 'ordinary_transform_ancestry')
                    self.assertEqual(result['browser']['y_conversion_count'], 0)
                    self.assertEqual(child['m_AnchoredPosition']['y'], stale_y)
                    self.assertEqual(child['m_LocalPosition']['y'], 0)
                    self.assertEqual(ordinary_parent['m_LocalPosition']['y'], translation[1])

    def test_world_canvas_retains_engine_limit(self):
        result = convert_layout(rect(), ({'canvas': {'m_RenderMode': 2}},))
        self.assertEqual(result['browser']['space'], 'world')
        self.assertIsNone(result['browser']['rect'])


    def test_export_world_canvas_and_null_script(self):
        root = ObjectId('level0', 1)
        canvas = ObjectId('level0', 2)
        null = ObjectId('level0', 3)
        ancestor = ObjectId('level0', 4)
        data = {root: {**rect(), 'm_GameObject': {'m_PathID': 10}, 'm_Father': {'m_PathID': 4}},
                ancestor: {'m_Father': {'m_PathID': 0}},
                canvas: {'m_GameObject': {'m_PathID': 10}, 'm_RenderMode': 2}}
        head = SimpleNamespace(m_Script=SimpleNamespace(path_id=0))
        readers = {key: SimpleNamespace(identity=key, parse_monobehaviour_head=lambda: head)
                   for key in (root, canvas, null, ancestor)}
        entries = {key: ObjectRecord(key, kind, 0, 0, 0, '') for key, kind in
                   ((root, 'RectTransform'), (canvas, 'Canvas'), (null, 'MonoBehaviour'), (ancestor, 'Transform'))}
        model = Catalog(entries, {}, {key: () for key in entries})
        with tempfile.TemporaryDirectory() as directory:
            source = SimpleNamespace(root=Path(directory), get=lambda f, p: readers[ObjectId(f, p)],
                                     read_record=lambda reader: data[reader.identity])
            def pointer(source, reader, value):
                return ObjectId('level0', value['m_PathID']) if value['m_PathID'] else None
            with patch('tools.reference.library.ui._pointer', side_effect=pointer):
                exports = export_ui(source, model, Path(directory))
        self.assertEqual(len(exports), 2)
        self.assertEqual(exports[0].details['ancestry'], [{'file': 'level0', 'path_id': 4}])
        self.assertEqual(exports[0].details['browser']['space'], 'world')
        self.assertEqual(exports[0].details['game_object_record'], 'records/level0/10.json')


if __name__ == '__main__':
    unittest.main()
