#import bevy_pbr::forward_io::Vertex
#import source_float_store::half2
struct MotionCamera {
    raster_vp: mat4x4<f32>, current_vp: mat4x4<f32>, previous_vp: mat4x4<f32>,
    camera_to_world: mat4x4<f32>, inverse_projection: mat4x4<f32>, projection: vec4<f32>,
}
struct MotionObject { current_model: mat4x4<f32>, previous_model: mat4x4<f32>, control: vec4<u32> }
@group(1) @binding(0) var<uniform> camera: MotionCamera;
@group(3) @binding(0) var<uniform> object: MotionObject;
struct ObjectOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) current_clip: vec4<f32>, @location(1) previous_clip: vec4<f32>,
}
@vertex fn object_vertex(input: Vertex) -> ObjectOutput {
    let world = object.current_model * vec4(input.position, 1.);
    var output: ObjectOutput;
    output.position = camera.raster_vp * world;
    output.current_clip = camera.current_vp * world;
    output.previous_clip = camera.previous_vp * object.previous_model * vec4(input.position, 1.);
    return output;
}
fn viewport(clip: vec4<f32>) -> vec2<f32> {
    let uv = (clip.xy / clip.w + 1.) * .5;
    return vec2(uv.x, 1. - uv.y);
}
@fragment fn object_fragment(input: ObjectOutput) -> @location(0) vec2<f32> {
    let velocity = viewport(input.current_clip) - viewport(input.previous_clip);
    return half2(select(velocity, vec2(0.), object.control.x != 0u));
}
