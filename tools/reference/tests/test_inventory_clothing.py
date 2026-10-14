import copy
import hashlib
import os
import json
import unittest
from pathlib import Path
from tools.reference.inventory import recover_inventory, validate_inventory
from tools.reference.library.reader import LibraryReader
from tools.reference.library.types import LibraryReceipt
from tools.reference.source import LibrarySource

INPUTS = [
    ('equiprecenter', 19, 'Tab'), ('removeSunglasses', 30, 'X'),
    ('inventory1', 31, 'Alpha1'), ('inventory2', 32, 'Alpha2'),
    ('inventory3', 33, 'Alpha3'), ('inventory4', 34, 'Alpha4'),
    ('smoke', 29, 'CapsLock'),
]

class InventoryClothingTests(unittest.TestCase):
    def setUp(self):
        self.manifest = json.loads(Path('tests/common/reference.json').read_text())

    def test_recovers_checked_inventory_from_accepted_source(self):
        root = os.environ.get('REFERENCE_LIBRARY_ROOT')
        self.assertIsNotNone(root, 'Set REFERENCE_LIBRARY_ROOT to the accepted source library')
        path = Path(root) / 'manifest.json'
        content = path.read_bytes()
        metadata = json.loads(content)
        receipt = LibraryReceipt(path, hashlib.sha256(content).hexdigest(),
                                 metadata['catalog_sha256'], metadata['source_manifest_sha256'], ())
        source = LibrarySource(LibraryReader(receipt))
        recovered = recover_inventory(source, self.manifest)
        self.assertEqual(recovered, self.manifest['inventory'])
        self.assertEqual(recovered['inputs'][-1], {'action': 'smoke', 'index': 29, 'key': 'CapsLock'})
        self.assertEqual([(i['action'], i['index'], i['key']) for i in recovered['inputs']], INPUTS)

    def test_checked_inventory_contract(self):
        r = self.manifest['inventory']
        self.assertEqual(r['source'], {'file': 'level1', 'path_id': 80125})
        self.assertEqual(r['slot_count'], 4)
        self.assertEqual(r['initial_slots'], [None] * 4)
        self.assertEqual(r['selected_slot'], 0)
        self.assertEqual(r['swap_gain'], 5.)
        self.assertEqual(r['starter_groups'], [False] * 3)
        self.assertEqual(r['equip_hold_seconds'], .25)
        self.assertEqual(r['hand_interpolation_factor'], 20.)
        self.assertEqual(r['hand_interpolation_seconds'], 1.)
        self.assertEqual(r['right_hand_anchor'], {'file': 'level1', 'path_id': 23118})
        self.assertEqual([(i['action'], i['index'], i['key']) for i in r['inputs']], INPUTS)
        validate_inventory(r, self.manifest)

    def test_rejects_changed_startup_anchor_and_controls(self):
        cases = [
            ('slot_count', 3), ('initial_slots', [None] * 3),
            ('initial_slots', [{'file': 'level1', 'path_id': 1}, None, None, None]),
            ('selected_slot', 1), ('swap_gain', float('nan')),
            ('starter_groups', [True, False, False]), ('equip_hold_seconds', .3),
            ('hand_interpolation_factor', 19.), ('hand_interpolation_seconds', 2.),
            ('source', {'file': 'level1', 'path_id': 1}),
            ('right_hand_anchor', {'file': 'level1', 'path_id': 23119}),
        ]
        for field, value in cases:
            r = copy.deepcopy(self.manifest['inventory']); r[field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                validate_inventory(r, self.manifest)
        for field, value in [('action', 'hum'), ('index', 28), ('key', 'H')]:
            r = copy.deepcopy(self.manifest['inventory']); r['inputs'][-1][field] = value
            with self.subTest(control=field), self.assertRaises(ValueError):
                validate_inventory(r, self.manifest)

    def test_requires_complete_contract_and_evidence(self):
        for field in self.manifest['inventory']:
            r = copy.deepcopy(self.manifest['inventory']); del r[field]
            with self.subTest(missing=field), self.assertRaises(ValueError):
                validate_inventory(r, self.manifest)
        for field, value in [('assembly_sha256', '0' * 64), ('methods', [])]:
            r = copy.deepcopy(self.manifest['inventory']); r['evidence'][field] = value
            with self.subTest(evidence=field), self.assertRaises(ValueError):
                validate_inventory(r, self.manifest)
        with self.assertRaises(ValueError): validate_inventory(None, self.manifest)
