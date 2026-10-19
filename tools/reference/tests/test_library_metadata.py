import hashlib
import json
import os
import struct
import subprocess
import tempfile
import unittest
from dataclasses import asdict
from pathlib import Path

from tools.reference.library.managed import SCRIPT, verify_managed_export
from tools.reference.library.types import FileRecord

ROOT = Path(__file__).resolve().parents[3]
TEST_TEMP = ROOT / '.cache/managed-tests'
FIXTURE = r'''
using System.Collections.Generic;
using System.Runtime.InteropServices;

namespace Unity
{
    public static class ThrowStub
    {
        public static void ThrowNotSupportedException()
        {
            throw new System.PlatformNotSupportedException();
        }
    }
}

public abstract class AbstractFixture
{
    public abstract int AbstractMethod();
}

public sealed class DefaultFixture { }

public sealed class MetadataFixture : AbstractFixture
{
    private MetadataFixture() { }
    public int Value { get; set; }
    public override int AbstractMethod() => 7;
    public static IEnumerable<int> Iterate()
    {
        yield return 3;
        yield return 5;
    }
    public static int Helper(int value) => value + 1;
    public static int Broken() => Helper(42);
    public static IEnumerable<int> StubReturningSequence()
    {
        Unity.ThrowStub.ThrowNotSupportedException();
        return null;
    }
    [DllImport("kernel32.dll", EntryPoint = "GetCurrentProcessId")]
    public static extern int NativeCall();
}
'''


def run_pwsh(*arguments):
    TEST_TEMP.mkdir(parents=True, exist_ok=True)
    env = os.environ.copy()
    env['TEMP'] = str(TEST_TEMP)
    env['TMP'] = str(TEST_TEMP)
    return subprocess.run(['pwsh', '-NoProfile', *map(str, arguments)], capture_output=True,
                          text=True, env=env)


def compile_fixture(folder):
    assembly = folder / 'Fixture.dll'
    command = f"Add-Type -CompilerOptions '/optimize+' -TypeDefinition @'\n{FIXTURE}\n'@ -OutputAssembly '{assembly.as_posix()}'"
    result = run_pwsh('-Command', command)
    if result.returncode:
        raise AssertionError(result.stderr)
    return assembly


def index_fixture(folder, assembly, name='index'):
    output = folder / name
    output.mkdir()
    result = run_pwsh('-File', SCRIPT, '-Mode', 'Index', '-AssemblyPath', folder,
                      '-OutputPath', output)
    if result.returncode:
        raise AssertionError(result.stderr)
    return json.loads((output / 'index.json').read_bytes())


def decompile_fixture(assembly, output):
    output.mkdir()
    result = run_pwsh('-File', SCRIPT, '-AssemblyPath', assembly, '-OutputPath', output)
    return result


def corrupt_method_header(assembly, rva):
    raw = bytearray(assembly.read_bytes())
    pe = struct.unpack_from('<I', raw, 0x3C)[0]
    section_count = struct.unpack_from('<H', raw, pe + 6)[0]
    optional_size = struct.unpack_from('<H', raw, pe + 20)[0]
    section = pe + 24 + optional_size
    file_offset = None
    for _ in range(section_count):
        virtual_size, virtual_address, raw_size, raw_pointer = struct.unpack_from('<IIII', raw, section + 8)
        if virtual_address <= rva < virtual_address + max(virtual_size, raw_size):
            file_offset = raw_pointer + rva - virtual_address
            break
        section += 40
    if file_offset is None:
        raise AssertionError(f'No PE section contains RVA 0x{rva:x}')
    raw[file_offset] = 0  # PEReader must reject this body.
    assembly.write_bytes(raw)


def patch_stub_tail(assembly, rva, opcode):
    raw = bytearray(assembly.read_bytes())
    pe = struct.unpack_from('<I', raw, 0x3C)[0]
    section_count = struct.unpack_from('<H', raw, pe + 6)[0]
    optional_size = struct.unpack_from('<H', raw, pe + 20)[0]
    section = pe + 24 + optional_size
    for _ in range(section_count):
        virtual_size, virtual_address, raw_size, raw_pointer = struct.unpack_from('<IIII', raw, section + 8)
        if virtual_address <= rva < virtual_address + max(virtual_size, raw_size):
            file_offset = raw_pointer + rva - virtual_address
            break
        section += 40
    else:
        raise AssertionError(f'No PE section contains RVA 0x{rva:x}')
    header = raw[file_offset]
    if header & 3 == 2:
        il = file_offset + 1
        il_size = header >> 2
    elif header & 3 == 3:
        header_word = struct.unpack_from('<H', raw, file_offset)[0]
        il = file_offset + ((header_word >> 12) * 4)
        il_size = struct.unpack_from('<I', raw, file_offset + 4)[0]
    else:
        raise AssertionError(f'Unexpected method header 0x{header:02x}')
    if il_size != 7:
        raise AssertionError(f'Expected seven IL bytes, got {il_size}')
    if raw[il] != 0x28 or raw[il + 5] != 0x14 or raw[il + 6] != 0x2A:
        raise AssertionError(f'Unexpected throw-stub caller IL: {raw[il:il + 7].hex()}')
    raw[il + 5] = opcode
    assembly.write_bytes(raw)


class MetadataTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        TEST_TEMP.mkdir(parents=True, exist_ok=True)

    def test_real_ilspy_covers_hidden_methods_and_declarations(self):
        with tempfile.TemporaryDirectory(dir=TEST_TEMP) as directory:
            root = Path(directory)
            assembly = compile_fixture(root)
            index = index_fixture(root, assembly)
            result = decompile_fixture(assembly, root / 'managed')
            self.assertEqual(result.returncode, 0, result.stderr)
            receipt = json.loads((root / 'managed/Fixture.decompilation.json').read_bytes())
            self.assertTrue(receipt['complete'], receipt['diagnostics'])
            self.assertEqual(receipt['decompiler']['version'], '11.1.0.9782')
            coverage = json.loads((root / 'managed/Fixture.coverage.json').read_bytes())
            methods = coverage['methods']
            indexed_tokens = [row['token'] for row in index['methods']]
            covered_tokens = [row['token'] for row in methods]
            self.assertEqual(len(indexed_tokens), len(set(indexed_tokens)))
            self.assertEqual(len(covered_tokens), len(set(covered_tokens)))
            self.assertEqual(set(covered_tokens), set(indexed_tokens))
            by_name = {}
            for method in methods:
                by_name.setdefault((method['type'], method['name']), []).append(method)

            required = [
                ('MetadataFixture', '.ctor'),
                ('MetadataFixture', 'get_Value'),
                ('MetadataFixture', 'set_Value'),
                ('MetadataFixture', 'Iterate'),
                ('MetadataFixture+<Iterate>d__', 'MoveNext'),
            ]
            for type_prefix, name in required:
                matches = [row for (type_name, method_name), rows in by_name.items()
                           if type_name.startswith(type_prefix) and method_name == name for row in rows]
                self.assertTrue(matches, f'Missing MethodDef {type_prefix}::{name}')
                self.assertTrue(all(row['status'] == 'decompiled' for row in matches), matches)

            declarations = {
                ('AbstractFixture', 'AbstractMethod'): 'abstract',
                ('MetadataFixture', 'NativeCall'): 'pinvoke',
            }
            for (type_prefix, name), reason in declarations.items():
                matches = [row for row in methods if row['type'].startswith(type_prefix) and row['name'] == name]
                self.assertEqual(len(matches), 1)
                self.assertEqual(matches[0]['status'], 'declaration_only')
                self.assertEqual(matches[0]['reason'], reason)
            self.assertEqual(len(methods), len(index['methods']))

    def test_malformed_method_keeps_output_provenance_and_gap(self):
        with tempfile.TemporaryDirectory(dir=TEST_TEMP) as directory:
            root = Path(directory)
            assembly = compile_fixture(root)
            index = index_fixture(root, assembly)
            broken = next(row for row in index['methods'] if row['name'] == 'Broken')
            corrupt_method_header(assembly, broken['rva'])
            original = assembly.read_bytes()
            result = decompile_fixture(assembly, root / 'managed')
            self.assertNotEqual(result.returncode, 0)
            receipt = json.loads((root / 'managed/Fixture.decompilation.json').read_bytes())
            self.assertFalse(receipt['complete'])
            self.assertEqual(receipt['input']['sha256'], hashlib.sha256(original).hexdigest())
            output = (root / 'managed/Fixture.cs').read_bytes()
            self.assertEqual(receipt['output']['bytes'], len(output))
            self.assertEqual(receipt['output']['sha256'], hashlib.sha256(output).hexdigest())
            coverage = json.loads((root / 'managed/Fixture.coverage.json').read_bytes())
            failed = next(row for row in coverage['methods'] if row['token'] == broken['token'])
            self.assertEqual(failed['status'], 'failed')
            self.assertTrue(failed['diagnostics'])
            self.assertEqual(assembly.read_bytes(), original)

    def test_verified_throw_stub_normalization_requires_exact_il(self):
        for opcode, expect_proof in ((0x16, True), (0x17, False)):
            with self.subTest(opcode=opcode), tempfile.TemporaryDirectory(dir=TEST_TEMP) as directory:
                root = Path(directory)
                assembly = compile_fixture(root)
                initial = index_fixture(root, assembly)
                caller = next(row for row in initial['methods'] if row['name'] == 'StubReturningSequence')
                patch_stub_tail(assembly, caller['rva'], opcode)
                index = index_fixture(root, assembly, 'proof-index')
                proofs = [row for row in index['verified_throw_stubs']
                          if row['proof']['method_token'] == caller['token']]
                self.assertEqual(len(proofs), int(expect_proof), index['verified_throw_stubs'])

                result = decompile_fixture(assembly, root / 'managed')
                self.assertEqual(result.returncode, 0, result.stderr)
                receipt = json.loads((root / 'managed/Fixture.decompilation.json').read_bytes())
                self.assertTrue(receipt['complete'], receipt['diagnostics'])
                coverage = json.loads((root / 'managed/Fixture.coverage.json').read_bytes())
                row = next(method for method in coverage['methods'] if method['token'] == caller['token'])
                if expect_proof:
                    proof = proofs[0]['proof']
                    self.assertEqual(proofs[0]['assembly'], 'TheLongDrive_Data/Managed/Fixture.dll')
                    indexed_caller = next(method for method in index['methods'] if method['token'] == caller['token'])
                    self.assertEqual(indexed_caller['il_sha256'], proof['caller_il_sha256'])
                    self.assertEqual(proof['caller_il_hex'][-4:], '162a')
                    self.assertEqual(proof['caller_il_sha256'], hashlib.sha256(bytes.fromhex(proof['caller_il_hex'])).hexdigest())
                    resolution = row['resolution']
                    self.assertEqual(resolution['kind'], 'verified_nonreturning_stub')
                    self.assertEqual(resolution['caller_il_hex'], proof['caller_il_hex'])
                    self.assertEqual(resolution['callee_il_sha256'], proof['callee_il_sha256'])
                    self.assertEqual(row['resolved_diagnostics'], ['IL_0007: Expected Obj, but got I4'])
                    self.assertEqual(row['status'], 'decompiled')
                    self.assertEqual(row['diagnostics'], [])
                    code = (root / 'managed/Fixture.cs').read_text()
                    self.assertIn('StubReturningSequence', code)
                    self.assertIn('return null;', code)
                    from tools.reference.library.managed import _assembly_coverage
                    proof_map = {caller['token']: proof}
                    entry, _, _, resolved = _assembly_coverage(
                        root, index['assemblies'][0], index['methods'], proof_map)
                    self.assertEqual(entry['resolved_warnings'], 1)
                    self.assertEqual(len(resolved), 1)

                    coverage_path = root / 'managed/Fixture.coverage.json'
                    coverage['methods'] = [dict(item) for item in coverage['methods']]
                    covered = next(item for item in coverage['methods'] if item['token'] == caller['token'])
                    del covered['resolution']
                    del covered['resolved_diagnostics']
                    coverage_path.write_text(json.dumps(coverage))
                    receipt_path = root / 'managed/Fixture.decompilation.json'
                    receipt['coverage'] = asdict(_record(coverage_path, 'managed/Fixture.coverage.json'))
                    receipt_path.write_text(json.dumps(receipt))
                    with self.assertRaisesRegex(ValueError, 'Missing resolved managed warning proof'):
                        _assembly_coverage(root, index['assemblies'][0], index['methods'], proof_map)
                else:
                    self.assertIsNone(row.get('resolution'))
                    self.assertEqual(row['status'], 'warning')
                    self.assertTrue(any('Expected Obj, but got I4' in value for value in row['diagnostics']),
                                    row['diagnostics'])

    def test_verify_rejects_method_token_and_source_changes(self):
        for change in ('omitted_token', 'duplicate_token', 'source_bytes', 'output_bytes', 'missing_coverage'):
            with self.subTest(change=change), tempfile.TemporaryDirectory(dir=TEST_TEMP) as directory:
                root = Path(directory)
                source = root / 'source'
                managed = root / 'output/managed'
                source_file = source / 'TheLongDrive_Data/Managed/Fixture.dll'
                source_file.parent.mkdir(parents=True)
                source_file.write_bytes(b'fixture assembly')
                output_root = managed.parent
                managed.mkdir(parents=True)
                source_record = FileRecord('TheLongDrive_Data/Managed/Fixture.dll', source_file.stat().st_size,
                                           hashlib.sha256(source_file.read_bytes()).hexdigest())
                manifest_hash = 'a' * 64
                method = {'assembly': source_record.file, 'type': 'Fixture', 'name': 'Run', 'token': 0x06000001,
                          'rva': 1, 'flags': 6, 'implementation_flags': 0, 'signature': '0000',
                          'il_sha256': hashlib.sha256(b'fixture IL').hexdigest()}
                index = {'schema_version': 1, 'complete': True,
                         'assemblies': [{'file': source_record.file, 'name': 'Fixture', 'version': '1.0.0.0',
                                         'mvid': 'fixture', 'bytes': source_record.bytes,
                                         'sha256': source_record.sha256, 'type_count': 1, 'method_count': 1}],
                         'methods': [method], 'types': [], 'asset_calls': [], 'diagnostics': [],
                         'verified_throw_stubs': []}
                index_path = managed / 'index.json'
                index_path.write_text(json.dumps(index))
                code_path = managed / 'Fixture.cs'
                code_path.write_text('class Fixture { void Run() {} }')
                code_record = _record(code_path, 'managed/Fixture.cs')
                row = {'token': method['token'], 'type': method['type'], 'name': method['name'],
                       'rva': method['rva'], 'flags': method['flags'],
                       'implementation_flags': method['implementation_flags'], 'signature': method['signature'],
                       'il_sha256': method['il_sha256'],
                       'status': 'decompiled', 'reason': None, 'source': code_record.file,
                       'diagnostics': []}
                coverage = {'schema_version': 1, 'input': {'file': source_record.file,
                            'sha256': source_record.sha256}, 'methods': [row]}
                coverage_path = managed / 'Fixture.coverage.json'
                coverage_path.write_text(json.dumps(coverage))
                coverage_record = _record(coverage_path, 'managed/Fixture.coverage.json')
                receipt = {'schema_version': 1, 'complete': True, 'mode': 'whole_module',
                           'input': {'file': source_record.file, 'sha256': source_record.sha256},
                           'method_count': 1, 'type_count': 1, 'diagnostics': [],
                           'output': asdict(code_record), 'coverage': asdict(coverage_record), 'supplement': None}
                receipt_path = managed / 'Fixture.decompilation.json'
                receipt_path.write_text(json.dumps(receipt))
                diagnostics_path = managed / 'diagnostics.json'
                executions = [{'source': 'index', 'exit_code': 0, 'stdout': '', 'stderr': ''},
                              {'source': source_record.file, 'exit_code': 0, 'stdout': '', 'stderr': ''}]
                diagnostics_path.write_text(json.dumps({'schema_version': 1, 'complete': True,
                    'source_manifest_sha256': manifest_hash, 'source_files': [asdict(source_record)],
                    'executions': executions}))
                coverage_file = managed / 'coverage.json'
                coverage_file.write_text(json.dumps({'schema_version': 1}))
                # The verifier compares this aggregate after it validates every method row.
                from tools.reference.library.managed import _coverage_report
                report, _ = _coverage_report(source, (source_record,), output_root, manifest_hash, False, executions)
                coverage_file.write_text(json.dumps(report))
                verify_managed_export(source, (source_record,), output_root, manifest_hash)

                if change == 'omitted_token':
                    coverage['methods'] = []
                elif change == 'duplicate_token':
                    coverage['methods'].append(dict(row))
                elif change == 'missing_coverage':
                    coverage_path.unlink()
                    report, _ = _coverage_report(source, (source_record,), output_root, manifest_hash, False, executions)
                    self.assertFalse(report['accounting_complete'])
                    retained = report['assemblies'][0]['retained_files']
                    self.assertTrue({'managed/Fixture.cs', 'managed/Fixture.decompilation.json'} <=
                                    {item['file'] for item in retained})
                    coverage_file.write_text(json.dumps(report))
                elif change == 'output_bytes':
                    code_path.write_bytes(b'changed output')
                else:
                    source_file.write_bytes(b'changed source')
                if change in ('omitted_token', 'duplicate_token'):
                    coverage_path.write_text(json.dumps(coverage))
                    receipt['coverage'] = asdict(_record(coverage_path, 'managed/Fixture.coverage.json'))
                    receipt_path.write_text(json.dumps(receipt))
                    report, _ = _coverage_report(source, (source_record,), output_root, manifest_hash, False, executions)
                    coverage_file.write_text(json.dumps(report))

                with self.assertRaises(ValueError):
                    verify_managed_export(source, (source_record,), output_root, manifest_hash)


def _record(path, name):
    payload = path.read_bytes()
    return FileRecord(name, len(payload), hashlib.sha256(payload).hexdigest())


if __name__ == '__main__':
    unittest.main()
