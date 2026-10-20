//! Compile the actual selection and fragment iteration code with the renderer's shader compiler.
use naga::valid::{Capabilities, ValidationFlags, Validator};

fn shader() -> String {
    let lighting = include_str!("../web/assets/shaders/source_lighting.wgsl")
        .replace("#{MATERIAL_BIND_GROUP}", "3");
    let surface = include_str!("../web/assets/shaders/source_surface.wgsl");
    let start = surface.find("    // Visit low bits first.").unwrap();
    let end = surface[start..].find("        var direction =").unwrap() + start;
    let iteration = &surface[start..end];
    format!(
        r#"{lighting}
struct Output {{
    @builtin(position) position: vec4<f32>,
    @location(12) @interpolate(flat) source_lights: vec2<u32>,
}}
@vertex fn vertex(@location(0) position: vec3<f32>) -> Output {{
    var out: Output;
    out.position = vec4(position, 1.0);
    out.source_lights = object_light_mask(
        mat4x4(vec4(1.0,0.0,0.0,0.0),vec4(0.0,1.0,0.0,0.0),
            vec4(0.0,0.0,1.0,0.0),vec4(0.0,0.0,0.0,1.0)),
        vec3(0.0), vec3(1.0), true);
    return out;
}}
@fragment fn fragment(@location(12) @interpolate(flat) source_lights: vec2<u32>) -> @location(0) vec4<f32> {{
    var result = vec4(0.0);
{iteration}
        result += lamp.color;
    }}
    return result;
}}
"#
    )
}

#[test]
fn selection_and_ordered_iteration_validate() {
    let source = shader();
    let module = naga::front::wgsl::parse_str(&source)
        .unwrap_or_else(|error| panic!("{}", error.emit_to_string(&source)));
    Validator::new(ValidationFlags::all(), Capabilities::empty())
        .validate(&module)
        .unwrap_or_else(|error| panic!("{}", error.emit_to_string(&source)));
}
