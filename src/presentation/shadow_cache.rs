// Preserve shadow resources while Bevy removes inactive view entries.
pub(super) fn retain_removed<T>(saved: &mut Option<T>, removed: Option<T>) {
    if let Some(value) = removed {
        *saved = Some(value);
    }
}
// Forward invalidations while the render phase is outside Bevy's active maps.
pub(super) fn invalidate_saved<T>(saved: &mut Option<T>, mut invalidate: impl FnMut(&mut T)) {
    if let Some(cache) = saved {
        invalidate(cache);
    }
}

/// Exact identities used by the source shadow specialization cache.
/// Cache entries must retain both Arc owners to prevent address reuse.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct SpecializationKey {
    mesh_bits: u64,
    layout: usize,
    properties: usize,
}
impl SpecializationKey {
    pub(super) fn new<L, P>(
        mesh_bits: u64,
        layout: &std::sync::Arc<L>,
        properties: &std::sync::Arc<P>,
    ) -> Self {
        Self {
            mesh_bits,
            layout: std::sync::Arc::as_ptr(layout) as usize,
            properties: std::sync::Arc::as_ptr(properties) as usize,
        }
    }
}
