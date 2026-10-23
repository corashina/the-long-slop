"""Run the coordinator-approved selected runtime derivation with existing validators."""
import hashlib
import json
from pathlib import Path
import sys

sys.path.insert(0, str(Path.cwd()))
from tools.reference.export import derive_reference, verify_reference
from tools.reference.library.reader import LibraryReader
from tools.reference.library.types import LibraryReceipt
from tools.reference.runtime_package import package_id, verify_package

manifest_path = Path('C:/Users/Corashina/Documents/Projects/the-long-drive-wasm/web/assets/library/manifest.json')
raw = manifest_path.read_bytes()
header = json.loads(raw)
receipt = LibraryReceipt(
    manifest_path=manifest_path,
    manifest_sha256=hashlib.sha256(raw).hexdigest(),
    catalog_sha256=header['catalog_sha256'],
    source_manifest_sha256=header['source_manifest_sha256'],
    exports=(),
)
print(f'Accepted library manifest SHA256: {receipt.manifest_sha256}', flush=True)
reader = LibraryReader(receipt)
print('LibraryReader provenance checks passed', flush=True)
output = Path('web/assets/reference')
derive_reference(reader.snapshot, receipt, output)
print('Selected runtime derivation passed', flush=True)
verify_reference(output, receipt)
print('Selected runtime verification passed', flush=True)
manifest = json.loads((output / 'manifest.json').read_bytes())
verify_package(manifest, output.parent / 'runtime' / package_id(manifest))
print(f'Runtime package verification passed: {package_id(manifest)}', flush=True)
