import json,os,unittest
from pathlib import Path
from tools.reference.source import Source
from tools.reference.instruments import recover_instruments

RUNTIME=Path(os.environ.get('TLD_REFERENCE_OUTPUT','web/assets/reference'))
STEAM=Path(r'C:\\Program Files (x86)\\Steam\\steamapps\\common\\The Long Drive')
@unittest.skipUnless(STEAM.exists() and (RUNTIME/'manifest.json').exists(),'installed source and export required')
class InstrumentExportTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.source=Source(STEAM)
        cls.root=cls.source.transform_for(cls.source.get('level1',17687))
        cls.manifest=json.loads((RUNTIME/'manifest.json').read_text())
        cls.reports=json.loads((RUNTIME/'geometry-evidence.json').read_text())
    def test_lada_ranges_and_digit_links(self):
        result=recover_instruments(self.source,self.manifest,self.reports,'lada',self.root)
        self.assertEqual([(n['min_value'],n['max_value'],n['min_angle'],n['max_angle']) for n in result['needles']],[(0,160,-64,64),(10,130,-30,30),(0,39,-26,26)])
        self.assertEqual([n['signal'] for n in result['needles']],['speed','temp','fuel'])
        self.assertEqual([n['place'] for n in result['odometer']['digits']],[0,1,2,3,4])
    def test_missing_required_node_names_failure(self):
        m=dict(self.manifest);m['nodes']=[n for n in m['nodes'] if n['source']['path_id']!=34042]
        with self.assertRaisesRegex(ValueError,'34042'):
            recover_instruments(self.source,m,self.reports,'lada',self.root)
