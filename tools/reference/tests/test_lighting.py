import copy
import json
import os
import unittest
from tempfile import TemporaryDirectory
from tools.reference.export import NATIVE_ENGINE_SHA256, native_engine_evidence
from pathlib import Path
from tools.reference.lighting import recover_cycle, recover_grading_temperatures, recover_lighting, validate_lighting_export
from tools.reference.source import Source

class GradingTemperatureTests(unittest.TestCase):
    def source(self,values):
        class Reader:
            def __init__(self,pid,kind):self.path_id=pid;self.type=type('T',(),{'name':kind});self.assets_file=type('F',(),{'source_container_path':'level1'})
        class Fake:
            objects={80125:(Reader(80125,'MonoBehaviour'),{'Cam':2}),2:(Reader(2,'Camera'),{'m_GameObject':3}),
                3:(Reader(3,'GameObject'),{'m_Component':[{'component':4}]}),4:(Reader(4,'MonoBehaviour'),{'TemperatureValues':values})}
            def get(self,file,pid):return self.objects[pid][0]
            def resolve(self,reader,pointer):return self.objects[pointer][0]
            def read(self,reader):return self.objects[reader.path_id][1]
            def class_name(self,reader):return 'postprocesscontrol'
        return Fake()

    def test_main_camera_temperature_values(self):
        self.assertEqual(recover_grading_temperatures(self.source([10.,-40.,4.])),[10.,-40.,4.])
        with self.assertRaisesRegex(ValueError,'TemperatureValues'):recover_grading_temperatures(self.source([1.,2.]))

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
