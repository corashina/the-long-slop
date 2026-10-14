"""Recover selected storm, shelter, and particle data from checked records."""
from .source import source_id,key,vector
from .survival import component,managed_evidence,snake,finite_record
from .lighting import gradient,rgba,sky_material
from .collision import geometry,world_pose,quaternion_product,bake_points
from .gltf import GltfExporter

STORM_FIELDS='lifeLossOnOtherBiome lifeGain destroyDist life radius fogDensity atmosphereThickness intensityMultipler ambientMultipler volume ambientDecrease yoffset windForce defSRateT defSRateD defBRateT defBRateD'.split()

def curve(data):
    def keys(c):
        return {'keys':[{'time':k['time'],'value':k['value'],'in_tangent':k['inSlope'],'out_tangent':k['outSlope'],
            'in_weight':k['inWeight'],'out_weight':k['outWeight'],'weighted_mode':k['weightedMode']} for k in c['m_Curve']],
            'pre_wrap_mode':c['m_PreInfinity'],'post_wrap_mode':c['m_PostInfinity']}
    return {'mode':data['minMaxState'],'min_scalar':data['minScalar'],'scalar':data['scalar'],
        'min_curve':keys(data['minCurve']),'max_curve':keys(data['maxCurve'])}

def color(data):
    return {'mode':data['minMaxState'],'min_color':rgba(data['minColor']),'max_color':rgba(data['maxColor']),
        'min_gradient':gradient(data['minGradient']),'max_gradient':gradient(data['maxGradient'])}

def particle(source,reader,role,rate,output,owner=None):
    d=source.read(reader);root=source.transform_for(source.resolve(reader,d['m_GameObject']))
    renderer=component(source,root,'ParticleSystemRenderer');rd=source.read(renderer)
    result={'source':source_id(reader),'role':role,'capacity':d['InitialModule']['maxNumParticles'],
        'rate_over_time':rate,'rate_over_distance':d['EmissionModule']['rateOverDistance']['scalar'],
        'duration':d['lengthInSec'],'simulation_space':d['moveWithTransform'],'local_pose':relative_pose(source,root,owner) if owner is not None else world_pose(source,root),
        'module_flags':{k:bool(v['enabled']) for k,v in d.items() if isinstance(v,dict) and 'enabled' in v},
        'curves':{},'colors':{},'vectors':{},'scalars':{},'flags':{},'links':{},
        'materials':[sky_material(source,source.resolve(renderer,p),output) for p in rd['m_Materials'] if p['m_PathID']],
        'limits':[]}
    def visit(value,path):
        if isinstance(value,dict):
            if 'minMaxState' in value:
                target='curves' if 'scalar' in value else 'colors'
                result[target][path]=curve(value) if target=='curves' else color(value)
            elif set(value)=={'x','y','z'}:result['vectors'][path]=list(vector(value))
            elif set(value)=={'m_FileID','m_PathID'}:
                linked=source.resolve(reader,value)
                if linked:result['links'][path]=source_id(linked)
            else:
                for k,v in value.items():visit(v,f'{path}.{k}' if path else k)
        elif isinstance(value,list):
            for i,v in enumerate(value):visit(v,f'{path}.{i}')
        elif isinstance(value,bool):result['flags'][path]=value
        elif isinstance(value,(int,float)):result['scalars'][path]=value
        else:raise ValueError(f'Unsupported particle field: {path}')
    for k,v in d.items():
        if isinstance(v,dict) and 'enabled' in v and not v['enabled']:continue
        visit(v,k)
    result['flags']['gameObject.active']=bool(source.read(source.resolve(reader,d['m_GameObject']))['m_IsActive'])
    result['flags']['renderer.enabled']=bool(rd['m_Enabled'])
    result['scalars']['gameObject.layer']=source.read(source.resolve(reader,d['m_GameObject']))['m_Layer']
    for k in ['m_RenderMode','m_SortMode','m_MinParticleSize','m_MaxParticleSize','m_CameraVelocityScale','m_VelocityScale','m_LengthScale','m_SortingFudge','m_RenderAlignment','m_Pivot','m_Flip']:
        visit(rd[k],'renderer.'+k)
    if result['module_flags']['CollisionModule']:
        result['limits'].append({'source':source_id(reader),'module':'CollisionModule','visible_effect':'Retained ParticleState spray pools and FluidContact outlet pools cast masked non-sensor Rapier swept spheres, apply bounce, dampen and lifetime loss, and remove initial penetration. Native collision cache, collision messages and world tank-fill callbacks remain outside this route. Native particle trigger membership remains unproved; ray queriesHitTriggers does not govern this adaptation.'})
    if result['module_flags'].get('TriggerModule'):
        result['limits'].append({'source':source_id(reader),'module':'TriggerModule','visible_effect':'Retained shelter AABBs adapt native particle collider overlap, including radiusScale.'})
    for m in result['materials']:
        if m['shader']['name']=='Legacy Shaders/Particles/Alpha Blended':
            result['limits'].append({'source':source_id(reader),'module':'material.soft_particles','visible_effect':'Source Ultra enables soft particles. ParticleMaterial has no sampled scene-depth binding; dust intersections keep hard depth edges.'})
        if m['floats'].get('_DistortionEnabled')==1:
            result['limits'].append({'source':source_id(reader),'module':'material.distortion','visible_effect':'Material sharedassets1.assets:481 uses EFFECT_BUMP with distortion strength 50. ParticleMaterial has no scene-color binding; fluid sprites do not refract the background.'})
    return result

