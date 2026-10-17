import copy
import unittest
from types import SimpleNamespace
from unittest.mock import patch
from tools.reference.native_bc import eligible, mip_sizes, source_blocks, validate_storage


class NativeBc(unittest.TestCase):
    def test_source_levels_keep_order_and_exact_full_byte_count(self):
        sizes = mip_sizes(7, 3, 3, 8)
        self.assertEqual(sizes, [16, 8, 8])
        raw = b''.join(bytes([i]) * n for i, n in enumerate(sizes))
        def texture(data, fmt=10):
            return SimpleNamespace(m_TextureFormat=fmt, m_Width=7, m_Height=3, m_MipCount=3, get_image_data=lambda: data)
        self.assertEqual(list(source_blocks(texture(raw))), [bytes([i]) * n for i, n in enumerate(sizes)])
        for data in [raw[:-1], raw+b'x']:
            with self.assertRaises(ValueError): list(source_blocks(texture(data)))
        with patch('texture2ddecoder.unpack_unity_crunch', side_effect=lambda data, level: bytes([level]) * sizes[level]) as decoder:
            self.assertEqual(b''.join(source_blocks(texture(b'crunch',28))),raw)
            self.assertEqual([call.args[1] for call in decoder.call_args_list],[0,1,2])
        with patch('texture2ddecoder.unpack_unity_crunch', return_value=b'bad'):
            with self.assertRaises(ValueError): list(source_blocks(texture(b'crunch',28)))
        self.assertEqual(mip_sizes(1024,512,11,8),[262144,65536,16384,4096,1024,256,64,16,8,8,8])
        self.assertEqual(mip_sizes(1,2,2,16),[16,16])

    def test_storage_gate_and_checked_receipt(self):
        data=dict(m_TextureFormat=28,m_ImageCount=1,m_TextureDimension=2,m_MipsStripped=0,m_Width=8,m_Height=4,m_MipCount=4)
        entry=dict(source=dict(file='fixture.assets',path_id=12),color_space='srgb',encoding='rgba',source_encoding='rgba',sampler=dict(mip_count=4))
        self.assertTrue(eligible(entry,data))
        for key,bad in [('m_TextureFormat',4),('m_ImageCount',2),('m_TextureDimension',3),('m_MipsStripped',1),('m_MipCount',3)]:
            changed=dict(data);changed[key]=bad
            self.assertFalse(eligible(entry,changed))
        for key in ['encoding','source_encoding']:
            changed=dict(entry);changed[key]='ag';self.assertFalse(eligible(changed,data))
        receipt=dict(file='reference/texture-fixture.assets-12-srgb.bc',bytes=40,sha256='0'*64,image_size=None)
        value=dict(kind='native_bc',format='bc1',source_format=28,width=8,height=4,mip_count=4,image_count=1,dimension=2,mips_stripped=0,input=receipt)
        entry['storage']=value; files={receipt['file']:receipt}
        validate_storage(entry,files)
        for key,bad in [('format','bc3'),('source_format',29),('width',16),('image_count',2),('dimension',3),('mips_stripped',1),('mip_count',3)]:
            changed=copy.deepcopy(entry);changed['storage'][key]=bad
            with self.subTest(key=key),self.assertRaises(ValueError): validate_storage(changed,files)
        for n in [39,41]:
            changed=copy.deepcopy(entry);changed['storage']['input']['bytes']=n
            with self.assertRaises(ValueError): validate_storage(changed,files)
        changed=copy.deepcopy(entry);changed['storage']['input']['sha256']='1'*64
        with self.assertRaises(ValueError): validate_storage(changed,files)
        with self.assertRaises(KeyError): validate_storage(entry,{})
