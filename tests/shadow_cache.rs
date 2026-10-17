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

#[test]
fn specialization_keys_match_linear_lookup_without_cloning_owners() {
    use shadow_cache::SpecializationKey;
    use std::sync::Arc;
    let layouts: Vec<_> = (0..5).map(|_| Arc::new(0_u32)).collect();
    let materials: Vec<_> = (0..7).map(|_| Arc::new(0_u32)).collect();
    let mut linear = Vec::new();
    let mut keyed = Vec::new();
    for bits in [0, 1, u64::MAX] {
        for layout in &layouts {
            for properties in &materials {
                let id = linear.len();
                linear.push((bits, layout.clone(), properties.clone(), id));
                let key = SpecializationKey::new(bits, layout, properties);
                let index = keyed
                    .binary_search_by_key(&key, |entry: &(SpecializationKey, usize)| entry.0)
                    .unwrap_err();
                keyed.insert(index, (key, id));
            }
        }
    }
    let absent_layout = Arc::new(0_u32);
    let absent_material = Arc::new(0_u32);
    for bits in [0, 1, 2, u64::MAX] {
        for layout in layouts.iter().chain([&absent_layout]) {
            for properties in materials.iter().chain([&absent_material]) {
                let counts = (Arc::strong_count(layout), Arc::strong_count(properties));
                let key = SpecializationKey::new(bits, layout, properties);
                let expected = linear
                    .iter()
                    .find(|e| {
                        e.0 == bits && Arc::ptr_eq(&e.1, layout) && Arc::ptr_eq(&e.2, properties)
                    })
                    .map(|e| &e.3);
                let actual = keyed
                    .binary_search_by_key(&key, |entry| entry.0)
                    .ok()
                    .map(|index| &keyed[index].1);
                assert_eq!(actual, expected);
                assert_eq!(
                    counts,
                    (Arc::strong_count(layout), Arc::strong_count(properties))
                );
            }
        }
    }
}
