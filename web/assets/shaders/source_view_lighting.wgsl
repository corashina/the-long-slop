// Source startup lighting. Binding order matches SourceViewLighting.
#define_import_path source_view_lighting
// Installed Hidden/Internal-ScreenSpaceShadows blobs46/60, sampled at the fragment instead of a screen pass.
struct SunShadow {
    world_to_shadow: array<mat4x4<f32>, 4>,
    spheres: array<vec4<f32>, 4>,
    radii_squared: vec4<f32>,
    // x=1-strength, y=inverse atlas extent, z=atlas extent, w=has valid cascades
    params: vec4<f32>,
}
struct LightingUniform {
    ambient_sh: array<vec4<f32>, 7>,
    sun_radiance: vec4<f32>,
    direction_to_sun: vec4<f32>,
    cookie_from_world: mat4x4<f32>,
    fog_color_density: vec4<f32>,
    shadow_fade: vec4<f32>,
    shadow_fade_center: vec4<f32>,
    probe_center: array<vec4<f32>, 1>,
    probe_extents: array<vec4<f32>, 1>,
    source_time: vec4<f32>,
    clip_from_world: mat4x4<f32>,
    near_far: vec4<f32>,
    sun_shadow: SunShadow,
}
@group(1) @binding(0) var<uniform> lighting: LightingUniform;
@group(1) @binding(1) var sun_cookie: texture_2d<f32>;
@group(1) @binding(2) var sun_cookie_sampler: sampler;
@group(1) @binding(3) var local_probe: texture_cube<f32>;
@group(1) @binding(4) var local_probe_sampler: sampler;
@group(1) @binding(5) var shadow_atlas: texture_depth_2d;
@group(1) @binding(6) var shadow_compare: sampler_comparison;

