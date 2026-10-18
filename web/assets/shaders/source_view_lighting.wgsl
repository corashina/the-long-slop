// Source startup lighting. Binding order matches SourceViewLighting.
#define_import_path source_view_lighting
struct LightingUniform {
    ambient_sh: array<vec4<f32>, 7>,
    sun_radiance: vec4<f32>,
    direction_to_sun: vec4<f32>,
    cookie_from_world: mat4x4<f32>,
    fog_color_density: vec4<f32>,
    shadow_fade: vec4<f32>,
    shadow_fade_center: vec4<f32>,
    probe_center: array<vec4<f32>, 2>,
    probe_extents: array<vec4<f32>, 2>,
    source_time: vec4<f32>,
    clip_from_world: mat4x4<f32>,
    near_far: vec4<f32>,
}
@group(1) @binding(0) var<uniform> lighting: LightingUniform;
@group(1) @binding(1) var sun_cookie: texture_2d<f32>;
@group(1) @binding(2) var sun_cookie_sampler: sampler;
@group(1) @binding(3) var local_probe: texture_cube<f32>;
@group(1) @binding(4) var local_probe_sampler: sampler;
@group(1) @binding(5) var screen_shadow: texture_2d<f32>;
@group(1) @binding(6) var screen_shadow_sampler: sampler;
@group(1) @binding(7) var local_probe_second: texture_cube<f32>;

// Both selected car probes have equal importance and equal axis-aligned box sizes.
// Unity 2021.3 BlendProbes weights use renderer/probe intersection volumes.
// Renderer extents retain the recovered native Vector3f::epsilon clamp.
fn probe_weight(model: mat4x4<f32>, center_usage: vec4<f32>, extents_flags: vec4<f32>) -> vec2<f32> {
    if center_usage.w == 0.0 { return vec2(0.0); }
    let center = (model * vec4(center_usage.xyz,1.0)).xyz;
    let extents = max(abs(model[0].xyz)*extents_flags.x
        + abs(model[1].xyz)*extents_flags.y + abs(model[2].xyz)*extents_flags.z,
        vec3(9.999999747378752e-06));
    var volumes = vec2(0.0);
    for (var index = 0u; index < 2u; index++) {
        if lighting.probe_extents[index].w == 0.0 { continue; }
        let minimum = max(center-extents, lighting.probe_center[index].xyz-lighting.probe_extents[index].xyz);
        let maximum = min(center+extents, lighting.probe_center[index].xyz+lighting.probe_extents[index].xyz);
        let size = max(maximum-minimum, vec3(0.0));
        volumes[index] = size.x*size.y*size.z;
    }
    let total = volumes.x+volumes.y;
    if total == 0.0 { return vec2(0.0); }
    return volumes/total;
}
fn diffuse_sh(normal: vec3<f32>) -> vec3<f32> {
    // ShadeSH9 in source world axes. Flat startup SH has only the constant term.
    let n=vec4(normal.x,normal.y,-normal.z,1.0);
    let linear=vec3(dot(lighting.ambient_sh[0],n),dot(lighting.ambient_sh[1],n),dot(lighting.ambient_sh[2],n));
    let products=n.xyzz*n.yzzx;
    let quadratic=vec3(dot(lighting.ambient_sh[3],products),dot(lighting.ambient_sh[4],products),dot(lighting.ambient_sh[5],products));
    return max(vec3(0.0),linear+quadratic+lighting.ambient_sh[6].xyz*(n.x*n.x-n.y*n.y));
}
fn indirect_specular(specular: vec3<f32>,one_minus_reflectivity: f32,smoothness: f32,
    normal: vec3<f32>,view: vec3<f32>,occlusion: f32,weight: vec2<f32>) -> vec3<f32> {
    let reflected=reflect(-view,normal);
    // Native face storage retains source axes; target world coordinates reflect Z.
    let source_ray=vec3(reflected.x,reflected.y,-reflected.z);
    let perceptual=1.0-smoothness;
    let level=(perceptual*(1.7-0.7*perceptual))*6.0;
    // RGBA16F carries linear radiance directly, equivalent to native FP decode.
    // During capture this binding uses the proven black local-probe override.
    let first=textureSampleLevel(local_probe,local_probe_sampler,source_ray,level).rgb;
    let second=textureSampleLevel(local_probe_second,local_probe_sampler,source_ray,level).rgb;
    let radiance=(first*weight.x+second*weight.y)*occlusion;
    let roughness=max(perceptual*perceptual,0.002);
    let reduction=1.0/(roughness*roughness+1.0);
    let grazing=clamp(smoothness+(1.0-one_minus_reflectivity),0.0,1.0);
    let f=1.0-abs(dot(normal,view));let f2=f*f;let f5=f2*f2*f;
    return reduction*radiance*(specular+(vec3(grazing)-specular)*f5);
}
fn direct_attenuation(world_position: vec3<f32>,screen_uv: vec2<f32>,view_depth: f32,receive_shadows: bool) -> f32 {
    let cookie_uv=(lighting.cookie_from_world*vec4(world_position,1.0)).xy;
    let cookie=textureSample(sun_cookie,sun_cookie_sampler,cookie_uv).a;
    let shadow=textureSample(screen_shadow,screen_shadow_sampler,screen_uv).r;
    let sphere_distance=distance(world_position,lighting.shadow_fade_center.xyz);
    let distance=mix(view_depth,sphere_distance,lighting.shadow_fade_center.w);
    let fade=clamp(distance*lighting.shadow_fade.z+lighting.shadow_fade.w,0.0,1.0);
    return cookie*select(1.0,mix(shadow,1.0,fade),receive_shadows);
}
fn fog_factor(source_clip_z: f32,near: f32,far: f32) -> f32 {
    // D3D11 reversed-Z UnityCG::UNITY_Z_0_FAR_FROM_CLIPSPACE, then FOG_EXP2.
    let distance=max((1.0-source_clip_z/near)*far,0.0);
    let exponent=(lighting.fog_color_density.w*1.2011224031448364)*distance;
    return clamp(exp2(-exponent*exponent),0.0,1.0);
}
