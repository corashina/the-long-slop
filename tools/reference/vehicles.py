"""Vehicle records of car world prototypes and engine and radiator records of part nodes.

Ids are local to one world prototype: node, body and collider indices.
"""
import math
from .source import key, source_id
from .wheels import wheel_collider_record
from .driving import car_driving, road_wheels, hub_record, wheel_graphics
from .instruments import instrument_record, gauge_limits, Meters
from .lighting import vehicle_light_links, vehicle_states
from .particles import recover_exhaust
from .wheel_effects import recover_wheel_effects, effect_gains
from .plates import prototype_plates
from .render_lighting import probe_settings

ENGINE_FIELDS = {'start_chance':'startChance','starter_rpm':'starterRpm','starter_rpm_lerp':'starterRpmLerp',
    'idle_rpm':'alapRpm','max_rpm':'maxRpm','shutdown_rpm':'shutdownRpm','max_torque_nm':'maxNm',
    'consumption_modifier':'consumptionM','oil_consumption_modifier':'OilConsumptionModifier',
    'engine_brake_modifier':'motorBrakeModifier','rpm_change_modifier':'rpmChangeModifier',
    'engine_start_time_modifier':'engineStartTimeModifier','heat_gain_min':'engineHeatGainMin',
    'heat_gain_max':'engineHeatGainMax','heat_loss':'engineHeatLoss','initial_temperature':'temp',
    'heat_loss_speed_modifier':'engineHeatLossSpeedModifier','max_rpm_difference':'maxRpmDifference',
    'torque_modifier':'modifier','oil_fluid':'Oilfluid','auto_shift_clutch_rpm':'autoShiftClutchRpm',
    'auto_shift_clutch_rpm_range':'autoShiftClutchRpmRange'}
RADIATOR_FIELDS = {'fan_temperature_on':'fanOnTemp','fan_temperature_off':'fanOffTemp','max_value':'maxValue',
    'max_fan_value':'maxFanValue','max_fan_speed':'maxFanSpeed','max_fluid_loss':'maxFluidLoss'}


def smoke_rate(engine: dict) -> float:
    """Read enginescript.defSmokeRate."""
    rate = engine['defSmokeRate']
    if not math.isfinite(rate) or rate < 0:
        raise ValueError('Invalid engine smoke rate')
    return rate


def curve_record(curve: dict) -> dict:
    """Unity AnimationCurve keys and wrap modes."""
    return {'keys':[{'time':k['time'],'value':k['value'],'in_tangent':k['inSlope'],'out_tangent':k['outSlope'],
        'in_weight':k['inWeight'],'out_weight':k['outWeight'],'weighted_mode':k['weightedMode']} for k in curve['m_Curve']],
        'pre_wrap_mode':curve['m_PreInfinity'],'post_wrap_mode':curve['m_PostInfinity']}


BALANCE_FIELDS = {'lean_min_speed':'balanceLeanMinSpeed','lean_max_speed':'balanceLeanMaxSpeed',
    'lean_min_angle':'balanceLeanMinAngle','lean_max_angle':'balanceLeanMaxAngle','lean_angle_lerp':'balanceLeanAngleLerp',
    'steer_angle_min':'balanceSteerAngleMin','steer_angle_max':'balanceSteerAngleMax',
    'steer_angle_speed_min':'balanceSteerAngleSpeedMin','steer_angle_speed_max':'balanceSteerAngleSpeedMax',
    'steer_angle_lerp':'balanceSteerAngleLerp','p':'balanceP','i':'balanceI','d':'balanceD',
    'set_angle':'balanceSetAngle','min_angle':'balanceMinAngle','max_angle':'balanceMaxAngle','max_force':'balanceMaxForce'}


def balance_record(car: dict) -> dict | None:
    """carscript balance values (source Balance). None when the car is not balanced."""
    if not car['balanced']:return None
    return {'lean':bool(car['balanceLean']),'lean_curve':curve_record(car['balanceLeanCurve']),
        'use_angle_limit':bool(car['balanceUseAngleLimit']),**{field:car[unity] for field,unity in BALANCE_FIELDS.items()}}