def recover_weather(source, output):
    source.particle_output=output
    main=source.get('level1',64919);scheduler=source.resolve(main,source.read(main)['sandStormSpawn']);sd=source.read(scheduler)
    database=source.get('level1',66024)
    root=source.transform_for(source.resolve(database,source.read(database)['storms'][sd['prefabid']]))
    storm=component(source,root,'sandstormscript');d=source.read(storm)
    if key(storm)!=('sharedassets1.assets',158276):raise ValueError('Storm database link differs')
    record={'source':source_id(scheduler),'min_interval_seconds':sd['minTime'],'max_interval_seconds':sd['maxTime'],'spawn_radius':sd['radius'],
        'storm':{'source':source_id(storm),'biome':d['biome'],**{snake(k):d[k] for k in STORM_FIELDS},
            **{snake(k):rgba(d[k]) for k in ['nightColor','skyColor','skyGroundColor','fogColor','CloudColor']},
            'day_color1':gradient(d['dayColor1']),'day_color2':gradient(d['dayColor2']),'wind_lm':d['windLm']['m_Bits'],
            'current_force':list(vector(d['currentForce'])),'next_force':list(vector(d['nextForce'])),'distance_lerp':d['distanceLerp'],
            'size_range':[.5,1.5],'speed_range':[3.,12.],'wind_time_range':[.1,1.5],'wind_scan_seconds':1.,'wind_bounds_range':[1.,30.]},
        'evidence':managed_evidence(source,['size = UnityEngine.Random.Range(0.5f, 1.5f);','speed = UnityEngine.Random.Range(3f, 12f);',
            'yield return new WaitForSeconds(1f);','nextWindTime = Time.time + UnityEngine.Random.Range(0.1f, 1.5f);',
            'mainscript.M.interiorCols[j].transform.root != windRbs[i].transform.root']), 'particles':[]}
    for role,field,rate in [('storm_big','bigParticleSystem',d['defBRateT']),('storm_small','smallParticleSystem',d['defSRateT'])]:
        record['particles'].append(particle(source,source.resolve(storm,d[field]),role,rate,output,root))
    player=source.get('level1',80125);outlet=source.resolve(player,source.read(player)['piss']);od=source.read(outlet)
    player_root=source.transform_for(source.resolve(outlet,od['m_GameObject']))
    for role,field in [('urine','ps'),('urine_splash','sps')]:
        reader=source.resolve(outlet,od[field]);rate=od['maxRate'] if field=='ps' else source.read(reader)['EmissionModule']['rateOverTime']['scalar']
        record['particles'].append(particle(source,reader,role,rate,output,player_root))
    for p in record['particles'][2:]:
        outlet_values(source,p,od)
    record['ambient']=recover_ambient(source)
    source.lighting['textures']=list(source.lighting_textures.values())
    source.weather=record
    return record

