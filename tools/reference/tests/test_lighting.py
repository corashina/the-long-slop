import copy
import json
import unittest
from pathlib import Path
from tools.reference.lighting import recover_cycle, recover_lighting, validate_lighting_export
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

    def test_sun_flare_chain_is_exported(self):
        lighting=self.recover()
        flares=lighting['cycle']['sun_flares']
        self.assertEqual(len(flares),3)
        for flare in flares:
            self.assertEqual(flare['flare']['source'],{'file':'sharedassets1.assets','path_id':4516})
            atlas=next(t for t in lighting['textures'] if t['id']==flare['atlas'])
            self.assertEqual(atlas['source'],{'file':'sharedassets1.assets','path_id':744})
            self.assertTrue((self.output/Path(flare['flare']['path']).name).is_file())
            self.assertTrue((self.output/Path(atlas['path']).name).is_file())

    def test_acceptance_rejects_missing_cabin_input_and_incomplete_door_links(self):
        output=Path('web/assets/reference')
        original=json.loads((output/'manifest.json').read_text())
        validate_lighting_export(original,output)
        mutations=[lambda c:c.pop('cabin_input'),lambda c:c.update(cabin_input=None)]
        for field in ('door_slots','door_usable_sources','door_nodes'):
            mutations.extend([lambda c,f=field:c['cabin_input'].update({f:[]}),
                lambda c,f=field:c['cabin_input'][f].pop(),
                lambda c,f=field:c['cabin_input'][f].reverse()])
        mutations.append(lambda c:c['cabin_input'].update(modes=[0,2,1]))
        for mutate in mutations:
            with self.subTest(mutation=mutate):
                altered=copy.deepcopy(original)
                cabin=next(c for c in altered['lighting']['controllers'] if c['kind']=='cabin')
                mutate(cabin)
                with self.assertRaisesRegex(ValueError,'cabin input|door links|modes'):
                    validate_lighting_export(altered,output)

    def test_acceptance_rejects_missing_flare_payload_and_atlas(self):
        output=Path('web/assets/reference')
        original=json.loads((output/'manifest.json').read_text())
        mutations=[lambda m:m['lighting']['cycle']['sun_flares'].clear(),
            lambda m:m['lighting']['cycle']['sun_flares'][0]['flare'].update(sha256='0'*64),
            lambda m:m['lighting']['textures'].__setitem__(slice(None),[t for t in m['lighting']['textures'] if t['source']['path_id']!=744]),
            lambda m:m['lighting']['cycle']['sun_flares'][0].update(atlas=999999)]
        for mutate in mutations:
            altered=copy.deepcopy(original);mutate(altered)
            with self.assertRaisesRegex(ValueError,'flare|resource hash'):
                validate_lighting_export(altered,output)

    def test_visual_temperature_arrays_match_checked_scene_values(self):
        source_fields=self.source.read(self.source.get('level1',64920))
        cycle=recover_cycle(self.source,self.source.get('level1',64920),self.output)
        for source_field,field in [('tempDays','temp_days'),('tempNights','temp_nights'),('tempAddNoons','temp_add_noons')]:
            self.assertEqual(cycle[field],source_fields[source_field])
            self.assertEqual(len(cycle[field]),3)

    def test_acceptance_rejects_incomplete_and_nonfinite_visual_temperature_arrays(self):
        output=Path('web/assets/reference');original=json.loads((output/'manifest.json').read_text())
        for field in ('temp_days','temp_nights','temp_add_noons'):
            for values in (None,[],[1.,2.],[1.,2.,3.,4.],[float('nan'),1.,2.]):
                altered=copy.deepcopy(original);altered['lighting']['cycle'][field]=values
                with self.assertRaisesRegex(ValueError,field):validate_lighting_export(altered,output)

    def recover(self):
        s=self.source
        roots=[s.get('level1',21400),s.transform_for(s.get('level1',17687))]
        return recover_lighting(s,roots,self.output)
