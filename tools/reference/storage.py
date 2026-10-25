"""Checked source mountStuff storage volumes, policy, and item flags."""
import json
import re
from .source import key, source_id, vector
from .gltf import convert_point
from .managed_code import managed_code, class_code, method, require

VOLUMES=(65838,65854,67560,79410,77806,78464,79301,69971)
OWNER=73405
TYPES=(0,0,5,5,5,5,3,4)
SLOTS=((),(),(),(),(),(),(77859,),(77842,))
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

def flag_entries(entries):
    """Keep items whose pickupable sets dontMount or dontMountTillPickup."""
    return [{'item':item,'dont_mount':bool(f['dontMount']),'dont_mount_till_pickup':bool(f['dontMountTillPickup'])}
            for item,f in entries if f['dontMount'] or f['dontMountTillPickup']]

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
            'volumes':volumes,'prototype_volumes':[],'flags':flags}
