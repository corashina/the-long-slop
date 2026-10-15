#[path = "../src/presentation/shadow_cache.rs"]
mod shadow_cache;

use shadow_cache::retain_removed;

#[test]
fn inactive_shadow_caches_remove_changed_materials_without_losing_storage() {
    let mut phase = Some(vec![11_u64, 12, 13]);
    let mut pipelines = Some(vec![(11_u64, 31_u64), (12, 32), (13, 33)]);
    let phase_storage = phase.as_ref().unwrap().as_ptr();
    let pipeline_storage = pipelines.as_ref().unwrap().as_ptr();
    shadow_cache::invalidate_saved(&mut phase, |items| items.retain(|item| *item != 11));
    shadow_cache::invalidate_saved(&mut pipelines, |items| items.retain(|item| item.0 != 11));
    // A second off frame must neither restore the old cloud entry nor lose the cache.
    retain_removed(&mut phase, None);
    retain_removed(&mut pipelines, None);
    assert_eq!(phase.as_deref(), Some([12, 13].as_slice()));
    assert_eq!(pipelines.as_deref(), Some([(12, 32), (13, 33)].as_slice()));
    assert_eq!(phase.as_ref().unwrap().as_ptr(), phase_storage);
    assert_eq!(pipelines.as_ref().unwrap().as_ptr(), pipeline_storage);
    let mut absent: Option<Vec<u64>> = None;
    shadow_cache::invalidate_saved(&mut absent, |_| panic!("No dormant cache"));
    assert!(absent.is_none());
}