def engine_fields(engine: dict) -> dict:
    """enginescript values. The car and mounted parts add tanks, starter delay and electronics."""
    return {**{field:engine[unity] for field,unity in ENGINE_FIELDS.items()},
        'starting_push_force':engine['maxNm']*9.,
        'optimal_temperature':[engine[k] for k in ('minOptimalTemp1','maxOptimalTemp1','minOptimalTemp2','maxOptimalTemp2')],
        'fuel_consumption':[sum(f['amount'] for f in engine['FuelConsumption']['fluids'] if f['type']==i) for i in range(6)],
        'two_stroke':bool(engine['twostroke']),'no_overheat':bool(engine['noOverHeat']),'kickstart':bool(engine['kickstart']),
        'start_chance_interval_seconds':1.,'smoke_rate':smoke_rate(engine),
        'torque_curve':curve_record(engine['torqueCurve'])}


def radiator_fields(radiator: dict) -> dict:
    """coolantTankscript values. Links to the fan and the coolant tank are added by the caller."""
    return {'thermostat_range':[radiator['minThermostatTemp'],radiator['maxThermostatTemp']],
        'fan_on_when_engine_runs':bool(radiator['fanOnWhenEngineRuns']),
        **{field:radiator[unity] for field,unity in RADIATOR_FIELDS.items()}}


def vehicle_globals(source) -> dict:
    """Shared engine values of mainscript. The world record holds them once for every car."""
    reader=source.get('level1',64919);main=source.read(reader)
    return {'source':source_id(reader),'fuel_consumption':main['defConsumption'],'oil_consumption':main['defOilConsumption'],
        'rpm_change':2000.,'car_temperature_bounds':[main['minCarTemp'],main['maxCarTemp']],
        'coolant_evaporate_rate':main['coolantEvaporateRate'],'temperature_difference_modifier':main['tempDifferenceModifier'],
        'temperature_move_modifier':main['tempChangeMoveModifier'],'max_car_velocity':main['maxCarVelocity'],
        'outside_temperature_modifier':main['outsideTempM'],'min_steam_temperature':main['minSteamTemp']}


def dry_mass(source, components) -> float:
    """massScript mass of one node. The runtime adds tank fluids at 1 kg per liter."""
    return sum(source.read(r)['mass'] for r in components
        if r.type.name=='MonoBehaviour' and source.class_name(r)=='massScript')


def seat_fields(source, reader, value, car_reader) -> dict:
    """Driver roles of a primary seat in the car of `car_reader`."""
    car=source.resolve(reader,value['Car'])
    own=car is not None and key(car)==key(car_reader)
    return {'driver':bool(value['driverSeat0']) and own,'driving2':bool(value['driverSeat1']) and own}


class Prototype:
    """Resolve source links to node indices of one world prototype hierarchy."""
    def __init__(self, source, hierarchy):
        self.source=source;self.hierarchy=hierarchy
        self.nodes={key(n['reader']):i for i,n in enumerate(hierarchy)}

    def scripts(self, name, active_only=False):
        """(node index, component) of each MonoBehaviour of class `name`."""
        source=self.source
        return [(i,c) for i,n in enumerate(self.hierarchy) if n['active'] or not active_only for c in n['components']
            if c.type.name=='MonoBehaviour' and source.read(c)['m_Script']['m_PathID'] and source.class_name(c)==name]

    def node(self, reader):
        source=self.source
        if reader.type.name not in ('Transform','RectTransform'):
            if reader.type.name!='GameObject':reader=source.resolve(reader,source.read(reader)['m_GameObject'])
            reader=source.transform_for(reader)
        if key(reader) not in self.nodes:raise ValueError(f'{key(reader)} link outside the prototype')
        return self.nodes[key(reader)]

    def link(self, owner, pointer):
        """Node index of a pointer target, or None for a null pointer."""
        reader=self.source.resolve(owner,pointer)
        return None if reader is None else self.node(reader)

    def links(self, owner, pointers):
        return [self.node(r) for p in pointers if (r:=self.source.resolve(owner,p))]


