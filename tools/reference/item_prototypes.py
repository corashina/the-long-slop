"""Checked physical templates. All body and collider IDs are prototype-local."""
from .source import key, source_id
from .actions import recover_pickup
from .collision import world_pose, bake_points, quaternion_product, geometry, step_surface, collider_tag
from .gltf import GltfExporter
from .survival import component, managed_evidence, finite_record

# Checked prefab root Transforms, database numbers, pickup counts, and asset names.
SELECTED = ((75443,74,1,'item_pack'),(51722,62,1,'item_box'),(61587,216,1,'item_shovel'),
            (81642,None,56,'item_box_debris'),(81645,None,14,'item_box_debris_reduced'),
            (70812,217,1,'item_sunglasses'),(53838,218,1,'item_hat'),(53837,221,1,'item_cap'),
            (63605,245,1,'item_cone'),(63259,92,1,'item_pumpkin'),(54495,230,1,'item_cigarette'),
            (83430,61,6,'item_siphon_hose'),(51557,312,1,'item_tumbleweed'))


def prepare_item_prototypes(source, roots):
    source.item_geometry_nodes = set()
    for pid, _, _, name in SELECTED:
        root = source.get('sharedassets1.assets', pid)
        roots[name] = [root]
        source.item_geometry_nodes.update(key(n['reader']) for n in source.hierarchy(root, activate_root=True))


