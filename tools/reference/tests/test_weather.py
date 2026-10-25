import unittest
from tools.reference.weather import STORM_COLORS, STORM_FIELDS, validate_weather_export
from tools.reference.survival import snake

SOURCE = {'file': 'sharedassets1.assets', 'path_id': 158276}
GRADIENT = {'mode': 0, 'colors': [{'time': 0., 'value': [1., 1., 1., 1.]}], 'alphas': [{'time': 0., 'value': 1.}]}
MANIFEST = {'lighting': {'textures': [{'id': 7}]}}


def particle(role):
    return {'role': role, 'capacity': 100, 'rate_over_time': 10., 'rate_over_distance': 0.,
        'scalars': {'InitialModule.maxNumParticles': 100}, 'curves': {'InitialModule.startLifetime': {}},
        'materials': [{'textures': [{'texture': 7}]}]}


def weather():
    storm = {'source': SOURCE, 'biome': 0, **{snake(k): 1. for k in STORM_FIELDS}, **{snake(k): [1., 1., 1., 1.] for k in STORM_COLORS},
        'day_color1': GRADIENT, 'day_color2': GRADIENT, 'wind_lm': 1, 'current_force': [0., 0., 0.], 'next_force': [0., 0., 0.],
        'distance_lerp': 0., 'size_range': [.5, 1.5], 'speed_range': [3., 12.], 'wind_time_range': [.1, 1.5],
        'wind_scan_seconds': 1., 'wind_bounds_range': [1., 30.]}
    return {'source': {'file': 'level1', 'path_id': 1}, 'prefab_id': 0, 'min_interval_seconds': 1200., 'max_interval_seconds': 5400., 'spawn_radius': 15000.,
        'storm': storm, 'evidence': {'assembly_sha256': 'a', 'methods': ['m']}, 'particles': [particle('storm_big'), particle('storm_small')]}


class WeatherExportTests(unittest.TestCase):
    def test_valid_record(self):
        validate_weather_export(weather(), MANIFEST)

    def test_invalid_records(self):
        def broken(change):
            record = weather(); change(record); return record
        cases = {
            'missing field': broken(lambda w: w['storm'].pop('wind_lm')),
            'non-finite value': broken(lambda w: w['storm'].update(radius=float('nan'))),
            'reversed schedule': broken(lambda w: w.update(min_interval_seconds=6000.)),
            'zero schedule': broken(lambda w: w.update(min_interval_seconds=0.)),
            'negative prefab id': broken(lambda w: w.update(prefab_id=-1)),
            'zero radius': broken(lambda w: w.update(spawn_radius=0.)),
            'missing particle': broken(lambda w: w['particles'].pop()),
            'wrong particle role': broken(lambda w: w['particles'][1].update(role='urine')),
            'missing texture': broken(lambda w: w['particles'][0]['materials'][0]['textures'][0].update(texture=8)),
        }
        for name, record in cases.items():
            with self.subTest(name), self.assertRaises(ValueError):
                validate_weather_export(record, MANIFEST)


if __name__ == '__main__':
    unittest.main()