def bind_weather(source,manifest):
    record=source.weather;record['shelters']=[]
    nodes={(n['source']['file'],n['source']['path_id']):n for n in manifest['nodes']}
    exporter=GltfExporter(source,None,source.library)
    from .export import CARS
    for name in ('home',*CARS):
        root=source.survival_roots[name][0];body=next(b for b in manifest['bodies'] if b['node']==nodes[key(root)]['id'])
        for node in source.hierarchy(root,activate_root=True):
            for c in node['components']:
                if c.type.name!='MonoBehaviour' or source.class_name(c)!='interiorscript':continue
                d=source.read(c);collider=source.resolve(c,d['col'])
                if collider is None:
                    collider=next(r for r in node['components'] if r.type.name in ('BoxCollider','SphereCollider','CapsuleCollider','MeshCollider'))
                pose=world_pose(source,node['reader']);pose['translation']=bake_points([(0.,0.,0.)],source.chain(node['reader']),body['pose'])[0]
                inverse=[-v for v in body['pose']['rotation'][:3]]+[body['pose']['rotation'][3]]
                pose['rotation']=list(quaternion_product(inverse,pose['rotation']))
                pose['scale']=[1.,1.,1.]
                record['shelters'].append({'source':source_id(c),'root':body['id'],
                    'geometry':geometry(source,exporter,collider,node['reader'],world_pose(source,node['reader']),False),'local_pose':pose})
    record['particles'].extend(recover_outlet_particles(source,manifest,source.particle_output))
    manifest['lighting']['textures']=list(source.lighting_textures.values())
    manifest['weather']=record
    validate_weather_export(record,manifest)

def validate_weather_export(record,manifest):
    finite_record(record)
    if not 0<record['min_interval_seconds']<=record['max_interval_seconds'] or record['spawn_radius']<=0:raise ValueError('Invalid storm schedule')
    bodies={b['id']:b for b in manifest['bodies']};nodes={n['id']:n for n in manifest['nodes']}
    identities={(s['source']['file'],s['source']['path_id']) for s in record['shelters']}
    expected={('level1',i) for i in (73428,79089,72174,71312,76881,68729,70386,75239)}
    if identities!=expected or len(identities)!=len(record['shelters']):raise ValueError('Invalid selected shelters')
    roots={manifest['area']['home_root'],*[v['root'] for v in manifest['vehicles']]}
    for shelter in record['shelters']:
        body=bodies.get(shelter['root'])
        if not body or body['parent_body'] is not None or body['node'] not in roots:raise ValueError('Invalid shelter owner')
    if len(record['particles'])!=4+2*len(manifest['openings']) or len({p['role'] for p in record['particles']})!=len(record['particles']):raise ValueError('Missing selected particles')
    textures={t['id'] for t in manifest['lighting']['textures']}
    for p in record['particles']:
        if not 0<p['capacity']<=1000000 or p['rate_over_time']<0 or p['rate_over_distance']<0:raise ValueError('Invalid particle capacity or rate')
        if p['capacity']!=p['scalars']['InitialModule.maxNumParticles']:raise ValueError('Particle authored capacity differs')
        if not p['materials'] or 'InitialModule.startLifetime' not in p['curves']:raise ValueError('Missing particle data')
        for material in p['materials']:
            if any(t['texture'] not in textures for t in material['textures']):raise ValueError('Missing particle texture')

