import copy
import json
import hashlib
import os
import unittest
from pathlib import Path
from tools.reference.item_prototypes import validate_item_prototypes


class ItemPrototypeTests(unittest.TestCase):
    def fixture(self):
        return json.loads(Path('tests/common/reference.json').read_bytes())

    def test_recovers_checked_source_fields_and_geometry(self):
        from tools.reference.item_prototypes import recover_item_prototypes
        from tools.reference.library.reader import LibraryReader
        from tools.reference.library.types import LibraryReceipt
        from tools.reference.source import LibrarySource
        path=Path(os.environ['REFERENCE_LIBRARY_ROOT'])/'manifest.json'
        raw=path.read_bytes();d=json.loads(raw)
        source=LibrarySource(LibraryReader(LibraryReceipt(path,hashlib.sha256(raw).hexdigest(),d['catalog_sha256'],d['source_manifest_sha256'],())))
        manifest=self.fixture()
        recovered=recover_item_prototypes(source,manifest,None)
        self.assertEqual(json.loads(json.dumps(recovered)),manifest['item_prototypes'])
        for prototype in recovered:
            for binding in prototype['pickups']:
                pickup=binding['pickup'];reader=source.get(pickup['source']['file'],pickup['source']['path_id'])
                self.assertEqual(pickup['can_inventory'],bool(source.read(reader)['canInventory']))
        from unittest.mock import patch
        reader=source.get('sharedassets1.assets',149224)
        fields=copy.deepcopy(source.read(reader));fields['invImg']={'m_FileID':0,'m_PathID':141307}
        read=source.read
        from tools.reference.actions import recover_pickup
        with patch.object(source,'read',side_effect=lambda r:fields if r==reader else read(r)):
            with self.assertRaisesRegex(ValueError,'expected Sprite'):recover_pickup(source,reader)
        self.assertEqual([len(p['renderers']) for p in recovered],[3,3,3,56,14,25,4,2,2,2,7])
        self.assertEqual([len(p['colliders']) for p in recovered],[1,2,2,58,15,3,2,1,12,3,1])
        for prototype in recovered:
            asset=next(a for a in manifest['assets'] if a['id']==prototype['asset'])
            document=json.loads((Path('web/assets')/asset['path']).read_bytes())
            for renderer in prototype['renderers']:
                node=next(n for n in manifest['nodes'] if n['id']==renderer['node'])
                exported=document['nodes'][node['gltf_node']]
                self.assertEqual(exported['extras']['renderer_enabled'],renderer['enabled'])
                mesh=exported['mesh']
                primitives=document['meshes'][mesh]['primitives']
                self.assertTrue(primitives)
                self.assertTrue(all('material' in primitive for primitive in primitives))
            self.assertTrue(asset['dependencies'])

    def test_complete_checked_templates_and_isolated_physics(self):
        m = self.fixture()
        p = m['item_prototypes'][:5]
        validate_item_prototypes(m['item_prototypes'], m)
        self.assertEqual([x['id']['path_id'] for x in p], [75443, 51722, 61587, 81642, 81645])
        self.assertEqual([x['item_id'] for x in p], [74, 62, 216, None, None])
        self.assertEqual([len(x['pickups']) for x in p], [1, 1, 1, 56, 14])
        self.assertEqual([len(x['bodies']) for x in p], [1, 1, 1, 56, 14])
        self.assertEqual([x['pickups'][0]['pickup']['can_inventory'] for x in p], [True, False, True, True, True])
        self.assertEqual([x['pickups'][0]['pickup']['can_hold_right_hand'] for x in p], [False, False, True, False, False])
        prototype_nodes = {n['id'] for n in m['nodes'] if n['asset'] in {x['asset'] for x in p}}
        self.assertFalse(prototype_nodes.intersection(b['node'] for b in m['bodies']))
        self.assertFalse(prototype_nodes.intersection(i['node'] for i in m['items']))
        self.assertEqual([[x['pickup']['source']['path_id'] for x in t['pickups'] if not x['pickup']['can_inventory']] for t in p[3:]],[[135313,140775],[135314]])
        self.assertEqual([c['path_id'] for c in p[2]['behavior']['contact_clips'][4]],[4354,4119,4406,4305])
        self.assertEqual(len(p[0]['variants'][0]['nodes']), 3)
        self.assertEqual(p[0]['behavior']['cigarette_prefab'], {'file':'sharedassets1.assets','path_id':11985})
        self.assertEqual([c['path_id'] for c in p[1]['behavior']['clips']], [4223,4140])

    def test_rejects_broken_local_contracts(self):
        original = self.fixture()
        mutations = [
            lambda p,m: p[0].update(root=m['nodes'][0]['id']),
            lambda p,m: p[0]['pickups'][0].update(body=999999),
            lambda p,m: p[0]['bodies'].append(copy.deepcopy(p[0]['bodies'][0])),
            lambda p,m: p[0]['colliders'].append(copy.deepcopy(p[0]['colliders'][0])),
            lambda p,m: p[3]['colliders'][0].update(body=p[3]['bodies'][1]['id']),
            lambda p,m: p[0]['pickups'][0]['pickup']['inventory_icon'].update(path_id=1),
            lambda p,m: p[1]['behavior']['clips'][0].update(path_id=1),
            lambda p,m: p[0]['variants'][0]['nodes'].pop(),
            lambda p,m: p[3]['pickups'].pop(),
            lambda p,m: p[3]['pickups'][10]['pickup'].update(can_inventory=True),
            lambda p,m: p[2]['behavior']['contact_clips'][4][0].update(path_id=1),
            lambda p,m: p[0]['renderers'][0].update(node=m['nodes'][0]['id']),
            lambda p,m: p[0]['pickups'][0]['pickup'].update(disable_when_stored=m['nodes'][0]['source']),
        ]
        for mutate in mutations:
            m=copy.deepcopy(original);p=m['item_prototypes'];mutate(p,m)
            with self.subTest(mutation=mutate), self.assertRaises(ValueError):validate_item_prototypes(p,m)

    def test_rejects_appended_variants_and_active_prototype_collider(self):
        original=self.fixture()
        for node in (999999,1,original['item_prototypes'][0]['variants'][0]['nodes'][0]):
            m=copy.deepcopy(original)
            m['item_prototypes'][0]['variants'][0]['nodes'].append(node)
            with self.subTest(node=node),self.assertRaises((ValueError,KeyError)):
                validate_item_prototypes(m['item_prototypes'],m)
        m=copy.deepcopy(original)
        collider=copy.deepcopy(m['item_prototypes'][0]['colliders'][0])
        collider.update(id=max(c['id'] for c in m['colliders'])+1,body=m['bodies'][0]['id'])
        m['colliders'].append(collider)
        binding=copy.deepcopy(m['action_bindings'][0]);binding['collider']=collider['id']
        for field in ('use_owner','pickup_owner','removal_owner','push_owner'):binding[field]=None
        m['action_bindings'].append(binding)
        with self.assertRaises(ValueError):validate_item_prototypes(m['item_prototypes'],m)
