import copy
import hashlib
import json
import os
import unittest
from pathlib import Path
from tools.reference.clothing import recover_clothing, validate_clothing


class WearableReferenceTests(unittest.TestCase):
    def test_checked_source_and_isolated_templates(self):
        from tools.reference.library.reader import LibraryReader
        from tools.reference.library.types import LibraryReceipt
        from tools.reference.source import LibrarySource
        path=Path(os.environ['REFERENCE_LIBRARY_ROOT'])/'manifest.json'
        raw=path.read_bytes();metadata=json.loads(raw)
        source=LibrarySource(LibraryReader(LibraryReceipt(path,hashlib.sha256(raw).hexdigest(),metadata['catalog_sha256'],metadata['source_manifest_sha256'],())))
        manifest=json.loads(Path('tests/common/reference.json').read_bytes())
        recovered=recover_clothing(source,manifest,None)
        self.assertEqual(json.loads(json.dumps(recovered)),manifest['clothing'])
        validate_clothing(recovered,manifest)
        self.assertEqual([w['source']['path_id'] for w in recovered['wearables']],[146739,147668,147667,131348,148817,142834])
        self.assertEqual(recovered['player_root'],{'file':'level1','path_id':39843})
        self.assertEqual([a['path_id'] for a in recovered['anchors']],[26477,28766,35477,36615,39162])
        self.assertEqual([p['source']['path_id'] for p in recovered['glasses_profiles']],list(range(130128,130134)))
        self.assertEqual(len(manifest['item_prototypes']),11)
        glasses=recovered['wearables'][0]
        self.assertEqual(sum(v['lens'] is not None for s in glasses['selectors'] for v in s['variants']),19)
        self.assertEqual([v['profile'] for v in glasses['selectors'][0]['variants']],[None,None,0,None,4])
        self.assertEqual([p['pickups'][0]['pickup']['can_inventory'] for p in manifest['item_prototypes'][5:]],[True,True,True,False,True,False])
        self.assertTrue(all(not p['pickups'][0]['pickup']['can_hold_right_hand'] for p in manifest['item_prototypes'][5:]))


    def test_rejects_broken_required_links(self):
        original=json.loads(Path('tests/common/reference.json').read_bytes())
        for mutate in (
            lambda c:c['wearables'][0]['prototype'].update(path_id=75443),
            lambda c:c['wearables'][0]['renderers'].append(999999),
            lambda c:c['anchors'][0].update(path_id=1),
            lambda c:c['anchor_nodes'][1].update(parent=None),
            lambda c:c['glasses_profiles'].pop(),
            lambda c:c['wearables'][0]['selectors'][1].update(node=c['wearables'][0]['selectors'][2]['node']),
            lambda c:c['wearables'][0]['lenses'][0]['source'].update(file='other'),
            lambda c:c['wearables'][0]['selectors'][0]['variants'][2].update(lens=None,profile=None,disable_lens_flare=None),
            lambda c:c['wearables'][0]['selectors'][2]['variants'][0].update(nodes=[999999]),
            lambda c:c['wearables'][0]['selectors'][1]['variants'][0]['renderers'][0].update(slot=99),
            lambda c:c['wearables'][0]['selectors'][1]['variants'][0]['material'].update(path_id=1),
            lambda c:c['wearables'][0]['selectors'][2]['variants'][0].update(profile=5),
            lambda c:c['glasses_profiles'][0]['effects'][0].update(kind='unknown'),
            lambda c:c['glasses_profiles'][0]['effects'][0]['parameters']['ldrLut'].update(texture=999999),
            lambda c:c['glasses_profiles'][0]['effects'][0]['parameters']['contrast'].update(override_state=True),
        ):
            m=copy.deepcopy(original);mutate(m['clothing'])
            with self.subTest(mutation=mutate),self.assertRaises(ValueError):validate_clothing(m['clothing'],m)

    def test_local_consumable_and_required_resources(self):
        from tools.reference.item_prototypes import validate_item_prototypes
        original=json.loads(Path('tests/common/reference.json').read_bytes())
        mutations=[
            lambda m:m['item_prototypes'][-1]['consumable'].update(min_particle=0),
            lambda m:m['item_prototypes'][-1]['consumable'].update(continuous=False),
            lambda m:m['item_prototypes'][-1]['consumable'].update(destroy_on_end=True),
            lambda m:m['item_prototypes'][-1]['consumable'].update(parts=[999999]),
            lambda m:m['item_prototypes'][-1]['consumable']['ember']['particle'].update(simulation_space=2),
            lambda m:m['item_prototypes'][-1]['consumable']['ember']['particle']['module_flags'].update(NoiseModule=True),
            lambda m:m['lighting']['textures'].clear(),
            lambda m:m['lighting']['lights'].append(m['item_prototypes'][-1]['consumable']['ember']['light']),
            lambda m:m['weather']['particles'].append(m['item_prototypes'][-1]['consumable']['ember']['particle']),
            lambda m:m['item_prototypes'][6]['renderers'][-1]['state'].update(cast_shadows=1),
        ]
        for mutate in mutations:
            m=copy.deepcopy(original);mutate(m)
            with self.subTest(mutation=mutate),self.assertRaises(ValueError):validate_item_prototypes(m['item_prototypes'],m)

    def test_shadow_only_source_rejected_for_active_use(self):
        from tools.reference.render_lighting import recover_renderer
        from tools.reference.library.reader import LibraryReader
        from tools.reference.library.types import LibraryReceipt
        from tools.reference.source import LibrarySource
        path=Path(os.environ['REFERENCE_LIBRARY_ROOT'])/'manifest.json'
        raw=path.read_bytes();metadata=json.loads(raw)
        source=LibrarySource(LibraryReader(LibraryReceipt(path,hashlib.sha256(raw).hexdigest(),metadata['catalog_sha256'],metadata['source_manifest_sha256'],())))
        manifest=json.loads(Path('tests/common/reference.json').read_bytes())
        records=[r for p in manifest['item_prototypes'] for r in p['renderers'] if r['state']['cast_shadows']==3]
        self.assertEqual({r['source']['path_id'] for r in records},{98477,98194,93128,91870,98218})
        for r in records:
            renderer=source.get(r['source']['file'],r['source']['path_id'])
            mesh=source.get(r['mesh']['file'],r['mesh']['path_id'])
            self.assertEqual(recover_renderer(source,renderer,mesh,'item_prototype'),r['state'])
            with self.assertRaises(ValueError):recover_renderer(source,renderer,mesh)

    def test_checked_rgb24_lut_mips_and_bounds(self):
        from types import SimpleNamespace
        from tools.reference.mips import decoded_mips,validate_mips
        from tools.reference.library.reader import LibraryReader
        from tools.reference.library.types import LibraryReceipt
        from tools.reference.source import LibrarySource
        path=Path(os.environ['REFERENCE_LIBRARY_ROOT'])/'manifest.json'
        raw=path.read_bytes();metadata=json.loads(raw)
        source=LibrarySource(LibraryReader(LibraryReceipt(path,hashlib.sha256(raw).hexdigest(),metadata['catalog_sha256'],metadata['source_manifest_sha256'],())))
        obj=source.texture_object(source.get('sharedassets0.assets',255))
        self.assertEqual(obj.m_TextureFormat,3)
        raw=obj.get_image_data();decoded=decoded_mips(obj)
        validate_mips(decoded,obj.m_Width,obj.m_Height,obj.m_MipCount)
        source_offset=0;decoded_offset=16
        for level in range(obj.m_MipCount):
            count=max(1,obj.m_Width>>level)*max(1,obj.m_Height>>level)
            data=raw[source_offset:source_offset+3*count]
            expected=b''.join(data[i:i+3]+b'\xff' for i in range(0,len(data),3))
            self.assertEqual(decoded[decoded_offset:decoded_offset+count*4],expected)
            source_offset+=3*count;decoded_offset+=4*count
        self.assertEqual(source_offset,len(raw))
        for data,kind in [(raw[:-1],3),(raw+b'\x00',3),(raw,999)]:
            invalid=SimpleNamespace(m_TextureFormat=kind,m_Width=obj.m_Width,m_Height=obj.m_Height,m_MipCount=obj.m_MipCount,get_image_data=lambda:data)
            with self.assertRaises(ValueError):decoded_mips(invalid)

    def test_rejects_unknown_cigarette_operations(self):
        from tools.reference.item_prototypes import validate_item_prototypes
        original=json.loads(Path('tests/common/reference.json').read_bytes())
        for field in ('ShapeModule.type','ShapeModule.arc.mode','ShapeModule.radius.mode','ShapeModule.m_MeshSpawn.mode',
                      'ShapeModule.placementMode','ShapeModule.m_TextureClipChannel','ShapeModule.m_TextureUVChannel',
                      'renderer.m_RenderMode','renderer.m_RenderAlignment','renderer.m_SortMode',
                      'cullingMode','emitterVelocityMode','ringBufferMode','scalingMode','stopAction','moveWithTransform'):
            m=copy.deepcopy(original)
            m['item_prototypes'][-1]['consumable']['ember']['particle']['scalars'][field]=999
            with self.subTest(field=field),self.assertRaises(ValueError):validate_item_prototypes(m['item_prototypes'],m)

    def test_rejects_incomplete_typed_payloads(self):
        from tools.reference.item_prototypes import validate_item_prototypes
        original=json.loads(Path('tests/common/reference.json').read_bytes())
        mutations=[
            lambda m:m['clothing']['glasses_profiles'][0]['effects'][0]['parameters']['blueCurve'].pop('value'),
            lambda m:m['clothing']['glasses_profiles'][0]['effects'][0]['parameters']['blueCurve']['value']['curve']['keys'][0].pop('in_weight'),
            lambda m:m['clothing']['glasses_profiles'][0]['effects'][0]['parameters']['blueCurve']['value'].update(cached_data=[]),
            lambda m:m['clothing']['glasses_profiles'][0]['effects'][0]['parameters']['colorFilter'].update(value=[1,1,1]),
            lambda m:m['clothing']['glasses_profiles'][0]['effects'][0]['parameters']['brightness'].update(value='1'),
            lambda m:m['item_prototypes'][-1]['consumable']['ember'].pop('colors'),
            lambda m:m['item_prototypes'][-1]['consumable']['ember'].update(colors=[[1,1,1,1]]),
            lambda m:m['item_prototypes'][-1]['consumable']['ember']['light'].pop('color'),
            lambda m:m['item_prototypes'][-1]['consumable']['ember']['particle']['curves'].update({'InitialModule.startLifetime':{}}),
            lambda m:m['item_prototypes'][-1]['consumable']['ember']['particle']['curves']['SizeModule.curve']['max_curve']['keys'][0].pop('out_tangent'),
            lambda m:m['item_prototypes'][-1]['consumable']['ember']['particle']['colors']['ColorModule.gradient'].update(min_gradient={}),
            lambda m:m['item_prototypes'][-1]['consumable']['ember']['particle']['vectors'].update({'renderer.m_Pivot':[0,0]}),
            lambda m:m['item_prototypes'][-1]['consumable']['ember']['particle']['materials'][0]['shader'].pop('compiled_data'),
            lambda m:m['item_prototypes'][-1]['consumable']['ember']['particle']['materials'][0]['textures'][0].pop('scale'),
            lambda m:m['item_prototypes'][-1]['consumable']['ember']['particle'].pop('duration'),
        ]
        for index,mutate in enumerate(mutations):
            m=copy.deepcopy(original);mutate(m)
            with self.subTest(case=index),self.assertRaises(ValueError):
                validate_clothing(m['clothing'],m);validate_item_prototypes(m['item_prototypes'],m)
