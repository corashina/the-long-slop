import contextlib
import io
import json
import tempfile
import subprocess
import unittest
from pathlib import Path
from types import SimpleNamespace as N
from unittest.mock import patch

from tools.reference import export
from tools.reference.library.types import Catalog, Snapshot

class ExportSafetyTests(unittest.TestCase):
    def test_library_rejects_resolved_steam_outputs_before_prerequisites(self):
        with tempfile.TemporaryDirectory() as directory:
            steam = Path(directory) / 'Steam'
            steam.mkdir()
            for output in (steam / 'generated/reference', steam / 'child/../reference',
                           steam / '../Steam/reference'):
                with self.subTest(output=output), patch.object(export, 'stage_library') as stage, \
                     contextlib.redirect_stderr(io.StringIO()):
                    with self.assertRaises(SystemExit) as error:
                        export.main(['--stage', 'library', '--steam-root', str(steam), '--output', str(output)])
                    self.assertEqual(error.exception.code, 2)
                    stage.assert_not_called()
            self.assertEqual(list(steam.iterdir()), [])

    def test_catalog_publication_failures_keep_prior_inventory_and_no_acceptance(self):
        for failed_name in ('records', 'catalog.json'):
            with self.subTest(step=failed_name), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                (root / 'source-manifest.json').write_text('source')
                (root / 'catalog.json').write_bytes(b'old inventory')
                before = (root / 'catalog.json').stat().st_mtime_ns
                snapshot = Snapshot(root / 'source', (), root / 'source-manifest.json', 'source')
                catalog = Catalog({}, {}, {})
                def build(source, snapshot, pending):
                    (pending / 'manifest.json').write_text('{}')
                    (pending / 'typed.json').write_text('typed')
                    return catalog
                replace = Path.replace
                def locked(path, target):
                    if Path(target) == root / failed_name:
                        raise PermissionError('injected promotion lock')
                    return replace(path, target)
                with patch.object(export, 'SOURCE_WORK_ROOT', root), \
                     patch('tools.reference.library.archive.load_snapshot', return_value=snapshot), \
                     patch('tools.reference.library.archive.verify_snapshot'), \
                     patch('tools.reference.source.Source'), \
                     patch('tools.reference.library.records.build_catalog', side_effect=build), \
                     patch('tools.reference.library.inventory.catalog_data', return_value={}), \
                     patch.object(Path, 'replace', locked):
                    with self.assertRaisesRegex(PermissionError, 'promotion lock'):
                        export.stage_catalog(None)
                self.assertEqual((root / 'catalog.json').read_bytes(), b'old inventory')
                self.assertEqual((root / 'catalog.json').stat().st_mtime_ns, before)
                self.assertFalse((root / 'records').exists())
                self.assertTrue((root / 'work/catalog-records/typed.json').is_file())

    def test_missing_managed_tool_stops_before_source_staging(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(export, 'SOURCE_WORK_ROOT', Path(directory)), \
             patch('tools.reference.library.managed.preflight_managed',
                   side_effect=ValueError('Managed tools unavailable')) as preflight, \
             patch.object(export, 'stage_source') as source:
            with self.assertRaisesRegex(ValueError, 'Managed tools unavailable'):
                export.stage_library(None, Path(directory) / 'assets/reference')
            preflight.assert_called_once()
            source.assert_not_called()

    def test_invalid_accepted_catalog_does_not_rebuild_or_remove_managed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'source-manifest.json').write_text('source')
            records = root / 'records'
            records.mkdir()
            (records / 'manifest.json').write_text('obsolete')
            (records / 'managed').mkdir()
            managed = records / 'managed/analysis.json'
            managed.write_text('accepted analysis')
            snapshot = Snapshot(root / 'source', (), root / 'source-manifest.json', 'source')
            with patch.object(export, 'SOURCE_WORK_ROOT', root), \
                 patch('tools.reference.library.archive.load_snapshot', return_value=snapshot), \
                 patch('tools.reference.library.archive.verify_snapshot'), \
                 patch('tools.reference.source.Source'), \
                 patch('tools.reference.library.records.verify_catalog', side_effect=ValueError('Catalog schema mismatch')), \
                 patch('tools.reference.library.records.build_catalog') as build:
                with self.assertRaisesRegex(ValueError, 'schema mismatch'):
                    export.stage_catalog(None)
                build.assert_not_called()
            self.assertEqual(managed.read_text(), 'accepted analysis')

    def test_fresh_library_space_failure_precedes_archive_and_conversion(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(export, 'SOURCE_WORK_ROOT', Path(directory)), \
             patch('tools.reference.library.managed.preflight_managed'), \
             patch('tools.reference.library.storage.shutil.disk_usage', return_value=N(free=1)), \
             patch.object(export, 'stage_source') as source, \
             patch.object(export, '_category_stages') as categories, \
             contextlib.redirect_stdout(io.StringIO()) as output:
            with self.assertRaisesRegex(OSError, 'Insufficient'):
                export.stage_library(None, Path(directory) / 'assets/reference')
            self.assertIn('decoded categories, assembly staging', output.getvalue())
            source.assert_not_called()
            categories.assert_not_called()
            self.assertEqual(list(Path(directory).iterdir()), [])

if __name__ == '__main__':
    unittest.main()
