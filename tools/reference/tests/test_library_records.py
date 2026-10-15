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
    def test_missing_pointer_and_transform_cycle_fail(self):
        a, b = ObjectId('owner', 1), ObjectId('owner', 2)
        with self.assertRaisesRegex(ValueError, 'Missing'):
            validate_graph(Catalog({a: record(a)}, {}, {a: (b,)}))
        with self.assertRaisesRegex(ValueError, 'cycle'):
            validate_transform_parents({a: b, b: a})
        with self.assertRaisesRegex(ValueError, 'range'):
            validate_graph(Catalog({a: ObjectRecord(a, 'Material', 21, -1, 2, 'x')}, {}, {a: ()}))

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
                       'record_count': 65, 'records': [asdict(item) for item in saved], 'type_trees': [asdict(item) for item in src.record_schema_cache[str(root.resolve())].values()], 'stream_ranges': [],
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
                (root / 'owner/65.0.bin').write_bytes(before[root / 'owner/65.0.bin'][1])
                receipt['records'][0]['file'] = '../escape.json'
                (root / 'manifest.json').write_text(json.dumps(receipt))
                with self.assertRaisesRegex(ValueError, 'Unsafe Windows path'):
                    verify_catalog(src, N(manifest_sha256='source'), catalog_path, root)
                receipt['records'][0]['file'] = saved[0].file
                (root / 'manifest.json').write_text(json.dumps(receipt))
                (root / saved[0].file).unlink()
                with self.assertRaises(FileNotFoundError):
                    verify_catalog(src, N(manifest_sha256='source'), catalog_path, root)

if __name__ == '__main__':
    unittest.main()
