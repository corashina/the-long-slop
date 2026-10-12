import unittest, tempfile, json, hashlib, wave
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
        manifest = {'assets': [{'path': f'reference/{name}.gltf', 'dependencies': []}
                               for name in ('home', 'lada', 'items', 'road_lamp', 'sky')]}
        validate_runtime_paths(manifest)
        manifest['ui']={'sprites':[{'path':'reference/ui-test-3.png','source':{'file':'test','path_id':3}}]}
        validate_runtime_paths(manifest)
        manifest['ui']['sprites'][0]['path']='reference/unused.png'
        with self.assertRaisesRegex(ValueError,'Invalid selected runtime UI sprite'):validate_runtime_paths(manifest)
        del manifest['ui']
        manifest['assets'][0]['dependencies'].append({'file': 'reference/unused.wav'})
        with self.assertRaisesRegex(ValueError, 'Invalid selected runtime dependency'):
            validate_runtime_paths(manifest)

    def test_lighting_dependency_receipts_require_matching_typed_hashes(self):
        from tools.reference.export import validate_runtime_paths
        manifest={'assets':[{'path':f'reference/{name}.gltf','dependencies':[]} for name in ('home','lada','items','road_lamp','sky')]}
        resource={'path':'reference/lighting-test-1.smip','sha256':'a'*64}
        manifest['lighting']={'textures':[resource]}
        manifest['assets'][0]['dependencies']=[{'file':resource['path'],'sha256':resource['sha256']}]
        validate_runtime_paths(manifest)
        manifest['assets'][0]['dependencies'][0]['sha256']='b'*64
        with self.assertRaisesRegex(ValueError,'Lighting dependency hash'):validate_runtime_paths(manifest)
        manifest['assets'][0]['dependencies']=[{'file':'reference/unused.smip','sha256':'a'*64}]
        with self.assertRaisesRegex(ValueError,'Invalid selected runtime dependency'):validate_runtime_paths(manifest)

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

    def test_sprite_payload_uses_receipt_and_preserves_orientation(self):
        import hashlib
        from tools.reference.library.reader import LibraryReader
        from tools.reference.library.types import ObjectId
        from PIL import Image
        with tempfile.TemporaryDirectory() as tmp:
            reader=LibraryReader(self.fixture(Path(tmp)))
            source=ObjectId('test',3)
            path=reader.root/'images/test/3/sprite.png';path.parent.mkdir(parents=True)
            image=Image.new('RGB',(1,2));image.putdata([(255,0,0),(0,0,255)]);image.save(path)
            content=path.read_bytes()
            import dataclasses
            index=reader.root/'images/index.json'
            index.write_text(json.dumps([{'source':{'file':'test','path_id':3},'status':'converted',
                'details':{'type':'Sprite'},'files':[
                {'file':path.relative_to(reader.root).as_posix(),'bytes':len(content),
                 'sha256':hashlib.sha256(content).hexdigest()}]}]))
            manifest=json.loads(reader.receipt.manifest_path.read_bytes())
            manifest['category_indexes'].append({'file':'images/index.json','bytes':index.stat().st_size,
                'sha256':hashlib.sha256(index.read_bytes()).hexdigest()})
            reader.receipt.manifest_path.write_text(json.dumps(manifest))
            receipt=dataclasses.replace(reader.receipt,manifest_sha256=hashlib.sha256(reader.receipt.manifest_path.read_bytes()).hexdigest())
            reader=LibraryReader(receipt)
            self.assertEqual(reader.sprite(source).read_bytes(),content)
            self.assertEqual(reader.selected_sprites,{source})
            with Image.open(reader.sprite(source)) as stored:self.assertEqual(stored.getpixel((0,0)),(255,0,0))
            path.write_bytes(b'changed')
            with self.assertRaisesRegex(ValueError,'hash mismatch'):reader.sprite(source)

    def test_sky_geometry_has_no_surface_materials(self):
        from types import SimpleNamespace as NS
        from tools.reference.library.reader import LibraryReader
        from tools.reference.gltf import GltfExporter
        with tempfile.TemporaryDirectory() as tmp:
            reader=LibraryReader(self.fixture(Path(tmp)))
            mesh=NS(assets_file=NS(source_container_path='test'),path_id=1)
            writer=GltfExporter(None,Path(tmp)/'sky.gltf',reader,5)
            writer.material=lambda *_: self.fail('Sky geometry must use the existing sky renderer')
            index=writer.mesh(mesh,[NS(assets_file=mesh.assets_file,path_id=99)])
            self.assertIsNone(writer.document.meshes[index].primitives[0].material)
            self.assertEqual(writer.material_records,[])
            self.assertEqual(writer.material_dependencies,{})

    def test_typed_library_decodes_authored_mip_bytes(self):
        from types import SimpleNamespace as NS
        from PIL import Image
        from UnityPy.enums import TextureFormat
        from tools.reference.source import LibrarySource
        from tools.reference.materials import source_mip_images
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/'stored.png';Image.new('RGBA',(2,2),(10,20,30,128)).save(path)
            data=dict(m_TextureFormat=int(TextureFormat.RGBA32),m_Width=2,m_Height=2,m_MipCount=2,
                m_TextureSettings=dict(m_FilterMode=1,m_WrapU=0,m_WrapV=0,m_Aniso=1,m_MipBias=0))
            library=NS(records_root=Path(directory)/'records',catalog={
                'containers':[{'file':'fixture','unity_version':'2021.3.18f1','externals':[]}],
                'objects':[{'source':{'file':'fixture','path_id':1},'type':'Texture2D','bytes':20}]},
                record=lambda _:data,texture=lambda _:path,
                texture_data=lambda _:bytes([10,20,30,128]*4+[40,50,60,64]))
            source=LibrarySource(library)
            levels=list(source_mip_images(source.texture_object(source.get('fixture',1))))
            self.assertEqual([level.size for level in levels],[(2,2),(1,1)])
            self.assertEqual([level.getpixel((0,0)) for level in levels],[(10,20,30,128),(40,50,60,64)])

    def test_runtime_flips_stored_pixels_and_preserves_input(self):
        from types import SimpleNamespace as NS
        from PIL import Image
        from tools.reference.library.reader import LibraryReader
        from tools.reference.library.types import ObjectId
        from tools.reference.materials import export_material_textures
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp)
            reader=LibraryReader(self.fixture(root))
            texture=NS(assets_file=NS(source_container_path='test',name='test'),path_id=2,type=NS(name='Texture2D'))
            data={'m_SavedProperties':{'m_TexEnvs':[('_MainTex',{'m_Texture':{'m_PathID':2}})]}}
            original=reader.texture(ObjectId('test',2));before=original.read_bytes()
            def texture_object(value):
                with Image.open(reader.texture(ObjectId('test',2))) as stored:
                    return NS(image=stored.copy(),m_MipCount=1)
            source=NS(resolve=lambda *args:texture,read=lambda _:data,texture_object=texture_object)
            record={'source':{'file':'test','path_id':1},'properties':{'textures':[
                {'slot':'_MainTex','file':'reference/main.png','source_encoding':'rgba','mips':[{'file':'reference/main.png'}]}]}}
            dependencies=export_material_textures(source,None,record,root)
            with Image.open(root/'main.png') as image:self.assertEqual(image.getpixel((0,0)),(0,0,255,255))
            self.assertEqual(dependencies[0]['file'],'reference/main.png')
            self.assertEqual(original.read_bytes(),before)
            original.write_bytes(b'changed image')
            with self.assertRaisesRegex(ValueError,'hash mismatch'): reader.texture(ObjectId('test',2))