def recover_item_prototypes(source, manifest, output) -> list[dict]:
    managed_evidence(source, ['disableThisWhenStored = transform;',
        'spawnedGib = UnityEngine.Object.Instantiate(reducedGib, transform.position, transform.rotation);',
        'spawnedGib = UnityEngine.Object.Instantiate(Gib, transform.position, transform.rotation);'])
    nodes = {(n['source']['file'], n['source']['path_id']):n for n in manifest['nodes']}
    matrix = source.read(next(r for r in source.objects.values() if r.type.name=='PhysicsManager'))['m_LayerCollisionMatrix']
    exporter = GltfExporter(source, None, source.library)
    source.item_clips = set()
    result = []
    for pid, item_id, _, name in SELECTED:
        root = source.get('sharedassets1.assets', pid)
        records = source.hierarchy(root, activate_root=True)
        by_node = {nodes[key(r['reader'])]['id']:r for r in records}
        root_pose = world_pose(source, root)
        inverse = [-v for v in root_pose['rotation'][:3]] + [root_pose['rotation'][3]]
        bodies = []; body_nodes = {}; poses = {}
        for node, record in by_node.items():
            rigid = [c for c in record['components'] if c.type.name=='Rigidbody']
            if not rigid: continue
            if len(rigid)!=1: raise ValueError('Duplicate prototype Rigidbody')
            reader = rigid[0]; d = source.read(reader)
            pose = world_pose(source, record['reader']); pose['scale'] = [1.,1.,1.]
            poses[node] = pose
            local_pose = {'translation':bake_points([(0.,0.,0.)],source.chain(record['reader']),root_pose)[0],
                'rotation':list(quaternion_product(inverse,pose['rotation'])), 'scale':[1.,1.,1.]}
            mass = component(source,record['reader'],'massScript')
            bodies.append({'id':len(bodies)+1,'source':source_id(reader),'node':node,'dynamic':not d['m_IsKinematic'],
                'mass_kg':source.read(mass)['mass'],'linear_damping':d['m_Drag'],'angular_damping':d['m_AngularDrag'],
                'pose':local_pose,'parent_body':None})
            body_nodes[node] = bodies[-1]['id']
        def body_for(node):
            while node is not None:
                if node in body_nodes:return body_nodes[node]
                node = manifest['nodes'][node-1]['parent']
            return None
        for body in bodies:body['parent_body'] = body_for(manifest['nodes'][body['node']-1]['parent'])
        colliders=[];pickups=[];renderers=[];variants=[]
        for node, record in by_node.items():
            group = {(source.class_name(c) if c.type.name=='MonoBehaviour' else c.type.name):c for c in record['components']}
            for c in record['components']:
                if c.type.name not in ('BoxCollider','SphereCollider','CapsuleCollider','MeshCollider'):continue
                d=source.read(c); owner=body_for(node)
                if owner is None:raise ValueError(f'Prototype collider lacks body: {key(c)}')
                body=bodies[owner-1];layer=source.read(record['game_object'])['m_Layer']
                material=source.resolve(c,d['m_Material'])
                properties=source.read(material) if material else {'m_DynamicFriction':.6,'m_Bounciness':0.}
                colliders.append({'id':len(colliders)+1,'source':source_id(c),'node':node,'body':owner,
                    'sensor':bool(d['m_IsTrigger']),'enabled':bool(d['m_Enabled']), 'membership':1<<layer,'filter':matrix[layer],
                    'friction':properties['m_DynamicFriction'],'restitution':properties['m_Bounciness'],
                    'geometry':geometry(source,exporter,c,record['reader'],poses[body['node']],body['dynamic']),
                    'step_surface':step_surface(source,record['game_object']),'tag':collider_tag(source,record['game_object'])})
            if 'pickupable' in group:
                c=group['pickupable'];d=source.read(c)
                clip=source.resolve(c,d['collideSound'])
                if clip:
                    if clip.type.name!='AudioClip':raise ValueError('Invalid pickup collision clip')
                    source.item_clips.add(key(clip))
                pickups.append({'node':node,'body':body_for(node),'pickup':recover_pickup(source,c),
                    'collision_clip':source_id(clip) if clip else None,'collision_gain':d['collisionSoundM']})
            if 'MeshRenderer' in group:
                c=group['MeshRenderer'];d=source.read(c);mesh=source.resolve(group['MeshFilter'],source.read(group['MeshFilter'])['m_Mesh'])
                from .render_lighting import recover_renderer
                renderers.append({'state':recover_renderer(source,c,mesh,'item_prototype'),'source':source_id(c),'node':node,'mesh':source_id(mesh),
                    'enabled':bool(d['m_Enabled']), 'materials':[source_id(source.resolve(c,p)) for p in d['m_Materials']]})
            if 'partconditionscript' in group and item_id not in (217,218,221,245,92,230):
                c=group['partconditionscript'];d=source.read(c)
                if d['UseMatList'] or d['UseMesList'] or d['useRandomTipus']:raise ValueError('Unaudited physical prototype selector')
                if d['UseGoList']:
                    variants.append({'source':source_id(c),'node':node,'nodes':[nodes[key(source.transform_for(source.resolve(c,p)))]['id'] for p in d['GoList']],
                        'authored_index':d['state3'],'random_start':bool(d['randomStart'])})
        def link(c, field, kind):
            target=source.resolve(c,source.read(c)[field])
            if target is None or target.type.name!=kind:raise ValueError(f'Invalid prototype {field}: {key(c)}')
            return target
        behavior={'pack':None,'ammo':None,'cigarette_prefab':None,'spawn':None,'breakable':None,'mount':None,
            'debris':None,'reduced_debris':None,'melee':None,'attack_target':None,'attack_trigger':None,'gib':None,'gib_children':[],'clips':[],'contact_sound_source':None,'contact_clips':[]}
        if item_id==74:
            c=component(source,root,'cigiscript');behavior.update(pack=source_id(c),ammo=source_id(component(source,root,'ammoscript')),
                cigarette_prefab=source_id(link(c,'prefab','GameObject')))
        elif item_id==62:
            c=component(source,root,'breakablescript');spawn=component(source,root,'spawnInBoxScript')
            behavior.update(breakable=source_id(c),spawn=source_id(spawn),mount=source_id(link(spawn,'mount','MonoBehaviour')),
                debris=source_id(source.transform_for(link(c,'Gib','GameObject'))),
                reduced_debris=source_id(source.transform_for(link(c,'reducedGib','GameObject'))))
            behavior['clips']=[source_id(source.resolve(c,p)) for p in source.read(c)['clips']]
        elif item_id==216:
            c=component(source,root,'meleeweaponscript')
            behavior.update(melee=source_id(c),attack_target=source_id(link(c,'T','Transform')),attack_trigger=source_id(link(c,'Trigger','Transform')))
            behavior['clips']=[source_id(source.resolve(c,p)) for p in source.read(c)['attackClips']]
            main=source.get('level1',64919)
            behavior['contact_sound_source']=source_id(main)
            behavior['contact_clips']=[[source_id(source.resolve(main,p)) for p in group['a']] for group in source.read(main)['meleeHitSounds']]
            managed_evidence(source,['mainscript.PlayClipAtPoint(mainscript.M.meleeHitSounds[(int)hitType].a[num], V, mainscript.hitVolume[(int)hitType], mainscript.AudioPriorities[7]);'])
        elif item_id is None:
            c=component(source,root,'gibscript')
            behavior.update(gib=source_id(c),gib_children=[nodes[key(source.resolve(c,p))]['id'] for p in source.read(c)['childs']])
        for identity in [*behavior['clips'],*(c for group in behavior['contact_clips'] for c in group)]:
            c=source.get(identity['file'],identity['path_id'])
            if c.type.name!='AudioClip':raise ValueError('Invalid behavior clip')
            source.item_clips.add(key(c))
        from .survival import recover_cigarette
        consumable = recover_cigarette(source, root, manifest, output) if item_id==230 else None
        from .hose import recover_hose
        hose = recover_hose(source,by_node,bodies,poses,{k:n['id'] for k,n in nodes.items()},exporter,source.read(source.get('level1',64919)),output) if item_id==61 else None
        result.append({'hose':hose,'consumable':consumable,'id':source_id(root),'source':source_id(root),'item_id':item_id,
            'asset':nodes[key(root)]['asset'],'root':nodes[key(root)]['id'],'bodies':bodies,'colliders':colliders,
            'pickups':pickups,'variants':variants,'renderers':renderers,'behavior':behavior})
    return result


