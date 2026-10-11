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

    def test_shader_text_failure_keeps_native_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source, catalog = self.fixture(root, ['Shader'])
            reader = N(parse_as_object=lambda **kwargs: N(export=lambda: (_ for _ in ()).throw(ValueError('GPU program unsupported'))))
            source.get = lambda file, number: reader
            with patch('tools.reference.library.records.SHADER_TEXT_FAILURES', frozenset(catalog.objects)):
                result, = export_native_assets(source, catalog, root, frozenset())
            self.assertEqual(result.status, 'native_record')
            self.assertEqual(result.details['native_range']['bytes'], 8)
            self.assertEqual(result.details['native_range']['container_sha256'], 'a' * 64)
            self.assertEqual(result.details['shader_text']['status'], 'failed_optional')
            self.assertIn('GPU program unsupported', result.details['shader_text']['reason'])
            self.assertFalse(result.details['shader_text']['original_source_recovered'])
            self.assertEqual(json.loads((root / result.details['record']['file']).read_text())['data']['m_ParsedForm']['m_Name'], 'shader')

    def test_runtime_texture_is_descriptor_not_fake_image(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source, catalog = self.fixture(root, ['RenderTexture', 'Camera'])
            texture, camera = catalog.objects
            catalog.dependencies[camera] = (texture,)
            result = export_native_assets(source, catalog, root, frozenset())
            self.assertEqual(result[0].details['representation'], 'runtime_descriptor')
            self.assertEqual(result[0].files, ())
            self.assertEqual(result[1].dependencies, (texture,))
            self.assertEqual(list(root.rglob('*.png')), [])

    def test_material_and_particle_links_close(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source, catalog = self.fixture(root, ['Material', 'Shader', 'ParticleSystem', 'Mesh', 'Texture2D'])
            material, shader, particle, mesh, texture = catalog.objects
            catalog.dependencies[material] = (shader, texture)
            catalog.dependencies[particle] = (material, mesh)
            claims = frozenset((mesh, texture))
            exports = export_native_assets(source, catalog, root, claims)
            self.assertEqual({e.source for e in exports}, set(catalog.objects) - claims)
            self.assertEqual(exports[0].dependencies, (shader, texture))
            self.assertEqual(exports[2].dependencies, (material, mesh))
            self.assertEqual(export_native_assets(source, catalog, root, claims), exports)
            with self.assertRaisesRegex(ValueError, 'Unknown claimed'):
                export_native_assets(source, catalog, root, claims | {ObjectId('bad', 1)})
            catalog.dependencies[particle] = (ObjectId('bad', 1),)
            with self.assertRaisesRegex(ValueError, 'Missing dependency'):
                export_native_assets(source, catalog, root, claims)

    def test_decompiler_error_cannot_claim_complete_code(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            original = root / 'source/TheLongDrive_Data/Managed/Assembly-CSharp.dll'
            original.parent.mkdir(parents=True)
            original.write_bytes(b'original')
            receipt = FileRecord('TheLongDrive_Data/Managed/Assembly-CSharp.dll', 8, hashlib.sha256(b'original').hexdigest())
            snapshot = Snapshot(root / 'source', (receipt,), root / 'manifest.json', 'x')
            output = root / 'analysis'
            with patch('tools.reference.library.managed.subprocess.run', side_effect=[N(returncode=0, stdout='', stderr=''), N(returncode=1, stdout='', stderr='decompiler failed')]):
                with self.assertRaisesRegex(ValueError, 'Managed analysis incomplete'):
                    index_managed(snapshot, output)
            self.assertEqual(original.read_bytes(), b'original')
            self.assertFalse(json.loads((output / 'managed/diagnostics.json').read_text())['complete'])

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

    def test_actual_early_decompile_failure_keeps_available_provenance(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            assembly, nuspec = self.stub_decompiler(root)
            nuspec.write_text(nuspec.read_text().replace('11.1.0.9782', '11.0.0.0'))
            _, receipt = self.run_failure_decompiler(root, assembly)
            self.assertEqual(receipt['input']['sha256'], hashlib.sha256(assembly.read_bytes()).hexdigest())
            self.assertEqual(receipt['decompiler']['package_sha256'], hashlib.sha256(nuspec.read_bytes()).hexdigest())
            self.assertEqual(receipt['decompiler']['sha256'], hashlib.sha256(assembly.read_bytes()).hexdigest())
            self.assertEqual(receipt['decompiler']['package_version'], '11.0.0.0')
            self.assertTrue(receipt['host']['dotnet'])
            self.assertTrue(any('package version mismatch' in value for value in receipt['diagnostics']))

    def test_text_asset_payload_preserves_invalid_utf8(self):
        import struct
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source, catalog = self.fixture(root, ['TextAsset'])
            payload = b'\xff\x00A\r\n'
            raw = struct.pack('<i', 3) + b'abc' + b'\x00' + struct.pack('<i', len(payload)) + payload
            source.get = lambda file, number: N(get_raw_data=lambda: raw)
            export, = export_native_assets(source, catalog, root, frozenset())
            self.assertEqual((root / export.files[0].file).read_bytes(), payload)
            self.assertEqual(export.details['text']['encoding'], 'binary')
            self.assertEqual(export.details['text']['source_payload_offset'], 20)

    def test_standalone_text_has_file_receipts_without_object_ids(self):
        from tools.reference.library.managed import index_source_text
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            files = []
            for name in ('news.txt', 'musicbox.txt', 'Languages/English.txt'):
                name = 'TheLongDrive_Data/' + name
                path = root / 'source' / name
                path.parent.mkdir(parents=True, exist_ok=True)
                payload = b'authored\r\n'
                path.write_bytes(payload)
                files.append(FileRecord(name, len(payload), hashlib.sha256(payload).hexdigest()))
            snapshot = Snapshot(root / 'source', tuple(files), root / 'manifest.json', 'x')
            receipts = index_source_text(snapshot, root / 'output')
            index = json.loads((root / 'output/native/source-text.json').read_bytes())
            self.assertEqual(len(index['files']), 2)
            self.assertTrue(all(set(item['source']) == {'file', 'bytes', 'sha256'} for item in index['files']))
            self.assertEqual(len(receipts), 3)

    def test_null_resource_name_remains_null(self):
        from tools.reference.library.managed import native_managed_links
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source, catalog = self.fixture(root, ['ResourceManager'])
            source.read_record = lambda reader: {'m_Container': [('null_shader', {'m_FileID': 0, 'm_PathID': 0})]}
            result = native_managed_links(source, catalog, root)
            self.assertIsNone(json.loads((root / result.file).read_text())['resources'][0]['target'])

    def test_missing_namespace_candidate_does_not_claim_exact_mapping(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source, catalog = self.fixture(root, ['MonoScript', 'MonoBehaviour'])
            script, component = catalog.objects
            catalog.dependencies[component] = (script,)
            (root / 'assets/1.json').write_text(json.dumps({'data': {'m_AssemblyName': 'Package.dll',
                'm_Namespace': '', 'm_ClassName': 'Type'}}))
            (root / 'managed').mkdir()
            (root / 'native').mkdir()
            index = {'schema_version': 1, 'complete': True, 'asset_calls': [],
                'assemblies': [{'file': 'Package.dll', 'name': 'Package', 'sha256': 'x'}],
                'types': [{'assembly': 'Package.dll', 'full_name': 'Namespace.Type', 'token': 1}]}
            raw = json.dumps(index).encode()
            (root / 'managed/index.json').write_bytes(raw)
            (root / 'native/managed-links.json').write_text(json.dumps({'schema_version': 1,
                'resources': [], 'creation_bindings': [], 'controller_assignments': [], 'source_unbound_rigs': []}))
            receipt = FileRecord('managed/index.json', len(raw), hashlib.sha256(raw).hexdigest())
            report = write_analysis_report(catalog, (receipt,), root)
            result = json.loads((root / report.file).read_bytes())
            self.assertEqual(result['scripts'][0]['status'], 'unresolved')
            self.assertEqual(result['scripts'][0]['reason'], 'declared_type_not_in_archived_assembly')
            self.assertEqual(result['scripts'][0]['candidates'][0]['full_name'], 'Namespace.Type')
            self.assertEqual(result['script_summary']['unmatched_referenced_scripts'], 1)

    def test_resources_call_requires_exact_literal_and_source_target(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'managed').mkdir()
            (root / 'native').mkdir()
            material = ObjectId('assets', 1)
            catalog = Catalog({material: ObjectRecord(material, 'Material', 21, 0, 8, 'x')}, {}, {material: ()})
            assembly = 'TheLongDrive_Data/Managed/Assembly-CSharp.dll'
            call = {'assembly': assembly, 'type': 'Outline', 'method': 'Awake', 'method_token': 1,
                    'il_offset': 5, 'target': {'type': 'UnityEngine.Resources', 'method': 'Load'},
                    'preceding_string_literals': [{'il_offset': 0, 'value': 'Materials/OutlineMask'}]}
            index = {'schema_version': 1, 'complete': True, 'types': [], 'asset_calls': [call],
                     'assemblies': [{'file': assembly, 'name': 'Assembly-CSharp',
                         'sha256': '9919a9eeddcce23bb4ff86e890437f32b4a3ef3c10af2efe3b8d889d894dd62e'}]}
            links = {'schema_version': 1, 'resources': [{'name': 'materials/outlinemask', 'target': asdict(material)}],
                     'creation_bindings': [], 'controller_assignments': [], 'source_unbound_rigs': []}
            (root / 'native/managed-links.json').write_text(json.dumps(links))
            def write_index():
                raw = json.dumps(index).encode()
                (root / 'managed/index.json').write_bytes(raw)
                return FileRecord('managed/index.json', len(raw), hashlib.sha256(raw).hexdigest())
            report = write_analysis_report(catalog, (write_index(),), root)
            saved = json.loads((root / report.file).read_bytes())
            self.assertEqual(saved['proven_dynamic_links'][0]['resource']['target'], asdict(material))
            call['il_offset'] = 12
            report = write_analysis_report(catalog, (write_index(),), root)
            self.assertEqual(json.loads((root / report.file).read_bytes())['proven_dynamic_links'], [])
            call['il_offset'] = 5
            links['resources'][0]['target'] = asdict(ObjectId('missing', 2))
            (root / 'native/managed-links.json').write_text(json.dumps(links))
            with self.assertRaisesRegex(ValueError, 'unknown target'):
                write_analysis_report(catalog, (write_index(),), root)

    def test_analysis_limits_do_not_change_serialized_edges(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'managed').mkdir()
            payload = {'schema_version': 1, 'complete': True, 'assemblies': [], 'asset_calls': [], 'types': []}
            raw = json.dumps(payload).encode()
            (root / 'managed/index.json').write_bytes(raw)
            receipt = FileRecord('managed/index.json', len(raw), hashlib.sha256(raw).hexdigest())
            (root / 'native').mkdir()
            (root / 'native/managed-links.json').write_text(json.dumps({'schema_version': 1, 'resources': [], 'creation_bindings': [], 'controller_assignments': [], 'source_unbound_rigs': []}))
            catalog = Catalog({}, {}, {})
            report = write_analysis_report(catalog, (receipt,), root)
            self.assertEqual(catalog.dependencies, {})
            self.assertIn('limits', json.loads((root / report.file).read_text()))


if __name__ == '__main__': unittest.main()
