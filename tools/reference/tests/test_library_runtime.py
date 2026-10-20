import unittest, tempfile, json, hashlib, wave
from pathlib import Path
from unittest.mock import patch
from tools.reference.export import verify_reference, promote_reference

class RuntimeTests(unittest.TestCase):
    def test_archived_lighting_ranges_reject_changes_and_out_of_bounds_reads(self):
        import hashlib
        from types import SimpleNamespace as NS
        from tools.reference.library.reader import LibraryReader
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);path=root/'source.bin';path.write_bytes(b'accepted source')
            reader=LibraryReader.__new__(LibraryReader)
            reader.snapshot=NS(root=root)
            reader.source_files={'source.bin':NS(bytes=15,sha256=hashlib.sha256(path.read_bytes()).hexdigest())}
            reader.verified_source_files={}
            self.assertEqual(reader.source_range('source.bin',9,6),b'source')
            with self.assertRaisesRegex(ValueError,'exceeds file'):reader.source_range('source.bin',10,6)
            path.write_bytes(b'changed source!')
            with self.assertRaisesRegex(ValueError,'hash mismatch'):reader.source_range('source.bin',0,1)

    def test_failed_runtime_promotion_restores_previous_directory(self):
        with tempfile.TemporaryDirectory() as tmp:
            accepted=Path(tmp)/'reference'; accepted.mkdir(); (accepted/'manifest.json').write_text('previous')
            staged=Path(tmp)/'staged'; staged.mkdir(); (staged/'manifest.json').write_text('new')
            replace=Path.replace
            def fail(path,target):
                if path==staged: raise PermissionError('injected rename error')
                return replace(path,target)
            with patch.object(Path,'replace',fail), self.assertRaises(PermissionError): promote_reference(staged,accepted)
            self.assertEqual((accepted/'manifest.json').read_text(),'previous')

class LibraryInputTests(unittest.TestCase):
    def fixture(self, root):
        import hashlib
        import struct
        import pygltflib as gl
        from PIL import Image
        from tools.reference.gltf import GltfBuilder
        from tools.reference.library.types import LibraryReceipt
        library=root/'library'; records=root/'cache/records'
        library.mkdir(); records.mkdir(parents=True)
        def write(base,name,value):
            path=base/name; path.parent.mkdir(parents=True,exist_ok=True)
            content=value if isinstance(value,bytes) else json.dumps(value).encode()
            path.write_bytes(content)
            return {'file':name,'bytes':len(content),'sha256':hashlib.sha256(content).hexdigest()}
        source=write(records.parent,'source-manifest.json',{'files':[]})['sha256']
        catalog=write(records.parent,'catalog.json',{'containers':[],'objects':[]})['sha256']
        typed=write(records,'test/1.json',{'schema_version':1,'source':{'file':'test','path_id':1},'data':{'m_Name':'accepted','pairs':[{'$pair':['x',2]}]}})
        typed_manifest=write(records,'manifest.json',{'schema_version':1,'source_manifest_sha256':source,'catalog_sha256':catalog,'records':[typed]})
        directory=library/'models/meshes/test/1'; directory.mkdir(parents=True)
        writer=GltfBuilder(None,directory/'geometry.gltf')
        position=writer.accessor([(0,0,-1),(1,0,-1),(0,1,-1)],'VEC3')
        indices=writer.accessor([0,2,1],'SCALAR',5125,34963)
        writer.document.meshes.append(gl.Mesh(primitives=[gl.Primitive(attributes=gl.Attributes(POSITION=position),indices=indices,extras={'submesh':0})],extras={}))
        writer.document.buffers.append(gl.Buffer(uri='geometry.bin',byteLength=len(writer.binary)))
        write(directory,'geometry.bin',bytes(writer.binary));writer.document.save_json(str(writer.output))
        files=[write(library,f'models/meshes/test/1/{name}',(directory/name).read_bytes()) for name in ('geometry.bin','geometry.gltf')]
        image_path=library/'images/test/2/source.png';image_path.parent.mkdir(parents=True)
        image=Image.new('RGB',(1,2));image.putdata([(255,0,0),(0,0,255)]);image.save(image_path)
        image_record=write(library,'images/test/2/source.png',image_path.read_bytes())
        exports=[{'source':{'file':'test','path_id':1},'status':'converted','details':{'type':'Mesh'},'files':files},
                 {'source':{'file':'test','path_id':2},'status':'converted','details':{'type':'Texture2D'},'files':[image_record]}]
        index=write(library,'models/index.json',exports)
        manifest=write(library,'manifest.json',{'schema_version':1,'catalog_sha256':catalog,'source_manifest_sha256':source,'records_root':'../cache/records','records_manifest':typed_manifest,'category_indexes':[index]})
        return LibraryReceipt(library/'manifest.json',manifest['sha256'],catalog,source,())

    def test_reader_checks_selected_record_and_geometry_hashes(self):
        from tools.reference.library.reader import LibraryReader
        from tools.reference.library.types import ObjectId
        with tempfile.TemporaryDirectory() as tmp:
            receipt=self.fixture(Path(tmp));reader=LibraryReader(receipt)
            self.assertEqual(reader.record(ObjectId('test',1))['pairs'],[('x',2)])
            document,directory,index=reader.mesh(ObjectId('test',1))
            self.assertEqual(index,0)
            (directory/'geometry.bin').write_bytes(b'changed')
            with self.assertRaisesRegex(ValueError,'hash mismatch'):reader.mesh(ObjectId('test',1))
            (reader.records_root/'test/1.json').write_text('{}')
            with self.assertRaisesRegex(ValueError,'hash mismatch'):reader.record(ObjectId('test',1))

    def test_runtime_copies_geometry_without_second_reflection(self):
        from types import SimpleNamespace as NS
        from tools.reference.library.reader import LibraryReader
        from tools.reference.gltf import GltfExporter
        with tempfile.TemporaryDirectory() as tmp:
            reader=LibraryReader(self.fixture(Path(tmp)))
            mesh=NS(assets_file=NS(source_container_path='test'),path_id=1)
            writer=GltfExporter(None,Path(tmp)/'selected.gltf',reader)
            index=writer.mesh(mesh,[None])
            primitive=writer.document.meshes[index].primitives[0]
            points=writer.document.accessors[primitive.attributes.POSITION]
            self.assertEqual(points.min,[0,0,-1])
            original, directory,_=reader.mesh(__import__('tools.reference.library.types',fromlist=['ObjectId']).ObjectId('test',1))
            for old,new in [(original.meshes[0].primitives[0].indices,primitive.indices),(original.meshes[0].primitives[0].attributes.POSITION,primitive.attributes.POSITION)]:
                before=original.bufferViews[original.accessors[old].bufferView]
                after=writer.document.bufferViews[writer.document.accessors[new].bufferView]
                raw=(directory/'geometry.bin').read_bytes()
                self.assertEqual(bytes(writer.binary[after.byteOffset:after.byteOffset+after.byteLength]),raw[before.byteOffset:before.byteOffset+before.byteLength])
            self.assertEqual(writer.geometry(mesh)[2],[[(0,2,1)]])


