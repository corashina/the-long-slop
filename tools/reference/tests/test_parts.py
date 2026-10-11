"""Installed general part and slot contracts."""
import json
import unittest
from pathlib import Path
from tools.reference.source import Source,key
STEAM=Path('C:/Program Files (x86)/Steam/steamapps/common/The Long Drive')
MANIFEST=Path('web/assets/reference/manifest.json')
@unittest.skipUnless(STEAM.exists() and MANIFEST.exists(),'requires installed export')
class InstalledParts(unittest.TestCase):
 @classmethod
 def setUpClass(cls):
  cls.source=Source(STEAM);cls.m=json.loads(MANIFEST.read_text())
 def test_parts_slots_covers_and_source_seat_limits(self):
  m=self.m;s=self.source
  main=s.read(s.get('level1',64919))
  self.assertEqual(m['part_scan'],{'source':{'file':'level1','path_id':64919},'mask':main['scanLayer']['m_Bits'],'radius':main['defScanRadius']})
  parts={p['source']['path_id']:p for p in m['parts']}
  slots={p['source']['path_id']:p for p in m['part_slots']}
  self.assertEqual(len(parts),24);self.assertEqual(len(slots),27)
  for identity,p in parts.items():
   raw=s.read(s.get('level1',identity));pickup=s.read(s.resolve(s.get('level1',identity),raw['p']))
   self.assertEqual(p['main_types'],raw['maintipus']);self.assertEqual(p['part_types'],raw['tipus'])
   usable=pickup['usable']['m_PathID']
   self.assertEqual(p['usable'],next((c['item'] for c in m['mechanisms'] if c['source']['path_id']==usable),None))
   self.assertEqual(p['scan_radius'],raw['scanRadius'])
   self.assertEqual(p['cover_slot'],raw['cover_CantCraftIfCovered']['m_PathID'] or None)
   self.assertEqual(p['linear_damping'],pickup['RBs']['drag']);self.assertEqual(p['angular_damping'],pickup['RBs']['angularDrag'])
   for k in ['enableIfMounted','enableIfDismounted','disableIfMounted','disableIfDismounted']:self.assertEqual(raw[k],[])
  for identity,slot in slots.items():
   reader=s.get('level1',identity);raw=s.read(reader)
   part=s.resolve(reader,raw['part']);self.assertEqual(slot['part'],None if part is None else parts[part.path_id]['item'])
   for out,field in [('cant_craft','cantCraft'),('cant_uncraft','cantUnCraft'),('disable_colliders_on_craft','disableCollidersOnCraft'),('fixed_detach','fixDt')]:self.assertEqual(slot[out],bool(raw[field]))
   self.assertEqual(slot['main_types'],raw['maintipus']);self.assertEqual(slot['part_types'],raw['tipus'])
  for rim,cover in [(65509,65545),(66181,66804),(76917,77392),(67245,67799)]:
   wheel=next(p for p in m['wheel_parts'] if p['source']['path_id']==rim)
   self.assertEqual(wheel['cover_slot'],cover);self.assertIsNotNone(slots[cover]['part'])
  for identity in [73311,77177]:self.assertIsNone(slots[identity]['part'])
  for seat in m['seats']:
   item=next(i for i in m['items'] if i['id']==seat['item']);node=m['nodes'][item['node']-1]
   transform=s.get(node['source']['file'],node['source']['path_id'])
   classes=[s.class_name(c) for c in s.components(transform)[1] if c.type.name=='MonoBehaviour']
   self.assertNotIn('pickupable',classes);self.assertNotIn('partscript',classes);self.assertIsNone(item['pickup'])
