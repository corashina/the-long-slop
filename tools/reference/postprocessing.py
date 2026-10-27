"""Source postprocessing recovery and exact histogram transport."""
from .render_lighting import f32

LUT_SIZE = 33


# These are constructor values from the installed package, not serialized overrides.
GRADING_DEFAULTS = {
    'temperature': 0., 'tint': 0., 'hueShift': 0., 'saturation': 0.,
    'postExposure': 0., 'contrast': 0.,
    'colorFilter': {'r': 1., 'g': 1., 'b': 1., 'a': 1.},
    **{f'mixer{out}Out{inp}In': 100. if out == inp else 0.
       for out in ('Red', 'Green', 'Blue') for inp in ('Red', 'Green', 'Blue')},
    **{name: {'x': 1., 'y': 1., 'z': 1., 'w': 0.} for name in ('lift', 'gamma', 'gain')},
}
CURVES = ('masterCurve', 'redCurve', 'greenCurve', 'blueCurve',
          'hueVsHueCurve', 'hueVsSatCurve', 'satVsSatCurve', 'lumVsSatCurve')


def effective_parameter(settings, name, constructor=None):
    parameter = settings[name]
    if parameter['overrideState']:
        return parameter['value']
    if constructor is None:
        raise ValueError(f'Unrecovered constructor value for {name}')
    return constructor


def recover_grading(settings):
    if effective_parameter(settings, 'gradingMode') != 1 or effective_parameter(settings, 'tonemapper') != 1:
        raise ValueError('Source grading requires the recovered HDR Neutral variant')
    if any(settings[name]['overrideState'] for name in CURVES):
        raise ValueError('Source grading curve overrides require new recovery')
    values = {name: effective_parameter(settings, name, default) for name, default in GRADING_DEFAULTS.items()}
    return {
        'lut_size': LUT_SIZE, 'curves': 'source_default',
        'temperature': values['temperature'], 'tint': values['tint'],
        'hue_shift': values['hueShift'], 'saturation': values['saturation'],
        'post_exposure': values['postExposure'], 'contrast': values['contrast'],
        'color_filter': [values['colorFilter'][c] for c in ('r', 'g', 'b', 'a')],
        'channel_mixer': [[values[f'mixer{out}Out{inp}In'] for inp in ('Red', 'Green', 'Blue')]
                          for out in ('Red', 'Green', 'Blue')],
        'trackballs': [[values[name][c] for c in ('x', 'y', 'z', 'w')] for name in ('lift', 'gamma', 'gain')],
    }


def recover_postprocessing(source):
    """Recover the selected main-camera stack after startup settings and volume blending."""
    from .source import source_id
    def read(file, pid):
        reader = source.get(file, pid)
        return reader, source.read(reader)
    def effect(pid):
        reader, data = read('sharedassets0.assets', pid)
        if not data['active'] or not effective_parameter(data, 'enabled'):
            raise ValueError(f'Unexpected inactive source effect {pid}')
        return reader, data
    layer_reader, layer = read('level1', 68049)
    bloom_reader, bloom = effect(1371)
    _, grading = effect(1375)
    _, ao = effect(1378)
    _, exposure = effect(1380)
    p = effective_parameter
    if p(ao, 'mode') != 1 or p(exposure, 'eyeAdaptation') != 0:
        raise ValueError('Unrecovered AO or exposure variant')
    resources_reader = source.resolve(layer_reader, layer['m_Resources'])
    resources = source.read(resources_reader)
    blue_noise = [source_id(source.resolve(resources_reader, item)) for item in resources['blueNoise64']]
    if len(blue_noise) != 64 or len({(v['file'], v['path_id']) for v in blue_noise}) != 64:
        raise ValueError('Source dithering requires 64 distinct blue-noise textures')
    dirt = source_id(source.resolve(bloom_reader, p(bloom, 'dirtTexture')))
    filtering = p(exposure, 'filtering')
    return {
        'main_layer': source_id(layer_reader),
        'stop_nan': bool(layer['stopNaNPropagation']),
        'exposure': {'filtering_percent': [filtering['x'], filtering['y']],
                     'luminance_ev': [p(exposure, 'minLuminance'), p(exposure, 'maxLuminance')],
                     'key': p(exposure, 'keyValue'), 'speed_up': p(exposure, 'speedUp'),
                     'speed_down': p(exposure, 'speedDown')},
        'bloom': {'intensity': p(bloom, 'intensity'), 'threshold': p(bloom, 'threshold'),
                  'soft_knee': p(bloom, 'softKnee'), 'clamp': p(bloom, 'clamp'),
                  'diffusion': p(bloom, 'diffusion'), 'anamorphic_ratio': p(bloom, 'anamorphicRatio'),
                  'color': [p(bloom, 'color')[c] for c in ('r', 'g', 'b', 'a')],
                  'fast': bool(p(bloom, 'fastMode')), 'dirt': dirt, 'dirt_intensity': p(bloom, 'dirtIntensity')},
        'occlusion': {'intensity': p(ao, 'intensity'), 'thickness': p(ao, 'thicknessModifier'),
                      'noise_tolerance': p(ao, 'noiseFilterTolerance', 0.),
                      'blur_tolerance': p(ao, 'blurTolerance', f32(-4.6)),
                      'upsample_tolerance': p(ao, 'upsampleTolerance', -12.),
                      'direct_lighting_strength': p(ao, 'directLightingStrength', 0.)},
        'grading': recover_grading(grading), 'blue_noise': blue_noise,
        'heat_haze_weight': 0.,
    }


