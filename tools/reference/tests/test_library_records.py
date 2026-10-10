import hashlib
import json
import tempfile
import struct
import unittest
from pathlib import Path
from types import SimpleNamespace as N
from unittest.mock import Mock, patch
from dataclasses import asdict

from tools.reference.source import Source, normalize_arrays
from tools.reference.library.types import Catalog, ObjectId, ObjectRecord
from tools.reference.library.records import resolve_dependencies, write_record, validate_graph, validate_transform_parents, verify_catalog


def node(kind, name='', children=()):
    return N(m_Type=kind, m_Name=name, m_Children=list(children), m_MetaFlag=0)


def record(identity, kind='Material'):
    return ObjectRecord(identity, kind, 21, 0, 32, '0' * 64)


def source(tree, data, owner='owner', external='target'):
    reader = N(path_id=1, assets_file=N(source_container_path=owner, externals=[N(path=external)]), type=N(name='Material'), class_id=21, byte_size=32)
    src = N(record_tree=lambda reader: tree, read_record=lambda reader: data,
            container_path=lambda reader: reader.assets_file.source_container_path,
            serialized_files={owner: None, external: None}, objects={(owner, 1): reader, (external, 2): N()},
            root=Path('.'), resource_streams=())
    return src, reader


class RecordsTests(unittest.TestCase):
    def test_external_pointer_uses_owner_table(self):
        tree = node('Material', children=[node('PPtr<Material>', 'real'), node('Fake', 'fake', [node('int', 'm_FileID'), node('long long', 'm_PathID')])])
        src, reader = source(tree, {'real': {'m_FileID': 1, 'm_PathID': 2}, 'fake': {'m_FileID': 9, 'm_PathID': 42}})
        self.assertEqual(resolve_dependencies(src, reader, src.read_record(reader)), (ObjectId('target', 2),))
        reader.assets_file.externals[0].path = 'missing'
        with self.assertRaisesRegex(ValueError, 'real'):
            resolve_dependencies(src, reader, src.read_record(reader))

    def test_null_and_cyclic_asset_pointers(self):
        src, reader = source(node('Material', children=[node('PPtr<Material>', 'null')]), {'null': {'m_FileID': 99, 'm_PathID': 0}})
        self.assertEqual(resolve_dependencies(src, reader, src.read_record(reader)), ())
        a, b = ObjectId('owner', 1), ObjectId('target', 2)
        validate_graph(Catalog({a: record(a), b: record(b)}, {}, {a: (b,), b: (a,)}))

    def test_missing_pointer_and_transform_cycle_fail(self):
        a, b = ObjectId('owner', 1), ObjectId('owner', 2)
        with self.assertRaisesRegex(ValueError, 'Missing'):
            validate_graph(Catalog({a: record(a)}, {}, {a: (b,)}))
        with self.assertRaisesRegex(ValueError, 'cycle'):
            validate_transform_parents({a: b, b: a})
        with self.assertRaisesRegex(ValueError, 'range'):
            validate_graph(Catalog({a: ObjectRecord(a, 'Material', 21, -1, 2, 'x')}, {}, {a: ()}))

    def test_binary_and_nonfinite_fields_preserved(self):
        data = {'pairs': [(1, 'a'), (1, 'b')], 'blob': b'\x00\xff', 'numbers': [float('nan'), float('inf'), -float('inf')]}
        pair = node('pair', 'data', [node('int', 'first'), node('string', 'second')])
        pairs = node('map', 'pairs', [node('Array', children=[node('int', 'size'), pair])])
        array = node('vector', 'array', [node('Array', children=[node('int', 'size'), node('int', 'data')])])
        data['array'] = (1, 2, 3)
        src, reader = source(node('Material', children=[pairs, array]), data)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            receipt = write_record(src, reader, root)
            raw = (root / receipt.file).read_bytes()
            self.assertEqual(receipt.sha256, hashlib.sha256(raw).hexdigest())
            saved = json.loads(raw)
            self.assertEqual(saved['data']['pairs'], [{'$pair': [1, 'a']}, {'$pair': [1, 'b']}])
            self.assertEqual(saved['data']['array'], [1, 2, 3])
            blob = saved['data']['blob']['$binary']
            self.assertEqual((root / blob['file']).read_bytes(), b'\x00\xff')
            self.assertEqual(saved['data']['numbers'], [{'$float': 'nan'}, {'$float': '+inf'}, {'$float': '-inf'}])
            self.assertEqual(write_record(src, reader, root), receipt)

    def test_custom_header_and_complete_read(self):
        src = Source.__new__(Source)
        src.evidence = {}
        src.record_tree = Mock(return_value=node('MonoBehaviour'))
        header = N(m_Script=N(file_id=0, path_id=2), m_GameObject=N(file_id=0, path_id=3), m_Enabled=1, m_Name='name')
        reader = N(path_id=1, assets_file=N(source_container_path='owner'), type=N(name='MonoBehaviour'), byte_size=32,
                   parse_monobehaviour_head=Mock(return_value=header), parse_as_dict=Mock(return_value={'m_Script': {'m_FileID': 0, 'm_PathID': 9}, 'm_GameObject': {'m_FileID': 0, 'm_PathID': 3}, 'm_Enabled': 1, 'm_Name': 'name'}))
        with self.assertRaisesRegex(ValueError, 'Header mismatch'):
            src.read_record(reader)
        reader.parse_as_dict.side_effect = ValueError('Expected to read 32 bytes, but only read 28 bytes')
        with self.assertRaisesRegex(ValueError, 'only read 28'):
            src.read_record(reader)
        self.assertTrue(reader.parse_as_dict.call_args.kwargs['check_read'])

    def test_stream_ranges_and_uncached_read(self):
        tree = node('Texture2D', children=[node('StreamingInfo', 'stream')])
        with tempfile.TemporaryDirectory() as directory:
            src, reader = source(tree, {'stream': {'path': 'bytes.resS', 'offset': 1, 'size': 2}})
            src.root = Path(directory)
            src.resource_streams = ('bytes.resS',)
            (src.root / 'TheLongDrive_Data').mkdir()
            (src.root / 'TheLongDrive_Data/bytes.resS').write_bytes(b'abc')
            self.assertEqual(resolve_dependencies(src, reader, src.read_record(reader)), ())
            src.read_record(reader)['stream']['size'] = 3
            with self.assertRaisesRegex(ValueError, 'range'):
                resolve_dependencies(src, reader, src.read_record(reader))
        src = Source.__new__(Source)
        src.data_cache = {}
        src.record_tree = Mock(return_value=node('Material'))
        reader = N(path_id=1, assets_file=N(source_container_path='owner'), type=N(name='Material'), parse_as_dict=Mock(return_value={'name': 'a'}))
        self.assertEqual(src.read_record(reader), {'name': 'a'})
        self.assertEqual(src.data_cache, {})

    def test_verified_record_hashes_and_read_only_acceptance(self):
        src, reader = source(node('Material'), {'blob': b'abc'})
        identities = tuple(ObjectId('owner', number) for number in range(1, 66))
        catalog = Catalog({identity: record(identity) for identity in identities}, {'Material': 65}, {identity: () for identity in identities})
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            saved = []
            for identity in identities:
                reader.path_id = identity.path_id
                saved.append(write_record(src, reader, root))
            data = {'objects': [{'source': asdict(identity), 'dependencies': []} for identity in identities]}
            raw = json.dumps(data).encode()
            catalog_path = root / 'catalog.json'
            catalog_path.write_bytes(raw)
            receipt = {'schema_version': 1, 'source_manifest_sha256': 'source', 'catalog_sha256': hashlib.sha256(raw).hexdigest(),
                       'record_count': 65, 'records': [asdict(item) for item in saved], 'type_trees': [asdict(item) for item in src.record_schema_cache.values()], 'stream_ranges': [],
                       'checks': {'complete_byte_consumption': True, 'graph_closed': True, 'transform_parent_cycles': 0, 'reciprocal_hierarchy': True, 'reciprocal_component_ownership': True}}
            (root / 'manifest.json').write_text(json.dumps(receipt))
            before = {p: (p.stat().st_mtime_ns, p.read_bytes()) for p in root.rglob('*') if p.is_file()}
            with patch('tools.reference.library.records.inventory_source', return_value=catalog), patch('tools.reference.library.inventory.catalog_data', return_value=data):
                self.assertEqual(verify_catalog(src, N(manifest_sha256='source'), catalog_path, root), catalog)
                after = {p: (p.stat().st_mtime_ns, p.read_bytes()) for p in root.rglob('*') if p.is_file()}
                self.assertEqual(before, after)
                (root / 'owner/65.0.bin').write_bytes(b'changed')
                with self.assertRaisesRegex(ValueError, 'hash mismatch'):
                    verify_catalog(src, N(manifest_sha256='source'), catalog_path, root)

    def test_generated_pair_type_collision(self):
        custom = node('pair', 'custom', [node('vector', 'm', [node('Array', children=[node('int', 'size'), node('int', 'data')])])])
        native = node('pair', 'native', [node('int', 'first'), node('int', 'second')])
        normalize_arrays(node('Root', children=[custom, native]))
        self.assertNotEqual(custom.m_Type, 'pair')
        self.assertEqual(native.m_Type, 'pair')
        from UnityPy.helpers import TypeTreeHelper
        from UnityPy.streams import EndianBinaryReader
        TypeTreeHelper.read_typetree_boost = None
        decoded = TypeTreeHelper.read_typetree(custom, EndianBinaryReader(struct.pack('<Iii', 2, 7, 8), endian='<'), byte_size=12)
        self.assertEqual(decoded, {'m': [7, 8]})
        decoded = TypeTreeHelper.read_typetree(native, EndianBinaryReader(struct.pack('<ii', 7, 8), endian='<'), byte_size=8)
        self.assertEqual(decoded, (7, 8))


if __name__ == '__main__':
    unittest.main()
