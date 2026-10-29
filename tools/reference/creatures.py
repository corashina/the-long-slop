"""Rabbit prefab (aispawn.AIPrefab) and the stimulscript sources that rabbits sense.

Directions and positions use the converted space: Unity z is negated. Angles are in degrees.
"""
import math
from .source import key, source_id, vector
from .gltf import convert_point
from .collision import geometry, step_surface, collider_tag, world_pose
from .survival import component, managed_evidence, finite_record
from .acquisition_motion import motion_clips
from .breakables import breakable
from .library.types import ObjectId
from .validation import record, array, number, u32, boolean, source as source_shape

RABBIT = ('sharedassets1.assets', 33572)
# aispawn on the Player GameObject. The copy on the inactive map player (level1 74614) never runs.
SPAWNER = ('level1', 74613)
STIMULUS_CLASS = 'stimulscript'
STIMULUS_KINDS = ('nothing', 'food', 'water', 'threat', 'light', 'command')
EVIDENCE = [
    'if ((move.RB.velocity - lastVelocity).sqrMagnitude > accelerationDeath)',
    'interestPos = transform.position + ((transform.position - tempV1 + (transform.position - tempV2)) * 0.5f).normalized * fleeDist;',
    'moveSpeed = Mathf.InverseLerp(targetAngleMove, targetAngleGoal, angleFromTarget) * Mathf.Lerp(walkSpeedModifier, 1f, Mathf.InverseLerp(walkDist, runDist, distFromTarget));',
    'RB.velocity = Vector3.Lerp(new Vector3(RB.velocity.x, 0f, RB.velocity.y), Vector3.ClampMagnitude(horizontalDir, speed) * (modifier ? modifierValue : 1f), acceleration * Time.deltaTime);',
    'stimuls.Add(new stimul(collider2.transform.position, stimul2.tipus, stimul2.value * hearModifier * Mathf.InverseLerp(0f, hearDistance, dist)));',
    'if (Vector3.Angle(transform.forward, ppos - transform.forward) > halfFov && !Physics.CheckSphere(ppos, radius, maskSphere, QueryTriggerInteraction.Ignore))',
    'move.rotSpeed = Mathf.Lerp(maxRotSpeed, minRotSpeed, size);',
    'RB.mass = num;',
    'anim.SetFloat("RunValue", Mathf.Clamp(runValue * 2f, 0f, 1f));',
    'component.GetComponent<stimulscript>().SetAudioValue(volume);',
]


def prefab_root(source):
    spawner = source.get(*SPAWNER)
    prefab = source.resolve(spawner, source.read(spawner)['AIPrefab'])
    if prefab is None or key(prefab) != RABBIT:
        raise ValueError('Rabbit prefab link differs')
    return source.transform_for(prefab)


def prepare_creatures(source, roots):
    """Export the rabbit prefab as its own asset. Instantiate replaces the root position and rotation."""
    root = prefab_root(source)
    roots['creature_rabbit'] = [root]
    source.creature_root = key(root)


def ray_direction(euler):
    """sensoryscript.Start: Quaternion.Euler(rayDir) * Vector3.forward, converted."""
    x, y = math.radians(euler['x']), math.radians(euler['y'])
    return list(convert_point((math.cos(x) * math.sin(y), -math.sin(x), math.cos(x) * math.cos(y))))


def mask(value):
    return value['m_Bits']


def spawner_record(source):
    reader = source.get(*SPAWNER)
    d = source.read(reader)
    return {'source': source_id(reader), 'chance': d['chance'], 'time_range': [d['minTime'], d['maxTime']],
        'count_range': [d['minCount'], d['maxCount']], 'max_count': d['maxAINum'], 'distance_range': [d['minDist'], d['maxDist']],
        'offset': list(convert_point(vector(d['offset']))), 'radius': d['radius'], 'half_fov': d['halfFov'],
        'surface_mask': mask(d['maskSurface']), 'sphere_mask': mask(d['maskSphere'])}


