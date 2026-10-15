import copy
import hashlib
import json
import os
import unittest
from pathlib import Path
from tools.reference.clothing import recover_clothing, validate_clothing


class WearableReferenceTests(unittest.TestCase):


    def test_checked_rgb24_lut_mips_and_bounds(self):
        from types import SimpleNamespace
        from tools.reference.mips import decoded_mips,validate_mips
        from tools.reference.library.reader import LibraryReader
        from tools.reference.library.types import LibraryReceipt
        from tools.reference.source import LibrarySource
        path=Path(os.environ['REFERENCE_LIBRARY_ROOT'])/'manifest.json'
        raw=path.read_bytes();metadata=json.loads(raw)
        source=LibrarySource(LibraryReader(LibraryReceipt(path,hashlib.sha256(raw).hexdigest(),metadata['catalog_sha256'],metadata['source_manifest_sha256'],())))
        obj=source.texture_object(source.get('sharedassets0.assets',255))
        self.assertEqual(obj.m_TextureFormat,3)
        raw=obj.get_image_data();decoded=decoded_mips(obj)
        validate_mips(decoded,obj.m_Width,obj.m_Height,obj.m_MipCount)
        source_offset=0;decoded_offset=16
        for level in range(obj.m_MipCount):
            count=max(1,obj.m_Width>>level)*max(1,obj.m_Height>>level)
            data=raw[source_offset:source_offset+3*count]
            expected=b''.join(data[i:i+3]+b'\xff' for i in range(0,len(data),3))
            self.assertEqual(decoded[decoded_offset:decoded_offset+count*4],expected)
            source_offset+=3*count;decoded_offset+=4*count
        self.assertEqual(source_offset,len(raw))
        for data,kind in [(raw[:-1],3),(raw+b'\x00',3),(raw,999)]:
            invalid=SimpleNamespace(m_TextureFormat=kind,m_Width=obj.m_Width,m_Height=obj.m_Height,m_MipCount=obj.m_MipCount,get_image_data=lambda:data)
            with self.assertRaises(ValueError):decoded_mips(invalid)
