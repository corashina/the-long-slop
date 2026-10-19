#[path = "../src/presentation/shadow_preparation.rs"]
mod preparation;
use preparation::{CascadeKey, CollectorKey, ViewRevision, same_floats};

#[test]
fn shadow_upload_inputs_invalidate_for_epoch_fit_bias_projection_and_target_changes() {
    let view = ViewRevision {
        session: 1,
        scene: 1,
        fit: 1,
    };
    let cascade = CascadeKey { view, bias: [0; 2] };
    let collector = CollectorKey {
        view,
        projection: [0; 16],
        view_from_world: [0; 16],
        near_far_strength: [0; 3],
        atlas_size: 256,
        primary: true,
    };
    assert_eq!(cascade, cascade);
    assert_eq!(collector, collector);
    for changed in [
        ViewRevision { session: 2, ..view },
        ViewRevision { scene: 2, ..view },
        ViewRevision { fit: 2, ..view },
    ] {
        assert_ne!(
            CascadeKey {
                view: changed,
                ..cascade
            },
            cascade
        );
        assert_ne!(
            CollectorKey {
                view: changed,
                ..collector
            },
            collector
        );
    }
    for index in 0..2 {
        let mut changed = cascade;
        changed.bias[index] = (-0_f32).to_bits();
        assert_ne!(changed, cascade);
    }
    for index in 0..16 {
        let mut changed = collector;
        changed.projection[index] = 1;
        assert_ne!(changed, collector);
        let mut changed = collector;
        changed.view_from_world[index] = 1;
        assert_ne!(changed, collector);
    }
    for index in 0..3 {
        let mut changed = collector;
        changed.near_far_strength[index] = 1;
        assert_ne!(changed, collector);
    }
    assert_ne!(
        CollectorKey {
            atlas_size: 512,
            ..collector
        },
        collector
    );
    assert_ne!(
        CollectorKey {
            primary: false,
            ..collector
        },
        collector
    );
}

#[test]
fn upload_comparison_covers_every_shader_word_including_signed_zero() {
    for count in [72, 124] {
        let original = vec![0_f32; count];
        assert!(same_floats(
            original.iter().copied(),
            original.iter().copied()
        ));
        for index in 0..count {
            let mut changed = original.clone();
            changed[index] = -0.;
            assert!(!same_floats(
                original.iter().copied(),
                changed.iter().copied()
            ));
            changed[index] = 1.;
            assert!(!same_floats(
                original.iter().copied(),
                changed.iter().copied()
            ));
        }
    }
    assert!(!same_floats([0.].into_iter(), [0., 0.].into_iter()));
}
