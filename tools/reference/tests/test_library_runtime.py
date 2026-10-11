import unittest, tempfile, json
from pathlib import Path
from unittest.mock import patch
from tools.reference.export import verify_reference, promote_reference

class RuntimeTests(unittest.TestCase):
    def test_runtime_requires_accepted_library(self):
        from types import SimpleNamespace as NS
        receipt=NS(manifest_sha256='a'*64,catalog_sha256='b'*64,source_manifest_sha256='c'*64)
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp)
            for value in ({}, {'library':{'schema_version':0}}, {'library':{'schema_version':1,'manifest_sha256':'z'*64}}):
                (root/'manifest.json').write_text(json.dumps(value))
                with self.assertRaises(ValueError): verify_reference(root, receipt)
            with self.assertRaises(ValueError): verify_reference(root, None)

    def test_only_selected_assets_in_runtime(self):
        from tools.reference.export import validate_runtime_paths
        with self.assertRaises(ValueError): validate_runtime_paths({'assets':[{'path':'library/models/unused.gltf','dependencies':[]}]})
        with self.assertRaises(ValueError): validate_runtime_paths({'assets':[{'path':'reference/home.gltf','dependencies':[{'file':'reference/unused.wav'}]}]})

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

    def test_runtime_flips_stored_pixels_and_preserves_input(self):
        from types import SimpleNamespace as NS
        from PIL import Image
        from tools.reference.library.reader import LibraryReader
        from tools.reference.gltf import GltfExporter
        from tools.reference.library.types import ObjectId
        with tempfile.TemporaryDirectory() as tmp:
            reader=LibraryReader(self.fixture(Path(tmp)))
            texture=NS(assets_file=NS(source_container_path='test',name='test'),path_id=2,type=NS(name='Texture2D'))
            data={'m_TextureSettings':{'m_FilterMode':1,'m_WrapU':0,'m_WrapV':0,'m_Aniso':0,'m_MipBias':0},'m_MipCount':1,'m_ColorSpace':1}
            source=NS(resolve=lambda *args:texture,read=lambda _:data)
            writer=GltfExporter(source,Path(tmp)/'selected.gltf',reader)
            original=reader.texture(ObjectId('test',2));before=original.read_bytes()
            writer.texture(None,{'m_Texture':{},'m_Scale':{'x':1,'y':1},'m_Offset':{'x':0,'y':0}},'srgb')
            with Image.open(Path(tmp)/writer.document.images[0].uri) as image:
                self.assertEqual(image.getpixel((0,0)),(0,0,255))
            self.assertEqual(original.read_bytes(),before)
            self.assertTrue(writer.document.images[0].extras['conversion']['flip_y'])
            original.write_bytes(b'changed image')
            with self.assertRaisesRegex(ValueError,'hash mismatch'): reader.texture(ObjectId('test',2))
