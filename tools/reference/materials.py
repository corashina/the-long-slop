"""Decode source normal channels before writing glTF textures."""
import math

from PIL import Image, ImageMath

from .source import source_id


def decode_normal_rgba(pixel: tuple[int, int, int, int], encoding: str) -> tuple[float, float, float]:
    if encoding not in {'ag', 'rg', 'rgb'}:
        raise ValueError(f'Unknown normal encoding: {encoding}')
    x = pixel[3 if encoding == 'ag' else 0] * 2 / 255 - 1
    y = pixel[1] * 2 / 255 - 1
    z = pixel[2] * 2 / 255 - 1 if encoding == 'rgb' else math.sqrt(max(0., 1 - x*x - y*y))
    return x, y, z


def decode_normal_image(image: Image.Image, encoding: str) -> Image.Image:
    if encoding not in {'ag', 'rg', 'rgb'}:
        raise ValueError(f'Unknown normal encoding: {encoding}')
    red, green, blue, alpha = image.convert('RGBA').split()
    if encoding == 'rgb':
        return Image.merge('RGB', (red, green, blue))
    red = alpha if encoding == 'ag' else red
    x = red.point(lambda value: value * 2 / 255 - 1, mode='F')
    y = green.point(lambda value: value * 2 / 255 - 1, mode='F')
    z = ImageMath.lambda_eval(lambda p: p['max'](1 - p['x']**2 - p['y']**2, 0)**0.5, x=x, y=y)
    blue = z.point(lambda value: (value + 1) * 127.5 + 0.5).convert('L')
    return Image.merge('RGB', (red, green, blue))


def normal_encoding(texture, shader, shader_name: str, identity: dict) -> str:
    # Unity 2021.3.18f1 uses UnpackNormal/UnpackScaleNormal on D3D11.
    # DXT5nm stores X in alpha; BC5 stores X in red. Both store Y in green.
    platforms = shader['platforms'] if shader else []
    if shader_name in {'Standard', 'FX/Glass/Stained BumpDistort'} and set(platforms) == {4}:
        from UnityPy.enums import TextureFormat
        formats = {TextureFormat.DXT5: 'ag', TextureFormat.DXT5Crunched: 'ag', TextureFormat.BC5: 'rg'}
        if texture['m_TextureFormat'] in formats:
            return formats[texture['m_TextureFormat']]
    raise ValueError(f'Unresolved normal encoding: {identity} _BumpMap; '
                     f'shader={shader_name}, platforms={platforms}, format={texture['m_TextureFormat']}')

# Defaults come from the installed shader property table and Unity 2021.3.18f1 Standard.shader.
STANDARD_FLOATS = {'_Cutoff': .5, '_Glossiness': .5, '_GlossMapScale': 1.,
    '_SmoothnessTextureChannel': 0., '_Metallic': 0., '_SpecularHighlights': 1.,
    '_GlossyReflections': 1., '_BumpScale': 1., '_Parallax': .02,
    '_OcclusionStrength': 1., '_DetailNormalMapScale': 1., '_UVSec': 0.,
    '_Mode': 0., '_SrcBlend': 1., '_DstBlend': 0., '_ZWrite': 1.}
TEXTURE_DEFAULTS = {'_MainTex': 'white', '_BumpMap': 'bump',
    '_DetailAlbedoMap': 'grey', '_DetailMask': 'white', '_DetailNormalMap': 'bump',
    '_MetallicGlossMap': 'white', '_OcclusionMap': 'white', '_EmissionMap': 'white',
    '_ParallaxMap': 'black'}
NORMAL_SLOTS = {'_BumpMap', '_DetailNormalMap'}
FAMILIES = {'Standard': 'standard', 'Unlit/Color': 'unlit_color',
            'FX/Glass/Stained BumpDistort': 'stained_glass', 'AmplifyRope': 'amplify_rope'}
