"""Export original sound clips and the source sound rules."""
import hashlib
import io
import re
from .source import key, source_id
from .collision import world_pose, bake_points

# Unity layers mainscript.sandLayer and mainscript.sandLayer2.
SAND_LAYERS={8,23}
# mainscript.walkSoundType order.
SURFACES=('road','desert','winter','grass','wood')
# Sample rates that MPEG audio supports.
MP3_RATES=(8000,11025,12000,16000,22050,24000,32000,44100,48000)


class Clips:
    def __init__(self,source,output):
        self.source=source;self.output=output;self.records=[];self.ids={}
        output.mkdir(parents=True,exist_ok=True)

    def add(self,owner,pointer,compressed=False):
        """A compressed clip is MP3. Radio songs would take gigabytes as WAV."""
        reader=self.source.resolve(owner,pointer)
        if reader is None:return None
        if key(reader) in self.ids:return self.ids[key(reader)]
        clip=reader.parse_as_object()
        if clip.m_AudioData:data=bytes(clip.m_AudioData)
        else:
            resource=clip.m_Resource
            with (self.source.directory/resource.m_Source.split('/')[-1]).open('rb') as stream:
                stream.seek(resource.m_Offset);data=stream.read(resource.m_Size)
        if len(data)==0:raise ValueError(f'Clip has no data: {clip.m_Name}')
        from fmod_toolkit import raw_to_wav
        samples=list(raw_to_wav(data,clip.m_Name,clip.m_Channels or 2,clip.m_Frequency or 44100).values())
        if len(samples)!=1:raise ValueError(f'Clip must have one sample set: {clip.m_Name}')
        data=samples[0]
        if compressed:
            import soundfile
            sound,rate=soundfile.read(io.BytesIO(data))
            if rate not in MP3_RATES:
                import numpy
                target=next(r for r in MP3_RATES if r>=rate)
                time=numpy.arange(round(len(sound)*target/rate))*rate/target
                frames=numpy.arange(len(sound))
                sound=numpy.stack([numpy.interp(time,frames,c) for c in sound.reshape(len(sound),-1).T],axis=1);rate=target
            stream=io.BytesIO();soundfile.write(stream,sound,rate,format='MP3');data=stream.getvalue()
        name=f"{len(self.records)}-{re.sub(r'[^A-Za-z0-9_-]+','_',clip.m_Name)}.{'mp3' if compressed else 'wav'}"
        (self.output/name).write_bytes(data)
        self.ids[key(reader)]=len(self.records)
        self.records.append({'id':len(self.records),'source':source_id(reader),'name':clip.m_Name,'path':f'reference/audio/{name}',
            'seconds':clip.m_Length,'sha256':hashlib.sha256(data).hexdigest()})
        return self.ids[key(reader)]

    def many(self,owner,pointers):
        return [self.add(owner,pointer) for pointer in pointers if pointer['m_PathID']]


