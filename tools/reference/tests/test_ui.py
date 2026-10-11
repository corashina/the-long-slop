import unittest
from tools.reference.ui import convert_anchor, resolve_rect

class UiExportTests(unittest.TestCase):
    def test_bottom_origin_anchor_conversion(self):
        self.assertEqual(convert_anchor((0.5,0.35)),(0.5,0.65))

    def test_parent_anchor_pivot_and_scale_resolve(self):
        parent=(100.,50.,400.,200.)
        rect={'anchor_min':[0.5,0.5],'anchor_max':[0.5,0.5],'position':[20.,-10.],'size':[100.,40.],'pivot':[0.5,0.5],'scale':[2.,1.]}
        self.assertEqual(resolve_rect(rect,parent),(220.,120.,200.,40.))

    def test_installed_menu_slots_do_not_overlap(self):
        import json
        from pathlib import Path
        from tools.reference.ui import export_ui
        from tools.reference.source import LibrarySource
        from tools.reference.library.reader import LibraryReader
        from tools.reference.library.types import LibraryReceipt
        import hashlib
        library=Path('web/assets/library/manifest.json')
        if not library.exists():self.skipTest('accepted library required')
        data=json.loads(library.read_bytes())
        reader=LibraryReader(LibraryReceipt(library,hashlib.sha256(library.read_bytes()).hexdigest(),
            data['catalog_sha256'],data['source_manifest_sha256'],()))
        ui=export_ui(LibrarySource(reader),Path('.cache/ui-test'),reader)
        positions=[ui['layouts'][role]['rect'][1] for role in ['continue','controls','restart','recover']]
        self.assertEqual(len(set(positions)),4)

if __name__=='__main__' : unittest.main()