def gauge_records(source, proto, asset, material_indices) -> list:
    """Gauge part records: each meterscript on a part node that is not a built-in car meter."""
    source=proto.source
    built_in={key(source.resolve(c,p)) for _,c in proto.scripts('carscript') for p in source.read(c)['meters'] if p['m_PathID']}
    build=Meters(source,proto.nodes,asset,material_indices)
    records=[]
    for node,reader in proto.scripts('meterscript'):
        if key(reader) in built_in:continue
        # The meter belongs to the nearest partscript node at or above it.
        current=proto.hierarchy[node]['reader'];part=False
        while current is not None and key(current) in proto.nodes:
            if any(c.type.name=='MonoBehaviour' and source.read(c)['m_Script']['m_PathID'] and source.class_name(c)=='partscript'
                   for c in proto.hierarchy[proto.nodes[key(current)]]['components']):
                part=True;break
            current=source.resolve(current,source.read(current)['m_Father'])
        if part:records.append(build.gauge(reader,node))
    return records


def kmh_meters(source, proto, car_reader, car) -> list:
    """carscript.kmhmeters: gauge slots and how their meters read."""
    rows=[]
    for k in car['kmhmeters']:
        slots=[source.resolve(car_reader,p) for p in k['slots'] if p['m_PathID']]
        for slot in slots:
            if source.class_name(slot)!='partslotscript':raise ValueError(f'{key(car_reader)} kmhmeters slot is not a partslotscript')
            proto.node(slot)
        rows.append({'force':bool(k['force']),'modifier':k['modifier'],'types':list(k['types']),'slots':[source_id(s) for s in slots]})
    return rows


def fuel_tank_slot(source, proto, car_reader, car):
    """carscript.tankslot. Its part holds the fuel tank when the root has no tankscript."""
    slot=source.resolve(car_reader,car['tankslot'])
    if slot is None:return None
    if source.class_name(slot)!='partslotscript':raise ValueError(f'{key(car_reader)} tankslot is not a partslotscript')
    proto.node(slot)
    return source_id(slot)


def part_records(source, hierarchy, asset, material_indices) -> tuple[list, list, list]:
    """Engine, radiator and gauge records of the part nodes of one prototype."""
    proto=Prototype(source,hierarchy)
    engines=[{'source':source_id(reader),'node':node,'oil_tank':proto.link(reader,value['T']),**engine_fields(value)}
        for node,reader in proto.scripts('enginescript') for value in (source.read(reader),)]
    radiators=[{'source':source_id(reader),'node':node,'coolant_tank':proto.link(reader,value['coolant']),
        'fan_node':proto.link(reader,value['Fan']),**radiator_fields(value)}
        for node,reader in proto.scripts('coolantTankscript') for value in (source.read(reader),)]
    return engines,radiators,gauge_records(source,proto,asset,material_indices)


def material_link(source, owner, pointer, asset, material_indices):
    reader=source.resolve(owner,pointer)
    if reader is None:return None
    identity=f'{reader.assets_file.name}:{reader.path_id}'
    if identity not in material_indices:raise ValueError(f'Missing vehicle material: {identity}')
    return {'source':source_id(reader),'asset':asset,'gltf_material':material_indices[identity]}


