import copy
import json
import unittest
from pathlib import Path
from tools.reference.survival import recover_survival, validate_survival_export


class SourceRecoveryTests(unittest.TestCase):
    def test_recovers_active_player_and_source_values(self):
        import os, hashlib
        from tempfile import TemporaryDirectory
        from tools.reference.library.types import LibraryReceipt
        from tools.reference.library.reader import LibraryReader
        from tools.reference.source import LibrarySource
        root=os.environ.get('REFERENCE_LIBRARY_ROOT')
        if not root:self.skipTest('Set REFERENCE_LIBRARY_ROOT to the accepted source library')
        path=Path(root)/'manifest.json'; content=path.read_bytes(); data=json.loads(content)
        receipt=LibraryReceipt(path,hashlib.sha256(content).hexdigest(),data['catalog_sha256'],data['source_manifest_sha256'],())
        source=LibrarySource(LibraryReader(receipt))
        with TemporaryDirectory() as tmp:
            r=recover_survival(source,{},Path(tmp))
            self.assertEqual(r['initial'],dict(hp=5.,food=.75,water=1.8000000715255737,bowel=1.))
            self.assertEqual(r['source'],dict(file='level1',path_id=73239))
            self.assertEqual(r['mode'],2)
            self.assertEqual([v['key'] for v in r['inputs']],['Mouse0','P','B','V'])
