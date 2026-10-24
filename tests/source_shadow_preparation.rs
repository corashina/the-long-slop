use the_long_slop::reference::{
    shadow_bounds::{BoundsKey, WorldBoundsCache},
    shadow_candidates::{Bounds, CandidateState},
    shadow_frame::{CasterInput, ShadowFrame, ShadowViewCache, prepare_frame, validate_casters},
    shadow_planes::{CullPlanes, implicit_cull_planes},
    shadow_scene::{CasterData, CasterIndex},
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

#[test]
fn incremental_casters_and_extraction_match_fresh_after_edits_and_rebuilds() {
    let (mut expected, planes, input) = fixture();
    let mut data = CasterData::<u32>::default();
    let mut extracted = CasterData::<u32>::default();
    let mut index = CasterIndex::default();
    data.reserve(4);
    extracted.reserve(4);
    index.reserve(4);
    index.clear(&mut data);
    for (entity, caster) in expected.iter().enumerate() {
        index.update(&mut data, entity as u32, *caster);
    }
    data.publish().unwrap();
    assert!(extracted.extract_from(&data));
    let revision = data.revision();
    let allocation = data.casters.as_ptr();
    for _ in 0..4 {
        for (entity, caster) in expected.iter().enumerate() {
            index.update(&mut data, entity as u32, *caster);
        }
        data.publish().unwrap();
        assert_eq!(data.revision(), revision);
        assert!(!extracted.extract_from(&data));
    }
    let mut view = ShadowViewCache::default();
    let mut masks = Vec::with_capacity(4);
    for edit in [
        |c: &mut CasterInput| c.bounds.center[0] += 2.,
        |c: &mut CasterInput| c.bounds.extents[1] += 0.5,
        |c: &mut CasterInput| c.state.disabled = true,
        |c: &mut CasterInput| c.state.disabled = false,
        |c: &mut CasterInput| c.material_has_caster = false,
        |c: &mut CasterInput| c.material_has_caster = true,
        |c: &mut CasterInput| c.state.layer = 1,
        |c: &mut CasterInput| c.state.shadow_casting_mode = 3,
    ] {
        edit(&mut expected[0]);
        let previous = data.revision();
        index.update(&mut data, 0, expected[0]);
        data.publish().unwrap();
        assert_eq!(data.revision(), previous + 1);
        assert!(extracted.extract_from(&data));
        assert_eq!(extracted.entities, [0, 1]);
        assert!(
            extracted
                .casters
                .iter()
                .zip(&expected)
                .all(|(a, b)| a.unchanged(b))
        );
        compare_fresh(
            &mut view,
            data.revision(),
            &extracted.casters,
            &mut masks,
            planes,
            3,
            3,
            input,
        );
    }
    // Rebuild after removal, insertion, scene replacement, origin shift, or recovery.
    // Entity order can change, even when all caster values remain equal.
    for entities in [
        &[1, 0][..],
        &[1][..],
        &[1, 2][..],
        &[1, 2][..],
        &[][..],
        &[0][..],
    ] {
        let previous = data.revision();
        index.clear(&mut data);
        for &entity in entities {
            index.update(&mut data, entity, expected[(entity as usize).min(1)]);
        }
        data.publish().unwrap();
        assert_eq!(data.revision(), previous + 1);
        assert_eq!(index.contains(0), entities.contains(&0));
        assert!(extracted.extract_from(&data));
        assert_eq!(extracted.entities, entities);
        compare_fresh(
            &mut view,
            data.revision(),
            &extracted.casters,
            &mut masks,
            planes,
            3,
            3,
            input,
        );
    }
    assert_eq!(
        data.casters.as_ptr(),
        allocation,
        "rebuilds reuse reserved storage"
    );
    let revision = data.revision();
    let mut invalid = expected[0];
    invalid.bounds.extents[0] = -1.;
    index.update(&mut data, 0, invalid);
    assert!(data.publish().is_err());
    assert_eq!(
        data.revision(),
        revision,
        "partial collection is not published"
    );
    index.clear(&mut data);
    index.update(&mut data, 0, expected[0]);
    data.publish().unwrap();
    assert!(extracted.extract_from(&data));
    assert!(extracted.casters[0].unchanged(&expected[0]));
}

#[test]
fn view_revisions_retain_masks_until_camera_changes_and_clear_after_failure() {
    let (casters, planes, input) = fixture();
    let mut cache = ShadowViewCache::default();
    let mut masks = vec![0; casters.len()];
    let mut run = |cache: &mut ShadowViewCache, planes, mut input| {
        cache.prepare(1, &casters, &mut masks, planes, 1, 1, &mut input)
    };
    run(&mut cache, planes, input).unwrap();
    let first = cache.revision();
    run(&mut cache, planes, input).unwrap();
    assert_eq!(
        cache.revision(),
        first,
        "unchanged views skip mask extraction"
    );
    let mut camera = input;
    camera.camera_position[0] += 1.;
    run(&mut cache, planes, camera).unwrap();
    assert!(cache.revision() > first);
    cache.clear();
    let inactive = cache.revision();
    cache.clear();
    assert_eq!(
        cache.revision(),
        inactive,
        "inactive views do not recopy empty masks"
    );
    run(&mut cache, planes, input).unwrap();
    let before_failure = cache.revision();
    let mut invalid = planes;
    invalid.shadow[0][0] = f32::NAN;
    assert!(run(&mut cache, invalid, input).is_err());
    cache.clear();
    assert!(
        cache.revision() > before_failure,
        "failed masks cannot retain their last published revision"
    );
    run(&mut cache, planes, input).unwrap();
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
        assert_eq!(
            cache.begin(session, scene, origin),
            matches!(step, 0 | 5 | 6 | 7),
            "only session, scene, and origin changes invalidate the epoch"
        );
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
    }
    assert_eq!(
        calculations, 7,
        "stable inputs retain bounds in steps 1 and 8"
    );
    cache.begin(2, 2, [1000., 0., 0.]);
    cache.clear();
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
