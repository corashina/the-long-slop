//! Per-view source candidate selection, projection fit, and split masks without heap allocation.
use super::{
    shadow_candidates::{
        Bounds, CandidateState, DirectionalHull, ProjectionUnion, no_umbra_candidate,
    },
    shadow_planes::CullPlanes,
    shadow_splits::{SplitCull, visible_split_mask},
    shadows::{ShadowFit, ShadowFitInput, fit_cascades, frustum_scale},
};

/// Each material primitive carries its complete renderer AABB.
/// Repeated bounds from multiple valid material parts do not change the geometric union.
#[derive(Clone, Copy)]
pub struct CasterInput {
    pub bounds: Bounds,
    pub state: CandidateState,
    pub material_has_caster: bool,
}
pub struct ShadowFrame {
    pub fit: ShadowFit,
    pub candidate_count: usize,
    pub split_counts: [usize; 4],
}

/// The caller supplies stable entity order and preallocated output storage of the same length.
/// fit_input contains this live view's matrices and source settings. This function fills its caster bounds.
/// A valid empty candidate set returns None and leaves every output mask zero.
pub fn prepare_frame(
    casters: &[CasterInput],
    masks: &mut [u8],
    planes: &CullPlanes,
    light_mask: u32,
    view_mask: u32,
    fit_input: &mut ShadowFitInput,
) -> Result<Option<ShadowFrame>, &'static str> {
    if casters.len() != masks.len() {
        return Err("Source shadow candidate and mask counts differ");
    }
    masks.fill(0);
    if planes
        .camera
        .iter()
        .chain(planes.shadow.iter())
        .flatten()
        .any(|value| !value.is_finite())
        || casters.iter().any(|caster| {
            caster
                .bounds
                .center
                .iter()
                .chain(caster.bounds.extents.iter())
                .any(|value| !value.is_finite())
                || caster.bounds.extents.iter().any(|value| *value < 0.)
        })
    {
        return Err("Source shadow scene has malformed bounds or planes");
    }
    // Source far camera near300 exceeds shadowDistance100. The native scale test
    // at 0x2b42c4..0x2b42cb rejects its cascades. The collector stays fully lit.
    if frustum_scale(fit_input)? <= 9.999999747378752e-6 {
        return Ok(None);
    }
    let hull = DirectionalHull::new(planes.shadow, fit_input.light_basis[2])?;
    let mut union = ProjectionUnion::default();
    let mut candidate_count = 0;
    for (caster, mask) in casters.iter().zip(masks.iter_mut()) {
        if !caster.material_has_caster
            || !no_umbra_candidate(&caster.state, caster.bounds, &hull, light_mask, view_mask)
        {
            continue;
        }
        *mask = 1;
        candidate_count += 1;
        union.include(caster.bounds, &planes.camera);
    }
    let Some(bounds) = union.bounds() else {
        return Ok(None);
    };
    fit_input.caster_min = bounds.min;
    fit_input.caster_max = bounds.max;
    let fit = fit_cascades(fit_input)?;
    let make_split = |index: usize| {
        let cascade = &fit.cascades[index];
        SplitCull::new(
            index,
            [
                cascade.sphere_center[0],
                cascade.sphere_center[1],
                cascade.sphere_center[2],
                cascade.sphere_radius,
            ],
            fit_input.light_basis,
            planes.shadow,
            fit_input.near,
            fit_input.shadow_distance.min(fit_input.far),
            cascade.min_view_distance,
            cascade.max_view_distance,
        )
    };
    let splits = [
        make_split(0)?,
        make_split(1)?,
        make_split(2)?,
        make_split(3)?,
    ];
    let mut split_counts = [0; 4];
    for (caster, mask) in casters.iter().zip(masks) {
        if *mask == 0 {
            continue;
        }
        *mask = visible_split_mask(
            caster.bounds,
            fit_input.light_basis,
            &planes.shadow,
            &splits,
        );
        for (index, count) in split_counts.iter_mut().enumerate() {
            *count += usize::from(*mask & (1 << index) != 0);
        }
    }
    Ok(Some(ShadowFrame {
        fit,
        candidate_count,
        split_counts,
    }))
}

#[cfg(test)]
mod tests {
    use super::super::shadow_planes::implicit_cull_planes;
    use super::*;

    #[test]
    fn offscreen_draw_survives_preferred_fit_and_empty_frame_clears_masks() {
        let projection = [
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
        ];
        let identity = [
            1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
        ];
        let planes = implicit_cull_planes(projection, identity, 1., 10., 10., 0.);
        let make = |center| CasterInput {
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
        let mut casters = [make([0., 0., -5.]), make([20., 0., -5.])];
        let mut masks = [255; 2];
        let mut input = ShadowFitInput {
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
        let frame = prepare_frame(&casters, &mut masks, &planes, 1, 1, &mut input)
            .unwrap()
            .unwrap();
        assert_eq!(frame.candidate_count, 2);
        assert_eq!(input.caster_min, [-0.5, -0.5, -5.5]);
        assert_eq!(input.caster_max, [0.5, 0.5, -4.5]);
        assert_ne!(masks[1], 0, "offscreen caster must retain a draw split");
        for caster in &mut casters {
            caster.material_has_caster = false;
        }
        assert!(
            prepare_frame(&casters, &mut masks, &planes, 1, 1, &mut input)
                .unwrap()
                .is_none()
        );
        assert_eq!(masks, [0, 0]);
        // Valid source-rejected range differs from malformed input.
        for caster in &mut casters {
            caster.material_has_caster = true;
        }
        input.near = 300.;
        input.far = 20000.;
        input.shadow_distance = 100.;
        masks.fill(255);
        assert!(
            prepare_frame(&casters, &mut masks, &planes, 1, 1, &mut input)
                .unwrap()
                .is_none()
        );
        assert_eq!(masks, [0, 0]);
        input.frustum_points[6][0] = f32::NAN;
        assert!(prepare_frame(&casters, &mut masks, &planes, 1, 1, &mut input).is_err());
    }
}
