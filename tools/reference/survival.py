"""Recover the selected player's needs, food prototypes, and sleep contracts."""
import hashlib
import json
import math
import re
import struct
from pathlib import Path
from .source import LibrarySource, key, source_id
from .collision import world_pose, geometry
from .gltf import GltfExporter
from .actions import recover_pickup

SURVIVAL_FIELDS = 'waterToPissRatio foodToShitRatio bloodToHpRatio alcoholHeal alcoholConvertToPiss carFluidsDamage maxHealingSpeed healingToiletFullModfiier waterDamage foodDamage defTemp coldTemp hotTemp foodLoss coldFoodLossModifier hotFoodLossModifier waterLoss coldWaterLossModifier hotWaterLossModifier runFoodLossModifier runWaterLossModifier delaySpeed'.split()

def snake(name):
    return re.sub(r'(?<!^)(?=[A-Z])', '_', name).lower()

def f32(value):
    return struct.unpack('<f', struct.pack('<f', value))[0]

def managed_evidence(source, assertions):
    root=source.library.records_root/'managed'
    receipt=json.loads((root/'decompilation.json').read_bytes())
    if not receipt['complete'] or receipt['diagnostics'] or receipt['input']['sha256']!='9919a9eeddcce23bb4ff86e890437f32b4a3ef3c10af2efe3b8d889d894dd62e':
        raise ValueError('Required managed source receipt differs')
    content=(root/'Assembly-CSharp.cs').read_bytes()
    if len(content)!=receipt['output']['bytes'] or hashlib.sha256(content).hexdigest()!=receipt['output']['sha256']:
        raise ValueError('Managed output hash mismatch')
    code=content.decode('utf-8-sig')
    for text in assertions:
        if text not in code: raise ValueError(f'Required managed source assertion absent: {text}')
    return {'assembly_sha256':receipt['input']['sha256'], 'methods':assertions}

def component(source, root, name):
    matches=[c for c in source.components(root)[1] if (source.class_name(c) if c.type.name=='MonoBehaviour' else c.type.name)==name]
    if len(matches)!=1:raise ValueError(f'{key(root)} requires one {name}')
    return matches[0]

def selected_foods(source):
    database=source.get('level1',66024); db=source.read(database)
    player=source.get('level1',80125)
    return [('sausage',source.transform_for(source.resolve(database,db['gkolbasz']))),
            ('turd',source.transform_for(source.resolve(player,source.read(player)['GTurd'])))]

def part_groups(source, edible):
    data=source.read(edible)
    groups=[v['parts'] for v in data['moreFoodParts']] if data['moreParts'] else [[v] for v in data['parts']]
    return [[source.transform_for(source.resolve(edible,p)) for p in group] for group in groups]

def prepare_food(source, roots):
    roots['food']=[root for _,root in selected_foods(source)]
    source.food_geometry_nodes=set()
    source.food_active={}
    for root in [roots['items'][-1],*roots['food']]:
        edible=component(source,root,'ediblescript');data=source.read(edible)
        for index,group in enumerate(part_groups(source,edible)):
            for part in group:
                for node in source.hierarchy(part,activate_root=True):
                    source.food_geometry_nodes.add(key(node['reader']))
                    source.food_active[key(node['reader'])]=index==data['index']