def recover_camera_stack(source):
    """Keep each source camera's clear, mask, and effect scope separate."""
    from .source import source_id
    player = source.get('level1', 80125)
    fields = source.read(player)
    result = []
    for role, field in (('sky', 'skyCamera'), ('far', 'farCamera'), ('main', 'Cam'), ('foreground', 'mainCam')):
        camera = source.resolve(player, fields[field])
        data = source.read(camera)
        if data['orthographic'] or data['m_projectionMatrixMode'] != 1:
            raise ValueError('Unrecovered source camera projection')
        go = source.resolve(camera, data['m_GameObject'])
        layers = []
        for pointer in source.read(go)['m_Component']:
            component = source.resolve(go, pointer['component'])
            if component.type.name == 'MonoBehaviour' and source.class_name(component) == 'PostProcessLayer':
                layers.append(component)
        if len(layers) != (0 if role == 'sky' else 1):
            raise ValueError('Unexpected source camera postprocessing layers')
        post = None
        if layers:
            layer = source.read(layers[0])
            expected_mask = {'far': 0, 'main': 2097408, 'foreground': 32}[role]
            if not layer['m_Enabled'] or layer['volumeLayer']['m_Bits'] != expected_mask:
                raise ValueError('Source camera volume scope changed')
            if role != 'main' and layer['antialiasingMode'] != 0:
                raise ValueError('Unexpected source secondary-camera antialiasing')
            post = {'source': source_id(layers[0]), 'volume_mask': expected_mask,
                    'scope': 'main_stack' if role == 'main' else 'dither_only',
                    'stop_nan': bool(layer['stopNaNPropagation'])}
        result.append({'role': role, 'source': source_id(camera), 'order': data['m_Depth'],
                       'clear': {1: 'skybox', 3: 'depth'}[data['m_ClearFlags']],
                       'near': data['near clip plane'], 'far': data['far clip plane'],
                       'field_of_view': data['field of view'], 'mask': data['m_CullingMask']['m_Bits'],
                       'hdr': bool(data['m_HDR']), 'allow_msaa': bool(data['m_AllowMSAA']),
                       'postprocessing': post})
    return result


