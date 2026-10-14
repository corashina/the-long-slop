#import bevy_pbr::{mesh_functions, mesh_bindings::mesh, forward_io::Vertex, pbr_bindings}
#import bevy_render::maths::mat2x4_f32_to_mat3x3_unpack
#import source_caster_math::{caster_transform, caster_normal_bias, caster_clip_bias, caster_cutout_margin, caster_premultiplied_alpha, caster_dither_coordinate}

struct CasterView {
    clip_from_world: mat4x4<f32>,
    direction_to_light: vec4<f32>,
    bias: vec4<f32>,
}
@group(1) @binding(0) var<uniform> caster_view: CasterView;
@group(1) @binding(1) var caster_dither: texture_3d<f32>;
@group(1) @binding(2) var caster_dither_sampler: sampler;

// Same typed material block and offsets as SourceSurface; only MainTex and scalar alpha controls are active.
struct CasterMaterial {
    color: vec4<f32>, emission: vec4<f32>, specular:vec4<f32>, factors: vec4<f32>, scales: vec4<f32>, flags: vec4<u32>, banks: array<vec4<u32>, 3>, layers:array<vec4<u32>,3>,
    uv: array<vec4<f32>, 9>, sampling: array<vec4<f32>, 9>, defaults: array<vec4<f32>, 9>,
}
@group(#{MATERIAL_BIND_GROUP}) @binding(120) var<uniform> material: CasterMaterial;
#ifdef WORLD_TEXTURE_ARRAYS
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var array0:texture_2d_array<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var sampler0:sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var array1:texture_2d_array<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var sampler1:sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(5) var array2:texture_2d_array<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(6) var sampler2:sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(7) var array3:texture_2d_array<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(8) var sampler3:sampler;
#endif
struct CasterOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}
@vertex
fn vertex(input: Vertex) -> CasterOutput {
    let model = mesh_functions::get_world_from_local(input.instance_index);
    let position = caster_transform(model, vec4(input.position, 1.0));
    let normal_matrix = mat2x4_f32_to_mat3x3_unpack(mesh[input.instance_index].local_from_world_transpose_a, mesh[input.instance_index].local_from_world_transpose_b);
    let raw_normal = normal_matrix * input.normal;
    let normal = raw_normal * inverseSqrt(dot(raw_normal, raw_normal));
    let displaced = caster_normal_bias(position.xyz, normal, normalize(caster_view.direction_to_light.xyz), caster_view.bias.z);
    let clip = caster_transform(caster_view.clip_from_world, vec4(displaced, position.w));
    var output: CasterOutput;
    output.position = caster_clip_bias(clip, caster_view.bias.xy);
    output.uv = vec2(0.0);
#ifdef VERTEX_UVS_A
    output.uv = fma(input.uv, material.uv[0].xy, material.uv[0].zw);
#endif
    return output;
}
@fragment
fn fragment(input: CasterOutput) {
    if (material.flags.x & ((1u << 9u) | (1u << 8u))) == 0u { return; }
    var main_alpha = material.defaults[0].a;
    if (material.flags.y & 1u) != 0u {
        let uv = select(input.uv, abs(input.uv), material.sampling[0].zw > vec2(0.5));
#ifdef WORLD_TEXTURE_ARRAYS
        let layer=i32(material.layers[0].x);let bias=material.sampling[0].y;
        switch material.banks[0].x {
            case 0u:{main_alpha=textureSampleBias(array0,sampler0,uv,layer,bias).a;}
            case 1u:{main_alpha=textureSampleBias(array1,sampler1,uv,layer,bias).a;}
            case 2u:{main_alpha=textureSampleBias(array2,sampler2,uv,layer,bias).a;}
            default:{main_alpha=textureSampleBias(array3,sampler3,uv,layer,bias).a;}
        }
#else
        main_alpha = textureSampleBias(pbr_bindings::base_color_texture, pbr_bindings::base_color_sampler, uv, material.sampling[0].y).a;
#endif
    }
    if (material.flags.x & (1u << 9u)) != 0u {
        if caster_cutout_margin(main_alpha, material.color.a, material.factors.w) < 0.0 { discard; }
    } else {
        let alpha = caster_premultiplied_alpha(main_alpha, material.color.a, material.factors.x);
        // WGPU fragment position is top-left, matching source SV_POSITION. Native A8 maps to target R8.
        let coordinate = caster_dither_coordinate(input.position.xy, alpha);
        if textureSample(caster_dither, caster_dither_sampler, coordinate).r - 0.01 < 0.0 { discard; }
    }
}