def export_audio(source,manifest,output):
    clips=Clips(source,output/'audio')
    nodes={(n['source']['file'],n['source']['path_id']):n for n in manifest['nodes']}
    item_of_node={item['node']:item['id'] for item in manifest['items']}
    def node_for(reader):
        go=source.resolve(reader,source.read(reader)['m_GameObject'])
        return nodes[key(source.transform_for(go))]
    def item_for(node):
        while node['id'] not in item_of_node:node=manifest['nodes'][node['parent']-1]
        return item_of_node[node['id']]
    def offset(item,reader):
        transform=source.transform_for(source.resolve(reader,source.read(reader)['m_GameObject']))
        node=manifest['nodes'][next(i for i in manifest['items'] if i['id']==item)['node']-1]
        pose=world_pose(source,source.get(node['source']['file'],node['source']['path_id']))
        return bake_points([(0.,0.,0.)],source.chain(transform),pose)[0]
    def voice(owner,pointer,item=None,clip=True):
        reader=source.resolve(owner,pointer);value=source.read(reader)
        spatial=value['panLevelCustomCurve']['m_Curve'][0]['value']
        if value['rolloffMode']!=0 or len(value['panLevelCustomCurve']['m_Curve'])!=1:
            raise ValueError(f'Unsupported audio source: {key(reader)}')
        if item is None:item=item_for(node_for(reader))
        return {'source':source_id(reader),'clip':clips.add(reader,value['m_audioClip']) if clip else None,'volume':value['m_Volume'],
            'min_distance':value['MinDistance'],'max_distance':value['MaxDistance'],'spatial':spatial,'looping':bool(value['Loop']),
            'doppler':value['DopplerLevel'],'item':item,'offset':offset(item,reader)}
    main_reader=source.get('level1',64919);main=source.read(main_reader)
    player_reader=source.get('level1',80125);player=source.read(player_reader)
    car_root=manifest['nodes'][manifest['vehicle']['root']-1]
    car_transform=source.get(car_root['source']['file'],car_root['source']['path_id'])
    car_parts={}
    for entry in source.hierarchy(car_transform,activate_root=True):
        for reader in entry['components']:
            if reader.type.name=='MonoBehaviour':car_parts.setdefault(source.class_name(reader),[]).append(reader)
    car_reader=car_parts['carscript'][0];car=source.read(car_reader)
    engine_reader=car_parts['enginescript'][0];engine=source.read(engine_reader)
    prefab=source.read(next(p for p in source.components(source.transform_for(source.resolve(main_reader,main['Audio'])))[1] if p.type.name=='AudioSource'))
    record={'clips':clips.records}
    record['one_shot']={'min_distance':prefab['MinDistance'],'max_distance':prefab['MaxDistance'],
        'spatial':prefab['panLevelCustomCurve']['m_Curve'][0]['value'],'doppler':prefab['DopplerLevel'],'random_pitch':[0.85,1.15]}
    # fpscontroller.SetOutSideAudio and mainscript.SetVolume.
    record['buses']={'default_interior':player['defInteriorM'],'interior_lerp':player['interiorLerpM'],'car_multiplier':player['FCarSoundM']}
    engine_item=manifest['engine']['item'];car_item=manifest['vehicle']['item']
    record['engine']={'idle':voice(engine_reader,engine['SAlap'],engine_item),'motor':voice(engine_reader,engine['SMotor'],engine_item),
        'start':clips.add(engine_reader,engine['SStart']),'start_volume':engine['SStartVolume'],
        'stop':clips.add(engine_reader,engine['SStop']),'stop_volume':engine['SStopVolume'],
        'idle_rpm':[engine['SMinAlapRpm'],engine['SMaxAlapRpm']],'idle_pitch':[engine['minAlapPitch'],engine['maxAlapPitch']],'idle_volume':engine['SMaxAlapSound'],
        'motor_rpm':[engine['SMinMotorRpm'],engine['SMaxMotorRpm']],'motor_pitch':[engine['minMotorPitch'],engine['maxMotorPitch']],'motor_volume':engine['SMaxMotorSound'],
        'reverse_volume':engine['SReverseVolume']}
    record['cooling']={'clips':clips.many(main_reader,main['CarCoolingSounds']),'temperature':[main['minCoolSoundTemp'],main['maxCoolSoundTemp']],
        'time':[[main['minCoolTimeMin'],main['minCoolTimeMax']],[main['maxCoolTimeMin'],main['maxCoolTimeMax']]],
        'volume':[[main['minCoolVolumeMin'],main['minCoolVolumeMax']],[main['maxCoolVolumeMin'],main['maxCoolVolumeMax']]],
        'pitch':[[main['minCoolPitchMin'],main['minCoolPitchMax']],[main['maxCoolPitchMin'],main['maxCoolPitchMax']]]}
    wheels=[]
    for reader in car_parts['talajnezo']:
        value=source.read(reader)
        wheels.append({name:voice(reader,value[field],car_item) for name,field in (('ground','GroundSource'),('spring','SpringSource'),('slide','SlideSource'))})
    if len(wheels)!=4 or any({k:{f:v for f,v in w[k].items() if f not in ('source','offset')} for k in w}!={k:{f:v for f,v in wheels[0][k].items() if f not in ('source','offset')} for k in wheels[0]} for w in wheels):
        raise ValueError('Wheel sound sources must match')
    record['car']={'collide':[clips.many(main_reader,main[f'collide{i}']) for i in range(4)],'collision_modifier':car['collisionSoundM'],
        'brake':voice(car_reader,car['SBrake'],car_item),'brake_modifier':car['brakeSoundM'],
        'reverse':voice(car_reader,car['SReverseSound'],car_item),
        'handbrake':clips.add(car_reader,car['handbrakeSound']),'handbrake_voice':voice(car_reader,car['SValtoSound'],car_item,clip=False),
        'boil':voice(car_reader,car['CoolantBoil'],car_item),'boil_volume':car['coolantBoilMaxSound'],
        # Wheel sources follow the wheel positions.
        'wheel':{k:{**wheels[0][k],'offset':[0.,0.,0.]} for k in wheels[0]},
        'ground_modifier':car['groundSoundM'],'spring_modifier':car['springSoundM'],'spring_pitch_modifier':car['springPitchM'],
        'slide_angle':car['slideAngle'],'max_slide_volume_speed':car['maxSlideVolumeSpeed'],
        'skid_road':{'clip':clips.add(main_reader,main['SSkidRoad']),'volume':main['SkidRoadVolume'],'pitch':[main['SkidRoadPitchMin'],main['SkidRoadPitchMax']]},
        'skid_dirt':{'clip':clips.add(main_reader,main['SSkidDirt']),'volume':main['SkidDirtVolume'],'pitch':[main['SkidDirtPitchMin'],main['SkidDirtPitchMax']]},
        'speed_difference':[main['CarMinSpeedDif'],main['CarMaxSpeedDif']],
        # carscript.HornHit without blood on the car.
        'horn':{'voice':voice(car_reader,car['SHornSound'],car_item),'pitch':[car['HornPitchMin'],car['HornPitchMax']],
            'crash_multiplier':car['crashHornMultiplier'],'crash_speed':[main['crashSpeedMinHorn'],main['crashSpeedMaxHorn']],
            'crash_chance':main['crashHornChance'],'crash_seconds':[0.3,1.0]}}
    steps=source.read(source.resolve(player_reader,player['stepScript']))
    surfaces=[]
    for collider in manifest['colliders']:
        node=manifest['nodes'][collider['node']-1]
        game_object,parts=source.components(source.get(node['source']['file'],node['source']['path_id']))
        go=source.read(game_object)
        other=[p for p in parts if p.type.name=='MonoBehaviour' and source.class_name(p)=='otherStepSoundScript']
        if other:surfaces.append({'collider':collider['id'],'surface':SURFACES[source.read(other[0])['tip']]})
        elif go['m_Layer'] in SAND_LAYERS or go['m_Tag']=='Sand':surfaces.append({'collider':collider['id'],'surface':'terrain'})
        elif go['m_Tag']=='Wood':surfaces.append({'collider':collider['id'],'surface':'wood'})
    record['steps']={'surfaces':[{'clips':clips.many(main_reader,value['clips']),'volume':value['volume']} for value in main['walkSounds']],
        'modifier':steps['stepSound'],'ray_distance':steps['rayDist'],'bob_speed':player['walkBobSpeed'],'bob_speed_velocity':player['walkBobSpeedRB'],
        'bob_height':player['maxWalkBob'],'bob_threshold':player['walkbobSoundF'],'run_bonus':0.15,'colliders':surfaces}
    if len(record['steps']['surfaces'])!=len(SURFACES):raise ValueError('Walk sounds must match walkSoundType')
    napszak=source.read(source.resolve(main_reader,main['napszak']))
    record['wind']={'voice':voice(player_reader,player['SwindSource'],car_item),'max_speed_kmh':player['FwindVolumeMaxSpeed'],
        'lerp':player['FwindVolumeLerp'],'view_modifier':player['CamViewWindVolumeModifier']}
    record['ambient']={'day':[voice(main_reader,p,car_item) for p in napszak['DayAmbientSound']],'night':[voice(main_reader,p,car_item) for p in napszak['NightAmbientSound']],
        'day_volume':[napszak['dayAmbientSoundVolumeDay'],napszak['dayAmbientSoundVolumeNight']],
        'night_volume':[napszak['nightAmbientSoundVolumeDay'],napszak['nightAmbientSoundVolumeNight']],'fade_rate':0.1}
    # Two-dimensional sources follow the listener.
    for value in [record['wind']['voice'],*record['ambient']['day'],*record['ambient']['night']]:
        if value['spatial']!=0.:raise ValueError('Wind and ambient sources must be two-dimensional')
        value['item']=None;value['offset']=[0.,0.,0.]
    record['interiors']=export_interiors(source,manifest,nodes)
    hinges=[]
    for hinge in manifest['hinges']:
        reader=source.get(hinge['source']['file'],hinge['source']['path_id']);value=source.read(reader)
        squeak=None
        if value['moveSound'] and value['SSource']['m_PathID']:
            # usablescript.RotMoveSound: only the Y axis rolls the skip chance.
            y_axis=value['minY']!=value['maxY']
            squeak={'clip':clips.add(reader,value['SMove']),'voice':voice(reader,value['SSource'],hinge['item'],clip=False),
                'skip_chance':(value['dontSqueakChanceOverride'] if value['useDontSqueakChanceOverride'] else 0.75) if y_axis else 0.}
        hinges.append({'item':hinge['item'],'min_open':clips.add(reader,value['startPosStart']),'min_close':clips.add(reader,value['startPosEnd']),
            'max_open':clips.add(reader,value['endPosStart']),'max_close':clips.add(reader,value['endPosEnd']),'modifier':value['audioM'],'inverted':bool(value['invY']),
            'squeak':squeak})
    record['hinges']=hinges
    openings=[]
    for opening in manifest['openings']:
        reader=source.get(opening['source']['file'],opening['source']['path_id']);value=source.read(reader)
        openings.append({'opening':opening['id'],'item':opening['cap'],'open':clips.add(reader,value['openSound']),'open_volume':value['openVolume'],
            'close':clips.add(reader,value['closeSound']),'close_volume':value['closeVolume'],
            'tap':clips.add(reader,value['tekerSound']),'pour':voice(reader,value['SSource'],opening['cap'])})
    record['openings']=openings
    record['pour_speed']=[main['minPourSpeedSound'],main['maxPourSpeedSound']]
    record['radio']=export_radio(source,manifest,clips,main_reader,main,car_parts,voice)
    items=[]
    for item in manifest['items']:
        node=manifest['nodes'][item['node']-1]
        _,parts=source.components(source.get(node['source']['file'],node['source']['path_id']))
        for reader in parts:
            if reader.type.name!='MonoBehaviour' or source.class_name(reader)!='pickupable':continue
            value=source.read(reader)
            if value['collideSound']['m_PathID']:
                items.append({'item':item['id'],'clip':clips.add(reader,value['collideSound']),'modifier':value['collisionSoundM']})
    record['items']=items
    menu_reader=source.get('level0',17131);menu=source.read(menu_reader);sound=menu['sounds'][0]
    record['menu']={'pitch':[menu['minPitch'],menu['maxPitch']],'volume':sound['volume'],'touch_volume':sound['volumeT'],
        'down':clips.many(menu_reader,sound['soundDown']),'up':clips.many(menu_reader,sound['soundUp']),
        'touch_down':clips.many(menu_reader,sound['soundTDown']),'touch_up':clips.many(menu_reader,sound['soundTUp'])}
    return record


