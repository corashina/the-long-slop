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
        from tools.reference.source import Source
        from tools.reference.ui import export_ui
        steam=Path(r'C:\\Program Files (x86)\\Steam\\steamapps\\common\\The Long Drive')
        if not steam.exists():self.skipTest('installed source required')
        ui=export_ui(Source(steam),Path('.cache/ui-test'))
        positions=[ui['layouts'][role]['rect'][1] for role in ['continue','controls','restart','recover']]
        self.assertEqual(len(set(positions)),4)

    def test_failed_export_removes_acceptance(self):
        import tempfile
        from pathlib import Path
        from unittest.mock import patch
        from tools.reference.export import export_reference
        with tempfile.TemporaryDirectory(dir='.cache') as directory:
            output=Path(directory);acceptance=output/'manifest.json';acceptance.write_text('{}')
            with patch('tools.reference.export.validate_build'), patch('tools.reference.source.Source',side_effect=ValueError('broken source')):
                with self.assertRaisesRegex(ValueError,'broken source'):
                    export_reference(Path('C:/source-test/steam'),output)
            self.assertFalse(acceptance.exists())

if __name__=='__main__' : unittest.main()