class WorldCaptureVerificationTests(unittest.TestCase):
    def setUp(self):
        from types import SimpleNamespace as NS
        from tools.reference.export import provenance
        self.library = NS(manifest_sha256='a' * 64, catalog_sha256='b' * 64,
                          source_manifest_sha256='c' * 64)
        self.current = provenance(self.library)
        self.capture = dict(self.current, manifest_sha256='d' * 64)
        self.reader = object()

    def verify(self, world, proof_error=None, library=None):
        from tools.reference.export import provenance
        library = library or self.library
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp)
            (output / 'manifest.json').write_text(json.dumps(dict(library=provenance(library), world=world)))
            with patch('tools.reference.library.reader.LibraryReader', return_value=self.reader) as reader, \
                    patch('tools.reference.world_capture.verify_capture_inputs', return_value=self.capture,
                          side_effect=proof_error) as proof, \
                    patch('tools.reference.acquisition.validate_acquisition') as acquisition, \
                    patch('tools.reference.export.validate_runtime_manifest') as native:
                try:
                    verify_reference(output, library)
                finally:
                    reader.assert_called_once_with(library)
                    proof.assert_called_once_with(self.reader)
                    acquisition.assert_not_called()
                    native.assert_not_called()

    def test_verification_requires_the_same_input_proof_before_native_validation(self):
        with self.assertRaisesRegex(ValueError, 'required capture proof'):
            self.verify(dict(library=self.current, capture_library=self.capture),
                        ValueError('required capture proof'))

    def test_equal_hash_verification_still_requires_the_input_proof(self):
        self.capture = dict(self.current)
        with self.assertRaisesRegex(ValueError, 'required capture proof'):
            self.verify(dict(library=self.current, capture_library=self.capture),
                        ValueError('required capture proof'))

    def test_verification_rejects_missing_or_changed_capture_identity(self):
        for capture in (None, dict(self.capture, manifest_sha256='e' * 64)):
            world = dict(library=self.current)
            if capture is not None: world['capture_library'] = capture
            with self.subTest(capture=capture), self.assertRaisesRegex(ValueError, 'world capture provenance'):
                self.verify(world)

    def test_verification_rejects_current_identity_independently(self):
        world = dict(capture_library=self.capture, library=dict(self.current, manifest_sha256='e' * 64))
        with self.assertRaisesRegex(ValueError, 'world current library provenance'):
            self.verify(world)
