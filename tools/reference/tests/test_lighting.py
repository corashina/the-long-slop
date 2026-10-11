import copy
import json
import unittest
from tempfile import TemporaryDirectory
from tools.reference.export import NATIVE_ENGINE_SHA256, native_engine_evidence
from pathlib import Path
from tools.reference.lighting import recover_cycle, recover_lighting, validate_lighting_export
from tools.reference.source import Source

@unittest.skipUnless(Path(r'C:/Program Files (x86)/Steam/steamapps/common/The Long Drive/TheLongDrive_Data/level1').is_file(),'Requires installed Steam source')
class LightingTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.source=Source(Path(r'C:/Program Files (x86)/Steam/steamapps/common/The Long Drive'))
        cls.output=Path('.cache/lighting-test');cls.output.mkdir(parents=True,exist_ok=True)

    def test_decoded_mips_preserve_source_levels_and_alpha(self):
        from tools.reference.mips import decoded_mips,validate_mips
        import texture2ddecoder
        from PIL import Image
        for file,pid in [('sharedassets1.assets',744),('sharedassets1.assets',1278)]:
            obj=self.source.object(self.source.get(file,pid));data=decoded_mips(obj)
            validate_mips(data,obj.m_Width,obj.m_Height,obj.m_MipCount)
            base=obj.image.transpose(Image.Transpose.FLIP_TOP_BOTTOM).convert('RGBA').tobytes()
            self.assertEqual(data[16:16+len(base)],base)
            self.assertRaises(ValueError,validate_mips,data[:-1],obj.m_Width,obj.m_Height,obj.m_MipCount)
            if pid==744:
                self.assertEqual(obj.m_MipCount,11)
                offset=16
                for level in range(obj.m_MipCount):
                    width,height=max(1,obj.m_Width>>level),max(1,obj.m_Height>>level)
                    blocks=texture2ddecoder.unpack_unity_crunch(obj.get_image_data(),level)
                    decoded=texture2ddecoder.decode_bc1(blocks,width,height)
                    expected=Image.frombytes('RGBA',(width,height),decoded,'raw','BGRA').tobytes()
                    self.assertEqual(data[offset:offset+len(expected)],expected,f'Source mip {level}')
                    offset+=len(expected)
            if pid==1278:
                alpha=data[19::4]
                self.assertEqual(alpha,obj.get_image_data())
                self.assertLess(min(alpha),max(alpha))

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

    def test_high_beam_instrument_uses_checked_source_links(self):
        output=Path('web/assets/reference')
        original=json.loads((output/'manifest.json').read_text())
        lamp=original['lighting']['high_beam']
        self.assertEqual(lamp['source'],{'file':'level1','path_id':66369})
        self.assertEqual(lamp['on']['source'],{'file':'sharedassets1.assets','path_id':325})
        self.assertEqual(lamp['off']['source'],{'file':'sharedassets1.assets','path_id':324})
        self.assertEqual(lamp['signal'],'headlight2')
        mutations=[lambda l:l.pop('high_beam'),lambda l:l['high_beam'].update(node=original['vehicle']['root']),
                   lambda l:l['high_beam'].update(on=l['high_beam']['off'])]
        for mutate in mutations:
            altered=copy.deepcopy(original);mutate(altered['lighting'])
            with self.assertRaises(ValueError):validate_lighting_export(altered,output)

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
            component=self.source.read(self.source.get(flare['source']['file'],flare['source']['path_id']))
            self.assertEqual(flare['color'],[component['m_Color'][channel] for channel in ('r','g','b','a')])
            self.assertEqual(flare['fade_speed'],component['m_FadeSpeed'])
            self.assertEqual(flare['ignore_layers'],component['m_IgnoreLayers']['m_Bits'])
            self.assertEqual(flare['directional'],bool(component['m_Directional']))

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

    def test_acceptance_rejects_changed_typed_flare_values(self):
        output=Path('web/assets/reference');original=json.loads((output/'manifest.json').read_text())
        mutations=[lambda f:f.update(elements=[]),lambda f:f['elements'][0].update(size=42),
            lambda f:f.update(texture_layout=0),lambda f:f.update(fade_speed=-1),
            lambda f:f.update(color=[float('nan'),0,0,1]),lambda f:f.update(directional=False)]
        for mutate in mutations:
            altered=copy.deepcopy(original);mutate(altered['lighting']['cycle']['sun_flares'][0])
            with self.assertRaisesRegex(ValueError,'typed flare'):
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

    def test_fog_modifier_retains_source_draw_distance_setting(self):
        cycle=recover_cycle(self.source,self.source.get('level1',64920),self.output)
        self.assertEqual(cycle['fog_modifier'],3.2959184646606445)
        evidence=self.source.lighting_fog_evidence
        self.assertEqual(evidence['source'],{'file':'level0','path_id':16865})
        self.assertEqual(evidence['draw_distance'],2500.)
        self.assertEqual(evidence['minimum'],{'draw_distance':500.,'fog_modifier':3.5})
        self.assertEqual(evidence['maximum'],{'draw_distance':25000.,'fog_modifier':1.})

    def test_acceptance_rejects_missing_or_invalid_fog_modifier(self):
        output=Path('web/assets/reference');original=json.loads((output/'manifest.json').read_text())
        for value in (None,-1.,float('nan'),float('inf')):
            altered=copy.deepcopy(original);altered['lighting']['cycle']['fog_modifier']=value
            with self.assertRaisesRegex(ValueError,'fog_modifier'):validate_lighting_export(altered,output)
        altered=copy.deepcopy(original);del altered['lighting']['cycle']['fog_modifier']
        with self.assertRaisesRegex(ValueError,'fog_modifier'):validate_lighting_export(altered,output)

    def test_native_engine_evidence_rejects_an_unchecked_dll(self):
        evidence=native_engine_evidence(Path(r'C:/Program Files (x86)/Steam/steamapps/common/The Long Drive'))
        self.assertEqual(evidence['sha256'],NATIVE_ENGINE_SHA256)
        with TemporaryDirectory() as directory:
            root=Path(directory);(root/'UnityPlayer.dll').write_bytes(b'unchecked engine')
            with self.assertRaisesRegex(ValueError,'checked native cloud noise source'):native_engine_evidence(root)

    def recover(self):
        s=self.source
        roots=[s.get('level1',21400),s.transform_for(s.get('level1',17687))]
        return recover_lighting(s,roots,self.output)