def export_interiors(source,manifest,nodes):
    """interiorscript records. A door that is not a modeled hinge stays closed."""
    readers=[]
    for collider in manifest['colliders']:
        node=manifest['nodes'][collider['node']-1]
        _,parts=source.components(source.get(node['source']['file'],node['source']['path_id']))
        for reader in parts:
            if reader.type.name=='MonoBehaviour' and source.class_name(reader)=='interiorscript':readers.append((reader,collider['id']))
    hinge_items={(h['source']['file'],h['source']['path_id']):h['item'] for h in manifest['hinges']}
    slot_items={(p['source']['file'],p['source']['path_id']):p['item'] for p in manifest['parts']}
    order={key(r):i for i,(r,_) in enumerate(readers)}
    def door(owner,pointer,removable):
        reader=source.resolve(owner,pointer)
        if removable:return {'item':slot_items[key(reader)],'removable':True}
        return {'item':hinge_items.get(key(reader)),'removable':False}
    result=[]
    for reader,collider in readers:
        value=source.read(reader)
        parent=source.resolve(reader,value['parent'])
        result.append({'source':source_id(reader),'collider':collider,'parent':order[key(parent)] if parent else None,
            'doors':[door(reader,p,bool(value['removableDoor'])) for p in (value['doorSlots'] if value['removableDoor'] else value['doors'])],
            'parent_door':door(reader,value['parentDoorSlot'] if value['removableDoor'] else value['parentDoor'],bool(value['removableDoor'])) if (value['parentDoorSlot'] if value['removableDoor'] else value['parentDoor'])['m_PathID'] else None,
            'min_doors_to_max':value['minDoorToMax'],'max_value':value['maxAtengedes'],'value':value['value'] if value['useValue'] else None,
            'inverted':bool(value['invert']),'vehicle':nodes[(manifest['nodes'][manifest['colliders'][collider-1]['node']-1]['source']['file'],manifest['nodes'][manifest['colliders'][collider-1]['node']-1]['source']['path_id'])]['asset']==2})
        if value['invertindividual'] or value['invertindividual2']:
            if any(value['invertindividual']+value['invertindividual2']):raise ValueError('Inverted interior doors are not supported')
    return result


