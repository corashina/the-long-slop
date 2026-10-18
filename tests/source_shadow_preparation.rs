use my_summer_slop::reference::{
    shadow_bounds::{BoundsKey, WorldBoundsCache},
    shadow_candidates::{Bounds, CandidateState},
    shadow_frame::{CasterInput, ShadowFrame, ShadowViewCache, prepare_frame, validate_casters},
    shadow_planes::{CullPlanes, implicit_cull_planes},
    shadows::ShadowFitInput,
};

fn fixture() -> (Vec<CasterInput>, CullPlanes, ShadowFitInput) {
    let planes = implicit_cull_planes(
        [
            1.,
            0.,
            0.,
            0.,
            0.,
            1.,
            0.,
            0.,
            0.,
            0.,
            -11. / 9.,
            -1.,
            0.,
            0.,
            -20. / 9.,
            0.,
        ],
        [
            1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
        ],
        1.,
        10.,
        10.,
        0.,
    );
    let caster = |center| CasterInput {
        bounds: Bounds {
            center,
            extents: [0.5; 3],
        },
        state: CandidateState {
            shadow_casting_mode: 1,
            layer: 0,
            renderer_present: true,
            disabled: false,
            force_off: false,
            lod_group: 0,
            lod_index_mask: 0,
            active_lod: 0,
        },
        material_has_caster: true,
    };
    let input = ShadowFitInput {
        frustum_points: [
            [-1., -1., -1.],
            [1., -1., -1.],
            [1., 1., -1.],
            [-1., 1., -1.],
            [-10., -10., -10.],
            [10., -10., -10.],
            [10., 10., -10.],
            [-10., 10., -10.],
        ],
        camera_basis: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        camera_position: [0.; 3],
        light_basis: [[0., 0., 1.], [0., 1., 0.], [-1., 0., 0.]],
        caster_min: [0.; 3],
        caster_max: [0.; 3],
        near: 1.,
        far: 10.,
        vertical_fov: 90.,
        shadow_distance: 10.,
        split_fractions: [0.1, 0.3, 0.6],
        tile_size: [64; 2],
        quality_near_offset: 0.,
        shadow_strength: 0.9,
    };
    (
        vec![caster([0., 0., -5.]), caster([20., 0., -5.])],
        planes,
        input,
    )
}

fn frame_words(frame: Option<ShadowFrame>) -> Option<(usize, [usize; 4], Vec<u32>)> {
    frame.map(|frame| {
        let mut words = Vec::new();
        for c in frame.fit.cascades {
            words.extend(
                c.sphere_center
                    .into_iter()
                    .chain([c.sphere_radius])
                    .chain(c.snapped_center)
                    .chain(c.light_position)
                    .chain(c.width_height)
                    .chain(c.texel_size)
                    .chain([c.near, c.far, c.min_view_distance, c.max_view_distance])
                    .map(f32::to_bits),
            );
        }
        words.extend(
            frame
                .fit
                .fade
                .into_iter()
                .chain(frame.fit.fade_center)
                .map(f32::to_bits),
        );
        (frame.candidate_count, frame.split_counts, words)
    })
}

fn compare_fresh(
    cache: &mut ShadowViewCache,
    revision: u64,
    casters: &[CasterInput],
    masks: &mut Vec<u8>,
    planes: CullPlanes,
    light_mask: u32,
    view_mask: u32,
    input: ShadowFitInput,
) {
    validate_casters(casters).unwrap();
    masks.resize(casters.len(), 0);
    let mut fresh_masks = vec![255; casters.len()];
    let mut fresh_input = input;
    let fresh = prepare_frame(
        casters,
        &mut fresh_masks,
        &planes,
        light_mask,
        view_mask,
        &mut fresh_input,
    );
    let mut cached_input = input;
    let cached = cache.prepare(
        revision,
        casters,
        masks,
        planes,
        light_mask,
        view_mask,
        &mut cached_input,
    );
    assert_eq!(cached.map(frame_words), fresh.map(frame_words));
    assert_eq!(*masks, fresh_masks);
    assert_eq!(
        cached_input.caster_min.map(f32::to_bits),
        fresh_input.caster_min.map(f32::to_bits)
    );
    assert_eq!(
        cached_input.caster_max.map(f32::to_bits),
        fresh_input.caster_max.map(f32::to_bits)
    );
}