def recover_ambient(source):
    main=source.get('level1',64919)
    cycle=source.resolve(main,source.read(main)['napszak']);cd=source.read(cycle)
    managed_evidence(source,['public static float jamDecrease = 0.1f;'])
    return {'source':source_id(cycle),
        'day_volume':[cd['dayAmbientSoundVolumeDay'][0],cd['dayAmbientSoundVolumeNight'][0]],
        'night_volume':[cd['nightAmbientSoundVolumeDay'][0],cd['nightAmbientSoundVolumeNight'][0]],
        'modifier':cd['ambientSoundModifier'],'only_modifier':cd['onlyAmbientSoundModifier'],'recovery_per_second':0.1}

def validate_ambient_export(record, library):
    from .library.reader import LibraryReader
    from .source import LibrarySource
    source=LibrarySource(LibraryReader(library))
    if record['ambient'] != recover_ambient(source):
        raise ValueError('Selected weather ambient differs from source')

def relative_pose(source, root, owner):
    from .gltf import transform_point
    a=world_pose(source,owner);b=world_pose(source,root)
    inverse=[-v for v in a['rotation'][:3]]+[a['rotation'][3]]
    delta=[b['translation'][i]-a['translation'][i] for i in range(3)]
    return {'translation':list(transform_point(delta,(0.,0.,0.),inverse,(1.,1.,1.))),
        'rotation':list(quaternion_product(inverse,b['rotation'])),
        'scale':[b['scale'][i]/a['scale'][i] for i in range(3)]}

def outlet_values(source,p,cap):
    p['scalars'].update({'outlet.maxSpeed':cap['maxSpeed'],'outlet.minRate':cap['minRate'],'outlet.maxRate':cap['maxRate'],
        'outlet.forceColorM':cap['forceColorM']})
    p['flags']['outlet.forceColor']=bool(cap['BforceColor'])
    p['vectors']['outlet.forceColor']=rgba(cap['forceColor'])[:3]
    p['scalars']['outlet.forceAlpha']=cap['forceColor']['a']
    main=source.read(source.get('level1',64919))
    for i,f in enumerate(main['fluids']):
        p['vectors'][f'fluid.{i}']=rgba(f['color'])[:3]
        p['scalars'][f'fluid.{i}.alpha']=f['color']['a']

def recover_outlet_particles(source,manifest,output):
    particles=[]
    for opening in manifest['openings']:
        cap=source.get(opening['source']['file'],opening['source']['path_id']);cd=source.read(cap)
        owner=source.transform_for(source.resolve(cap,cd['m_GameObject']))
        for prefix,field in [('outlet','ps'),('splash','sps')]:
            reader=source.resolve(cap,cd[field]);rate=cd['maxRate'] if field=='ps' else source.read(reader)['EmissionModule']['rateOverTime']['scalar']
            p=particle(source,reader,f'{prefix}_{opening["id"]}',rate,output,owner)
            outlet_values(source,p,cd);particles.append(p)
    return particles

def validate_particle_export(record, manifest, library):
    from .library.reader import LibraryReader
    from .source import LibrarySource
    from tempfile import TemporaryDirectory
    from pathlib import Path
    checked=LibrarySource(LibraryReader(library))
    from .export import CARS,SKODA_PARKING,relocate
    skoda=checked.transform_for(checked.get('level1',CARS['skoda'][0]))
    relocate(checked,skoda,SKODA_PARKING,180.)
    checked.lighting={'textures':manifest['lighting']['textures']}
    checked.lighting_textures={(t['source']['file'],t['source']['path_id']):t for t in manifest['lighting']['textures']}
    with TemporaryDirectory(prefix='source-particles-') as temp:
        recovered=recover_weather(checked,Path(temp))['particles']
        recovered.extend(recover_outlet_particles(checked,manifest,Path(temp)))
    if record['particles']!=recovered:
        raise ValueError('Selected particle source records differ')