def light_records(source, proto, car_reader, car, asset, material_indices):
    """headlightscript and rearlights controllers and the light switch of one car prototype."""
    scripts=[c for n in proto.hierarchy for c in n['components'] if c.type.name=='MonoBehaviour' and source.read(c)['m_Script']['m_PathID']]
    routes,switches=vehicle_light_links(source,scripts,car_reader)
    if len(switches)>1:raise ValueError(f'{key(car_reader)} has more than one light switch')
    def lights(owner,pointers):
        result=[]
        for p in pointers:
            r=source.resolve(owner,p)
            if not r:continue
            candidates=[r] if r.type.name=='Light' else [c for n in source.hierarchy(source.transform_for(r)) for c in n['components'] if c.type.name=='Light']
            result.extend(candidates)
        return result
    controllers=[]
    for node,reader in [*proto.scripts('headlightscript'),*proto.scripts('rearlights')]:
        f=source.read(reader)
        kind='rear' if source.class_name(reader)=='rearlights' else 'cabin' if key(reader) in routes else 'headlight'
        def state(signal,intensity,ls,rs,on,off,broken=False,working=(),not_working=(),dont_overwrite=False,reader=reader):
            return {'signal':signal,'lights':[proto.node(c) for c in lights(reader,ls)],'intensity':intensity,
                'renderers':proto.links(reader,rs),'on_material':material_link(source,reader,on,asset,material_indices),
                'off_material':material_link(source,reader,off,asset,material_indices),'broken':bool(broken),
                'working_nodes':proto.links(reader,working),'broken_nodes':proto.links(reader,not_working),'dont_overwrite':bool(dont_overwrite)}
        def intensity(pointers,reader=reader):
            found=lights(reader,pointers)
            return source.read(found[0])['m_Intensity'] if found else 0.
        electronics=car['electronics'] if kind=='rear' else f.get('electronics',{'m_PathID':0})
        record={'source':source_id(reader),'kind':kind,'node':node,
            'electronics':proto.link(car_reader if kind=='rear' else reader,electronics),
            'startup_on':bool(f.get('ON',False)),'current_state':f.get('currentState',0),'requested_state':f.get('switchToState',0),
            'energy':f.get('energy',1.),'energy_move':f.get('energyMove',0.),'enabled':bool(f['m_Enabled']),
            'reverse_switch':bool(f.get('reverseSwitch',False)),'states':vehicle_states(f,kind,state,intensity),'cabin':None}
        if kind=='cabin':
            route=routes[key(reader)];rf=source.read(route)
            switch=source.resolve(route,rf['turnOverride'])
            record['cabin']={'source':source_id(route),'switch_node':proto.node(switch),'modes':rf['turnOverrideT'],
                'startup_mode':source.read(switch)['currentTurnState'],'door_slots':proto.links(route,rf['ISlotUsableV1To1']),
                'threshold':rf['iUsableV1To1Limit'],'include_children':bool(rf['includeUsablesChild1To1']),
                'missing_slot_is_same':bool(rf['ISlotUsableV1To1NoPartInSlotIsSame'])}
        controllers.append(record)
    switch=None
    if switches:
        switch={'usable':proto.node(switches[0]),'nodes':proto.links(car_reader,car['LightSwitch']),
            'state_nodes':[proto.links(car_reader,car[k]) for k in ('LightOff','LightLow','LightHigh')]}
    return controllers,switch


def probe_record(source, proto):
    """The car's realtime ReflectionProbe, or None. Its node is a prototype node index."""
    probes=[(i,c) for i,n in enumerate(proto.hierarchy) for c in n['components'] if c.type.name=='ReflectionProbe']
    if len(probes)>1:raise ValueError(f'{key(proto.hierarchy[0]["reader"])} has more than one reflection probe')
    if not probes:return None
    node,reader=probes[0]
    return {'source':source_id(reader),'node':node,**probe_settings(key(reader),source.read(reader))}


