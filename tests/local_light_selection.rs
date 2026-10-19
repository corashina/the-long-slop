//! Compile the actual selection and fragment iteration code with the renderer's shader compiler.
use naga::{
    ShaderStage,
    back::glsl,
    valid::{Capabilities, ValidationFlags, Validator},
};

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
fn selection_and_ordered_iteration_compile_for_webgl2() {
    let source = shader();
    let module = naga::front::wgsl::parse_str(&source)
        .unwrap_or_else(|error| panic!("{}", error.emit_to_string(&source)));
    let info = Validator::new(ValidationFlags::all(), Capabilities::empty())
        .validate(&module)
        .unwrap_or_else(|error| panic!("{}", error.emit_to_string(&source)));
    let options = glsl::Options {
        version: glsl::Version::new_gles(300),
        ..Default::default()
    };
    for (shader_stage, entry_point) in [
        (ShaderStage::Vertex, "vertex"),
        (ShaderStage::Fragment, "fragment"),
    ] {
        let mut output = String::new();
        let reflection = glsl::Writer::new(
            &mut output,
            &module,
            &info,
            &options,
            &glsl::PipelineOptions {
                shader_stage,
                entry_point: entry_point.into(),
                multiview: None,
            },
            Default::default(),
        )
        .unwrap()
        .write()
        .unwrap();
        assert!(output.starts_with("#version 300 es"));
        assert!(!output.contains("findLSB"), "Requires GLSL ES 3.10");
        // Selection adds uniform visibility, without adding sampled textures to the vertex stage.
        if shader_stage == ShaderStage::Vertex {
            assert!(reflection.texture_mapping.is_empty());
            assert_eq!(reflection.uniforms.len(), 2);
        }
    }
    let mut layouter = naga::proc::Layouter::default();
    layouter.update(module.to_ctx()).unwrap();
    let lighting = module
        .global_variables
        .iter()
        .find(|(_, global)| global.name.as_deref() == Some("lamp_lighting"))
        .unwrap()
        .1;
    assert!(
        layouter[lighting.ty].size <= 16_384,
        "WebGL2 uniform block limit"
    );
}
