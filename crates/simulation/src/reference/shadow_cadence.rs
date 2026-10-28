//! World view cascade cadence. Far cascades render less often and keep their last fit.
use super::shadows::ShadowCascade;

/// A change of these values renders all cascades.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CascadeEpoch {
    pub session: u64,
    pub scene: u32,
    pub origin: [u64; 3],
    /// Depth bias, normal bias, strength and split fractions, as bits.
    pub settings: [u32; 6],
}
pub struct CascadeInputs {
    pub epoch: CascadeEpoch,
    pub cascades: [ShadowCascade; 4],
    /// The atlas is new, or its last planned frame was not encoded.
    pub forced: bool,
}
/// The fit of each cascade at its last render. The atlas tile holds this fit.
#[derive(Clone, Copy, Debug, Default)]
pub struct CascadeHistory {
    pub epoch: Option<CascadeEpoch>,
    pub cascades: [ShadowCascade; 4],
}
impl CascadeHistory {
    /// Keep the current fit for each rendered cascade. Returns true when a value changed.
    pub fn commit(&mut self, render: [bool; 4], current: &CascadeInputs) -> bool {
        let mut changed = self.epoch != Some(current.epoch);
        self.epoch = Some(current.epoch);
        for (index, cascade) in current.cascades.iter().enumerate() {
            if render[index] {
                changed |= !same(&self.cascades[index], cascade);
                self.cascades[index] = *cascade;
            }
        }
        changed
    }
}
fn same(a: &ShadowCascade, b: &ShadowCascade) -> bool {
    let words = |c: &ShadowCascade| {
        c.sphere_center
            .into_iter()
            .chain([c.sphere_radius])
            .chain(c.snapped_center)
            .chain(c.light_position)
            .chain(c.width_height)
            .chain(c.texel_size)
            .chain([c.near, c.far, c.min_view_distance, c.max_view_distance])
            .map(f32::to_bits)
    };
    words(a).eq(words(b))
}
/// Cascades 0 and 1 render each frame, cascade 2 on even frames, cascade 3 when frame % 4 == 1.
/// All render after an epoch change, a forced refresh, or a cascade 2 or 3 sphere move above 25% of its radius.
pub fn cascades_to_render(frame: u32, last: &CascadeHistory, current: &CascadeInputs) -> [bool; 4] {
    // Cascades 0 and 1 render each frame, so only the skipped cascades can hold a stale fit.
    let moved = last.cascades[2..]
        .iter()
        .zip(&current.cascades[2..])
        .any(|(old, new)| {
            let distance = (0..3)
                .map(|i| (new.sphere_center[i] - old.sphere_center[i]).powi(2))
                .sum::<f32>();
            distance > (0.25 * old.sphere_radius).powi(2)
        });
    if current.forced || last.epoch != Some(current.epoch) || moved {
        return [true; 4];
    }
    [true, true, frame.is_multiple_of(2), frame % 4 == 1]
}
