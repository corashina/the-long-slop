"""Checked acquisition inputs. Runtime draws and publication belong to gameplay."""
import hashlib
import re
from .source import key, source_id
from .gltf import converted_transform
from .survival import component, finite_record, managed_evidence
from .stations import STATIONS

ASSEMBLY_SHA256 = '9919a9eeddcce23bb4ff86e890437f32b4a3ef3c10af2efe3b8d889d894dd62e'
TEXT_SHA256 = '22ddc62b91954205d90ff513232f39becebbc58a71b718685075f91f64b1f3ee'


def _block(code, start):
    opening = code.index('{', start)
    depth, end = 1, opening + 1
    while depth:
        depth += (code[end] == '{') - (code[end] == '}')
        end += 1
    return code[start:end]


def _database(code):
    database = _block(code, code.index('public class itemdatabase :'))
    enums = {}
    for name in ('i', 'k'):
        match = re.search(r'public enum ' + name + r'\s*\{([^}]+)\}', database)
        entries = [v.strip() for v in match[1].split(',') if v.strip()]
        if any(not re.fullmatch(r'\w+', v) for v in entries):
            raise ValueError('Acquisition enum requires explicit recovery')
        enums[name] = {v: i for i, v in enumerate(entries)}
    categories = {}
    for match in re.finditer(r'categories\[(\d+)\] = new category\((.*?)\);', database, re.S):
        arrays = re.findall(r'new int\[\d+\]\s*\{([^}]+)\}', match[2])
        if len(arrays) not in (1, 2) or 'kategoria: ' not in match[2]:
            raise ValueError('Acquisition category constructor requires explicit recovery')
        values = [[int(v.strip()) for v in a.split(',') if v.strip()] for a in arrays]
        is_category = 'kategoria: true' in match[2]
        supplied = values[1] if len(values) == 2 else None
        categories[int(match[1])] = {
            'id': int(match[1]), 'children': values[0], 'categories': is_category,
            'weights': supplied if is_category and supplied is not None else [1] * len(values[0]),
            'constructor_weights': supplied, 'depend_on_car': 'pDependOnCar: true' in match[2],
            'constructor_sha256': hashlib.sha256(match[0].encode()).hexdigest(),
        }
    return enums, categories


def recover_station_graphs(source, manifest, code):
    enums, all_categories = _database(code)
    nodes = {(n['source']['file'], n['source']['path_id']): n for n in manifest['nodes']}
    prototypes = {p['item_id']: p['id'] for p in manifest['item_prototypes'] if p['item_id'] is not None}
    prototype_roots = {(p['id']['file'], p['id']['path_id']): p for p in manifest['item_prototypes']}
    selected_categories = set()

    def marker(reader):
        if reader.type.name == 'GameObject':
            reader = source.transform_for(reader)
        elif reader.type.name not in ('Transform', 'RectTransform'):
            reader = source.transform_for(source.resolve(reader, source.read(reader)['m_GameObject']))
        n = nodes[key(reader)]
        return {'source': source_id(reader), 'node': n['id'], 'transform': converted_transform(source.read(reader))}

    def selection(owner, name, is_item, prefab=None):
        if prefab is not None:
            transform = source.transform_for(prefab)
            accepted = prototype_roots.get(key(transform))
            saves = [c for c in source.components(transform)[1]
                     if c.type.name == 'MonoBehaviour' and source.class_name(c) == 'tosaveitemscript']
            item_id = source.read(saves[0])['id'] if len(saves) == 1 else None
            return {'kind': 'prefab', 'name': name, 'source': source_id(prefab),
                    'item_id': item_id, 'prototype': accepted['id'] if accepted else None}
        if is_item:
            item_id = enums['i'][name]
            return {'kind': 'item', 'name': name, 'item_id': item_id, 'prototype': prototypes.get(item_id)}
        category_id = enums['k'][name]
        selected_categories.add(category_id)
        return {'kind': 'category', 'name': name, 'category_id': category_id}

    def table(reader, values):
        result, threshold = [], 0.
        for entry in values:
            threshold += entry['chance']
            result.append({'selection': selection(reader, entry['category'], entry['itemNameInsteadOfCategory']),
                           'weight': entry['chance'], 'threshold': threshold})
        return result

    stations = []
    main = source.get('level1', 64919)
    settings = source.get('level1', 65886)
    sd = source.read(settings)
    distance = {'source': source_id(settings), 'draw_distance': [sd['DDSmin']['drawDistance'], sd['DDSmax']['drawDistance']],
                'item_distance': [sd['DDSmin']['ItemSpawnDist'], sd['DDSmax']['ItemSpawnDist']],
                'authored_draw_distance': sd['S']['FDrawDistance']}
    for name, (_, _, go_id) in STATIONS.items():
        go = source.get('sharedassets1.assets', go_id)
        root = source.transform_for(go)
        building = component(source, root, 'buildingscript')
        bd = source.read(building)
        station = next(s for s in manifest['stations']['types'] if s['source'] == source_id(go))
        ordinary, underground = [], []
        for record in source.hierarchy(root, activate_root=True):
            for reader in record['components']:
                if reader.type.name != 'MonoBehaviour':
                    continue
                kind = source.class_name(reader)
                if kind not in ('newRandomStuffSpawnScript', 'undergroundSpawnScript'):
                    continue
                d = source.read(reader)
                if kind == 'undergroundSpawnScript':
                    underground.append({'source': source_id(reader), 'center': marker(source.resolve(reader, d['center'])),
                        'chance': d['chance'], 'count': [d['minCount'], d['maxCount']],
                        'radius': [d['minRadius'], d['maxRadius']], 'vertical': [d['minOffsetty'], d['maxOffsetty']],
                        'table': table(reader, d['spawn'])})
                    continue
                entries, threshold = [], 0
                for item in d['items']:
                    threshold += item['chance'] or 1
                    places, place_threshold = [], 0
                    for place in item['places']:
                        place_threshold += place['chance'] or 1
                        target = source.resolve(reader, place['T'])
                        places.append({'weight': place['chance'], 'threshold': place_threshold,
                                       'marker': marker(target) if target else None})
                    entries.append({'selection': selection(reader, item['prefab'], item['isitem'], source.resolve(reader, item['G'])),
                        'weight': item['chance'], 'threshold': threshold, 'places': places,
                        'random_rotation': bool(item['randomRot']), 'disable_random': bool(item['disableRandom']),
                        'initialization': {'integer_range': [item['ir1'], item['ir2']], 'float_range': [item['fr1'], item['fr2']],
                            'override_fluid_amount': bool(item['overrideFluidAmount']), 'fluid_range': [item['minFluid'], item['maxFluid']],
                            'override_fluid_type': bool(item['overrideFluidType']), 'fluids': [
                                {'type': f['type'], 'weight': f['chance'], 'authored_threshold': f['chance2']} for f in item['fluids']],
                            'no_mixed_fluids': bool(item['noMixedFluids']), 'override_mix_chance': bool(item['overrideMixChance']),
                            'mix_chance': item['mixChance']}})
                ordinary.append({'source': source_id(reader), 'marker': marker(record['reader']),
                    'dont_spawn': bool(d['dontSpawn']), 'holiday': d['holiday'],
                    'building_affects_this': bool(d['buildingAffectsThis']), 'chance': d['chanceToSpawn'],
                    'spawned': bool(d['spawned']), 'entries': entries})
        stations.append({'source': source_id(go), 'building': source_id(building), 'asset': station['asset'],
            'root': station['root'], 'item_spawn_distance': distance,
            'building_chance': [bd['minRandomSpawn'], bd['maxRandomSpawn']],
            'initial_items_spawned': bool(bd['itemsSpawned']),
            'flyers': {'force':bool(bd['forceRandomFlyer']), 'halloween':bool(bd['randomFlyerOnHalloween']),
                'vertical_offset':bd['randomFlyerOffsetty'], 'vertical_scale':bd['randomFlyerVertical'],
                'horizontal':[bd['randomFlyerHorizontalMin'],bd['randomFlyerHorizontalMax']],
                'prefab':source_id(source.resolve(main,source.read(main)['randomFlyerPrefab'])), 'supported':False},
            'to_start': [source_id(source.resolve(building, p)) for p in bd['toStart']],
            'ordinary': ordinary, 'underground': underground,
            'misc4': [s['source'] for s in ordinary if any(i['selection'].get('category_id') == 35 for i in s['entries'])]})
    box = source.get('sharedassets1.assets', 158204)
    box_table = table(box, source.read(box)['spawn'])
    def include(cid):
        if cid not in all_categories:
            raise ValueError(f'Missing checked category {cid}')
        if cid in closure:
            return
        closure.add(cid)
        if all_categories[cid]['categories']:
            for child in all_categories[cid]['children']:
                if child not in all_categories:
                    expected = {12: (186,265,303,387), 13: (187,266,304,386)}
                    if child not in expected.get(cid, ()):
                        raise ValueError(f'Unreviewed native invalid category {cid}:{child}')
                else:
                    include(child)
    closure = set()
    for cid in sorted(selected_categories):
        include(cid)
    categories = [all_categories[cid] for cid in sorted(closure)]
    for c in categories:
        c['prototypes'] = [None if c['categories'] else prototypes.get(i) for i in c['children']]
        c['invalid_children'] = [{'ordinal':i, 'target':child, 'failure':'category_index_out_of_range'}
            for i,child in enumerate(c['children']) if c['categories'] and child not in all_categories]
    return stations, categories, box_table

METHODS = {
    'itemdatabase': ['public class category', 'private void Awake()'],
    'buildingscript': ['public void CheckDist()', 'public void SpawnStuff(int pId)', 'private IEnumerator SPAWN()', 'public static void RandomFlyerSpawn(bool _forceRandomFlyer, bool _randomFlyerOnHalloween, Vector3 _pos, float _offsetty, float _vertical, float _horizontalMin, float _horizontalMax)'],
    'newRandomStuffSpawnScript': ['public void Spawn()', 'public void InitSpawn()', 'public void SelectSpawn()', 'public void SpawnItem()', 'public static int SelectItem(string _categoryName, bool _isOneItem)', 'public static int selectId(int categoryId)'],
    'undergroundSpawnScript': ['private void Init()', 'public void Spawn()', 'public Vector3 RandomPoint()'],
    'spawnInBoxScript': ['public void FStart()', 'public void Spawn()', 'public void Opened()'],
    'breakablescript': ['private void Start()', 'private void OnCollisionEnter(Collision c)', 'public void TryBreak(float force)', 'public void Break()'],
    'mountStuff': ['public void Unstore()'],
    'tosaveitemscript': ['public void Bury()', 'public void UnBury()', 'public bool IsRecentlyUnburied()', 'public void UpdBuried(float _shovelSpeed)', 'public bool IsBuried()', 'public bool IsBuriedRay()', 'public void PreDestroy()', 'public void RandomSpawned(newRandomStuffSpawnScript.item _item)', 'public void RandomSpawned()'],
    'pickupable': ['public bool CanPickup()'],
    'meleeweaponscript': ['private void Update()'],
    'weaponhandlingscript': ['private void Update()', 'public void AttackStart()', 'public void AttackEnd()', 'private void StartAttack(meleeweaponscript.attackType tipus)'],
    'animationEventScript': ['public override void OnStateEnter(Animator animator, AnimatorStateInfo stateInfo, int layerIndex)', 'public override void OnStateExit(Animator animator, AnimatorStateInfo stateInfo, int layerIndex)'],
    'fpscontroller': ['public void usePickedUpStuff(pickupable pP, bool hand)'],
    'TerrainGenerationSettings': ['public void FStart()', 'public void DestroyOthers(GameObject selectedCar)', 'public void CheckCar(GameObject selectedCar, GameObject other)'],
    'mainscript': ['public static bool CheckHoliday(holiday _h)'],
    'settingsscript': ['private void RefreshInMain(bool _changeRes)'],
}


def recover_methods(source, code):
    managed_evidence(source, ['return selectId(itemdatabase.d.categories[categoryId].categories[num9]);'])
    raw = (source.library.records_root/'managed/Assembly-CSharp.cs').read_bytes()
    if hashlib.sha256(raw).hexdigest() != TEXT_SHA256:
        raise ValueError('Acquisition decompilation differs')
    methods = []
    for owner, signatures in METHODS.items():
        owner_text = _block(code, code.index('public class '+owner+' :'))
        for signature in signatures:
            if signature not in owner_text:
                raise ValueError(f'Missing acquisition method {owner}.{signature}')
            text = _block(owner_text, owner_text.index(signature))
            methods.append({'owner':owner, 'signature':signature, 'sha256':hashlib.sha256(text.encode()).hexdigest()})
    return {'assembly_sha256':ASSEMBLY_SHA256, 'text_sha256':TEXT_SHA256, 'methods':methods}


def recover_acquisition(source, manifest):
    code = (source.library.records_root/'managed/Assembly-CSharp.cs').read_text(encoding='utf-8-sig')
    evidence = recover_methods(source, code)
    stations, categories, box_table = recover_station_graphs(source, manifest, code)
    prototypes = {p['item_id']:p for p in manifest['item_prototypes'] if p['item_id'] is not None}
    nodes = {(n['source']['file'],n['source']['path_id']):n for n in manifest['nodes']}
    def ptr(reader, d, field):
        target = source.resolve(reader,d[field])
        return source_id(target) if target else None
    def pose(reader):
        if reader.type.name=='GameObject':reader=source.transform_for(reader)
        d = source.read(reader)
        return {'source':source_id(reader), 'parent':ptr(reader,d,'m_Father'),
                'coordinate_space':'source_parent_local', 'transform':converted_transform(d)}
    def ammo(reader):
        d = source.read(reader)
        return {'source':source_id(reader), 'count':d['ammo'], 'float_count':d['fammo'], 'maximum':d['max'],
                'float_or_int':bool(d['floatOrInt']), 'type':d['tipus'], 'random_range':[d['ammoRmin'],d['ammoRmax']]}
    def startup(go):
        root = source.transform_for(go)
        save = component(source,root,'tosaveitemscript');d=source.read(save)
        pickup = component(source,root,'pickupable');pd=source.read(pickup)
        return {'source':source_id(go), 'pose':pose(root), 'active':bool(source.read(go)['m_IsActive']),
                'save':source_id(save), 'item_id':d['id'], 'body':ptr(save,d,'RB'), 'pickup':source_id(pickup),
                'manual_start':bool(d['manualStart']), 'started':bool(d['started']), 'loaded':bool(d['loaded']),
                'buried':bool(d['buried']), 'car':ptr(save,d,'car'), 'fixed_in_car':pd['fixedInCar']}
    boxp = prototypes[62]
    spawn=source.get('sharedassets1.assets',158204);sd=source.read(spawn)
    broken=source.get('sharedassets1.assets',135113);bd=source.read(broken)
    mount=source.resolve(spawn,sd['mount']);md=source.read(mount)
    box_root=source.get('sharedassets1.assets',51722)
    save=component(source,box_root,'tosaveitemscript');saved=source.read(save)
    break_fields={'health':'health','decrease_after_attack':'decreaseAfterAttack','only_decrease_above':'onlyDecreaseAfterAttackIfAbove',
        'decrease_above_health':'onlyDecreaseAfterAttackIfAboveHealth','mass_modifier':'massModifier','only_velocity':'onlyVelocity',
        'no_trigger':'noTrigger','no_collider':'noCollider','no_destroy':'noDestroy','no_gib':'noGib','destroyed':'destroyed'}
    bool_fields={'decrease_after_attack','only_decrease_above','only_velocity','no_trigger','no_collider','no_destroy','no_gib','destroyed'}
    box={'prototype':boxp['id'], 'source':source_id(spawn), 'table':box_table,
        'disable_spawn':bool(sd['disableSpawn']), 'spawn_before_open':bool(sd['spawnBeforeOpen']),
        'spawned':bool(sd['spawned']), 'opened':bool(sd['opened']), 'spawn_position':pose(source.resolve(spawn,sd['spawnPos']) or box_root),
        'authored_spawn_position':ptr(spawn,sd,'spawnPos'), 'breakable':source_id(broken), 'save':source_id(save),
        'break_rules':{k:bool(bd[v]) if k in bool_fields else bd[v] for k,v in break_fields.items()},
        'mount':{'source':source_id(mount),'parent_save':ptr(mount,md,'parentTosave'),'collider':ptr(mount,md,'col'),
                 'only_manual':bool(md['onlyManual']),'no_physics_lock':bool(md['noPhysicsLock']),
                 'dont_lock_if_picked_up':bool(md['dontLockIfPickedUp'])},
        'release':[{'owner':source_id(broken),'method':'Break','target':source_id(save),'target_method':'PreDestroy'},
                   {'owner':source_id(save),'method':'PreDestroy','target':source_id(mount),'target_method':'Unstore'}],
        'debris':boxp['behavior']['debris'], 'reduced_debris':boxp['behavior']['reduced_debris'],
        'clips':boxp['behavior']['clips'], 'disabled_starter_boxes':[], 'content_counts':[]}
    if saved['physlocks'] != [sd['mount']]:raise ValueError('Box release mount link differs')
    for pid in range(79792,79800):
        r=source.get('level1',pid);d=source.read(r);go=source.resolve(r,d['m_GameObject'])
        box['disabled_starter_boxes'].append({'source':source_id(r),'instance':source_id(go),'disable_spawn':bool(d['disableSpawn'])})
    for p in prototypes.values():
        r=component(source,source.get(p['id']['file'],p['id']['path_id']),'massScript')
        box['content_counts'].append({'prototype':p['id'],'source':source_id(r),'maximum':source.read(r)['maxCountIfSpawnedInBox']})
    packp=prototypes[74];pack=source.get('sharedassets1.assets',141307)
    packs=[{'prototype':packp['id'], 'source':source_id(pack),'ammo':ammo(source.get('sharedassets1.assets',141757)),
            'cigarette':prototypes[230]['id'], 'cigarette_prefab':ptr(pack,source.read(pack),'prefab'),
            'take_one_order':['decrement_integer_count','instantiate_at_pack_pose','initialize','wear_if_slot_empty'],
            'wearable_type':4}]
    terrain=source.get('level1',64898);td=source.read(terrain)
    startup_packs=[]
    for field,pack_id,car_id in [('StartCar01',14079,69517),('StartCar02',14082,77839)]:
        group=source.resolve(terrain,td[field]);go=source.get('level1',pack_id)
        s=startup(go);root=source.transform_for(go)
        cigi=component(source,root,'cigiscript')
        s.update(prototype=packp['id'], group=source_id(group), group_pose=pose(group),
                 group_active=bool(source.read(group)['m_IsActive']),vehicle={'file':'level1','path_id':car_id},
                 pack=source_id(cigi),cigarette_prefab=ptr(cigi,source.read(cigi),'prefab'),
                 ammo=ammo(component(source,root,'ammoscript')))
        startup_packs.append(s)
    pumpkin=source.get('level1',82);holiday=startup(pumpkin)
    holiday.update(prototype=prototypes[92]['id'], wearable={'file':'level1','path_id':79857}, terrain=source_id(terrain),
                   start=[10,15],end=[11,15],inclusive=True,local_calendar=True,new_game_only=True,multiplayer=False)
    shovel=source.get('sharedassets1.assets',132938);sh=source.read(shovel);sp=prototypes[216]
    from .acquisition_motion import recover_motion
    shovel_rules={'prototype':sp['id'],'source':source_id(shovel), 'pickup':ptr(shovel,sh,'p'),
        'target':ptr(shovel,sh,'T'),'trigger':ptr(shovel,sh,'Trigger'),'contact_mask':sh['mask']['m_Bits'],
        'speed':sh['speed'],'shovel_speed':sh['shovelSpeed'],'recent_recovery_seconds':1.,
        'attack_volume':sh['attackVolume'],'attack_clips':sp['behavior']['clips'],
        'contact_clips':sp['behavior']['contact_clips'], 'motion':recover_motion(source,manifest),
        'controller':ptr(shovel,sh,'AnimationController'),
        'right_hand_poses':[ptr(shovel,{'pose':v},'pose') for v in sh['RHs']],
        'left_hand_poses':[ptr(shovel,{'pose':v},'pose') for v in sh['LHs']],
        'contact':{k:sh[v] for k,v in SHOVEL_CONTACT_FIELDS.items()}}
    shovel_rules['contact']['hurt_food']=bool(sh['hurtFood'])
    return {'stations':stations,'categories':categories,'box':box,'shovel':shovel_rules,'packs':packs,
        'startup_packs':startup_packs,'holiday_pumpkin':source_id(pumpkin),'holiday_startup':holiday,
        'pack_item_id':74,'cigarette_item_id':230,'evidence':evidence, 'rules':recover_rules(code,evidence)}


