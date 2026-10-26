"""Recover the sky traffic spawner and its prefabs from checked records."""
import math
from .source import source_id,key,vector
from .survival import managed_evidence,finite_record
from .particles import particle,validate_particle
from .validation import record,array,number,u32,optional,string,source as source_shape
from .weather import game_object
from .world_biome_materials import scene_component

MAIN_OBJECT=('level1',425)
# AirPlaneSpawnScript.AirPlanePrefabs in order: asset name, sharedassets1 GameObject.
PREFABS=(('sky_airplane',6483),('sky_ufo',38330))
# Spawner rules that only exist in AirPlaneSpawnScript.Update code.
SKY_EVIDENCE=['PlaneCustomSpace.position = mainscript.M.player.transform.position + Vector3.up * height;',
    'Vector3 vector = UnityEngine.Random.insideUnitCircle * spawnDistance;',
    'gameObject.transform.Rotate(0f, UnityEngine.Random.Range(0f - maxAngle, maxAngle), 0f);',
    'if (Planes[i] == null || Vector3.Distance(Planes[i].position, PlaneCustomSpace.position) > destroyDistance)',
    'Planes[i].position += Planes[i].forward * speed * Time.deltaTime;']

def spawner(source):
    reader=scene_component(source,source.library,'AirPlaneSpawnScript')
    if key(game_object(source,reader))!=MAIN_OBJECT:raise ValueError('Sky spawner is not on Main')
    return reader

def prefabs(source,reader):
    """Prefab root Transforms in AirPlanePrefabs order."""
    links=[source.resolve(reader,p) for p in source.read(reader)['AirPlanePrefabs']]
    if any(r is None for r in links) or [key(game_object(source,r)) for r in links]!=[('sharedassets1.assets',pid) for _,pid in PREFABS]:
        raise ValueError('Sky prefab links differ')
    return [source.transform_for(game_object(source,r)) for r in links]

def prepare_sky(source, roots):
    for (name,_),root in zip(PREFABS,prefabs(source,spawner(source))):roots[name]=[root]

def spin(source,records,nodes):
    found=[c for n in records for c in n['components'] if c.type.name=='MonoBehaviour' and source.class_name(c)=='fedoscript']
    if len(found)>1:raise ValueError('Sky prefab has several fedoscript')
    if not found:return None
    d=source.read(found[0]);rot=source.resolve(found[0],d['rot'])
    if rot is None or key(rot) not in nodes:raise ValueError('Sky spin node missing')
    # A rotation axis converts like a quaternion vector part.
    x,y,z=vector(d['rotaxis'])
    return {'node':nodes[key(rot)]['id'],'axis':[-x,-y,z],'degrees_per_second':d['rotspeed']}

def recover_sky(source, manifest, output):
    reader=spawner(source);d=source.read(reader)
    nodes={(n['source']['file'],n['source']['path_id']):n for n in manifest['nodes']}
    result={'source':source_id(reader),'chances':list(d['chances']),'max_planes':d['maxPlaneNum'],
        'spawn_time_range':[d['minSpawnTime'],d['maxSpawnTime']],'spawn_distance':d['spawnDistance'],'height':d['height'],
        'speed':d['speed'],'max_angle':d['maxAngle'],'destroy_distance':d['destroyDistance'],'contrail_rate':d['rate'],
        'prefabs':[],'evidence':managed_evidence(source,SKY_EVIDENCE)}
    for (name,_),root in zip(PREFABS,prefabs(source,reader)):
        if key(root) not in nodes:raise ValueError(f'Sky prefab not exported: {name}')
        records=source.hierarchy(root,activate_root=True)
        # GetComponentInChildren finds the first active ParticleSystem.
        system=next((c for n in records if n['active'] for c in n['components'] if c.type.name=='ParticleSystem'),None)
        result['prefabs'].append({'source':source_id(root),'asset':nodes[key(root)]['asset'],'root':nodes[key(root)]['id'],
            'contrail':particle(source,system,f'{name}_contrail',d['rate'],output,root) if system else None,
            # recover_audio replaces these AudioSource identities with voice ids.
            'voices':[source_id(c) for n in records for c in n['components'] if c.type.name=='AudioSource'],
            'spin':spin(source,records,nodes)})
    source.lighting['textures']=list(source.lighting_textures.values())
    return result

def validate_sky(sky, manifest):
    textures={t['id'] for t in manifest['lighting']['textures']}
    nodes={n['id']:n for n in manifest['nodes']};assets={a['id']:a for a in manifest['assets']}
    voices={v['id']:v for v in manifest['audio']['voices']};emitters={e['id']:e for e in manifest['audio']['emitters']}
    def prefab(p):
        record(p,{'source':source_shape,'asset':u32,'root':u32,'contrail':optional(lambda c:validate_particle(c,textures)),'voices':array(u32),
            'spin':optional(lambda s:record(s,{'node':u32,'axis':array(number,3),'degrees_per_second':number}))})
    record(sky,{'source':source_shape,'chances':array(number),'max_planes':u32,'spawn_time_range':array(number,2),
        'spawn_distance':number,'height':number,'speed':number,'max_angle':number,'destroy_distance':number,'contrail_rate':number,
        'prefabs':array(prefab,len(PREFABS)),'evidence':lambda e:record(e,{'assembly_sha256':string,'methods':array(string)})})
    finite_record(sky)
    if len(sky['chances'])!=len(sky['prefabs']) or any(c<0 for c in sky['chances']) or sum(sky['chances'])<=0:raise ValueError('Invalid sky chances')
    if sky['max_planes']<=0 or not 0<=sky['spawn_time_range'][0]<=sky['spawn_time_range'][1]:raise ValueError('Invalid sky schedule')
    if (min(sky['spawn_distance'],sky['speed'],sky['destroy_distance'])<=0 or sky['contrail_rate']<0
            or not 0<=sky['max_angle']<=180):raise ValueError('Invalid sky spawner')
    for p in sky['prefabs']:
        asset=assets.get(p['asset'],{});root=nodes.get(p['root'],{})
        if asset.get('usage')!='sky' or asset.get('root')!=p['source'] or root.get('source')!=p['source'] or root.get('parent') is not None:
            raise ValueError('Invalid sky prefab root')
        if any(v not in voices or nodes.get(emitters[voices[v]['emitter']]['node'],{}).get('asset')!=p['asset'] for v in p['voices']):
            raise ValueError('Invalid sky voice')
        s=p['spin']
        if s and (nodes.get(s['node'],{}).get('asset')!=p['asset'] or math.hypot(*s['axis'])==0):raise ValueError('Invalid sky spin')
