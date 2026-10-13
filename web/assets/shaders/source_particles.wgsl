// Selected Unity particle alpha blending and camera-facing billboards.
#import bevy_pbr::mesh_view_bindings::view
struct Particles {
    positions:array<vec4<f32>,128>,colors:array<vec4<f32>,128>,angles:array<vec4<f32>,128>,
    tint:vec4<f32>,st:vec4<f32>,fog:vec4<f32>,settings:vec4<f32>,
}
@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> particles:Particles;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var image:texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var image_sampler:sampler;
struct Vertex {@location(0) position:vec3<f32>,@location(1) uv:vec2<f32>}
struct Varyings {@builtin(position) position:vec4<f32>,@location(0) uv:vec2<f32>,@location(1) color:vec4<f32>,@location(2) depth:f32}
@vertex fn vertex(v:Vertex)->Varyings {
    let i=u32(v.position.z);let p=particles.positions[i];let angle=particles.angles[i];
    var center=view.view_from_world*vec4(p.xyz,1.);
    let diameter=clamp(p.w,particles.settings.y*abs(center.z)*2./abs(view.clip_from_view[1][1]),particles.settings.z*abs(center.z)*2./abs(view.clip_from_view[1][1]));
    let corner=vec2(v.position.x*angle.x-v.position.y*angle.y,v.position.x*angle.y+v.position.y*angle.x)*diameter;
    center.x+=corner.x;center.y+=corner.y;
    var o:Varyings;o.position=view.clip_from_view*center;o.uv=v.uv*particles.st.xy+particles.st.zw;
    o.color=particles.colors[i]*particles.tint*particles.settings.x;let a=view.clip_from_view[2][2];let b=view.clip_from_view[3][2];let near=b/(a+1.);let far=b/a;
    o.depth=max((abs(center.z)-near)*far/(far-near),0.);
    if i>=u32(particles.settings.w) {o.position=vec4(2.,2.,2.,1.);}
    return o;
}
@fragment fn fragment(v:Varyings)->@location(0) vec4<f32> {
    var c=textureSample(image,image_sampler,v.uv)*v.color;
    c.a=clamp(c.a,0.,1.);
    let fog=exp2(-1.442695*pow(particles.fog.w*v.depth,2.));
    c=vec4(mix(particles.fog.rgb,c.rgb,fog),c.a);
    return c;
}
