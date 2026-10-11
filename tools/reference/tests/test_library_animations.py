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
    def test_optimized_clip_with_empty_top_level_arrays(self):
        record=decode_clip(*fixture([binding()],constant=(0.,0.,7.)))
        self.assertEqual(len(record.tracks),1)
        self.assertTrue(record.settings['m_HasMotionFloatCurves'])
        self.assertEqual(sample_track(record.tracks[0],.5),(0.,0.,7.))
        constant_track=AnimationTrack({},(0.,1.),((7.,),(7.,)),'CONSTANT')
        linear_translation=AnimationTrack({},(0.,1.),((0.,0.,0.),(0.,0.,2.)),'LINEAR')
        self.assertEqual(sample_track(constant_track,.5),(7.,))
        self.assertEqual(sample_track(linear_translation,.5),(0.,0.,1.))
        self.assertEqual(convert_point(sample_track(linear_translation,.5)),(0.,0.,-1.))

    def test_stream_sentinels_and_binding_dimensions(self):
        keys=lambda v:[(i,(0.,0.,2. if i==2 else 0.),v if i==2 else 0.) for i in range(3)]
        frames=[(-3.4028234663852886e38,keys(99.)),(0.,keys(0.)),(1.,keys(2.)),(math.inf,[])]
        track=decode_clip(*fixture([binding()],frames=frames,streamed_count=3)).tracks[0]
        self.assertEqual(track.times,(0.,1.))
        self.assertEqual(sample_track(track,.5),(0.,0.,1.))
        self.assertEqual(track.in_tangents[1],(0.,0.,2.))

    def test_dense_then_constant_curve_offsets(self):
        args=fixture([binding(12,95),binding(),binding(3)],
                     frames=[(-3.4e38,[]),(0.,[(0,(0.,0.,1.),2.)]),(1.,[(0,(0.,0.,0.),3.)]),(math.inf,[])],
                     streamed_count=1,dense_count=3,dense=(1.,2.,3.,4.,5.,6.),constant=(7.,8.,9.))
        tracks=decode_clip(*args).tracks
        self.assertEqual([t.binding['curve_offset'] for t in tracks],[0,1,4])
        self.assertEqual(sample_track(tracks[1],.5),(2.5,3.5,4.5))
        self.assertEqual(sample_track(tracks[2],.5),(7.,8.,9.))

    def test_path_hash_collision_and_missing_target_fail(self):
        a=ObjectId('test.assets',1);b=ObjectId('test.assets',2)
        with self.assertRaisesRegex(ValueError,'Ambiguous'):resolve_path(7,{7:[('a',a),('b',b)]})
        with self.assertRaisesRegex(ValueError,'Missing'):resolve_path(8,{7:[('a',a)]})
        self.assertEqual(resolve_path(7,{7:[('a',a)]}),('a',a))

    def test_muscle_event_and_object_tracks_retained(self):
        pointer=NS(m_FileID=0,m_PathID=9)
        args=fixture([binding(12,95),binding(5,114,pptr=1)],constant=(4.,0.),mapping=(pointer,),
                     events=(NS(time=.5,functionName='event',objectReferenceParameter=pointer),))
        record=decode_clip(*args)
        self.assertEqual(record.tracks[0].binding['type_id'],95)
        self.assertEqual(record.tracks[1].binding['object_references'],[{'file':'test.assets','path_id':9}]*2)
        self.assertEqual(record.events[0]['objectReferenceParameter'],{'file':'test.assets','path_id':9})

    def test_rotation_subdivision_steps_and_zero_quaternion(self):
        track=AnimationTrack({'attribute':4},(0.,1.),((0.,0.,0.),(0.,180.,0.)),'CUBIC',
                             ((0.,0.,0.),(0.,0.,0.)),((0.,0.,0.),(0.,0.,0.)))
        times,values,mode,error=bake_track(track)
        self.assertGreater(len(times),2)
        self.assertLessEqual(error,.01)
        step=AnimationTrack({'attribute':1},(0.,1.),((0.,0.,0.),(1.,0.,0.)),'STEP')
        self.assertEqual(bake_track(step)[2],'STEP')
        bad=AnimationTrack({'attribute':2},(0.,),((0.,0.,0.,0.),),'CONSTANT')
        with self.assertRaisesRegex(ValueError,'zero|Zero'):bake_track(bad)

    def test_constant_clip_with_only_stream_dummy(self):
        record=decode_clip(*fixture([binding()],frames=[(math.inf,[])],constant=(1.,2.,3.)))
        self.assertEqual(sample_track(record.tracks[0],.5),(1.,2.,3.))

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

    def test_packages_keep_models_and_scope_repeated_paths(self):
        from unittest.mock import patch
        from tools.reference.library.models import export_models
        from tools.reference.library.animations import export_animations
        from tools.reference.gltf import validate_gltf
        from urllib.parse import unquote
        import json
        source=self.package_fixture()
        with tempfile.TemporaryDirectory() as temp:
            output=Path(temp)
            with patch('tools.reference.library.models.MeshHandler',side_effect=lambda r:source.handlers[r.path_id]):
                models=export_models(source,source.catalog(),output)
            before={f.file:(output/f.file).read_bytes() for m in models for f in m.files}
            exports=export_animations(source,source.catalog(),models,output)
            clip=next(e for e in exports if e.source.path_id==13)
            self.assertEqual(len(clip.details['packages']),2)
            targets=[]
            for package in clip.details['packages']:
                path=output/package['file'];document=validate_gltf(path)
                channel=document.animations[0].channels[0]
                targets.append(document.nodes[channel.target.node].extras['source']['path_id'])
                sampler=document.animations[0].samplers[0]
                accessor=document.accessors[sampler.output];view=document.bufferViews[accessor.bufferView]
                self.assertEqual(view.buffer,len(document.buffers)-1)
                self.assertIsNone(view.target)
                raw=(path.parent/unquote(document.buffers[view.buffer].uri)).read_bytes()
                values=struct.unpack_from('<6f',raw,view.byteOffset)
                self.assertEqual(values[-3:],(0.,0.,-2.))
                for buffer in document.buffers:
                    self.assertTrue((path.parent/unquote(buffer.uri)).is_file())
            self.assertEqual(targets,[2,41])
            self.assertEqual(before,{f:(output/f).read_bytes() for f in before})
            repeated=export_animations(source,source.catalog(),models,output)
            self.assertEqual(exports,repeated)
            controller=next(e for e in exports if e.source.path_id==12)
            self.assertEqual(controller.details['state_clip_links'][0]['clip'],{'file':source.file,'path_id':13})

    def test_morph_channel_uses_shared_mesh_and_flat_scalar_weights(self):
        from unittest.mock import patch
        from tools.reference.library.models import export_models
        from tools.reference.library.animations import export_animations
        from tools.reference.gltf import validate_gltf
        source=self.package_fixture(morph=True)
        with tempfile.TemporaryDirectory() as temp:
            output=Path(temp)
            with patch('tools.reference.library.models.MeshHandler',side_effect=lambda r:source.handlers[r.path_id]):
                models=export_models(source,source.catalog(),output)
            exports=export_animations(source,source.catalog(),models,output)
            clip=next(e for e in exports if e.source.path_id==13)
            path=output/clip.details['packages'][0]['file'];document=validate_gltf(path)
            channel=document.animations[0].channels[0]
            self.assertEqual(channel.target.path,'weights')
            a=document.accessors[document.animations[0].samplers[0].output]
            self.assertEqual(a.type,'SCALAR')
            self.assertEqual(a.count,2)
            self.assertEqual(a.min,[0.]);self.assertEqual(a.max,[1.])

    def test_stream_steps_keep_coefficients_and_object_start_sentinel(self):
        pointer=NS(m_FileID=0,m_PathID=9)
        record=decode_clip(*fixture([binding(5,114,pptr=1)],frames=[(-3.4e38,[(0,(0.,0.,0.),0.)]),
                         (1.,[(0,(0.,0.,0.),-1.)]),(math.inf,[])],streamed_count=1,mapping=(pointer,)))
        self.assertEqual(record.tracks[0].times,(0.,1.))
        self.assertEqual(record.tracks[0].binding['object_references'],[{'file':'test.assets','path_id':9},None])
        self.assertEqual(record.tracks[0].coefficients,(((0.,0.,0.),),((0.,0.,0.),)))
        frames=[(-3.4e38,[]),(0.,[(i,(0.,0.,0.),0.) for i in range(3)]),
                (1.,[(i,(0.,0.,0.),1.) for i in range(3)]),(math.inf,[])]
        track=decode_clip(*fixture([binding()],frames=frames,streamed_count=3)).tracks[0]
        self.assertEqual(sample_track(track,.999),(0.,0.,0.))
        self.assertEqual(bake_track(track)[2],'STEP')

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