def _shape(value, fields):
    if type(value) is not dict or set(value) != set(fields.split()):
        raise ValueError('Acquisition record fields differ')


def _integer(value, maximum=2**32-1):
    if type(value) is not int or not 0 <= value <= maximum:
        raise ValueError('Acquisition unsigned integer differs')


def _number(value):
    import math
    if type(value) not in (int,float) or not math.isfinite(value) or abs(value)>3.4028234663852886e38:
        raise ValueError('Acquisition finite float differs')


def _boolean(value):
    if type(value) is not bool:raise ValueError('Acquisition boolean differs')


def _source(value):
    _shape(value,'file path_id')
    if value['file'] not in ('sharedassets1.assets','sharedassets0.assets','level1','level0'):
        raise ValueError('Acquisition source container differs')
    _integer(value['path_id'],2**63-1)
    if not value['path_id']:raise ValueError('Acquisition source identity is null')


def _list(value):
    if type(value) is not list:raise ValueError('Acquisition array differs')
    return value


def _numbers(value, length):
    if len(_list(value))!=length:raise ValueError('Acquisition numeric array length differs')
    for v in value:_number(v)


def _pose(value):
    _shape(value,'source parent coordinate_space transform');_source(value['source'])
    if value['parent'] is not None:_source(value['parent'])
    if value['coordinate_space']!='source_parent_local':raise ValueError('Acquisition pose space differs')
    _transform(value['transform'])
    if value not in CHECKED_POSES:raise ValueError('Acquisition source pose differs')


def _transform(t):
    _shape(t,'translation rotation scale')
    for f,n in [('translation',3),('rotation',4),('scale',3)]:_numbers(t[f],n)
    if any(x==0 for x in t['scale']) or abs(sum(x*x for x in t['rotation'])-1)>.002:
        raise ValueError('Acquisition transform differs')


def validate_acquisition(a, manifest):
    try:
        _validate_acquisition(a, manifest)
    except (KeyError, TypeError, IndexError, OverflowError) as error:
        raise ValueError("Malformed acquisition contract") from error


def _validate_acquisition(a, manifest):
    _shape(a,'stations categories box shovel packs startup_packs holiday_pumpkin holiday_startup pack_item_id cigarette_item_id evidence rules')
    finite_record(a)
    validate_rules(a['rules'],a['evidence'])
    if type(a['pack_item_id']) is not int or a['pack_item_id']!=74 or type(a['cigarette_item_id']) is not int or a['cigarette_item_id']!=230:
        raise ValueError('Acquisition item identities differ')
    _source(a['holiday_pumpkin'])
    if a['holiday_pumpkin']!={'file':'level1','path_id':82}:raise ValueError('Acquisition holiday identity differs')
    nodes={n['id']:n for n in manifest['nodes']}
    prototypes={p['item_id']:p for p in manifest['item_prototypes'] if p['item_id'] is not None}
    by_root={(p['id']['file'],p['id']['path_id']):p for p in manifest['item_prototypes']}
    categories={c['id']:c for c in _list(a['categories'])}
    if len(categories)!=len(a['categories']):raise ValueError('Duplicate acquisition category')
    def prototype(identity,item=None):
        _source(identity)
        p=by_root.get((identity['file'],identity['path_id']))
        if p is None or (item is not None and p['item_id']!=item):raise ValueError('Acquisition prototype link differs')
        return p
    def marker(v,asset):
        _shape(v,'source node transform');_source(v['source']);_integer(v['node']);_transform(v['transform'])
        n=nodes.get(v['node'],{})
        if n.get('source')!=v['source'] or n.get('asset')!=asset or n.get('transform')!=v['transform']:
            raise ValueError('Acquisition marker link or pose differs')
    def selection(s):
        if type(s) is not dict:raise ValueError('Acquisition selector differs')
        kind=s.get('kind')
        if kind=='category':
            _shape(s,'kind name category_id');_integer(s['category_id'])
            if s['category_id'] not in categories or CATEGORY_NAMES.get(s['name'])!=s['category_id']:
                raise ValueError('Acquisition category link differs')
        elif kind in ('item','prefab'):
            _shape(s,'kind name item_id prototype'+(' source' if kind=='prefab' else ''))
            if type(s['name']) is not str:raise ValueError('Acquisition selector name differs')
            if s['item_id'] is not None:_integer(s['item_id'])
            elif kind=='item':raise ValueError('Missing direct item ID')
            expected=prototypes.get(s['item_id'])
            if kind=='prefab':
                _source(s['source'])
                if s['source']['file']!='sharedassets1.assets' or SOURCE_PREFABS.get(s['source']['path_id'])!=(s['name'],s['item_id']):raise ValueError('Acquisition direct prefab source item link differs')
                expected_root=SUPPORTED_PREFABS.get((s['source']['file'],s['source']['path_id']))
                expected=by_root.get(expected_root) if expected_root else None
            if s['prototype'] is not None:_source(s['prototype'])
            if s['prototype']!=(expected['id'] if expected else None):raise ValueError('Acquisition selected prototype differs')
            if kind=='item' and ITEM_NAMES.get(s['name'])!=s['item_id']:raise ValueError('Acquisition item enum differs')
        else:raise ValueError('Acquisition selector operation differs')
    def table(entries):
        threshold=0.
        for row in _list(entries):
            _shape(row,'selection weight threshold');selection(row['selection'])
            _number(row['weight']);_number(row['threshold']);threshold+=row['weight']
            if row['weight']<=0 or row['threshold']!=threshold:raise ValueError('Acquisition table threshold differs')
        if not entries:raise ValueError('Empty acquisition table')
    for c in categories.values():
        _shape(c,'id children categories weights constructor_weights depend_on_car constructor_sha256 prototypes invalid_children')
        _integer(c['id']);_boolean(c['categories']);_boolean(c['depend_on_car'])
        expected=CATEGORY_CONTRACTS.get(c['id'])
        if expected is None or [c[k] for k in ('children','categories','weights','constructor_weights','depend_on_car','constructor_sha256')]!=expected:
            raise ValueError('Acquisition checked category branch differs')
        for x in _list(c['children']):_integer(x)
        for x in _list(c['weights']):_integer(x)
        if c['constructor_weights'] is not None:
            for x in _list(c['constructor_weights']):_integer(x)
        if len(_list(c['prototypes']))!=len(c['children']):raise ValueError('Acquisition leaf prototype count differs')
        invalid=[]
        for index,(child,pid) in enumerate(zip(c['children'],c['prototypes'],strict=True)):
            if pid is not None:_source(pid)
            if c['categories']:
                if pid is not None:raise ValueError('Category must not resolve as prototype')
                if child not in categories:
                    if child not in {12:(186,265,303,387),13:(187,266,304,386)}.get(c['id'],()):
                        raise ValueError('Missing acquisition category')
                    invalid.append({'ordinal':index,'target':child,'failure':'category_index_out_of_range'})
            elif pid!=(prototypes[child]['id'] if child in prototypes else None):
                raise ValueError('Acquisition leaf prototype differs')
        for v in _list(c['invalid_children']):
            _shape(v,'ordinal target failure');_integer(v['ordinal']);_integer(v['target'])
        if c['invalid_children']!=invalid:raise ValueError('Native invalid-category terminal differs')
    if list(categories)!=list(CATEGORY_CONTRACTS):raise ValueError('Selected category closure or order differs')
    if len(_list(a['stations']))!=3:raise ValueError('Selected station count differs')
    for index,(s,station) in enumerate(zip(a['stations'],manifest['stations']['types'],strict=True)):
        _shape(s,'source building asset root item_spawn_distance building_chance initial_items_spawned to_start ordinary underground misc4 flyers')
        _source(s['source']);_source(s['building']);_integer(s['asset']);_integer(s['root'])
        if s['building']!={'file':'sharedassets1.assets','path_id':(158348,155446,156988)[index]}:raise ValueError('Station building owner differs')
        f=s['flyers'];_shape(f,'force halloween vertical_offset vertical_scale horizontal prefab supported')
        for field in ('force','halloween','supported'):_boolean(f[field])
        for field in ('vertical_offset','vertical_scale'):_number(f[field])
        _numbers(f['horizontal'],2);_source(f['prefab'])
        if f!=STATION_FLYERS[index]:raise ValueError('Station unsupported flyer contract differs')
        if any(s[f]!=station[f] for f in ('source','asset','root')):raise ValueError('Acquisition station asset link differs')
        _boolean(s['initial_items_spawned']);_numbers(s['building_chance'],2)
        if s['initial_items_spawned'] or s['building_chance']!=[0.,0.] or s['to_start']!=[]:raise ValueError('Station startup differs')
        d=s['item_spawn_distance'];_shape(d,'source draw_distance item_distance authored_draw_distance');_source(d['source'])
        _numbers(d['draw_distance'],2);_numbers(d['item_distance'],2);_number(d['authored_draw_distance'])
        if d!={'source':{'file':'level1','path_id':65886},'draw_distance':[500.,25000.],'item_distance':[140.,340.],'authored_draw_distance':2500.}:
            raise ValueError('Station source distance differs')
        ordinary=_list(s['ordinary'])
        if [v['source']['path_id'] for v in ordinary]!=STATION_SPAWNERS[index]:raise ValueError('Station spawner order differs')
        misc=[]
        for o in ordinary:
            _shape(o,'source marker dont_spawn holiday building_affects_this chance spawned entries')
            binding=SPAWNER_BINDINGS.get(o['source']['path_id'])
            actual=[o['marker']['source']['path_id'],o['chance'],o['dont_spawn'],o['holiday'],o['building_affects_this'],[]]
            for e in o['entries']:
                choice=e['selection']
                sid=choice['source']['path_id'] if choice['kind']=='prefab' else choice.get('category_id',choice.get('item_id'))
                actual[-1].append(((choice['kind'],sid),e['weight'],e['random_rotation'],e['disable_random'],[p['marker']['source']['path_id'] if p['marker'] else 0 for p in e['places']]))
            if o['source']['file']!='sharedassets1.assets' or actual!=binding:raise ValueError('Acquisition spawner source binding differs')
            _source(o['source']);marker(o['marker'],s['asset'])
            for f in ('dont_spawn','building_affects_this','spawned'):_boolean(o[f])
            _integer(o['holiday']);_number(o['chance'])
            if o['holiday']>2 or not 0<=o['chance']<=1 or o['spawned']:raise ValueError('Ordinary spawn guard differs')
            threshold=0
            for e in _list(o['entries']):
                _shape(e,'selection weight threshold places random_rotation disable_random initialization');selection(e['selection'])
                _integer(e['weight']);_integer(e['threshold']);threshold+=e['weight'] or 1
                if e['threshold']!=threshold:raise ValueError('Ordinary item threshold differs')
                for f in ('random_rotation','disable_random'):_boolean(e[f])
                pt=0
                for place in _list(e['places']):
                    _shape(place,'weight threshold marker');_integer(place['weight']);_integer(place['threshold']);pt+=place['weight'] or 1
                    if pt!=place['threshold']:raise ValueError('Ordinary place threshold differs')
                    if place['marker'] is not None:marker(place['marker'],s['asset'])
                v=e['initialization'];_shape(v,'integer_range float_range override_fluid_amount fluid_range override_fluid_type fluids no_mixed_fluids override_mix_chance mix_chance')
                for f in ('integer_range','float_range','fluid_range'):_numbers(v[f],2)
                for x in v['integer_range']:_integer(x)
                for f in ('override_fluid_amount','override_fluid_type','no_mixed_fluids','override_mix_chance'):_boolean(v[f])
                _number(v['mix_chance'])
                for fluid in _list(v['fluids']):
                    _shape(fluid,'type weight authored_threshold');_integer(fluid['type']);_number(fluid['weight']);_number(fluid['authored_threshold'])
            if any(e['selection'].get('category_id')==35 for e in o['entries']):misc.append(o['source'])
        for identity in _list(s['misc4']):_source(identity)
        if s['misc4']!=misc or len(misc)!=(18,6,38)[index]:raise ValueError('misc4 subset differs')
        if len(_list(s['underground']))!=1:raise ValueError('Underground spawner count differs')
        u=s['underground'][0];_shape(u,'source center chance count radius vertical table');_source(u['source']);marker(u['center'],s['asset'])
        for f in ('count','radius','vertical'):_numbers(u[f],2)
        for x in u['count']:_integer(x)
        _number(u['chance'])
        if u['source']!={'file':'sharedassets1.assets','path_id':(154992,132529,157204)[index]} or u['count']!=[1,5] or u['radius']!=([3.,9.],[5.,11.],[13.,20.])[index] or u['vertical']!=[-.15000000596046448,-.4000000059604645] or u['chance']!=.4000000059604645:
            raise ValueError('Underground source bounds differ')
        table(u['table'])
        if u['table']!=UNDERGROUND_TABLES[index]:raise ValueError('Underground complete source table differs')
    _validate_objects(a,manifest,prototype,table)


