#[path = "../src/presentation/shadow_batch.rs"]
mod shadow_batch;

use shadow_batch::{CasterBatchState, ShadowBatchKey};
use std::collections::BTreeMap;

const OPAQUE: CasterBatchState = CasterBatchState {
    standard: true,
    opaque: true,
    flags: 0,
    lod_fade: 1.,
};

#[test]
fn only_proven_opaque_casters_share_material_keys() {
    // Color, textures, and lighting flags do not affect the opaque caster fragment.
    for flags in 0..256 {
        let state = CasterBatchState { flags, ..OPAQUE };
        assert_eq!(state.material_key(17, 4, Some(4)), None);
        assert_eq!(state.material_key(29, 4, Some(4)), None);
    }
    for state in [
        CasterBatchState {
            standard: false,
            ..OPAQUE
        },
        CasterBatchState {
            opaque: false,
            ..OPAQUE
        },
        CasterBatchState {
            flags: 1 << 8,
            ..OPAQUE
        },
        CasterBatchState {
            flags: 1 << 9,
            ..OPAQUE
        },
        CasterBatchState {
            flags: 1 << 10,
            ..OPAQUE
        },
    ] {
        assert_eq!(state.material_key(17, 4, Some(4)), Some(17));
        assert_eq!(state.material_key(29, 4, Some(4)), Some(29));
    }
    for lod_fade in [-1., -0.5, 0., 0.5, f32::NAN, f32::INFINITY] {
        let state = CasterBatchState { lod_fade, ..OPAQUE };
        assert_eq!(state.material_key(17, 4, Some(4)), Some(17));
    }
}

#[test]
fn changed_or_missing_preparation_keeps_materials_separate() {
    assert_eq!(OPAQUE.material_key(17, 4, None), Some(17));
    assert_eq!(OPAQUE.material_key(17, 5, Some(4)), Some(17));
    assert_eq!(OPAQUE.material_key(17, 5, Some(5)), None);
    let cutout = CasterBatchState {
        flags: 1 << 9,
        ..OPAQUE
    };
    assert_eq!(cutout.material_key(17, 6, Some(5)), Some(17));
    assert_eq!(cutout.material_key(17, 6, Some(6)), Some(17));
    assert_eq!(OPAQUE.material_key(17, 7, Some(6)), Some(17));
    assert_eq!(OPAQUE.material_key(17, 7, Some(7)), None);
}

#[test]
fn specular_casters_require_current_preparation_and_full_lod() {
    let opaque = CasterBatchState::opaque_specular((1 << 0) | (1 << 3), 1.);
    assert_eq!(opaque.material_key(17, 4, Some(4)), None);
    assert_eq!(opaque.material_key(29, 4, Some(4)), None);
    assert_eq!(opaque.material_key(17, 4, None), Some(17));
    assert_eq!(opaque.material_key(17, 5, Some(4)), Some(17));
    assert_eq!(opaque.material_key(17, 5, Some(5)), None);
    for lod_fade in [-1., -0.5, 0., 0.5, f32::NAN, f32::INFINITY] {
        let state = CasterBatchState::opaque_specular(9, lod_fade);
        assert_eq!(state.material_key(17, 4, Some(4)), Some(17));
    }
    for flag in [1 << 8, 1 << 9, 1 << 10] {
        let state = CasterBatchState::opaque_specular(9 | flag, 1.);
        assert_eq!(state.material_key(17, 4, Some(4)), Some(17));
    }
}

#[test]
fn shared_bins_preserve_geometry_draw_state_and_exact_cascade_instances() {
    type Key = ShadowBatchKey<u32, u32, u32>;
    let key = Key {
        pipeline: 1,
        draw: 2,
        material: None,
        split_mask: 0b0101,
        indexed: true,
    };
    // Two opaque materials share geometry. All other draw and geometry changes stay separate.
    let mut fixtures = vec![
        (key.clone(), 10, OPAQUE),
        (key.clone(), 10, OPAQUE),
        (
            Key {
                pipeline: 3,
                ..key.clone()
            },
            10,
            OPAQUE,
        ),
        (
            Key {
                draw: 4,
                ..key.clone()
            },
            10,
            OPAQUE,
        ),
        (
            Key {
                split_mask: 0b1010,
                ..key.clone()
            },
            10,
            OPAQUE,
        ),
        (
            Key {
                indexed: false,
                ..key.clone()
            },
            10,
            OPAQUE,
        ),
        (key.clone(), 11, OPAQUE),
        (
            key.clone(),
            10,
            CasterBatchState {
                flags: 1 << 9,
                ..OPAQUE
            },
        ),
        (
            key.clone(),
            10,
            CasterBatchState {
                lod_fade: 0.5,
                ..OPAQUE
            },
        ),
    ];
    // Specular materials share their own pipeline. Reflected winding uses another pipeline.
    for pipeline in [20, 21] {
        for _ in 0..2 {
            fixtures.push((
                Key {
                    pipeline,
                    ..key.clone()
                },
                10,
                CasterBatchState::opaque_specular(9, 1.),
            ));
        }
    }
    // Every nonzero cascade mask has distinct membership. This includes shadow-only entities.
    for split_mask in 1..16 {
        fixtures.push((
            Key {
                split_mask,
                ..key.clone()
            },
            10,
            OPAQUE,
        ));
    }
    let mut old = BTreeMap::new();
    let mut shared = BTreeMap::new();
    for (entity, (mut key, mesh, state)) in fixtures.into_iter().enumerate() {
        let material = entity as u32 + 100;
        // Preserve a distinct instance transform with each entity.
        let instance = (entity, [entity as i32, 2, -3]);
        key.material = Some(material);
        old.entry((key.clone(), mesh))
            .or_insert_with(Vec::new)
            .push(instance);
        key.material = state.material_key(material, 1, Some(1));
        shared
            .entry((key, mesh))
            .or_insert_with(Vec::new)
            .push(instance);
    }
    assert_eq!(old.len() - shared.len(), 5);
    let merged = &shared[&(key, 10)];
    assert_eq!(merged.len(), 3);
    assert_eq!(merged[0], (0, [0, 2, -3]));
    assert_eq!(merged[1], (1, [1, 2, -3]));
    for cascade in 0..4 {
        let instances = |bins: &BTreeMap<(Key, u32), Vec<(usize, [i32; 3])>>| {
            let mut selected = Vec::new();
            for ((key, mesh), values) in bins {
                if key.split_mask & (1 << cascade) != 0 {
                    selected.extend(values.iter().map(|instance| (*mesh, *instance)));
                }
            }
            selected.sort();
            selected
        };
        assert_eq!(instances(&old), instances(&shared));
    }
}
