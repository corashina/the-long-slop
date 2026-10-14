import hashlib
import json
import unittest
from pathlib import Path
from tools.reference.conditions import prepare_cleaning_roots

ROOT=Path(__file__).resolve().parents[3]
PRIMARY=Path('C:/Users/Corashina/Documents/Projects/the-long-drive-wasm')

class ConditionsTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        from tools.reference.library.types import LibraryReceipt
        from tools.reference.library.reader import LibraryReader
        from tools.reference.source import LibrarySource
        path=PRIMARY/'web/assets/library/manifest.json'
        content=path.read_bytes(); manifest=json.loads(content)
        receipt=LibraryReceipt(path,hashlib.sha256(content).hexdigest(),manifest['catalog_sha256'],manifest['source_manifest_sha256'],())
        cls.source=LibrarySource(LibraryReader(receipt))

    def setUp(self):
        self.manifest=json.loads((ROOT/'web/assets/reference/manifest.json').read_bytes())

    def test_startup_tools_follow_enable_after_start(self):
        from tools.reference.source import key
        roots={'items':[]}
        prepare_cleaning_roots(self.source,roots)
        self.assertEqual(len(roots['items']),6)
        self.assertEqual({r['game_object']['path_id'] for r in self.source.cleaning_startup},{10842,10846,10838,10849,10841,10840})
        record=self.manifest['conditions']
        self.assertEqual(len(record['brushes']), 4)
        self.assertEqual(sorted(t['cleaning_type'] for t in record['brushes']), [1, 1, 2, 2])
        self.assertEqual({t['source']['path_id'] for t in record['brushes']}, {66615,66617,66616,66619})
        self.assertEqual({t['source']['path_id'] for t in record['sprays']}, {73834,73835})
        nodes={n['id']:n for n in self.manifest['nodes']}
        for tool in [*record['brushes'],*record['sprays']]:
            item=next(i for i in self.manifest['items'] if i['id']==tool['item'])
            self.assertTrue(nodes[item['node']]['active'])
            self.assertIsNotNone(item['pickup'])
            self.assertEqual(item['body'],tool['body'])
            reader=self.source.get(tool['source']['file'],tool['source']['path_id'])
            data=self.source.read(reader)
            if 'slide' in data:
                slide=self.source.resolve(reader,data['slide'])
                self.assertEqual(tool['slide'],{'file':key(slide)[0],'path_id':key(slide)[1]})
                self.assertEqual(tool['distance'],2.0)
                self.assertEqual(tool['mask'],8552449)
                self.assertEqual(tool['minimum_displacement'],0.009999999776482582)
                self.assertEqual(tool['maximum_displacement'],0.10000000149011612)

    def test_spray_keeps_authored_float_ammo(self):
        record=self.manifest['conditions']
        self.assertEqual([t['startup_quantity'] for t in record['sprays']], [100.0, 100.0])
        self.assertEqual([t['maximum_quantity'] for t in record['sprays']], [100.0,100.0])
        self.assertTrue(all(t['float_quantity'] for t in record['sprays']))
        for tool in record['sprays']:
            reader=self.source.get(tool['source']['file'],tool['source']['path_id'])
            ammo=self.source.resolve(reader,self.source.read(reader)['ammo'])
            self.assertEqual(self.source.read(ammo)['fammo'],tool['startup_quantity'])

    def test_variants_close_material_dependencies(self):
        materials={(m['asset'],m['gltf_material']):m for m in self.manifest['rendering']['materials']}
        assets={a['id']:a for a in self.manifest['assets']}
        for surface in self.manifest['conditions']['surfaces']:
            for renderer in surface['renderers']:
                self.assertEqual(len(renderer['states']),5)
                for state in renderer['states']:
                    for binding in state['materials']:
                        material=materials[binding['asset'],binding['gltf_material']]
                        self.assertEqual(material['source'],binding['source'])
                        for texture in material['properties']['textures']:
                            for mip in texture['mips']:
                                self.assertTrue(any(d['file']==mip['file'] for d in assets[binding['asset']]['dependencies']))
                for binding in renderer['meshes']:
                    asset=assets[binding['asset']]
                    document=json.loads((ROOT/'web/assets'/asset['path']).read_bytes())
                    self.assertEqual(document['meshes'][binding['gltf_mesh']]['extras']['source'],binding['source'])

if __name__=='__main__':unittest.main()
