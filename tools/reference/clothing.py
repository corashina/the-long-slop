"""Checked wearable templates, source anchors, and complete selected variants."""
from .source import key, source_id
from .gltf import converted_transform
from .survival import managed_evidence, finite_record

FAMILIES=((146739,70812,217,0),(147668,53838,218,1),(147667,53837,221,2),
          (131348,63605,245,1),(148817,63259,92,3),(142834,54495,230,4))
ANCHORS=(26477,28766,35477,36615,39162)
ANCESTRY=(39843,39851,39811,22249,39820,25362)
RANDOM_SELECTORS={135226:(70812,[10]*5),157851:(57452,[25,10,10,10,10,10]),154182:(49311,[25,10,10,10,10,10]),149977:(63724,[10]*5)}
MATERIAL_SELECTORS=(132932,150967)


def recover_clothing(source,manifest,output):
    from .postprocessing import recover_glasses_profiles
    evidence=managed_evidence(source,['w.sunglass = this;', 'mainscript.M.disableSunFlare = disableLensFlare;',
        'mainscript.M.PPVSunGlasses.profile = PP;', 'tipusok[i].objects[j].SetActive(value: false);',
        'tipusok[rtipus].objects[k].SetActive(value: true);'])
    nodes={key(source.get(n['source']['file'],n['source']['path_id'])):n for n in manifest['nodes']}
    nodes_by_id={n['id']:n for n in manifest['nodes']}
    def node(reader):
        if reader.type.name=='GameObject':reader=source.transform_for(reader)
        elif reader.type.name not in ('Transform','RectTransform'):reader=source.transform_for(source.resolve(reader,source.read(reader)['m_GameObject']))
        return nodes[key(reader)]['id']
    def links(reader,values):return [node(source.resolve(reader,p)) for p in values]
    result={'wearable_type_count':5,'player_root':{'file':'level1','path_id':39843},'anchor_nodes':[],
        'anchors':[{'file':'level1','path_id':p} for p in ANCHORS],'wearables':[],
        'glasses_profiles':recover_glasses_profiles(source,manifest,output),'evidence':evidence}
    player=source.get('level1',80125)
    root=source.transform_for(source.resolve(player,source.read(player)['m_GameObject']))
    if key(root)!=('level1',39843):raise ValueError('Player Transform root differs')
    for pid in (*ANCESTRY,*ANCHORS):
        reader=source.get('level1',pid);d=source.read(reader);parent=source.resolve(reader,d['m_Father'])
        result['anchor_nodes'].append({'source':source_id(reader),'parent':source_id(parent) if parent else None,
            'coordinate_space':'source_parent_local','transform':converted_transform(d)})
    for pid,root_id,item_id,type_index in FAMILIES:
        reader=source.get('sharedassets1.assets',pid);d=source.read(reader)
        prototype=next(p for p in manifest['item_prototypes'] if p['id']=={'file':'sharedassets1.assets','path_id':root_id})
        if d['tipus']!=type_index or d['sunglass']['m_PathID']:raise ValueError('Wearable source type or startup differs')
        local_colliders={(c['source']['file'],c['source']['path_id']):c['id'] for c in prototype['colliders']}
        root=source.get('sharedassets1.assets',root_id)
        records=source.hierarchy(root,activate_root=True)
        selectors=[];lenses=[]
        for record in records:
            for c in record['components']:
                if c.type.name!='MonoBehaviour':continue
                name=source.class_name(c);v=source.read(c)
                if name=='szbglasscript':
                    pp=source.resolve(c,v['PP'])
                    if key(source.resolve(c,v['w']))!=key(reader):raise ValueError('Foreign glasses startup owner')
                    lenses.append({'source':source_id(c),'node':node(c),'wearable':source_id(reader),
                        'profile':next((i for i,p in enumerate(result['glasses_profiles']) if p['source']==source_id(pp)),None) if pp else None,
                        'disable_lens_flare':bool(v['disableLensFlare'])})
        for record in records:
            for c in record['components']:
                if c.type.name!='MonoBehaviour':continue
                name=source.class_name(c);v=source.read(c)
                if name not in ('randomTypeSelector','partconditionscript'):continue
                selection={'source':source_id(c),'node':node(c),'kind':'random' if name=='randomTypeSelector' else 'material',
                    'authored_index':v['rtipus'] if name=='randomTypeSelector' else v['state'],
                    'random_start':bool(v['forceStart'] if name=='randomTypeSelector' else v['randomStart']),
                    'save_index':v['inTosaveId'] if name=='randomTypeSelector' else None,
                    'children':[source_id(source.resolve(c,p)) for p in v['childs']],
                    'material_rules':None,'variants':[]}
                if name=='partconditionscript':
                    if not v['UseMatList'] or v['UseGoList'] or v['UseMesList'] or v['useRandomTipus']:raise ValueError('Unsupported wearable material selector')
                    if any(v[k] for k in ('isChildnewColor','forceColor','dontSetRandomColorTo1','useOnlyMaterialTipusForMaterial','randomStartState3InFStart','setRandom','started','refresh','chanceToRandom','repairValue','RepairProperty','state2','state3','slectedRandomTipus','randomChildsSet','randomTipusok','colors')):raise ValueError('Unaudited material selector branch')
                    if any(v[k]['m_PathID'] for k in ('defMiddle','defNew','defOld','defRusty','defUsed','mMiddle','mNew','mOld','mRusty','mUsed','tosave')):raise ValueError('Unaudited material selector resource')
                    selection['material_rules']={k:(v[k] if k in ('randomMin','randomMax','overRideMin','overRideMax') else bool(v[k])) for k in ('randomMin','randomMax','isChild','noInheritState','sameStateAsParent','DontUseWorn','disableColor','useMatListDontCreateNewMaterial','enableOverrideMin','enableOverrideMax','overRideMin','overRideMax','loaded')}
                    choices=[{'objects':[],'Move':[],'Target':[],'partslots':[],'parts':[],'tank':{'m_PathID':0,'m_FileID':0},
                        'fluid':{'all':0.,'f':0.,'fluids':[],'maxC':0.},'R':v['renderers'],'M':p,'chance':None} for p in v['MatList']]
                else:choices=v['tipusok']
                for choice in choices:
                    selected=links(c,choice['objects']);material=source.resolve(c,choice['M']);tank=source.resolve(c,choice['tank'])
                    # These selected source payloads contain no transforms, mounts, or fluids.
                    if choice['Move'] or choice['Target'] or choice['partslots'] or choice['parts'] or tank or choice['fluid']['fluids'] or any(choice['fluid'][f] for f in ('all','f','maxC')):raise ValueError('Unaudited wearable variant payload')
                    candidates=[l for l in lenses if any(descendant(nodes_by_id, l['node'], n) for n in selected)]
                    lens=candidates[0] if len(candidates)==1 else None
                    selection['variants'].append({'nodes':selected,'weight':choice['chance'],'material':source_id(material) if material else None,
                        'renderers':[{'node':n,'slot':0} for n in links(c,choice['R'])],
                        'moves':[],'targets':[],'part_slots':[],'parts':[],'tank':None,'fluid':{'all':0.,'f':0.,'fluids':[],'max_c':0.},
                        'lens':lens['source'] if lens else None,'profile':lens['profile'] if lens else None,
                        'disable_lens_flare':lens['disable_lens_flare'] if lens else None})
                selectors.append(selection)
        center=source.resolve(reader,d['CenterObj'])
        result['wearables'].append({'source':source_id(reader),'prototype':prototype['id'],'type_index':type_index,
            'colliders':[local_colliders[key(source.resolve(reader,p))] for p in d['c']],
            'renderers':links(reader,d['Renderers']),'use_center_object':bool(d['useCenterObj']),
            'center':node(center) if center else None,'selectors':selectors,'lenses':lenses})
    return result


