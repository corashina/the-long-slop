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


def car_driving(car):
    """Per-car startup gear and transmission flags of one carscript."""
    initial=car['gear']+1
    if not 0<=initial<len(car['gears']):raise ValueError('Driving startup gear outside source gear list')
    return {'initial_gear':initial,'force_automatic':bool(car['forceAutomaticTransmission']),'play_shift_sounds_in_automatic':False}


def road_wheels(source, reader, car):
    """WheelCollider identities that carscript drives, steers or brakes."""
    return {key(source.resolve(reader,p)) for field in ('WHDriven','WHSteer','WHBrakeOnly') for p in car[field]}


def hub_record(hubs, car):
    """mainscript.kerekagyProperties[carscript.kerekagyType]."""
    index=car['kerekagyType']
    if not 0<=index<len(hubs):raise ValueError('Driving hub selection outside source properties')
    hub=hubs[index]
    return {'radius_m':hub['radius'],'mass_kg':hub['mass'],'forward_stiffness':hub['slip1'],'sideways_stiffness':hub['slip2']}


def wheel_graphics(source, nodes, road):
    """Active wheelgraphicsscript per road wheel identity, from hierarchy records."""
    graphics={}
    for node in nodes:
        if not node['active']:continue
        for component in node['components']:
            if component.type.name!='MonoBehaviour' or source.class_name(component)!='wheelgraphicsscript':continue
            raw=source.read(component);wheel=source.resolve(component,raw['W'])
            if wheel is None:raise ValueError('Unresolved driving wheel collider')
            if key(wheel) in road:
                if key(wheel) in graphics:raise ValueError('Duplicate driving wheel graphics owner')
                graphics[key(wheel)]=(component,raw)
    return graphics


def recover_driving(source, car_readers, wheel_readers):
    """Resolve selected cars and wheel owners; fail on missing records or links."""
    bodies, methods, minimum, text = checked_methods(source)
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
        vehicles.append({'source':source_id(reader),'source_gear':value['gear'],'driving':car_driving(value)})
        road=road_wheels(source,reader,value)
        if len(road)!=4 or any(k in owners for k in road):raise ValueError('Driving car must own four wheels')
        owners.update({k:identity for k in road})
        transform=source.transform_for(source.resolve(reader,value['m_GameObject']))
        graphics.update(wheel_graphics(source,source.hierarchy(transform,activate_root=True),road))
    if {key(r) for r in wheel_readers}!=set(owners) or len(wheel_readers)!=len(owners):
        raise ValueError('Driving wheel selection differs from car roles')
    slots=[]
    for reader in wheel_readers:
        identity=key(reader);car_reader,car=cars[owners[identity]]
        graphic,raw=graphics[identity]
        linked_car=source.resolve(graphic,raw['car']);slot=source.resolve(graphic,raw['slot'])
        if linked_car is None or key(linked_car)!=key(car_reader) or slot is None:
            raise ValueError('Unresolved driving wheel hub link')
        slots.append({'source':source_id(slot),'collider_source':source_id(reader),'graphics_source':source_id(graphic),
            'vehicle_source':source_id(car_reader),'hub_source':source_id(main_reader),'hub_index':car['kerekagyType'],
            'sideways_stiffness_modifier':raw['sideStiffnessModifier'],'hub':hub_record(hubs,car)})
    return {'global':global_record,'vehicles':vehicles,'slots':slots,'keyboard_steering_rate':steering_rate,
        'evidence':{'assembly_sha256':ASSEMBLY_SHA256,'decompiled_sha256':DECOMPILED_SHA256,
            'methods':methods,'assist_toggles':toggles,'bindings':binding_evidence,
            'steering_source':source_id(slider),'serialized_assists':{name:bool(settings['S'][name]) for name in ('BAutoClutch','BAutoShift','BForceManual')},
            'startup':'FStart -> LoadSettings -> ResetSettings(true) -> DoReset (one yielded frame) -> tab.PressedReset -> togglesetting.ResetToDefault -> ResetCustomRadioPath -> OnChangedAnything -> RefreshSettings. The settled fresh reset applies true/true; serialized S contains true/false. Static source evidence, without a native startup measurement.',
            'shift_sounds':'Private bool has no initializer or assignment; CLR startup value is false.'}}