def food_record(source, kind, root, manifest):
    nodes={(n['source']['file'],n['source']['path_id']):n for n in manifest['nodes']}
    edible=component(source,root,'ediblescript');data=source.read(edible)
    if data['continous'] or not data['destroyOnEnd']:raise ValueError('Selected food consumption contract differs')
    groups=part_groups(source,edible)
    rigid=source.read(component(source,root,'Rigidbody'))
    pose=world_pose(source,root);pose['scale']=[1.,1.,1.]
    exporter=GltfExporter(source,None,source.library)
    physics=next(r for r in source.objects.values() if r.type.name=='PhysicsManager')
    matrix=source.read(physics)['m_LayerCollisionMatrix']
    colliders=[]
    for n in source.hierarchy(root,activate_root=True):
        layer=source.read(n['game_object'])['m_Layer']
        for c in n['components']:
            if c.type.name not in ('BoxCollider','SphereCollider','CapsuleCollider','MeshCollider'):continue
            d=source.read(c)
            colliders.append({'source':source_id(c),'node':nodes[key(n['reader'])]['id'],
                'membership':1<<layer,'filter':matrix[layer],'sensor':bool(d['m_IsTrigger']),
                'enabled':bool(d['m_Enabled']), 'geometry':geometry(source,exporter,c,n['reader'],pose,True)})
    return {'source':source_id(edible),'kind':kind,'effect':dict(food=data['foodValue'],water=data['waterValue'],bowel=data['shitValue'],hp=data['hpValue']),
        'bites':len(groups),'index':data['index'],'destroy_on_end':bool(data['destroyOnEnd']),
        'asset':nodes[key(root)]['asset'],'root':nodes[key(root)]['id'],
        'mass_kg':source.read(component(source,root,'massScript'))['mass'],
        'linear_damping':rigid['m_Drag'],'angular_damping':rigid['m_AngularDrag'],
        'pickup':recover_pickup(source,component(source,root,'pickupable')),
        'parts':[[nodes[key(part)]['id'] for part in group] for group in groups],
        'colliders':colliders,'particle_type':data['particleType'],'min_particle':data['minParticle'],'max_particle':data['maxParticle'],
        'sound':data['sound'],'text':data['text']}

def recover_survival(source: LibrarySource, roots: dict, output: Path) -> dict:
    player=source.get('level1',80125);pd=source.read(player)
    owner=source.resolve(player,pd['survival']);d=source.read(owner)
    if key(owner)!=('level1',73239):raise ValueError('Active player survival link differs')
    main=source.get('level1',64919);md=source.read(main)
    outlet=source.resolve(player,pd['piss']);od=source.read(outlet)
    bladder=source.resolve(outlet,od['Tank']);bd=source.read(bladder)['F']
    maximum={name:d[field] for name,field in [('hp','maxHp'),('food','maxFood'),('water','maxWater'),('bowel','maxShit')]}
    initial={name:f32(value*f32(factor)) for (name,value),factor in zip(maximum.items(),[1.,.75,.6,.5])}
    quantities=[0.]*6
    for fluid in bd['fluids']:quantities[fluid['type']]=f32(quantities[fluid['type']]+fluid['amount'])
    evidence=managed_evidence(source,['water = maxWater * 0.6f;', 'food = maxFood * 0.75f;', 'shit = maxShit * 0.5f;',
        'currentWaterLoss = foodLoss * sleep *', 'piss.valve = Mathf.Lerp(piss.valve, (pissing && !emptiedBladder) ? 1f : 0f, dt * 3f);',
        'survival.shit -= survival.maxShit * 0.3f;', 'sleepEndTime = Time.time + sleepRealTime * 2f;',
        'float num = Mathf.InverseLerp(0f, 14f, Time.time - deathTime);', 'private float foodStatShowTime = 5f;', 'private float foodStatHideTime = 1f;', 'defaultInput[21].keys[0].Set(KeyCode.P);','defaultInput[22].keys[0].Set(KeyCode.B);'])
    result={'source':source_id(owner),'player':source_id(player),'blood':source_id(source.resolve(owner,d['blood'])),
        'initial':initial,'maximum':maximum,'initial_bladder':quantities,'bladder_capacity_liters':bd['maxC'],
        'initial_queue':dict(food=d['toEat'],water=d['toDrink'],bowel=d['toShit'],hp=d['toHp']),
        **{snake(k):d[k] for k in SURVIVAL_FIELDS},'evidence':evidence,
        'drink_modifier':pd['drinkModifier'],'death_velocity_change':pd['deathVelocityChange'],
        'food_stat_show_time':5.,'food_stat_hide_time':1.,
        'toilet':{'source':source_id(outlet),'bladder':source_id(bladder),'radius':od['radius'],'no_angle':bool(od['noAngle']),
            'valve_gain':3.,'empty_amount':.01,'bowel_fraction':.3,'fart_burp_ratio':pd['fartBurpRatio'],'fart_shit_ratio':pd['fartShitRatio'],
            **{snake(k):pd[k] for k in ['minFartPitch','maxFartPitch','minFartVolume','maxFartVolume','minBurpPitch','maxBurpPitch','minBurpVolume','maxBurpVolume']},
            'max_rate':od['maxRate'],'min_rate':od['minRate'],'max_speed':od['maxSpeed'],'force_color':list(od['forceColor'][c] for c in 'rgba'),'force_color_m':od['forceColorM']},
        'inputs':recover_inputs(source),
        'mode_source':source_id(source.get('level0',17179)),'mode':source.read(source.get('level0',17179))['defS']['survival'],
        'death_seconds':14.,'drinking':recover_drinking(source,player)}
    cycle=source.read(source.resolve(main,md['napszak']))
    sleep={'source':source_id(main),'need_modifier':d['sleepModifier'],'hold_gain':md['sleepGain'],'sleep_real_seconds':md['sleepRealTime'],
        'sleep_cycle_seconds':md['sleepTimeInGame'],'cooldown_seconds':md['minTimeBeforeSleep'],
        'sleep_cam_ring_min':md['sleepCamRingMin'],'sleep_cam_ring_max':md['sleepCamRingMax'],
        'sleep_cam_height_min':md['sleepCamHeightMin'],'sleep_cam_height_max':md['sleepCamHeightMax'],'sleep_layer_mask':md['sleepLayerMask']['m_Bits'],
        **{snake(k):cycle[k] for k in ['CloudLowMinSpeed','CloudLowMidSpeed','CloudLowMaxSpeed','CloudHighMinSpeed','CloudHighMidSpeed','CloudHighMaxSpeed']},
        'evidence':managed_evidence(source,['sleep *= sleepModifier;', 'sleepModifierDef = sleepTimeInGame / sleepRealTime * 0.5f;', 'Vector3 vector2 = new Vector3(vector.x, 10000f, vector.y);','player.MoveCamParent(position + Vector3.up * 50f);'])}
    from .audio import recover_audio
    source.audio=recover_audio(source,output)
    source.survival=result; source.sleep=sleep;source.survival_roots=roots
    return result

