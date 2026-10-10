import unittest
from pathlib import Path
from tools.reference.lighting import recover_cycle, recover_lighting
from tools.reference.source import Source

@unittest.skipUnless(Path(r'C:/Program Files (x86)/Steam/steamapps/common/The Long Drive/TheLongDrive_Data/level1').is_file(),'Requires installed Steam source')
class LightingTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.source=Source(Path(r'C:/Program Files (x86)/Steam/steamapps/common/The Long Drive'))
        cls.output=Path('.cache/lighting-test');cls.output.mkdir(parents=True,exist_ok=True)

    def test_cycle_uses_scene_values_and_nested_transition_durations(self):
        cycle=recover_cycle(self.source,self.source.get('level1',64920),self.output)
        self.assertEqual([cycle[k] for k in ('day_seconds','night_seconds','dawn_seconds','dusk_seconds')],[1200,300,240,180])
        self.assertEqual(cycle['startup_offset_seconds'],240)
        self.assertEqual(cycle['day_seconds']+cycle['night_seconds'],1500)

    def test_runtime_intensity_overrides_serialized_light_intensity(self):
        lighting=self.recover()
        right=next(c for c in lighting['controllers'] if c['source']['path_id']==72934)
        high=right['states'][2]
        self.assertEqual(high['intensity'],1)
        light=next(l for l in lighting['lights'] if l['id']==high['lights'][0])
        self.assertEqual(light['serialized_intensity'],1.25)

    def test_selected_scene_lamps_have_checked_links(self):
        lighting=self.recover();controls=lighting['controllers']
        house=[c for c in controls if c['kind']=='house']
        self.assertEqual(len(house),5)
        self.assertEqual(sum(len(c['states']) for c in house),6)
        self.assertEqual(sum(c['kind']=='headlight' for c in controls),2)
        self.assertEqual(sum(c['kind']=='cabin' for c in controls),1)
        self.assertEqual(sum(c['kind']=='rear' for c in controls),1)
        for c in house:
            self.assertTrue(c['aim_colliders']);self.assertFalse(c['startup_on'])
            for state in c['states']:
                self.assertTrue(state['lights']);self.assertTrue(state['renderers'])
                self.assertIsNotNone(state['on_material']);self.assertIsNotNone(state['off_material'])

    def recover(self):
        s=self.source
        roots=[s.get('level1',21400),s.transform_for(s.get('level1',17687))]
        return recover_lighting(s,roots,self.output)
