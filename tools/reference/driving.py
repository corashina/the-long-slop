"""Driving controls and wheel records from checked build 11140890 sources."""
import hashlib
import re
from .source import key, source_id

ASSEMBLY_SHA256 = '9919a9eeddcce23bb4ff86e890437f32b4a3ef3c10af2efe3b8d889d894dd62e'
DECOMPILED_SHA256 = '22ddc62b91954205d90ff513232f39becebbc58a71b718685075f91f64b1f3ee'
ACTIONS = {'clutch':'clutch', 'shift_up':'shiftup', 'shift_down':'shiftdown', 'reverse':'gearr',
    'neutral':'gearn', **{f'gear{i}':f'gear{i}' for i in range(1,7)}}
KEY_CODES = {'LeftControl':'ControlLeft', 'RightControl':'ControlRight', 'UpArrow':'ArrowUp',
    'DownArrow':'ArrowDown', 'KeypadPlus':'NumpadAdd', 'KeypadMinus':'NumpadSubtract',
    'R':'KeyR', 'N':'KeyN', **{f'Alpha{i}':f'Digit{i}' for i in range(7)},
    **{f'Keypad{i}':f'Numpad{i}' for i in range(7)}}


def method(text, declaration):
    start = text.index(declaration)
    opening = text.index('{', start)
    depth = 1; end = opening + 1
    while depth:
        if text[end] == '{':depth += 1
        elif text[end] == '}':depth -= 1
        end += 1
    body = text[start:end]
    return body, {'declaration':declaration, 'first_line':text.count('\n',0,start)+1,
        'last_line':text.count('\n',0,end)+1, 'sha256':hashlib.sha256(body.encode()).hexdigest()}


def checked_methods(source):
    from .library.reader import checked
    import json
    library = source.library
    assembly = library.snapshot.root / 'TheLongDrive_Data/Managed/Assembly-CSharp.dll'
    digest = hashlib.sha256(assembly.read_bytes()).hexdigest()
    if digest != ASSEMBLY_SHA256:raise ValueError('Driving source assembly differs')
    manifest = json.loads((library.records_root/'managed/manifest.json').read_bytes())
    record = next(r for r in manifest['files'] if r['file']=='managed/Assembly-CSharp.cs')
    if record['sha256'] != DECOMPILED_SHA256:raise ValueError('Driving managed source receipt differs')
    text = checked(library.records_root,record).decode('utf-8-sig')
    declarations = ['public void DriveStuff()', 'private void InitDefault()', 'public void RefreshSettings()',
        'private void LoadSettings()', 'public IEnumerator DoReset(bool _force)',
        'public void FStart()', 'public void RefreshMenu()', 'public void ResetSettings(bool _force)',
        'public void ResetCustomRadioPath()', 'public void OnChangedAnything()',
        'public void ResetToDefault()', 'public void WheelSize()', 'public float GetRadius()',
        'public bool IsAutoShift()', 'public void ShiftTo(int pg, bool _tryanyway = false, bool multi = false)',
        'public void Shifted(bool playSoundAndAnimation = true)']
    # Scope declarations that also occur on unrelated classes.
    scopes = {'public void RefreshSettings()':'public class settingsscript :',
        'private void LoadSettings()':'public class settingsscript :',
        'public void OnChangedAnything()':'public class settingsscript :',
        'public void FStart()':'public class settingsscript :',
        'public void RefreshMenu()':'public class settingsscript :',
        'public void ResetToDefault()':'public class togglesetting :'}
    bodies={}; evidence=[]
    for declaration in declarations:
        scoped = text[text.index(scopes[declaration]):] if declaration in scopes else text
        body, entry = method(scoped,declaration)
        offset = len(text)-len(scoped)
        entry['first_line'] += text.count('\n',0,offset)
        entry['last_line'] += text.count('\n',0,offset)
        bodies[declaration] = body;evidence.append(entry)
    if 'private bool playTransmissionSoundsInAutomatic;' not in text:
        raise ValueError('Driving shift sound default differs')
    if re.search(r'playTransmissionSoundsInAutomatic\s*=', text):
        raise ValueError('Driving shift sound default has an assignment')
    minimum = re.search(r'public static float minWheelMotorTorque = ([\d.]+)f;',text)
    if minimum is None:raise ValueError('Missing wheel motor torque constant')
    return bodies, evidence, float(minimum[1]), text


LEVER_FIELDS = {'seconds':'TransmissionLerp', 'stage1':'transmissionStage1', 'stage2':'transmissionStage2'}
LEVER_LERPS = [f'Transmission.{part} = {kind}.Lerp(Transmission.{part}, gears[{gear} + 1].{target}.{part}, transMissionLerp3);'
    for gear,target in (('lastgear','Path'),('gear','Path'),('gear','Pos'))
    for part,kind in (('position','Vector3'),('rotation','Quaternion'))]


