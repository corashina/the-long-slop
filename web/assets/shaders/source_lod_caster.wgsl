#import bevy_pbr::{prepass_io::VertexOutput,pbr_prepass_functions::prepass_alpha_discard}
#import "shaders/source_lod.wgsl"::source_lod_keep
@group(#{MATERIAL_BIND_GROUP}) @binding(120) var<uniform> fade: vec4<f32>;
@fragment
fn fragment(input:VertexOutput)
#ifdef UNCLIPPED_DEPTH_ORTHO_EMULATION
 -> @builtin(frag_depth) f32
#endif
{
    if !source_lod_keep(input.position.xy,fade.x) {discard;}
    prepass_alpha_discard(input);
#ifdef UNCLIPPED_DEPTH_ORTHO_EMULATION
    return input.unclipped_depth;
#endif
}
