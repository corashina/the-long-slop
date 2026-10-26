import unittest
from types import SimpleNamespace as NS

from tools.reference import computer


class FakeSource:
    """Two nodes: the TV root with displayscript and a switch with usablescript."""
    def __init__(self, calls):
        file = NS(source_container_path='sharedassets1.assets')
        def reader(path_id, kind):
            return NS(assets_file=file, path_id=path_id, type=NS(name=kind))
        self.objects = {p: reader(p, k) for p, k in ((1, 'Transform'), (2, 'Transform'), (10, 'MonoBehaviour'),
                                                     (20, 'MonoBehaviour'), (30, 'AudioClip'), (3, 'GameObject'))}
        self.classes = {10: 'displayscript', 20: 'usablescript'}
        usable = {'turnable': 1, 'slideTurnable': 0, 'rotateAble': 0, 'slideAble': 0, 'maxTurnState': 0,
                  'turnClips': [{'m_FileID': 0, 'm_PathID': 30}], 'turnVolume': .5, 'm_GameObject': {'m_FileID': 0, 'm_PathID': 3},
                  'turnedEvent': {'m_PersistentCalls': {'m_Calls': calls}}}
        self.data = {20: usable, 3: {'m_Component': [{'component': {'m_FileID': 0, 'm_PathID': 2}}]}}
        self.item_clips = set()

    def get(self, file, path_id):
        return self.objects[path_id]

    def components(self, transform):
        return None, [self.objects[20]] if transform.path_id == 2 else [self.objects[10]]

    def class_name(self, reader):
        return self.classes[reader.path_id]

    def read(self, reader):
        return self.data[reader.path_id]

    def resolve(self, owner, pointer):
        return self.objects.get(pointer['m_PathID'])


def call(target, method='Turned', state=2, mode=1):
    return {'m_CallState': state, 'm_MethodName': method, 'm_Mode': mode, 'm_Target': {'m_FileID': 0, 'm_PathID': target}}


PROTOTYPE = {'nodes': [{'source': {'file': 'sharedassets1.assets', 'path_id': 1}},
                       {'source': {'file': 'sharedassets1.assets', 'path_id': 2}}]}


class ComputerTests(unittest.TestCase):
    def test_display_switch_finds_the_turned_caller(self):
        source = FakeSource([call(10)])
        node = lambda reader: source.objects[2]
        switch = computer.display_switch(source, PROTOTYPE, source.objects[10], node)
        self.assertEqual(switch['source']['path_id'], 20)
        self.assertEqual(switch['clip']['path_id'], 30)
        self.assertEqual(switch['volume'], .5)
        self.assertEqual(source.item_clips, {('sharedassets1.assets', 30)})

    def test_display_switch_rejects_missing_or_other_calls(self):
        for calls in ([], [call(10, state=0)], [call(10, method='Upd')]):
            source = FakeSource(calls)
            with self.assertRaises(ValueError):
                computer.display_switch(source, PROTOTYPE, source.objects[10], lambda r: source.objects[2])

    def test_glyph_bake_follows_character_info_orientation(self):
        # 2x2 texel glyph in the atlas bottom-left: the upper texel row (v 0.25..0.5) is opaque.
        alpha = [[0] * 4, [0] * 4, [255, 255, 0, 0], [0] * 4]
        def rect(flipped):
            return {'advance': 4, 'flipped': flipped, 'uv': {'x': 0, 'y': .5, 'width': .5, 'height': -.5},
                    'vert': {'x': 0, 'y': 4, 'width': 4, 'height': -4}}
        for flipped in (False, True):
            rects = {code: rect(flipped) for code in range(32, 127)}
            g = computer.bake_glyphs(rects, alpha, 1, 4, 4, 1)
            self.assertEqual((g['cell'], g['offset']), ([4, 4], [0, 0]))
            cell = g['coverage'][(ord('a') - 32) * 16:(ord('a') - 31) * 16]
            self.assertEqual(cell, g['coverage'][(ord('A') - 32) * 16:(ord('A') - 31) * 16], 'lowercase uses uppercase')
            rows = [cell[i * 4:i * 4 + 4] for i in range(4)]
            if flipped:
                self.assertTrue(all(r[0] > 128 and r[3] == 0 for r in rows), 'uv.y runs along x')
            else:
                self.assertTrue(rows[3][0] > 128 and rows[0] == [0] * 4, 'uv.y is the glyph bottom')

    def test_record_rejects_extra_fields(self):
        with self.assertRaises(ValueError):
            computer.validate_computer({'computer': {}, 'display': {}, 'extra': {}}, {'audio': {'clips': []}})


if __name__ == '__main__':
    unittest.main()
