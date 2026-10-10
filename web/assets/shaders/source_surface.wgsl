#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_fragment::pbr_input_from_vertex_output,
    pbr_functions::{calculate_tbn_mikktspace, main_pass_post_lighting_processing},
    pbr_types,
    pbr_bindings,
    mesh_view_bindings,
    mesh_view_types,
    mesh_types::MESH_FLAGS_SHADOW_RECEIVER_BIT,
    shadows,
}

struct SourceUniform {
    color: vec4<f32>,
    emission: vec4<f32>,
    factors: vec4<f32>,
    scales: vec4<f32>,
    flags: vec4<u32>,
    uv: array<vec4<f32>, 9>,
    sampling: array<vec4<f32>, 9>,
    defaults: array<vec4<f32>, 9>,
}
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> source: SourceUniform;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var detail_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var detail_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(103) var mask_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(104) var mask_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(105) var detail_normal_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(106) var detail_normal_sampler: sampler;

fn source_flag(bit: u32) -> bool { return (source.flags.x & (1u << bit)) != 0u; }
fn sample_source(slot: u32, main_uv: vec2<f32>, detail_uv: vec2<f32>, offset: vec2<f32>) -> vec4<f32> {
    if (source.flags.y & (1u << slot)) == 0u { return source.defaults[slot]; }
    var uv = main_uv;
    if source.sampling[slot].x > 0.5 { uv = detail_uv; }
    uv = uv * source.uv[slot].xy + source.uv[slot].zw + offset;
    // Unity MirrorOnce: reflect negative coordinates, then clamp at the edge.
    uv = select(uv, abs(uv), source.sampling[slot].zw > vec2(0.5));
    let bias = source.sampling[slot].y;
    switch slot {
        case 0u: { return textureSampleBias(pbr_bindings::base_color_texture, pbr_bindings::base_color_sampler, uv, bias); }
        case 1u: { return textureSampleBias(pbr_bindings::normal_map_texture, pbr_bindings::normal_map_sampler, uv, bias); }
        case 2u: { return textureSampleBias(pbr_bindings::metallic_roughness_texture, pbr_bindings::metallic_roughness_sampler, uv, bias); }
        case 3u: { return textureSampleBias(pbr_bindings::occlusion_texture, pbr_bindings::occlusion_sampler, uv, bias); }
        case 4u: { return textureSampleBias(pbr_bindings::emissive_texture, pbr_bindings::emissive_sampler, uv, bias); }
        case 5u: { return textureSampleBias(pbr_bindings::depth_map_texture, pbr_bindings::depth_map_sampler, uv, bias); }
        case 6u: { return textureSampleBias(detail_texture, detail_sampler, uv, bias); }
        case 7u: { return textureSampleBias(mask_texture, mask_sampler, uv, bias); }
        default: { return textureSampleBias(detail_normal_texture, detail_normal_sampler, uv, bias); }
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
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
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
    let tbn = calculate_tbn_mikktspace(in.world_normal, in.world_tangent);
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
    var light = diffuse * mesh_view_bindings::lights.ambient_color.rgb * occlusion;
    let view_z = (mesh_view_bindings::view.view_from_world * in.world_position).z;
    for (var i = 0u; i < mesh_view_bindings::lights.n_directional_lights; i += 1u) {
        let source_light = mesh_view_bindings::lights.directional_lights[i];
        var shadow = 1.0;
        if (pbr.flags & MESH_FLAGS_SHADOW_RECEIVER_BIT) != 0u && (source_light.flags & mesh_view_types::DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u {
            shadow = shadows::fetch_directional_shadow(i, in.world_position, pbr.world_normal, view_z, in.position.xy);
        }
        light += source_brdf(diffuse, specular, smoothness, pbr.N, pbr.V, source_light.direction_to_light) * source_light.color.rgb * shadow;
    }
    light *= mesh_view_bindings::view.exposure;
    if source_flag(3u) { light += sample_source(4u, uv, detail_uv, offset).rgb * source.emission.rgb; }
    if source.flags.z == 1u { light = source.color.rgb; alpha = source.color.a; }
    // The separate stained-glass layer supplies grab composition.
    if source.flags.z == 2u { light = albedo; }
    var out: FragmentOutput;
    // Source premultiplication affects diffuse only. Bevy must not multiply specular by alpha again.
    pbr.material.flags = pbr_types::STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT;
    out.color = main_pass_post_lighting_processing(pbr, vec4(light, alpha));
    return out;
}