def export_radio(source,manifest,clips,main_reader,main,car_parts,voice):
    """radioscript and the mainscript stations. Custom radio and holiday clips are left out."""
    reader=car_parts['radioscript'][0];value=source.read(reader)
    colliders={(manifest['nodes'][c['node']-1]['source']['file'],manifest['nodes'][c['node']-1]['source']['path_id']):c['id'] for c in manifest['colliders']}
    def button(pointer):
        usable=source.resolve(reader,pointer)
        return colliders[key(source.transform_for(source.resolve(usable,source.read(usable)['m_GameObject'])))]
    def entries(clips_field,volumes,holidays):
        result=[]
        for i,pointer in enumerate(clips_field):
            if not pointer['m_PathID'] or holidays[i]!=0:continue
            volume=volumes[i] if i<len(volumes) and volumes[i]!=0. else 1.
            result.append({'clip':clips.add(main_reader,pointer,compressed=True),'volume':volume})
        return result
    def stations(field):
        return [{'name':ch['name'],'frequency':ch['FR'],'custom':bool(ch['custom']),
            'music':entries(ch['music'],ch['musicVolume'],ch['musicHoliday']),
            'talk':entries(ch['talk'],ch['talkVolume'],ch['talkHoliday'])} for ch in main[field]]
    main_object=key(source.resolve(reader,value['DefMainObj']))
    radio=next(item['id'] for item in manifest['items'] if (item['source']['file'],item['source']['path_id'])==main_object)
    return {'item':radio,'volume_button':button(value['Bvolume']),'tune_button':button(value['Btune']),'band_button':button(value['BChangeAMFM']),
        'am':[value['AMmin'],value['AMmax']],'fm':[value['FMmin'],value['FMmax']],'am_step':value['AMStep'],'fm_step':value['FMStep'],
        'tune':value['currentTeker'],'volume_step':value['vSpeed'],'turn':clips.add(reader,value['Cturned']),'turn_volume':value['turnVolume'],
        'noise':voice(reader,value['SS'],radio),'channel':voice(reader,value['SC'],radio),'second':voice(reader,value['SC2'],radio),
        'tolerance_fm':main['FRTFM'],'tolerance_am':main['FRTAM'],'fm_stations':stations('radioChs'),'am_stations':stations('radioChsAM')}
