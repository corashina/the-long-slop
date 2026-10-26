import unittest
from tools.reference.manifest import burn_colors
from tools.reference.vehicles import smoke_rate
from tools.reference.particles import validate_exhaust


def color(r, g, b, a):
    return {'r': r, 'g': g, 'b': b, 'a': a}


class ExhaustExportTests(unittest.TestCase):
    def test_burn_colors_keep_fluid_order(self):
        main = {'fluids': [{'color': color(9, 9, 9, 9), 'burnColor': color(i, i + .5, 0, 1)} for i in range(6)]}
        self.assertEqual(burn_colors(main), [[i, i + .5, 0, 1] for i in range(6)])
        with self.assertRaises(ValueError):
            burn_colors({'fluids': main['fluids'][:5]})
        main['fluids'][2]['burnColor']['g'] = float('nan')
        with self.assertRaises(ValueError):
            burn_colors(main)

    def test_smoke_rate(self):
        self.assertEqual(smoke_rate({'defSmokeRate': 20.}), 20.)
        for rate in (-1., float('nan'), float('inf')):
            with self.subTest(rate=rate), self.assertRaises(ValueError):
                smoke_rate({'defSmokeRate': rate})

    def test_unsupported_module_rejected(self):
        flags = {'InitialModule': True, 'ShapeModule': True, 'SizeModule': True, 'NoiseModule': True, 'TrailModule': False}
        with self.assertRaisesRegex(ValueError, r"Unsupported exhaust particle modules exhaust_0_0: \['NoiseModule'\]"):
            validate_exhaust({'role': 'exhaust_0_0', 'module_flags': flags})

    def test_texture_sheet_must_play_grid_over_lifetime(self):
        flags = {'InitialModule': True, 'UVModule': True}
        scalars = {'UVModule.mode': 0, 'UVModule.animationType': 1, 'UVModule.timeMode': 0}
        with self.assertRaisesRegex(ValueError, 'Unsupported exhaust texture sheet'):
            validate_exhaust({'role': 'exhaust_0_0', 'module_flags': flags, 'scalars': scalars})


if __name__ == '__main__':
    unittest.main()
