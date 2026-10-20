use my_summer_slop::{
    reference::shadow_candidates::DirectionalHull,
    world::{REQUIRED_VIEW_CAPACITY, RequiredView, RequiredViews},
};
use naga::{
    ShaderStage,
    back::glsl,
    valid::{Capabilities, ValidationFlags, Validator},
};

#[test]
fn one_probe_shader_has_one_cube_and_matching_uniform_layout() {
    let lighting = include_str!("../web/assets/shaders/source_view_lighting.wgsl")
        .lines()
        .filter(|line| !line.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    let source = format!(
        r#"{lighting}
struct Output {{
    @builtin(position) position: vec4<f32>,
    @location(0) @interpolate(flat) weight: f32,
}}
@vertex fn vertex(@location(0) position: vec3<f32>) -> Output {{
    var out: Output;
    out.position = vec4(position, 1.0);
    out.weight = probe_weight(mat4x4(vec4(1.0,0.0,0.0,0.0),vec4(0.0,1.0,0.0,0.0),
        vec4(0.0,0.0,1.0,0.0),vec4(0.0,0.0,0.0,1.0)), vec4(0.0,0.0,0.0,1.0),vec4(1.0));
    return out;
}}
@fragment fn fragment(@location(0) @interpolate(flat) weight: f32) -> @location(0) vec4<f32> {{
    return vec4(indirect_specular(vec3(0.5),0.5,0.5,vec3(0.0,1.0,0.0),vec3(0.0,0.0,1.0),1.0,weight),1.0);
}}
"#
    );
    let module = naga::front::wgsl::parse_str(&source)
        .unwrap_or_else(|error| panic!("{}", error.emit_to_string(&source)));
    let info = Validator::new(ValidationFlags::all(), Capabilities::empty())
        .validate(&module)
        .unwrap_or_else(|error| panic!("{}", error.emit_to_string(&source)));
    let mut layout = naga::proc::Layouter::default();
    layout.update(module.to_ctx()).unwrap();
    let uniform = module
        .global_variables
        .iter()
        .find(|(_, value)| value.name.as_deref() == Some("lighting"))
        .unwrap()
        .1;
    assert_eq!(layout[uniform.ty].size, 736);
    let mut bindings: Vec<_> = module
        .global_variables
        .iter()
        .filter_map(|(_, value)| {
            value
                .binding
                .as_ref()
                .map(|binding| (binding.group, binding.binding))
        })
        .collect();
    bindings.sort();
    assert_eq!(
        bindings,
        (0..7).map(|binding| (1, binding)).collect::<Vec<_>>()
    );
    for (shader_stage, entry_point, textures) in [
        (ShaderStage::Vertex, "vertex", 0),
        (ShaderStage::Fragment, "fragment", 1),
    ] {
        let mut output = String::new();
        let reflection = glsl::Writer::new(
            &mut output,
            &module,
            &info,
            &glsl::Options {
                version: glsl::Version::new_gles(300),
                ..Default::default()
            },
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
        assert_eq!(reflection.texture_mapping.len(), textures);
        assert!(output.starts_with("#version 300 es"));
    }
}

#[test]
fn required_views_retain_one_world_and_six_probe_faces() {
    let planes = [
        [1., 0., 0., 1.],
        [-1., 0., 0., 1.],
        [0., 1., 0., 1.],
        [0., -1., 0., 1.],
        [0., 0., 1., 1.],
        [0., 0., -1., 1.],
    ];
    let view = RequiredView {
        camera: planes,
        shadow: DirectionalHull::new(planes, [1., 0., 0.]).unwrap(),
        mask: 1,
        light_mask: 1,
        origin: [0.; 3],
    };
    let mut views = RequiredViews::new(view);
    let storage = views.as_slice().as_ptr();
    let mut faces = Vec::new();
    views
        .append_probe_faces(&[true], |probe, face| {
            faces.push((probe, face));
            Ok(view)
        })
        .unwrap();
    assert_eq!(faces, (0..6).map(|face| (0, face)).collect::<Vec<_>>());
    assert_eq!(views.as_slice().len(), 7);
    assert_eq!(REQUIRED_VIEW_CAPACITY, 7);
    assert!(views.push(view).is_err());
    views.reset(view);
    views
        .append_probe_faces(&[false], |_, _| panic!("Disabled probe face"))
        .unwrap();
    assert_eq!(views.as_slice().len(), 1);
    assert_eq!(views.as_slice().as_ptr(), storage);
}