KEYWORDS = {'_NORMALMAP', '_DETAIL_MULX2', '_METALLICGLOSSMAP', '_EMISSION',
    '_PARALLAXMAP', '_SMOOTHNESS_TEXTURE_ALBEDO_CHANNEL_A', '_SPECULARHIGHLIGHTS_OFF',
    '_GLOSSYREFLECTIONS_OFF', '_ALPHATEST_ON', '_ALPHABLEND_ON', '_ALPHAPREMULTIPLY_ON'}


def required_pointer(source, owner, pointer, property_name):
    try:
        reader = source.resolve(owner, pointer)
    except Exception as error:
        raise ValueError(f'{source_id(owner)} {property_name}: unresolved source pointer') from error
    if pointer['m_PathID'] and reader is None:
        raise ValueError(f'{source_id(owner)} {property_name}: unresolved source pointer')
    return reader


def shader_defaults(shader, family):
    floats = dict(STANDARD_FLOATS) if family == 'standard' else {}
    colors = {} if family == 'amplify_rope' else {'_Color': [1., 1., 1., 1.]}
    textures = dict(TEXTURE_DEFAULTS)
    parsed = shader.get('m_ParsedForm')
    if parsed:
        for prop in parsed['m_PropInfo']['m_Props']:
            values = [prop[f'm_DefValue[{i}]'] for i in range(4)]
            if prop['m_Type'] in (2, 3): floats[prop['m_Name']] = values[0]
            elif prop['m_Type'] in (0, 1): colors[prop['m_Name']] = values
            elif prop['m_Type'] == 4: textures[prop['m_Name']] = prop['m_DefTexture']['m_DefaultName']
    if family == 'standard': colors.setdefault('_EmissionColor', [0., 0., 0., 0.])
    return floats, colors, textures


