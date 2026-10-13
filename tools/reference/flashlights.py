"""Checked portable flashlight and vehicle storage bindings."""
import json
from .source import key, source_id

ROOTS=(32123,32124)
VOLUMES=(65838,65854,67560,79410,77806,78464,79301,69971)

def managed_contract(source):
    from .library.reader import checked
    library=source.library
    managed=library.manifest['managed_analysis']
    receipts={record['file']:record for record in managed['files']}
    code=checked(library.records_root,receipts['managed/Assembly-CSharp.cs']).decode('utf-8').replace('\r\n','\n')
    decomp=json.loads(checked(library.records_root,receipts['managed/decompilation.json']))
    assembly='TheLongDrive_Data/Managed/Assembly-CSharp.dll'
    if decomp['input']['sha256']!=library.source_files[assembly].sha256:
        raise ValueError('Flashlight managed source provenance differs')
    def selected(name):
        start=code.index(f'public class {name} : MonoBehaviour')
        end=code.find('\npublic class ',start+1)
        return code[start:end if end!=-1 else len(code)]
    required={
        'flashlightscript':('public void TURN()','public void Refresh(bool sendmulti = true)','return true;'),
        'mountStuff':('private static float neededDropTime = 1f;','yield return null;\n\t\tyield return null;',
                      'QuickFreeze(i == 0);','lastcols = cols;','if (cols[i] == lastcols[j])',
                      'GetValue() > 0.1f'),
        'mainscript':('Time.time - lastPhysicsLockTime1 > 1.5f','UpdPhysicsLock1();'),
    }
    for name,expressions in required.items():
        class_code=selected(name)
        for expression in expressions:
            if expression not in class_code:raise ValueError(f'{name} managed source differs: {expression}')
    return {'assembly':{'file':assembly,'sha256':decomp['input']['sha256']},
            'source':receipts['managed/Assembly-CSharp.cs']}

def prepare_flashlight_roots(source, roots):
    source.flashlight_root_keys={('level1',i) for i in ROOTS}
    source.flashlight_inactive_nodes={key(r['reader']) for r in source.hierarchy(source.get('level1',32124))}
    source.storage_collider_keys={key(source.resolve(r,source.read(r)['col'])) for r in (source.get('level1',i) for i in VOLUMES)}
    for i in ROOTS:roots['items'].append(source.get('level1',i))