def recover_inputs(source):
    managed_evidence(source,[])
    code=(source.library.records_root/'managed/Assembly-CSharp.cs').read_text(encoding='utf-8-sig')
    enum=code[code.index('public enum IN\n'):];enum=enum[enum.index('{')+1:enum.index('}')]
    actions=[s.strip() for s in enum.split(',') if s.strip()]
    result=[]
    for name in ('lmb','piss','burpfart','sleep'):
        index=actions.index(name)
        match=re.search(r'defaultInput\['+str(index)+r'\]\.keys\[0\]\.Set\(KeyCode\.(\w+)\);',code)
        if not match:raise ValueError(f'Missing input default: {name}')
        result.append({'action':name,'index':index,'key':match.group(1)})
    return result

def bind_survival(source,manifest):
    r=source.survival
    r['foods']=[food_record(source,kind,root,manifest) for kind,root in selected_foods(source)]
    starter=source.transform_for(source.get('level1',3542))
    node=next(n for n in manifest['nodes'] if n['source']==source_id(starter))
    r['starter_food']=next(i['id'] for i in manifest['items'] if i['node']==node['id'])
    node_ids={(n['source']['file'],n['source']['path_id']):n['id'] for n in manifest['nodes']}
    r['starter_parts']=[[node_ids[key(p)] for p in group] for group in part_groups(source,component(source,starter,'ediblescript'))]
    terrain=source.get('level1',64898)
    parent=source.resolve(starter,source.read(starter)['m_Father'])
    starts=[source.resolve(terrain,p) for p in source.read(terrain)['StartObjs']]
    if key(parent) not in {key(p) for p in starts}:raise ValueError('Starter food not under StartObjs')
    r['starter_parent']=source_id(parent);r['starter_pose']=world_pose(source,starter)
    r['evidence']['methods'].extend(managed_evidence(source,['StartObjs[i].position += Vector3.up * sp;'])['methods'])
    manifest.update(survival=r,sleep=source.sleep,audio=source.audio)
    validate_survival_export(r,manifest)

def finite_record(value):
    if isinstance(value,float) and not math.isfinite(value):raise ValueError('Non-finite source value')
    if isinstance(value,dict):
        for child in value.values():finite_record(child)
    elif isinstance(value,list):
        for child in value:finite_record(child)

