"""Select original condition ranges and material variants once at export."""
import hashlib
from .source import key, source_id


def roll(identity, purpose, count):
    digest=hashlib.sha256(f'11140890:{identity}:{purpose}'.encode()).digest()
    return int.from_bytes(digest[:4],'little')%count


def condition_material(source, reader, data, group, main_reader, selection):
    state = selection['state']; material = None; color = None
    def selected(name):
        return source.resolve(main_reader, group[name]) if group else source.resolve(reader, data['def' + name])
    if data['UseMatList']:
        if not 0 <= state < len(data['MatList']):
            raise ValueError(f'{source_id(reader)} MatList state {state}')
        material = source.resolve(reader, data['MatList'][state])
        if data['forceColor'] and not data['disableColor']: color = selection['color']
    if not data['DontUseWorn']:
        material = selected(('New', 'Used', 'Middle', 'Old', 'Rusty')[state])
        if material and not data['disableColor']:
            if state < 3 or data['forceColor']: color = selection['color']
            else:
                rusty = selected('Rusty')
                if rusty is None: raise ValueError(f'{source_id(reader)} defRusty color missing')
                color = dict(source.read(rusty)['m_SavedProperties']['m_Colors'])['_Color']
    if material is None and not data['disableColor']:
        colors = group['colors'] if group else data.get('colors', [])
        if len(colors) > state: color = colors[state]
    return material, color


