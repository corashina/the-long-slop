import hashlib
import json
import tempfile
import subprocess
import unittest
from dataclasses import asdict
from pathlib import Path
from types import SimpleNamespace as N
from unittest.mock import patch

from tools.reference.library.types import Catalog, FileRecord, ObjectId, ObjectRecord, Snapshot
from tools.reference.library.records import export_native_assets
from tools.reference.library.managed import index_managed, write_analysis_report

class MetadataTests(unittest.TestCase):
    def fixture(self, root, kinds):
        objects, dependencies, receipts, data = {}, {}, [], {}
        for number, kind in enumerate(kinds, 1):
            identity = ObjectId('assets', number)
            objects[identity] = ObjectRecord(identity, kind, 1, number * 8, 8, 'a' * 64)
            dependencies[identity] = ()
            value = {'m_Name': kind, 'settings': {'retained': True}}
            if kind == 'Shader': value['m_ParsedForm'] = {'m_Name': 'shader'}
            data[number] = value
            path = root / f'assets/{number}.json'
            path.parent.mkdir(exist_ok=True)
            payload = json.dumps({'data': value}).encode()
            path.write_bytes(payload)
            receipts.append(asdict(FileRecord(f'assets/{number}.json', len(payload), hashlib.sha256(payload).hexdigest())))
        (root / 'manifest.json').write_text(json.dumps({'schema_version': 1, 'records': receipts}))
        source = N(get=lambda file, number: N(path_id=number), read_record=lambda reader: data[reader.path_id],
                   root=root / 'source')
        return source, Catalog(objects, {}, dependencies)

    def stub_decompiler(self, root):
        package = root / 'package'
        assembly = package / 'lib/netstandard2.0/ICSharpCode.Decompiler.dll'
        assembly.parent.mkdir(parents=True)
        code = """using System.Reflection;
[assembly: AssemblyVersion("11.1.0.9782")]
namespace ICSharpCode.Decompiler { public class DecompilerSettings {} }
namespace ICSharpCode.Decompiler.CSharp {
    public class CSharpDecompiler {
        public CSharpDecompiler(string path, ICSharpCode.Decompiler.DecompilerSettings settings) {}
        public string DecompileWholeModuleAsString() { return "// Error decompiling fixture method"; }
    }
}"""
        source = root / 'stub.cs'
        source.write_text(code)
        command = f"Add-Type -TypeDefinition ([IO.File]::ReadAllText('{source.as_posix()}')) -OutputAssembly '{assembly.as_posix()}'"
        build = subprocess.run(['pwsh', '-NoProfile', '-Command', command], capture_output=True, text=True)
        self.assertEqual(build.returncode, 0, build.stderr)
        nuspec = package / 'ICSharpCode.Decompiler.nuspec'
        nuspec.write_text("""<package><metadata><version>11.1.0.9782</version><dependencies>
<group><dependency id="System.Collections.Immutable" version="9.0.0"/>
<dependency id="System.Reflection.Metadata" version="9.0.0"/></group>
</dependencies></metadata></package>""")
        return assembly, nuspec

    def run_failure_decompiler(self, root, assembly):
        from tools.reference.library.managed import SCRIPT
        output = root / 'output'
        result = subprocess.run(['pwsh', '-NoProfile', '-File', str(SCRIPT),
            '-AssemblyPath', str(assembly), '-DecompilerPath', str(assembly), '-OutputPath', str(output)],
            capture_output=True, text=True)
        self.assertNotEqual(result.returncode, 0)
        receipt = json.loads((output / 'decompilation.json').read_bytes())
        self.assertFalse(receipt['complete'])
        return output, receipt

    def test_actual_failed_decompile_keeps_diagnostics_and_provenance(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            assembly, nuspec = self.stub_decompiler(root)
            original = assembly.read_bytes()
            output, receipt = self.run_failure_decompiler(root, assembly)
            self.assertTrue(any('Error decompiling fixture method' in value for value in receipt['diagnostics']))
            self.assertTrue(any('reported incomplete code' in value for value in receipt['diagnostics']))
            raw = (output / 'Assembly-CSharp.cs').read_bytes()
            self.assertEqual(receipt['output']['sha256'], hashlib.sha256(raw).hexdigest())
            self.assertEqual(receipt['output']['bytes'], len(raw))
            self.assertEqual(receipt['decompiler']['version'], '11.1.0.9782')
            self.assertEqual(receipt['decompiler']['sha256'], hashlib.sha256(original).hexdigest())
            self.assertEqual(receipt['decompiler']['package_sha256'], hashlib.sha256(nuspec.read_bytes()).hexdigest())
            self.assertEqual(len(receipt['decompiler']['dependencies']), 2)
            self.assertTrue(receipt['host']['powershell'])
            self.assertGreater(receipt['type_count'], 0)
            self.assertGreater(receipt['method_count'], 0)
            self.assertEqual(assembly.read_bytes(), original)

if __name__ == '__main__': unittest.main()
