import hashlib
import json
import os
import unittest
from pathlib import Path
from tools.reference.library.reader import LibraryReader
from tools.reference.library.types import LibraryReceipt
from tools.reference.source import LibrarySource
from tools.reference.clothing import recover_clothing, validate_clothing

class PlayerReferenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        path = Path(os.environ['REFERENCE_LIBRARY_ROOT']) / 'manifest.json'
        raw = path.read_bytes(); metadata = json.loads(raw)
        cls.source = LibrarySource(LibraryReader(LibraryReceipt(path, hashlib.sha256(raw).hexdigest(),
            metadata['catalog_sha256'], metadata['source_manifest_sha256'], ())))
        cls.output=Path(os.environ.get('TLD_REFERENCE_OUTPUT','web/assets/reference'))
        cls.manifest = json.loads(Path('tests/common/reference.json').read_bytes())

    def test_checked_source_has_intact_characters_and_pose(self):
        clothing = recover_clothing(self.source, self.manifest, self.output)
        self.assertEqual([c['source']['path_id'] for c in clothing['characters']], [71215, 67763])
        self.assertEqual([len(c['renderers']) for c in clothing['characters']], [4, 18])
        self.assertEqual([len(c['skins']) for c in clothing['characters']], [2, 16])
        self.assertEqual([len(c['segments']) for c in clothing['characters']], [8, 8])
        self.assertTrue(all(len(s['joints']) == 53 for c in clothing['characters'] for s in c['skins']))
        self.assertEqual(len(clothing['pose']['ik_chains']), 5)
        validate_clothing(clothing, self.manifest)

    def test_source_pose_and_appearance_values(self):
        from tools.reference.player_pose import recover_player_pose, WALK_IDS, SIT_IDS, ANCHOR_FIELDS, FOLLOW_NODES, FOLLOW_TARGETS
        pose=recover_player_pose(self.source,self.manifest)
        nodes={n['id']:n for n in self.manifest['nodes']}
        ids=lambda values:[nodes[n]['source']['path_id'] for n in values]
        self.assertEqual([ids(p) for p in pose['walk_paths']],list(WALK_IDS))
        self.assertEqual([ids(p) for p in pose['sit_poses']],list(SIT_IDS))
        self.assertEqual(pose['walk_offsets'],[.5,0.])
        self.assertEqual(ids(pose['wearable_anchors']),[26477,28766,35477,36615,39162])
        self.assertEqual(ids(pose['follow']['nodes']),FOLLOW_NODES)
        self.assertEqual(ids(pose['follow']['targets']),FOLLOW_TARGETS)
        self.assertEqual({k:v['source']['path_id'] for k,v in pose['anchors'].items()},ANCHOR_FIELDS)
        from tools.reference.gltf import converted_transform
        for anchor in pose['anchors'].values():
            self.assertEqual(anchor['transform'],converted_transform(self.source.read(self.source.get(anchor['source']['file'],anchor['source']['path_id']))))
        for c in self.manifest['clothing']['characters']:
            self.assertFalse(c['segments'][0]['enabled'])
            self.assertFalse(c['segments'][4]['disable_able'])
            self.assertFalse(c['segments'][4]['default_enabled'])

    def test_source_rejects_missing_or_unequal_links(self):
        import copy
        from unittest.mock import patch
        from tools.reference.source import key
        from tools.reference.player_pose import recover_player_pose
        cases=[(71215,lambda d:d['ch']['segments'][0]['materialSlotNums'].append(0)),
            (71215,lambda d:d['ch']['renderers'].pop()),
            (58557,lambda d:d['m_Bones'].__setitem__(0,{'m_FileID':0,'m_PathID':39843})),
            (80125,lambda d:d['LeftLegWalkWayPoints'].reverse()),
            (74950,lambda d:d['Target'].pop()),
            (74950,lambda d:d['Target'].reverse())]
        for pid,mutate in cases:
            reader=self.source.get('level1',pid);changed=copy.deepcopy(self.source.read(reader));mutate(changed)
            read=self.source.read
            with self.subTest(source=pid),patch.object(self.source,'read',side_effect=lambda r:changed if key(r)==key(reader) else read(r)):
                with self.assertRaises(ValueError):
                    if pid in (80125,74950):recover_player_pose(self.source,self.manifest)
                    else:recover_clothing(self.source,self.manifest,self.output)

    def test_malformed_contracts_and_exported_binary(self):
        import copy,struct,tempfile,shutil
        from tools.reference.clothing import validate_player_export
        from tools.reference.gltf import validate_gltf
        original=self.manifest['clothing']
        for mutate in [lambda c:c['characters'][0]['skins'][0]['joints'].reverse(),
                lambda c:c['characters'][0]['skins'][0]['inverse_bind'].update(count=52),
                lambda c:c['characters'][0]['segments'][0]['renderers'][0].update(slot=99),
                lambda c:c['pose']['follow']['targets'].reverse(),
                lambda c:c['pose']['anchors']['RH']['transform']['translation'].__setitem__(0,float('nan'))]:
            changed=copy.deepcopy(original);mutate(changed)
            with self.assertRaises(ValueError):validate_clothing(changed,self.manifest)
        validate_player_export(original,self.manifest,self.output)
        with tempfile.TemporaryDirectory() as temporary:
            directory=Path(temporary);path=directory/'player.gltf'
            document=json.loads((self.output/'player.gltf').read_bytes())
            binary=(self.output/'player.bin').read_bytes()
            (directory/'player.bin').write_bytes(binary)
            for mutate in [lambda d:d['skins'][0]['joints'].__setitem__(0,999999),
                    lambda d:d['skins'][0]['joints'].reverse(),
                    lambda d:d['accessors'][d['skins'][0]['inverseBindMatrices']].update(count=52),
                    lambda d:d['accessors'][d['skins'][0]['inverseBindMatrices']].update(byteOffset=999999999)]:
                changed=copy.deepcopy(document);mutate(changed);path.write_text(json.dumps(changed))
                with self.assertRaises((ValueError,IndexError)):validate_player_export(original,self.manifest,directory)
            path.write_text(json.dumps(document))
            node=next(n for n in document['nodes'] if 'skin' in n)
            primitive=document['meshes'][node['mesh']]['primitives'][0]
            for field,value in [('JOINTS_0',65535),('WEIGHTS_0',-1.)]:
                accessor=document['accessors'][primitive['attributes'][field]];view=document['bufferViews'][accessor['bufferView']]
                offset=view.get('byteOffset',0)+accessor.get('byteOffset',0)
                changed=bytearray(binary);struct.pack_into('<'+('H' if field=='JOINTS_0' else 'f'),changed,offset,value)
                (directory/'player.bin').write_bytes(changed)
                with self.assertRaises(ValueError):validate_gltf(path)

    def test_renderer_contract_rejects_invalid_state_and_shape_in_both_paths(self):
        import copy
        from tools.reference.clothing import validate_player_export
        original=self.manifest['clothing']
        validators=[('clothing',lambda c:validate_clothing(c,self.manifest)),
            ('export',lambda c:validate_player_export(c,self.manifest,self.output))]
        for _,validate in validators:validate(original)
        # The first five cases reproduce review I1. Others cover the remaining predicates and types.
        cases=[(0,'state','cast_shadows',99),(0,'state','bounds_extents',[-1,0,0]),
            (0,None,'update_when_offscreen',False),(0,None,'skinned_motion_vectors',False),
            (2,None,'quality',2),
            (1,'state','cast_shadows',0),(1,'state','receive_shadows',True),
            (0,'state','cast_shadows',1),(0,'state','receive_shadows',False),
            (2,'state','cast_shadows',0),(2,'state','receive_shadows',False),
            (0,'state','reflection_probe_usage',1),(2,'state','reflection_probe_usage',0),
            (0,'state','motion_vectors',0),(0,'state','sorting_layer',1),(0,'state','sorting_order',1),
            (0,None,'skin',None),(2,None,'skin',0),(0,None,'quality',1),
            (2,None,'update_when_offscreen',False),(2,None,'skinned_motion_vectors',False),
            (0,'state','bounds_center',[0,0]),(0,'state','bounds_extents',[0,0,0,0]),
            (0,'state','bounds_center',[0,'0',0]),(0,'state','bounds_extents',[0,False,0]),
            (0,'state','bounds_center',[0,float('inf'),0]),(0,'state','bounds_extents',None),
            (0,'state','bounds_center',{'x':0,'y':0,'z':0}),
            (0,'state','cast_shadows',False),(0,'state','receive_shadows',1),
            (0,'state','reflection_probe_usage',False),(0,'state','motion_vectors',1.0),
            (0,'state','sorting_layer',0.0),(0,'state','sorting_order',False),
            (0,None,'enabled',1),(0,None,'update_when_offscreen',1),
            (0,None,'skinned_motion_vectors',1),(0,None,'quality',False),
            (0,None,'skin',False),(0,None,'node',True),(0,None,'gltf_mesh',0.0),
            (0,None,'materials',{}),(0,None,'state',None),
            (0,None,'source',{'file':'level1','path_id':58557,'extra':0}),
            (0,'state','source',{'file':'level1','path_id':58557.0})]
        for index,section,field,value in cases:
            changed=copy.deepcopy(original);r=changed['characters'][0]['renderers'][index]
            (r if section is None else r[section])[field]=value
            for name,validate in validators:
                with self.subTest(path=name,index=index,section=section,field=field,value=value):
                    with self.assertRaises(ValueError):validate(changed)
        renderer=original['characters'][0]['renderers'][0]
        for section in (None,'state'):
            fields=renderer if section is None else renderer[section]
            for field in [*fields,'unknown']:
                changed=copy.deepcopy(original);r=changed['characters'][0]['renderers'][0]
                record=r if section is None else r[section]
                if field=='unknown':record[field]=0
                else:del record[field]
                for name,validate in validators:
                    with self.subTest(path=name,section=section,field=field):
                        with self.assertRaises(ValueError):validate(changed)
