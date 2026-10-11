// Unity 2021.3 built-in shader algebra. See UNITY-LICENSE.txt.
#import bevy_pbr::{forward_io::{VertexOutput, FragmentOutput}, pbr_fragment::pbr_input_from_standard_material, pbr_functions::{alpha_discard, main_pass_post_lighting_processing}, mesh_view_bindings::view, pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT, mesh_types::MESH_FLAGS_SHADOW_RECEIVER_BIT}
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
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> source: SourceLighting;
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
        if view_depth > source.cascade_bounds.w { return 1.0; }
        face=u32(view_depth>source.cascade_bounds.x)+u32(view_depth>source.cascade_bounds.y)+u32(view_depth>source.cascade_bounds.z);
    } else if light.direction_kind.w == 1.0 {
        let delta=position-light.position_range.xyz; let a=abs(delta);
        if a.x>=a.y && a.x>=a.z { face=select(0u,1u,delta.x<0.0); }
        else if a.y>=a.z { face=select(2u,3u,delta.y<0.0); }
        else { face=select(4u,5u,delta.z<0.0); }
    }
    let shadow=source.shadows[u32(light.shadow.x)+face];
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
@fragment
fn fragment(in:VertexOutput,@builtin(front_facing) front:bool)->FragmentOutput {
    var p=pbr_input_from_standard_material(in,front);
    p.material.base_color=alpha_discard(p.material,p.material.base_color);
    let depth=max(-(view.view_from_world*p.world_position).z,0.0);
    var rgb=p.material.base_color.rgb*source.ambient.rgb*p.diffuse_occlusion+p.material.emissive.rgb;
    if (p.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT)!=0u { rgb=p.material.base_color.rgb; }
    else {
        for (var i=0u;i<source.counts.x;i+=1u) {
            let light=source.lights[i];
            if (receiver.x & light.mask.x)==0u { continue; }
            if max(light.color.r,max(light.color.g,light.color.b))<=0.0 { continue; }
            var l=-light.direction_kind.xyz;
            var weight=1.0;
            let local=light.cookie_from_world*p.world_position;
            if light.direction_kind.w!=0.0 {
                let delta=light.position_range.xyz-p.world_position.xyz;
                let d2=dot(delta,delta); let range2=light.position_range.w*light.position_range.w;
                if d2>=range2 { continue; }
                l=delta*inverseSqrt(max(d2,0.000001));
                weight=textureSampleLevel(attenuation,attenuation_sampler,vec2<f32>(d2/range2,0.5),0.0).r;
            }
            if light.cookie_rect.x>0.0 {
                var uv=vec2<f32>(0.0);
                if light.direction_kind.w==0.0 { uv=local.xy/light.shadow.w+light.cookie_rect.yz+0.5; }
                else {
                    if local.z<=0.0 { continue; }
                    uv=local.xy/local.w+0.5;
                    if any(uv<vec2<f32>(0.0)) || any(uv>vec2<f32>(1.0)) { continue; }
                }
                if light.cookie_rect.x==1.0 { weight*=textureSampleGrad(sun_cookie,sun_sampler,uv,dpdx(uv),dpdy(uv)).a; }
                else if light.cookie_rect.x==2.0 { weight*=textureSampleGrad(default_cookie,default_sampler,uv,dpdx(uv),dpdy(uv)).a; }
                else { weight*=textureSampleGrad(cookies,cookie_sampler,uv,dpdx(uv),dpdy(uv)).a; }
            }
            if (p.flags & MESH_FLAGS_SHADOW_RECEIVER_BIT)!=0u {weight*=source_shadow(light,p.world_position.xyz,p.world_normal,depth);}
            rgb+=direct_brdf(p.material.base_color.rgb,p.material.metallic,p.material.perceptual_roughness,p.N,p.V,l)*light.color.rgb*weight;
        }
    }
    var planes=source.fog_projection.xy;
    if depth>planes.y && (receiver.x & source.counts.z)!=0u {planes=source.fog_projection.zw;}
    // Unity D3D11 UNITY_Z_0_FAR_FROM_CLIPSPACE applied to the authored projection.
    let fog_depth=max((depth-planes.x)*planes.y/(planes.y-planes.x),0.);
    let fog=exp2(-1.442695*pow(source.ambient.w*fog_depth,2.0));
    rgb=mix(source.fog.rgb,rgb,fog);
    var out:FragmentOutput;
    out.color=main_pass_post_lighting_processing(p,vec4<f32>(rgb,p.material.base_color.a));
    return out;
}
