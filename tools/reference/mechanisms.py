"""Recover source rotation targets and window animation tracks."""
from .source import key, source_id, vector
from .gltf import convert_point


def recover_mechanisms(source, nodes, components, pointer_node, node_of, main):
    result = []
    for node, group in components.items():
        if not nodes[node-1]['active'] or 'tankcapscript' in group:
            continue
        for reader in group.get('usablescript', []):
            value = source.read(reader)
            if not (value['rotateAble'] or value['slideAble']):
                continue
            # Controls without a driven transform are tune/audio controls.
            rotation = None
            if value['rotateAble'] and value['RT1']['m_PathID']:
                targets = []
                same = value['RT1'] == value['RT2']
                for index in (1,) if same else (1,2):
                    target = pointer_node(reader,value[f'RT{index}'])
                    targets.append({'node':target,'mapping':[[bool(value[f'{axis}to{index}{c}']) for c in 'XYZ'] for axis in 'XY']})
                limits = [[value[f'min{a}'],value[f'max{a}']] for a in 'XY']
                if value['veryInvert']:
                    limits = [[-b,-a] for a,b in limits]
                rotation = {'targets':targets,'sum_axes':same,'limits_degrees':limits,
                    'initial_degrees':[value['xRot'],value['yRot']],
                    'inverted':[bool(value['invX']),bool(value['invY'])], 'sensitivity':main['rotSens']}
            slide = None
            if value['slideAble']:
                paths=[];rotations=[];animation=None
                if value['TDoor']['m_PathID']:
                    if value['useSlidePathInstead']:
                        points=[{'node':pointer_node(reader,p['T']),'value':p['value']} for p in value['slidePath']]
                    else:
                        points=[{'node':pointer_node(reader,value[field]),'value':i} for i,field in enumerate(('TstartPos','TendPos'))]
                    paths.append({'node':pointer_node(reader,value['TDoor']),'points':points,'rotate':bool(value['slideRotationToo'])})
                if value['anim']['m_PathID']:
                    anim=source.resolve(reader,value['anim']);animation=source_id(anim);fields=source.read(anim)
                    for path in fields['Anim']:
                        if len(path['points']) != len(path['time']):raise ValueError('Window animation times differ from points')
                        for obj in path['objs']:
                            paths.append({'node':pointer_node(anim,obj),'points':[{'node':pointer_node(anim,p),'value':t} for p,t in zip(path['points'],path['time'])],'rotate':True})
                    for track in fields['Rots']:
                        if track['time'] != [0.,1.]:raise ValueError('Selected handle time range changed')
                        for i,obj in enumerate(track['objs']):
                            rotations.append({'node':pointer_node(anim,obj),'reference':pointer_node(anim,track['def'][i]),
                                'axis':[-int(track['x'][i]),-int(track['y'][i]),int(track['z'][i])], 'angle_degrees':track['angle'][i]})
                if paths or rotations:
                    slide={'initial_value':value['slideValue'],'speed':value['slideSpeed'],'sensitivity':main['slideSens'],
                        'axes':[bool(value[f'slide{a}']) for a in ('X','Y','Z','Xi','Yi','Zi')],
                        'multiplier':[v if v else 1. for v in vector(value['slideMultiplier'])],
                        'scroll_without_use':bool(value['slideWithScrollWithoutUse']), 'animation_source':animation,
                        'paths':paths,'rotations':rotations}
            if rotation is None and slide is None:continue
            driven = list(dict.fromkeys([t['node'] for t in rotation['targets']] if rotation else []))
            if slide:driven=list(dict.fromkeys(driven+[p['node'] for p in slide['paths']]+[r['node'] for r in slide['rotations']]))
            result.append({'id':len(result)+1,'source':source_id(reader),'use_node':node,'driven_nodes':driven,
                'enabled':bool(value['m_Enabled']) and not value['disable'],
                'child_source':key(source.resolve(reader,value['childGetValue'])) if value['childGetValue']['m_PathID'] else None,
                'invert_child':bool(value['invertChildValue']),'invert_value':bool(value['veryInvert']) != bool(value['onlyInvertDoorValue']),
                'drag':{'mode':'relative' if value['relativeRot'] else 'relative_two' if value['relativeRot2'] else 'direct',
                    'node':pointer_node(reader,value['MouseT']) if value['MouseT']['m_PathID'] else node,
                    'node2':pointer_node(reader,value['MouseT2']) if value['MouseT2']['m_PathID'] else node,
                    'axis':list(convert_point(vector(value['MouseAxis']))),
                    'axes2':[list(convert_point(vector(value[f'MouseAxis2{a}']))) for a in 'XY']},
                'rotation':rotation,'slide':slide})
    return result


def bind_mechanisms(source, mechanisms, nodes, components, items, item_nodes):
    use_items={(m['source']['file'],m['source']['path_id']):item_nodes[m['use_node']] for m in mechanisms}
    pickups={}
    mounted=set()
    for node,group in components.items():
        for slot in group.get('partslotscript',[]):
            part=source.resolve(slot,source.read(slot)['part'])
            if part:
                pick=source.resolve(part,source.read(part)['p'])
                if pick:mounted.add(key(pick))
        for pick in group.get('pickupable',[]):
            fields=source.read(pick)
            if fields['usable']['m_PathID']:
                pickups[key(source.resolve(pick,fields['usable']))]=(pick,fields)
    for m in mechanisms:
        node=m.pop('use_node');m['item']=item_nodes[node]
        # Keep the nearest containing pickup, including the control's own part.
        parent=node
        while parent and 'pickupable' not in components[parent]:parent=nodes[parent-1]['parent']
        m['parent_item']=item_nodes.get(parent)
        child=m.pop('child_source')
        m['child']=use_items[child] if child else None
        m['disable_on_craft']=False;m['disable_on_uncraft']=False
        linked=pickups.get((m['source']['file'],m['source']['path_id']))
        if linked:
            pick,fields=linked
            m['disable_on_craft']=bool(fields['disableOnCraft'])
            m['disable_on_uncraft']=bool(fields['disableOnUnCraft']) and not fields['DONTdisableUsableOnUnCraft']
            if key(pick) in mounted:
                if m['disable_on_craft']:m['enabled']=False
                if m['disable_on_uncraft']:m['enabled']=True
            else:
                if m['disable_on_craft']:m['enabled']=True
                if m['disable_on_uncraft']:m['enabled']=False
