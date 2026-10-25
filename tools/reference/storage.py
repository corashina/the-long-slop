"""Checked source mountStuff storage volumes, policy, and item flags."""
import json
import re
from .source import key, source_id, vector
from .gltf import convert_point, convert_rotation, transform_point
from .collision import quaternion_product
from .managed_code import managed_code, class_code, method, require

VOLUMES=(65838,65854,67560,79410,77806,78464,79301,69971)
OWNER=73405
TYPES=(0,0,5,5,5,5,3,4)
SLOTS=((),(),(),(),(),(),(77859,),(77842,))
# World prototype mountStuff: Box, Fridge, Raklap (pallet), Coffin (4).
PROTOTYPE_FILE='sharedassets1.assets'
PROTOTYPE_VOLUMES=(146589,146111,133282,144518,150537,134531,153699)
SETTING=2
MASK=8388609
DROP_DELAY=1.
SCAN_INTERVAL=1.5
REQUIRED={
    ('mountStuff','public void QuickFreeze(bool _prescan)'):('if (_prescan)','Scan();','if (CanStore())'),
    ('mountStuff','public void FreezeUpd()'):('lastcols = cols;','if (cols[i] == lastcols[j])'),
    ('mountStuff','public bool CanStore()'):('GetValue() > 0.1f',),
    ('mainscript','private void Update()'):('Time.time - lastPhysicsLockTime1 > 1.5f','UpdPhysicsLock1();'),
}

def managed_contract(source):
    code,contract=managed_code(source,'Storage')
    require(code,REQUIRED)
    if 'private static float neededDropTime = 1f;' not in class_code(code,'mountStuff'):
        raise ValueError('mountStuff drop delay differs')
    start_freeze=method(code,'mountStuff','private IEnumerator StartFreeze()')
    if not re.search(r'yield return null;\s*yield return null;\s*for \(int i = 0; i < 3; i\+\+\)\s*\{\s*QuickFreeze\(i == 0\);\s*yield return null;\s*\}',start_freeze):
        raise ValueError('mountStuff.StartFreeze three-pass initialization differs')
    return contract

def check_policy(dropdown, settings_linked, mask):
    if not dropdown['useDefInt'] or dropdown['defInt']!=SETTING or not settings_linked:raise ValueError('Storage fresh default differs')
    if mask!=MASK:raise ValueError('level1:64919 storage layer mask differs')

def check_volume(volume, owner, slots, expected):
    """Require the selected Lada owner, type, gate slots, and an active automatic lock."""
    kind,expected_slots=expected
    if (owner!=OWNER or volume['storage_type']!=kind or slots!=expected_slots
            or volume['usables'] or volume['only_manual'] or volume['no_physics_lock'] or not volume['active']):
        raise ValueError(f"{volume['source']} storage policy differs")

def box_shape(fields):
    return {'center':list(convert_point(vector(fields['m_Center']))),'half_extents':[x*0.5 for x in vector(fields['m_Size'])]}

def root_box(chain, fields):
    """Box centre, half extents, and rotation in the prototype root frame.
    The chain runs from the collider Transform up to the child of the root."""
    center=vector(fields['m_Center']);rotation=(0.,0.,0.,1.);scale=(1.,1.,1.)
    for data in chain:
        local=vector(data['m_LocalRotation'],4);size=vector(data['m_LocalScale'])
        center=transform_point(center,vector(data['m_LocalPosition']),local,size)
        rotation=quaternion_product(local,rotation)
        scale=tuple(scale[i]*size[i] for i in range(3))
    return {'center':list(convert_point(center)),'half_extents':[abs(scale[i])*x*0.5 for i,x in enumerate(vector(fields['m_Size']))],
            'rotation':list(convert_rotation(rotation))}

def containing(prototypes, node):
    """Return (prototype source, node index) for each prototype that holds this node source."""
    return [(p['source'],i) for p in prototypes for i,n in enumerate(p['nodes']) if n['source']==node]

def check_prototype_volume(name, fields, active, homes, usables, mechanisms):
    """Require an ungated, active volume inside a prototype whose usables are mechanisms.
    An onlyManual volume is kept: it skips the periodic Upd scan."""
    if not homes:raise ValueError(f'{name} storage volume is outside the world prototypes')
    if fields['slot'] or fields['noPhysicsLock'] or not active:
        raise ValueError(f'{name} storage policy differs')
    for prototype,_ in homes:
        for usable in usables:
            if (prototype['file'],prototype['path_id'],usable['file'],usable['path_id']) not in mechanisms:
                raise ValueError(f"{name} usable {usable['path_id']} is not a mechanism of {prototype['path_id']}")

def flag_entries(entries, field='item'):
    """Keep owners whose pickupable sets dontMount or dontMountTillPickup."""
    return [{field:owner,'dont_mount':bool(f['dontMount']),'dont_mount_till_pickup':bool(f['dontMountTillPickup'])}
            for owner,f in entries if f['dontMount'] or f['dontMountTillPickup']]

def prepare_storage_roots(source):
    source.storage_collider_keys={key(source.resolve(r,source.read(r)['col'])) for r in (source.get('level1',i) for i in VOLUMES)}