def prepare_conditions(source, roots):
    main_reader=source.get('level1',64919)
    materials={entry['tipus']:entry for entry in source.read(main_reader)['conditionmaterials']}
    readers={}
    for root in roots:
        for node in source.hierarchy(root,activate_root=True):
            if not node['active']:continue
            for reader in node['components']:
                if reader.type.name=='MonoBehaviour' and source.class_name(reader)=='partconditionscript':readers[key(reader)]=reader
    states={identity:{'state':source.read(reader)['state'],'range':None,'color':source.read(reader)['color']} for identity,reader in readers.items()}
    selected_types = {}
    for identity, reader in readers.items():
        data = source.read(reader)
        selected_types[identity] = data['tipus']
        if not data['useRandomTipus']: continue
        choices = data['randomTipusok']
        selection = data['slectedRandomTipus'] if data['setRandom'] else 0
        if not data['setRandom'] and roll(identity, 'type-chance', 1 << 24) / (1 << 24) <= data['chanceToRandom']:
            total = sum(choice['chance'] for choice in choices)
            threshold = roll(identity, 'type-choice', 1 << 24) / (1 << 24) * total
            accumulated = 0.
            for index, choice in enumerate(choices):
                accumulated += choice['chance']
                if threshold <= accumulated:
                    selection = index + 1; break
        if selection:
            selected_types[identity] = choices[selection - 1]['tipus']
            group = materials.get(selected_types[identity])
            if group and not group['colors'] and not data['dontSetRandomColorTo1']:
                states[identity]['color'] = dict(r=1., g=1., b=1., a=1.)
    def randomize(identity,minimum,maximum,color,visiting=None):
        visiting=set() if visiting is None else visiting
        if identity in visiting:raise ValueError('Part condition cycle')
        visiting.add(identity)
        reader=readers[identity];data=source.read(reader)
        if data['isChild']:
            visiting.remove(identity)
            return
        low=data['overRideMin'] if data['enableOverrideMin'] else minimum
        high=data['overRideMax'] if data['enableOverrideMax'] else maximum
        if not 0<=low<=high<=4:raise ValueError(f'Invalid condition range: {identity}')
        state=low+roll(identity,'state',high-low+1)
        states[identity]={'state':state,'range':[low,high],'color':color}
        for pointer in data['childs']:
            child=source.resolve(reader,pointer);child_id=key(child)
            if child_id not in readers:continue
            child_data=source.read(child)
            if child_data['noInheritState'] or child_data['loaded']:continue
            randomize(child_id,minimum,maximum,color,visiting)
        visiting.remove(identity)
    for identity,reader in readers.items():
        data=source.read(reader)
        if not data['randomStart'] or data['isChild']:continue
        group=materials.get(data['materialTipus'] if data['useOnlyMaterialTipusForMaterial'] else selected_types[identity]);colors=group['colors'] if group else data['colors']
        color=colors[roll(identity,'color',len(colors))] if colors else data['color']
        randomize(identity,data['randomMin'],data['randomMax'],color)
    # Refresh copies all three states to child parts before selecting source variants.
    variants = {identity: [data['state2'], data['state3']] for identity, reader in readers.items()
                for data in [source.read(reader)]}
    renderer_meshes = {}; active_nodes = {}
    def refresh_children(identity, visiting=None):
        visiting = set() if visiting is None else visiting
        if identity in visiting: raise ValueError(f'{identity} condition child cycle')
        visiting.add(identity)
        reader = readers[identity]; data = source.read(reader)
        for pointer in data['childs']:
            child = source.resolve(reader, pointer); child_id = key(child)
            if child_id not in readers: continue
            child_data = source.read(child)
            if (child_data['isChild'] and not data['noInheritState']) or child_data['sameStateAsParent']:
                states[child_id]['state'] = states[identity]['state']
                variants[child_id][0] = variants[identity][0]
                if not child_data['randomStartState3InFStart']: variants[child_id][1] = variants[identity][1]
            if child_data['isChildnewColor'] and not child_data['loaded']:
                group = materials.get(child_data['materialTipus'] if child_data['useOnlyMaterialTipusForMaterial'] else selected_types[child_id])
                colors = group['colors'] if group else child_data['colors']
                if colors: states[child_id]['color'] = colors[roll(child_id, 'child-color', len(colors))]
                elif selected_types[child_id] != child_data['tipus'] and not child_data['dontSetRandomColorTo1']:
                    states[child_id]['color'] = dict(r=1., g=1., b=1., a=1.)
            refresh_children(child_id, visiting)
        visiting.remove(identity)
    child_ids = {key(source.resolve(reader, pointer)) for reader in readers.values()
                 for pointer in source.read(reader)['childs']}
    for identity, reader in readers.items():
        data = source.read(reader)
        if data['randomStart'] and not data['isChild']:
            for index, name in enumerate(('MesList', 'GoList')):
                if data[name]: variants[identity][index] = roll(identity, name, len(data[name]))
        if data['randomStartState3InFStart'] and not data['loaded'] and not data['setRandom'] and data['GoList']:
            variants[identity][1] = roll(identity, 'GoList', len(data['GoList']))
    for identity in readers:
        if identity not in child_ids: refresh_children(identity)
    renderer_states={};renderer_materials={};renderer_colors={};evidence=[]
    for identity,reader in readers.items():
        data=source.read(reader);selection=states[identity];state=selection['state'];group=materials.get(data['materialTipus'] if data['useOnlyMaterialTipusForMaterial'] else selected_types[identity])
        material,color=condition_material(source,reader,data,group,main_reader,selection)
        state2, state3 = variants[identity]
        mesh = None
        if data['UseMesList'] and data['MesList']:
            if not 0 <= state2 < len(data['MesList']): raise ValueError(f'{source_id(reader)} MesList state2')
            mesh = source.resolve(reader, data['MesList'][state2])
        if data['UseGoList'] and data['GoList']:
            if not 0 <= state3 < len(data['GoList']): raise ValueError(f'{source_id(reader)} GoList state3')
            selected = source.resolve(reader, data['GoList'][state3])
            if selected:
                for pointer in data['GoList']:
                    game_object = source.resolve(reader, pointer)
                    if game_object: active_nodes[key(source.transform_for(game_object))] = key(game_object) == key(selected)
        for pointer in data['renderers']:
            renderer=source.resolve(reader,pointer)
            if renderer is None:raise ValueError(f'Condition renderer missing: {identity}')
            renderer_states[key(renderer)]={'source':source_id(reader),'state':state,'quality':1.-state/4.,'range':selection['range']}
            if mesh:renderer_meshes[key(renderer)]=mesh
            if material:renderer_materials[key(renderer)]=material
            if color:renderer_colors[key(renderer)]=color
        evidence.append({'source':source_id(reader),**selection,'material':source_id(material) if material else None,
            'state2':state2,'state3':state3,'type':selected_types[identity],
            'mesh':source_id(mesh) if mesh else None,'rule':'partconditionscript.FStart and Refresh; fixed build hash seed.'})
    source.condition_meshes=renderer_meshes
    source.condition_active=active_nodes
    source.conditions=states
    source.condition_renderers=renderer_states
    source.condition_materials=renderer_materials
    source.condition_colors=renderer_colors
    source.condition_evidence=evidence
