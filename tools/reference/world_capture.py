"""Prove retained world-capture inputs match the current accepted library."""
import hashlib
import json
import re

from .library.archive import _check_path, validate_paths
from .library.reader import LibraryReader, checked

MANAGED_TEXT_SHA256 = '22ddc62b91954205d90ff513232f39becebbc58a71b718685075f91f64b1f3ee'
EVIDENCE_FIELDS = {'records_root', 'managed_analysis', 'coverage', 'files'}


def _file_map(records):
    for record in records:
        if (set(record) != {'file', 'bytes', 'sha256'} or not isinstance(record['file'], str)
                or type(record['bytes']) is not int or record['bytes'] < 0
                or not isinstance(record['sha256'], str)
                or re.fullmatch('[0-9a-f]{64}', record['sha256']) is None):
            raise ValueError('Invalid world capture file receipt')
    validate_paths([record['file'] for record in records])
    return {record['file']: record for record in records}


def _payload_files(manifest):
    coverage = manifest['coverage']
    if coverage['file'] != 'coverage.json':
        raise ValueError('World capture coverage must name coverage.json')
    files = _file_map(manifest['files'])
    if files.pop('coverage.json', None) != coverage:
        raise ValueError('World capture coverage receipt differs from file map')
    return files


def _managed_text(manifest):
    files = _file_map(manifest['managed_analysis']['files'])
    return files['managed/Assembly-CSharp.cs']


def verify_capture_inputs(reader: LibraryReader) -> dict:
    from . import world_probe
    world_probe.verify(world_probe.PROBE_ROOT)
    root = world_probe.PROBE_ROOT
    _check_path(root, root.resolve())
    original_path = root / 'capture-library-manifest.json'
    _check_path(original_path, root.resolve())
    original_bytes = original_path.read_bytes()
    sample_path = root / 'samples.json'
    _check_path(sample_path, root.resolve())
    sample = json.loads(sample_path.read_bytes())
    original_hash = hashlib.sha256(original_bytes).hexdigest()
    if original_hash != world_probe.LIBRARY_SHA256 or original_hash != sample['library']['manifest_sha256']:
        raise ValueError('World capture manifest hash changed')
    # The pinned manifest is evidence only. Never follow its storage paths.
    original = json.loads(original_bytes)
    _check_path(reader.receipt.manifest_path, reader.root.resolve())
    current_bytes = reader.receipt.manifest_path.read_bytes()
    if hashlib.sha256(current_bytes).hexdigest() != reader.receipt.manifest_sha256:
        raise ValueError('World current manifest hash changed')
    current = json.loads(current_bytes)
    if any(current[name] != getattr(reader.receipt, name)
           for name in ('source_manifest_sha256', 'catalog_sha256')):
        raise ValueError('World current verified provenance changed')
    if (set(original) != set(current) or original['schema_version'] != 1
            or current['schema_version'] != 1):
        raise ValueError('World capture manifest schema or key set changed')
    for name in original.keys() - EVIDENCE_FIELDS:
        if original[name] != current[name]:
            raise ValueError(f'World capture input changed: {name}')
    if _payload_files(original) != _payload_files(current):
        raise ValueError('World capture payload receipts changed')
    if (original['source_manifest_sha256'] != reader.receipt.source_manifest_sha256
            or original['catalog_sha256'] != reader.receipt.catalog_sha256
            or original['records_manifest'] != current['records_manifest']):
        raise ValueError('World capture source, catalog, or typed records changed')
    checked(reader.records_root, original['records_manifest'])
    original_text = _managed_text(original)
    current_text = _managed_text(current)
    if original_text != current_text or current_text['sha256'] != MANAGED_TEXT_SHA256:
        raise ValueError('World capture managed text receipt changed')
    checked(reader.records_root, current_text)
    captured = dict(manifest_sha256=original_hash, catalog_sha256=original['catalog_sha256'],
                    source_manifest_sha256=original['source_manifest_sha256'])
    if sample['library'] != captured:
        raise ValueError('World capture sample provenance changed')
    return dict(schema_version=1, **captured)
