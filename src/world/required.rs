//! Required identities refer only to generated content and current source views.
use super::{GeneratedId, GenerationCoverage, SurfaceKey, WorldBounds, WorldRequest};
use crate::reference::shadow_candidates::{Bounds, DirectionalHull, intersects_camera};

#[derive(Clone, Copy)]
pub struct RequiredView {
    pub camera: [[f32; 4]; 6],
    pub shadow: DirectionalHull,
    pub mask: u32,
    pub light_mask: u32,
    pub origin: [f64; 3],
}
impl RequiredView {
    pub fn includes(&self, bounds: WorldBounds, layers: u32) -> bool {
        let center: [f64; 3] =
            std::array::from_fn(|i| (bounds.min[i] + bounds.max[i]) * 0.5 - self.origin[i]);
        let bounds = Bounds {
            center: [center[0] as f32, center[1] as f32, -center[2] as f32],
            extents: std::array::from_fn(|i| ((bounds.max[i] - bounds.min[i]) * 0.5) as f32),
        };
        self.mask & layers != 0
            && (intersects_camera(bounds, &self.camera)
                || self.light_mask & layers != 0 && self.shadow.intersects(bounds))
    }
}
#[derive(Default)]
pub(crate) struct RequiredSelection {
    pub views_ready: bool,
    pub revision: u64,
    pub roads: Vec<u32>,
    pub terrain: Vec<SurfaceKey>,
    pub structures: Vec<GeneratedId>,
    pub lights: Vec<GeneratedId>,
}
pub struct RequiredContent<'a> {
    pub request: WorldRequest,
    pub coverage: GenerationCoverage,
    pub selection_revision: u64,
    pub roads: &'a [u32],
    pub terrain: &'a [SurfaceKey],
    pub structures: &'a [GeneratedId],
    pub lights: &'a [GeneratedId],
    pub collision_ranges: [WorldBounds; 2],
}

pub(crate) fn select<'a, T: Clone + PartialEq + 'a>(
    output: &mut Vec<T>,
    values: impl Iterator<Item = &'a T>,
) -> bool {
    let mut count = 0;
    let mut changed = false;
    for value in values {
        if count == output.len() {
            output.push(value.clone());
            changed = true;
        } else if output[count] != *value {
            output[count] = value.clone();
            changed = true;
        }
        count += 1;
    }
    changed |= output.len() != count;
    output.truncate(count);
    changed
}