def profile_effects(source,manifest,profile,output,kinds=None):
    """Effects of one PostProcessProfile with every serialized parameter. `kinds` limits the effect classes."""
    import re
    from .source import source_id,key
    from .lighting import texture
    from .particles import curve
    source.lighting_textures=getattr(source,'lighting_textures',{(t['source']['file'],t['source']['path_id']):t for t in manifest['lighting']['textures']})
    effects=[]
    for pointer in source.read(profile)['settings']:
        effect=source.resolve(profile,pointer);data=source.read(effect);name=source.class_name(effect)
        if kinds is not None and name not in kinds:raise ValueError(f'Unknown required profile effect: {name}')
        parameters={}
        for field,p in data.items():
            if not isinstance(p,dict) or 'overrideState' not in p:continue
            value=p['value'];v={'override_state':bool(p['overrideState'])}
            if isinstance(value,dict) and 'm_PathID' in value:
                target=source.resolve(effect,value)
                v.update(value=source_id(target) if target else None,default_state=p['defaultState'],texture=None)
                if target:
                    if output is None:
                        v['texture']=source.lighting_textures[key(target)]['id']
                    else:v['texture']=texture(source,target,output)
            elif isinstance(value,dict) and 'curve' in value:
                c=curve({'minMaxState':0,'minScalar':0.,'scalar':1.,'minCurve':value['curve'],'maxCurve':value['curve']})['max_curve']
                v['value']={'curve':c,'cached_data':value['cachedData'],'loop':bool(value['m_Loop']),'range':value['m_Range'],'zero_value':value['m_ZeroValue']}
            elif isinstance(value,dict):v['value']=[value[c] for c in ('rgba' if 'r' in value else 'xyzw') if c in value]
            else:v['value']=bool(value) if field=='enabled' else value
            parameters[field]=v
        effects.append({'kind':re.sub(r'(?<!^)(?=[A-Z])','_',name).lower(),'source':source_id(effect),'active':bool(data['active']),'parameters':parameters})
    return effects


def recover_glasses_profiles(source,manifest,output):
    from .source import source_id
    profiles=[]
    for pid in range(130128,130134):
        reader=source.get('sharedassets1.assets',pid)
        profiles.append({'source':source_id(reader),'effects':profile_effects(source,manifest,reader,output,{'ColorGrading'})})
    return profiles


# fpscontroller.DeadV. mainscript.Died enables it; Update sets weight = InverseLerp(0, 14, t - deathTime).
MAIN_VOLUME_MASK = 2097408


def recover_dead_profile(source,manifest,output):
    from .source import source_id
    from .survival import managed_evidence
    player=source.get('level1',80125)
    volume=source.resolve(player,source.read(player)['DeadV'])
    if volume is None or volume.type.name!='MonoBehaviour' or source.class_name(volume)!='PostProcessVolume':raise ValueError('Missing DeadV volume')
    d=source.read(volume)
    profile=source.resolve(volume,d['sharedProfile'])
    if profile is None:raise ValueError('Missing DeadV profile')
    return {'source':source_id(volume),'enabled':bool(d['m_Enabled']),'global':bool(d['isGlobal']),
        'blend_distance':d['blendDistance'],'weight':d['weight'],'priority':d['priority'],
        'layer':source.read(source.resolve(volume,d['m_GameObject']))['m_Layer'],
        'profile':source_id(profile),'effects':profile_effects(source,manifest,profile,output),
        'evidence':managed_evidence(source,['player.DeadV.enabled = true;','player.DeadV.weight = num;',
            'float num = Mathf.InverseLerp(0f, 14f, Time.time - deathTime);','player.deathSens = Mathf.Lerp(0.8f, 0f, num);'])}


def validate_dead_profile(p,manifest):
    from .survival import finite_record
    finite_record(p)
    if p['source']['file']!='level1' or not p['global'] or not 0<=p['weight']<=1 or not 0<=p['layer']<32 or not MAIN_VOLUME_MASK>>p['layer']&1:
        raise ValueError('DeadV volume')
    if not p['effects'] or len({(e['source']['file'],e['source']['path_id']) for e in p['effects']})!=len(p['effects']):raise ValueError('DeadV effects')
    for e in p['effects']:
        if not e['kind'] or not e['kind'].replace('_','').isalnum() or 'enabled' not in e['parameters']:raise ValueError('DeadV effect')
        for v in e['parameters'].values():
            if type(v['override_state']) is not bool:raise ValueError('DeadV parameter')
            if 'texture' in v and v['texture'] is not None and not any(t['id']==v['texture'] and t['source']==v['value'] for t in manifest['lighting']['textures']):
                raise ValueError('DeadV texture dependency')