#[test]
fn cached_masks_and_fits_match_fresh_for_caster_and_view_changes() {
    let (mut casters, mut planes, mut input) = fixture();
    let mut cache = ShadowViewCache::default();
    let mut masks = Vec::new();
    let mut revision = 1;
    compare_fresh(
        &mut cache, revision, &casters, &mut masks, planes, 1, 1, input,
    );
    assert_ne!(masks[1], 0, "offscreen caster remains in a draw split");
    compare_fresh(
        &mut cache, revision, &casters, &mut masks, planes, 1, 1, input,
    );
    let changes: [fn(&mut CasterInput); 9] = [
        |c| c.bounds.center[0] += 1.,
        |c| c.bounds.extents[1] += 0.25,
        |c| c.material_has_caster = false,
        |c| c.material_has_caster = true,
        |c| c.state.layer = 1,
        |c| c.state.layer = 0,
        |c| c.state.disabled = true,
        |c| c.state.disabled = false,
        |c| c.state.shadow_casting_mode = 3,
    ];
    for change in changes {
        let before = casters[0];
        change(&mut casters[0]);
        assert!(!before.unchanged(&casters[0]));
        revision += 1;
        compare_fresh(
            &mut cache, revision, &casters, &mut masks, planes, 1, 1, input,
        );
    }
    casters.swap(0, 1);
    revision += 1;
    compare_fresh(
        &mut cache, revision, &casters, &mut masks, planes, 1, 1, input,
    );
    casters.pop();
    revision += 1;
    compare_fresh(
        &mut cache, revision, &casters, &mut masks, planes, 1, 1, input,
    );
    input.camera_position[0] += 1.;
    input.frustum_points[0][0] -= 0.1;
    compare_fresh(
        &mut cache, revision, &casters, &mut masks, planes, 1, 1, input,
    );
    input.light_basis = [[1., 0., 0.], [0., 0., 1.], [0., -1., 0.]];
    input.tile_size = [128; 2];
    input.shadow_strength = 0.5;
    compare_fresh(
        &mut cache, revision, &casters, &mut masks, planes, 1, 1, input,
    );
    planes.camera[0][3] += 0.25;
    planes.shadow[0][3] += 0.25;
    compare_fresh(
        &mut cache, revision, &casters, &mut masks, planes, 1, 1, input,
    );
    compare_fresh(
        &mut cache, revision, &casters, &mut masks, planes, 0, 1, input,
    );
    compare_fresh(
        &mut cache, revision, &casters, &mut masks, planes, 1, 0, input,
    );
    // An absent view clears its masks and its cache before it returns.
    masks.clear();
    cache.clear();
    compare_fresh(
        &mut cache, revision, &casters, &mut masks, planes, 1, 1, input,
    );
    planes.shadow[0][0] = f32::NAN;
    compare_fresh(
        &mut cache, revision, &casters, &mut masks, planes, 1, 1, input,
    );
    casters.clear();
    revision += 1;
    let (_, planes, mut input) = fixture();
    compare_fresh(
        &mut cache, revision, &casters, &mut masks, planes, 1, 1, input,
    );
    input.near = 300.;
    input.far = 20000.;
    input.shadow_distance = 100.;
    compare_fresh(
        &mut cache, revision, &casters, &mut masks, planes, 1, 1, input,
    );
}

#[test]
fn retained_bounds_invalidate_for_transform_local_bounds_scene_origin_and_removal() {
    let mut cache = WorldBoundsCache::<u32>::default();
    cache.reserve(4);
    let mut transform = [1., 0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0.];
    let mut center = [0., 0., -5.];
    let mut extents = [0.5; 3];
    let mut calculations = 0;
    for step in 0..9 {
        let session = if step >= 7 { 2 } else { 1 };
        let scene = if step >= 6 { 2 } else { 1 };
        let origin = if step >= 5 { [1000., 0., 0.] } else { [0.; 3] };
        match step {
            2 => transform[9] = 2.,
            3 => center[0] = 1.,
            4 => extents[0] = 1.,
            _ => {}
        }
        cache.begin(session, scene, origin);
        let expected = Bounds {
            center: [center[0] + transform[9], center[1], center[2]],
            extents,
        };
        let actual = cache
            .get_or_update(1, BoundsKey::new(transform, center, extents), || {
                calculations += 1;
                expected
            })
            .unwrap();
        assert_eq!(actual, expected);
        cache.finish();
    }
    assert_eq!(
        calculations, 7,
        "stable inputs retain bounds in steps 1 and 8"
    );
    cache.begin(2, 2, [1000., 0., 0.]);
    cache.finish();
    cache.begin(2, 2, [1000., 0., 0.]);
    cache
        .get_or_update(1, BoundsKey::new(transform, center, extents), || {
            calculations += 1;
            Bounds { center, extents }
        })
        .unwrap();
    assert_eq!(
        calculations, 8,
        "removed entity must be recomputed if it returns"
    );
    let malformed = Bounds {
        center: [f32::NAN, 0., 0.],
        extents,
    };
    assert!(
        cache
            .get_or_update(2, BoundsKey::new(transform, center, extents), || malformed)
            .is_err()
    );
    let (mut casters, _, _) = fixture();
    casters[0].bounds.extents[0] = -1.;
    assert!(validate_casters(&casters).is_err());
}