class RuntimeContractTests(unittest.TestCase):
    def runtime(self, root):
        from types import SimpleNamespace as NS
        from tools.reference.export import provenance
        receipt=NS(manifest_sha256='a'*64,catalog_sha256='b'*64,source_manifest_sha256='c'*64)
        manifest=json.loads((Path(__file__).resolve().parents[3]/'tests/common/reference.json').read_bytes())
        audio=manifest['audio']
        clip=audio['clips'][0]
        clip['usages']=['effect','radio']
        path=root/Path(clip['path']).name
        with wave.open(str(path),'wb') as stream:
            stream.setparams((1,2,8000,80,'NONE','not compressed'))
            stream.writeframes(b'\0'*160)
        from tools.reference.library.audio import validate_wav
        clip.update(validate_wav(path,{'m_Channels':1,'m_Frequency':8000,'m_BitsPerSample':16,'m_Length':0.01}))
        clip['bytes']=path.stat().st_size
        clip['sha256']=hashlib.sha256(path.read_bytes()).hexdigest()
        audio['clips']=[clip]
        for voice in audio['voices']:
            for field in ('clip','authored_clip'):
                if voice[field] is not None:voice[field]=clip['id']
        for binding in audio['bindings']:binding['clips']=[clip['id']]
        for station in audio['radio']['stations']:
            for field in ('music','talk'):
                station[field]=[clip['id']]*len(station[field])
        manifest['library']=provenance(receipt)
        manifest['assets'].append({'id':5,'usage':'unique_scene','material_count':1,'path':'reference/items.gltf','sha256':'a'*64,'root':{'file':'fixture','path_id':999},'dependencies':[]})
        manifest['rendering']['materials'].append({
            'source':{'file':'fixture','path_id':999},'asset':5,'gltf_material':0,
            'shader_family':'unlit_color','color':[1.,1.,1.,1.],'condition':None,'keywords':[],
            'state':{'alpha_mode':'opaque','cull':2,'z_write':True,'z_test':4,'src_blend':1,'dst_blend':0,'render_queue':-1},
            'properties':{'floats':{},'colors':{'_Color':[1.,1.,1.,1.]},'textures':[],'defaults':{},'exclusions':[]}})
        for asset in manifest['assets']:
            scene={'asset':{'version':'2.0'},'nodes':[],'scenes':[{'nodes':[]}],'scene':0}
            content=json.dumps(scene).encode()
            (root/Path(asset['path']).name).write_bytes(content)
            asset['sha256']=hashlib.sha256(content).hexdigest()
            material_mips={mip['file'] for material in manifest['rendering']['materials'] if material['asset']==asset['id']
                for texture in material['properties']['textures'] for mip in texture['mips']}
            asset['dependencies']=[d for d in asset['dependencies'] if d['file'].startswith('reference/lighting-') or d['file'] in material_mips]
            for dependency in asset['dependencies']:
                if dependency['file'] in material_mips:
                    content=b'fixture source mip bytes'
                    (root/Path(dependency['file']).name).write_bytes(content)
                    dependency['sha256']=hashlib.sha256(content).hexdigest()
        for sprite in manifest.get('ui', {}).get('sprites', []):
            content=b'fixture sprite bytes'
            (root/Path(sprite['path']).name).write_bytes(content)
            sprite['sha256']=hashlib.sha256(content).hexdigest()
        from tools.reference.export import lighting_resources
        def update_resource_hashes(value):
            if isinstance(value,dict):
                if 'path' in value and 'sha256' in value:
                    value['sha256']=hashlib.sha256(b'fixture lighting bytes').hexdigest()
                for child in value.values():update_resource_hashes(child)
            elif isinstance(value,list):
                for child in value:update_resource_hashes(child)
        update_resource_hashes(manifest['lighting'])
        for resource in lighting_resources(manifest):
            content=b'fixture lighting bytes'
            (root/Path(resource['path']).name).write_bytes(content)
            resource['sha256']=hashlib.sha256(content).hexdigest()
            for asset in manifest['assets']:
                for dependency in asset['dependencies']:
                    if dependency['file']==resource['path']:dependency['sha256']=resource['sha256']
        for name in ('geometry-evidence.json','parameter-evidence.json','library-selection.json','lighting-evidence.json','material-evidence.json'):
            (root/name).write_text('{}')
        return manifest,receipt

    def save(self, root, manifest):
        (root/'manifest.json').write_text(json.dumps(manifest))

    def test_runtime_contract_accepts_current_fixture(self):
        with tempfile.TemporaryDirectory() as tmp, patch('tools.reference.lighting.validate_lighting_export'):
            root=Path(tmp);manifest,receipt=self.runtime(root);self.save(root,manifest)
            verify_reference(root,receipt)

    def test_runtime_contract_rejects_missing_records(self):
        with tempfile.TemporaryDirectory() as tmp, patch('tools.reference.lighting.validate_lighting_export'):
            root=Path(tmp);manifest,receipt=self.runtime(root)
            for field in ('app_id','build_id','nodes','player','vehicle','colliders'):
                incomplete=dict(manifest);del incomplete[field];self.save(root,incomplete)
                with self.subTest(field=field),self.assertRaisesRegex(ValueError,'Runtime manifest contract'):
                    verify_reference(root,receipt)
            self.save(root,{key:manifest[key] for key in ('library','assets')})
            with self.assertRaisesRegex(ValueError,'Runtime manifest contract'):verify_reference(root,receipt)

    def test_runtime_contract_rejects_wrong_build_and_broken_relationships(self):
        with tempfile.TemporaryDirectory() as tmp, patch('tools.reference.lighting.validate_lighting_export'):
            root=Path(tmp);manifest,receipt=self.runtime(root)
            manifest['build_id']=0;self.save(root,manifest)
            with self.assertRaisesRegex(ValueError,'Runtime manifest contract'):verify_reference(root,receipt)
            manifest['build_id']=11140890;manifest['nodes'][0]['parent']=999999;self.save(root,manifest)
            with self.assertRaisesRegex(ValueError,'Runtime manifest contract'):verify_reference(root,receipt)

    def test_runtime_cli_verify_rejects_missing_gameplay_records(self):
        from tools.reference.export import main
        with tempfile.TemporaryDirectory() as tmp, patch('tools.reference.lighting.validate_lighting_export'):
            root=Path(tmp);manifest,receipt=self.runtime(root);del manifest['player'];self.save(root,manifest)
            before={p.name:(p.read_bytes(),p.stat().st_mtime_ns) for p in root.iterdir()}
            with patch('tools.reference.export.stage_library',return_value=receipt),self.assertRaisesRegex(ValueError,'Runtime manifest contract'):
                main(['--stage','runtime','--verify','--output',str(root)])
            self.assertEqual(before,{p.name:(p.read_bytes(),p.stat().st_mtime_ns) for p in root.iterdir()})

    def test_validator_delegates_exact_bytes_with_read_only_checkout(self):
        from types import SimpleNamespace as NS
        from tools.reference.export import validate_runtime_manifest
        content=b'{"runtime":"unchanged bytes"}\n'
        with patch('tools.reference.export.subprocess.run',return_value=NS(returncode=0)) as run:
            validate_runtime_manifest(content)
        args,kwargs=run.call_args
        self.assertEqual(kwargs['input'],content)
        self.assertIn('-i',args[0])
        self.assertIn(f'type=bind,source={Path(__file__).resolve().parents[3]},target=/workspace,readonly',args[0])
        self.assertEqual(args[0][-6:],['cargo','run','--quiet','--locked','--bin','validate-reference'])
        with patch('tools.reference.export.subprocess.run',return_value=NS(returncode=1,stderr=b'missing field player')):
            with self.assertRaisesRegex(ValueError,'missing field player'):validate_runtime_manifest(content)

    def test_unavailable_validator_preserves_accepted_runtime_before_promotion(self):
        from types import SimpleNamespace as NS
        from tools.reference.export import derive_reference
        with tempfile.TemporaryDirectory() as tmp, patch('tools.reference.lighting.validate_lighting_export'):
            root=Path(tmp);accepted=root/'reference';accepted.mkdir()
            (accepted/'manifest.json').write_text('accepted runtime')
            before=(accepted/'manifest.json').read_bytes(),(accepted/'manifest.json').stat().st_mtime_ns
            _,receipt=self.runtime(root)
            snapshot=NS(manifest_sha256=receipt.source_manifest_sha256,files=(),root=root/'cache/source',manifest_path=root/'cache/source-manifest.json')
            reader=NS(snapshot=snapshot,root=root/'library',records_root=root/'cache/records',exports={})
            (root/'cache/work').mkdir(parents=True)
            def export(source,staged,library,snapshot):
                manifest,_=self.runtime(staged);self.save(staged,manifest)
            with patch('tools.reference.library.reader.LibraryReader',return_value=reader),patch('tools.reference.source.LibrarySource'),patch('tools.reference.export._export_selected',side_effect=export),patch('tools.reference.export.RUNTIME_WORK_ROOT',root/'cache'),patch('tools.reference.export.subprocess.run',side_effect=FileNotFoundError('docker')),patch('tools.reference.export.promote_reference') as promote:
                with self.assertRaisesRegex(ValueError,'needs Docker'):derive_reference(snapshot,receipt,accepted)
                promote.assert_not_called()
            self.assertEqual(before,((accepted/'manifest.json').read_bytes(),(accepted/'manifest.json').stat().st_mtime_ns))

    def test_library_only_verification_does_not_call_runtime_validator(self):
        from tools.reference.export import main
        with patch('tools.reference.export.stage_library') as library,patch('tools.reference.export.subprocess.run') as run:
            main(['--stage','library','--verify'])
            library.assert_called_once()
            run.assert_not_called()