def brain_record(source, ai):
    d = source.read(ai)
    preferences = [{'kind': STIMULUS_KINDS[s['tipus']], 'value': s['value']} for s in d['stimulPreferences']]
    return {'source': source_id(ai), 'acceleration_death': d['accelerationDeath'],
        'go_to_target': bool(d['goToTarget']), 'flee': bool(d['flee']), 'pre_calc_flee': bool(d['preCalcFlee']),
        'avoid_obstacles': bool(d['avoidObstacles']), 'roam': bool(d['roam']), 'attack': bool(d['attack']),
        'jump': bool(d['jump']), 'jump_if_speed': d['jumpIfSpeed'], 'jump_sense_distance': d['jumpSensDist'],
        'jump_mask': mask(d['jumpLayerMask']), 'enable_invert': bool(d['enableInvert']),
        'go_to_player_target': bool(d['goToPlayerTarget']), 'target_pre_calc_distance': d['targetPreCalcDist'],
        'target_distance_goal': d['targetDistGoal'], 'target_angle_goal': d['targetAngleGoal'],
        'target_angle_move': d['targetAngleMove'], 'walk_distance': d['walkDist'], 'run_distance': d['runDist'],
        'attack_speed': d['attackSpeed'], 'flee_distance': d['fleeDist'], 'min_flee_speed': d['minFleeSpeed'],
        'flee_time': d['fleeTime'], 'walk_speed_modifier': d['walkSpeedModifier'],
        'move_towards_target_modifier': d['moveTowardsTargetModifier'], 'move_forward_modifier': d['moveForwardModifier'],
        'roam_x': [d['roamMinDistX'], d['roamMaxDistX']], 'roam_z': [d['roamMinDistZ'], d['roamMaxDistZ']],
        'roam_time': [d['roamMinTime'], d['roamMaxTime']], 'invert_flight_if_above': d['invertFlightIfAbove'],
        'preferences': preferences}


def motion_record(source, move):
    d = source.read(move)
    if d['modifier']:
        raise ValueError('movecreaturescript starts with a speed modifier')
    return {'source': source_id(move), 'acceleration': d['acceleration'], 'speed': d['speed'], 'rot_speed': d['rotSpeed'],
        'jump_force': d['jumpForce'], 'grounded_ray_distance': d['groundedRayDist'], 'grounded_mask': mask(d['groundedLM'])}


def senses_record(source, senses, node):
    d = source.read(senses)
    if not d['alive'] or not d['hasEyes'] or not d['hasEars']:
        raise ValueError('sensoryscript start state differs')
    rays = [{'enabled': bool(r['enabled']), 'inverted': bool(r['inverted']), 'symmetry': bool(r['symmetry']),
             'boolean': bool(r['BOOL']), 'direction': ray_direction(r['rayDir']), 'length': r['rayLength'],
             'mask': mask(r['rayMask']), 'max_value': r['maxValue'], 'spread': r['spread']} for r in d['visionRays']]
    return {'source': source_id(senses), 'scan_interval': d['scanInterval'],
        'eyes': node(source.resolve(senses, d['eyes'])),
        'eye_objects': [node(source.transform_for(source.resolve(senses, p))) for p in d['eyeObjects']],
        'fov': [d['fovX'], d['fovY'], d['fovZ']], 'hear_distance': d['hearDistance'], 'min_hear_distance': d['minHearDistance'],
        'min_hear_value': d['minHearValue'], 'hear_modifier': d['hearModifier'], 'stimulus_mask': mask(d['stimulMask']),
        'vision_mask': mask(d['visionMask']), 'rays': rays}


def size_record(source, nyul, node):
    d = source.read(nyul)
    if d['started'] or d['size'] < 0:
        raise ValueError('nyulscript start state differs')
    return {'source': source_id(nyul), 'scale': [d['minScale'], d['maxScale']], 'mass': [d['minMass'], d['maxMass']],
        'health': [d['minHp'], d['maxHp']], 'shoot_health': [d['minShootHp'], d['maxShootHp']],
        'speed': [d['minSpeed'], d['maxSpeed']],
        # SetBasedOnSize lerps from maxRotSpeed to minRotSpeed: big rabbits turn slower.
        'rot_speed': [d['maxRotSpeed'], d['minRotSpeed']], 'run_lerp': d['lerp'],
        'eyes': [node(renderer_transform(source, source.resolve(nyul, r))) for r in d['Eye']]}


def renderer_transform(source, renderer):
    return source.transform_for(source.resolve(renderer, source.read(renderer)['m_GameObject']))


def damage_record(source, nyul, node):
    d = source.read(nyul)
    reader = source.resolve(nyul, d['breakable'])
    rules = breakable(source, reader, actor=True)
    children = []
    for pointer in source.read(reader)['childs']:
        child = source.resolve(reader, pointer)
        f = source.read(child)
        children.append({'source': source_id(child), 'node': node(source.transform_for(source.resolve(child, f['m_GameObject']))),
            'lose_eye_chance': f['looseEyeChance'], 'lose_ear_chance': f['looseEarChance'], 'lose_sense_force': f['looseSenseForce'],
            'senses': source.resolve(child, f['senses']) is not None})
    return {'rules': rules, 'children': children}