def validate_clothing(clothing,manifest):
    from .postprocessing import validate_glasses_profiles
    finite_record(clothing)
    sid=lambda pid,file='sharedassets1.assets':{'file':file,'path_id':pid}
    if clothing['wearable_type_count']!=5 or clothing['player_root']!=sid(39843,'level1') or clothing['anchors']!=[sid(p,'level1') for p in ANCHORS]:raise ValueError('Wearable anchor identities')
    anchor_nodes=clothing['anchor_nodes'];expected=(*ANCESTRY,*ANCHORS)
    if [n['source'] for n in anchor_nodes]!=[sid(p,'level1') for p in expected]:raise ValueError('Anchor ancestry identities')
    for n,pid in zip(anchor_nodes,expected,strict=True):
        parent=None if pid==39843 else sid(ANCESTRY[ANCESTRY.index(pid)-1] if pid in ANCESTRY else 25362,'level1')
        t=n['transform']
        if n['parent']!=parent or n['coordinate_space']!='source_parent_local' or len(t['translation'])!=3 or len(t['scale'])!=3 or any(v==0 for v in t['scale']) or len(t['rotation'])!=4 or abs(sum(v*v for v in t['rotation'])-1)>.002:raise ValueError('Anchor ancestry pose')
    validate_glasses_profiles(clothing['glasses_profiles'],manifest)
    if len(clothing['wearables'])!=6:raise ValueError('Selected wearable count')
    nodes={n['id']:n for n in manifest['nodes']}
    for w,(pid,root,item,type_index) in zip(clothing['wearables'],FAMILIES,strict=True):
        p=next((p for p in manifest['item_prototypes'] if p['id']==w['prototype']),None)
        if w['source']!=sid(pid) or w['prototype']!=sid(root) or w['type_index']!=type_index or p is None or p['item_id']!=item:raise ValueError('Wearable prototype identity')
        if [next((c['source']['path_id'] for c in p['colliders'] if c['id']==i),None) for i in w['colliders']]!=WEARABLE_COLLIDERS[pid]:raise ValueError('Wearable collider links')
        if [nodes.get(i,{}).get('source') for i in w['renderers']]!=[sid(i) for i in WEARABLE_RENDERERS[pid]]:raise ValueError('Wearable source renderers')
        renderers={r['node']:r for r in p['renderers']}
        if len(set(w['renderers']))!=len(w['renderers']) or any(n not in renderers for n in w['renderers']):raise ValueError('Wearable renderer links')
        expected_center={131348:69634,142834:56826}.get(pid)
        if w['use_center_object']!=(expected_center is not None) or (nodes.get(w['center'],{}).get('source') if w['center'] is not None else None)!=(sid(expected_center) if expected_center else None):raise ValueError('Wearable center')
        def local(n):return nodes.get(n,{}).get('asset')==p['asset']
        if pid!=146739:
            if w['selectors'] or w['lenses']:raise ValueError('Unexpected wearable selectors')
            continue
        if [s['source']['path_id'] for s in w['selectors']]!=[135226, 132932, 157851, 154182, 149977, 150967]:raise ValueError('Complete wearable selectors')
        if len(w['lenses'])!=19 or len({l['source']['path_id'] for l in w['lenses']})!=19:raise ValueError('Complete glasses startup lenses')
        for lens in w['lenses']:
            if lens['source']['file']!='sharedassets1.assets' or not local(lens['node']) or lens['node'] not in renderers or lens['wearable']!=w['source'] or lens['profile'] not in range(6):raise ValueError('Glasses startup links')
        validate_selected_glasses(w,manifest,p)
        for s in w['selectors']:
            if not local(s['node']) or s['source']['file']!='sharedassets1.assets' or s['authored_index']!=0 or not s['random_start']:raise ValueError('Wearable selector owner')
            source=s['source']['path_id']
            if source in RANDOM_SELECTORS:
                root_id,weights=RANDOM_SELECTORS[source]
                if s['kind']!='random' or nodes[s['node']]['source']!=sid(root_id) or [v['weight'] for v in s['variants']]!=weights or not s['random_start'] or s['save_index']!=0 or s['children'] or s['material_rules'] is not None:raise ValueError('Wearable random selector')
            elif nodes[s['node']]['source']!=sid(70812 if source==132932 else 85646) or s['kind']!='material' or len(s['variants'])!=3 or s['save_index'] is not None or s['material_rules'] is None:raise ValueError('Wearable material selector')
            for child in s['children']:
                if not any(t['source']==child for t in w['selectors']):raise ValueError('Selector child')
            for v in s['variants']:
                if any(not local(n) for n in v['nodes']) or len(set(v['nodes']))!=len(v['nodes']):raise ValueError('Variant nodes')
                if v['moves'] or v['targets'] or v['part_slots'] or v['parts'] or v['tank'] or v['fluid']!={'all':0.,'f':0.,'fluids':[],'max_c':0.}:raise ValueError('Variant selection payload')
                for r in v['renderers']:
                    if r['node'] not in renderers or not 0<=r['slot']<len(renderers[r['node']]['materials']):raise ValueError('Variant renderer slot')
                if v['material'] and not any(m['asset']==p['asset'] and m['source']==v['material'] for m in manifest['rendering']['materials']):raise ValueError('Variant material dependency')
                lens=next((l for l in w['lenses'] if l['source']==v['lens']),None)
                candidates=[l for l in w['lenses'] if any(descendant(nodes,l['node'],n) for n in v['nodes'])]
                expected=candidates[0]['source'] if len(candidates)==1 else None
                if v['lens']!=expected:raise ValueError('Missing effective variant lens')
                if v['lens'] is None:
                    if v['profile'] is not None or v['disable_lens_flare'] is not None:raise ValueError('Variant startup profile')
                elif lens is None or not any(descendant(nodes,lens['node'],n) for n in v['nodes']) or lens['profile']!=v['profile'] or lens['disable_lens_flare']!=v['disable_lens_flare']:raise ValueError('Variant effective lens')