def _validate_objects(a,manifest,prototype,table):
    b=a['box'];_shape(b,'prototype source table disable_spawn spawn_before_open spawned opened spawn_position authored_spawn_position breakable save break_rules mount release debris reduced_debris clips disabled_starter_boxes content_counts')
    p=prototype(b['prototype'],62);behavior=p['behavior']
    for f in ('source','breakable','save','debris','reduced_debris'):_source(b[f])
    if b['source']!=behavior['spawn'] or b['breakable']!=behavior['breakable'] or b['debris']!=behavior['debris'] or b['reduced_debris']!=behavior['reduced_debris']:
        raise ValueError('Box behavior links differ')
    prototype(b['debris']);prototype(b['reduced_debris']);table(b['table']);_pose(b['spawn_position'])
    if b['spawn_position']!=CHECKED_POSES[0]:raise ValueError('Box spawn pose owner differs')
    if b['table']!=BOX_TABLE:raise ValueError('Box complete source table differs')
    for f in ('disable_spawn','spawn_before_open','spawned','opened'):_boolean(b[f])
    if [b[f] for f in ('disable_spawn','spawn_before_open','spawned','opened')]!=[False,True,False,False] or b['authored_spawn_position'] is not None:
        raise ValueError('Box spawn guards differ')
    r=b['break_rules'];_shape(r,'health decrease_after_attack only_decrease_above decrease_above_health mass_modifier only_velocity no_trigger no_collider no_destroy no_gib destroyed')
    for f in ('health','decrease_above_health','mass_modifier'):_number(r[f])
    for f in set(r)-{'health','decrease_above_health','mass_modifier'}:_boolean(r[f])
    if r!=BOX_BREAK:raise ValueError('Box break thresholds differ')
    m=b['mount'];_shape(m,'source parent_save collider only_manual no_physics_lock dont_lock_if_picked_up')
    for f in ('source','parent_save','collider'):_source(m[f])
    for f in ('only_manual','no_physics_lock','dont_lock_if_picked_up'):_boolean(m[f])
    if m['source']!=behavior['mount'] or m['parent_save']!=b['save'] or not any(c['source']==m['collider'] for c in p['colliders']) or not m['only_manual'] or m['no_physics_lock'] or m['dont_lock_if_picked_up']:
        raise ValueError('Box content mount links differ')
    for e in _list(b['release']):
        _shape(e,'owner method target target_method');_source(e['owner']);_source(e['target'])
    if b['release']!=BOX_RELEASE:raise ValueError('Box release method or target differs')
    if b['clips']!=behavior['clips']:raise ValueError('Box clip links differ')
    for c in _list(b['disabled_starter_boxes']):
        _shape(c,'source instance disable_spawn');_source(c['source']);_source(c['instance']);_boolean(c['disable_spawn'])
    if b['disabled_starter_boxes']!=DISABLED_BOXES:raise ValueError('Disabled starter Box source differs')
    for c in _list(b['content_counts']):
        _shape(c,'prototype source maximum');prototype(c['prototype']);_source(c['source']);_integer(c['maximum'])
    if b['content_counts']!=CONTENT_COUNTS:raise ValueError('Box content count rules differ')
    if len(_list(a['packs']))!=1:raise ValueError('Pack count differs')
    pack=a['packs'][0];_shape(pack,'prototype source ammo cigarette cigarette_prefab take_one_order wearable_type')
    p=prototype(pack['prototype'],74);prototype(pack['cigarette'],230)
    for f in ('source','cigarette_prefab'):_source(pack[f])
    if pack['source']!=p['behavior']['pack'] or pack['cigarette_prefab']!=p['behavior']['cigarette_prefab'] or pack['wearable_type']!=4 or type(pack['wearable_type']) is not int or pack['take_one_order']!=['decrement_integer_count','instantiate_at_pack_pose','initialize','wear_if_slot_empty']:
        raise ValueError('Pack Take one rules differ')
    def ammo(v,pid):
        _shape(v,'source count float_count maximum float_or_int type random_range');_source(v['source'])
        _integer(v['count']);_integer(v['type']);_number(v['float_count']);_number(v['maximum']);_boolean(v['float_or_int']);_numbers(v['random_range'],2)
        if v!={'source':{'file':'sharedassets1.assets' if pid==141757 else 'level1','path_id':pid},'count':20,'float_count':0.,'maximum':20.,'float_or_int':False,'type':5,'random_range':[20.,1.]}:
            raise ValueError('Source pack count or reversed range differs')
    ammo(pack['ammo'],141757)
    instance_fields='source pose active save item_id body pickup manual_start started loaded buried car fixed_in_car'
    def instance(v):
        for f in ('source','save','body','pickup'):_source(v[f])
        _pose(v['pose']);_integer(v['item_id']);_integer(v['fixed_in_car'])
        for f in ('active','manual_start','started','loaded','buried'):_boolean(v[f])
        if v['car'] is not None:_source(v['car'])
        if v['started'] or v['loaded'] or v['buried'] or v['car'] is not None or v['fixed_in_car']!=0:
            raise ValueError('Startup instance initialization differs')
    if len(_list(a['startup_packs']))!=2:raise ValueError('Selected-car startup packs differ')
    for index,(v,expected) in enumerate(zip(a['startup_packs'],STARTUP_PACKS,strict=True)):
        _shape(v,instance_fields+' prototype group group_pose group_active vehicle pack cigarette_prefab ammo');instance(v)
        prototype(v['prototype'],74);_pose(v['group_pose']);_boolean(v['group_active'])
        if [v['pose'],v['group_pose']]!=CHECKED_POSES[2+index*2:4+index*2]:raise ValueError('Startup pack pose owner differs')
        for f in ('group','vehicle','pack','cigarette_prefab'):_source(v[f])
        ammo(v['ammo'],expected['ammo'])
        for f in ('source','save','body','pickup','group','vehicle','pack'):
            if v[f]!={'file':'level1','path_id':expected[f]}:raise ValueError('Selected startup source link differs')
        if not any(car['source']==v['vehicle'] for car in manifest['vehicles']):raise ValueError('Startup group not selected')
        if v['group_active'] or not v['active'] or not v['manual_start'] or v['item_id']!=74 or v['pose']['parent']!=v['group_pose']['source'] or v['cigarette_prefab']!=pack['cigarette_prefab']:
            raise ValueError('Selected startup group rule differs')
    h=a['holiday_startup'];_shape(h,instance_fields+' prototype wearable terrain start end inclusive local_calendar new_game_only multiplayer');instance(h)
    prototype(h['prototype'],92)
    if h!=HOLIDAY_STARTUP:raise ValueError('Holiday source startup fields differ')
    for f in ('wearable','terrain'):_source(h[f])
    for f in ('inclusive','local_calendar','new_game_only','multiplayer'):_boolean(h[f])
    for f in ('start','end'):
        if len(_list(h[f]))!=2:raise ValueError('Holiday date shape differs')
        for x in h[f]:_integer(x)
    if h['source']!=a['holiday_pumpkin'] or h['wearable']!={'file':'level1','path_id':79857} or h['terrain']!={'file':'level1','path_id':64898} or h['start']!=[10,15] or h['end']!=[11,15] or not h['inclusive'] or not h['local_calendar'] or not h['new_game_only'] or h['multiplayer']:
        raise ValueError('Holiday startup predicate differs')
    s=a['shovel'];_shape(s,'prototype source pickup target trigger contact_mask speed shovel_speed recent_recovery_seconds attack_volume attack_clips contact_clips motion controller right_hand_poses left_hand_poses contact')
    p=prototype(s['prototype'],216);behavior=p['behavior']
    for f in ('source','pickup','target','trigger'):_source(s[f])
    for f in ('speed','shovel_speed','recent_recovery_seconds','attack_volume'):_number(s[f])
    _integer(s['contact_mask'])
    if s['source']!=behavior['melee'] or s['target']!=behavior['attack_target'] or s['trigger']!=behavior['attack_trigger'] or s['pickup']!=p['pickups'][0]['pickup']['source'] or s['contact_mask']!=8552961 or s['speed']!=.699999988079071 or s['shovel_speed']!=.5 or s['recent_recovery_seconds']!=1. or s['attack_volume']!=.5 or s['attack_clips']!=behavior['clips'] or s['contact_clips']!=behavior['contact_clips']:
        raise ValueError('Shovel contact or equip links differ')
    _source(s['controller'])
    if s['controller']!=s['motion']['controller']:raise ValueError('Shovel controller owner differs')
    for f in ('right_hand_poses','left_hand_poses'):
        if _list(s[f]):raise ValueError('Selected shovel hand pose array differs')
    _shape(s['contact'],' '.join(SHOVEL_CONTACT_FIELDS))
    for k,v in s['contact'].items():
        if k=='hurt_food':_boolean(v)
        else:_number(v)
    if s['contact']!=SHOVEL_CONTACT:raise ValueError('Shovel source contact inputs differ')
    audio_sources={(c['source']['file'],c['source']['path_id']) for c in manifest['audio']['clips']}
    for clip in [*b['clips'],*s['attack_clips'],*(c for group in s['contact_clips'] for c in group)]:
        _source(clip)
        if (clip['file'],clip['path_id']) not in audio_sources:raise ValueError('Acquisition audio resource missing')
    e=a['evidence'];_shape(e,'assembly_sha256 text_sha256 methods')
    for method in _list(e['methods']):_shape(method,'owner signature sha256')
    if e!=METHOD_EVIDENCE:raise ValueError('Acquisition checked method evidence differs')
    from .acquisition_motion import validate_motion
    validate_motion(s['motion'],manifest)


# Checked build 11140890 identities and branches. Recovery must reproduce these records.
CATEGORY_NAMES = {'car': 0,
 'kaszni': 1,
 'carfull': 2,
 'ldoor': 3,
 'rdoor': 4,
 'door': 5,
 'fbumper': 6,
 'rbumper': 7,
 'bumper': 8,
 'hood': 9,
 'trunk': 10,
 'hoodtrunk': 11,
 'lmirror': 12,
 'rmirror': 13,
 'fmirror': 14,
 'mirror': 15,
 'lnapellenzo': 16,
 'rnapellenzo': 17,
 'napellenzo': 18,
 'rim': 19,
 'tire': 20,
 'hubcap': 21,
 'rimtire': 22,
 'wheel': 23,
 'wheelstuff': 24,
 'attachable': 25,
 'coolant': 26,
 'magazin': 27,
 'kesztyutarto': 28,
 'radio': 29,
 'headlight': 30,
 'kmh': 31,
 'misc1': 32,
 'misc2': 33,
 'misc3': 34,
 'misc4': 35,
 'turd': 36,
 'engine': 37,
 'utanfuto': 38,
 'bus': 39,
 'busengine': 40,
 'buscoolant': 41,
 'bushoodtrunk': 42,
 'buskmh': 43,
 'carkmh': 44,
 'krearseat': 45,
 'bed': 46,
 'beds': 47,
 'compasses': 48,
 'instruments': 49,
 'paints': 50,
 'washtools': 51,
 'miscwashtools': 52,
 'ldoorr': 53,
 'rdoorr': 54,
 'frontframes': 55,
 'bikekmh': 56,
 'biketrunk': 57,
 'bikemisc': 58,
 'bike': 59,
 'bike01': 60,
 'plates': 61,
 'smalldoors': 62,
 'bigdoors': 63,
 'bus02kmh': 64,
 'clocks': 65,
 'toys': 66,
 'melee': 67,
 'ammo': 68,
 'bodypart': 69,
 'food': 70,
 'balls': 71,
 'shotgun': 72,
 'bigfirearm': 73,
 'machineguns': 74,
 'smallfirearms': 75,
 'firearms': 76,
 'humanmonster': 77,
 'flashlights': 78,
 'books': 79,
 'weapon': 80,
 'biginstruments': 81,
 'lengthy': 82,
 'cans': 83,
 'wearables': 84,
 'inbox': 85,
 'bikeTanks': 86,
 'bike03': 87,
 'rodthings': 88,
 'trophies': 89}

ITEM_NAMES = {'box01': 62}

