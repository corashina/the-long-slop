"""Installed push and throw contracts."""
import json
import unittest
from pathlib import Path
from tools.reference.source import Source
STEAM = Path('C:/Program Files (x86)/Steam/steamapps/common/The Long Drive')
MANIFEST = Path('web/assets/reference/manifest.json')

@unittest.skipUnless(STEAM.exists() and MANIFEST.exists(), 'requires installed export')
class InstalledActions(unittest.TestCase):
    def test_push_grip_and_force_parameters(self):
        s = Source(STEAM)
        m = json.loads(MANIFEST.read_text(encoding='utf-8'))
        p = s.read(s.get('level1',80125))
        main = s.read(s.get('level1',64919))
        time = s.read(s.get('globalgamemanagers',8))
        for out, field in [('throw_minimum','minThrowForce'), ('throw_gain','throwForceGain'),
                ('grip_force','pushgripForce'), ('grip_speed','FspeedM'), ('grip_distance','pushGripDistance')]:
            self.assertEqual(m['player'][out],p[field])
        self.assertEqual(m['player']['throw_multiplier'],1000.)
        self.assertEqual(m['player']['source_force_interval'],time['Fixed Timestep'])
        self.assertEqual(m['player']['push_force'],main['pushForce'])
        self.assertEqual(m['player']['airborne_push_force'],main['selfNotGroundedPushForce'])
        self.assertEqual(len(m['pushables']),1)
        push = m['pushables'][0]
        raw = s.read(s.get('level1',72438))
        self.assertEqual(push['source']['path_id'],72438)
        self.assertEqual(push['mask'],163841)
        self.assertEqual(push['modifier'],raw['modifier'])
        for out, field in [('only_projected','onlyProjected'), ('both','both'), ('push_even_if_same_parent','pushEvenIfSameParent')]:
            self.assertEqual(push[out],bool(raw[field]))
        grip = push['grip']
        self.assertEqual(grip['source']['path_id'],70719)
        raw = s.read(s.get('level1',70719))
        self.assertEqual(raw['car']['m_PathID'],77839)
        hand = m['nodes'][grip['hand_node']-1]
        self.assertEqual(hand['source']['path_id'],raw['HT']['m_PathID'])
        self.assertEqual(grip['in_use'],bool(raw['inuse']))
        self.assertEqual(len(grip['colliders']),1)
        collider = next(c for c in m['colliders'] if c['id']==grip['colliders'][0])
        self.assertEqual(collider['source']['path_id'],54965)
        self.assertTrue(collider['sensor'])
        self.assertTrue(collider['membership'] & push['mask'])
        binding = next(b for b in m['action_bindings'] if b['collider']==collider['id'])
        self.assertEqual(binding['push_owner'],push['owner'])
        self.assertTrue(s.evidence['level1:72438']['complete_byte_consumption'])
        self.assertTrue(s.evidence['level1:70719']['complete_byte_consumption'])
        self.assertEqual(s.get('globalgamemanagers',8).type.name,'TimeManager')
