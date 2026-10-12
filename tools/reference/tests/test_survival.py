import copy
import json
import unittest
from pathlib import Path
from tools.reference.survival import recover_survival, validate_survival_export

class SurvivalTests(unittest.TestCase):
    def setUp(self):
        self.manifest = json.loads(Path('tests/common/reference.json').read_text())

    def test_recovers_active_player_and_source_values(self):
        record = self.manifest['survival']
        self.assertEqual(record['source'], {'file':'level1','path_id':73239})
        self.assertEqual(record['initial']['hp'], 5)
        self.assertAlmostEqual(record['initial']['water'], 1.8, places=6)
        self.assertEqual(record['maximum'], dict(hp=5,food=1,water=3,bowel=2))
        self.assertEqual(record['initial_bladder'], [0,0,.25,.5,0,0])
        self.assertEqual([(f['kind'],f['bites']) for f in record['foods']], [('sausage',6),('turd',3)])
        validate_survival_export(record, self.manifest)

    def test_rejects_missing_links_and_nonfinite_rates(self):
        for field,value in [('starter_food',999999),('food_loss',float('nan'))]:
            record=copy.deepcopy(self.manifest['survival']); record[field]=value
            with self.assertRaises(ValueError): validate_survival_export(record,self.manifest)
        record=copy.deepcopy(self.manifest['survival']);record['foods'][0]['index']=6
        with self.assertRaises(ValueError): validate_survival_export(record,self.manifest)
        record=copy.deepcopy(self.manifest['survival']);record['foods'][1]['kind']='sausage'
        with self.assertRaises(ValueError): validate_survival_export(record,self.manifest)


class SourceRecoveryTests(unittest.TestCase):
    def test_recovers_active_player_and_source_values(self):
        import os, hashlib
        from tempfile import TemporaryDirectory
        from tools.reference.library.types import LibraryReceipt
        from tools.reference.library.reader import LibraryReader
        from tools.reference.source import LibrarySource
        from tools.reference.weather import recover_weather
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
            self.assertEqual(len(r['audio']),7)
            source.lighting_textures={};source.lighting={}
            w=recover_weather(source,Path(tmp))
            self.assertEqual([p['capacity'] for p in w['particles']],[4000,4000,10000,500])
            self.assertEqual(w['particles'][0]['curves']['InitialModule.startLifetime']['scalar'],20.)
            self.assertEqual(w['particles'][0]['scalars']['TriggerModule.inside'],1)
            self.assertEqual(len(w['particles'][0]['curves']['SizeModule.curve']['max_curve']['keys']),3)
