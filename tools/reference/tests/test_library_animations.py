"""Critical optimized animation and rig binding checks."""
import math
import struct
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace as NS
from tools.reference.library.types import AnimationTrack, ObjectId
from tools.reference.library.animations import decode_clip, sample_track, resolve_path, bake_track
from tools.reference.gltf import convert_point

def binding(attribute=1, type_id=4, path=0, pptr=0):
    return NS(attribute=attribute, typeID=type_id, path=path, customType=0,
              isPPtrCurve=pptr, script=NS(m_FileID=0, m_PathID=0))

def stream(frames):
    data=bytearray()
    for time, keys in frames:
        data.extend(struct.pack('<fi', time,len(keys)))
        for index,coefficient,value in keys:
            data.extend(struct.pack('<i4f',index,*coefficient,value))
    return list(struct.unpack('<'+'I'*(len(data)//4),data))

def fixture(bindings, frames=(), dense=(), constant=(), streamed_count=0, dense_count=0, events=(), mapping=()):
    clip=NS(m_StreamedClip=NS(data=stream(frames),curveCount=streamed_count),
            m_DenseClip=NS(m_CurveCount=dense_count,m_FrameCount=len(dense)//dense_count if dense_count else 0,
                          m_SampleArray=list(dense),m_SampleRate=1.,m_BeginTime=0.),
            m_ConstantClip=NS(data=list(constant)))
    obj=NS(m_Name='clip',m_MuscleClip=NS(m_Clip=NS(data=clip),m_StartTime=0.,m_StopTime=1.,m_LoopTime=True),
           m_ClipBindingConstant=NS(genericBindings=bindings,pptrCurveMapping=list(mapping)),
           m_Events=list(events),m_SampleRate=30.,m_WrapMode=0,m_HasMotionFloatCurves=True,m_PositionCurves=[],m_RotationCurves=[],m_FloatCurves=[])
    reader=NS(path_id=1,assets_file=NS(source_container_path='test.assets',reader=NS(endian='<')),
              version='2021.3.18f1',parse_as_object=lambda **kw:obj)
    source=NS(container_path=lambda r:'test.assets',resolve=lambda r,p:NS(path_id=p['m_PathID'],assets_file=r.assets_file) if p['m_PathID'] else None)
    catalog=NS(objects={ObjectId('test.assets',1):None,ObjectId('test.assets',9):None})
    return source,reader,catalog

class AnimationTests(unittest.TestCase):
    def test_path_hash_collision_and_missing_target_fail(self):
        a=ObjectId('test.assets',1);b=ObjectId('test.assets',2)
        with self.assertRaisesRegex(ValueError,'Ambiguous'):resolve_path(7,{7:[('a',a),('b',b)]})
        with self.assertRaisesRegex(ValueError,'Missing'):resolve_path(8,{7:[('a',a)]})
        self.assertEqual(resolve_path(7,{7:[('a',a)]}),('a',a))

    def test_streamed_quaternion_and_constant_tracks(self):
        # The bat wing clip layout: streamed curves first, then dense, then constant, in binding order.
        q=[(i,(0.,0.,.5),v) for i,v in enumerate((0.,0.,0.,1.))]
        frames=[(-math.inf,q),(0.,q),(1.,[(i,(0.,0.,0.),v) for i,v in enumerate((0.,.5,0.,.75))]),(math.inf,[])]
        source,reader,catalog=fixture([binding(2),binding(1),binding(3)],frames=frames,constant=(1.,2.,3.,.5,.5,.5),streamed_count=4)
        record=decode_clip(source,reader,catalog)
        rotation,position,scale=record.tracks
        self.assertEqual([(t.binding['attribute'],t.binding['dimension'],t.binding['curve_offset'],t.binding['storage'],t.interpolation)
                          for t in record.tracks],[(2,4,0,'streamed','CUBIC'),(1,3,4,'constant','CONSTANT'),(3,3,7,'constant','CONSTANT')])
        self.assertEqual((rotation.times,rotation.values),((0.,1.),((0.,0.,0.,1.),(0.,.5,0.,.75))))
        self.assertEqual(rotation.coefficients[0],((0.,0.,.5),)*4)
        self.assertEqual((position.times,position.values,position.coefficients),((0.,1.),((1.,2.,3.),)*2,()))
        self.assertEqual(scale.values,((.5,.5,.5),)*2)

    def package_fixture(self, morph=False):
        from types import MethodType
        from tools.reference.tests.test_library_models import Fixture, ptr
        from tools.reference.source import Source
        source=Fixture('test assets')
        source.transform_for=MethodType(Source.transform_for,source)
        source.hierarchy=MethodType(Source.hierarchy,source)
        source.container_path=lambda r:source.file
        source.add(40,'GameObject',{'m_Name':'second','m_IsActive':True,'m_Layer':0,'m_Component':[{'component':ptr(41)}]})
        source.add(41,'Transform',source.transform(40))
        source.add(10,'Animator',{'m_GameObject':ptr(1),'m_Avatar':ptr(14),'m_Controller':ptr(12)})
        source.add(11,'Animator',{'m_GameObject':ptr(40),'m_Avatar':ptr(14),'m_Controller':ptr(12)})
        source.add(12,'AnimatorController',{'m_Name':'controller','m_AnimationClips':[ptr(13)],'m_Controller':{'m_ClipID':0}})
        source.add(14,'Avatar',{'m_Name':'avatar','m_TOS':[(0,'')]})
        source.add(13,'AnimationClip',{})
        curve=binding(1,137) if morph else binding()
        if morph:curve.customType=20
        _,reader,_=fixture([curve],dense=(0.,100.) if morph else (0.,0.,0.,0.,0.,2.),dense_count=1 if morph else 3)
        source.readers[13].parse_as_object=reader.parse_as_object
        source.readers[13].version=reader.version
        source.readers[13].assets_file.reader=NS(endian='<')
        if morph:
            del source.readers[11];del source.data[11]
        return source

    def test_gltf_animation_validation_rejects_bad_times_and_quaternions(self):
        from tools.reference.gltf import GltfBuilder,validate_gltf
        import pygltflib as gl
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp)
            for times,values,expected in [([1.,0.],[(0.,0.,0.,1.)]*2,'time'),
                                          ([0.,1.],[(0.,0.,0.,0.)]*2,'quaternion')]:
                writer=GltfBuilder(None,root)
                writer.document.nodes=[gl.Node()]
                a=writer.accessor(times,'SCALAR',target=None)
                b=writer.accessor(values,'VEC4',target=None)
                writer.document.animations=[gl.Animation(samplers=[gl.AnimationSampler(input=a,output=b)],
                    channels=[gl.AnimationChannel(sampler=0,target=gl.AnimationChannelTarget(node=0,path='rotation'))])]
                writer.document.buffers=[gl.Buffer(uri='animation.bin',byteLength=len(writer.binary))]
                (root/'animation.bin').write_bytes(writer.binary)
                (root/'animation.gltf').write_text(writer.document.to_json())
                with self.assertRaisesRegex(ValueError,expected):validate_gltf(root/'animation.gltf')
