"""Resolve only the selected Lada's source instrument components."""
from .source import key,source_id,vector

def recover_instruments(source,manifest,reports):
    nodes={(n['source']['file'],n['source']['path_id']):n['id'] for n in manifest['nodes']}
    material_indices=reports['lada']['materials']
    root=source.transform_for(source.get('level1',17687))
    parts=[c for n in source.hierarchy(root) for c in n['components'] if c.type.name=='MonoBehaviour'
           and source.read(c)['m_Script']['m_PathID']]
    car=next(c for c in parts if source.class_name(c)=='carscript'); cd=source.read(car)
    meter=source.get('level1',78501);md=source.read(meter)
    def node(owner,ptr,renderer=False):
        reader=source.resolve(owner,ptr)
        if renderer:reader=source.transform_for(source.resolve(reader,source.read(reader)['m_GameObject']))
        if key(reader) not in nodes:raise ValueError(f'Missing instrument node: {key(reader)}')
        return nodes[key(reader)]
    def material(owner,ptr):
        r=source.resolve(owner,ptr);identity=f'{r.assets_file.name}:{r.path_id}'
        return {'source':source_id(r),'asset':2,'gltf_material':material_indices[identity]}
    needles=[]
    for entry in md['mutatok']:
        r=source.resolve(meter,entry['mutato']);d=source.read(r)
        axes=[d['X'],d['Y'],d['Z']]
        if sum(bool(x) for x in axes)!=1:raise ValueError(f'Invalid needle axes: {key(r)}')
        needles.append({'source':source_id(r),'node':node(r,d['mutato']),'signal':entry['tipus'],
            'min_value':d['minValue'],'max_value':d['maxValue'],'min_angle':d['minAngle'],'max_angle':d['maxAngle'],
            'axis':axes.index(1),'rest':[d['x'],d['y'],d['z']],'offset':d['inputValueOffset'],'inv':bool(d['inv']),
            'value_smoothing':8*(d['lerpSpeedMultiplier'] or 1),'signal_smoothing':5 if entry['tipus'] in ('fuel','temp') else 0,
            'no_value_lerp':bool(d['noValueLerp']),'lerp_angle':bool(d['lerpAngle'])})
    lamps=[]
    for ptr in md['leds']:
        r=source.resolve(meter,ptr);d=source.read(r)
        for led in d['leds']:
            lamps.append({'source':source_id(r),'node':node(r,led['r'],True),'signal':d['vname'],
                'on':material(r,led['on']),'off':material(r,led['off']),'min_value':led['minValue'],
                'max_value':led['maxValue'],'stay_on':bool(led['stayOnAfterMaximum'])})
    count=md['szamlapok'][0];r=source.resolve(meter,count['forgo']);d=source.read(r)
    digits=[]
    for digit in d['szamok']:
        axes=[digit['x'],digit['y'],digit['z']]
        digits.append({'node':node(r,digit['T']),'place':digit['helyiertekHatvany'],
            'axis':next((i for i,x in enumerate(axes) if x),0),'inv':bool(digit['inv']),'direct':bool(digit['direct']),
            'continuous':bool(digit['continous']),'initial_value':digit['alakiertek'],'initial_angle':digit['angle'],
            'on':material(r,digit['MOn']),'off':material(r,digit['MOff'])})
    electronic=source.resolve(car,cd['electronics'])
    if electronic is None:electronic_output=1.
    else:
        e=source.read(electronic);maximum=e['overRideMaxJamValue'] if e['overRideMaxJamValue']>0 else 0.3
        electronic_output=e['maxOutput']*(1.-max(0.,min(1.,e['jamValue']/maximum)))
    return {'electronics_output':electronic_output,'electronics_off_threshold':0.2,'needles':needles,'lamps':lamps,'odometer':{'source':source_id(r),'initial_value':count['value'],
        'base':d['szamrendszer'],'decimals':d['tizedesjegyek'],'use_multiplier':bool(d['useMultiplier']),
        'multiplier':d['multiplier'],'digits':digits},
        'low_fuel':cd['lowFuelValue'],'low_oil':cd['lowOilValue'],
        'oil_test_rpm':cd['ledOilTestRpm'],'battery_test_rpm':cd['ledBatteryTestRpm']}
