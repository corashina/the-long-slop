"""Source postprocessing recovery and exact histogram transport."""
import math

from .render_lighting import f32

HISTOGRAM_BINS = 128
SCATTER_SAMPLES = 32
LUT_SIZE = 33


def histogram_weight(u, v):
    """Installed KEyeHistogram DXBC: vignette weighting, then float-to-uint."""
    x, y = f32(u - .5), f32(v - .5)
    distance = f32(f32(x * x) + f32(y * y))
    factor = max(0., f32(1. - distance))
    return int(f32(f32(factor * factor) * 64.))


def histogram_bin(luminance):
    if not math.isfinite(luminance) or luminance < 0:
        raise ValueError('Invalid histogram luminance')
    value = 0. if luminance == 0 else min(1., max(0., f32(f32(math.log2(luminance)) * f32(1./18.) + .5)))
    return int(f32(value * 127.))


def histogram_transport_bounds(width, height):
    if not isinstance(width, int) or not isinstance(height, int) or min(width, height) <= 0:
        raise ValueError('Invalid histogram dimensions')
    columns, rows = (width + 1) // 2, (height + 1) // 2
    # At least ceil(width/8) sampled columns have u<.25. Their weight is <=56.
    # All other weights are <=64. This also bounds any single source bin.
    total = (64 * columns - 8 * ((width + 7) // 8)) * rows
    if total > 0xffffffff:
        raise ValueError('Histogram count can exceed uint32')
    return {'tile_maximum': SCATTER_SAMPLES * 64, 'total_maximum': total,
            'sample_count': columns * rows, 'tiles': (columns * rows + SCATTER_SAMPLES - 1) // SCATTER_SAMPLES,
            'texels_per_tile': HISTOGRAM_BINS // 4}


def histogram_channel(bin_index):
    if not 0 <= bin_index < HISTOGRAM_BINS:
        raise ValueError('Invalid histogram bin')
    return bin_index // 4, bin_index % 4

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


def recover_taa(controls):
    # pplayermelle.Start -> TurnOn overwrites the layer's serialized settings.
    return {target: controls[source] for target, source in (
        ('jitter_spread', 'AntialiasingJitterSpread'),
        ('stationary_blending', 'AntialiasingStationaryBlending'),
        ('motion_blending', 'AntialiasingMotionBlending'),
        ('sharpness', 'AntialiasingSharpness'))}


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
    _, controls = read('level1', 64919)
    _, controller = read('level1', 80667)
    bloom_reader, bloom = effect(1371)
    _, grading = effect(1375)
    _, ao = effect(1378)
    _, exposure = effect(1380)
    _, motion = effect(1383)
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
        'taa': recover_taa(controls),
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
        'motion_blur': {'samples': p(motion, 'sampleCount'),
                        'shutter_range': [controller['minShutter'], controller['maxShutter']],
                        'speed_range': [controller['minSpeed'], controller['maxSpeed']],
                        'shutter_lerp': controller['shutterLerp']},
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
