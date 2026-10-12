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

def particle(source,reader,role,rate,output):
    d=source.read(reader);root=source.transform_for(source.resolve(reader,d['m_GameObject']))
    renderer=component(source,root,'ParticleSystemRenderer');rd=source.read(renderer)
    result={'source':source_id(reader),'role':role,'capacity':d['InitialModule']['maxNumParticles'],
        'rate_over_time':rate,'rate_over_distance':d['EmissionModule']['rateOverDistance']['scalar'],
        'duration':d['lengthInSec'],'simulation_space':d['moveWithTransform'],'local_pose':world_pose(source,root),
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
    for k in ['m_RenderMode','m_SortMode','m_MinParticleSize','m_MaxParticleSize','m_CameraVelocityScale','m_VelocityScale','m_LengthScale','m_SortingFudge','m_RenderAlignment','m_Pivot','m_Flip']:
        visit(rd[k],'renderer.'+k)
    if result['module_flags']['CollisionModule']:
        result['limits'].append({'source':source_id(reader),'module':'CollisionModule','visible_effect':'Unity collision bounce and splash appearance need a particle renderer adapter; gameplay drinking contacts remain required.'})
    return result

def recover_weather(source, output):
    main=source.get('level1',64919);scheduler=source.resolve(main,source.read(main)['sandStormSpawn']);sd=source.read(scheduler)
    database=source.get('level1',66024)
    root=source.transform_for(source.resolve(database,source.read(database)['storms'][sd['prefabid']]))
    storm=component(source,root,'sandstormscript');d=source.read(storm)
    if key(storm)!=('sharedassets1.assets',158276):raise ValueError('Storm database link differs')
    record={'source':source_id(scheduler),'min_interval_seconds':sd['minTime'],'max_interval_seconds':sd['maxTime'],'spawn_radius':sd['radius'],
        'storm':{'source':source_id(storm),'biome':d['biome'],**{snake(k):d[k] for k in STORM_FIELDS},
            **{snake(k):rgba(d[k]) for k in ['nightColor','skyColor','skyGroundColor','fogColor','CloudColor']},
            'day_color1':gradient(d['dayColor1']),'day_color2':gradient(d['dayColor2']),'wind_lm':d['windLm']['m_Bits'],
            'size_range':[.5,1.5],'speed_range':[3.,12.],'wind_time_range':[.1,1.5],'wind_scan_seconds':1.,'wind_bounds_range':[1.,30.]},
        'evidence':managed_evidence(source,['size = UnityEngine.Random.Range(0.5f, 1.5f);','speed = UnityEngine.Random.Range(3f, 12f);',
            'yield return new WaitForSeconds(1f);','nextWindTime = Time.time + UnityEngine.Random.Range(0.1f, 1.5f);',
            'mainscript.M.interiorCols[j].transform.root != windRbs[i].transform.root']), 'particles':[]}
    for role,field,rate in [('storm_big','bigParticleSystem',d['defBRateT']),('storm_small','smallParticleSystem',d['defSRateT'])]:
        record['particles'].append(particle(source,source.resolve(storm,d[field]),role,rate,output))
    player=source.get('level1',80125);outlet=source.resolve(player,source.read(player)['piss']);od=source.read(outlet)
    for role,field in [('urine','ps'),('urine_splash','sps')]:
        reader=source.resolve(outlet,od[field]);rate=od['maxRate'] if field=='ps' else source.read(reader)['EmissionModule']['rateOverTime']['scalar']
        record['particles'].append(particle(source,reader,role,rate,output))
    source.lighting['textures']=list(source.lighting_textures.values())
    source.weather=record
    return record

def bind_weather(source,manifest):
    record=source.weather;record['shelters']=[]
    nodes={(n['source']['file'],n['source']['path_id']):n for n in manifest['nodes']}
    exporter=GltfExporter(source,None,source.library)
    for name in ('home','lada'):
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
    manifest['weather']=record
    validate_weather_export(record,manifest)

def validate_weather_export(record,manifest):
    finite_record(record)
    if not 0<record['min_interval_seconds']<=record['max_interval_seconds'] or record['spawn_radius']<=0:raise ValueError('Invalid storm schedule')
    bodies={b['id']:b for b in manifest['bodies']};nodes={n['id']:n for n in manifest['nodes']}
    owners={73428:21400,79089:21400,72174:21400,71312:21400,76881:37191,68729:37191}
    if len(record['shelters'])!=6 or len({(s['source']['file'],s['source']['path_id']) for s in record['shelters']})!=6:raise ValueError('Invalid selected shelters')
    for shelter in record['shelters']:
        identity=shelter['source'];owner=owners.get(identity['path_id']) if identity['file']=='level1' else None
        body=bodies.get(shelter['root']);node=nodes.get(body['node']) if body else None
        if owner is None or not node or body['parent_body'] is not None or node['source']!={'file':'level1','path_id':owner}:raise ValueError('Invalid shelter owner')
    if len(record['particles'])!=4 or len({p['role'] for p in record['particles']})!=4:raise ValueError('Missing selected particles')
    textures={t['id'] for t in manifest['lighting']['textures']}
    for p in record['particles']:
        if not 0<p['capacity']<=1000000 or p['rate_over_time']<0 or p['rate_over_distance']<0:raise ValueError('Invalid particle capacity or rate')
        if p['capacity']!=p['scalars']['InitialModule.maxNumParticles']:raise ValueError('Particle authored capacity differs')
        if not p['materials'] or 'InitialModule.startLifetime' not in p['curves']:raise ValueError('Missing particle data')
        for material in p['materials']:
            if any(t['texture'] not in textures for t in material['textures']):raise ValueError('Missing particle texture')
