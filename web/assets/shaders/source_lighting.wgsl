struct SourceLight {
    cookie_from_world: mat4x4<f32>,
    position_range: vec4<f32>,
    direction_kind: vec4<f32>,
    color: vec4<f32>,
    cookie_rect: vec4<f32>,
    mask: vec4<u32>,
    cookie_size: f32,
}
struct SourceLighting {
    lights: array<SourceLight,64>,
    ambient: vec4<f32>,
    fog: vec4<f32>,
    counts: vec4<u32>,
}
@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> lamp_lighting: SourceLighting;
@group(#{MATERIAL_BIND_GROUP}) @binding(103) var cookies: texture_2d_array<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(104) var cookie_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(105) var attenuation: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(106) var<uniform> receiver: vec4<u32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(107) var default_cookie: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(108) var default_sampler: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(111) var attenuation_sampler: sampler;
fn power5(v:f32)->f32 { let v2=v*v; return v2*v2*v; }
