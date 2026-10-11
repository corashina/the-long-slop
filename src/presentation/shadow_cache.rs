// Preserve shadow resources while Bevy removes inactive view entries.
pub(super) fn retain_removed<T>(saved: &mut Option<T>, removed: Option<T>) {
    if let Some(value) = removed {
        *saved = Some(value);
    }
}
