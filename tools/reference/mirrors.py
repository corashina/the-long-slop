"""Recover the selected Lada capture contract from checked library records."""
import math
from .source import key, source_id

SELECTED = (66131, 67237, 70638)


def prepare_mirror_materials(source, root):
    main = source.get('level1', 64919)
    data = source.read(main)
    source.mirror_live_material = source.resolve(main, data['defMirrorMaterial'])
    source.mirror_disabled_material = source.resolve(main, data['noMirrorMaterial'])
    source.mirror_renderers = {}
    for node in source.hierarchy(root, activate_root=True):
        for reader in node['components']:
            if reader.type.name == 'MonoBehaviour' and source.class_name(reader) == 'mirrorscript':
                value = source.read(reader)['M']
                if reader.path_id in SELECTED:
                    for field in ('R', 'T', 'C', 'CT'):
                        if not value[field]['m_PathID']:
                            raise ValueError(f'level1:{reader.path_id} M.{field}: required link')
                    source.mirror_renderers[key(source.resolve(reader, value['R']))] = source.mirror_live_material


def recover_mirrors(source, nodes, components, pointer_node, item_nodes, reports):
    main = source.get('level1', 64919)
    md = source.read(main)
    settings = source.get('level0', 16865)
    sd = source.read(settings)
    slider = source.resolve(settings, sd['tabGraphics']['sliders'][1])
    slider_data = source.read(slider)
    toggle = source.resolve(settings, sd['tabGraphics']['toggles'][3])
    enabled = bool(source.read(toggle)['def'])
    draw_distance = slider_data['defValue'] * (slider_data['valueMultipler'] or 1.)
    dropdown = source.resolve(settings, sd['tabGraphics']['dropdrops'][4])
    dd = source.read(dropdown)
    render_path = dd['defInt'] if dd['useDefInt'] else 0 if dd['defFirstOrLast'] else len(dd['words'])-1
    target_reader = source.resolve(main, md['defMirrorTexture'])
    target_data = source.read(target_reader)
    sampler = target_data['m_TextureSettings']
    def material(field):
        reader = source.resolve(main, md[field])
        identity = f'{reader.assets_file.name}:{reader.path_id}'
        if identity not in reports['lada']['material_indices']:
            raise ValueError(f'level1:64919 {field}: required material {identity}')
        return {'source':source_id(reader), 'asset':2,
                'gltf_material':reports['lada']['material_indices'][identity]}
    live, disabled = material('defMirrorMaterial'), material('noMirrorMaterial')
    by_id = {n['id']:n for n in nodes}
    readers = {r.path_id:(node,r) for node,group in components.items() for r in group.get('mirrorscript',[]) if key(r)[0]=='level1'}
    result = []
    for identity in SELECTED:
        if identity not in readers: raise ValueError(f'level1:{identity} mirrorscript: required capture')
        node, reader = readers[identity]
        data = source.read(reader)['M']
        def link(field):
            pointer = data[field]
            if not pointer['m_PathID']: raise ValueError(f'level1:{identity} M.{field}: required link')
            try: return pointer_node(reader,pointer)
            except (KeyError, AttributeError) as error:
                raise ValueError(f'level1:{identity} M.{field}: missing node') from error
        surface, plane, camera_node = link('R'), link('T'), link('CT')
        camera = source.resolve(reader,data['C']) if data['C']['m_PathID'] else None
        if camera is None: raise ValueError(f'level1:{identity} M.C: required link')
        if link('C') != camera_node: raise ValueError(f'level1:{identity} M.CT: camera transform differs')
        cd = source.read(camera)
        scripts = components[camera_node].get('camscript',[])
        if len(scripts)!=1: raise ValueError(f'level1:{identity} camscript: required camera policy')
        script=scripts[0]; policy=source.read(script)
        if policy['updMirrors'] or policy['postprocessing'] or policy['depth']['m_PathID'] or policy['heathaze']['m_PathID']:
            raise ValueError(f'level1:{identity} camscript: recursive capture or effects')
        owner_node=node
        while owner_node not in item_nodes:
            owner_node=by_id[owner_node]['parent']
            if owner_node is None: raise ValueError(f'level1:{identity} owner: missing item')
        values=[cd['near clip plane'],cd['far clip plane'],cd['field of view'],draw_distance]
        if not all(math.isfinite(v) for v in values): raise ValueError(f'level1:{identity} camera: nonfinite projection')
        result.append({'source':source_id(reader),'owner':item_nodes[owner_node], 'owner_node':owner_node,
            'surface_node':surface,'surface_source':source_id(source.resolve(reader,data['R'])),
            'plane_node':plane,'camera_node':camera_node,'camera_source':source_id(camera),
            'camera_script_source':source_id(script),'near_plane':cd['near clip plane'],
            'serialized_far_plane':cd['far clip plane'],'initial_far_plane':cd['far clip plane'],
            'serialized_fov_degrees':cd['field of view'],'culling_mask':cd['m_CullingMask']['m_Bits'],
            'hdr':bool(cd['m_HDR']),'clear_flags':cd['m_ClearFlags'],
            'clear_color':[cd['m_BackGroundColor'][c] for c in ('r','g','b','a')],
            'occlusion_culling':bool(cd['m_OcclusionCulling']),'allow_msaa':bool(cd['m_AllowMSAA']),
            'serialized_rendering_path':cd['m_RenderingPath'],'rendering_path':1 if render_path==0 else 3,
            'orthographic':bool(cd['orthographic']),'update_mirrors':bool(policy['updMirrors']),
            'postprocessing_sources':[],'depth_source':None,'heat_haze_source':None,
            'live_material':live,'disabled_material':disabled,'live_texture_slots':['_MainTex','_EmissionMap']})
    return {'rules':{'source':source_id(main),'settings_source':source_id(settings),
        'draw_distance_source':source_id(slider),'enabled_source':source_id(toggle),
        'enabled':enabled,'draw_distance_meters':draw_distance,
        'fov_bounds_degrees':[md['minFov'],md['maxFov']],
        'distance_bounds_meters':[md['minDist'],md['maxDist']],'count_limit':5},
        'target':{'source':source_id(target_reader),'extent':[target_data['m_Width'],target_data['m_Height']],
            'samples':target_data['m_AntiAliasing'],'color_format_code':target_data['m_ColorFormat'],
            'depth_stencil_format_code':target_data['m_DepthStencilFormat'],'linear':not target_data['m_SRGB'],
            'filter':sampler['m_FilterMode'],'wrap':[sampler['m_WrapU'],sampler['m_WrapV'],sampler['m_WrapW']],
            'mip_sampling':bool(target_data['m_MipMap']),'generate_mips':bool(target_data['m_GenerateMips']),
            'mip_count':target_data['m_MipCount'],'dynamic_scale':bool(target_data['m_UseDynamicScale'])},
        'mirrors':result}


