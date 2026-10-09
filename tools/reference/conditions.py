"""Select original condition ranges and material variants once at export."""
import hashlib
from .source import key, source_id


def roll(identity, purpose, count):
    digest=hashlib.sha256(f'11140890:{identity}:{purpose}'.encode()).digest()
    return int.from_bytes(digest[:4],'little')%count


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
    def kind(data):return data['materialTipus'] if data['useOnlyMaterialTipusForMaterial'] else data['tipus']
    def randomize(identity,minimum,maximum,color,parent_state=None,visiting=None):
        visiting=set() if visiting is None else visiting
        if identity in visiting:raise ValueError('Part condition cycle')
        visiting.add(identity)
        reader=readers[identity];data=source.read(reader)
        low=data['overRideMin'] if data['enableOverrideMin'] else minimum
        high=data['overRideMax'] if data['enableOverrideMax'] else maximum
        if not 0<=low<=high<=4:raise ValueError(f'Invalid condition range: {identity}')
        state=parent_state if parent_state is not None else low+roll(identity,'state',high-low+1)
        states[identity]={'state':state,'range':[low,high],'color':color}
        for pointer in data['childs']:
            child=source.resolve(reader,pointer);child_id=key(child)
            if child_id not in readers:continue
            child_data=source.read(child)
            if child_data['noInheritState'] or child_data['loaded']:continue
            randomize(child_id,minimum,maximum,color,state if child_data['isChild'] else None,visiting)
        visiting.remove(identity)
    for identity,reader in readers.items():
        data=source.read(reader)
        if not data['randomStart'] or data['isChild']:continue
        group=materials.get(kind(data));colors=group['colors'] if group else data['colors']
        color=colors[roll(identity,'color',len(colors))] if colors else data['color']
        randomize(identity,data['randomMin'],data['randomMax'],color)
    renderer_states={};renderer_materials={};renderer_colors={};evidence=[]
    for identity,reader in readers.items():
        data=source.read(reader);selection=states[identity];state=selection['state'];group=materials.get(kind(data))
        material=None;color=None
        if not data['DontUseWorn']:
            if group:material=source.resolve(main_reader,group[('New','Used','Middle','Old','Rusty')[state]])
            else:material=source.resolve(reader,data[('defNew','defUsed','defMiddle','defOld','defRusty')[state]])
            if material and not data['disableColor']:
                if state<3 or data['forceColor']:color=selection['color']
        elif data['UseMatList']:
            material=source.resolve(reader,data['MatList'][state])
            if data['forceColor'] and not data['disableColor']:color=selection['color']
        if not material and not data['disableColor']:
            colors=group['colors'] if group else data['colors']
            if len(colors)>state:color=colors[state]
        for pointer in data['renderers']:
            renderer=source.resolve(reader,pointer)
            if renderer is None:raise ValueError(f'Condition renderer missing: {identity}')
            renderer_states[key(renderer)]={'source':source_id(reader),'state':state,'quality':1.-state/4.,'range':selection['range']}
            if material:renderer_materials[key(renderer)]=material
            if color:renderer_colors[key(renderer)]=color
        evidence.append({'source':source_id(reader),**selection,'material':source_id(material) if material else None,
            'differences':'Source mesh and object variants retain selected serialized geometry. Source shader maps to glTF PBR.'})
    source.conditions=states
    source.condition_renderers=renderer_states
    source.condition_materials=renderer_materials
    source.condition_colors=renderer_colors
    source.condition_evidence=evidence