// The selected Lada has one probe. Positive intersection gives it full weight.
// Renderer extents retain the recovered native Vector3f::epsilon clamp.
fn probe_weight(model: mat4x4<f32>, center_usage: vec4<f32>, extents_flags: vec4<f32>) -> f32 {
    if center_usage.w == 0.0 || lighting.probe_extents[0].w == 0.0 { return 0.0; }
    let center = (model * vec4(center_usage.xyz,1.0)).xyz;
    let extents = max(abs(model[0].xyz)*extents_flags.x
        + abs(model[1].xyz)*extents_flags.y + abs(model[2].xyz)*extents_flags.z,
        vec3(9.999999747378752e-06));
    let minimum = max(center-extents, lighting.probe_center[0].xyz-lighting.probe_extents[0].xyz);
    let maximum = min(center+extents, lighting.probe_center[0].xyz+lighting.probe_extents[0].xyz);
    let size = max(maximum-minimum, vec3(0.0));
    return select(0.0, 1.0, size.x*size.y*size.z > 0.0);
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
    normal: vec3<f32>,view: vec3<f32>,occlusion: f32,weight: f32) -> vec3<f32> {
    let reflected=reflect(-view,normal);
    // Native face storage retains source axes; target world coordinates reflect Z.
    let source_ray=vec3(reflected.x,reflected.y,-reflected.z);
    let perceptual=1.0-smoothness;
    let level=(perceptual*(1.7-0.7*perceptual))*6.0;
    // RGBA16F carries linear radiance directly, equivalent to native FP decode.
    // During capture this binding uses the proven black local-probe override.
    let radiance=textureSampleLevel(local_probe,local_probe_sampler,source_ray,level).rgb*weight*occlusion;
    let roughness=max(perceptual*perceptual,0.002);
    let reduction=1.0/(roughness*roughness+1.0);
    let grazing=clamp(smoothness+(1.0-one_minus_reflectivity),0.0,1.0);
    let f=1.0-abs(dot(normal,view));let f2=f*f;let f5=f2*f2*f;
    return reduction*radiance*(specular+(vec3(grazing)-specular)*f5);
}
fn source_mul(m: mat4x4<f32>, v: vec4<f32>) -> vec4<f32> {
    return ((m[1]*v.y + m[0]*v.x) + m[2]*v.z) + m[3]*v.w;
}
struct Tent { weight: vec4<f32>, offset: vec4<f32> };
fn tent(offset: f32) -> Tent {
    let square = (offset + .5)*(offset + .5);
    let negative = min(offset, 0.);
    let positive = max(offset, 0.);
    // Retain the installed constants and arithmetic grouping.
    let even = vec4<f32>((square*.5-offset)*.081632,
        (2.-offset)*.081632, ((1.+offset)-positive*positive+2.)*.081632,
        (1.+offset)*.081632);
    let odd = vec4<f32>((1.-offset)*.081632,
        ((1.-offset)-negative*negative+2.)*.081632,
        (2.+offset)*.081632, square*.040816);
    var result: Tent;
    result.weight = even + odd;
    result.offset = (odd/result.weight + vec4<f32>(-3.5,-1.5,.5,2.5))*lighting.sun_shadow.params.y;
    return result;
}
// 16-tap PCF7x7.
fn filtered_shadow(coord: vec3<f32>) -> f32 {
    let texel = coord.xy*lighting.sun_shadow.params.z;
    let center = floor(texel+.5);
    let horizontal = tent(texel.x-center.x);
    let vertical = tent(texel.y-center.y);
    let origin = center*lighting.sun_shadow.params.y;
    let a = textureSampleCompareLevel(shadow_atlas,shadow_compare,
        origin+vec2<f32>(horizontal.offset.x,vertical.offset.x),coord.z);
    let b = textureSampleCompareLevel(shadow_atlas,shadow_compare,
        origin+vec2<f32>(horizontal.offset.y,vertical.offset.x),coord.z);
    var result = b*(horizontal.weight.y*vertical.weight.x);
    result = (horizontal.weight.x*vertical.weight.x)*a + result;
    for(var y=0u;y<4u;y++) {
        for(var x=0u;x<4u;x++) {
            if(y==0u && x<2u) { continue; }
            let value = textureSampleCompareLevel(shadow_atlas,shadow_compare,
                origin+vec2<f32>(horizontal.offset[x],vertical.offset[y]),coord.z);
            result = (horizontal.weight[x]*vertical.weight[y])*value + result;
        }
    }
    return result;
}
// Sphere cascades. The installed operation order starts with cascade1, then adds cascade0,2,3.
fn sun_shadow_visibility(world_position: vec3<f32>) -> f32 {
    if(lighting.sun_shadow.params.w==0.) { return 1.; }
    let world = vec4<f32>(world_position,1.);
    var weights = vec4<f32>(0.);
    for(var i=0u;i<4u;i++) {
        let difference = world_position-lighting.sun_shadow.spheres[i].xyz;
        weights[i] = select(0.,1.,dot(difference,difference)<lighting.sun_shadow.radii_squared[i]);
    }
    weights = vec4<f32>(weights.x,max(weights.yzw-weights.xyz,vec3<f32>(0.)));
    var coord = source_mul(lighting.sun_shadow.world_to_shadow[1],world).xyz*weights.y;
    coord = source_mul(lighting.sun_shadow.world_to_shadow[0],world).xyz*weights.x+coord;
    coord = source_mul(lighting.sun_shadow.world_to_shadow[2],world).xyz*weights.z+coord;
    coord = source_mul(lighting.sun_shadow.world_to_shadow[3],world).xyz*weights.w+coord;
    coord.z = (coord.z-dot(weights,vec4<f32>(1.)))+1.;
    return filtered_shadow(coord)*(1.-lighting.sun_shadow.params.x)+lighting.sun_shadow.params.x;
}
fn direct_attenuation(world_position: vec3<f32>,view_depth: f32,receive_shadows: bool) -> f32 {
    let cookie_uv=(lighting.cookie_from_world*vec4(world_position,1.0)).xy;
    let cookie=textureSample(sun_cookie,sun_cookie_sampler,cookie_uv).a;
    let shadow=sun_shadow_visibility(world_position);
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
