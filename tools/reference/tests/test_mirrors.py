import unittest, json
from pathlib import Path
from copy import deepcopy
from tools.reference.mirrors import recover_mirrors
from tools.reference.library.reader import LibraryReader
from tools.reference.source import LibrarySource, key
from tools.reference.library.types import LibraryReceipt

PRIMARY = Path('C:/Users/Corashina/Documents/Projects/the-long-drive-wasm')
class MirrorRecoveryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        import hashlib
        path = PRIMARY/'web/assets/library/manifest.json'
        content = path.read_bytes(); manifest = json.loads(content)
        receipt = LibraryReceipt(path, hashlib.sha256(content).hexdigest(), manifest['catalog_sha256'], manifest['source_manifest_sha256'], ())
        cls.source = LibrarySource(LibraryReader(receipt))
        root = cls.source.transform_for(cls.source.get('level1',17687))
        hierarchy = cls.source.hierarchy(root, activate_root=True)
        cls.nodes = [{'id':i+1,'source':{'file':key(n['reader'])[0],'path_id':key(n['reader'])[1]}} for i,n in enumerate(hierarchy)]
        mapping = {key(n['reader']):i+1 for i,n in enumerate(hierarchy)}
        cls.components = {i+1:{} for i in range(len(hierarchy))}
        for i,n in enumerate(hierarchy):
            for c in n['components']:
                if c.type.name == 'MonoBehaviour': cls.components[i+1].setdefault(cls.source.class_name(c),[]).append(c)
        def pointer_node(owner,pointer):
            r=cls.source.resolve(owner,pointer)
            if r.type.name!='Transform': r=cls.source.transform_for(cls.source.resolve(r,cls.source.read(r)['m_GameObject']))
            return mapping[key(r)]
        cls.pointer_node=staticmethod(pointer_node)
        cls.items={n:i for i,n in enumerate(cls.components,1)}
        cls.reports={'lada':{'material_indices':{'sharedassets1.assets:624':0,'sharedassets1.assets:625':1}}}
        main=cls.source.get('level1',64919)
        off=cls.source.resolve(main,cls.source.read(main)['noMirrorMaterial'])
        cls.reports['lada']['material_indices'][f'{off.assets_file.name}:{off.path_id}']=1
    def test_selected_lada_mirrors(self):
        result=recover_mirrors(self.source,self.nodes,self.components,self.pointer_node,self.items,self.reports)
        self.assertEqual([m['source']['path_id'] for m in result['mirrors']],[66131,67237,70638])
        self.assertEqual([m['camera_source']['path_id'] for m in result['mirrors']],[40260,40324,40222])
        self.assertEqual([m['surface_source']['path_id'] for m in result['mirrors']],[43665,40735,44371])
        by_id={n['id']:n['source']['path_id'] for n in self.nodes}
        self.assertEqual([by_id[m['plane_node']] for m in result['mirrors']],[28414,38060,32190])
        self.assertEqual([by_id[m['surface_node']] for m in result['mirrors']],[31962,33202,28868])
        for mirror in result['mirrors']:
            screen=self.source.get('level1',by_id[mirror['surface_node']])
            self.assertEqual(self.source.read(screen)['m_Father']['m_PathID'],by_id[mirror['plane_node']])
            renderer=self.source.get('level1',mirror['surface_source']['path_id'])
            game_object=self.source.resolve(renderer,self.source.read(renderer)['m_GameObject'])
            self.assertEqual(self.source.transform_for(game_object).path_id,screen.path_id)
        self.assertEqual([by_id[m['camera_node']] for m in result['mirrors']],[39254,31061,26965])
        self.assertEqual(result['target']['extent'],[256,256]); self.assertEqual(result['target']['samples'],1)
        self.assertEqual(result['rules']['fov_bounds_degrees'],[10,55]); self.assertEqual(result['rules']['distance_bounds_meters'],[0,4])
        self.assertEqual(result['rules']['count_limit'],5)
        self.assertEqual(result['rules']['draw_distance_meters'],100.)
        self.assertTrue(result['rules']['enabled'])
        empty=[r for cs in self.components.values() for r in cs.get('mirrorscript',[]) if not self.source.read(r)['M']['C']['m_PathID']]
        self.assertGreater(len(empty),0)
        self.assertTrue({r.path_id for r in empty}.isdisjoint(m['source']['path_id'] for m in result['mirrors']))
        from tools.reference.materials import recover_material
        material=recover_material(self.source,self.source.get('sharedassets1.assets',624))
        self.assertEqual(material['properties']['floats']['_Metallic'],1.)
        self.assertEqual(material['properties']['floats']['_Glossiness'],0.)
        self.assertEqual(material['properties']['colors']['_Color'],[1.,1.,1.,1.])
        self.assertEqual(material['properties']['colors']['_EmissionColor'],[0.8942506909370422,0.8900325298309326,0.8900325298309326,1.])
        self.assertEqual(material['properties']['exclusions'],[])
        for m in result['mirrors']:
            self.assertEqual(m['culling_mask'],73113395); self.assertAlmostEqual(m['near_plane'],.1)
            self.assertEqual(m['live_texture_slots'],['_MainTex','_EmissionMap'])
            self.assertEqual(m['live_material']['source']['path_id'],624)
            self.assertEqual(m['disabled_material']['source'],{'file':'sharedassets0.assets','path_id':116})
            self.assertEqual(m['initial_far_plane'],1500.)
    def test_required_link_failure(self):
        reader=self.source.get('level1',66131);original=self.source.read(reader)
        bad=deepcopy(original);bad['M']['C']={'m_FileID':0,'m_PathID':0}
        self.source.data_cache[key(reader)]=bad
        try:
            with self.assertRaisesRegex(ValueError,'level1:66131.*M.C'):
                recover_mirrors(self.source,self.nodes,self.components,self.pointer_node,self.items,self.reports)
        finally:self.source.data_cache[key(reader)]=original
    def test_runtime_staging_preserves_accepted_cache(self):
        import tempfile
        from types import SimpleNamespace as NS
        from unittest.mock import patch
        from tools.reference.export import derive_reference
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp).resolve();cache=root/'cache';cache.mkdir();(cache/'evidence').write_bytes(b'accepted')
            output=root/'output/reference';output.parent.mkdir()
            snapshot=NS(manifest_sha256='a'*64,files=(),root=cache/'source',manifest_path=cache/'source-manifest.json')
            receipt=NS(source_manifest_sha256='a'*64)
            reader=NS(snapshot=snapshot,root=root/'library',records_root=cache/'records')
            before=[(p.relative_to(cache),p.read_bytes(),p.stat().st_mtime_ns) for p in cache.rglob('*') if p.is_file()]
            def selected(source,staged,library,selected_snapshot):
                self.assertEqual(staged.parent,output.parent)
                (staged/'result').write_bytes(b'new')
            with patch('tools.reference.library.reader.LibraryReader',return_value=reader), patch('tools.reference.source.LibrarySource'), patch('tools.reference.library.storage.check_space') as space, patch('tools.reference.export._export_selected',side_effect=selected), patch('tools.reference.export.verify_reference'), patch('tools.reference.export.promote_reference') as promote:
                derive_reference(snapshot,receipt,output)
                self.assertEqual(space.call_args.args[1],output.parent)
                self.assertEqual(promote.call_args.args[1],output)
            self.assertEqual(before,[(p.relative_to(cache),p.read_bytes(),p.stat().st_mtime_ns) for p in cache.rglob('*') if p.is_file()])
            self.assertEqual(list(cache.iterdir()),[cache/'evidence'])
