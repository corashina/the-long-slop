"""Selected source audio bindings. Runtime queue eviction is a bounded adapter."""
import json
from pathlib import Path
from .source import LibrarySource, key, source_id
from .survival import managed_evidence, finite_record, component
from .collision import world_pose

def recover_audio(source: LibrarySource, output: Path) -> dict:
    from .library.reader import checked
    from .library.audio import validate_wav
    from .library.types import ObjectId
    library=source.library
    library.selected_audio=set()
    index=next(v for v in library.manifest['category_indexes'] if v['file']=='audio/index.json')
    records=json.loads(checked(library.root,index))
    by_source={(r['source']['file'],r['source']['path_id']):r for r in records}
    main=source.get('level1',64919);md=source.read(main)
    player=source.get('level1',80125);pd=source.read(player)
    prefab=component(source,source.transform_for(source.resolve(main,md['Audio'])),'AudioSource')
    sounds=source.get('level1',70770)
    if source.class_name(sounds)!='soundMainScript':raise ValueError('Toilet source differs')
    groups=[('eat',0,main,md['soundEat'],prefab,[md['eatVolume']]*2,[1.,1.])]
    for role,kind,field,group in zip(('burp','fart','shit'),(1,2,9),('SBurp','SFart','SFart'),source.read(sounds)['clips'],strict=True):
        prefix='Burp' if role=='burp' else 'Fart'
        groups.append((role,kind,sounds,group['clips'],source.resolve(player,pd[field]),[pd['min'+prefix+'Volume'],pd['max'+prefix+'Volume']],[pd['min'+prefix+'Pitch'],pd['max'+prefix+'Pitch']]))
    groups.append(('death',5,main,[md['SdeathBell']],source.resolve(player,pd['SDeathBellSource']),[1.,1.],[.8,1.2]))
    storm=source.get('sharedassets1.assets',158276)
    cycle=source.resolve(main,md['napszak']);cd=source.read(cycle)
    for role,kind,owner,pointer in [('sleep',4,player,pd['SleepCamSource']),('storm',6,storm,source.read(storm)['source']),('ambient_day',7,cycle,cd['DayAmbientSound'][0]),('ambient_night',8,cycle,cd['NightAmbientSound'][0])]:
        audio=source.resolve(owner,pointer);ad=source.read(audio)
        groups.append((role,kind,audio,[ad['m_audioClip']],audio,[ad['m_Volume']]*2,[ad['m_Pitch']]*2))
    mixer=source.get('sharedassets0.assets',810);mc=source.read(mixer)['m_MixerConstant']
    snapshot=mc['snapshots'][0]['values']
    bindings=[];clips=[];clip_ids={}
    for role,kind,owner,pointers,audio,gain,pitch in groups:
        ids=[]
        for pointer in pointers:
            clip=source.resolve(owner,pointer)
            if clip is None or clip.type.name!='AudioClip':raise ValueError('Missing selected audio clip')
            source.read(clip);identity=ObjectId(*key(clip));library.selected_audio.add(identity)
            if key(clip) not in clip_ids:
                record=by_source[key(clip)]
                if record['status']!='converted' or len(record['details']['samples'])!=1:raise ValueError('Missing source WAV')
                sample=record['details']['samples'][0];file=next(f for f in record['files'] if f['file']==sample['file'])
                payload=checked(library.root,file);name=f'survival-audio-{clip.assets_file.name}-{clip.path_id}.wav'
                (output/name).write_bytes(payload)
                metadata=validate_wav(output/name,source.read(clip))
                if any(metadata[k]!=sample[k] for k in ("frames","frequency","channels","duration_seconds")):raise ValueError("Selected WAV metadata differs")
                clip_ids[key(clip)]=len(clips)
                clips.append({'id':len(clips),'source':source_id(clip),'path':'reference/'+name,'sha256':file['sha256'],**{k:sample[k] for k in ['duration_seconds','frames','frequency','channels']}})
            ids.append(clip_ids[key(clip)])
        ad=source.read(audio)
        group=source.resolve(player,pd['deathbellGroup']) if role=='death' else source.resolve(audio,ad['OutputAudioMixerGroup'])
        gd=source.read(group);group_index=mc['groupGUIDs'].index(gd['m_GroupID']);route=[];db=0.;route_pitch=1.
        while group_index>=0:
            if any(e['groupConstantIndex']==group_index for e in mc['effects']):raise ValueError('Selected mixer route needs unsupported processing')
            g=mc['groups'][group_index]
            if g['mute'] or g['solo'] or g['bypassEffects']:raise ValueError('Selected mixer state differs')
            db+=snapshot[g['volumeIndex']];route_pitch*=snapshot[g['pitchIndex']];route.append(group_index);group_index=g['parentConstantIndex']
        blend=ad['panLevelCustomCurve']['m_Curve']
        if len(blend)!=1 or blend[0]['value'] not in (0.,1.):raise ValueError('Selected spatial blend differs')
        transform=source.transform_for(source.resolve(audio,ad['m_GameObject']))
        pose=world_pose(source,transform)
        player_pose=world_pose(source,source.transform_for(source.resolve(player,pd['m_GameObject'])))
        position=[pose['translation'][i]-player_pose['translation'][i] for i in range(3)] if role in ('burp','fart','shit','death','sleep') else [0.,0.,0.]
        bindings.append({'role':role,'kind':kind,'source':source_id(owner),'audio_source':source_id(audio),'transform':source_id(transform),'offset':position,
            'clips':ids,'gain':gain,'pitch':pitch,'looped':bool(ad['Loop']) if role!='death' else False,
            'spatial_blend':1. if role=='eat' else blend[0]['value'],'min_distance':ad['MinDistance'],'max_distance':ad['MaxDistance'],
            'rolloff_mode':ad['rolloffMode'],'rolloff_curve':ad['rolloffCustomCurve']['m_Curve'],'priority':70 if role=='eat' else ad['Priority'],
            'mixer':source_id(group),'mixer_route':route,'mixer_gain':10**(db/20),'mixer_pitch':route_pitch,
            'clip_length_lifetime':role=='eat','doppler':ad['DopplerLevel'],'spread':ad['spreadCustomCurve']['m_Curve'][0]['value'],'pan':ad['Pan2D']})
    manager=source.get('globalgamemanagers',4);manager_data=source.read(manager)
    evidence=managed_evidence(source,['component.priority = priority;','UnityEngine.Object.Destroy(component.gameObject, component.clip.length);','SFart.Play();','SBurp.Play();','SDeathBellSource.pitch = UnityEngine.Random.Range(0.8f, 1.2f);'])
    # AudioPriorities[4] is the default one-shot priority.
    managed_evidence(source,['public static byte[] AudioPriorities = new byte[8] { 10, 25, 40, 55, 70, 80, 85, 100 };'])
    result={'bindings':bindings,'clips':clips,'manager':source_id(manager),'real_voices':manager_data['m_RealVoiceCount'],'virtual_voices':manager_data['m_VirtualVoiceCount'],'queue_capacity':512,'evidence':evidence}
    finite_record(result)
    return result


def validate_audio_export(record: dict, output: Path, library) -> None:
    """Reject edited bindings or WAV metadata against the accepted source."""
    from tempfile import TemporaryDirectory
    from .library.reader import LibraryReader
    source=LibrarySource(LibraryReader(library))
    with TemporaryDirectory() as temporary:
        expected=recover_audio(source,Path(temporary))
    if record!=expected:raise ValueError('Selected audio contract differs from source')
    from .library.audio import validate_wav
    for clip in record['clips']:
        owner=source.get(clip['source']['file'],clip['source']['path_id'])
        metadata=validate_wav(output/Path(clip['path']).name,source.read(owner))
        if any(metadata[k]!=clip[k] for k in ('frames','frequency','channels','duration_seconds')):raise ValueError('Runtime WAV metadata differs')