def recover_storage(source, pointer_node, item_nodes, components, colliders, part_slots, output):
    def linked(reader,pointer):return source.resolve(reader,pointer) if pointer['m_PathID'] else None
    def sid(reader,pointer):
        target=linked(reader,pointer)
        return source_id(target) if target else None
    evidence={'managed':managed_contract(source),'volumes':[]}
    slot_ids={(s['source']['file'],s['source']['path_id']):s['id'] for s in part_slots}
    collider_ids={(c['source']['file'],c['source']['path_id']):c['id'] for c in colliders}
    volumes=[]
    for i,kind,slots in zip(VOLUMES,TYPES,SLOTS):
        r=source.get('level1',i);f=source.read(r);col=linked(r,f['col']);owner=pointer_node(r,f['parentTosave'])
        if col is None or col.type.name!='BoxCollider':raise ValueError(f'level1:{i} storage collider is not a BoxCollider')
        cf=source.read(col)
        volume={'source':source_id(r),'node':pointer_node(r,f['m_GameObject']),'owner':item_nodes[owner],'collider':collider_ids[key(col)],**box_shape(cf),'storage_type':f['sType'],'only_manual':bool(f['onlyManual']),'no_physics_lock':bool(f['noPhysicsLock']),'owner_pickup':sid(r,f['Pick']),'dont_lock_if_owner_picked_up':bool(f['dontLockIfPickedUp']),'slots':[slot_ids[key(linked(r,p))] for p in f['slot']],'usables':[sid(r,p) for p in f['susable']],'active':bool(f['m_Enabled']) and bool(source.read(source.resolve(r,f['m_GameObject']))['m_IsActive'])}
        check_volume(volume,f['parentTosave']['m_PathID'],tuple(linked(r,p).path_id for p in f['slot']),(kind,slots))
        volumes.append(volume);evidence['volumes'].append({'source':source_id(r),'fields':f,'collider':cf})
    dropdown=source.get('level1',70023);df=source.read(dropdown);settings=source.get('level1',65886);source.read(settings)
    main=source.get('level1',64919);mask=source.read(main)['mountLayerMask']['m_Bits']
    check_policy(df,key(linked(dropdown,df['setting']))==key(settings),mask)
    flags=flag_entries(sorted(((item,source.read(components[node]['pickupable'][0])) for node,item in item_nodes.items() if 'pickupable' in components[node]),key=lambda e:e[0]))
    evidence.update(dropdown=df,mask=mask,flags=flags)
    (output/'storage-evidence.json').write_text(json.dumps(evidence,indent=2)+'\n')
    return {'policy':{'settings_source':source_id(settings),'default_setting':df['defInt'],'mount_mask':mask,'drop_delay_seconds':DROP_DELAY,'scan_interval_seconds':SCAN_INTERVAL},
            'volumes':volumes,'prototype_volumes':[],'flags':flags,'prototype_flags':[]}

def recover_prototype_storage(source, manifest, output):
    """Fill world prototype volumes and prototype flags after the world record exists."""
    world=manifest['world'];prototypes=world['prototypes']
    mechanisms={(m['prototype']['file'],m['prototype']['path_id'],m['recipe']['source']['file'],m['recipe']['source']['path_id']) for m in world['mechanisms']}
    def transform(owner,pointer):return source.transform_for(source.resolve(owner,pointer))
    volumes=[];evidence=[]
    for i in PROTOTYPE_VOLUMES:
        name=f'{PROTOTYPE_FILE}:{i}';r=source.get(PROTOTYPE_FILE,i)
        if r.type.name!='MonoBehaviour' or source.class_name(r)!='mountStuff':raise ValueError(f'{name} is not mountStuff')
        f=source.read(r);node=transform(r,f['m_GameObject'])
        col=source.resolve(r,f['col']) if f['col']['m_PathID'] else None
        if col is None or col.type.name!='BoxCollider':raise ValueError(f'{name} storage collider is not a BoxCollider')
        cf=source.read(col);col_node=transform(col,cf['m_GameObject'])
        usables=[source_id(source.resolve(r,p)) for p in f['susable'] if p['m_PathID']]
        if len(usables)!=len(f['susable']):raise ValueError(f'{name} has an empty usable')
        homes=containing(prototypes,source_id(node))
        # Upd needs activeInHierarchy below each prototype root. The stored root is inactive; the spawn activates it.
        # Scan reads only the collider box, so a disabled collider or inactive collider object still counts.
        roots=[source.get(p['file'],p['path_id']) for p,_ in homes]
        active=bool(f['m_Enabled']) and all(source.read(source.resolve(node,data['m_GameObject']))['m_IsActive']
                                            for root in roots for data in source.chain(node)[:-len(source.chain(root))])
        check_prototype_volume(name,f,active,homes,usables,mechanisms)
        for (prototype,index),root in zip(homes,roots):
            if not any(p==prototype for p,_ in containing(prototypes,source_id(col_node))):raise ValueError(f'{name} collider is outside {prototype}')
            chain=source.chain(col_node)[:-len(source.chain(root))]
            volumes.append({'prototype':prototype,'source':source_id(r),'node':index,**root_box(chain,cf),'storage_type':f['sType'],
                            'only_manual':bool(f['onlyManual']),'usables':usables,'dont_lock_if_owner_picked_up':bool(f['dontLockIfPickedUp'])})
        evidence.append({'source':source_id(r),'fields':f,'collider':cf})
    entries=[]
    for prototype in prototypes:
        _,parts=source.components(source.get(prototype['source']['file'],prototype['source']['path_id']))
        picks=[c for c in parts if c.type.name=='MonoBehaviour' and source.class_name(c)=='pickupable']
        if picks:entries.append((prototype['source'],source.read(picks[0])))
    flags=flag_entries(entries,'prototype')
    manifest['storage'].update(prototype_volumes=volumes,prototype_flags=flags)
    path=output/'storage-evidence.json';record=json.loads(path.read_text())
    record.update(prototype_volumes=evidence,prototype_flags=flags)
    path.write_text(json.dumps(record,indent=2)+'\n')
