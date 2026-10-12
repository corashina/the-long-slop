#import source_float_store::half2
struct MotionCamera {
    raster_vp: mat4x4<f32>, current_vp: mat4x4<f32>, previous_vp: mat4x4<f32>,
    camera_to_world: mat4x4<f32>, inverse_projection: mat4x4<f32>, projection: vec4<f32>,
}
@group(0) @binding(0) var<uniform> camera: MotionCamera;
@group(0) @binding(1) var camera_depth: texture_2d<f32>;
@vertex fn camera_vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4(uv * 2. - 1., 0., 1.);
}
fn viewport(clip: vec4<f32>) -> vec2<f32> {
    let uv = (clip.xy / clip.w + 1.) * .5;
    return vec2(uv.x, 1. - uv.y);
}
struct CameraOutput { @location(0) velocity: vec2<f32>, @builtin(frag_depth) depth: f32 }
@fragment fn camera_fragment(@builtin(position) position: vec4<f32>) -> CameraOutput {
    let extent = vec2<f32>(textureDimensions(camera_depth));
    let uv = position.xy / extent;
    let pixel = clamp(vec2<i32>(floor(position.xy)), vec2(0), vec2<i32>(extent) - vec2(1));
    let depth = textureLoad(camera_depth, pixel, 0).r;
    let clip = vec4(uv * 2. - 1., -1., 1.);
    let h = camera.inverse_projection * clip;
    var ray = vec3(0., 0., camera.projection.x);
    if abs(h.w) > bitcast<f32>(0x33d6bf95u) { ray = vec3(h.xy / h.w, camera.projection.x); }
    let linear01 = 1. / (camera.projection.z * depth + camera.projection.w);
    let view_position = ray * (camera.projection.y / ray.z) * linear01;
    let world = camera.camera_to_world * vec4(view_position, 1.);
    var output: CameraOutput;
    output.velocity = half2(viewport(camera.current_vp * world) - viewport(camera.previous_vp * world));
    output.depth = depth;
    return output;
}