def lever_constants(text):
    """carscript lever timing from the private field initializers."""
    values = {}
    for name,field in LEVER_FIELDS.items():
        match = re.search(rf'private float {field} = ([\d.]+)f;', text)
        if match is None:raise ValueError(f'Missing lever constant: {field}')
        values[name] = float(match[1])
    if any(line not in text for line in LEVER_LERPS):raise ValueError('Lever lerp source differs')
    return values


def recover_lever(source, car_reader, car, root, node_ids, constants):
    """carscript.Transmission and gear Path/Pos, posed in the lever parent space.
    The source writes world poses. The runtime writes the node local pose."""
    from .collision import world_pose
    from .particles import relative_pose
    lever = source.resolve(car_reader, car['Transmission'])
    if lever is None:return None
    def inside(reader):
        while reader is not None:
            if key(reader)==key(root):return True
            reader = source.resolve(reader, source.read(reader)['m_Father'])
        return False
    parent = source.resolve(lever, source.read(lever)['m_Father'])
    if parent is None or not inside(parent):raise ValueError(f'Lever parent is outside the car: {source_id(lever)}')
    if key(lever) not in node_ids:raise ValueError(f'Missing lever node: {key(lever)}')
    scale = world_pose(source, parent)['scale']
    if max(scale)-min(scale) > 1e-5 * max(abs(v) for v in scale):raise ValueError(f'Lever parent scale is not uniform: {source_id(lever)}')
    def pose(pointer):
        reader = source.resolve(car_reader, pointer)
        if reader is None or not inside(reader):raise ValueError(f'Lever gear pose is outside the car: {source_id(car_reader)}')
        value = relative_pose(source, reader, parent)
        value['translation'] = [v/scale[0] for v in value['translation']]
        return value
    return {'node':node_ids[key(lever)], 'gears':[{'path':pose(g['Path']), 'pos':pose(g['Pos'])} for g in car['gears']],
        'seconds':constants['seconds'], 'stages':[constants['stage1'], constants['stage2']]}