def collider_record(source, matrix, reader, record_, pose, sensor):
    d = source.read(reader)
    layer = source.read(record_['game_object'])['m_Layer']
    material = source.resolve(reader, d['m_Material'])
    properties = source.read(material) if material else {'dynamicFriction': .6, 'bounciness': 0.}
    return {'source': source_id(reader), 'sensor': sensor, 'membership': 1 << layer, 'filter': matrix[layer],
        'friction': properties['dynamicFriction'], 'restitution': properties['bounciness'],
        'geometry': geometry(source, None, reader, record_['reader'], pose, True),
        'step_surface': step_surface(source, record_['game_object']), 'tag': collider_tag(source, record_['game_object'])}


def animator_record(source, animator, node):
    """The base layer default state: the `RunValue` 1D blend of the idle and run clips."""
    d = source.read(animator)
    controller = source.resolve(animator, d['m_Controller'])
    data = source.read(controller)
    names = dict(data['m_TOS'])
    c = data['m_Controller']
    if len(c['m_StateMachineArray']) != 1 or len(c['m_LayerArray']) != 1:
        raise ValueError('Rabbit Animator layer count differs')
    machine = c['m_StateMachineArray'][0]['data']
    state = machine['m_StateConstantArray'][machine['m_DefaultState']]['data']
    nodes = state['m_BlendTreeConstantArray'][0]['data']['m_NodeArray']
    blend = nodes[0]['data']
    if blend['m_BlendType'] != 0 or not state['m_Loop'] or state['m_Speed'] != 1:
        raise ValueError('Rabbit default state is not a looping 1D blend')
    thresholds = blend['m_Blend1dData']['data']['m_ChildThresholdArray']
    slots = [source.resolve(controller, p) for p in data['m_AnimationClips']]
    children = [nodes[i]['data'] for i in blend['m_ChildIndices']]
    if len(thresholds) != len(children) or any(n['m_ChildIndices'] for n in children):
        raise ValueError('Rabbit blend children differ')
    def bind(b, target):
        if b['type_id'] != 4 or b['dimension'] != (4 if b['attribute'] == 2 else 3) or b['is_pptr'] or b['script'] is not None:
            raise ValueError('Unsupported rabbit track binding')
        return node(source.get(target['file'], target['path_id']))
    clips = motion_clips(source, [source_id(slots[n['m_ClipID']]) for n in children], animator, bind)
    return {'source': source_id(animator), 'parameter': names[blend['m_BlendEventID']],
        'thresholds': thresholds, 'clips': clips}


