import copy
import json
import os
import unittest
from tempfile import TemporaryDirectory
from tools.reference.export import NATIVE_ENGINE_SHA256, native_engine_evidence
from pathlib import Path
from tools.reference.lighting import recover_cycle, recover_lighting, validate_lighting_export
from tools.reference.source import Source

@unittest.skipUnless(Path(r'C:/Program Files (x86)/Steam/steamapps/common/The Long Drive/TheLongDrive_Data/level1').is_file(),'Requires installed Steam source')
class LightingTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.source=Source(Path(r'C:/Program Files (x86)/Steam/steamapps/common/The Long Drive'))
        cls.output=Path('.cache/lighting-test');cls.output.mkdir(parents=True,exist_ok=True)
        cls.runtime_output=Path(os.environ.get('TLD_REFERENCE_OUTPUT','web/assets/reference'))

    def test_native_engine_evidence_rejects_an_unchecked_dll(self):
        evidence=native_engine_evidence(Path(r'C:/Program Files (x86)/Steam/steamapps/common/The Long Drive'))
        self.assertEqual(evidence['sha256'],NATIVE_ENGINE_SHA256)
        with TemporaryDirectory() as directory:
            root=Path(directory);(root/'UnityPlayer.dll').write_bytes(b'unchecked engine')
            with self.assertRaisesRegex(ValueError,'checked native cloud noise source'):native_engine_evidence(root)

    def recover(self):
        s=self.source
        roots=[s.get('level1',21400),s.transform_for(s.get('level1',17687))]
        s.lighting_cars=[roots[1]]
        return recover_lighting(s,roots,self.output)