def recover_driving(source, car_readers, wheel_readers, node_ids):
    """Resolve selected cars and wheel owners; fail on missing records or links.
    `node_ids` maps source Transform keys to manifest node ids."""
    bodies, methods, minimum, text = checked_methods(source)
    constants = lever_constants(text)
    drive = bodies['public void DriveStuff()']
    rates = [float(value) for value in re.findall(r'B(?:Gas|Brake|Clutch) = Mathf.MoveTowards\([^;]+Time.deltaTime \* ([\d.]+)f\);',drive)]
    if len(rates)!=3 or len(set(rates))!=1:raise ValueError('Driving pedal rates differ')
    settings_reader = source.get('level0',16865)
    if source.class_name(settings_reader)!='settingsscript':raise ValueError('Driving settings owner differs')
    settings = source.read(settings_reader)
    toggles=[]
    for index in (0,1,7):
        reader=source.resolve(settings_reader,settings['tabControls']['toggles'][index])
        if reader is None or source.class_name(reader)!='togglesetting':raise ValueError('Unresolved driving assist toggle')
        value=source.read(reader)
        children=[source.resolve(reader,p) for p in value['childToggles']]
        if any(r is None for r in children):raise ValueError('Unresolved driving assist child')
        toggle=source.resolve(reader,value['toggle'])
        if toggle is None:raise ValueError('Unresolved driving toggle state')
        toggles.append({'source':source_id(reader),'default':bool(value['def']),
            'serialized_is_on':bool(source.read(toggle)['m_IsOn']),
            'reset_child_on_enable':bool(value['resetChildOnEnable']),
            'child_sources':[source_id(r) for r in children]})
    if toggles[0]['child_sources'] != [toggles[1]['source']]:raise ValueError('Driving assist dependency differs')
    if 'OnChangedAnything();' not in bodies['public void ResetCustomRadioPath()'] or 'RefreshSettings();' not in bodies['public void OnChangedAnything()']:
        raise ValueError('Driving startup refresh chain differs')
    refresh=bodies['public void RefreshSettings()']
    if 'S.BAutoShift = tabControls.toggles[1].toggle.isOn && S.BAutoClutch;' not in refresh:
        raise ValueError('Driving assist refresh differs')
    slider=source.resolve(settings_reader,settings['tabControls']['sliders'][7])
    if slider is None:raise ValueError('Unresolved steering slider')
    steering=source.read(slider)
    steering_rate=steering['defValue']*(steering['valueMultipler'] if steering['valueMultipler'] else 1.)
    enum = re.search(r'public enum IN\s*\{([^}]+)\}',text)
    if enum is None:raise ValueError('Missing driving input enum')
    names=[name.strip() for name in enum[1].split(',') if name.strip()]
    defaults=bodies['private void InitDefault()']
    bindings=[];binding_evidence=[]
    for action,name in ACTIONS.items():
        index=names.index(name)
        keys=re.findall(rf'defaultInput\[{index}\]\.keys\[\d+\]\.Set\(KeyCode\.(\w+)\);',defaults)
        if not keys:raise ValueError(f'Unresolved driving input: {name}')
        bindings.append({'action':action,'codes':[KEY_CODES[value] for value in keys]})
        binding_evidence.append({'action':action,'source_input':name,'index':index,'keys':keys})
    global_record={'source':source_id(settings_reader),'pedal_rate':rates[0], 'force_manual':toggles[2]['default'],
        'min_motor_torque_nm':minimum,'default_assists':{'automatic_clutch':toggles[0]['default'],
            'automatic_gears':toggles[1]['default'] and toggles[0]['default']},'bindings':bindings}
    main_reader=source.get('level1',64919)
    if source.class_name(main_reader)!='mainscript':raise ValueError('Driving hub owner differs')
    hubs=source.read(main_reader)['kerekagyProperties']
    cars={};vehicles=[];owners={};graphics={}
    for reader in car_readers:
        if source.class_name(reader)!='carscript':raise ValueError('Driving car owner differs')
        value=source.read(reader);identity=key(reader);cars[identity]=(reader,value)
        initial=value['gear']+1
        if not 0<=initial<len(value['gears']):raise ValueError('Driving startup gear outside source gear list')
        transform=source.transform_for(source.resolve(reader,value['m_GameObject']))
        vehicles.append({'source':source_id(reader),'source_gear':value['gear'],'driving':{'initial_gear':initial,
            'force_automatic':bool(value['forceAutomaticTransmission']),'play_shift_sounds_in_automatic':False},
            'lever':recover_lever(source,reader,value,transform,node_ids,constants)})
        road={key(source.resolve(reader,p)) for field in ('WHDriven','WHSteer','WHBrakeOnly') for p in value[field]}
        if len(road)!=4 or any(k in owners for k in road):raise ValueError('Driving car must own four wheels')
        owners.update({k:identity for k in road})
        for node in source.hierarchy(transform,activate_root=True):
            if not node['active']:continue
            for component in node['components']:
                if component.type.name!='MonoBehaviour' or source.class_name(component)!='wheelgraphicsscript':continue
                raw=source.read(component);wheel=source.resolve(component,raw['W'])
                if wheel is None:raise ValueError('Unresolved driving wheel collider')
                if key(wheel) in road:
                    if key(wheel) in graphics:raise ValueError('Duplicate driving wheel graphics owner')
                    graphics[key(wheel)]=(component,raw)
    if {key(r) for r in wheel_readers}!=set(owners) or len(wheel_readers)!=len(owners):
        raise ValueError('Driving wheel selection differs from car roles')
    slots=[]
    for reader in wheel_readers:
        identity=key(reader);car_reader,car=cars[owners[identity]]
        graphic,raw=graphics[identity]
        linked_car=source.resolve(graphic,raw['car']);slot=source.resolve(graphic,raw['slot'])
        if linked_car is None or key(linked_car)!=key(car_reader) or slot is None:
            raise ValueError('Unresolved driving wheel hub link')
        hub_index=car['kerekagyType']
        if not 0<=hub_index<len(hubs):raise ValueError('Driving hub selection outside source properties')
        hub=hubs[hub_index]
        slots.append({'source':source_id(slot),'collider_source':source_id(reader),'graphics_source':source_id(graphic),
            'vehicle_source':source_id(car_reader),'hub_source':source_id(main_reader),'hub_index':hub_index,
            'sideways_stiffness_modifier':raw['sideStiffnessModifier'],
            'hub':{'radius_m':hub['radius'],'mass_kg':hub['mass'],'forward_stiffness':hub['slip1'],'sideways_stiffness':hub['slip2']}})
    return {'global':global_record,'vehicles':vehicles,'slots':slots,'keyboard_steering_rate':steering_rate,
        'evidence':{'assembly_sha256':ASSEMBLY_SHA256,'decompiled_sha256':DECOMPILED_SHA256,
            'methods':methods,'assist_toggles':toggles,'bindings':binding_evidence,
            'steering_source':source_id(slider),'serialized_assists':{name:bool(settings['S'][name]) for name in ('BAutoClutch','BAutoShift','BForceManual')},
            'startup':'FStart -> LoadSettings -> ResetSettings(true) -> DoReset (one yielded frame) -> tab.PressedReset -> togglesetting.ResetToDefault -> ResetCustomRadioPath -> OnChangedAnything -> RefreshSettings. The settled fresh reset applies true/true; serialized S contains true/false. Static source evidence, without a native startup measurement.',
            'shift_sounds':'Private bool has no initializer or assignment; CLR startup value is false.'}}