def mirror_source_evidence(source, contract):
    from .library.reader import checked
    import json
    library = source.library
    managed = library.manifest['managed_analysis']
    checked(library.records_root, managed['manifest'])
    receipts = {item['file']:item for item in managed['files']}
    code = checked(library.records_root, receipts['managed/Assembly-CSharp.cs']).decode('utf-8')
    decomp = json.loads(checked(library.records_root, receipts['managed/decompilation.json']))
    assembly = 'TheLongDrive_Data/Managed/Assembly-CSharp.dll'
    if decomp['input']['sha256'] != library.source_files[assembly].sha256:
        raise ValueError('Mirror managed source provenance mismatch')
    library.source_range(assembly, 0, library.source_files[assembly].bytes)
    for expression in ('M.rt = new RenderTexture(mainscript.M.defMirrorTexture);',
                       'M.mat.SetTexture("_EmissionMap", M.rt);',
                       'if (!settingsscript.s.S.BMirrors || !crsrLocked)',
                       'Mirrors[num].C.farClipPlane = settingsscript.s.S.FMirrorDrawDistance * num2;'):
        if expression not in code: raise ValueError(f'Mirror executed source differs: {expression}')
    identities = [contract['rules'][field] for field in ('source','settings_source','draw_distance_source','enabled_source')]
    identities.append(contract['target']['source'])
    for mirror in contract['mirrors']:
        identities += [mirror[field] for field in ('source','camera_source','camera_script_source','surface_source')]
        identities += [mirror[field]['source'] for field in ('live_material','disabled_material')]
    records = []
    for file, path_id in sorted({(s['file'],s['path_id']) for s in identities}):
        source.raw_data(source.get(file,path_id))
        records.append(library.record_files[f'{file}/{path_id}.json'])
    return {'contract':contract,'records':records,'managed':managed,
            'assembly':{'file':assembly,'sha256':decomp['input']['sha256']},
            'formats':{'8':'R8G8B8A8_UNorm','92':'D24_UNorm_S8_UInt'},
            'startup':'Inactive starter ancestors defer manual mirror registration until after startup settings refresh. FStart keeps serialized far; SettingsRefreshed assigns draw distance before its capture.'}