def validate_pickup(pickup, manifest, asset):
    finite_record(pickup)
    by_source={(n['source']['file'],n['source']['path_id']):n for n in manifest['nodes']}
    for field in ('right_hand_grip','left_hand_grip','disable_when_stored','keep_when_stored'):
        value=pickup[field]
        if field=='disable_when_stored' and value is None:raise ValueError('Missing effective storage Transform')
        if value and by_source.get((value['file'],value['path_id']),{}).get('asset')!=asset:raise ValueError('Foreign pickup Transform')
    icon=pickup['inventory_icon']
    if icon and not any(s['source']==icon for s in manifest['ui']['sprites']):raise ValueError('Missing pickup sprite')
    for field in ('right_hand_rotation','recenter_rotation'):
        if abs(sum(v*v for v in pickup[field])-1)>0.002:raise ValueError('Invalid pickup rotation')


def validate_item_prototypes(prototypes, manifest) -> None:
    finite_record(prototypes)
    if [(p['id'],p['item_id']) for p in prototypes] != [({'file':'sharedassets1.assets','path_id':pid},item) for pid,item,_,_ in SELECTED]:raise ValueError('Selected prototype identities')
    nodes={n['id']:n for n in manifest['nodes']};assets={a['id']:a for a in manifest['assets']}
    def unique(values):
        values=list(values)
        if len(set(values))!=len(values):raise ValueError('Duplicate prototype local identity')
    for p,(_,_,count,_) in zip(prototypes,SELECTED,strict=True):
        if p['source']!=p['id'] or nodes.get(p['root'],{}).get('source')!=p['id'] or assets.get(p['asset'],{}).get('usage')!='item_prototype':raise ValueError('Invalid prototype root')
        def within(node,root=p['root']):
            seen=set()
            while node is not None:
                if node in seen or nodes.get(node,{}).get('asset')!=p['asset']:return False
                seen.add(node)
                if node==root:return True
                node=nodes[node]['parent']
            return False
        if not within(p['root']) or nodes[p['root']]['parent'] is not None or assets[p['asset']]['root']!=p['source']:raise ValueError('Foreign prototype root')
        counts={75443:(1,3),51722:(2,3),61587:(2,3),81642:(58,56),81645:(15,14),70812:(3,25),53838:(2,4),53837:(1,2),63605:(12,2),63259:(3,2),54495:(1,7),83430:(6,0),51557:(1,1)}
        if (len(p['colliders']),len(p['renderers']))!=counts[p['id']['path_id']]:raise ValueError('Incomplete selected geometry')
        bodies={b['id']:b for b in p['bodies']}
        for records in (p['bodies'],p['colliders']):unique(r['id'] for r in records);unique((r['source']['file'],r['source']['path_id']) for r in records)
        unique(b['node'] for b in p['bodies']);unique(x['node'] for x in p['pickups']);unique((x['pickup']['source']['file'],x['pickup']['source']['path_id']) for x in p['pickups'])
        def owner(node,exclude=False):
            if exclude:node=nodes[node]['parent']
            while node is not None and within(node):
                found=next((b['id'] for b in p['bodies'] if b['node']==node),None)
                if found:return found
                node=nodes[node]['parent']
            return None
        for b in p['bodies']:
            if not within(b['node']) or b['mass_kg']<=0 or b['parent_body']!=owner(b['node'],True):raise ValueError('Invalid local body')
        for c in p['colliders']:
            if not within(c['node']) or c['body'] not in bodies or c['body']!=owner(c['node']):raise ValueError('Wrong collider owner')
        if len(p['pickups'])!=count or len(p['bodies'])!=count:raise ValueError('Missing selected pickup bodies')
        for x in p['pickups']:
            pickup=x['pickup'];source=pickup['source']
            exceptions={(81642,135313):77169,(81642,140775):82214,(81645,135314):77170}
            exception=exceptions.get((p['id']['path_id'],source['path_id']))
            expected_icon={61:None,74:4624,62:None,216:4553,None:4623,217:4557,218:4527,221:4529,245:None,92:4590,230:None,312:4569}[p['item_id']] if exception is None else None
            icon=pickup['inventory_icon']
            if source['file']!='sharedassets1.assets' or pickup['can_hold_right_hand']!=(p['item_id']==216):raise ValueError('Pickup source eligibility')
            # Hose segment eligibility and icons are not checked yet against build 11140890.
            if p['item_id']!=61 and pickup['can_inventory']!=(p['item_id'] not in (62,245,230,312) and exception is None):raise ValueError('Pickup source eligibility')
            if p['item_id']!=61 and icon!=({'file':'sharedassets1.assets','path_id':expected_icon} if expected_icon else None):raise ValueError('Pickup source icon')
            if exception is not None and nodes.get(x['node'],{}).get('source')!={'file':'sharedassets1.assets','path_id':exception}:raise ValueError('Debris eligibility owner')
            if not within(x['node']) or x['body'] not in bodies or x['body']!=owner(x['node']):raise ValueError('Wrong pickup body')
            validate_pickup(x['pickup'],manifest,p['asset'])
            if x['collision_clip'] and not any(c['source']==x['collision_clip'] for c in manifest['audio']['clips']):raise ValueError('Missing collision clip')
        for c in [*p['behavior']['clips'],*(c for group in p['behavior']['contact_clips'] for c in group)]:
            if not any(a['source']==c for a in manifest['audio']['clips']):raise ValueError('Missing behavior clip')
        unique(r['node'] for r in p['renderers']);unique((r['source']['file'],r['source']['path_id']) for r in p['renderers'])
        for r in p['renderers']:
            state=r['state']
            shadow_only=r['source']['file']=='sharedassets1.assets' and r['source']['path_id'] in (98477,98194,93128,91870,98218)
            valid_shadow=state['cast_shadows']==3 if shadow_only else state['cast_shadows'] in (0,1)
            if state['source']!=r['source'] or not valid_shadow or state['reflection_probe_usage'] not in (0,1) or state['motion_vectors']!=1 or state['sorting_layer']!=0 or state['sorting_order']!=0:raise ValueError('Prototype renderer state')
            if len(state['bounds_center'])!=3 or len(state['bounds_extents'])!=3 or min(state['bounds_extents'])<0:raise ValueError('Prototype renderer bounds')
            if not within(r['node']) or r['source']['file']!='sharedassets1.assets' or not r['materials']:raise ValueError('Invalid renderer owner')
            if any(not any(m['asset']==p['asset'] and m['source']==material for m in manifest['rendering']['materials']) for material in r['materials']):raise ValueError('Missing renderer material slot')
        if p['item_id']==74:
            if len(p['variants'])!=1 or [nodes[n]['source']['path_id'] for n in p['variants'][0]['nodes']]!=[56803,56801,56802]:raise ValueError('Missing pack variants')
        elif p['variants']:raise ValueError('Unexpected prototype variants')
        from .survival import validate_cigarette
        from .hose import validate_hose
        if (p['hose'] is not None)!=(p['item_id']==61):raise ValueError('Hose owner')
        if p['hose'] is not None:
            validate_hose(p['hose'],p)
            from .particles import validate_particle
            for e in p['hose']['ends']:
                for particle in (e['visual'],e['splash']):validate_particle(particle,{t['id'] for t in manifest['lighting']['textures']})
        if (p['consumable'] is not None)!=(p['item_id']==230):raise ValueError('Cigarette consumable owner')
        if p['consumable'] is not None:validate_cigarette(p['consumable'],p,manifest)
        if p['item_id'] is None and p['behavior']['gib_children']!=[x['node'] for x in p['pickups']]:raise ValueError('Missing debris child owners')
        if (any(within(b['node']) for b in manifest['bodies']) or any(within(i['node']) for i in manifest['items'])
            or any(nodes.get(c['node'],{}).get('asset')==p['asset'] for c in manifest['colliders'])):
            raise ValueError('Prototype entered active tables')
