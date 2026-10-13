import hashlib,json,tempfile,unittest
from copy import deepcopy
from pathlib import Path
from tools.reference.plates import recover_plates
from tools.reference.library.reader import LibraryReader
from tools.reference.library.types import LibraryReceipt
from tools.reference.source import LibrarySource,key

PRIMARY=Path('C:/Users/Corashina/Documents/Projects/the-long-drive-wasm')
class PlateRecoveryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        path=PRIMARY/'web/assets/library/manifest.json'; content=path.read_bytes();m=json.loads(content)
        cls.source=LibrarySource(LibraryReader(LibraryReceipt(path,hashlib.sha256(content).hexdigest(),m['catalog_sha256'],m['source_manifest_sha256'],())))
        manifest=json.loads((PRIMARY/'web/assets/reference/manifest.json').read_bytes())
        cls.nodes=manifest['nodes'];cls.items={i['node']:i['id'] for i in manifest['items']};cls.colliders=manifest['colliders']
        cls.components={}
        for pid in [22542,30779]:
            r=cls.source.get('level1',pid);_,cs=cls.source.components(r)
            node=next(n['id'] for n in cls.nodes if n['source']=={'file':'level1','path_id':pid})
            cls.components[node]={}
            for c in cs:
                name=cls.source.class_name(c) if c.type.name=='MonoBehaviour' else c.type.name
                cls.components[node].setdefault(name,[]).append(c)
        mapping={(n['source']['file'],n['source']['path_id']):n['id'] for n in cls.nodes}
        def pointer_node(owner,pointer):
            r=cls.source.resolve(owner,pointer)
            if r.type.name not in ('Transform','RectTransform'):
                go=r if r.type.name=='GameObject' else cls.source.resolve(r,cls.source.read(r)['m_GameObject'])
                r=next(cls.source.resolve(go,c['component']) for c in cls.source.read(go)['m_Component'] if cls.source.resolve(go,c['component']).type.name in ('Transform','RectTransform'))
            return mapping[key(r)]
        cls.pointer_node=staticmethod(pointer_node)
    def recover(self):
        with tempfile.TemporaryDirectory() as tmp:
            return recover_plates(self.source,self.nodes,self.components,self.pointer_node,self.items,self.colliders,Path(tmp))
    def test_lada_startup_plate_bindings(self):
        result=self.recover()
        self.assertEqual([p['source']['path_id'] for p in result['plates']],[78750,73955])
        self.assertEqual([p['startup_target'] for p in result['plates']],[66346,66601])
        self.assertEqual(result['font']['source']['path_id'],1366)
        self.assertEqual(result['font']['atlas']['source']['path_id'],366)
        self.assertEqual(result['material']['source']['path_id'],18)
        self.assertEqual(len(result['font']['glyphs']),250)
        self.assertEqual([p['mass_kg'] for p in result['plates']],[1.,1.])
    def test_broken_startup_link_rejects_export(self):
        reader=self.source.get('level1',66420);original=self.source.read(reader);bad=deepcopy(original)
        bad['attachAtStartTarget']={'m_FileID':0,'m_PathID':0};self.source.data_cache[key(reader)]=bad
        try:
            with self.assertRaisesRegex(ValueError,'level1:78750.*attachAtStartTarget'):self.recover()
        finally:self.source.data_cache[key(reader)]=original