def recover_material(source, reader, color_override=None, condition=None):
    data = source.read(reader)
    shader_reader = required_pointer(source, reader, data['m_Shader'], 'm_Shader')
    if shader_reader is None: raise ValueError(f'{source_id(reader)} m_Shader: missing shader')
    shader = source.read(shader_reader)
    shader_name = shader['m_Name'] or shader['m_ParsedForm']['m_Name']
    family = FAMILIES.get(shader_name)
    if family is None: raise ValueError(f'{source_id(reader)} required shader: {shader_name}')
    keywords = sorted(data.get('m_ValidKeywords', data.get('m_ShaderKeywords', '').split()))
    if family == 'standard' and set(keywords) - KEYWORDS:
        raise ValueError(f'{source_id(reader)} {shader_name}: unsupported keywords {set(keywords) - KEYWORDS}')
    floats, colors, defaults = shader_defaults(shader, family)
    saved = data['m_SavedProperties']
    floats.update(dict(saved.get('m_Floats', [])))
    colors.update({name: [value[c] for c in ('r', 'g', 'b', 'a')]
                   for name, value in saved.get('m_Colors', [])})
    if not all(math.isfinite(v) for v in floats.values()) or not all(
            math.isfinite(v) for color in colors.values() for v in color):
        raise ValueError(f'{source_id(reader)} nonfinite shader property')
    color = [color_override[c] for c in ('r', 'g', 'b', 'a')] if color_override is not None else colors.get('_Color', [1.,1.,1.,1.])
    if not all(math.isfinite(value) for value in color):
        raise ValueError(f'{source_id(reader)} _Color: nonfinite override')
    active = []
    if family == 'standard':
        active = ['_MainTex', '_OcclusionMap']
        if '_NORMALMAP' in keywords: active.append('_BumpMap')
        if '_DETAIL_MULX2' in keywords:
            active += ['_DetailMask', '_DetailAlbedoMap']
            if '_NORMALMAP' in keywords: active.append('_DetailNormalMap')
        for keyword, slot in [('_METALLICGLOSSMAP', '_MetallicGlossMap'),
                              ('_EMISSION', '_EmissionMap'), ('_PARALLAXMAP', '_ParallaxMap')]:
            if keyword in keywords: active.append(slot)
    elif family == 'stained_glass': active = ['_MainTex', '_BumpMap']
    environments = dict(saved.get('m_TexEnvs', []))
    identity_uv = {'m_Scale': {'x': 1., 'y': 1.}, 'm_Offset': {'x': 0., 'y': 0.}}
    textures = []; active_defaults = {}; exclusions = []
    for slot in active:
        env = environments.get(slot)
        pointer = env['m_Texture'] if env else {'m_PathID': 0}
        texture_reader = required_pointer(source, reader, pointer, slot)
        if texture_reader is None:
            active_defaults[slot] = defaults[slot]; continue
        if texture_reader.type.name == 'RenderTexture':
            raise ValueError(f'{source_id(reader)} {slot}: unsupported RenderTexture {source_id(texture_reader)}')
        if texture_reader.type.name != 'Texture2D':
            raise ValueError(f'{source_id(reader)} {slot}: required Texture2D')
        texture = source.read(texture_reader); settings = texture['m_TextureSettings']
        encoding = normal_encoding(texture, shader, shader_name, source_id(texture_reader)) if slot in NORMAL_SLOTS else None
        # Native texture upload uses stored TextureColorSpace: Linear=0, SRGB=1.
        stored_space = texture['m_ColorSpace']
        if stored_space not in (0, 1):
            raise ValueError(f'{source_id(texture_reader)} unsupported stored color space')
        color_space = 'srgb' if stored_space == 1 else 'linear'
        if encoding and color_space != 'linear':
            raise ValueError(f'{source_id(texture_reader)} unresolved normal color space')
        uv_env = env if family == 'stained_glass' else environments.get(
            '_DetailAlbedoMap' if slot in {'_DetailAlbedoMap', '_DetailNormalMap'} else '_MainTex', identity_uv)
        uv_channel = int(floats['_UVSec'] != 0) if family == 'standard' and slot in {
            '_DetailAlbedoMap', '_DetailNormalMap'} else 0
        scale = [uv_env['m_Scale'][c] for c in ('x', 'y')]
        # UnityPy flips source images; export reverses that flip. UV coordinates stay authored.
        offset = [uv_env['m_Offset'][c] for c in ('x', 'y')]
        if not all(math.isfinite(v) for v in scale + offset):
            raise ValueError(f'{source_id(reader)} {slot}: nonfinite UV transform')
        if settings['m_FilterMode'] not in (0, 1, 2) or settings['m_WrapU'] not in (0, 1, 2, 3) or settings['m_WrapV'] not in (0, 1, 2, 3):
            raise ValueError(f'{source_id(reader)} {slot}: unsupported sampler')
        suffix = f'-normal-{encoding}' if encoding else ''
        name = f'texture-{texture_reader.assets_file.name}-{texture_reader.path_id}-{color_space}{suffix}.png'
        width, height = texture['m_Width'], texture['m_Height']
        mips = [{'file': f'reference/{name}' if level == 0 else f'reference/{name[:-4]}-mip-{level}.png',
                 'width': max(1, width >> level), 'height': max(1, height >> level)}
                for level in range(texture['m_MipCount'])]
        textures.append({'slot': slot, 'source': source_id(texture_reader), 'file': f'reference/{name}',
            'color_space': color_space, 'encoding': 'rgb' if encoding else 'rgba',
            'source_encoding': encoding or 'rgba', 'uv_channel': uv_channel, 'scale': scale, 'offset': offset,
            'sampler': {'filter': settings['m_FilterMode'], 'wrap_u': settings['m_WrapU'],
                'wrap_v': settings['m_WrapV'], 'aniso': settings['m_Aniso'], 'mip_bias': settings['m_MipBias'],
                'mip_count': texture['m_MipCount']}, 'mips': mips})
    if source_id(reader) == {'file': 'sharedassets1.assets', 'path_id': 624}:
        exclusions.extend({'slot': slot, 'source': entry, 'reason': 'live_mirror'}
                          for entry in getattr(source, 'live_mirror_sources', [])
                          for slot in ('_MainTex', '_EmissionMap'))
    alpha = 'opaque'
    if family == 'standard':
        alpha = ('premultiply' if '_ALPHAPREMULTIPLY_ON' in keywords else
                 'blend' if '_ALPHABLEND_ON' in keywords else 'mask' if '_ALPHATEST_ON' in keywords else 'opaque')
    elif family == 'stained_glass': alpha = 'refract'
    state = {'alpha_mode': alpha, 'cull': 2, 'z_write': bool(floats.get('_ZWrite', 1)),
        'z_test': 4, 'src_blend': int(floats.get('_SrcBlend', 1)),
        'dst_blend': int(floats.get('_DstBlend', 0)), 'render_queue': data.get('m_CustomRenderQueue', -1)}
    parsed = shader.get('m_ParsedForm')
    if parsed:
        render_pass = next(p for p in parsed['m_SubShaders'][0]['m_Passes'] if p['m_Type'] == 0)
        def value(field): return floats.get(field['name'], field['val'])
        pass_state = render_pass['m_State']
        state.update(cull=int(value(pass_state['culling'])), z_write=bool(value(pass_state['zWrite'])),
                     z_test=int(value(pass_state['zTest'])),
                     src_blend=int(value(pass_state['rtBlend0']['srcBlend'])),
                     dst_blend=int(value(pass_state['rtBlend0']['destBlend'])))
    return {'source': source_id(reader), 'shader_family': family, 'color': color,
        'enable_instancing': bool(data['m_EnableInstancingVariants']),
        'condition': condition, 'keywords': keywords, 'state': state,
        'properties': {'floats': floats, 'colors': colors, 'textures': textures,
                       'defaults': active_defaults, 'exclusions': exclusions}}