def recover_creatures(source, manifest, output):
    from .ragdoll import recover_rig, recover_joints
    nodes = {key(source.get(n['source']['file'], n['source']['path_id'])): n for n in manifest['nodes']}
    parents = {n['id']: n['parent'] for n in manifest['nodes']}
    def node(reader):
        if reader is None or key(reader) not in nodes:
            raise ValueError('Rabbit link outside retained hierarchy')
        return nodes[key(reader)]['id']
    root = prefab_root(source)
    asset = next(a for a in manifest['assets'] if a['usage'] == 'creature_prototype' and a['root'] == source_id(root))
    if key(root) != getattr(source, 'creature_root', None) or nodes[key(root)]['asset'] != asset['id']:
        raise ValueError('Rabbit asset root')
    records = source.hierarchy(root, activate_root=True)
    by_node = {nodes[key(r['reader'])]['id']: r for r in records}
    nyul = component(source, root, 'nyulscript')
    ai = component(source, root, 'aiscript')
    move = component(source, root, 'movecreaturescript')
    senses = component(source, root, 'sensoryscript')
    ragdoll = component(source, root, 'ragdollactivatescript')
    save = component(source, root, 'tosaveitemscript')
    rd = source.read(ragdoll)
    if rd['ragdoll'] or rd['dontChangeRagdolCollidersEnabled']:
        raise ValueError('Rabbit ragdoll start state differs')
    live = source.resolve(ragdoll, rd['RB'])
    live_data = source.read(live)
    if live_data['m_IsKinematic'] or not live_data['m_UseGravity']:
        raise ValueError('Rabbit live body differs')
    # SetRagdol(true) makes these bodies dynamic with gravity and enables these colliders.
    bones = {key(source.resolve(ragdoll, p)) for p in rd['RBs']}
    ragdoll_colliders = {key(source.resolve(ragdoll, p)) for p in rd['ragdollColliders']}
    anim_colliders = [source.resolve(ragdoll, p) for p in rd['animColliders']]
    bodies, colliders, poses = recover_rig(source, root, by_node, parents, lambda c, d: key(c) in bones,
        lambda c: key(c) in ragdoll_colliders, bool(rd['ragdollCollidersTriggerWhenOn']))
    if len(bodies) != len(bones) or len(colliders) != len(ragdoll_colliders):
        raise ValueError('Rabbit ragdoll records differ')
    joints = recover_joints(source, {n: r for n, r in by_node.items() if n in {b['node'] for b in bodies}}, bodies, poses)
    matrix = source.read(next(r for r in source.objects.values() if r.type.name == 'PhysicsManager'))['m_LayerCollisionMatrix']
    by_reader = {key(r['reader']): r for r in records}
    live_colliders = []
    for c in anim_colliders:
        owner = by_reader[key(source.transform_for(source.resolve(c, source.read(c)['m_GameObject'])))]
        pose = world_pose(source, owner['reader']); pose['scale'] = [1., 1., 1.]
        # Collider geometry is in the space of its own node. The runtime moves bone nodes with the animation.
        live_colliders.append({'node': node(owner['reader']),
            **collider_record(source, matrix, c, owner, pose, bool(source.read(c)['m_IsTrigger']))})
    clips = [source.resolve(nyul, c) for c in source.read(nyul)['Sdie']]
    for clip in clips:
        if clip is None or clip.type.name != 'AudioClip':
            raise ValueError('Rabbit die clip is not AudioClip')
    return {'rabbit': {'source': source_id(root), 'asset': asset['id'], 'root': node(root), 'item_id': source.read(save)['id'],
        'spawner': spawner_record(source), 'brain': brain_record(source, ai), 'motion': motion_record(source, move),
        'senses': senses_record(source, senses, node), 'size': size_record(source, nyul, node),
        'damage': damage_record(source, nyul, node),
        'body': {'source': source_id(live), 'mass_kg': live_data['m_Mass'], 'constraints': live_data['m_Constraints'],
                 'linear_damping': live_data['m_Drag'], 'angular_damping': live_data['m_AngularDrag'],
                 'colliders': live_colliders},
        'ragdoll': {'source': source_id(ragdoll), 'bodies': bodies, 'colliders': colliders, 'joints': joints},
        'animator': animator_record(source, source.resolve(nyul, source.read(nyul)['anim']), node),
        # recover_audio replaces these AudioClip identities with clip ids and sets the shared one-shot voice.
        'die_clips': [source_id(c) for c in clips], 'die_voice': None,
        'evidence': managed_evidence(source, EVIDENCE)},
        'stimuli': recover_stimuli(source)}


def recover_stimuli(source):
    """Every stimulscript, as rows the runtime binds to the objects that carry them.
    `root` is the top Transform of the owner hierarchy. Sound values come from `audio_sources` or from code."""
    scripts = {key(o) for o in source.objects.values() if o.type.name == 'MonoScript' and source.read(o)['m_ClassName'] == STIMULUS_CLASS}
    rows = []
    for o in source.objects.values():
        if o.type.name != 'MonoBehaviour':
            continue
        if not any((d['file'], d['path_id']) in scripts for d in source.library.source_objects[ObjectId(*key(o))]['dependencies']):
            continue
        d = source.read(o)
        transform = source.transform_for(source.resolve(o, d['m_GameObject']))
        top = transform
        while (father := source.resolve(top, source.read(top)['m_Father'])) is not None:
            top = father
        rows.append({'source': source_id(o), 'node': source_id(transform), 'root': source_id(top),
            'visible': bool(d['visible']), 'sound': bool(d['sound']), 'smell': bool(d['smell']),
            'stimuli': [{'kind': STIMULUS_KINDS[s['tipus']], 'value': s['value']} for s in d['stimuls']],
            'max_sound_value': d['maxSoundValue'], 'check_audio_sources': not d['DontStartCheckingAudioSources'],
            'audio_sources': [source_id(a) for a in (source.resolve(o, p) for p in d['audioSources']) if a is not None]})
    rows.sort(key=lambda r: (r['source']['file'], r['source']['path_id']))
    return rows


