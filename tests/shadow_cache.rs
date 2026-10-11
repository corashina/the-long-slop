#[path = "../src/presentation/shadow_cache.rs"]
mod shadow_cache;

use shadow_cache::retain_removed;

#[test]
fn inactive_shadow_caches_keep_identity_across_off_off_on() {
    let mut active_phase = Some(vec![11_u64, 12]);
    let mut active_key = Some(23_u32);
    let mut active_pipeline = Some(vec![31_u64, 32, 33]);
    let phase_storage = active_phase.as_ref().unwrap().as_ptr();
    let pipeline_storage = active_pipeline.as_ref().unwrap().as_ptr();
    let mut saved_phase = None;
    let mut saved_key = None;
    let mut saved_pipeline = None;

    // Each frame reserves before activation. Two off frames leave the maps empty.
    for _ in 0..3 {
        retain_removed(&mut saved_phase, active_phase.take());
        retain_removed(&mut saved_key, active_key.take());
        retain_removed(&mut saved_pipeline, active_pipeline.take());
        assert_eq!(saved_phase.as_ref().unwrap().as_ptr(), phase_storage);
        assert_eq!(saved_key, Some(23));
        assert_eq!(saved_pipeline.as_ref().unwrap().as_ptr(), pipeline_storage);
    }

    // The on frame restores the same resources, including their backing storage.
    active_phase = saved_phase.take();
    active_key = saved_key.take();
    active_pipeline = saved_pipeline.take();
    assert_eq!(active_phase.as_ref().unwrap().as_ptr(), phase_storage);
    assert_eq!(active_phase.as_deref(), Some([11, 12].as_slice()));
    assert_eq!(active_key, Some(23));
    assert_eq!(active_pipeline.as_ref().unwrap().as_ptr(), pipeline_storage);
    assert_eq!(active_pipeline.as_deref(), Some([31, 32, 33].as_slice()));
}