def recover_flashlights(source, roots, pointer_node, item_nodes, controllers, colliders, output, part_slots):
    def linked(reader,pointer):return source.resolve(reader,pointer) if pointer['m_PathID'] else None
    def sid(reader,pointer):
        target=linked(reader,pointer)
        return source_id(target) if target else None
    models=[];instances=[];evidence={'startup':{},'bodies':[],'storage':[],'prefabs':[],
                                      'managed':managed_contract(source)}
    terrain=source.get('level1',64898)
    if ('level1',117) not in [key(source.resolve(terrain,p)) for p in source.read(terrain)['EnableAfterStart']]:raise ValueError('StartSpawnItems activation link missing')
    expected_beams=((56881,12.,0.8500000238418579,90.,8.,(1.,0.8154796361923218,0.6009999513626099,1.)),
                    (56882,15.,1.5,80.,62.188602447509766,(1.,1.,1.,1.)))
    for transform_id,prefab_id,control_id,pickup_id,save_id,body_id,beam in zip(ROOTS,(28750,28749),(74834,74835),(69340,69341),(74314,74315),(50144,50143),expected_beams):
        root=source.get('level1',transform_id);go,components=source.components(root);active=bool(source.read(go)['m_IsActive'])
        ctrl=source.get('level1',control_id);cf=source.read(ctrl);pickup=source.get('level1',pickup_id);pf=source.read(pickup);save=source.get('level1',save_id)
        if any(key(source.resolve(r,source.read(r)['m_GameObject']))!=key(go) for r in (ctrl,pickup,save)):raise ValueError('Flashlight owner links differ')
        if key(linked(pickup,pf['lampa']))!=key(ctrl) or cf['ON'] or cf['randomStart'] or cf['SingleLight']['m_PathID'] or cf['electronics']['m_PathID'] or any(l['breaktrigger'] for l in cf['Lamps']):raise ValueError('Flashlight switch contract differs')
        body=source.get('level1',body_id);bf=source.read(body)
        if (bf['m_Mass'],bf['m_Drag'],bf['m_AngularDrag'],bool(bf['m_UseGravity']),bool(bf['m_IsKinematic']))!=(1.,0.,0.05000000074505806,True,False):raise ValueError('Flashlight body contract differs')
        node=pointer_node(ctrl,cf['m_GameObject']);control=next(c for c in controllers if c['source']==source_id(ctrl));control['enabled']=active and control['enabled']
        if len(control['states'])!=1 or len(control['states'][0]['lights'])!=1:
            raise ValueError(f'{key(ctrl)} flashlight state count differs')
        light=source.lighting_lights[control['states'][0]['lights'][0]-1]
        if (light['source']!=source_id(source.get('level1',beam[0]))
                or (light['kind'],light['range_meters'],light['serialized_intensity'],light['full_cone_degrees'],light['inner_cone_degrees'],tuple(light['color']))!=('spot',*beam[1:])
                or light['culling_mask']!=0xffffffff or light['cookie'] is None or light['shadows']['kind']!=1
                or (light['shadows']['strength'],light['shadows']['bias'],light['shadows']['normal_bias'],light['shadows']['near_plane'])!=(1.,0.05000000074505806,0.4000000059604645,0.20000000298023224)):
            raise ValueError(f'{key(ctrl)} flashlight beam differs')
        prefab=source.get('sharedassets1.assets',prefab_id);proot=source.transform_for(prefab);records=source.hierarchy(proot)
        meshes=[];materials=[];renderers=[];lights=[]
        for r in records:
            for c in r['components']:
                f=source.read(c)
                if c.type.name=='MeshFilter':meshes.append(sid(c,f['m_Mesh']))
                if c.type.name=='MeshRenderer':
                    renderers.append(source_id(c));materials.extend(sid(c,p) for p in f['m_Materials'])
                if c.type.name=='Light':lights.append(source_id(c))
        pc=source.get('sharedassets1.assets',148780 if prefab_id==28750 else 148779)
        models.append({'source':source_id(prefab),'controller':source_id(pc),'node':node,'meshes':meshes,'materials':materials,'renderers':renderers,'lights':lights})
        instances.append({'model':source_id(prefab),'item':item_nodes[node],'node':node,'pickup':source_id(pickup),'save':source_id(save),'controller':control['id'],'startup_active':active,'colliders':control['aim_colliders']})
        evidence['bodies'].append({'source':source_id(body),'fields':bf});evidence['prefabs'].append({'source':source_id(prefab),'controller':source.read(pc),'pickup':pf})
    if [i['startup_active'] for i in instances]!=[True,False]:raise ValueError('Flashlight startup activation differs')
    slot_ids={(s['source']['file'],s['source']['path_id']):s['id'] for s in part_slots}
    collider_ids={(c['source']['file'],c['source']['path_id']):c['id'] for c in colliders}
    volumes=[]
    for i in VOLUMES:
        r=source.get('level1',i);f=source.read(r);col=linked(r,f['col']);owner=pointer_node(r,f['parentTosave'])
        volumes.append({'source':source_id(r),'node':pointer_node(r,f['m_GameObject']),'owner':item_nodes[owner],'collider':collider_ids[key(col)],'storage_type':f['sType'],'only_manual':bool(f['onlyManual']),'no_physics_lock':bool(f['noPhysicsLock']),'owner_pickup':sid(r,f['Pick']),'dont_lock_if_owner_picked_up':bool(f['dontLockIfPickedUp']),'slots':[slot_ids[key(linked(r,p))] for p in f['slot']],'usables':[sid(r,p) for p in f['susable']],'active':bool(f['m_Enabled']) and bool(source.read(source.resolve(r,f['m_GameObject']))['m_IsActive'])})
        evidence['storage'].append({'source':source_id(r),'fields':f})
    expected_types=(0,0,5,5,5,5,3,4)
    expected_slots=((),(),(),(),(),(),(77859,),(77842,))
    for volume,kind,slots in zip(volumes,expected_types,expected_slots):
        source_reader=source.get(volume['source']['file'],volume['source']['path_id'])
        fields=source.read(source_reader)
        selected_slots=tuple(linked(source_reader,p).path_id for p in fields['slot'])
        if (fields['parentTosave']['m_PathID']!=73405 or volume['storage_type']!=kind
                or selected_slots!=slots or volume['usables'] or volume['only_manual']
                or volume['no_physics_lock'] or not volume['active']):
            raise ValueError(f"{volume['source']} storage policy differs")
    dropdown=source.get('level1',70023);df=source.read(dropdown);settings=source.get('level1',65886);source.read(settings)
    if not df['useDefInt'] or df['defInt']!=2 or key(linked(dropdown,df['setting']))!=key(settings):raise ValueError('Storage fresh default differs')
    main=source.get('level1',64919);mask=source.read(main)['mountLayerMask']['m_Bits']
    if mask!=8388609:raise ValueError('level1:64919 storage layer mask differs')
    evidence['startup']={'terrain':source_id(terrain),'parent':source_id(source.get('level1',117)),'dropdown':df}
    (output/'flashlights-evidence.json').write_text(json.dumps(evidence,indent=2)+'\n')
    return {'models':models,'instances':instances,'storage':{'settings_source':source_id(settings),'default_setting':df['defInt'],'mount_mask':mask,'drop_delay_seconds':1.,'scan_interval_seconds':1.5,'volumes':volumes}}