def validate_creatures(c, manifest):
    finite_record(c)
    r = c['rabbit']
    nodes = {n['id']: n for n in manifest['nodes']}
    assets = {a['id']: a for a in manifest['assets']}
    def local(n):
        return nodes.get(n, {}).get('asset') == r['asset']
    pair = array(number, 2)
    record(r['spawner'], {'source': source_shape, 'chance': number, 'time_range': pair, 'count_range': pair, 'max_count': u32,
        'distance_range': pair, 'offset': array(number, 3), 'radius': number, 'half_fov': number,
        'surface_mask': u32, 'sphere_mask': u32})
    s = r['spawner']
    if not 0 <= s['time_range'][0] <= s['time_range'][1] or not 0 <= s['distance_range'][0] <= s['distance_range'][1] or s['max_count'] == 0:
        raise ValueError('Invalid rabbit spawner')
    if assets.get(r['asset'], {}).get('usage') != 'creature_prototype' or nodes.get(r['root'], {}).get('source') != r['source']:
        raise ValueError('Rabbit asset identity')
    ray = lambda v: record(v, {'enabled': boolean, 'inverted': boolean, 'symmetry': boolean, 'boolean': boolean,
        'direction': array(number, 3), 'length': number, 'mask': u32, 'max_value': number, 'spread': number})
    record(r['senses'], {'source': source_shape, 'scan_interval': number, 'eyes': u32, 'eye_objects': array(u32), 'fov': array(number, 3),
        'hear_distance': number, 'min_hear_distance': number, 'min_hear_value': number, 'hear_modifier': number,
        'stimulus_mask': u32, 'vision_mask': u32, 'rays': array(ray)})
    if r['senses']['scan_interval'] <= 0 or not local(r['senses']['eyes']) or not all(local(n) for n in r['senses']['eye_objects']):
        raise ValueError('Invalid rabbit senses')
    size = r['size']
    if any(size[k][0] < 0 or size[k][1] < 0 for k in ('scale', 'mass', 'health', 'shoot_health', 'speed', 'rot_speed')) or size['scale'][0] <= 0:
        raise ValueError('Invalid rabbit size ranges')
    if not all(local(n) for n in size['eyes']):
        raise ValueError('Invalid rabbit eyes')
    body = r['body']
    if body['mass_kg'] <= 0 or not body['colliders'] or not all(local(x['node']) for x in body['colliders']):
        raise ValueError('Invalid rabbit live body')
    rag = r['ragdoll']
    if not rag['bodies'] or [b['id'] for b in rag['bodies']] != list(range(1, len(rag['bodies']) + 1)) or not all(local(b['node']) for b in rag['bodies']):
        raise ValueError('Invalid rabbit ragdoll bodies')
    ids = {b['id'] for b in rag['bodies']}
    if any(x['body'] not in ids for x in rag['colliders']) or any(j['body'] not in ids or j['connected_body'] not in ids for j in rag['joints']):
        raise ValueError('Invalid rabbit ragdoll links')
    anim = r['animator']
    if len(anim['thresholds']) != len(anim['clips']) or len(anim['clips']) < 2 or any(b <= a for a, b in zip(anim['thresholds'], anim['thresholds'][1:])):
        raise ValueError('Invalid rabbit blend tree')
    for clip in anim['clips']:
        if not clip['loop'] or not clip['start_time'] < clip['stop_time'] or any(not local(t['node']) for t in clip['tracks']):
            raise ValueError('Invalid rabbit clip')
    clips = {x['id'] for x in manifest['audio']['clips']}
    shots = {v['id'] for v in manifest['audio']['car_voices'] if v['one_shot']}
    if not r['die_clips'] or any(x not in clips for x in r['die_clips']) or r['die_voice'] not in shots:
        raise ValueError('Invalid rabbit die sounds')
    for row in c['stimuli']:
        record(row, {'source': source_shape, 'node': source_shape, 'root': source_shape, 'visible': boolean, 'sound': boolean,
            'smell': boolean, 'stimuli': array(lambda v: record(v, {'kind': lambda k: None, 'value': number})),
            'max_sound_value': number, 'check_audio_sources': boolean, 'audio_sources': array(source_shape)})
        if any(s['kind'] not in STIMULUS_KINDS for s in row['stimuli']):
            raise ValueError('Invalid stimulus kind')
    if not any(row['visible'] for row in c['stimuli']):
        raise ValueError('No visible stimulus source')
