"""Critical installed source contracts for mechanical controls."""
import json
import unittest
from pathlib import Path
from tools.reference.source import Source, key
from tools.reference.mechanisms import recover_mechanisms

STEAM = Path('C:/Program Files (x86)/Steam/steamapps/common/The Long Drive')
MANIFEST = Path('web/assets/reference/manifest.json')

@unittest.skipUnless(STEAM.exists() and MANIFEST.exists(), 'requires installed Steam build and export')
class InstalledMechanisms(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.source = Source(STEAM)
        cls.manifest = json.loads(MANIFEST.read_text())

    def test_source_rotation_and_window_tracks(self):
        source=self.source;manifest=self.manifest
        ids={tuple(n['source'].values()):n['id'] for n in manifest['nodes']}
        def node_of(reader):
            go=source.resolve(reader,source.read(reader)['m_GameObject'])
            return ids[key(source.transform_for(go))]
        def pointer_node(owner,pointer):
            reader=source.resolve(owner,pointer)
            return ids[key(reader)] if reader.type.name=='Transform' else node_of(reader)
        components={}
        required=[64783,64777,64811,64916,64797,75324,77425,71545,71291,66558,73523,67612,70647,79151,74645,76322]
        for identity in required:
            reader=source.get('level1',identity)
            components[node_of(reader)]={'usablescript':[reader]}
        recovered=recover_mechanisms(source,manifest['nodes'],components,pointer_node,node_of,source.read(source.get('level1',64919)))
        exported={m['source']['path_id']:m for m in manifest['mechanisms']}
        self.assertEqual(len(recovered),len(required))
        for mechanism in recovered:
            actual=exported[mechanism['source']['path_id']]
            for field in ('rotation','slide','driven_nodes','drag','invert_value','invert_child'):
                self.assertEqual(mechanism[field],actual[field],(mechanism['source'],field))
        for usable,animation,glass,handle in [(70647,74542,36038,30430),(79151,67740,32978,25344),(74645,79477,32807,26450),(76322,75606,31582,38178)]:
            raw=source.read(source.get('level1',usable));window=exported[usable]
            self.assertEqual([raw[k]['m_PathID'] for k in ('TDoor','TstartPos','TendPos')],[0,0,0])
            self.assertEqual(window['slide']['animation_source']['path_id'],animation)
            self.assertEqual(window['slide']['paths'][0]['node'],ids['level1',glass])
            self.assertEqual(window['slide']['rotations'][0]['node'],ids['level1',handle])
            self.assertEqual(window['slide']['rotations'][0]['angle_degrees'],-810.)
            self.assertEqual(window['slide']['axes'],[False,False,False,False,True,True])
            self.assertEqual(window['slide']['multiplier'],[1.,1.,10.])
        for door,window in [(70795,70647),(76010,79151),(69957,74645),(66662,76322)]:
            self.assertEqual(exported[door]['child'],exported[window]['item'])
        # Child mesh and solid parent boxes must share the driven body.
        for collider_source,usable in [(50803,64811),(50799,64916),(50795,64797)]:
            collider=next(c for c in manifest['colliders'] if c['source']['path_id']==collider_source)
            body=next(b for b in manifest['bodies'] if b['id']==collider['body'])
            self.assertIn(body['node'],exported[usable]['driven_nodes'])
            binding=next(b for b in manifest['action_bindings'] if b['collider']==collider['id'])
            self.assertEqual(binding['use_owner'],exported[usable]['item'])

        spout=exported[67388]
        faucet=next(o for o in manifest['openings'] if o['source']['path_id']==64723)
        anchor=next(a for a in manifest['anchors'] if a['id']==faucet['anchor'])
        tank=next(i for i in manifest['items'] if i['id']==faucet['tank'])
        self.assertEqual(anchor['body'],tank['body'])
        self.assertNotIn(anchor['node'],spout['driven_nodes'])
        cap_node=manifest['nodes'][anchor['node']-1]
        spout_node=manifest['nodes'][spout['driven_nodes'][0]-1]
        self.assertEqual(cap_node['parent'],spout_node['parent'])

    def test_source_switches_and_lamps(self):
        m=self.manifest
        switches={x['source']['path_id']:x for x in m['mechanisms'] if x.get('turn')}
        self.assertEqual(switches[70733]['turn']['action'], {'kind':'cycle_headlights'})
        self.assertEqual(switches[68075]['turn']['action'], {'kind':'toggle_handbrake'})
        self.assertEqual(switches[69669]['turn']['max_state'],2)
        lights=m['lighting']
        self.assertEqual(len(lights['groups']),8)
        self.assertEqual({id for id,s in switches.items() if s['turn']['action']['kind']=='toggle_home_light'},{76065,68886,75508,67484,72019})
        for identity in [76065,68886,75508,67484,72019]:
            item=switches[identity]['item']
            self.assertTrue(any(b['use_owner']==item and m['colliders'][b['collider']-1]['enabled'] and m['colliders'][b['collider']-1]['membership'] & m['player']['action_masks']['use_mask'] for b in m['action_bindings']))
        self.assertEqual(lights['door_limit'],self.source.read(self.source.get('level1',73157))['iUsableV1To1Limit'])
        self.assertEqual(len(lights['door_items']),4)
        lamps=[l for g in lights['groups'] for st in g['states'] for l in st['lamps']]
        self.assertEqual({l['source']['path_id'] for l in lamps},{56907,57015,56880,56971,56933,56912,56909,56913,56908,56911,56910})
        for lamp in lamps:
            raw=self.source.read(self.source.get(lamp['source']['file'],lamp['source']['path_id']))
            self.assertEqual(lamp['range'],raw['m_Range'])
            self.assertEqual(lamp['outer_degrees'],raw['m_SpotAngle'])