def validate_glasses_profiles(profiles,manifest):
    from .survival import finite_record
    from .validation import record,array,number,u32,boolean,string,source,optional
    from .particles import validate_curve
    def spline(c):
        record(c,{'curve':validate_curve,'cached_data':array(number,128),'loop':boolean,'range':number,'zero_value':number})
    def parameters(d):
        if not isinstance(d,dict) or set(d)!=GLASSES_PARAMETERS:raise ValueError('Incomplete glasses parameters')
        for name,p in d.items():
            if name in ('ldrLut','externalLut'):
                record(p,{'override_state':boolean,'value':optional(source),'default_state':u32,'texture':optional(u32)})
            else:
                value=spline if name.endswith('Curve') else array(number,4) if name in ('colorFilter','gain','gamma','lift') else boolean if name=='enabled' else u32 if name in ('gradingMode','tonemapper') else number
                record(p,{'override_state':boolean,'value':value})
    def effect(e):record(e,{'kind':string,'source':source,'active':boolean,'parameters':parameters})
    def profile(p):record(p,{'source':source,'effects':array(effect,1)})
    array(profile,6)(profiles)
    finite_record(profiles)
    if [p['source'] for p in profiles]!=[{'file':'sharedassets1.assets','path_id':i} for i in range(130128,130134)]:raise ValueError('Selected glasses profiles')
    for i,p in enumerate(profiles):
        if len(p['effects'])!=1:raise ValueError('Required glasses effect count')
        e=p['effects'][0]
        if e['kind']!='color_grading' or e['source']!={'file':'sharedassets1.assets','path_id':130142+i} or not e['active']:raise ValueError('Unknown required glasses effect')
        d=e['parameters']
        if set(d)!=GLASSES_PARAMETERS:raise ValueError('Incomplete glasses parameters')
        for name,value in [('enabled',True),('gradingMode',1),('tonemapper',0),('toneCurveToeLength',.5),('toneCurveToeStrength',0.),('postExposure',-2. if i==0 else 0.)]:
            if d[name]!={'override_state':True,'value':value}:raise ValueError('Glasses source override')
        if any(d[k]['override_state'] for k in ['blueCurve', 'brightness', 'colorFilter', 'contrast', 'externalLut', 'gain', 'gamma', 'greenCurve', 'hueShift', 'hueVsHueCurve', 'hueVsSatCurve', 'ldrLut', 'ldrLutContribution', 'lift', 'lumVsSatCurve', 'masterCurve', 'mixerBlueOutGreenIn', 'mixerBlueOutRedIn', 'mixerGreenOutBlueIn', 'mixerGreenOutRedIn', 'mixerRedOutBlueIn', 'mixerRedOutGreenIn', 'redCurve', 'satVsSatCurve', 'saturation', 'temperature', 'tint', 'toneCurveGamma', 'toneCurveShoulderAngle', 'toneCurveShoulderLength', 'toneCurveShoulderStrength']):raise ValueError('Unexpected glasses override')
        diagonal=[(100.,100.,100.),(50.,100.,200.),(75.,200.,75.),(200.,150.,80.),(150.,50.,50.),(150.,150.,50.)][i]
        for channel,value in zip(('Red','Green','Blue'),diagonal):
            field=d[f'mixer{channel}Out{channel}In']
            if field['override_state']!=(i>0) or field['value']!=value:raise ValueError('Glasses source mixer')
        for name in ('ldrLut','externalLut'):
            t=d[name];expected={'file':'sharedassets0.assets','path_id':255} if name=='ldrLut' else None
            if t['value']!=expected or t['override_state'] or t['default_state']!=(4 if name=='ldrLut' else 1):raise ValueError('Glasses resource source')
            if expected and not any(r['id']==t['texture'] and r['source']==expected for r in manifest['lighting']['textures']):raise ValueError('Glasses resource dependency')
            if not expected and t['texture'] is not None:raise ValueError('Unexpected glasses resource')

GLASSES_PARAMETERS={'masterCurve', 'mixerBlueOutGreenIn', 'postExposure', 'toneCurveShoulderAngle', 'gamma', 'gradingMode', 'blueCurve', 'mixerRedOutBlueIn', 'toneCurveGamma', 'mixerRedOutRedIn', 'satVsSatCurve', 'toneCurveToeLength', 'mixerRedOutGreenIn', 'tonemapper', 'lumVsSatCurve', 'ldrLut', 'mixerBlueOutBlueIn', 'lift', 'mixerGreenOutRedIn', 'toneCurveShoulderLength', 'greenCurve', 'mixerBlueOutRedIn', 'toneCurveToeStrength', 'hueVsSatCurve', 'contrast', 'saturation', 'externalLut', 'colorFilter', 'enabled', 'gain', 'brightness', 'tint', 'mixerGreenOutGreenIn', 'ldrLutContribution', 'temperature', 'redCurve', 'hueShift', 'mixerGreenOutBlueIn', 'toneCurveShoulderStrength', 'hueVsHueCurve'}
