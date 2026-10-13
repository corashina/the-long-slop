#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_fragment::pbr_input_from_vertex_output,
    pbr_bindings,
    mesh_view_bindings,
}

#import source_float_store::half4

#import source_view_lighting::{lighting, diffuse_sh, indirect_specular, direct_attenuation, fog_factor, sun_cookie, sun_cookie_sampler}

struct SourceLight {
    cookie_from_world: mat4x4<f32>,
    position_range: vec4<f32>,
    direction_kind: vec4<f32>,
    color: vec4<f32>,
    cookie_rect: vec4<f32>,
    shadow: vec4<f32>,
    mask: vec4<u32>,
}
struct SourceShadow { clip_from_world: mat4x4<f32>, rect: vec4<f32>, bias: vec4<f32> }
struct SourceLighting {
    lights: array<SourceLight,40>,
    shadows: array<SourceShadow,100>,
    ambient: vec4<f32>,
    fog: vec4<f32>,
    counts: vec4<u32>,
}
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> lamp_lighting: SourceLighting;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var shadow_atlas: texture_depth_2d;
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var shadow_sampler: sampler_comparison;
@group(#{MATERIAL_BIND_GROUP}) @binding(103) var cookies: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(104) var cookie_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(105) var attenuation: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(106) var<uniform> receiver: vec4<u32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(107) var default_cookie: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(108) var default_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(111) var attenuation_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(112) var hard_shadow_sampler: sampler_comparison;
// The GLSL backend requires one sampler per shader image. Both bindings use the same depth atlas.
@group(#{MATERIAL_BIND_GROUP}) @binding(113) var hard_shadow_atlas: texture_depth_2d;
fn power5(v:f32)->f32 { let v2=v*v; return v2*v2*v; }
fn source_shadow(light:SourceLight,position:vec3<f32>,normal:vec3<f32>,view_depth:f32)->f32 {
    if light.shadow.y == 0.0 { return 1.0; }
    var face=0u;
    if light.direction_kind.w == 1.0 {
        let delta=position-light.position_range.xyz; let a=abs(delta);
        if a.x>=a.y && a.x>=a.z { face=select(0u,1u,delta.x<0.0); }
        else if a.y>=a.z { face=select(2u,3u,delta.y<0.0); }
        else { face=select(4u,5u,delta.z<0.0); }
    }
    let shadow=lamp_lighting.shadows[u32(light.shadow.x)+face];
    var distance=1.;var to_light=-light.direction_kind.xyz;
    if light.direction_kind.w!=0. {
        let delta=light.position_range.xyz-position;to_light=normalize(delta);
        if light.direction_kind.w==1. {distance=max(abs(delta.x),max(abs(delta.y),abs(delta.z)));}
        else {distance=dot(light.direction_kind.xyz,-delta);}
    }
    let p=shadow.clip_from_world*vec4<f32>(position+normal*shadow.bias.x*distance+to_light*shadow.bias.y,1.0);
    let ndc=p.xyz/p.w;
    if any(abs(ndc.xy)>vec2<f32>(1.0)) || ndc.z<0.0 || ndc.z>1.0 { return 1.0; }
    let uv=(ndc.xy*vec2<f32>(0.5,-0.5)+0.5)*shadow.rect.xy+shadow.rect.zw;
    var sampled:f32;
    if light.mask.z==1u { sampled=textureSampleCompareLevel(hard_shadow_atlas,hard_shadow_sampler,uv,ndc.z); }
    else { sampled=textureSampleCompareLevel(shadow_atlas,shadow_sampler,uv,ndc.z); }
    return mix(1.0,sampled,light.shadow.z);
}

struct SourceUniform {
    color: vec4<f32>,
    emission: vec4<f32>,
    factors: vec4<f32>,
    scales: vec4<f32>,
    flags: vec4<u32>,
    banks: array<vec4<u32>, 3>,
    uv: array<vec4<f32>, 9>,
    sampling: array<vec4<f32>, 9>,
    defaults: array<vec4<f32>, 9>,
}
@group(#{MATERIAL_BIND_GROUP}) @binding(120) var<uniform> source: SourceUniform;
@group(#{MATERIAL_BIND_GROUP}) @binding(121) var bank6_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(122) var bank6_sampler: sampler;

fn source_flag(bit: u32) -> bool { return (source.flags.x & (1u << bit)) != 0u; }
fn sample_source(slot: u32, main_uv: vec2<f32>, detail_uv: vec2<f32>, offset: vec2<f32>) -> vec4<f32> {
    if (source.flags.y & (1u << slot)) == 0u { return source.defaults[slot]; }
    var uv = main_uv;
    if source.sampling[slot].x > 0.5 { uv = detail_uv; }
    uv = uv * source.uv[slot].xy + source.uv[slot].zw + offset;
    // Unity MirrorOnce: reflect negative coordinates, then clamp at the edge.
    uv = select(uv, abs(uv), source.sampling[slot].zw > vec2(0.5));
    let bias = source.sampling[slot].y;
    let bank = source.banks[slot / 4u][slot % 4u];
    switch bank {
        case 0u: { return textureSampleBias(pbr_bindings::base_color_texture, pbr_bindings::base_color_sampler, uv, bias); }
        case 1u: { return textureSampleBias(pbr_bindings::emissive_texture, pbr_bindings::emissive_sampler, uv, bias); }
        case 2u: { return textureSampleBias(pbr_bindings::metallic_roughness_texture, pbr_bindings::metallic_roughness_sampler, uv, bias); }
        case 3u: { return textureSampleBias(pbr_bindings::occlusion_texture, pbr_bindings::occlusion_sampler, uv, bias); }
        case 4u: { return textureSampleBias(pbr_bindings::normal_map_texture, pbr_bindings::normal_map_sampler, uv, bias); }
        case 5u: { return textureSampleBias(pbr_bindings::depth_map_texture, pbr_bindings::depth_map_sampler, uv, bias); }
        default: { return textureSampleBias(bank6_texture, bank6_sampler, uv, bias); }
    }
}
fn source_normal(sampled: vec4<f32>, scale: f32) -> vec3<f32> {
    let xy = (sampled.xy * 2.0 - 1.0) * scale;
    return vec3(xy, sqrt(1.0 - clamp(dot(xy, xy), 0.0, 1.0)));
}
fn pow5(value: f32) -> f32 { let square = value * value; return square * square * value; }

// Unity 2021.3 BRDF1_Unity_PBS, Linear color space, desktop GGX branch.
fn source_brdf(diffuse: vec3<f32>, specular: vec3<f32>, smoothness: f32, normal: vec3<f32>, view: vec3<f32>, light: vec3<f32>) -> vec3<f32> {
    let half_sum = light + view;
    let half_dir = half_sum * inverseSqrt(max(dot(half_sum, half_sum), 0.001));
    let nv = abs(dot(normal, view));
    let nl = clamp(dot(normal, light), 0.0, 1.0);
    let nh = clamp(dot(normal, half_dir), 0.0, 1.0);
    let lh = clamp(dot(light, half_dir), 0.0, 1.0);
    let perceptual = 1.0 - smoothness;
    let roughness = max(perceptual * perceptual, 0.002);
    let fd90 = 0.5 + 2.0 * lh * lh * perceptual;
    let disney = (1.0 + (fd90 - 1.0) * pow5(1.0 - nl)) * (1.0 + (fd90 - 1.0) * pow5(1.0 - nv));
    let visibility = 0.5 / (nl * (nv * (1.0 - roughness) + roughness) + nv * (nl * (1.0 - roughness) + roughness) + 0.00001);
    let a2 = roughness * roughness;
    let d = (nh * a2 - nh) * nh + 1.0;
    var specular_term = max(0.0, visibility * a2 / (d * d + 0.0000001) * nl);
    if source_flag(6u) { specular_term = 0.0; }
    let fresnel = specular + (vec3(1.0) - specular) * pow5(1.0 - lh);
    return diffuse * disney * nl + specular_term * fresnel;
}

@fragment
fn fragment(
    in: VertexOutput,
#ifdef VERTEX_TANGENTS
    @location(8) world_bitangent: vec3<f32>,
#endif
    @location(9) @interpolate(flat) source_renderer: vec3<f32>,
    @location(10) source_fog: f32,
    @location(11) source_screen: vec4<f32>,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var pbr = pbr_input_from_vertex_output(in, is_front, false);
    var uv = vec2(0.0);
#ifdef VERTEX_UVS_A
    uv = in.uv;
#endif
    var detail_uv = uv;
#ifdef VERTEX_UVS_B
    detail_uv = in.uv_b;
#endif
    var offset = vec2(0.0);
#ifdef VERTEX_TANGENTS
    let tbn = mat3x3(in.world_tangent.xyz, world_bitangent, in.world_normal);
    if source_flag(4u) {
        let height = sample_source(5u, uv, detail_uv, vec2(0.0)).g;
        var tangent_view = normalize(transpose(tbn) * pbr.V);
        tangent_view.z += 0.42;
        offset = (height * source.scales.w - source.scales.w * 0.5) * tangent_view.xy / tangent_view.z;
    }
#endif
    let main = sample_source(0u, uv, detail_uv, offset);
    var albedo = main.rgb * source.color.rgb;
    var alpha = main.a * source.color.a;
#ifdef VERTEX_COLORS
    // Target-generated terrain keeps its linear vertex colors. Authored source materials do not use this bit.
    if source_flag(11u) { albedo *= in.color.rgb; alpha *= in.color.a; }
#endif
    if source_flag(5u) { alpha = source.color.a; }
    if source_flag(9u) && alpha < source.factors.w { discard; }
    let mask = sample_source(7u, uv, detail_uv, offset).a;
    if source_flag(1u) {
        albedo *= mix(vec3(1.0), sample_source(6u, uv, detail_uv, offset).rgb * 4.59479380, mask);
    }
#ifdef VERTEX_TANGENTS
    if source_flag(0u) {
        var tangent_normal = source_normal(sample_source(1u, uv, detail_uv, offset), source.scales.x);
        if source_flag(1u) {
            let detail_normal = source_normal(sample_source(8u, uv, detail_uv, offset), source.scales.y);
            let blended = normalize(vec3(tangent_normal.xy + detail_normal.xy, tangent_normal.z * detail_normal.z));
            tangent_normal = mix(tangent_normal, blended, mask);
        }
        pbr.N = normalize(tbn * tangent_normal);
    }
#endif
    var metallic = source.factors.x;
    var smoothness = source.factors.y;
    if source_flag(2u) {
        let metal = sample_source(2u, uv, detail_uv, offset);
        metallic = metal.r;
        smoothness = metal.a * source.factors.z;
    }
    if source_flag(5u) { smoothness = main.a * source.factors.z; }
    let occlusion = mix(1.0, sample_source(3u, uv, detail_uv, offset).g, source.scales.z);
    let specular = mix(vec3(0.04), albedo, metallic);
    let one_minus_reflectivity = 0.96 * (1.0 - metallic);
    var diffuse = albedo * one_minus_reflectivity;
    if source_flag(8u) {
        diffuse *= alpha;
        alpha = 1.0 - one_minus_reflectivity + alpha * one_minus_reflectivity;
    }
    var light: vec3<f32>;
#ifdef SOURCE_FORWARD_ADD
    let view_depth = -(mesh_view_bindings::view.view_from_world * in.world_position).z;
    let receive_shadows = (u32(source_renderer.z) & 2u) != 0u;
    let sun_attenuation = direct_attenuation(in.world_position.xyz,source_screen.xy/source_screen.w,view_depth,receive_shadows);
    light = source_brdf(diffuse,specular,smoothness,pbr.N,pbr.V,lighting.direction_to_sun.xyz)*lighting.sun_radiance.xyz*sun_attenuation;
    for (var i = 0u; i < lamp_lighting.counts.x; i += 1u) {
        let lamp = lamp_lighting.lights[i];
        if lamp.direction_kind.w == 0.0 { continue; }
        if (receiver.x & lamp.mask.x) == 0u { continue; }
        if max(lamp.color.r, max(lamp.color.g, lamp.color.b)) <= 0.0 { continue; }
        var direction = -lamp.direction_kind.xyz;
        var weight = 1.0;
        let local = lamp.cookie_from_world * in.world_position;
        if lamp.direction_kind.w != 0.0 {
            let delta = lamp.position_range.xyz - in.world_position.xyz;
            let d2 = dot(delta, delta);
            let range2 = lamp.position_range.w * lamp.position_range.w;
            if d2 >= range2 { continue; }
            direction = delta * inverseSqrt(max(d2, 0.000001));
            weight = textureSampleLevel(attenuation, attenuation_sampler, vec2(d2 / range2, 0.5), 0.0).r;
        }
        if lamp.cookie_rect.x > 0.0 {
            var cookie_uv = vec2(0.0);
            if lamp.direction_kind.w == 0.0 { cookie_uv = local.xy / lamp.shadow.w + lamp.cookie_rect.yz + 0.5; }
            else {
                if local.z <= 0.0 { continue; }
                cookie_uv = local.xy / local.w + 0.5;
                if any(cookie_uv < vec2(0.0)) || any(cookie_uv > vec2(1.0)) { continue; }
            }
            if lamp.cookie_rect.x == 1.0 { weight *= textureSampleGrad(sun_cookie, sun_cookie_sampler, cookie_uv, dpdx(cookie_uv), dpdy(cookie_uv)).a; }
            else if lamp.cookie_rect.x == 2.0 { weight *= textureSampleGrad(default_cookie, default_sampler, cookie_uv, dpdx(cookie_uv), dpdy(cookie_uv)).a; }
            else { weight *= textureSampleGrad(cookies, cookie_sampler, cookie_uv, dpdx(cookie_uv), dpdy(cookie_uv)).a; }
        }
        if receive_shadows { weight *= source_shadow(lamp, in.world_position.xyz, pbr.world_normal, view_depth); }
        light += source_brdf(diffuse, specular, smoothness, pbr.N, pbr.V, direction) * lamp.color.rgb * weight;
    }

#else
    light = diffuse * diffuse_sh(pbr.N) * occlusion;
    if !source_flag(7u) {
        light += indirect_specular(specular,one_minus_reflectivity,smoothness,pbr.N,pbr.V,occlusion,source_renderer.xy);
    }
    // Glossy-off unity_IndirectSpecColor is RGB0 in the fixed source profile.
    if source_flag(3u) { light += sample_source(4u, uv, detail_uv, offset).rgb * source.emission.rgb; }
    if source.flags.z == 1u { light = source.color.rgb; alpha = 1.0; }
#endif
    // Preserve source Standard OutputForward alpha, including premultiplied specular.
    if source.flags.z == 0u && !source_flag(8u) && !source_flag(10u) { alpha = 1.0; }
#ifdef SOURCE_UNLIT
    let fog = clamp(source_fog,0.0,1.0);
#else
    let fog = fog_factor(source_fog,lighting.near_far.x,lighting.near_far.y);
#endif
#ifdef SOURCE_FORWARD_ADD
    light *= fog;
#else
    light = mix(lighting.fog_color_density.xyz,light,fog);
#endif
    var out: FragmentOutput;
    out.color = vec4(light,alpha);
#ifdef SOURCE_REPLACEMENT
    out.color = half4(out.color);
#endif
    return out;
}
