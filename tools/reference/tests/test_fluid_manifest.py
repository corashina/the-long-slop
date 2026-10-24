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


class CapPresentationTests(unittest.TestCase):
    def test_f15_cap_name_reads_name_text_word(self):
        from tools.reference.caps import cap_presentation, parse_default_words
        code = ('\tprivate void FillDefWW()\r\n\t{\r\n\t\tdefww = new string[3];\r\n'
            '\t\tdefww[0] = "Continue";\r\n\t\tdefww[1] = "Oil Tank";\r\n\t\tdefww[2] = "Coolant";\r\n\t}\r\n')
        words = parse_default_words(code)
        # Source cap record: hasName=1, nameString="Coolant Tank", nameText=coolant.
        cap = {'hasName': 1, 'nameString': 'Coolant Tank', 'nameText': 2, 'noText': 0}
        self.assertEqual(cap_presentation(cap, words), {'has_name': True, 'name': 'Coolant', 'no_text': False})
        with self.assertRaises(ValueError):
            parse_default_words(code.replace('\t\tdefww[1] = "Oil Tank";\r\n', ''))
