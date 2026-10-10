"""Recover selected part slots and pickup ownership from installed components."""
from .source import source_id, vector
from .gltf import convert_point


def recover_parts(source,nodes,components,item_nodes,node_readers,colliders,car_root,pointer_node,anchor,euler):
    def within(node,root):
        while node is not None:
            if node==root:return True
            node=nodes[node-1]['parent']
        return False
    selected={node for node,group in components.items() if nodes[node-1]['active'] and within(node,car_root)}
    part_nodes={node for node in selected if 'partscript' in components[node] and not {'wheelscript','gumiscript'}.intersection(components[node])}
    # Wheel slots and rim/tire assembly stay in the wheel module.
    wheel_covers={source.read(group['partscript'][0])['cover_CantCraftIfCovered']['m_PathID'] for node,group in components.items() if nodes[node-1]['active'] and 'wheelscript' in group}
    slot_nodes=selected|{node for node,group in components.items() if any(r.path_id in wheel_covers for r in group.get('partslotscript',[]))}
    slot_readers=[(node,r) for node in slot_nodes for r in components[node].get('partslotscript',[]) if not {'felni','gumi'}.intersection(source.read(r)['tipus'])]
    def parent_item(node):
        node=nodes[node-1]['parent']
        while node is not None:
            if node in item_nodes and ('partscript' in components[node]):return item_nodes[node]
            node=nodes[node-1]['parent']
        return None
    parts=[]
    for node in sorted(part_nodes):
        reader=components[node]['partscript'][0];v=source.read(reader)
        pickup=source.resolve(reader,v['p']);p=source.read(pickup)
        body=source.resolve(pickup,p['RB'])
        rb=source.read(body) if body else None
        owned=[]
        for c in colliders:
            if not within(c['node'],node):continue
            current=c['node']
            while current!=node and current not in part_nodes:current=nodes[current-1]['parent']
            if current==node:owned.append(c['id'])
        parts.append({'item':item_nodes[node],'source':source_id(reader),'node':node,
            'main_types':v['maintipus'],'part_types':v['tipus'],
            'usable':item_nodes[pointer_node(pickup,p['usable'])] if p['usable']['m_PathID'] else None,
            'cover_slot':v['cover_CantCraftIfCovered']['m_PathID'] or None,
            'local_rotation_after_mount':euler(vector(v['localRotationAfterMount'])),
            'colliders':owned,'linear_damping':rb['m_Drag'] if rb else p['RBs']['drag'],
            'angular_damping':rb['m_AngularDrag'] if rb else p['RBs']['angularDrag'],
            'enable_if_mounted':[pointer_node(reader,x) for x in v['enableIfMounted']],
            'enable_if_dismounted':[pointer_node(reader,x) for x in v['enableIfDismounted']],
            'disable_if_mounted':[pointer_node(reader,x) for x in v['disableIfMounted']],
            'disable_if_dismounted':[pointer_node(reader,x) for x in v['disableIfDismounted']]})
    slots=[]
    for node,r in sorted(slot_readers):
        v=source.read(r);part=source.resolve(r,v['part'])
        item=None
        # The pickup root can differ from part.T.
        if part:
            pickup=source.resolve(part,source.read(part)['p'])
            item=item_nodes[pointer_node(pickup,source.read(pickup)['m_GameObject'])]
        slots.append({'id':r.path_id,'source':source_id(r),'node':node,'owner':parent_item(node),
            'part':item,'mount_anchor':anchor(pointer_node(r,v['T'])),
            'detach_anchor':anchor(pointer_node(r,v['DT'])) if v['DT']['m_PathID'] else None,
            'fixed_detach':bool(v['fixDt']),'detach_position':list(convert_point(vector(v['VfixDt']))),
            'detach_rotation':euler(vector(v['VfixDtR'])),
            'main_types':v['maintipus'],'part_types':v['tipus'],
            'cant_craft':bool(v['cantCraft']),'cant_uncraft':bool(v['cantUnCraft']),
            'disable_colliders_on_craft':bool(v['disableCollidersOnCraft']),
            'colliders':[c['id'] for c in colliders if c['node']==node]})
    return parts,slots
