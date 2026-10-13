//! Installed TAA sample sequence and Unity camera projection operations.
//! The caller converts the resulting source projection to the target clip convention.
#[derive(Default)]
pub struct TaaSequence {
    sample: u32,
}
#[derive(Clone, Copy, Debug)]
pub struct TaaJitter {
    pub pixels: [f32; 2],
    pub uv: [f32; 2],
}
impl TaaSequence {
    pub fn advance(&mut self, spread: f32, extent: [u32; 2]) -> Result<TaaJitter, &'static str> {
        if !(0. ..=1.).contains(&spread) || extent.contains(&0) {
            return Err("Invalid source TAA jitter parameters");
        }
        let index = (self.sample & 1023) + 1;
        let pixels = [
            (halton(index, 2) - 0.5) * spread,
            (halton(index, 3) - 0.5) * spread,
        ];
        self.sample += 1;
        if self.sample >= 8 {
            self.sample = 0;
        }
        Ok(TaaJitter {
            pixels,
            uv: [pixels[0] / extent[0] as f32, pixels[1] / extent[1] as f32],
        })
    }
}
fn halton(mut index: u32, radix: u32) -> f32 {
    let mut result = 0.;
    let mut fraction = 1. / radix as f32;
    while index > 0 {
        result += (index % radix) as f32 * fraction;
        index /= radix;
        fraction /= radix as f32;
    }
    result
}
/// Source row-major projection. Only elements [0][2] and [1][2] change.
pub fn jitter_perspective(
    mut source_projection: [[f32; 4]; 4],
    near: f32,
    vertical_fov_degrees: f32,
    aspect: f32,
    extent: [u32; 2],
    jitter: TaaJitter,
) -> Result<[[f32; 4]; 4], &'static str> {
    if !source_projection
        .iter()
        .flatten()
        .all(|value| value.is_finite())
        || !near.is_finite()
        || near <= 0.
        || !vertical_fov_degrees.is_finite()
        || vertical_fov_degrees <= 0.
        || vertical_fov_degrees >= 180.
        || !aspect.is_finite()
        || aspect <= 0.
        || extent.contains(&0)
        || !jitter.pixels.iter().all(|value| value.is_finite())
    {
        return Err("Invalid source TAA projection parameters");
    }
    // Preserve installed IL operation order instead of canceling the frustum extents.
    let height = (0.008726646192371845_f32 * vertical_fov_degrees).tan() * near;
    let width = height * aspect;
    let offset_x = jitter.pixels[0] * (width / (0.5 * extent[0] as f32));
    let offset_y = jitter.pixels[1] * (height / (0.5 * extent[1] as f32));
    source_projection[0][2] += offset_x / width;
    source_projection[1][2] += offset_y / height;
    Ok(source_projection)
}
