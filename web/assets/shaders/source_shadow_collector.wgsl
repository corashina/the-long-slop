// Installed Hidden/Internal-ScreenSpaceShadows blobs46/60; sphere cascades and 16-tap PCF7x7.
struct Collector {
    inverse_projection: mat4x4<f32>,
    world_from_view: mat4x4<f32>,
    world_to_shadow: array<mat4x4<f32>, 4>,
    spheres: array<vec4<f32>, 4>,
    radii_squared: vec4<f32>,
    // x=far/near-1, y=1, z=1-strength, w=forced inverse projection
    projection: vec4<f32>,
    // inverse atlas extent, atlas extent, has valid cascades, unused
    atlas: vec4<f32>,
};
@group(0) @binding(0) var<uniform> params: Collector;
@group(0) @binding(1) var camera_depth: texture_2d<f32>;
@group(0) @binding(2) var shadow_atlas: texture_depth_2d;
@group(0) @binding(3) var shadow_compare: sampler_comparison;
struct Vertex { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32>, @location(1) ray: vec3<f32> };
fn source_mul(m: mat4x4<f32>, v: vec4<f32>) -> vec4<f32> {
    return ((m[1]*v.y + m[0]*v.x) + m[2]*v.z) + m[3]*v.w;
}
@vertex fn vertex(@builtin(vertex_index) index: u32) -> Vertex {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    var output: Vertex;
    output.position = vec4<f32>(uv.x*2.-1., 1.-uv.y*2., 0., 1.);
    output.uv = uv;
    // Source ray uses the implicit perspective far plane. Camera-space Z remains
    // negative here; source's Z flip and CameraToWorld's Z flip cancel exactly.
    let far = source_mul(params.inverse_projection, vec4<f32>(uv*2.-1., 1., 1.));
    output.ray = far.xyz / far.w;
    return output;
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
    result.offset = (odd/result.weight + vec4<f32>(-3.5,-1.5,.5,2.5))*params.atlas.x;
    return result;
}
fn filtered(coord: vec3<f32>) -> f32 {
    let texel = coord.xy*params.atlas.y;
    let center = floor(texel+.5);
    let horizontal = tent(texel.x-center.x);
    let vertical = tent(texel.y-center.y);
    let origin = center*params.atlas.x;
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
@fragment fn collect(input: Vertex) -> @location(0) vec4<f32> {
    if(params.atlas.z==0.) { return vec4<f32>(1.); }
    let size = vec2<i32>(textureDimensions(camera_depth));
    let pixel = clamp(vec2<i32>(floor(input.uv * vec2<f32>(size))), vec2(0), size - vec2(1));
    let depth = textureLoad(camera_depth, pixel, 0).r;
    var camera = input.ray / (params.projection.x*depth+params.projection.y);
    if(params.projection.w!=0.) {
        let clip = vec4<f32>(input.uv*2.-1., (1.-depth)*2.-1., 1.);
        let projected = source_mul(params.inverse_projection,clip);
        camera = projected.xyz/projected.w;
    }
    let world = source_mul(params.world_from_view,vec4<f32>(camera,1.));
    var weights = vec4<f32>(0.);
    for(var i=0u;i<4u;i++) {
        let difference = world.xyz-params.spheres[i].xyz;
        weights[i] = select(0.,1.,dot(difference,difference)<params.radii_squared[i]);
    }
    weights = vec4<f32>(weights.x,max(weights.yzw-weights.xyz,vec3<f32>(0.)));
    // Installed operation order starts with cascade1, then adds cascade0,2,3.
    var coord = source_mul(params.world_to_shadow[1],world).xyz*weights.y;
    coord = source_mul(params.world_to_shadow[0],world).xyz*weights.x+coord;
    coord = source_mul(params.world_to_shadow[2],world).xyz*weights.z+coord;
    coord = source_mul(params.world_to_shadow[3],world).xyz*weights.w+coord;
    coord.z = (coord.z-dot(weights,vec4<f32>(1.)))+1.;
    let visibility = filtered(coord)*(1.-params.projection.z)+params.projection.z;
    return vec4<f32>(visibility);
}
