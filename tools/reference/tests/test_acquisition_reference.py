import copy
import hashlib
import json
import os
import unittest
from pathlib import Path
from tools.reference.acquisition import recover_acquisition, validate_acquisition, _block, _database
from tools.reference.library.reader import LibraryReader
from tools.reference.library.types import LibraryReceipt
from tools.reference.source import LibrarySource

class AcquisitionTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.manifest=json.loads(Path('tests/common/reference.json').read_text())

    def test_recovers_complete_checked_source_graph(self):
        path=Path(os.environ['REFERENCE_LIBRARY_ROOT'])/'manifest.json'
        raw=path.read_bytes();metadata=json.loads(raw)
        self.assertEqual(hashlib.sha256(raw).hexdigest(),'92a93abde82f7933090293e8c95880f98dcee1d50a7463e74b108e29e7b2db99')
        reader=LibraryReader(LibraryReceipt(path,hashlib.sha256(raw).hexdigest(),metadata['catalog_sha256'],metadata['source_manifest_sha256'],()))
        source=LibrarySource(reader)
        recovered=recover_acquisition(source,self.manifest)
        validate_acquisition(recovered,self.manifest)
        self.assertEqual(recovered,self.manifest['acquisition'])
        code=(reader.records_root/'managed/Assembly-CSharp.cs').read_text(encoding='utf-8-sig')
        database=_block(code,code.index('public class itemdatabase :'))
        constructors=_block(database,database.index('public class category'))
        self.assertEqual(constructors.count('public category('),5)
        self.assertIn('categorychances = cchance;',constructors)
        enums,categories=_database(code)
        self.assertEqual(enums['k']['misc4'],35)
        self.assertEqual(len(categories),89)
        self.assertNotIn(21,categories)
        self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(),hashlib.sha256(raw).hexdigest())

    def test_checked_acquisition_graphs(self):
        a=self.manifest['acquisition']
        self.assertEqual([len(s['ordinary']) for s in a['stations']],[33,37,236])
        self.assertEqual([len(s['misc4']) for s in a['stations']],[18,6,38])
        self.assertEqual([len(s['underground']) for s in a['stations']],[1,1,1])
        categories={c['id']:c for c in a['categories']}
        self.assertEqual(len(categories),86)
        self.assertEqual(categories[35]['children'],[32,33,34,49,50,51,58,68,70,78,89])
        self.assertEqual(categories[35]['weights'],[20,20,20,5,35,25,30,15,50,20,3])
        self.assertEqual(categories[32]['children'],[74,172,217,218,220,245,305,306])
        self.assertEqual(categories[32]['weights'],[1]*8)
        self.assertEqual(categories[32]['constructor_weights'],[10,20,70,30,10,10,10,10])
        self.assertEqual(categories[84]['children'],[217,218,245,221])
        self.assertEqual(categories[84]['weights'],[1]*4)
        self.assertIn(80,categories[82]['children']);self.assertIn(67,categories[80]['children']);self.assertIn(216,categories[67]['children'])
        self.assertEqual(sorted(v['target'] for c in categories.values() for v in c['invalid_children']),[186,187,265,266,303,304,386,387])
        self.assertTrue(any(p is None for c in categories.values() if not c['categories'] for p in c['prototypes']))
        spawners={s['source']['path_id']:s for station in a['stations'] for s in station['ordinary']}
        for pid,chance in [(132525,.20000000298023224),(137208,.6499999761581421),(130805,.4000000059604645)]:self.assertEqual(spawners[pid]['chance'],chance)
        self.assertTrue(all(s['underground'][0]['vertical']==[-.15000000596046448,-.4000000059604645] for s in a['stations']))
        self.assertEqual([p['source']['path_id'] for p in a['startup_packs']],[14079,14082])
        self.assertEqual(len(a['box']['disabled_starter_boxes']),8)
        self.assertEqual(a['packs'][0]['ammo']['random_range'],[20.,1.])
        self.assertEqual(len(a['shovel']['motion']['clips']),6)
        validate_acquisition(a,self.manifest)

    def test_rejects_malformed_source_operations_shapes_and_links(self):
        a=self.manifest['acquisition']
        validate_acquisition(a,self.manifest)
        cases=[
            ('stations/0/ordinary/0/chance',.9),('stations/0/ordinary/0/marker/node',4294967295),
            ('stations/0/ordinary/0/entries/0/selection/source/path_id',1),
            ('stations/0/ordinary/0/entries/0/selection/item_id',230),
            ('stations/0/ordinary/0/entries/0/selection/prototype',{'file':'sharedassets1.assets','path_id':32825}),
            ('stations/0/ordinary/0/entries/0/initialization/mix_chance',True),
            ('stations/0/underground/0/vertical',[-.4,-.15]),('stations/0/underground/0/table',[]),
            ('stations/0/flyers/supported',True),('stations/0/flyers/prefab/path_id',1),
            ('categories/0/children',[999]),('box/release',[]),('box/clips/0/path_id',1),
            ('box/content_counts/0/maximum',999),('box/table',[]),('box/disabled_starter_boxes/0/disable_spawn',False),
            ('packs/0/ammo/random_range',[1.,20.]),('packs/0/take_one_order',[]),('startup_packs/0/group/path_id',206),
            ('holiday_startup/start',[10,14]),('holiday_pumpkin/path_id',8094),
            ('rules/integer_upper_exclusive',False),('rules/flows/5/steps/0/lower/value',0.),
            ('rules/flows/5/steps/0/kind','integer_range'),('rules/flows/10/steps/2/upper/value','category_total'),
            ('shovel/contact/force',1),('shovel/contact/hurt_food',1),('shovel/controller/path_id',1),('shovel/right_hand_poses',[{'file':'level1','path_id':1}]),('shovel/motion/parameters/0/id',1),('shovel/motion/any_state_transitions/0/destination',5),
            ('shovel/motion/states/1/callback/path_id',1),('shovel/motion/clips/0/tracks/0/node',0),
            ('shovel/motion/clips/0/tracks/0/values/0',[0.,0.]),('shovel/motion/clips/0/tracks/0/coefficients/0/0/0',float('nan')),
            ('shovel/motion/clips/0/source_tracks/sha256','0'*64),('shovel/motion/clips',[]),('evidence/methods',[]),
        ]
        for path,value in cases:
            changed=copy.deepcopy(a);parts=path.split('/');target=changed
            for k in parts[:-1]:target=target[int(k)] if isinstance(target,list) else target[k]
            target[int(parts[-1]) if isinstance(target,list) else parts[-1]]=value
            with self.subTest(path=path),self.assertRaises(ValueError):validate_acquisition(changed,self.manifest)
        for path,field in [('',k) for k in a]+[('shovel/motion/states/0','callback'),('packs/0/ammo','float_or_int'),('box','authored_spawn_position')]:
            changed=copy.deepcopy(a);target=changed
            for k in path.split('/') if path else []:target=target[int(k)] if isinstance(target,list) else target[k]
            del target[field]
            with self.subTest(missing=path+'/'+field),self.assertRaises(ValueError):validate_acquisition(changed,self.manifest)
        changed=copy.deepcopy(a);changed['shovel']['motion']['clips'][0]['extra']=0
        with self.assertRaises(ValueError):validate_acquisition(changed,self.manifest)

    def test_rejects_crossed_startup_and_holiday_owners(self):
        a=self.manifest['acquisition'];validate_acquisition(a,self.manifest)
        for owner in ('save','body','pickup','active','manual_start','pose'):
            changed=copy.deepcopy(a);h=changed['holiday_startup']
            if owner in ('save','body','pickup'):h[owner]['path_id']=1
            elif owner=='pose':h[owner]=changed['startup_packs'][0]['pose']
            else:h[owner]=False
            with self.subTest(owner=owner),self.assertRaises(ValueError):validate_acquisition(changed,self.manifest)
        changed=copy.deepcopy(a)
        for field in ('pose','group_pose'):changed['startup_packs'][0][field]=changed['startup_packs'][1][field]
        with self.assertRaises(ValueError):validate_acquisition(changed,self.manifest)
