import copy, hashlib, struct, tempfile, unittest
from pathlib import Path
from tools.reference.lighting import export_texture_batches, validate_texture_batches, export_manifest_texture_batches

class TextureBatches(unittest.TestCase):
    def texture(self, root, width=512, height=512, count=10):
        chunks=[]
        for mip in range(count):
            w=max(1,width>>mip);h=max(1,height>>mip)
            data=b'SMIP'+struct.pack('<III',w,h,1)+bytes([mip])* (w*h*4)
            name=f'{mip}.smip';(root/name).write_bytes(data)
            chunks.append(dict(data=dict(source=dict(file='test',path_id=1),path='reference/'+name,sha256=hashlib.sha256(data).hexdigest()),mip=mip,row=0,width=w,rows=h))
        return dict(source=dict(file='test',path_id=1),width=width,height=height,mip_count=count,chunks=chunks)
    def test_boundaries_and_pixels(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder); t=self.texture(root)
            export_texture_batches(t,root); first=copy.deepcopy(t['batches'])
            export_texture_batches(t,root);self.assertEqual(first,t['batches'])
            self.assertEqual([1048576,349524],[b['decoded_bytes'] for b in first])
            self.assertEqual([1,9],[b['region_count'] for b in first])
            validate_texture_batches(t,root)
    def test_corrupt_chunk_rejected(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);t=self.texture(root);(root/'0.smip').write_bytes(b'bad')
            with self.assertRaises(ValueError):export_texture_batches(t,root)
    def test_missing_duplicate_reordered_dimensions(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);original=self.texture(root)
            for change in ('missing','duplicate','order','dimensions'):
                t=copy.deepcopy(original)
                if change=='missing':t['chunks'].pop()
                if change=='duplicate':t['chunks'].append(t['chunks'][-1])
                if change=='order':t['chunks'].reverse()
                if change=='dimensions':t['chunks'][0]['width']=1
                with self.assertRaises(ValueError):export_texture_batches(t,root)

    def test_exact_32_entry_boundary(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);t=self.texture(root,1,33,1)
            t['chunks']=[]
            for row in range(33):
                name=f'row-{row}.smip';data=b'SMIP'+struct.pack('<III',1,1,1)+bytes([row])*4
                (root/name).write_bytes(data)
                t['chunks'].append(dict(data=dict(source=t['source'],path='reference/'+name,sha256=hashlib.sha256(data).hexdigest()),mip=0,row=row,width=1,rows=1))
            export_texture_batches(t,root)
            self.assertEqual([32,1],[b['region_count'] for b in t['batches']])
            validate_texture_batches(t,root)

    def test_descriptor_tampering(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);t=self.texture(root);export_texture_batches(t,root)
            path=root/Path(t['batches'][0]['descriptor']['file']).name
            path.write_bytes(path.read_bytes()[:-1])
            with self.assertRaises(ValueError):validate_texture_batches(t,root)

    def test_manifest_includes_terrain_lighting_and_material_textures(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder)
            groups=[[self.texture(root)] for _ in range(3)]
            manifest=dict(lighting=dict(textures=groups[0]),world=dict(material_textures=groups[1],terrain=dict(textures=groups[2])))
            export_manifest_texture_batches(manifest,root)
            for group in groups:
                self.assertTrue(group[0]['batches'])
                validate_texture_batches(group[0],root)