GLASSES_BRANCHES={135226: [[57452], [49311], [69685], [63724], [74215]], 157851: [[63552], [66918], [82244], [73036], [66333], [56341]], 154182: [[66684], [87658], [79878], [83500], [74142], [77647]], 149977: [[58013], [55142], [57753], [81212], [83913]]}
GLASSES_LENSES=[(154009, 63552, 0, True), (135972, 66918, 4, True), (150488, 82244, 2, True), (143500, 73036, 1, True), (156901, 66333, 5, True), (151834, 56341, 3, True), (140782, 66684, 0, True), (141256, 87658, 4, True), (153056, 79878, 2, True), (147364, 83500, 1, True), (147011, 74142, 5, True), (136426, 77647, 3, True), (157020, 86499, 0, True), (155894, 58013, 4, False), (152881, 55142, 2, False), (140470, 57753, 1, False), (131005, 81212, 5, False), (144723, 83913, 3, False), (151873, 87099, 4, False)]

def validate_selected_glasses(w,manifest,p):
    nodes={n['id']:n['source']['path_id'] for n in manifest['nodes'] if n['asset']==p['asset']}
    for s in w['selectors']:
        pid=s['source']['path_id']
        if pid in GLASSES_BRANCHES:
            if [[nodes.get(n) for n in v['nodes']] for v in s['variants']]!=GLASSES_BRANCHES[pid] or any(v['material'] or v['renderers'] for v in s['variants']):raise ValueError('Exact selected variant nodes')
        else:
            if s['material_rules']!=MATERIAL_RULES[pid] or s['children']!=([{'file':'sharedassets1.assets','path_id':150967}] if pid==132932 else []):raise ValueError('Exact material selector rules')
            expected_renderers=MATERIAL_RENDERERS[pid]
            if [v['material'] for v in s['variants']]!=[{'file':file,'path_id':pid} for file,pid in [('sharedassets1.assets',335),('sharedassets1.assets',253),('sharedassets0.assets',89)]]:raise ValueError('Exact selected material variants')
            for v in s['variants']:
                if v['nodes'] or v['weight'] is not None or [(nodes.get(r['node']),r['slot']) for r in v['renderers']]!=[(n,0) for n in expected_renderers]:raise ValueError('Exact selected material slots')
    if [(l['source']['path_id'],nodes.get(l['node']),l['profile'],l['disable_lens_flare']) for l in w['lenses']]!=GLASSES_LENSES:raise ValueError('Exact selected startup lenses')

