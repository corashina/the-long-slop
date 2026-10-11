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