class LightingExportAcceptanceTests(unittest.TestCase):
    def test_failed_engine_check_removes_existing_manifest(self):
        from unittest.mock import MagicMock, patch
        from tools.reference.export import export_reference
        source=MagicMock()
        source.get.side_effect=lambda file,pid:(file,pid)
        source.transform_for.side_effect=lambda reader:reader
        source.resolve.return_value=('level1',123)
        source.hierarchy.return_value=[{'path':str(i)} for i in range(697)]
        records={('level1',71403):{'m_GameObject':{'m_PathID':123}},
            ('level1',64896):{'FirstHome':{'m_PathID':21400}},
            ('level1',64898):{'StartCar02':{'m_PathID':206}}}
        source.read.side_effect=lambda reader:records[reader]
        with TemporaryDirectory() as directory:
            base=Path(directory);root=base/'steamapps'/'common'/'The Long Drive'
            root.mkdir(parents=True)
            (root.parents[1]/'appmanifest_1017180.acf').write_text('"AppState" { "appid" "1017180" "buildid" "11140890" }')
            output=base/'export';output.mkdir()
            manifest=output/'manifest.json';engine=root/'UnityPlayer.dll'
            for payload,error in ((b'unchecked engine',ValueError),(None,FileNotFoundError)):
                with self.subTest(engine_present=payload is not None):
                    manifest.write_text('{"accepted":true}')
                    if payload is None:engine.unlink(missing_ok=True)
                    else:engine.write_bytes(payload)
                    with patch('tools.reference.source.Source',return_value=source), patch('tools.reference.conditions.prepare_conditions'):
                        with self.assertRaises(error):export_reference(root,output)
                    self.assertFalse(manifest.exists(),'Failed export retained its old acceptance manifest')
