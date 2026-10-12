import copy
import json
import unittest
from pathlib import Path
from tools.reference.weather import recover_weather, validate_weather_export

class WeatherTests(unittest.TestCase):
    def setUp(self):
        self.manifest=json.loads(Path('tests/common/reference.json').read_text())

    def test_recovers_active_player_and_source_values(self):
        r=self.manifest['weather']
        self.assertEqual([r[k] for k in ('min_interval_seconds','max_interval_seconds','spawn_radius')],[1200,5400,15000])
        self.assertEqual(r['storm']['source'],dict(file='sharedassets1.assets',path_id=158276))
        self.assertTrue(r['shelters']); self.assertTrue(r['particles'])
        validate_weather_export(r,self.manifest)

    def test_rejects_missing_links_and_nonfinite_rates(self):
        for mutate in [lambda r:r['shelters'][0].update(root=999999),lambda r:r['particles'][0].update(capacity=0),lambda r:r['particles'][0]['materials'][0]['textures'][0].update(texture=999999),lambda r:r['particles'][0].update(rate_over_time=float('inf'))]:
            r=copy.deepcopy(self.manifest['weather']); mutate(r)
            with self.assertRaises(ValueError):validate_weather_export(r,self.manifest)
