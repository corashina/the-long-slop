"""Recover the selected storm record from checked records."""
from .source import source_id,key,vector
from .survival import component,managed_evidence,snake,finite_record
from .lighting import gradient,rgba,validate_gradient
from .particles import particle,validate_particle
from .validation import record,array,number,integer,string,source as source_shape,u32

MAIN=('level1',64919)
DATABASE=('level1',66024)
STORM=('sharedassets1.assets',158276)
STORM_FIELDS='lifeLossOnOtherBiome lifeGain destroyDist life radius fogDensity atmosphereThickness intensityMultipler ambientMultipler volume ambientDecrease yoffset windForce defSRateT defSRateD defBRateT defBRateD'.split()
STORM_COLORS=['nightColor','skyColor','skyGroundColor','fogColor','CloudColor']
# Values that only exist in sandstormscript code. The evidence lines prove each one.
STORM_EVIDENCE=['size = UnityEngine.Random.Range(0.5f, 1.5f);','speed = UnityEngine.Random.Range(3f, 12f);',
    'yield return new WaitForSeconds(1f);','nextWindTime = Time.time + UnityEngine.Random.Range(0.1f, 1.5f);',
    'mainscript.M.interiorCols[j].transform.root != windRbs[i].transform.root']
ROLES=('storm_big','storm_small')

def owner(source,reader,name):
    if source.class_name(reader)!=name:raise ValueError(f'{key(reader)} is not {name}')
    return reader

def recover_weather(source, output):
    main=owner(source,source.get(*MAIN),'mainscript')
    scheduler=source.resolve(main,source.read(main)['sandStormSpawn'])
    if scheduler is None:raise ValueError('Storm scheduler link missing')
    sd=source.read(owner(source,scheduler,'sandStormSpawnScript'))
    database=owner(source,source.get(*DATABASE),'itemdatabase');storms=source.read(database)['storms']
    if not 0<=sd['prefabid']<len(storms):raise ValueError('Storm prefab id differs')
    prefab=source.resolve(database,storms[sd['prefabid']])
    if prefab is None:raise ValueError('Storm database link missing')
    root=source.transform_for(prefab);storm=component(source,root,'sandstormscript');d=source.read(storm)
    if key(storm)!=STORM:raise ValueError('Storm database link differs')
    result={'source':source_id(scheduler),'min_interval_seconds':sd['minTime'],'max_interval_seconds':sd['maxTime'],'spawn_radius':sd['radius'],
        'storm':{'source':source_id(storm),'biome':d['biome'],**{snake(k):d[k] for k in STORM_FIELDS},
            **{snake(k):rgba(d[k]) for k in STORM_COLORS},
            'day_color1':gradient(d['dayColor1']),'day_color2':gradient(d['dayColor2']),'wind_lm':d['windLm']['m_Bits'],
            'current_force':list(vector(d['currentForce'])),'next_force':list(vector(d['nextForce'])),'distance_lerp':d['distanceLerp'],
            'size_range':[.5,1.5],'speed_range':[3.,12.],'wind_time_range':[.1,1.5],'wind_scan_seconds':1.,'wind_bounds_range':[1.,30.]},
        'evidence':managed_evidence(source,STORM_EVIDENCE),
        'particles':[particle(source,source.resolve(storm,d[field]),role,rate,output,root)
            for role,field,rate in [('storm_big','bigParticleSystem',d['defBRateT']),('storm_small','smallParticleSystem',d['defSRateT'])]]}
    source.lighting['textures']=list(source.lighting_textures.values())
    return result

def validate_weather_export(weather, manifest):
    pair=array(number,2);textures={t['id'] for t in manifest['lighting']['textures']}
    record(weather,{'source':source_shape,'min_interval_seconds':number,'max_interval_seconds':number,'spawn_radius':number,
        'storm':lambda s:record(s,{'source':source_shape,'biome':integer(0,255),**dict.fromkeys(map(snake,STORM_FIELDS),number),
            **dict.fromkeys(map(snake,STORM_COLORS),array(number,4)),'day_color1':validate_gradient,'day_color2':validate_gradient,
            'wind_lm':u32,'current_force':array(number,3),'next_force':array(number,3),'distance_lerp':number,
            'size_range':pair,'speed_range':pair,'wind_time_range':pair,'wind_scan_seconds':number,'wind_bounds_range':pair}),
        'evidence':lambda e:record(e,{'assembly_sha256':string,'methods':array(string)}),
        'particles':array(lambda p:validate_particle(p,textures),len(ROLES))})
    finite_record(weather)
    if not 0<weather['min_interval_seconds']<=weather['max_interval_seconds'] or weather['spawn_radius']<=0:raise ValueError('Invalid storm schedule')
    if tuple(p['role'] for p in weather['particles'])!=ROLES:raise ValueError('Missing storm particles')