CATEGORY_CONTRACTS = {0: [[1, 2], True, [1, 1], None, False, '164553d9ba67111f6a177268eac5ac894e6fd8ab71eeddf35020fcd642a03115'],
 1: [[1, 15, 98, 29, 94, 122, 153, 222, 243, 257, 275, 309, 337, 356, 371, 390], False, [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
     None, True, '11c16d00d37603d6a41799373f7415ffd4dd80d03c1d3ebadeb04a1b429ab0fa'],
 2: [[2, 16, 99, 30, 95, 123, 154, 223, 244, 272, 276, 314, 338, 357, 372, 388, 391], False,
     [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1], None, True, '16dcc7ee8d46326f03f364ce8b91ab97a42756c8f6724a04740352d2f26513c5'],
 3: [[3, 17, 31, 124, 160, 181, 232, 247, 258, 296, 310, 339, 385], False, [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1], None, True,
     '99c6a8dfe27c3ab41739cf243c320e50e91ab41cd4515a9b9b621b46f1625e43'],
 4: [[4, 18, 32, 125, 158, 182, 233, 248, 259, 297, 311, 340, 384], False, [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1], None, True,
     '4bbeccced313f56772f0c044cdc7c1a50213d879933cb70a36c799a9967d0204'],
 5: [[3, 4, 53, 54], True, [1, 1, 1, 1], None, False, 'a54443fab8925ff5734e7502a6072ab758d3087e6b6de9e78696ee6537c977b3'],
 6: [[7, 21, 33, 128, 161, 188, 236, 262, 298, 351, 378], False, [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1], None, True,
     '1a6949daaea828ca7986158f4cc913e0801c3395a888e999bc625f3f934dd451'],
 7: [[8, 22, 34, 129, 162, 237, 246, 263, 299, 352, 379], False, [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1], None, True,
     '7253b51206eba5fd4ef92df506d4d0196c28d933a17eca214f59d6c4f31dfbb9'],
 8: [[6, 7, 55], True, [1, 1, 1], None, False, 'f54dccddcb99d1b361ccdb60c8d4fc529b3a429e0462a5942d73a8b5da25bd30'],
 9: [[9, 23, 35, 130, 163, 234, 252, 260, 301, 344], False, [1, 1, 1, 1, 1, 1, 1, 1, 1, 1], None, True,
     '61d4004ce52b7cb00ac049ea77dfd5d672aaaab75b0a8cc7a67cee84793707f1'],
 10: [[10, 24, 100, 36, 96, 131, 164, 235, 249, 261, 302, 343, 360], False, [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1], None, True,
      'b2eed83f96b0b933784ba27dedbd4102ab6c777eca2eb547bf58d60759d0076a'],
 11: [[9, 10, 42, 57], True, [100, 100, 50, 50], [100, 100, 50, 50], False,
      '9e68fe8da2504ca1e3602d40b7c62d9d2132764b931639cb2162d28bcdb38d8f'],
 12: [[41, 186, 265, 303, 387], True, [100, 50, 50, 50, 40], [100, 50, 50, 50, 40], False,
      '8cd08a2b0b4e02513ad4c81ed62d36c0db70730c8b2dc1416d193b750adfed92'],
 13: [[42, 187, 266, 304, 386], True, [100, 50, 50, 50, 40], [100, 50, 50, 50, 40], False,
      '13417bd9167032344c5a238825cdfa1da69d481172c6308dbc2d964b953f28fd'],
 14: [[40], False, [1], None, False, '6b71609ce02d931ca59aa36563cb81ef84184c3797dc607eea6ba7daeb84e7f0'],
 15: [[12, 13, 14], True, [1, 1, 1], None, False, '0eb5ba3d51ee4dfd3a5866db955144907165df2ef3b356bdff22a5890ec1509f'],
 16: [[49], False, [1], None, False, 'a8e7a7c038c6b550602de0c6f63f21e7707403e11be104be049e640d3e9feb6e'],
 17: [[50], False, [1], None, False, '6d8a8f7070206ac84c83a24baa954fc350ce13f4b0c43082509ce9fe06fe5a59'],
 18: [[16, 17], True, [1, 1], None, False, 'e223f1d8b04dc6330a88a95c53687f5488c7f4ed23f2705713c567bf9ac4f4f1'],
 19: [[43, 108, 142, 366], False, [1, 1, 1, 1], [100, 50, 25, 25], False,
      'd5bb70fa8a563bad4f8c3e68282defbc9c54acb0916e9aab8baed2717f8c1de3'],
 20: [[44, 109, 143, 268], False, [1, 1, 1, 1], [100, 50, 25, 25], False,
      '6b2ccf1a2549ae2814ea0bfd3b811ffde618685aa20e30b95f21b8904788769f'],
 22: [[47, 274], False, [1, 1], None, False, '607afb68da62ed2126feba47116b72cbd5efdd2128f381cb58b6959a853ba9db'],
 23: [[46, 112, 144, 273, 367, 389], False, [1, 1, 1, 1, 1, 1], [52, 50, 20, 25, 20, 52], False,
      'e69852615e23c585f785c587c3a3b8f08cb0c2b8a5b7e52a17b96d3c76cf88e5'],
 24: [[23, 22, 20, 19], True, [1, 1, 1, 1], None, False, 'ab1c070bf3cbafa07f862a935d623e7ec199de995e670ef57a1127f10db89678'],
 25: [[54, 55, 56, 57, 59, 93, 102], False, [1, 1, 1, 1, 1, 1, 1], None, False,
      '45a36a190a50de54ab2604e715edcabb2e721279f3db8ab614d66c9f95b6b78a'],
 26: [[12, 26, 133, 156, 242, 256, 346], False, [1, 1, 1, 1, 1, 1, 1], None, True,
      '977b372676267dad3c38e171d69f97470e1169dd0be91c225c0bf4974ad137d0'],
 27: [[79], False, [1], None, False, 'd81495db1020bde5278ac44357f03d5b8f6b833e560a882fca54f1c9e2aad51d'],
 28: [[14, 28, 135, 167, 185, 238, 264, 300], False, [1, 1, 1, 1, 1, 1, 1, 1], None, True,
      '4e47b187f8698d4d05a71176a3f7a844f3e0f7409492462788db33e00397d431'],
 29: [[39, 267], False, [1, 1], None, False, '883d94e742699e041f65a67f18dae0f5f3eb6ba705daec2e3840cba9f15dc30c'],
 30: [[48, 253, 104, 137, 147, 165, 166, 348], False, [1, 1, 1, 1, 1, 1, 1, 1], [80, 80, 50, 50, 50, 25, 25, 25], False,
      '1ea527584599fb161f9a0a72b094bfcb1011ffb0d6c405ec2a2e167e6c73560a'],
 31: [[44, 43, 56, 64], True, [1, 1, 1, 1], None, True, 'b0edca771eba3c1de81b8483cd164891888c751ec1460fa9456260d5481069a9'],
 32: [[74, 172, 217, 218, 220, 245, 305, 306], False, [1, 1, 1, 1, 1, 1, 1, 1], [10, 20, 70, 30, 10, 10, 10, 10], False,
      '753b7d68a2b4ab54889e5a952c41d5623675f4707b994d01a5dadc18d7087e88'],
 33: [[15, 18, 30, 31], True, [1, 1, 1, 1], None, False, '54922df1f6dacad6271501256768bc5de5a3b87a9f196d828a34afcc9a7f8c54'],
 34: [[28, 27, 29, 61, 66, 71, 79], True, [1, 1, 1, 1, 1, 1, 1], None, False,
      'bc5ff56a91d40b40561e03bb8e89eb77e32e8a72d8f32895cde7879419f77a0d'],
 35: [[32, 33, 34, 49, 50, 51, 58, 68, 70, 78, 89], True, [20, 20, 20, 5, 35, 25, 30, 15, 50, 20, 3],
      [20, 20, 20, 5, 35, 25, 30, 15, 50, 20, 3], False, 'bc9cfd6cf3f68a3d76114ed70c3a173149e87fd797a223c2ce0af322ea3f0802'],
 36: [[84], False, [1], None, False, '778b13cc4877e06e1931cbe5b1ea2a6d8b386e6fee16a17234bea3e86643289a'],
 37: [[11, 25, 37, 132, 155, 240, 241, 255, 295, 345], False, [1, 1, 1, 1, 1, 1, 1, 1, 1, 1], None, True,
      'e4bb3582c6bdd466debb680ced6fb3747a9f601c2da4eb9acb6d074e9cbc1b14'],
 38: [[52, 53], False, [1, 1], None, False, 'f92599892cb89b318e7201b7c7e410a2296c5b4feae0e2d00af56b160cf8bd7b'],
 39: [[105, 106, 173, 174, 178, 179, 193, 194, 396, 397], False, [1, 1, 1, 1, 1, 1, 1, 1, 1, 1], [15, 30, 40, 50, 20, 50, 30, 40, 30, 40],
      False, '2c58ebdeb731e648b0377bb74cf4219f7cbe51efcd5034dce492c7e19af7303d'],
 41: [[110, 176], False, [1, 1], None, False, 'c16028bb33ca63a000052f55d68a4b2cf94b6bee47a15e26ad833ca60e07f790'],
 42: [[107, 183, 184, 195, 191, 380, 382, 377, 376, 375, 381, 393, 394, 395], False, [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1], None,
      False, '3aeaa67e7a6e1fd1ce0496d46aef58c973cc084b5a935f669fa15b7a4123a061'],
 43: [[113, 114, 115], False, [1, 1, 1], None, False, 'f1be909f9b55612eb1df51d3ccf6e16f9e688c12b3408845443334b2afbf97a1'],
 44: [[13, 27, 38, 134, 138, 139, 168, 169, 250, 251, 270, 307, 308, 349, 350, 355, 398], False,
      [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1], None, True, '141a0a43b7f2950615138bc6462a064c51856d7372d0aebb7752a06935a35242'],
 45: [[101, 97, 361, 373], False, [1, 1, 1, 1], None, False, 'cece59f1b8d70a9a02e58f115c5fcf514e7da68b74b3ab44b00effc9c86901e1'],
 46: [[89, 383], False, [1, 1], [85, 15], False, '940349fd995e5e499aeb7d9b65339143412cb7adbb74f88e136a9d83f602cde7'],
 47: [[46, 45], True, [75, 25], [75, 25], False, '79cb58636bcb11450011e63df4679b27a745d1ad483a4e813133e9cf6f437af9'],
 48: [[116], False, [1], None, False, '54bd2d75c66d453be4307f62a565232549cf5f678cf982700f6eb0da25f194dc'],
 49: [[48, 65], True, [1, 1], None, False, '7a755de69d8b52797843a104f70aab6565c7ffea30655653797ab7ee3df141e5'],
 50: [[117], False, [1], None, False, '1d2f13c0a9d999bc8c424341e8c4ef76e3c0b8ff09750f16b9b82c80cb7834d6'],
 51: [[118, 119, 121], False, [1, 1, 1], None, False, 'fffaeb5b96a5a8cd5146a8d863ca71b1bf45a2669fb648b35c24fffb40f68670'],
 53: [[5, 19, 126, 159, 342, 359], False, [1, 1, 1, 1, 1, 1], None, True,
      '4c3028def489739c010dcd2f027de473fee215d2049b9db8f968bf07baa3a411'],
 54: [[6, 20, 127, 157, 341, 358], False, [1, 1, 1, 1, 1, 1], None, True,
      'fdd7147c24f2c1d59056b624c41cc22dd8d6590fcfb1467ad64a657620344ee7'],
 55: [[136, 239, 347], False, [1, 1, 1], None, False, 'ab61fe8f11805a1f027f1b7d3a3858a728678136642347a12c2f7ba13f0e9d8a'],
 56: [[151, 364], False, [1, 1], None, False, '015f64d6d7da41adaa4d572d88e82c27700299fbdf1a7efe98395fa05850d15a'],
 57: [[148, 149, 365, 369], False, [1, 1, 1, 1], [8, 5, 5, 8], False, 'cc1ec09a7fb16803642f2e127ca7a26b6faf634e882c481db080f1b413b7a95a'],
 58: [[145, 146, 148, 362, 363], False, [1, 1, 1, 1, 1], None, False, 'a09dd3fd8c062eb5f53a072369ca84f731f38fe41feef3526ef6cbb9a388b41f'],
 59: [[60, 87], True, [1, 1], None, False, '7b03dd8735fa7399aa431c7f9dc5a8a28d8606b2239f9edc22359f6bb6bc67fd'],
 60: [[140, 141], False, [1, 1], [10, 5], False, 'fed7c46410a421aed14bb094a6d88eef22b09bf0f785fe238a838742fa2f2864'],
 61: [[177], False, [1], None, False, 'c4480aaaec89e52d6a307f5b689ad4d789e26fdb66fbea25a00c6d207ac3abe0'],
 63: [[190, 377, 376, 375], False, [1, 1, 1, 1], None, False, '47bb006658673bfedc24fc3f9273407418ebf158c27e1fcb726f4f89cf80a3d5'],
 64: [[196, 197, 198, 199], False, [1, 1, 1, 1], None, False, 'c14e453b9ab389d59dc8c7fcda72ec690aebb23cf3766e52779764ce8b636b38'],
 65: [[200], False, [1], None, False, '877de4f7eba9e5d0b9a5f74214b9fe48c3e66b4e2571a75fb3564ef6e0bc99e7'],
 66: [[180, 353], False, [1, 1], None, False, 'c5866be3dae7f5e28679ba493790176101cc461f094d642ca1088d4e22a65355'],
 67: [[63, 212, 216], False, [1, 1, 1], None, False, '9800e427cbc34d57088f8e0b056b1ad7f40700f9e60235a847bc8f59d25de6f5'],
 68: [[76, 213, 215, 279], False, [1, 1, 1, 1], [200, 100, 75, 75], False,
      '0503d01a8ec907e8ee564d59ae9c281c296114cdaf6f43b4f5c214ab3179ad0f'],
 69: [[170, 219], False, [1, 1], None, False, '148d2e4f9be6b42274ea78ebe48f0d5eb5f28f6a9479bfeeeed163c348bc1b51'],
 70: [[224, 225, 226, 227, 228, 229], False, [1, 1, 1, 1, 1, 1], [20, 60, 60, 40, 30, 100], False,
      'b4078d01b818a6f897a0b3e94d0946daac1d75b22e88ec0db3f3b132467bc49a'],
 71: [[271], False, [1], None, False, 'ad1a864f4d5267518aff8fe6515dd4355b5bd627a27dc2fe970aaaa535910d01'],
 72: [[278, 292, 293], False, [1, 1, 1], [100, 100, 100], False, '626616c39d60cf7ae1cc2869c3e0290e97714478b778b4ebc4f925ffc1e1c7cc'],
 73: [[72, 74], True, [100, 100], [100, 100], False, '708498735fa4fe57cdaaa880becde0ba312216cb457944acc5d4f33b6fb46aa1'],
 74: [[214], False, [1], None, False, 'd645214424350beed7977eed1bf606cea0ad6acd1c0ec0bdb23f1173a6d20642'],
 75: [[64], False, [1], None, False, '3aab03171fd40f3789054531e4d1157aa6e01baf125f944f3be9394f1a859e09'],
 76: [[73, 74, 75], True, [100, 100, 100], [100, 100, 100], False, 'b62239d7c68b2e1595b9fa8ec51c2b7b0c8a131920dca513e27220af1bb1f7cc'],
 77: [[317], False, [1], None, False, '53175a530d0bdd73e9437c45df6a3b7faca187bc4874f0c4a78ec4bda82a636d'],
 78: [[66, 320], False, [1, 1], None, False, '82b46d1db7f2b4f7ade5e0ae1a1f920b1713e5b0ff39617c5abc28fb7d0b4cdb'],
 79: [[333], False, [1], None, False, 'ad2b97a19d5456f69e39610d40d09b8ea8cd5a5036d6e2e747f0e395dd787dd7'],
 80: [[76, 67], True, [100, 100], [100, 100], False, 'dade40c7f4f75d262e5b8a25840da8f7b9778af9afdfcaffc32e7b9100eb0df8'],
 81: [[354], False, [1], None, False, '1689ef647ae7448eb93d89c9acbf13d77d8bf0e37aa5fdb6bdc1f456ed9ca507'],
 82: [[80, 81], True, [100, 33], [100, 33], False, '51276cca1d3d47a2f34a674c1bbacbbb93e31388cd6e6befca229e3f95601ce2'],
 83: [[72, 60], False, [1, 1], None, False, 'f741ab5a34269a65a9dc57e301f60206bfdc1bb614161556fd06f099955ea797'],
 84: [[217, 218, 245, 221], False, [1, 1, 1, 1], None, False, 'd92c8f132c88f148637a918abc1d5fd853ffa401dbf1a0dd2bd8e3f4e91c084d'],
 85: [[202, 75, 211, 152, 73, 306, 65, 83, 84, 82, 69, 201, 74, 82, 79], False, [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1], None, False,
      '1d9da9ca3bbe380a3d036e5e9bc3cdfbdafd975aa768f3af8d70ffbd1082a0f9'],
 86: [[362, 363, 370, 392, 399], False, [1, 1, 1, 1, 1], None, False, 'f4501ac2be2fb90bfe2df588cff2844e3a47ca8a9100d9c41e77a588bc76cdb4'],
 87: [[335, 336, 368], False, [1, 1, 1], [20, 5, 5], False, 'bd110a2d0dcf28ae0935809f3c3cd25b0b9f3929e295cc3f61bdad85225e3aec'],
 88: [[171, 401], False, [1, 1], [80, 20], False, '9c38dae79eb916f794e23129e0362ae8401c2fecf192cd3535f6da43f140cb26'],
 89: [[91, 400], False, [1, 1], [50, 50], False, '4540f733c3021f2abf1d41c9f9e2747da553247f2f714f1a31faf5f68356e808']}

STATION_SPAWNERS = [[155092, 131522, 139976, 155127, 145959, 156024, 152647, 152199, 144347, 156794, 152085, 152852, 143968, 152401, 136189, 159015, 132525,
  157727, 144982, 148080, 154174, 148737, 137140, 158935, 154247, 141801, 157379, 159136, 145318, 143532, 144808, 142945, 135698],
 [153352, 135774, 150475, 146976, 144588, 143922, 147378, 145695, 143142, 140022, 142385, 130795, 131645, 134362, 149222, 143195, 158837,
  148932, 150294, 131020, 153823, 137208, 153740, 153176, 139372, 150978, 134469, 144495, 155447, 159356, 153506, 149796, 152316, 140091,
  140159, 158497, 135875],
 [133894, 132473, 134881, 152604, 143849, 141037, 134594, 155658, 155735, 131807, 148462, 133453, 151325, 159388, 151878, 156241, 150554,
  152271, 134832, 152756, 149070, 143444, 144153, 134567, 132539, 131865, 148411, 133089, 153078, 135175, 153070, 151379, 138654, 156146,
  138632, 134371, 131777, 142598, 147640, 136433, 152320, 152354, 149997, 134150, 141763, 145394, 131269, 149343, 154365, 132040, 143201,
  142584, 146500, 140319, 155187, 138169, 138563, 152202, 132607, 133602, 151471, 139581, 130933, 146077, 152232, 140250, 136758, 144321,
  143910, 158529, 147544, 157221, 137563, 157472, 140306, 133988, 156824, 149279, 154875, 153360, 134606, 153100, 151479, 141788, 134009,
  134004, 151772, 154990, 139952, 143756, 159395, 152486, 159108, 149708, 143441, 148956, 133810, 137616, 156686, 151104, 141085, 149001,
  156759, 144858, 155522, 151810, 142649, 158222, 135069, 142695, 135361, 149754, 156893, 142419, 152913, 156419, 130875, 146654, 154127,
  135643, 152942, 158813, 139621, 136713, 156175, 140582, 138065, 135847, 154094, 138584, 146062, 142303, 132513, 138338, 143718, 135235,
  151295, 144008, 153794, 135332, 132925, 136321, 136599, 149893, 133770, 148179, 134021, 138866, 150213, 137377, 148616, 148291, 150601,
  136148, 132645, 154037, 137393, 141383, 140922, 153038, 149590, 157508, 133298, 157793, 144300, 140956, 145078, 144514, 148891, 151014,
  143615, 136463, 154073, 158683, 153893, 137745, 151995, 135598, 130805, 155482, 155305, 143582, 156675, 131215, 159227, 140073, 140141,
  151054, 149896, 137792, 153493, 140991, 139566, 131889, 142600, 142749, 153239, 149396, 138457, 151905, 148855, 156815, 142703, 131517,
  131910, 131490, 156814, 136246, 153963, 154490, 158555, 147609, 139150, 154895, 143647, 139312, 139214, 147519, 153824, 137672, 153505,
  157398, 149671, 136219, 140191, 150410, 136300, 132674, 152360, 159041, 154385, 148893, 156861, 155859, 145390, 141891]]

SUPPORTED_PREFABS = {('sharedassets1.assets', 32825): ('sharedassets1.assets', 75443),
 ('sharedassets1.assets', 9085): ('sharedassets1.assets', 51722),
 ('sharedassets1.assets', 18963): ('sharedassets1.assets', 61587),
 ('sharedassets1.assets', 39185): ('sharedassets1.assets', 81642),
 ('sharedassets1.assets', 39188): ('sharedassets1.assets', 81645),
 ('sharedassets1.assets', 43845): ('sharedassets1.assets', 70812),
 ('sharedassets1.assets', 24481): ('sharedassets1.assets', 53838),
 ('sharedassets1.assets', 24480): ('sharedassets1.assets', 53837),
 ('sharedassets1.assets', 20981): ('sharedassets1.assets', 63605),
 ('sharedassets1.assets', 20649): ('sharedassets1.assets', 63259),
 ('sharedassets1.assets', 11985): ('sharedassets1.assets', 54495)}

BOX_BREAK = {'health': 7.0,
 'decrease_after_attack': False,
 'only_decrease_above': False,
 'decrease_above_health': 0.0,
 'mass_modifier': 0.0,
 'only_velocity': False,
 'no_trigger': True,
 'no_collider': False,
 'no_destroy': False,
 'no_gib': False,
 'destroyed': False}

BOX_RELEASE = [{'owner': {'file': 'sharedassets1.assets', 'path_id': 135113},
  'method': 'Break',
  'target': {'file': 'sharedassets1.assets', 'path_id': 145573},
  'target_method': 'PreDestroy'},
 {'owner': {'file': 'sharedassets1.assets', 'path_id': 145573},
  'method': 'PreDestroy',
  'target': {'file': 'sharedassets1.assets', 'path_id': 146589},
  'target_method': 'Unstore'}]

DISABLED_BOXES = [{'source': {'file': 'level1', 'path_id': 79792}, 'instance': {'file': 'level1', 'path_id': 1904}, 'disable_spawn': True},
 {'source': {'file': 'level1', 'path_id': 79793}, 'instance': {'file': 'level1', 'path_id': 1902}, 'disable_spawn': True},
 {'source': {'file': 'level1', 'path_id': 79794}, 'instance': {'file': 'level1', 'path_id': 1903}, 'disable_spawn': True},
 {'source': {'file': 'level1', 'path_id': 79795}, 'instance': {'file': 'level1', 'path_id': 1909}, 'disable_spawn': True},
 {'source': {'file': 'level1', 'path_id': 79796}, 'instance': {'file': 'level1', 'path_id': 1906}, 'disable_spawn': True},
 {'source': {'file': 'level1', 'path_id': 79797}, 'instance': {'file': 'level1', 'path_id': 1905}, 'disable_spawn': True},
 {'source': {'file': 'level1', 'path_id': 79798}, 'instance': {'file': 'level1', 'path_id': 1908}, 'disable_spawn': True},
 {'source': {'file': 'level1', 'path_id': 79799}, 'instance': {'file': 'level1', 'path_id': 1907}, 'disable_spawn': True}]

CONTENT_COUNTS = [{'prototype': {'file': 'sharedassets1.assets', 'path_id': 75443},
  'source': {'file': 'sharedassets1.assets', 'path_id': 132605},
  'maximum': 10},
 {'prototype': {'file': 'sharedassets1.assets', 'path_id': 51722},
  'source': {'file': 'sharedassets1.assets', 'path_id': 136544},
  'maximum': 0},
 {'prototype': {'file': 'sharedassets1.assets', 'path_id': 61587},
  'source': {'file': 'sharedassets1.assets', 'path_id': 156405},
  'maximum': 0},
 {'prototype': {'file': 'sharedassets1.assets', 'path_id': 70812},
  'source': {'file': 'sharedassets1.assets', 'path_id': 134682},
  'maximum': 5},
 {'prototype': {'file': 'sharedassets1.assets', 'path_id': 53838},
  'source': {'file': 'sharedassets1.assets', 'path_id': 147662},
  'maximum': 3},
 {'prototype': {'file': 'sharedassets1.assets', 'path_id': 53837},
  'source': {'file': 'sharedassets1.assets', 'path_id': 147661},
  'maximum': 3},
 {'prototype': {'file': 'sharedassets1.assets', 'path_id': 63605},
  'source': {'file': 'sharedassets1.assets', 'path_id': 134243},
  'maximum': 3},
 {'prototype': {'file': 'sharedassets1.assets', 'path_id': 63259},
  'source': {'file': 'sharedassets1.assets', 'path_id': 141432},
  'maximum': 0},
 {'prototype': {'file': 'sharedassets1.assets', 'path_id': 54495},
  'source': {'file': 'sharedassets1.assets', 'path_id': 134663},
  'maximum': 0}]

STARTUP_PACKS = [{'source': 14079, 'save': 72692, 'body': 50432, 'pickup': 75069, 'group': 298, 'vehicle': 69517, 'pack': 70704, 'ammo': 70998},
 {'source': 14082, 'save': 72693, 'body': 50434, 'pickup': 75072, 'group': 206, 'vehicle': 77839, 'pack': 70705, 'ammo': 70999}]

METHOD_EVIDENCE = {'assembly_sha256': '9919a9eeddcce23bb4ff86e890437f32b4a3ef3c10af2efe3b8d889d894dd62e',
 'methods': [{'owner': 'itemdatabase',
              'sha256': 'b9b61ee3764d32d237916d474e2c6e3d31b3af739542ed2fda19a23a8b8c905e',
              'signature': 'public class category'},
             {'owner': 'itemdatabase',
              'sha256': '22061576aceb7e2f81644f4fb933185beed315a8cd96ebbf03500872d4f8707b',
              'signature': 'private void Awake()'},
             {'owner': 'buildingscript',
              'sha256': 'f9a89c3b683110ad99f25f146b50522e30b912d8c4a7e932d5e63db39eaef409',
              'signature': 'public void CheckDist()'},
             {'owner': 'buildingscript',
              'sha256': 'c852c28b313f23d8e001a4f4567cb97029393689b55dcaca3db73985c73ca020',
              'signature': 'public void SpawnStuff(int pId)'},
             {'owner': 'buildingscript',
              'sha256': 'ea043ea67b565b933b3041fe616552991901ba4a157d8ecedf5bfa5d73620c24',
              'signature': 'private IEnumerator SPAWN()'},
             {'owner': 'buildingscript',
              'sha256': 'b07739fabe3e11eea742682b8ca8bddaecd0127d6f3a049c89f7545a56bd845f',
              'signature': 'public static void RandomFlyerSpawn(bool _forceRandomFlyer, bool '
                           '_randomFlyerOnHalloween, Vector3 _pos, float _offsetty, float _vertical, float '
                           '_horizontalMin, float _horizontalMax)'},
             {'owner': 'newRandomStuffSpawnScript',
              'sha256': 'cb0ee0cbf20c6477464f14ede286624d783e06b033ed96f36527a16edde311a3',
              'signature': 'public void Spawn()'},
             {'owner': 'newRandomStuffSpawnScript',
              'sha256': '8b49cde8582fd0df9fd24a186e87dad6572f57ae597c7f85f1b86f5412443d99',
              'signature': 'public void InitSpawn()'},
             {'owner': 'newRandomStuffSpawnScript',
              'sha256': '7a4695625546bb5ca6a9608ea3cca3aa45b2a3e36c743faec940a995acf7601e',
              'signature': 'public void SelectSpawn()'},
             {'owner': 'newRandomStuffSpawnScript',
              'sha256': 'd3b2b7bff0d643afb0bc0e9c46ff0ba4e222c7975700f1d466c926aad04e0c4a',
              'signature': 'public void SpawnItem()'},
             {'owner': 'newRandomStuffSpawnScript',
              'sha256': '30333202a88e361d1a297ecc4260c6a1747e6de2d8c544d6d90198fe910c2108',
              'signature': 'public static int SelectItem(string _categoryName, bool _isOneItem)'},
             {'owner': 'newRandomStuffSpawnScript',
              'sha256': '8f20129ab487b00b6a43d00a1d827cc2702c205042503d7b2db822568649e9fd',
              'signature': 'public static int selectId(int categoryId)'},
             {'owner': 'undergroundSpawnScript',
              'sha256': 'f55feec80ca147429458381487134c7b960e9915265ff88d98bc335c592cacb7',
              'signature': 'private void Init()'},
             {'owner': 'undergroundSpawnScript',
              'sha256': '307e136e3b4237c71f14908ca62fedd3f62e8800ccaf4f6e186c72e2658e2a0a',
              'signature': 'public void Spawn()'},
             {'owner': 'undergroundSpawnScript',
              'sha256': '02efe82674d916dc21ae4bbc2d942ae2d847069b9ccee8778e62ae87948c8ccf',
              'signature': 'public Vector3 RandomPoint()'},
             {'owner': 'spawnInBoxScript',
              'sha256': 'b4a8f1852338e5819ba02c84fc65832cde84eda60db40cb65acc0eda57ed9be5',
              'signature': 'public void FStart()'},
             {'owner': 'spawnInBoxScript',
              'sha256': '96b8b084e82c28cdf39b5ae1772dc74ac33c5189570a40ae753f35e45e0a70d8',
              'signature': 'public void Spawn()'},
             {'owner': 'spawnInBoxScript',
              'sha256': '8d52079846a3567f0f75f94a0e86acda2386856bbe7415851f48c23a72b1976b',
              'signature': 'public void Opened()'},
             {'owner': 'breakablescript',
              'sha256': '9753ebe873cf5ce7c315276df93901da01c38e3078f507f5a9025dd3dd872d51',
              'signature': 'private void Start()'},
             {'owner': 'breakablescript',
              'sha256': 'bb98ef04b1aaab4aceb4db24a3f37a3d725db7a14317109023abe3a332a99555',
              'signature': 'private void OnCollisionEnter(Collision c)'},
             {'owner': 'breakablescript',
              'sha256': 'b93980330be86c796b96ee6c133d5566e1c3dad6a7341e9999f478136604340a',
              'signature': 'public void TryBreak(float force)'},
             {'owner': 'breakablescript',
              'sha256': '96993e34070a480e03ea9f7078943c885f40f0065be8a643ba300bf71441b04f',
              'signature': 'public void Break()'},
             {'owner': 'mountStuff',
              'sha256': '90e704fe9745e42b01e2d3d7bc9df9a81bafaa6e1fb054e338417daf07495c6e',
              'signature': 'public void Unstore()'},
             {'owner': 'tosaveitemscript',
              'sha256': '684fbee4e2b10d87afa0dcc531bd17829dceaa71f3204c91004e66fff6b5996f',
              'signature': 'public void Bury()'},
             {'owner': 'tosaveitemscript',
              'sha256': 'd7c4139e456207ed942b6578533ebba65f875870cd3ce2c2ffc758c956c1b5de',
              'signature': 'public void UnBury()'},
             {'owner': 'tosaveitemscript',
              'sha256': 'a8274fc43289477a288c1eca56c86b6c1216809a233df37dda75ca76302fbde1',
              'signature': 'public bool IsRecentlyUnburied()'},
             {'owner': 'tosaveitemscript',
              'sha256': 'ea559f724cbc5a3f79b3197e7335060214dbdd2be3ccbfedcfa9d339d54c66fb',
              'signature': 'public void UpdBuried(float _shovelSpeed)'},
             {'owner': 'tosaveitemscript',
              'sha256': 'e8cc70ae7a846f17e558e7f08e2775e433e678343158e1cf8d35403a8aaf8b0b',
              'signature': 'public bool IsBuried()'},
             {'owner': 'tosaveitemscript',
              'sha256': 'f09fbc891f437bb6581bd15793d7fa6c8360e4cbb1edb2298ad64c35c87589bf',
              'signature': 'public bool IsBuriedRay()'},
             {'owner': 'tosaveitemscript',
              'sha256': 'e80c8ef220d699926fcbc7ed80d8f7595d0f349c7ec26cf9d76fafc89aadf77b',
              'signature': 'public void PreDestroy()'},
             {'owner': 'tosaveitemscript',
              'sha256': '3aa3df017a3d8c7b732675bbacbf59d8e0ca972c6ff02d0d9a1b780aa6308db3',
              'signature': 'public void RandomSpawned(newRandomStuffSpawnScript.item _item)'},
             {'owner': 'tosaveitemscript',
              'sha256': 'd50cff85375857709762223bd04a9d9f96ef3d30351a2be5aa28aff0032b561a',
              'signature': 'public void RandomSpawned()'},
             {'owner': 'pickupable',
              'sha256': 'a1f59e9f928f6cb0ccb9a6b333d6537f05e06f5700e7afcb20b2f45c2d566b18',
              'signature': 'public bool CanPickup()'},
             {'owner': 'meleeweaponscript',
              'sha256': '90a3ff38e366c3ff0be201fd0d483fc20efb339ecf766416031a4d0a3ae53d6c',
              'signature': 'private void Update()'},
             {'owner': 'weaponhandlingscript',
              'sha256': 'f27677d91a5be38c8410aa4c22d05c4b61e535e44d98cf0f036f617d2f76ccfd',
              'signature': 'private void Update()'},
             {'owner': 'weaponhandlingscript',
              'sha256': 'ca1e526bb6069b2f3c63d61b7edb9b90f6c7034e4c8a0c5d74c1f919eb5a7070',
              'signature': 'public void AttackStart()'},
             {'owner': 'weaponhandlingscript',
              'sha256': '24243fd2a50609203de6a37894c9245831eb43212b8b1881b63e521fe3ec233f',
              'signature': 'public void AttackEnd()'},
             {'owner': 'weaponhandlingscript',
              'sha256': '4994135333ba9ed241262820c5908192051e9bbf002111feb814c4868f300a71',
              'signature': 'private void StartAttack(meleeweaponscript.attackType tipus)'},
             {'owner': 'animationEventScript',
              'sha256': 'ac3b3c585431f5212ba04dbf9ceeaacdbc91e679b3391f1cccfcf9d84ce7da10',
              'signature': 'public override void OnStateEnter(Animator animator, AnimatorStateInfo '
                           'stateInfo, int layerIndex)'},
             {'owner': 'animationEventScript',
              'sha256': '9a76b8df3823f85846a6b0ec7546539a02ca86fdf22d4868a97ef0ac29af57a9',
              'signature': 'public override void OnStateExit(Animator animator, AnimatorStateInfo stateInfo, '
                           'int layerIndex)'},
             {'owner': 'fpscontroller',
              'sha256': 'a06a516862a5adfa6ede969a7f40cab3f35969a280d1526ae22b0165de6880f3',
              'signature': 'public void usePickedUpStuff(pickupable pP, bool hand)'},
             {'owner': 'TerrainGenerationSettings',
              'sha256': '024d1ccfbeb54e9a5a1cbfb12ab901e1b8a7462b8b71206eb4dd5afb3a997eb6',
              'signature': 'public void FStart()'},
             {'owner': 'TerrainGenerationSettings',
              'sha256': '525cbab037ff441ee3d2ad267004f8e813a7d83990a71deb078f500a78a6cec6',
              'signature': 'public void DestroyOthers(GameObject selectedCar)'},
             {'owner': 'TerrainGenerationSettings',
              'sha256': '49c4dbd4c4ae06b00af5bc812f7af4dde8db8b985146c25f44eaa2931feaf78d',
              'signature': 'public void CheckCar(GameObject selectedCar, GameObject other)'},
             {'owner': 'mainscript',
              'sha256': 'a0d401d36203d5775fc3a69a290fa922759863f7bb0c7456b0720272d27b2f85',
              'signature': 'public static bool CheckHoliday(holiday _h)'},
             {'owner': 'settingsscript',
              'sha256': '79c5c2ef69812b60f3426138d2f33f45e52cd8f72b01f8d0fd3cb50423e01f18',
              'signature': 'private void RefreshInMain(bool _changeRes)'}],
 'text_sha256': '22ddc62b91954205d90ff513232f39becebbc58a71b718685075f91f64b1f3ee'}

STATION_FLYERS = [{'force': False,
  'halloween': True,
  'horizontal': [6.0, 16.0],
  'prefab': {'file': 'sharedassets1.assets', 'path_id': 32394},
  'supported': False,
  'vertical_offset': 10.0,
  'vertical_scale': 5.0},
 {'force': False,
  'halloween': True,
  'horizontal': [6.0, 16.0],
  'prefab': {'file': 'sharedassets1.assets', 'path_id': 32394},
  'supported': False,
  'vertical_offset': 10.0,
  'vertical_scale': 5.0},
 {'force': False,
  'halloween': True,
  'horizontal': [6.0, 16.0],
  'prefab': {'file': 'sharedassets1.assets', 'path_id': 32394},
  'supported': False,
  'vertical_offset': 10.0,
  'vertical_scale': 5.0}]

# These records describe checked calls. They do not execute source control flow.
def _rule(action):
    return {'kind':'rule','action':action}


def _draw(kind, lower, upper, guard, use):
    def bound(v):
        return {'kind':'source','value':v} if isinstance(v,str) else {'kind':'constant','value':v}
    return {'kind':kind,'lower':bound(lower),'upper':bound(upper),'guard':guard,'use':use}


def _sphere(guard,use):
    return {'kind':'unit_sphere','guard':guard,'use':use}


def _flow(owner,signature,*steps):
    return {'owner':owner,'signature':signature,'steps':list(steps)}


ACQUISITION_FLOWS = [
    _flow('settingsscript','private void RefreshInMain(bool _changeRes)',
        _rule('clamped_inverse_lerp_draw_distance_then_lerp_item_distance')),
    _flow('buildingscript','public void CheckDist()',
        _rule('closest_important_object_distance_strictly_less_than_item_distance'),
        _rule('set_items_spawned_before_spawn_stuff')),
    _flow('buildingscript','public void SpawnStuff(int pId)',
        _rule('collect_ordinary_in_hierarchy_order_including_inactive'),
        _draw('float_range','minRandomSpawn','maxRandomSpawn','either_bound_nonzero','replace_chance_only_building_affects_this'),
        _rule('spawn_all_underground_before_starting_spawn_coroutine')),
    _flow('buildingscript','private IEnumerator SPAWN()',
        _rule('activate_then_fstart_to_start_in_order'),_rule('painting_fstart_boundary'),
        _rule('call_random_flyer_spawn_before_ordinary'),
        _rule('ordinary_hierarchy_order_skip_dont_spawn_yield_after_each_call')),
    _flow('buildingscript','public static void RandomFlyerSpawn(bool _forceRandomFlyer, bool _randomFlyerOnHalloween, Vector3 _pos, float _offsetty, float _vertical, float _horizontalMin, float _horizontalMax)',
        _draw('float_range',0.,1.,'not_force_and_halloween_flag_and_holiday','spawn_if_greater_than_point35'),
        _draw('integer_range',5,30,'force_or_successful_holiday_draw','flyer_count'),
        _sphere('each_flyer','scale_y_by_vertical'),
        _draw('float_range','_horizontalMin','_horizontalMax','each_flyer_after_sphere','scale_x_z_then_add_position_and_vertical_offset'),
        _rule('unsupported_flyer_instantiate_then_fstart_boundary')),
    _flow('newRandomStuffSpawnScript','public void Spawn()',
        _draw('float_range',.001,1.,'holiday_none_or_active_and_server_or_no_multi','reject_greater_than_chance'),
        _rule('init_spawn_only_if_not_spawned_then_select_spawn_then_spawn_item')),
    _flow('newRandomStuffSpawnScript','public void InitSpawn()',
        _rule('zero_item_and_place_weights_become_one_before_cumulative_sum')),
    _flow('newRandomStuffSpawnScript','public void SelectSpawn()',
        _draw('integer_range',0,'ci + 1','always','first_less_or_equal_item_threshold_excluding_peaceful_humanmonster'),
        _rule('default_selected_name_error_position_zero_rotation_spawner'),
        _draw('integer_range',0,'items[selected].cp + 1','selected_item','first_less_or_equal_place_threshold'),
        _rule('null_place_logs_and_keeps_default_pose_without_reroll')),
    _flow('newRandomStuffSpawnScript','public void SpawnItem()',
        _rule('direct_gameobject_precedes_item_name_otherwise_category_name'),
        _rule('selector_exception_logs_prefab_error_and_returns_before_instantiate'),
        _rule('null_prefab_instantiate_error_is_outside_selector_catch'),
        _draw('float_range',0.,360.,'selected_random_rotation_after_instantiate','rotate_world_y'),
        _rule('root_spawner_spawn_else_child_spawners_in_order'),
        _rule('child_unparent_replaces_instance_then_random_spawned_item_unless_disabled'),
        _rule('set_body_kinematic_then_fstart_then_set_spawned')),
    _flow('newRandomStuffSpawnScript','public static int SelectItem(string _categoryName, bool _isOneItem)',
        _rule('item_name_parse_or_category_select_without_catch')),
    _flow('newRandomStuffSpawnScript','public static int selectId(int categoryId)',
        _rule('unchecked_category_lookup_errors_are_native_terminals'),
        _rule('null_itemchances_and_categorychances_use_unit_weights'),
        _draw('integer_range',1,'item_total + category_total + 1','always','less_or_equal_item_total_selects_item_branch'),
        _draw('integer_range',0,100,'selected_branch_depend_on_car','less_than_25_and_mapped_car_index_in_bounds_returns_direct_child_id'),
        _draw('integer_range',0,'selected_branch_length','depend_on_car_without_direct_return','repeat_while_mapped_car_index_at_most_ten_draws_then_return_direct_child_id'),
        _rule('independent_items_first_less_or_equal_cumulative_threshold_return_item'),
        _rule('independent_categories_first_less_or_equal_threshold_recurse_without_subtracting_item_total'),
        _rule('no_matching_threshold_returns_item_zero')),
    _flow('undergroundSpawnScript','public void Spawn()',
        _rule('initialize_cumulative_float_weights_once'),
        _draw('float_range',0.,1.,'always','reject_greater_or_equal_chance'),
        _draw('integer_range','minCount','maxCount + 1','successful_chance','attempt_count'),
        _rule('each_attempt_random_point_before_table_selection'),
        _draw('float_range',0.,'maxChance','each_attempt','first_strictly_less_table_threshold'),
        _rule('select_item_exception_propagates_return_item_null_is_empty_attempt'),
        _sphere('nonnull_prefab','euler_rotation_times_360'),
        _rule('instantiate_then_fstart_then_random_spawned_default_then_bury')),
    _flow('undergroundSpawnScript','public Vector3 RandomPoint()',
        _draw('float_range',0.,360.,'always','angle_degrees'),
        _draw('float_range','minRadius','maxRadius','after_angle','radius'),
        _draw('float_range','minOffsetty','maxOffsetty','after_radius','vertical_preserve_reversed_bounds'),
        _rule('center_plus_vertical_up_plus_yaw_forward_times_radius')),
    _flow('spawnInBoxScript','public void FStart()',
        _rule('disabled_returns_before_table_initialization'),
        _draw('float_range',0.,'maxChance','enabled_after_cumulative_weights','first_strictly_less_table_threshold'),
        _rule('select_item_exception_propagates_no_threshold_or_null_item_keeps_empty'),
        _rule('spawn_before_open_calls_spawn')),
    _flow('spawnInBoxScript','public void Spawn()',
        _rule('disabled_or_spawned_or_null_prefab_returns'),_rule('null_spawn_position_uses_box_transform'),
        _rule('current_count_one_and_spawned_count_zero'),
        _sphere('spawned_count_greater_than_zero_before_instantiate','offset_spawn_scale_x_times_point15'),
        _draw('integer_range',1,'maxCountIfSpawnedInBox + 1','first_instance_mass_count_positive','current_count'),
        _rule('cant_pickup_while_mount_then_fstart_then_random_spawned_default'),
        _sphere('each_iteration_after_initialization','discarded_position_assignment'),
        _rule('increment_count_repeat_while_less_than_current_count'),
        _rule('mount_fstart_quickfreeze_prescan_disable_then_set_spawned')),
    _flow('spawnInBoxScript','public void Opened()',_rule('first_open_calls_spawn_then_sets_opened')),
    _flow('tosaveitemscript','public void RandomSpawned(newRandomStuffSpawnScript.item _item)',
        _draw('integer_range','(int)_item.fr1','(int)_item.fr2','ammo_exists_and_not_spray','set_integer_ammo'),
        _draw('float_range','ammo.ammoRmin','ammo.ammoRmax','ammo_above_max_or_equal_item_float_bounds','set_float_ammo_or_truncate_to_integer'),
        _rule('partcondition_random_start_item_integer_bounds_or_default_zero_to_four'),
        _rule('apply_fluid_overrides_then_randomfluid_tank_other_tanks_randomize_tanks_boundaries'),
        _draw('float_range',0.,1.,'food_exists','food_hp'),
        _draw('integer_range',0,'food.parts.Length - 2','food_exists_after_hp','food_index_clamped_at_zero')),
    _flow('tosaveitemscript','public void RandomSpawned()',
        _rule('nonspray_ammo_set_to_max_without_ammo_draw'),
        _rule('partcondition_random_start_default_zero_to_one'),
        _rule('engine_coolant_empty_chance_one_other_zero_mix_zero_then_fluid_boundaries')),
    _flow('breakablescript','public void TryBreak(float force)',_rule('force_greater_or_equal_health_break_else_decrease_health_if_enabled')),
    _flow('animationEventScript','public override void OnStateEnter(Animator animator, AnimatorStateInfo stateInfo, int layerIndex)',_rule('state_enter_calls_weapon_attack_start')),
    _flow('animationEventScript','public override void OnStateExit(Animator animator, AnimatorStateInfo stateInfo, int layerIndex)',_rule('state_exit_calls_weapon_attack_end')),
    _flow('breakablescript','public void Break()',
        _rule('pickup_drop2_before_mark_destroyed'),
        _rule('mark_destroyed_select_reduced_or_full_gib_native_eligibility'),
        _draw('integer_range',0,'clips.Length','clips_nonempty','break_clip'),
        _rule('set_remove_from_memory_predestroy_then_destroy_when_destroy_allowed')),
    _flow('tosaveitemscript','public void PreDestroy()',_rule('remove_from_memory_calls_each_physlock_unstore')),
    _flow('mountStuff','public void Unstore()',_rule('unphysicslock_then_rebury_if_parent_buried')),
    _flow('weaponhandlingscript','private void Update()',
        _rule('animator_speed_and_attack_trigger_follow_source_input_and_state'),
        _draw('integer_range',0,'melee.attackClips.Length','attacking_and_not_played_and_elapsed_greater_than_point1_divided_by_speed','attack_clip')),
    _flow('meleeweaponscript','private void Update()',
        _rule('hand_pose_arrays_guard_attack_index_position_lerp_delta16_rotation_lerp_delta8'),
        _rule('overlap_box_trigger_pose_scale_mask_query_collide_excludes_same_root_and_singleplayer_layer_nine'),
        _rule('contact_force_from_target_delta_times_half_delta_time_rigidbody_add_unless_recent'),
        _draw('float_range',.9,1.1,'accepted_contact','spark_force_upper_multiplier'),
        _draw('float_range','allparticle * FcurrentForce','allparticle * FcurrentForce * spark_multiplier','accepted_contact_after_multiplier','spark_count_truncated_integer'),
        _rule('particle_and_hit_type_layer17_sand_layer23_glass_then_flesh_plant_wood_else_metal'),
        _rule('nonzero_shovel_speed_updates_buried_or_recent_objects_else_native_damage_boundaries'),
        _draw('integer_range',0,'meleeHitSounds[hitType].a.Length','has_to_sound_not_sounded_after_next_time_and_point125_delay','contact_clip')),
    _flow('tosaveitemscript','public void Bury()',_rule('body_kinematic_buried_true_update_outline')),
    _flow('tosaveitemscript','public void UnBury()',_rule('record_time_only_if_buried_then_body_dynamic_buried_false_outline')),
    _flow('tosaveitemscript','public bool IsRecentlyUnburied()',_rule('time_strictly_less_than_unburied_time_plus_one')),
    _flow('tosaveitemscript','public void UpdBuried(float _shovelSpeed)',_rule('once_per_delta_time_move_up_delta_times_speed_then_is_buried_ray')),
    _flow('tosaveitemscript','public bool IsBuried()',_rule('buried_body_dynamic_unbury_else_is_buried_ray')),
    _flow('tosaveitemscript','public bool IsBuriedRay()',_rule('downward_raycast_all_sorted_distance_lowest_enabled_nontrigger_nonwheel_nonlayer9_collider'),
        _rule('exclude_self_and_player_hits_unbury_if_below_lowest_and_hit_not_buried')),
    _flow('pickupable','public bool CanPickup()',_rule('not_cant_pickup_and_no_save_or_not_is_buried')),
    _flow('fpscontroller','public void usePickedUpStuff(pickupable pP, bool hand)',
        _rule('cursor_locked_use_down_positive_pack_ammo'),
        _rule('decrement_integer_count_then_instantiate_cigarette_at_pack_pose_then_fstart'),
        _rule('empty_wearable_slot_wears_else_retains_world_cigarette')),
    _flow('TerrainGenerationSettings','public void FStart()',
        _rule('new_game_not_loaded_not_multiplayer_local_calendar_holiday_activation'),
        _rule('selected_car_group_check_outside_holiday_new_game_block')),
    _flow('TerrainGenerationSettings','public void CheckCar(GameObject selectedCar, GameObject other)',
        _rule('activate_selected_start_group_destroy_unselected_start_groups')),
    _flow('mainscript','public static bool CheckHoliday(holiday _h)',_rule('local_calendar_inclusive_start_and_end_same_year')),
]


def recover_rules(code,evidence):
    import copy
    methods={(v['owner'],v['signature']) for v in evidence['methods']}
    if any((f['owner'],f['signature']) not in methods for f in ACQUISITION_FLOWS):
        raise ValueError('Acquisition operation lacks checked method')
    database=_block(code,code.index('public class itemdatabase :'))
    # CarType has 23 source values; unassigned index17 retains the CLR zero value.
    car_map=[0]*23
    for i,v in re.findall(r'carTypeToCarID\[(\d+)\] = (\d+);',database):car_map[int(i)]=int(v)
    return {'integer_upper_exclusive':True,'float_upper_inclusive':True,
            'car_type_to_item_index':car_map,'flows':copy.deepcopy(ACQUISITION_FLOWS)}


def validate_rules(r,evidence):
    _shape(r,'integer_upper_exclusive float_upper_inclusive car_type_to_item_index flows')
    _boolean(r['integer_upper_exclusive']);_boolean(r['float_upper_inclusive'])
    if not r['integer_upper_exclusive'] or not r['float_upper_inclusive']:raise ValueError('Unity range endpoint contract differs')
    for v in _list(r['car_type_to_item_index']):_integer(v)
    if r['car_type_to_item_index']!=[1,1,15,98,29,94,105,122,140,153,173,178,222,243,257,275,309,0,337,356,371,390,396]:
        raise ValueError('Source car category index mapping differs')
    methods={(v['owner'],v['signature']) for v in evidence['methods']}
    for f in _list(r['flows']):
        _shape(f,'owner signature steps')
        if type(f['owner']) is not str or type(f['signature']) is not str or (f['owner'],f['signature']) not in methods:raise ValueError('Operation method link differs')
        for step in _list(f['steps']):
            if not isinstance(step,dict):raise ValueError('Operation record differs')
            kind=step.get('kind')
            if kind=='rule':
                _shape(step,'kind action')
                if type(step['action']) is not str:raise ValueError('Operation action differs')
            elif kind=='unit_sphere':
                _shape(step,'kind guard use')
            elif kind in ('integer_range','float_range'):
                _shape(step,'kind lower upper guard use')
                for field in ('lower','upper'):
                    b=step[field];_shape(b,'kind value')
                    if b['kind']=='constant':
                        if kind=='integer_range':_integer(b['value'])
                        else:_number(b['value'])
                    elif b['kind']!='source' or type(b['value']) is not str:raise ValueError('Operation bound differs')
            else:raise ValueError('Unknown acquisition operation')
            if kind!='rule' and (type(step['guard']) is not str or type(step['use']) is not str):raise ValueError('Operation guard differs')
    if r['flows']!=ACQUISITION_FLOWS:raise ValueError('Checked acquisition operation type, bounds, order or comparison differs')

SOURCE_PREFABS = {32825: ('', 74), 39986: ('', 54), 6496: ('', 71), 42570: ('', 60), 12981: ('', 86), 30814: ('', 72), 35111: ('', 76), 39989: ('', 57), 20979: ('', 75), 20649: ('', 92), 30548: ('', 254), 16468: ('', 80), 28749: ('', 66), 26645: ('', 85), 32753: ('', 83), 41097: ('', None), 9085: ('', 62), 9086: ('', 231), 24105: ('', 87), 26970: ('', 88), 35853: ('', 70), 36799: ('', 81), 35272: ('', 82), 7370: ('', 84)}
SPAWNER_BINDINGS = {
155092: [61833, 0.30000001192092896, False, 0, False, [(('prefab', 32825), 0, True, False, [66773, 80443, 83523, 78385]), (('prefab', 39986), 0, False, False, [57440, 57399, 72309, 64524, 88948, 71778, 73569, 84839])]],
131522: [63125, 0.5, False, 0, False, [(('prefab', 6496), 0, True, False, [66184, 69510, 73635, 69235, 54162, 75437]), (('prefab', 42570), 0, False, False, [81152, 70778, 73763])]],
139976: [85141, 0.75, False, 0, False, [(('prefab', 42570), 0, False, False, [68475]), (('prefab', 6496), 0, True, False, [77624]), (('prefab', 12981), 0, True, False, [60482, 87027])]],
155127: [80181, 0.5, False, 0, False, [(('prefab', 6496), 0, True, False, [66422]), (('prefab', 42570), 0, False, False, [62555, 71332, 72239, 69644, 58028, 74037, 74000, 84886, 54509, 67667]), (('prefab', 30814), 0, False, False, [83276, 87363]), (('prefab', 12981), 0, True, False, [54908, 82700, 68049, 51728, 82735])]],
145959: [54817, 1.0, False, 0, False, [(('prefab', 30814), 0, False, False, [63463, 52233, 82185, 53903, 71141, 83493, 58654]), (('prefab', 42570), 0, False, False, [87297, 53076, 66856, 72514, 61254, 50045, 57366, 50927, 58748])]],
156024: [86805, 0.5, False, 0, False, [(('prefab', 35111), 0, True, False, [76282, 64900, 57465, 70396, 64541, 69026, 64603, 51523, 61320, 88055, 58502, 52348, 51354])]],
152647: [59279, 0.4000000059604645, False, 0, False, [(('category', 27), 0, True, False, [58040, 60169, 69651, 68837, 57085, 85581, 77506, 68183, 74329, 71577])]],
152199: [54663, 0.30000001192092896, False, 0, False, [(('category', 24), 0, False, False, [60103, 70575, 50946, 54560, 83810, 69772, 61064, 74403, 54671])]],
144347: [70441, 0.30000001192092896, False, 0, False, [(('category', 24), 0, False, False, [61764, 77450, 87478, 84564, 77806, 58255, 57464, 55875])]],
156794: [79775, 0.30000001192092896, False, 0, False, [(('category', 24), 0, False, False, [85514, 88018, 50653, 51873, 80147, 50869, 88192, 75857, 68528])]],
152085: [84519, 0.5, False, 0, False, [(('category', 36), 0, True, False, [63707, 88971, 71243, 60068, 63841, 59733, 78034, 69859, 66250, 66887, 58098, 53489, 66456, 72562, 71391]), (('prefab', 39989), 0, True, False, [84511, 82387, 81186]), (('prefab', 20979), 0, False, False, [69100, 68356, 80028, 72149, 84398, 57774, 54247, 58940, 61339, 65564, 63376, 66886, 71218]), (('prefab', 6496), 0, True, False, [74073, 80843, 81473, 85080, 55877, 63961, 51266, 58706, 81793, 69862, 63899, 61284])]],
152852: [64283, 0.4000000059604645, False, 0, False, [(('category', 35), 0, True, False, [])]],
143968: [59834, 0.20000000298023224, False, 0, False, [(('category', 39), 0, False, False, [58045, 75373, 85937, 63613, 60660, 78868, 68007, 84953, 62723, 67777, 58479, 52707, 81992, 76363, 86350, 71849, 57313, 85667])]],
152401: [50102, 0.20000000298023224, False, 0, False, [(('category', 35), 0, True, False, [])]],
136189: [77086, 0.20000000298023224, False, 0, False, [(('category', 35), 0, True, False, [])]],
159015: [53593, 0.20000000298023224, False, 0, False, [(('category', 35), 0, True, False, [])]],
132525: [55063, 0.20000000298023224, False, 0, False, [(('category', 35), 0, True, False, [])]],
157727: [64387, 0.20000000298023224, False, 0, False, [(('category', 35), 0, True, False, [])]],
144982: [85175, 0.20000000298023224, False, 0, False, [(('category', 35), 0, True, False, [])]],
148080: [51257, 0.20000000298023224, False, 0, False, [(('category', 35), 0, True, False, [])]],
154174: [58428, 0.20000000298023224, False, 0, False, [(('category', 35), 0, True, False, [])]],
148737: [68583, 0.20000000298023224, False, 0, False, [(('category', 35), 0, True, False, [])]],
137140: [87476, 0.20000000298023224, False, 0, False, [(('category', 35), 0, True, False, [])]],
158935: [55769, 0.20000000298023224, False, 0, False, [(('category', 35), 0, True, False, [])]],
154247: [85614, 0.20000000298023224, False, 0, False, [(('category', 35), 0, True, False, [])]],
141801: [84613, 0.20000000298023224, False, 0, False, [(('category', 35), 0, True, False, [])]],
157379: [83677, 0.20000000298023224, False, 0, False, [(('category', 35), 0, True, False, [])]],
159136: [73371, 0.20000000298023224, False, 0, False, [(('category', 35), 0, True, False, [])]],
145318: [89167, 0.20000000298023224, False, 0, False, [(('category', 35), 0, True, False, [])]],
143532: [83521, 0.20000000298023224, False, 0, False, [(('category', 35), 0, True, False, [])]],
144808: [80501, 0.699999988079071, False, 0, False, [(('category', 59), 0, False, False, [57978, 54795, 77992, 54840, 74013])]],
142945: [72617, 0.30000001192092896, False, 1, False, [(('prefab', 20649), 0, True, False, [70928, 72981, 80546, 51167, 57744, 82135, 60596, 89351])]],
135698: [83565, 0.25, False, 2, False, [(('prefab', 30548), 0, True, False, [78977, 54691, 82950, 52665, 78542, 84177])]],
153352: [87594, 0.5, False, 0, False, [(('category', 47), 0, False, False, [79501, 50082, 68845, 62553]), (('category', 47), 0, True, False, [86761])]],
135774: [86243, 1.0, False, 0, False, [(('prefab', 30814), 0, True, False, [53406, 84535, 85797, 78786, 73479, 84848, 50655, 89503]), (('prefab', 42570), 0, True, False, [89067, 69936, 51177])]],
150475: [76174, 0.6000000238418579, False, 0, False, [(('category', 36), 0, True, False, [54767, 50018, 54379, 83422, 86274])]],
146976: [85611, 0.6000000238418579, False, 0, False, [(('category', 36), 0, True, False, [78136, 84244, 67066, 67640, 65542])]],
144588: [63527, 0.6000000238418579, False, 0, False, [(('category', 36), 0, True, False, [73955, 64381, 76682, 63039, 69784])]],
143922: [75167, 0.30000001192092896, False, 0, False, [(('prefab', 16468), 0, True, False, [85455]), (('category', 0), 0, True, False, [49364])]],
147378: [50406, 0.5, False, 0, False, [(('prefab', 42570), 0, True, False, [57159, 69564, 66752, 89146]), (('prefab', 6496), 0, True, False, [64070, 71881]), (('prefab', 12981), 0, True, False, [86246, 60089, 49579]), (('prefab', 28749), 0, True, False, [49546]), (('prefab', 26645), 0, True, False, [79846, 61353, 51330])]],
145695: [64272, 0.5, False, 0, False, [(('prefab', 42570), 0, True, False, [71438, 62139]), (('prefab', 6496), 0, True, False, [72879, 71130, 81310]), (('prefab', 12981), 0, True, False, [77123, 86312, 50021]), (('prefab', 28749), 0, True, False, [79913, 88705, 50369]), (('prefab', 26645), 0, True, False, [67702, 61408])]],
143142: [87051, 0.5, False, 0, False, [(('prefab', 42570), 0, True, False, [61471, 61434, 59329]), (('prefab', 6496), 0, True, False, [54675, 59263]), (('prefab', 12981), 0, True, False, [76229, 51412]), (('prefab', 28749), 0, True, False, [55248, 53517, 62644]), (('prefab', 26645), 0, True, False, [78096, 81613, 84842, 84918])]],
140022: [77163, 0.5, False, 0, False, [(('category', 24), 0, False, False, [77505, 70083, 89225, 76044, 55404, 89066, 54702])]],
142385: [73198, 0.5, False, 0, False, [(('category', 24), 0, False, False, [57410, 82089, 83215, 65474, 60362])]],
130795: [82164, 0.5, False, 0, False, [(('category', 24), 0, False, False, [53199, 74827, 88452, 56923, 56695, 84910])]],
131645: [81967, 0.550000011920929, False, 0, False, [(('category', 5), 0, False, False, [61517, 69519])]],
134362: [71600, 0.550000011920929, False, 0, False, [(('category', 5), 0, False, False, [50345, 83297, 60086])]],
149222: [58231, 0.4000000059604645, False, 0, False, [(('category', 5), 0, False, False, [66980]), (('category', 5), 0, True, False, [69024])]],
143195: [74787, 0.44999998807907104, False, 0, False, [(('category', 11), 0, False, False, [66940, 83675, 65523, 66386])]],
158837: [86638, 0.44999998807907104, False, 0, False, [(('category', 11), 0, False, False, [62350, 52299, 51444, 56105])]],
148932: [74453, 0.4000000059604645, False, 0, False, [(('category', 8), 0, False, False, [63626, 70921]), (('category', 8), 0, True, False, [71365, 67867])]],
150294: [77507, 0.4000000059604645, False, 0, False, [(('category', 8), 0, False, False, [85197]), (('category', 8), 0, True, False, [52647, 70360])]],
131020: [81477, 0.4000000059604645, False, 0, False, [(('prefab', 32753), 0, True, False, [57497, 84335, 54115, 72266]), (('prefab', 41097), 0, True, False, [63060, 67596])]],
153823: [85490, 0.6499999761581421, False, 0, False, [(('category', 35), 0, True, False, [73076, 63497, 51213, 56570, 69014])]],
137208: [78996, 0.6499999761581421, False, 0, False, [(('category', 35), 0, True, False, [80483, 52915, 51137, 80223])]],
153740: [78124, 0.6499999761581421, False, 0, False, [(('category', 35), 0, True, False, [67378, 89272, 86749, 62738, 67265])]],
153176: [74025, 0.6499999761581421, False, 0, False, [(('category', 35), 0, True, False, [67068, 68591, 80314, 70137])]],
139372: [83536, 0.30000001192092896, False, 0, False, [(('category', 26), 0, True, False, [89355, 63449, 59523])]],
150978: [56968, 0.550000011920929, False, 0, False, [(('prefab', 9085), 0, False, False, [62832])]],
134469: [57449, 0.30000001192092896, False, 0, False, [(('category', 38), 0, True, False, [])]],
144495: [54265, 0.550000011920929, False, 0, False, [(('prefab', 9085), 0, False, False, [71660])]],
155447: [69352, 0.6499999761581421, False, 0, False, [(('category', 35), 0, True, False, [81373, 85375, 84159, 80102, 51725])]],
159356: [66992, 0.6499999761581421, False, 0, False, [(('category', 35), 0, True, False, [81984, 88450, 85839, 72349])]],
153506: [68009, 0.30000001192092896, False, 0, False, [(('category', 51), 0, True, False, [49830, 77093, 65501, 72970, 73307])]],
149796: [53036, 0.30000001192092896, False, 0, False, [(('category', 51), 0, True, False, [52114, 80689, 50403, 88356])]],
152316: [73589, 0.3499999940395355, False, 0, False, [(('category', 59), 0, False, False, [75976, 89165, 80932, 53438, 59437])]],
140091: [82072, 0.30000001192092896, False, 1, False, [(('prefab', 20649), 0, True, False, [56983, 56454, 69360, 73581, 59419, 73868, 54017, 74058])]],
140159: [74546, 0.30000001192092896, False, 0, False, [(('category', 82), 0, False, False, [61667, 62561, 77415, 77029, 53558, 85076, 69422, 58690, 71311, 74749])]],
158497: [84486, 0.6000000238418579, False, 0, False, [(('prefab', 9086), 0, False, False, [64910, 55889, 80020, 87059, 49584, 50478, 67305])]],
135875: [50124, 0.6000000238418579, False, 0, False, [(('category', 77), 0, True, False, [56154, 72292, 52606, 52877, 68776])]],
133894: [54322, 0.5, False, 0, False, [(('prefab', 24105), 0, False, False, [])]],
132473: [66636, 0.5, False, 0, False, [(('prefab', 24105), 0, False, False, [])]],
134881: [81660, 0.5, False, 0, False, [(('prefab', 24105), 0, False, False, [])]],
152604: [78442, 0.5, False, 0, False, [(('prefab', 24105), 0, False, False, [])]],
143849: [61905, 0.5, False, 0, False, [(('prefab', 24105), 0, False, False, [])]],
141037: [66593, 0.5, False, 0, False, [(('prefab', 24105), 0, False, False, [])]],
134594: [89219, 0.5, False, 0, False, [(('prefab', 26970), 0, False, False, [])]],
155658: [73767, 0.5, False, 0, False, [(('prefab', 26970), 0, False, False, [])]],
155735: [63600, 0.5, False, 0, False, [(('prefab', 26970), 0, False, False, [])]],
131807: [67445, 0.5, False, 0, False, [(('prefab', 26970), 0, False, False, [])]],
148462: [77129, 0.5, False, 0, False, [(('prefab', 26970), 0, False, False, [])]],
133453: [52168, 0.5, False, 0, False, [(('prefab', 26970), 0, False, False, [])]],
151325: [81536, 0.5, False, 0, False, [(('prefab', 26970), 0, False, False, [])]],
159388: [84402, 0.5, False, 0, False, [(('prefab', 26970), 0, False, False, [])]],
151878: [63955, 0.5, False, 0, False, [(('prefab', 26970), 0, False, False, [])]],
156241: [74462, 0.5, False, 0, False, [(('prefab', 26970), 0, False, False, [])]],
150554: [68699, 0.5, False, 0, False, [(('prefab', 26970), 0, False, False, [])]],
152271: [72974, 0.5, False, 0, False, [(('prefab', 26970), 0, False, False, [])]],
134832: [86363, 0.5, False, 0, False, [(('prefab', 26970), 0, False, False, [])]],
152756: [66208, 0.5, False, 0, False, [(('prefab', 26970), 0, False, False, [])]],
149070: [80718, 0.5, False, 0, False, [(('prefab', 26970), 0, False, False, [])]],
143444: [49808, 0.5, False, 0, False, [(('prefab', 26970), 0, False, False, [])]],
144153: [71420, 0.5, False, 0, False, [(('prefab', 26970), 0, False, False, [])]],
134567: [67406, 0.5, False, 0, False, [(('prefab', 26970), 0, False, False, [])]],
132539: [54515, 0.5, False, 0, False, [(('prefab', 26970), 0, False, False, [])]],
131865: [61222, 0.5, False, 0, False, [(('prefab', 9085), 0, False, False, [])]],
148411: [67920, 0.5, False, 0, False, [(('prefab', 9085), 0, False, False, [])]],
133089: [81119, 0.5, False, 0, False, [(('prefab', 9085), 0, False, False, [])]],
153078: [76079, 0.5, False, 0, False, [(('prefab', 9085), 0, False, False, [])]],
135175: [87140, 0.5, False, 0, False, [(('prefab', 9085), 0, False, False, [])]],
153070: [87565, 0.5, False, 0, False, [(('prefab', 9085), 0, False, False, [])]],
151379: [79333, 0.5, False, 0, False, [(('prefab', 9085), 0, False, False, [])]],
138654: [85557, 0.5, False, 0, False, [(('prefab', 9085), 0, False, False, [])]],
156146: [84433, 0.5, False, 0, False, [(('prefab', 9085), 0, False, False, [])]],
138632: [81997, 0.5, False, 0, False, [(('prefab', 9085), 0, False, False, [])]],
134371: [58782, 0.5, False, 0, False, [(('category', 26), 0, True, False, [83903, 64276, 67216, 74355])]],
131777: [60002, 0.5, False, 0, False, [(('category', 37), 0, True, False, [65951, 79034, 61247, 89086])]],
142598: [82343, 1.0, False, 0, False, [(('prefab', 42570), 0, True, False, [67221, 82639, 58910, 87700, 58956, 53678, 76031, 76465, 66594, 77550, 79880, 72859, 55631, 75527, 72302, 81842, 53416, 50759, 64828, 67432, 52393, 69132, 86043])]],
147640: [69084, 1.0, False, 0, False, [(('prefab', 30814), 0, True, False, [51203, 65817, 86286, 66377, 85186, 53637, 63865, 68787, 83091, 51618, 69713, 52766, 66348, 50494, 67885, 68479, 64550, 75086, 76360, 63464, 82913, 55077, 68253]), (('prefab', 42570), 0, False, False, [87879])]],
136433: [56935, 0.5, False, 0, False, [(('prefab', 12981), 0, True, False, [81074, 69791, 88538, 81296, 73424]), (('prefab', 35853), 0, True, False, [58909, 71737])]],
152320: [54139, 0.5, False, 0, False, [(('prefab', 12981), 0, True, False, [56756, 80691, 65615, 70112]), (('prefab', 35853), 0, True, False, [51583, 71299])]],
152354: [55828, 0.5, False, 0, False, [(('prefab', 12981), 0, True, False, [62026, 62381, 89188, 89481]), (('prefab', 35853), 0, True, False, [51411, 58177])]],
149997: [53221, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
134150: [72486, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
141763: [58182, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
145394: [67070, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
131269: [59897, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
149343: [88483, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
154365: [62033, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
132040: [87561, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
143201: [89148, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
142584: [49292, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
146500: [87326, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
140319: [84011, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
155187: [65978, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
138169: [62711, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
138563: [73915, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
152202: [64652, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
132607: [62456, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
133602: [71806, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
151471: [65724, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
139581: [59297, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
130933: [54193, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
146077: [72241, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
152232: [53323, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
140250: [88098, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
136758: [87261, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
144321: [70033, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
143910: [55371, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
158529: [71159, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
147544: [80646, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
157221: [78452, 0.5, False, 0, False, [(('prefab', 36799), 0, False, False, [])]],
137563: [59163, 0.5, False, 0, False, [(('prefab', 36799), 0, False, False, [])]],
157472: [73537, 0.5, False, 0, False, [(('prefab', 36799), 0, False, False, [])]],
140306: [84304, 0.5, False, 0, False, [(('prefab', 36799), 0, False, False, [])]],
133988: [53191, 0.5, False, 0, False, [(('prefab', 36799), 0, False, False, [])]],
156824: [61814, 0.5, False, 0, False, [(('prefab', 36799), 0, False, False, [])]],
149279: [78742, 0.5, False, 0, False, [(('prefab', 35272), 0, False, False, [57480, 58289, 80493, 71450, 86786])]],
154875: [84298, 0.5, False, 0, False, [(('prefab', 32753), 0, False, False, [63134, 52009, 63699, 57354, 58762])]],
153360: [64498, 0.5, False, 0, False, [(('prefab', 16468), 0, False, False, [75352, 65320, 50483, 49502])]],
134606: [67662, 0.5, False, 0, False, [(('category', 24), 0, True, False, [])]],
153100: [62024, 0.5, False, 0, False, [(('category', 24), 0, True, False, [])]],
151479: [87605, 0.5, False, 0, False, [(('category', 24), 0, True, False, [])]],
141788: [64506, 0.5, False, 0, False, [(('category', 24), 0, True, False, [])]],
134009: [82476, 0.5, False, 0, False, [(('category', 24), 0, True, False, [])]],
134004: [81855, 0.5, False, 0, False, [(('category', 24), 0, True, False, [])]],
151772: [79307, 0.5, False, 0, False, [(('category', 24), 0, True, False, [])]],
154990: [56942, 0.5, False, 0, False, [(('category', 24), 0, True, False, [])]],
139952: [84617, 0.5, False, 0, False, [(('category', 24), 0, True, False, [])]],
143756: [80149, 0.5, False, 0, False, [(('category', 24), 0, True, False, [])]],
159395: [85111, 0.5, False, 0, False, [(('category', 24), 0, True, False, [])]],
152486: [58825, 0.5, False, 0, False, [(('category', 24), 0, True, False, [])]],
159108: [57327, 0.5, False, 0, False, [(('category', 24), 0, True, False, [])]],
149708: [52730, 0.5, False, 0, False, [(('category', 24), 0, True, False, [])]],
143441: [52806, 0.5, False, 0, False, [(('category', 24), 0, True, False, [])]],
148956: [80363, 0.5, False, 0, False, [(('category', 24), 0, True, False, [])]],
133810: [64808, 0.5, False, 0, False, [(('category', 24), 0, True, False, [])]],
137616: [56828, 0.5, False, 0, False, [(('category', 24), 0, True, False, [])]],
156686: [85844, 0.3499999940395355, False, 0, False, [(('prefab', 7370), 0, True, False, [])]],
151104: [80291, 0.3499999940395355, False, 0, False, [(('prefab', 7370), 0, True, False, [])]],
141085: [79554, 0.3499999940395355, False, 0, False, [(('prefab', 7370), 0, True, False, [])]],
149001: [82025, 0.3499999940395355, False, 0, False, [(('prefab', 7370), 0, True, False, [])]],
156759: [86615, 0.3499999940395355, False, 0, False, [(('prefab', 7370), 0, True, False, [])]],
144858: [85897, 0.3499999940395355, False, 0, False, [(('prefab', 7370), 0, True, False, [])]],
155522: [55043, 0.3499999940395355, False, 0, False, [(('prefab', 7370), 0, True, False, [])]],
151810: [52295, 0.3499999940395355, False, 0, False, [(('prefab', 7370), 0, True, False, [])]],
142649: [72785, 0.3499999940395355, False, 0, False, [(('prefab', 7370), 0, True, False, [])]],
158222: [61674, 0.3499999940395355, False, 0, False, [(('prefab', 7370), 0, True, False, [])]],
135069: [83114, 0.3499999940395355, False, 0, False, [(('prefab', 7370), 0, True, False, [])]],
142695: [73811, 0.3499999940395355, False, 0, False, [(('prefab', 7370), 0, True, False, [])]],
135361: [86220, 0.3499999940395355, False, 0, False, [(('prefab', 7370), 0, True, False, [])]],
149754: [72470, 0.3499999940395355, False, 0, False, [(('prefab', 7370), 0, True, False, [])]],
156893: [80407, 0.3499999940395355, False, 0, False, [(('prefab', 7370), 0, True, False, [])]],
142419: [66696, 0.3499999940395355, False, 0, False, [(('prefab', 7370), 0, True, False, [])]],
152913: [60313, 0.3499999940395355, False, 0, False, [(('prefab', 7370), 0, True, False, [])]],
156419: [69523, 0.3499999940395355, False, 0, False, [(('prefab', 7370), 0, True, False, [])]],
130875: [81270, 0.3499999940395355, False, 0, False, [(('prefab', 7370), 0, True, False, [])]],
146654: [87252, 0.3499999940395355, False, 0, False, [(('prefab', 7370), 0, True, False, [])]],
154127: [62205, 0.3499999940395355, False, 0, False, [(('prefab', 7370), 0, True, False, [])]],
135643: [65779, 0.3499999940395355, False, 0, False, [(('prefab', 7370), 0, True, False, [])]],
152942: [59999, 0.3499999940395355, False, 0, False, [(('prefab', 7370), 0, True, False, [])]],
158813: [76413, 0.3499999940395355, False, 0, False, [(('prefab', 7370), 0, True, False, [])]],
139621: [66403, 0.3499999940395355, False, 0, False, [(('prefab', 7370), 0, True, False, [])]],
136713: [66628, 0.3499999940395355, False, 0, False, [(('prefab', 7370), 0, True, False, [])]],
156175: [56506, 0.3499999940395355, False, 0, False, [(('prefab', 7370), 0, True, False, [])]],
140582: [51642, 0.3499999940395355, False, 0, False, [(('prefab', 7370), 0, True, False, [])]],
138065: [82215, 0.3499999940395355, False, 0, False, [(('prefab', 7370), 0, True, False, [])]],
135847: [75331, 0.3499999940395355, False, 0, False, [(('prefab', 7370), 0, True, False, [])]],
154094: [67717, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
138584: [60599, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
146062: [72644, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
142303: [72677, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
132513: [56877, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
138338: [77451, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
143718: [74275, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
135235: [62795, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
151295: [79028, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
144008: [59157, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
153794: [63704, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
135332: [55367, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
132925: [49781, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
136321: [53654, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
136599: [87618, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
149893: [59159, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
133770: [59558, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
148179: [62767, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
134021: [53935, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
138866: [89458, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
150213: [80846, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
137377: [49490, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
148616: [61818, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
148291: [86646, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
150601: [71153, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
136148: [74986, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
132645: [57348, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
154037: [84377, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
137393: [73369, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
141383: [51866, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
140922: [85998, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
153038: [71696, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
149590: [63252, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
157508: [80839, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
133298: [66277, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
157793: [71706, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
144300: [57956, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
140956: [75241, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
145078: [73883, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
144514: [80850, 0.20000000298023224, False, 0, False, [(('prefab', 6496), 0, True, False, [])]],
148891: [68271, 0.4000000059604645, False, 0, False, [(('category', 82), 0, False, False, [88080, 67166, 49538])]],
151014: [80774, 0.4000000059604645, False, 0, False, [(('category', 35), 7, False, False, [])]],
143615: [51719, 0.4000000059604645, False, 0, False, [(('category', 35), 7, False, False, [])]],
136463: [57701, 0.4000000059604645, False, 0, False, [(('category', 35), 7, False, False, [])]],
154073: [64030, 0.4000000059604645, False, 0, False, [(('category', 35), 7, False, False, [])]],
158683: [78990, 0.4000000059604645, False, 0, False, [(('category', 35), 7, False, False, [])]],
153893: [85822, 0.4000000059604645, False, 0, False, [(('category', 35), 7, False, False, [])]],
137745: [68575, 0.4000000059604645, False, 0, False, [(('category', 35), 7, False, False, [])]],
151995: [66210, 0.4000000059604645, False, 0, False, [(('category', 35), 7, False, False, [])]],
135598: [85228, 0.4000000059604645, False, 0, False, [(('category', 35), 7, False, False, [])]],
130805: [66831, 0.4000000059604645, False, 0, False, [(('category', 35), 7, False, False, [])]],
155482: [60208, 0.4000000059604645, False, 0, False, [(('category', 35), 7, False, False, [])]],
155305: [89466, 0.4000000059604645, False, 0, False, [(('category', 35), 7, False, False, [])]],
143582: [75427, 0.4000000059604645, False, 0, False, [(('category', 35), 7, False, False, [])]],
156675: [59359, 0.4000000059604645, False, 0, False, [(('category', 35), 7, False, False, [])]],
131215: [55579, 0.4000000059604645, False, 0, False, [(('category', 35), 7, False, False, [])]],
159227: [63014, 0.4000000059604645, False, 0, False, [(('category', 35), 7, False, False, [])]],
140073: [75263, 0.4000000059604645, False, 0, False, [(('category', 35), 7, False, False, [])]],
140141: [70361, 0.4000000059604645, False, 0, False, [(('category', 35), 7, False, False, [])]],
151054: [75361, 0.4000000059604645, False, 0, False, [(('category', 35), 7, False, False, [])]],
149896: [59634, 0.4000000059604645, False, 0, False, [(('category', 35), 7, False, False, [])]],
137792: [62817, 0.4000000059604645, False, 0, False, [(('category', 35), 7, False, False, [])]],
153493: [61946, 0.4000000059604645, False, 0, False, [(('category', 35), 7, False, False, [])]],
140991: [62840, 0.4000000059604645, False, 0, False, [(('category', 35), 7, False, False, [])]],
139566: [74970, 0.4000000059604645, False, 0, False, [(('category', 35), 7, False, False, [])]],
131889: [62209, 0.4000000059604645, False, 0, False, [(('category', 35), 7, False, False, [])]],
142600: [63850, 0.4000000059604645, False, 0, False, [(('category', 35), 7, False, False, [])]],
142749: [85814, 0.4000000059604645, False, 0, False, [(('category', 35), 7, False, False, [])]],
153239: [82117, 0.4000000059604645, False, 0, False, [(('category', 35), 7, False, False, [])]],
149396: [82199, 0.4000000059604645, False, 0, False, [(('category', 35), 7, False, False, [])]],
138457: [61628, 0.4000000059604645, False, 0, False, [(('category', 35), 7, False, False, [])]],
151905: [78743, 0.30000001192092896, False, 0, False, [(('category', 2), 0, False, False, [66306, 73235, 81985, 84545])]],
148855: [57196, 0.4000000059604645, False, 0, False, [(('category', 2), 0, False, False, [80958, 82827, 68790, 50113])]],
156815: [76090, 0.5, False, 0, False, [(('category', 2), 0, False, False, [74595, 59136, 52503, 58457])]],
142703: [56313, 0.5, False, 0, False, [(('prefab', 42570), 0, True, False, [66486, 68000, 58068])]],
131517: [80534, 0.5, False, 0, False, [(('prefab', 42570), 0, True, False, [71709, 65399, 74026])]],
131910: [72270, 0.5, False, 0, False, [(('prefab', 42570), 0, True, False, [84251, 72395, 64426])]],
131490: [60168, 0.30000001192092896, False, 0, False, [(('category', 35), 0, False, False, [])]],
156814: [80382, 0.30000001192092896, False, 0, False, [(('category', 35), 0, False, False, [])]],
136246: [56260, 0.30000001192092896, False, 0, False, [(('category', 35), 0, False, False, [])]],
153963: [63999, 0.30000001192092896, False, 0, False, [(('category', 35), 0, False, False, [])]],
154490: [60231, 0.30000001192092896, False, 0, False, [(('category', 35), 0, False, False, [])]],
158555: [49259, 0.30000001192092896, False, 0, False, [(('category', 35), 0, False, False, [])]],
147609: [85701, 0.30000001192092896, False, 0, False, [(('category', 35), 0, False, False, [])]],
139150: [76581, 0.30000001192092896, False, 0, False, [(('category', 35), 0, False, False, [])]],
154895: [58362, 0.20000000298023224, False, 0, False, [(('category', 25), 0, True, False, [])]],
143647: [65683, 0.20000000298023224, False, 0, False, [(('category', 25), 0, True, False, [])]],
139312: [57318, 0.20000000298023224, False, 0, False, [(('category', 25), 0, True, False, [])]],
139214: [60059, 0.20000000298023224, False, 0, False, [(('category', 25), 0, True, False, [])]],
147519: [80624, 0.20000000298023224, False, 0, False, [(('category', 25), 0, True, False, [])]],
153824: [54614, 0.20000000298023224, False, 0, False, [(('category', 25), 0, True, False, [])]],
137672: [53505, 0.20000000298023224, False, 0, False, [(('category', 25), 0, True, False, [])]],
153505: [63307, 0.20000000298023224, False, 0, False, [(('category', 25), 0, True, False, [])]],
157398: [86369, 0.20000000298023224, False, 0, False, [(('category', 25), 0, True, False, [])]],
149671: [56517, 0.20000000298023224, False, 0, False, [(('category', 25), 0, True, False, [])]],
136219: [57474, 0.20000000298023224, False, 0, False, [(('category', 25), 0, True, False, [])]],
140191: [62507, 0.20000000298023224, False, 0, False, [(('category', 25), 0, True, False, [])]],
150410: [50812, 0.4000000059604645, False, 0, False, [(('category', 39), 0, False, False, [70259, 50275, 64108, 88127, 58458, 59573, 52551, 84316, 53573, 86297, 86509, 74802, 67617, 64829, 54458, 63799, 51950, 53501])]],
136300: [64795, 0.3499999940395355, False, 0, False, [(('category', 59), 0, False, False, [64354, 69649, 51438, 71593, 84227])]],
132674: [70248, 0.30000001192092896, False, 1, False, [(('prefab', 20649), 0, True, False, [58043, 54476, 86222, 51198])]],
152360: [73946, 0.30000001192092896, False, 0, False, [(('category', 88), 0, True, False, [54377, 55221, 79864, 64471])]],
159041: [51632, 0.30000001192092896, False, 0, False, [(('category', 88), 0, True, False, [58030, 79596, 82277, 73612])]],
154385: [85449, 0.30000001192092896, False, 0, False, [(('category', 88), 0, True, False, [60539, 51983, 85570, 51673])]],
148893: [78308, 0.44999998807907104, False, 0, False, [(('category', 63), 0, False, False, [73357, 76518, 57524, 56293, 84845, 80411, 69391, 60384, 71121, 58972])]],
156861: [62528, 0.6000000238418579, False, 0, False, [(('prefab', 9086), 0, False, False, [56368, 65478, 86235, 66706, 88076])]],
155859: [74139, 0.6000000238418579, False, 0, False, [(('prefab', 9086), 0, True, False, [55447, 69514, 64927, 69676, 65384])]],
145390: [81868, 0.25, False, 0, False, [(('prefab', 30548), 0, True, False, [80794, 81724, 79669, 83853, 54738, 65132])]],
141891: [67272, 0.6000000238418579, False, 0, False, [(('category', 77), 0, True, False, [74340, 84753, 63048, 63737, 77374])]],
}

BOX_TABLE = [{'selection': {'category_id': 37, 'kind': 'category', 'name': 'engine'}, 'threshold': 0.5, 'weight': 0.5},
 {'selection': {'category_id': 26, 'kind': 'category', 'name': 'coolant'}, 'threshold': 1.0, 'weight': 0.5},
 {'selection': {'category_id': 41, 'kind': 'category', 'name': 'buscoolant'},
  'threshold': 1.25,
  'weight': 0.25},
 {'selection': {'category_id': 24, 'kind': 'category', 'name': 'wheelstuff'},
  'threshold': 1.75,
  'weight': 0.5},
 {'selection': {'category_id': 66, 'kind': 'category', 'name': 'toys'}, 'threshold': 2.25, 'weight': 0.5},
 {'selection': {'category_id': 71, 'kind': 'category', 'name': 'balls'}, 'threshold': 2.75, 'weight': 0.5},
 {'selection': {'category_id': 68, 'kind': 'category', 'name': 'ammo'}, 'threshold': 3.25, 'weight': 0.5},
 {'selection': {'category_id': 76, 'kind': 'category', 'name': 'firearms'}, 'threshold': 3.75, 'weight': 0.5},
 {'selection': {'category_id': 70, 'kind': 'category', 'name': 'food'}, 'threshold': 4.25, 'weight': 0.5},
 {'selection': {'category_id': 69, 'kind': 'category', 'name': 'bodypart'}, 'threshold': 4.75, 'weight': 0.5},
 {'selection': {'category_id': 57, 'kind': 'category', 'name': 'biketrunk'},
  'threshold': 5.25,
  'weight': 0.5},
 {'selection': {'category_id': 83, 'kind': 'category', 'name': 'cans'}, 'threshold': 6.25, 'weight': 1.0},
 {'selection': {'category_id': 84, 'kind': 'category', 'name': 'wearables'},
  'threshold': 7.25,
  'weight': 1.0},
 {'selection': {'category_id': 25, 'kind': 'category', 'name': 'attachable'},
  'threshold': 8.25,
  'weight': 1.0},
 {'selection': {'category_id': 85, 'kind': 'category', 'name': 'inbox'}, 'threshold': 10.25, 'weight': 2.0},
 {'selection': {'category_id': 30, 'kind': 'category', 'name': 'headlight'},
  'threshold': 11.25,
  'weight': 1.0},
 {'selection': {'category_id': 86, 'kind': 'category', 'name': 'bikeTanks'},
  'threshold': 12.25,
  'weight': 1.0}]
UNDERGROUND_TABLES = [[{'selection': {'category_id': 37, 'kind': 'category', 'name': 'engine'}, 'threshold': 0.5, 'weight': 0.5},
  {'selection': {'category_id': 26, 'kind': 'category', 'name': 'coolant'}, 'threshold': 1.0, 'weight': 0.5},
  {'selection': {'category_id': 41, 'kind': 'category', 'name': 'buscoolant'},
   'threshold': 1.25,
   'weight': 0.25},
  {'selection': {'category_id': 24, 'kind': 'category', 'name': 'wheelstuff'},
   'threshold': 1.75,
   'weight': 0.5},
  {'selection': {'category_id': 68, 'kind': 'category', 'name': 'ammo'}, 'threshold': 2.25, 'weight': 0.5},
  {'selection': {'category_id': 76, 'kind': 'category', 'name': 'firearms'},
   'threshold': 2.75,
   'weight': 0.5},
  {'selection': {'category_id': 70, 'kind': 'category', 'name': 'food'}, 'threshold': 3.25, 'weight': 0.5},
  {'selection': {'category_id': 71, 'kind': 'category', 'name': 'balls'}, 'threshold': 3.75, 'weight': 0.5},
  {'selection': {'category_id': 66, 'kind': 'category', 'name': 'toys'}, 'threshold': 4.25, 'weight': 0.5},
  {'selection': {'category_id': 69, 'kind': 'category', 'name': 'bodypart'},
   'threshold': 4.75,
   'weight': 0.5},
  {'selection': {'category_id': 57, 'kind': 'category', 'name': 'biketrunk'},
   'threshold': 5.25,
   'weight': 0.5},
  {'selection': {'category_id': 83, 'kind': 'category', 'name': 'cans'}, 'threshold': 6.25, 'weight': 1.0},
  {'selection': {'category_id': 84, 'kind': 'category', 'name': 'wearables'},
   'threshold': 7.25,
   'weight': 1.0},
  {'selection': {'category_id': 25, 'kind': 'category', 'name': 'attachable'},
   'threshold': 8.25,
   'weight': 1.0},
  {'selection': {'category_id': 85, 'kind': 'category', 'name': 'inbox'}, 'threshold': 10.25, 'weight': 2.0},
  {'selection': {'category_id': 30, 'kind': 'category', 'name': 'headlight'},
   'threshold': 11.25,
   'weight': 1.0},
  {'selection': {'category_id': 86, 'kind': 'category', 'name': 'bikeTanks'},
   'threshold': 12.25,
   'weight': 1.0},
  {'selection': {'item_id': 62,
                 'kind': 'item',
                 'name': 'box01',
                 'prototype': {'file': 'sharedassets1.assets', 'path_id': 51722}},
   'threshold': 13.25,
   'weight': 1.0}],
 [{'selection': {'category_id': 37, 'kind': 'category', 'name': 'engine'}, 'threshold': 0.5, 'weight': 0.5},
  {'selection': {'category_id': 26, 'kind': 'category', 'name': 'coolant'}, 'threshold': 1.0, 'weight': 0.5},
  {'selection': {'category_id': 41, 'kind': 'category', 'name': 'buscoolant'},
   'threshold': 1.25,
   'weight': 0.25},
  {'selection': {'category_id': 24, 'kind': 'category', 'name': 'wheelstuff'},
   'threshold': 1.75,
   'weight': 0.5},
  {'selection': {'category_id': 68, 'kind': 'category', 'name': 'ammo'}, 'threshold': 2.25, 'weight': 0.5},
  {'selection': {'category_id': 76, 'kind': 'category', 'name': 'firearms'},
   'threshold': 2.75,
   'weight': 0.5},
  {'selection': {'category_id': 70, 'kind': 'category', 'name': 'food'}, 'threshold': 3.25, 'weight': 0.5},
  {'selection': {'category_id': 71, 'kind': 'category', 'name': 'balls'}, 'threshold': 3.75, 'weight': 0.5},
  {'selection': {'category_id': 66, 'kind': 'category', 'name': 'toys'}, 'threshold': 4.25, 'weight': 0.5},
  {'selection': {'category_id': 69, 'kind': 'category', 'name': 'bodypart'},
   'threshold': 4.75,
   'weight': 0.5},
  {'selection': {'category_id': 57, 'kind': 'category', 'name': 'biketrunk'},
   'threshold': 5.25,
   'weight': 0.5},
  {'selection': {'category_id': 83, 'kind': 'category', 'name': 'cans'}, 'threshold': 6.25, 'weight': 1.0},
  {'selection': {'category_id': 84, 'kind': 'category', 'name': 'wearables'},
   'threshold': 7.25,
   'weight': 1.0},
  {'selection': {'category_id': 25, 'kind': 'category', 'name': 'attachable'},
   'threshold': 8.25,
   'weight': 1.0},
  {'selection': {'category_id': 85, 'kind': 'category', 'name': 'inbox'}, 'threshold': 10.25, 'weight': 2.0},
  {'selection': {'category_id': 30, 'kind': 'category', 'name': 'headlight'},
   'threshold': 11.25,
   'weight': 1.0},
  {'selection': {'category_id': 86, 'kind': 'category', 'name': 'bikeTanks'},
   'threshold': 12.25,
   'weight': 1.0},
  {'selection': {'item_id': 62,
                 'kind': 'item',
                 'name': 'box01',
                 'prototype': {'file': 'sharedassets1.assets', 'path_id': 51722}},
   'threshold': 13.25,
   'weight': 1.0}],
 [{'selection': {'category_id': 37, 'kind': 'category', 'name': 'engine'}, 'threshold': 0.5, 'weight': 0.5},
  {'selection': {'category_id': 26, 'kind': 'category', 'name': 'coolant'}, 'threshold': 1.0, 'weight': 0.5},
  {'selection': {'category_id': 41, 'kind': 'category', 'name': 'buscoolant'},
   'threshold': 1.25,
   'weight': 0.25},
  {'selection': {'category_id': 24, 'kind': 'category', 'name': 'wheelstuff'},
   'threshold': 1.75,
   'weight': 0.5},
  {'selection': {'category_id': 68, 'kind': 'category', 'name': 'ammo'}, 'threshold': 2.25, 'weight': 0.5},
  {'selection': {'category_id': 76, 'kind': 'category', 'name': 'firearms'},
   'threshold': 2.75,
   'weight': 0.5},
  {'selection': {'category_id': 70, 'kind': 'category', 'name': 'food'}, 'threshold': 3.25, 'weight': 0.5},
  {'selection': {'category_id': 71, 'kind': 'category', 'name': 'balls'}, 'threshold': 3.75, 'weight': 0.5},
  {'selection': {'category_id': 66, 'kind': 'category', 'name': 'toys'}, 'threshold': 4.25, 'weight': 0.5},
  {'selection': {'category_id': 69, 'kind': 'category', 'name': 'bodypart'},
   'threshold': 4.75,
   'weight': 0.5},
  {'selection': {'category_id': 57, 'kind': 'category', 'name': 'biketrunk'},
   'threshold': 5.25,
   'weight': 0.5},
  {'selection': {'category_id': 83, 'kind': 'category', 'name': 'cans'}, 'threshold': 6.25, 'weight': 1.0},
  {'selection': {'category_id': 84, 'kind': 'category', 'name': 'wearables'},
   'threshold': 7.25,
   'weight': 1.0},
  {'selection': {'category_id': 25, 'kind': 'category', 'name': 'attachable'},
   'threshold': 8.25,
   'weight': 1.0},
  {'selection': {'category_id': 85, 'kind': 'category', 'name': 'inbox'}, 'threshold': 10.25, 'weight': 2.0},
  {'selection': {'category_id': 30, 'kind': 'category', 'name': 'headlight'},
   'threshold': 11.25,
   'weight': 1.0},
  {'selection': {'category_id': 86, 'kind': 'category', 'name': 'bikeTanks'},
   'threshold': 12.25,
   'weight': 1.0},
  {'selection': {'item_id': 62,
                 'kind': 'item',
                 'name': 'box01',
                 'prototype': {'file': 'sharedassets1.assets', 'path_id': 51722}},
   'threshold': 13.25,
   'weight': 1.0}]]

CHECKED_POSES = [{'coordinate_space': 'source_parent_local',
  'parent': None,
  'source': {'file': 'sharedassets1.assets', 'path_id': 51722},
  'transform': {'rotation': [-0.0, -0.0, 0.0, 1.0],
                'scale': [1.0, 1.0, 1.0],
                'translation': [0.0, 0.0, -0.0]}},
 {'coordinate_space': 'source_parent_local',
  'parent': {'file': 'level1', 'path_id': 21481},
  'source': {'file': 'level1', 'path_id': 21449},
  'transform': {'rotation': [-0.0, -1.0, 0.0, 0.0],
                'scale': [1.0, 1.0, 1.0],
                'translation': [1.4539967775344849, 6.00600004196167, 14.700002670288086]}},
 {'coordinate_space': 'source_parent_local',
  'parent': {'file': 'level1', 'path_id': 21655},
  'source': {'file': 'level1', 'path_id': 33859},
  'transform': {'rotation': [0.7058812379837036,
                             0.7058789730072021,
                             0.04163286089897156,
                             0.04163298010826111],
                'scale': [100.00006103515625, 100.00003051757812, 100.00003051757812],
                'translation': [-0.15660084784030914, -0.27971217036247253, -0.5606002807617188]}},
 {'coordinate_space': 'source_parent_local',
  'parent': {'file': 'level1', 'path_id': 21481},
  'source': {'file': 'level1', 'path_id': 21655},
  'transform': {'rotation': [0.0, 0.0, -0.7071056962013245, 0.7071078419685364],
                'scale': [1.0, 1.0, 1.0],
                'translation': [1.4890013933181763, 2.002000093460083, 11.198002815246582]}},
 {'coordinate_space': 'source_parent_local',
  'parent': {'file': 'level1', 'path_id': 21567},
  'source': {'file': 'level1', 'path_id': 33862},
  'transform': {'rotation': [0.0, 0.058877840638160706, -0.998265266418457, 9.126959810146218e-08],
                'scale': [100.00015258789062, 100.0001449584961, 100.0001220703125],
                'translation': [0.3570023775100708, 0.35050010681152344, -1.0168991088867188]}},
 {'coordinate_space': 'source_parent_local',
  'parent': {'file': 'level1', 'path_id': 21481},
  'source': {'file': 'level1', 'path_id': 21567},
  'transform': {'rotation': [0.0, -1.0, -0.0, 0.0],
                'scale': [1.0, 1.0, 1.0],
                'translation': [1.565999984741211, 1.8350000381469727, 9.581000328063965]}}]
HOLIDAY_STARTUP = {'active': True,
 'body': {'file': 'level1', 'path_id': 50268},
 'buried': False,
 'car': None,
 'end': [11, 15],
 'fixed_in_car': 0,
 'inclusive': True,
 'item_id': 92,
 'loaded': False,
 'local_calendar': True,
 'manual_start': True,
 'multiplayer': False,
 'new_game_only': True,
 'pickup': {'file': 'level1', 'path_id': 80624},
 'pose': {'coordinate_space': 'source_parent_local',
          'parent': {'file': 'level1', 'path_id': 21481},
          'source': {'file': 'level1', 'path_id': 21449},
          'transform': {'rotation': [-0.0, -1.0, 0.0, 0.0],
                        'scale': [1.0, 1.0, 1.0],
                        'translation': [1.4539967775344849, 6.00600004196167, 14.700002670288086]}},
 'prototype': {'file': 'sharedassets1.assets', 'path_id': 63259},
 'save': {'file': 'level1', 'path_id': 72331},
 'source': {'file': 'level1', 'path_id': 82},
 'start': [10, 15],
 'started': False,
 'terrain': {'file': 'level1', 'path_id': 64898},
 'wearable': {'file': 'level1', 'path_id': 79857}}

SHOVEL_CONTACT_FIELDS = {'force':'force','damage':'damage','damage_tank':'damageTank','hurt_food':'hurtFood','all_particles':'allparticle','blood':'blood','sand':'sand','plant':'plant','wood':'wood','spark':'spark'}
SHOVEL_CONTACT = {'force':20000.,'damage':40.,'damage_tank':12.,'hurt_food':True,'all_particles':0.009999999776482582,'blood':.5,'sand':2.,'plant':1.,'wood':.5,'spark':.5}