def source_mip_images(texture):
    """Decode the authored levels. Never resize level zero to invent source mips."""
    from UnityPy.enums import TextureFormat as TF
    from UnityPy.export.Texture2DConverter import parse_image_data
    import texture2ddecoder
    if texture.m_MipCount == 1:
        yield texture.image; return
    fmt = TF(texture.m_TextureFormat); raw = texture.get_image_data(); offset = 0
    plain = {TF.DXT1Crunched: TF.DXT1, TF.DXT5Crunched: TF.DXT5}.get(fmt, fmt)
    block = {TF.DXT1: 8, TF.DXT5: 16, TF.BC5: 16, TF.BC7: 16}
    texel = {TF.RGBA32: 4, TF.RGB24: 3, TF.Alpha8: 1}
    if plain not in block and plain not in texel:
        raise ValueError(f'{source_id(texture.object_reader)} required source mips: {fmt.name}')
    for level in range(texture.m_MipCount):
        width, height = max(1, texture.m_Width >> level), max(1, texture.m_Height >> level)
        size = max(1, (width + 3) // 4) * max(1, (height + 3) // 4) * block[plain] if plain in block else width * height * texel[plain]
        pixels = texture2ddecoder.unpack_unity_crunch(raw, level) if fmt != plain else raw[offset:offset + size]
        if len(pixels) != size: raise ValueError(f'{source_id(texture.object_reader)} source mip {level}: invalid bytes')
        yield parse_image_data(pixels, width, height, plain, texture.object_reader.version,
                               texture.object_reader.platform)
        offset += size


def export_material_textures(source, owner, record, output):
    import hashlib
    environments = dict(source.read(owner)['m_SavedProperties'].get('m_TexEnvs', []))
    dependencies = {}
    cache = getattr(source, 'material_texture_cache', None)
    if cache is None: source.material_texture_cache = cache = set()
    for entry in record['properties']['textures']:
        reader = required_pointer(source, owner, environments[entry['slot']]['m_Texture'], entry['slot'])
        destination = output / entry['file'].removeprefix('reference/')
        identity = str(destination.resolve())
        if identity not in cache:
            texture = source.texture_object(reader)
            for mip, image in zip(entry['mips'], source_mip_images(texture), strict=True):
                pixels = decode_normal_image(image, entry['source_encoding']) if entry['slot'] in NORMAL_SLOTS else image.convert('RGBA')
                pixels.transpose(Image.Transpose.FLIP_TOP_BOTTOM).save(output / mip['file'].removeprefix('reference/'))
            cache.add(identity)
        for mip in entry['mips']:
            path = output / mip['file'].removeprefix('reference/')
            if not path.is_file(): raise ValueError(f'{record["source"]} {entry["slot"]}: missing {mip["file"]}')
            dependencies[mip['file']] = {'file': mip['file'], 'sha256': hashlib.sha256(path.read_bytes()).hexdigest()}
    return list(dependencies.values())


def material_source_evidence(source, reader, record):
    import hashlib
    data = source.read(reader)
    shader_reader = required_pointer(source, reader, data['m_Shader'], 'm_Shader')
    shader = source.read(shader_reader)
    if not hasattr(source, 'material_evidence'): source.material_evidence = {}
    identity = f'{reader.assets_file.name}:{reader.path_id}'
    if identity in source.material_evidence: return
    channels = {'_MainTex': 'RGB albedo; A alpha or smoothness by keyword',
        '_BumpMap': 'source R*A,G unpack; exported RGB; scale XY then reconstruct Z',
        '_DetailMask': 'A mask', '_DetailAlbedoMap': 'RGB multiply x unity_ColorSpaceDouble',
        '_DetailNormalMap': 'source R*A,G unpack; whiteout BlendNormals; mask lerp',
        '_MetallicGlossMap': 'R metallic; A smoothness unless albedo-alpha keyword; GlossMapScale',
        '_OcclusionMap': 'G occlusion; lerp(1,G,OcclusionStrength)',
        '_EmissionMap': 'RGB times EmissionColor only with EMISSION',
        '_ParallaxMap': 'G height; ParallaxOffset1Step offsets main and detail UV'}
    if record['shader_family'] == 'stained_glass':
        channels = {
            '_MainTex': 'RGBA times grab sample and Color; independent MainTex ST',
            '_BumpMap': 'source R*A,G unpack; XY times BumpAmt and GrabTexelSize; depth scale; project by clip W',
        }
    source.material_evidence[identity] = {
        'source': source_id(reader), 'sha256': hashlib.sha256(source.raw_data(reader)).hexdigest(),
        'shader': source_id(shader_reader), 'shader_sha256': hashlib.sha256(source.raw_data(shader_reader)).hexdigest(),
        'shader_name': shader['m_Name'] or shader['m_ParsedForm']['m_Name'], 'platforms': shader['platforms'],
        'official_unity_version': '2021.3.18f1',
        'official_shader_archive_sha256': '8fd9157efb775fc395e0e26e48c46b2195a3c66d234d20922caf06769b74794b',
        'keywords': record['keywords'], 'disabled_passes': data['disabledShaderPasses'],
        'passes': [{'subshader': index, 'active': index == 0, 'type': p['m_Type'],
                    'state': p['m_State'], 'tags': p['m_Tags']}
                   for index, sub in enumerate(shader['m_ParsedForm']['m_SubShaders']) for p in sub['m_Passes']],
        'channel_samples': SOURCE_CHANNEL_SAMPLES,
        'state': record['state'], 'channels': {slot: channels[slot] for slot in
            [*record['properties']['defaults'], *[t['slot'] for t in record['properties']['textures']]]},
        'textures': record['properties']['textures'], 'defaults': record['properties']['defaults'],
        'exclusions': record['properties']['exclusions'],
        'transport_limits': (['Imported glTF retains the existing white PBR rope material.',
            'Source rope sag and sway are not implemented.'] if record['shader_family'] == 'amplify_rope' else
            ['Base glTF does not apply layered shader operations.',
             'Final binding must upload authored mips, source anisotropy and mip bias.']),
    }


# UnityStandardInput.cginc Albedo and MetallicGloss; retain inputs for GPU probes.
SOURCE_CHANNEL_SAMPLES = [
    dict(operation='detail_albedo', color_space='gamma', detail=.2, mask_alpha=.5, albedo_multiplier=.7),
    dict(operation='smoothness', keyword='_SMOOTHNESS_TEXTURE_ALBEDO_CHANNEL_A',
         main_alpha=.75, gloss_map_scale=.4, smoothness=.3),
    dict(operation='detail_albedo', color_space='linear', detail=.2, mask_alpha=.5,
         unity_color_space_double=4.59479380, albedo_multiplier=.95947938),
]
