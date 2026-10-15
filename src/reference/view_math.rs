//! Recovered source culling selection and coordinate-history transport.
pub type Matrix = [[f32; 4]; 4];
/// GetScriptableCullingParameters450da0..450e4a. Corners are source world points.
pub fn requires_implicit_projection(
    corners: [[f32; 3]; 8],
    position: [f32; 3],
    near: f32,
    far: f32,
) -> bool {
    let ratio = far / near;
    for i in 0..4 {
        let predicted: [f32; 3] = std::array::from_fn(|axis| {
            position[axis] + (corners[i][axis] - position[axis]) * ratio
        });
        let mismatch: [f32; 3] = std::array::from_fn(|axis| predicted[axis] - corners[i + 4][axis]);
        let span: [f32; 3] = std::array::from_fn(|axis| corners[i + 4][axis] - corners[i][axis]);
        let square = |v: [f32; 3]| (v[0] * v[0] + v[1] * v[1]) + v[2] * v[2];
        if square(span) * 0.009999999776482582_f32 < square(mismatch) {
            return true;
        }
    }
    false
}
/// Culling inputs use source-world coordinates. Target world equals S * source world.
/// Right multiplication by S negates column2; camera/view space remains unchanged.
pub fn source_culling_view(mut target_view: Matrix) -> Matrix {
    for value in &mut target_view[2] {
        *value = -*value;
    }
    target_view
}