MATERIAL_RENDERERS={132932: [81689, 60387, 83695], 150967: [70739]}

WEARABLE_RENDERERS={146739: [81689, 70739, 63552, 66918, 82244, 73036, 66333, 56341, 60387, 66684, 87658, 79878, 83500, 74142, 77647, 80742, 86499, 53868, 58013, 55142, 57753, 81212, 83913, 83695, 87099], 147668: [86632, 88813], 147667: [80368], 131348: [75397], 148817: [84062], 142834: []}
WEARABLE_COLLIDERS={146739: [112724, 113317, 121089], 147668: [117484, 117486], 147667: [117485], 131348: [112577, 119446, 118285, 113889, 112407, 110965, 119440, 111925, 121241, 116901, 114763, 114256], 148817: [111200, 116405, 124754], 142834: [112435]}


def descendant(nodes,node,root):
    for _ in range(len(nodes)):
        if node==root:return True
        node=nodes.get(node,{}).get('parent')
        if node is None:return False
    return False

MATERIAL_RULES={132932: {'DontUseWorn': 1, 'disableColor': 1, 'enableOverrideMax': 0, 'enableOverrideMin': 0, 'isChild': 0, 'loaded': 0, 'noInheritState': 0, 'overRideMax': 0, 'overRideMin': 0, 'randomMax': 2, 'randomMin': 0, 'sameStateAsParent': 0, 'useMatListDontCreateNewMaterial': 1}, 150967: {'DontUseWorn': 1, 'disableColor': 1, 'enableOverrideMax': 0, 'enableOverrideMin': 0, 'isChild': 1, 'loaded': 0, 'noInheritState': 0, 'overRideMax': 0, 'overRideMin': 0, 'randomMax': 2, 'randomMin': 0, 'sameStateAsParent': 0, 'useMatListDontCreateNewMaterial': 1}}