def vehicle_record(source, hierarchy, bodies, colliders, asset, material_indices, output):
    """`vehicle` record of a world prototype whose root has carscript, else None.

    `hierarchy` is source.hierarchy(root, activate_root=True). `bodies` and `colliders` are the
    prototype tables; their order gives the local ids.
    """
    proto=Prototype(source,hierarchy)
    cars=[c for i,c in proto.scripts('carscript') if i==0]
    if not cars:return None
    car_reader=cars[0];car=source.read(car_reader)
    if car['speedBasedKeyboardSens'] or car['keySteerSensMultiplier']!=0:
        raise ValueError(f'Car requires unsupported speed-based keyboard steering: {key(car_reader)}')
    rigidbody=source.resolve(car_reader,car['RB'])
    body=next((i for i,b in enumerate(bodies) if b['source']==source_id(rigidbody)),None) if rigidbody else None
    if body is None:raise ValueError(f'{key(car_reader)} chassis body outside the prototype')
    main=source.read(source.get('level1',64919))
    # Wheels.
    road=road_wheels(source,car_reader,car)
    if not road:raise ValueError(f'{key(car_reader)} has no wheels')
    roles={name:{key(source.resolve(car_reader,p)) for p in car[field]}
        for name,field in (('drive','WHDriven'),('steer','WHSteer'),('brake','WHBrakeOnly'),('handbrake','WHHandBrake'))}
    graphics=wheel_graphics(source,hierarchy,road)
    wheels=[]
    for identity in sorted(road):
        if identity not in graphics:raise ValueError(f'Wheel slot missing: {identity}')
        reader=source.get(*identity);graphic,raw=graphics[identity]
        linked=source.resolve(graphic,raw['car']);slot=source.resolve(graphic,raw['slot'])
        if linked is None or key(linked)!=key(car_reader) or slot is None:raise ValueError('Unresolved driving wheel hub link')
        wheels.append({'source':source_id(reader),'node':proto.node(reader),'collider':wheel_collider_record(source.read(reader)),
            'slot':proto.node(slot),'mount':proto.link(slot,source.read(slot)['T']) or proto.node(slot),
            **{name:identity in roles[name] for name in ('steer','drive','brake','handbrake')},
            'sideways_stiffness_modifier':raw['sideStiffnessModifier']})
    # Driver roles of primary seats. The runtime binds the seats from the prototype.
    seats=[]
    for node,reader in proto.scripts('seatscript',active_only=True):
        value=source.read(reader)
        if value['mainseat']['m_PathID']:continue
        seats.append({'source':source_id(reader),**seat_fields(source,reader,value,car_reader)})
    lights,light_switch=light_records(source,proto,car_reader,car,asset,material_indices)
    instruments=instrument_record(source,car_reader,proto.nodes,asset,material_indices)
    tanks=[c for c in hierarchy[0]['components'] if c.type.name=='MonoBehaviour' and source.class_name(c)=='tankscript']
    return {'source':source_id(car_reader),'body':body,'car_type':car['carType'],
        'colliders':[i for i,c in enumerate(colliders) if c['body']==body and c['enabled']],
        'mass_kg':sum(dry_mass(source,n['components']) for n in hierarchy if n['active']),
        'serialized_mass_kg':source.read(rigidbody)['m_Mass'],
        'fuel_tank':0 if tanks else None,'fuel_tank_slot':fuel_tank_slot(source,proto,car_reader,car),'electronics':proto.link(car_reader,car['electronics']),
        'starter_delay_seconds':car['startingTime'],'balance':balance_record(car),'driving':car_driving(car),'handbrake':bool(car['BhandBrake']),
        'hub':hub_record(main['kerekagyProperties'],car),'wheels':wheels,
        'gear_ratios':[g['ratio'] for g in car['gears']],'gear_free_run':[bool(g['freeRun']) for g in car['gears']],
        'final_drive_ratio':car['differentialRatio'],'max_steer_degrees':car['steerAngle'],
        'brake_torque_nm':car['brakePower'],'handbrake_torque_nm':car['maxHandBrake'],
        'wheel_slip':[main[k] for k in ('WheelSlip1Min','WheelSlip1Max','WheelSlip2Min','WheelSlip2Max')],
        'rim_slip':[main['WheelSlip1'],main['WheelSlip2']],
        'auto_shift_stop_speed':car['autoShiftStopSpeed'],'auto_shift_min_rpm':car['autoShiftMinRpm'],'auto_shift_max_rpm':car['autoShiftMaxRpm'],
        'dont_ignite_with_throttle_in_automatic':bool(car['dontIgniteWithThorttleInAutomatic']),'auto_move_min':0.3,
        'collision_gain':car['collisionSoundM'],
        'exhaust':recover_exhaust(source,car_reader,car,hierarchy[0]['reader'],car_reader.path_id,output),
        'wheel_effects':recover_wheel_effects(source,car_reader,car,hierarchy[0]['reader'],[source.get(*identity) for identity in sorted(road)],car_reader.path_id,output),
        'effect_gains':effect_gains(car),
        'seats':seats,'lights':lights,'light_switch':light_switch,'instruments':instruments,
        'gauge_limits':gauge_limits(source,car_reader),'kmh_meters':kmh_meters(source,proto,car_reader,car),
        'plates':prototype_plates(source,proto),'probe':probe_record(source,proto),
        'high_beams':[i for i,lamp in enumerate(instruments['lamps']) if lamp['signal']=='headlight2'] if instruments else []}
