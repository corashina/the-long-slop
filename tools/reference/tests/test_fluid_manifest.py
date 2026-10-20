import unittest
from tools.reference.manifest import fluid_entries


class FluidManifestTests(unittest.TestCase):
    def test_order_and_retained_zero(self):
        source = {'maxC': 2., 'fluids': [
            {'type': 3, 'amount': 1.}, {'type': 0, 'amount': 0.}, {'type': 1, 'amount': .25}]}
        self.assertEqual(fluid_entries(source), [
            {'fluid_type': 3, 'amount': 1.}, {'fluid_type': 0, 'amount': 0.}, {'fluid_type': 1, 'amount': .25}])

    def test_invalid_initial_data(self):
        for source in [
            {'maxC': 2., 'fluids': [{'type': 3, 'amount': 1.}, {'type': 3, 'amount': 0.}]},
            {'maxC': 1., 'fluids': [{'type': 3, 'amount': 2.}]},
            {'maxC': 1., 'fluids': [{'type': 3, 'amount': 1.0001}]},
            {'maxC': 0., 'fluids': []},
            {'maxC': 2., 'fluids': [{'type': 6, 'amount': 0.}]},
            {'maxC': 2., 'fluids': [{'type': 1, 'amount': -.25}]},
            {'maxC': 2., 'fluids': [{'type': 1, 'amount': float('nan')}]},
        ]:
            with self.subTest(source=source), self.assertRaises(ValueError):
                fluid_entries(source)
