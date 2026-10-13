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
    lights: array<SourceLight,32>,
    shadows: array<SourceShadow,100>,
    ambient: vec4<f32>,
    fog: vec4<f32>,
    counts: vec4<u32>,
    cascade_bounds: vec4<f32>,
    fog_projection: vec4<f32>,
}
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> lighting: SourceLighting;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var shadow_atlas: texture_depth_2d;
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var shadow_sampler: sampler_comparison;
@group(#{MATERIAL_BIND_GROUP}) @binding(103) var cookies: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(104) var cookie_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(105) var attenuation: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(106) var<uniform> receiver: vec4<u32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(107) var default_cookie: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(108) var default_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(109) var sun_cookie: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(110) var sun_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(111) var attenuation_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(112) var hard_shadow_sampler: sampler_comparison;
// The GLSL backend requires one sampler per shader image. Both bindings use the same depth atlas.
@group(#{MATERIAL_BIND_GROUP}) @binding(113) var hard_shadow_atlas: texture_depth_2d;
fn power5(v:f32)->f32 { let v2=v*v; return v2*v2*v; }
fn source_shadow(light:SourceLight,position:vec3<f32>,normal:vec3<f32>,view_depth:f32)->f32 {
    if light.shadow.y == 0.0 { return 1.0; }
    var face=0u;
    if light.direction_kind.w == 0.0 {
        if view_depth > lighting.cascade_bounds.w { return 1.0; }
        face=u32(view_depth>lighting.cascade_bounds.x)+u32(view_depth>lighting.cascade_bounds.y)+u32(view_depth>lighting.cascade_bounds.z);
    } else if light.direction_kind.w == 1.0 {
        let delta=position-light.position_range.xyz; let a=abs(delta);
        if a.x>=a.y && a.x>=a.z { face=select(0u,1u,delta.x<0.0); }
        else if a.y>=a.z { face=select(2u,3u,delta.y<0.0); }
        else { face=select(4u,5u,delta.z<0.0); }
    }
    let shadow=lighting.shadows[u32(light.shadow.x)+face];
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
fn direct_brdf(base:vec3<f32>,metal:f32,perceptual:f32,n:vec3<f32>,v:vec3<f32>,l:vec3<f32>)->vec3<f32> {
    let nl=max(dot(n,l),0.0); let nv=abs(dot(n,v));
    let h=normalize(l+v); let nh=max(dot(n,h),0.0); let lh=max(dot(l,h),0.0);
    let rough=max(perceptual*perceptual,0.002);
    let fd90=0.5+2.0*lh*lh*perceptual;
    let diffuse=(1.0+(fd90-1.0)*power5(1.0-nl))*(1.0+(fd90-1.0)*power5(1.0-nv))*nl;
    let lambda_v=nl*(nv*(1.0-rough)+rough);
    let lambda_l=nv*(nl*(1.0-rough)+rough);
    let visibility=0.5/(lambda_v+lambda_l+0.00001);
    let a2=rough*rough; let d=(nh*a2-nh)*nh+1.0;
    let distribution=a2/(3.14159265*d*d+0.0000001);
    let specular=visibility*distribution*3.14159265*nl;
    let f0=mix(vec3<f32>(0.04),base,metal);
    let fresnel=f0+(vec3<f32>(1.0)-f0)*power5(1.0-lh);
    return base*(1.0-metal)*0.96*diffuse+fresnel*specular;
}