def validate_survival_export(record: dict, manifest: dict) -> None:
    finite_record(record);finite_record(manifest['sleep'])
    nodes={n['id']:n for n in manifest['nodes']}
    item=next((i for i in manifest['items'] if i['id']==record['starter_food'] and i['kind']=='food'),None)
    starter=nodes.get(item['node']) if item else None
    if not starter or starter['source']!=item['source'] or starter['source']!={'file':'level1','path_id':24573}:raise ValueError('Invalid starter food source')
    if len(record['starter_parts'])!=6 or any(not group or any(nodes.get(n,{}).get('asset')!=starter['asset'] for n in group) for group in record['starter_parts']):raise ValueError('Invalid starter parts')
    if {f['kind'] for f in record['foods']}!={'sausage','turd'} or len(record['foods'])!=2:raise ValueError('Duplicate or missing prototype kind')
    assets={a['id']:a for a in manifest['assets']}
    for food in record['foods']:
        if not 0<=food['index']<food['bites'] or food['bites']!=len(food['parts']):raise ValueError('Exhausted food index')
        if assets.get(food['asset'],{}).get('usage')!='food_prototype' or nodes.get(food['root'],{}).get('asset')!=food['asset']:raise ValueError('Missing food prototype')
        if food['mass_kg']<=0 or not food['colliders']:raise ValueError('Invalid food geometry')
        for group in food['parts']:
            if not group or any(n not in nodes or nodes[n]['asset']!=food['asset'] for n in group):raise ValueError('Missing food part')
    for name in ('hp','food','water','bowel'):
        if not 0<=record['initial'][name]<=record['maximum'][name] or record['maximum'][name]<=0:raise ValueError('Invalid needs')


def recover_drinking(source,player):
    pd=source.read(player);cap=source.resolve(player,pd['drink']);d=source.read(cap)
    root=source.transform_for(source.resolve(cap,d['m_GameObject']))
    collider=source.resolve(player,pd['drinkCol'])
    evidence=managed_evidence(source,['float num = mainscript.M.pourSpeed * t2.valve * Time.deltaTime * t2.flowA;',
        'num *= drinkModifier;', '(item.intersection - item2.transform.position).magnitude < item2.radius * 0.1f',
        't.player.survival.Drink(T.poured * T.Tank.F.GetPercent(mainscript.fluidenum.water));'])
    return {'source':source_id(cap),'tank':source_id(source.resolve(cap,d['Tank'])),
        'collider':source_id(collider),'geometry':geometry(source,GltfExporter(source,None,source.library),collider,root,world_pose(source,root),False),
        'local_position':list(source.read(root)['m_LocalPosition'][axis] for axis in 'xyz'),
        'radius':d['radius'],'valve':d['valve'],'no_angle':bool(d['noAngle']),'no_pour':bool(d['noPour']),
        'is_drink_cap':bool(d['isDrinkCap']),'particle_contact_radius_multiplier':.1,'evidence':evidence}


def validate_clock_export(manifest: dict, library) -> None:
    """Check clock fields against the accepted installed records."""
    from .library.reader import LibraryReader
    source = LibrarySource(LibraryReader(library))
    main = source.read(source.get('level1', 64919))
    survival = source.read(source.get('level1', 73239))
    if manifest['engine']['globals']['outside_temperature_modifier'] != main['outsideTempM']:
        raise ValueError('Outside temperature modifier differs from mainscript.outsideTempM')
    if manifest['sleep']['need_modifier'] != survival['sleepModifier']:
        raise ValueError('Sleep need modifier differs from survivalscript.sleepModifier')
    managed_evidence(source, [
        'sleepModifierDef = sleepTimeInGame / sleepRealTime * 0.5f;',
        'sleep *= sleepModifier;',
        'sleepEndTime = Time.time + sleepRealTime * 2f;',
        'starSpeed * Time.fixedDeltaTime + tekerve',
        'currentTempM = Mathf.Lerp(0f, outsideTempM, Mathf.InverseLerp(napszak.tempNights[0], napszak.tempDays[0] + napszak.tempAddNoons[0], napszak.temp));',
    ])
