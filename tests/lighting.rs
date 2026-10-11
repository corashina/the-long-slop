use my_summer_slop::lighting::{CyclePhase, CycleState};
use my_summer_slop::reference::{CycleReference, ReferenceManifest};

fn reference() -> CycleReference {
    ReferenceManifest::from_json(include_bytes!("common/reference.json"))
        .unwrap()
        .lighting
        .cycle
}
fn sample(seconds: f64) -> CycleState {
    let mut reference = reference();
    reference.startup_offset_seconds = 0.;
    let mut state = CycleState::new(&reference);
    state.advance(&reference, seconds);
    state
}
fn close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() <= 1e-6, "{actual} != {expected}");
}

#[test]
fn cycle_boundaries_match_source() {
    let phase = |t| sample(t).output().phase;
    assert_eq!(phase(240.0), CyclePhase::Dawn);
    assert_eq!(phase(240.001), CyclePhase::Day);
    assert_eq!(phase(1020.0), CyclePhase::Day);
    assert_eq!(phase(1020.001), CyclePhase::Dusk);
    assert_eq!(phase(1200.0), CyclePhase::Dusk);
    assert_eq!(phase(1200.001), CyclePhase::Night);
}
#[test]
fn source_cycle_formulas_match() {
    let dawn = sample(120.);
    close(dawn.output().directional_intensity, 0.5625);
    close(dawn.output().ambient, 0.11);
    close(dawn.output().sky_exposure, 1.);
    close(sample(630.).output().directional_intensity, 1.6);
    close(sample(1350.).output().directional_intensity, 0.3);
    close(sample(1350.).output().sky_exposure, 2.);
    close(sample(1350.).output().reflection_intensity, 0.);
    close(sample(1200.).output().sky_exposure, 0.);
    close(sample(1200.001).output().sky_exposure, 2.);
    close(dawn.output().directional_euler_degrees[0], 5.);
    close(dawn.output().directional_euler_degrees[1], 30.);
    close(sample(630.).output().directional_euler_degrees[0], 90.);
    close(sample(1350.).output().directional_euler_degrees[0], 100.);
}
#[test]
fn roadside_thresholds_match_source() {
    assert!(sample(119.999).output().roadside_enabled);
    assert!(!sample(120.).output().roadside_enabled);
    assert!(!sample(1110.).output().roadside_enabled);
    assert!(sample(1110.001).output().roadside_enabled);
    assert!(!sample(630.).output().roadside_enabled);
    assert!(sample(1350.).output().roadside_enabled);
}
#[test]
fn cycle_wrap_retains_source_update_order() {
    let mut reference = reference();
    reference.startup_offset_seconds = 0.;
    let mut state = CycleState::new(&reference);
    state.advance(&reference, 1500.);
    assert_eq!(state.output().phase, CyclePhase::Night);
    assert_eq!(state.elapsed_seconds(), 1500.);
    assert_eq!(state.output().cycle_seconds, 1500.);
    state.advance(&reference, 0.02);
    assert_eq!(state.output().phase, CyclePhase::Night);
    close(state.output().progress, 1.);
    assert_eq!(state.elapsed_seconds(), 1500.02);
    assert_eq!(state.output().cycle_seconds, 1500.02);
    let stars = state.output().star_rotation_degrees;
    let shader_time = state.output().shader_time_seconds;
    state.advance(&reference, 0.02);
    assert_eq!(state.output().phase, CyclePhase::Dawn);
    assert_eq!(state.elapsed_seconds(), 1500.04);
    assert_eq!(state.output().cycle_seconds, 0.02);
    assert!(state.output().star_rotation_degrees > stars);
    assert!(state.output().shader_time_seconds > shader_time);
}
#[test]
fn invalid_elapsed_does_not_corrupt_state() {
    let reference = reference();
    let mut state = CycleState::new(&reference);
    state.advance(&reference, 3.);
    let previous = state.output().clone();
    let clock = state.elapsed_seconds();
    for invalid in [-1., f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        state.advance(&reference, invalid);
        assert_eq!(*state.output(), previous);
        assert_eq!(state.elapsed_seconds(), clock);
    }
}
#[test]
fn startup_reset_and_biome_refresh_match_source() {
    let reference = reference();
    let mut state = CycleState::new(&reference);
    assert_eq!(state.elapsed_seconds(), 0.);
    assert_eq!(state.output().cycle_seconds, 240.);
    assert_eq!(state.output().phase, CyclePhase::Dawn);
    assert_eq!(state.output().biome_color, [0.; 4]);
    state.advance(&reference, 0.99);
    assert_eq!(state.output().biome_color, [0.; 4]);
    state.advance(&reference, 0.01);
    assert_eq!(state.output().biome_color, reference.desert_color);
    state.advance(&reference, 1500.);
    state.reset(&reference);
    assert_eq!(state.elapsed_seconds(), 0.);
    assert_eq!(state.output().cycle_seconds, 240.);
    assert_eq!(state.output().phase, CyclePhase::Dawn);
    assert_eq!(state.output().shader_time_seconds, 0.);
    assert_eq!(state.output().star_rotation_degrees, 0.);
    assert_eq!(state.output().biome_color, [0.; 4]);
}
#[test]
fn fog_clouds_stars_and_flare_keep_source_branches() {
    let r = reference();
    let dawn = sample(120.);
    close(
        dawn.output().fog_density,
        r.day_fog_densities[0] * r.fog_modifier,
    );
    assert_eq!(dawn.output().fog_color, r.night_fog_colors[0]);
    close(dawn.output().flare_brightness, r.flare_dawn[0]);
    close(dawn.output().star_color[3], 0.5);
    close(sample(630.).output().visual_temperature, 47.);
    close(sample(1020.).output().visual_temperature, 49.457832);
    close(
        sample(1110.).output().fog_density,
        r.day_fog_densities[0] * r.fog_modifier,
    );
    assert_eq!(sample(1110.).output().fog_color, r.night_fog_colors[0]);
    close(sample(1110.).output().flare_brightness, r.flare_dusk[0]);
    assert_eq!(sample(1350.).output().star_color, r.star_color_night);
    for (output, cloud) in dawn.output().clouds.iter().zip(&r.clouds) {
        assert_eq!(output.color, cloud.transition_color);
        close(output.speed, cloud.mid_speed);
    }
}
#[test]
fn gradients_use_exported_keys_and_independent_alpha() {
    let mut r = reference();
    r.startup_offset_seconds = 120.;
    r.color.colors[0].time = 0.25;
    r.color.colors[1].time = 0.75;
    r.color.colors[0].value = [0., 0., 0., 0.];
    r.color.colors[1].value = [1., 1., 1., 0.];
    r.color.alphas[0].value = 0.2;
    r.color.alphas[1].value = 0.8;
    let output = CycleState::new(&r);
    for v in output.output().directional_color {
        close(v, 0.5);
    }
    r.color.mode = 1;
    let output = CycleState::new(&r);
    assert_eq!(output.output().directional_color, [0., 0., 0., 0.2]);
}

#[test]
fn cloud_noise_motion_and_density_match_source() {
    let mut r = reference();
    r.startup_offset_seconds = 0.;
    r.cloud_perlin_speed = 1.;
    let mut state = CycleState::new(&r);
    state.advance(&r, 0.25);
    assert_eq!(state.output().cloud_noise_coordinate, -0.75);
    let native_noise = f32::from_bits(0x3ebba58b);
    let perturbation = (native_noise - 0.5) * 2.;
    assert_eq!(state.output().cloud_noise.to_bits(), perturbation.to_bits());
    state.advance(&r, 0.75);
    assert_eq!(state.output().cloud_noise_coordinate, 0.);
    assert_eq!(state.output().shader_time_seconds, 1.);
    close(state.output().star_rotation_degrees, 0.1);
    r.cloud_perlin_speed = 0.;
    r.cloud_perlin_spread = 0.;
    let mut state = CycleState::new(&r);
    state.advance(&r, 1350.);
    for (cloud, reference) in state.output().clouds.iter().zip(&r.clouds) {
        close(cloud.density, reference.max_density);
        assert_eq!(cloud.color, reference.night_color);
    }
    let mut state = CycleState::new(&r);
    state.advance(&r, 669.); // Day progress 0.55 reaches the full noon temperature.
    close(state.output().visual_temperature, 50.);
    for (cloud, reference) in state.output().clouds.iter().zip(&r.clouds) {
        close(cloud.density, reference.min_density);
    }
}

#[test]
fn source_draw_distance_fog_modifier_applies_to_each_phase() {
    let modifier = 3.2959185_f32;
    let r = reference();
    assert_eq!(r.fog_modifier.to_bits(), modifier.to_bits());
    for seconds in [120., 630., 1110., 1350.] {
        let base = if seconds == 1350. {
            r.night_fog_densities[0]
        } else {
            r.day_fog_densities[0]
        };
        close(sample(seconds).output().fog_density, base * modifier);
    }
}

#[test]
fn cycle_reference_rejects_missing_or_invalid_fog_modifier() {
    let original = ReferenceManifest::from_json(include_bytes!("common/reference.json")).unwrap();
    let mut json: serde_json::Value =
        serde_json::from_slice(include_bytes!("common/reference.json")).unwrap();
    json["lighting"]["cycle"]
        .as_object_mut()
        .unwrap()
        .remove("fog_modifier");
    assert!(ReferenceManifest::from_json(&serde_json::to_vec(&json).unwrap()).is_err());
    for invalid in [-1., f32::NAN, f32::INFINITY] {
        let mut manifest = original.clone();
        manifest.lighting.cycle.fog_modifier = invalid;
        assert!(manifest.validate().is_err());
    }
}

#[test]
fn source_upload_preserves_native_light_conversion_and_attenuation() {
    use my_summer_slop::lighting::{attenuation_table, source_linear};
    // Checksum of all 1024 byte texels from the checked native callback.
    let checksum = attenuation_table()
        .iter()
        .fold(0xcbf29ce484222325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        });
    assert_eq!(checksum, 0x43d0ffb8c640e4e7);
    close(source_linear(0.04045), 0.003130805);
    close(source_linear(0.5), 0.21404114);
    close(source_linear(1.25), 1.6338118);
    assert!((source_linear(0.5 * 1.25) - source_linear(0.5) * 1.25).abs() > 0.08);
}

#[test]
fn directional_cookie_phase_survives_large_origin_shifts() {
    use my_summer_slop::lighting::directional_cookie_phase;
    let basis = [[0.8_f32, 0., 0.6], [-0.3, 0.8660254, 0.4]];
    let anchor = [-17.45_f32, 95.48, -280.1];
    let size = 10000_f32;
    for origin_z in [0., -512., -100_000_000.] {
        let origin = [1024., 0., origin_z];
        let local = [237_f32, 4., -600.];
        let absolute: [f64; 3] = std::array::from_fn(|i| origin[i] + f64::from(local[i]));
        for shift in [[0., 0., 0.], [512., 0., -512.], [-1024., 0., 1024.]] {
            let rebased = std::array::from_fn(|i| origin[i] + shift[i]);
            let position: [f32; 3] = std::array::from_fn(|i| (absolute[i] - rebased[i]) as f32);
            let phase = directional_cookie_phase(basis, rebased, anchor, size);
            for axis in 0..2 {
                assert!((0. ..1.).contains(&phase[axis]));
                let gpu = (basis[axis][0] * position[0]
                    + basis[axis][1] * position[1]
                    + basis[axis][2] * position[2])
                    / size
                    + phase[axis]
                    + 0.5;
                let expected = ((0..3)
                    .map(|i| f64::from(basis[axis][i]) * (absolute[i] - f64::from(anchor[i])))
                    .sum::<f64>()
                    / f64::from(size)
                    + 0.5)
                    .rem_euclid(1.);
                let difference = (f64::from(gpu.rem_euclid(1.)) - expected).abs();
                assert!(
                    difference.min(1. - difference) < 0.000001,
                    "origin {rebased:?}, axis {axis}, difference {difference}"
                );
            }
        }
    }
}

#[test]
fn spot_cookie_matches_native_projection_fixtures() {
    use my_summer_slop::lighting::spot_cookie_matrix;
    // Isolated UnityPlayer CalculateLightMatrix, RVA 0x312ff0, cached cotangent 1.
    // DLL SHA-256: 553c1e7560e1b42bd4638c53d0168dcd1b4b9eb5b9df324b0a44f14b3f18bc86.
    let half = std::f32::consts::FRAC_1_SQRT_2;
    let fixtures = [
        (
            8.,
            [0., 0., 0., 1.],
            [
                [0.125, 0., 0., 0.],
                [0., 0.125, 0., 0.],
                [0., 0., 0.125, 0.25],
                [0.; 4],
            ],
            [[1., 0.5], [0.5, 1.]],
        ),
        (
            32.,
            [0., 0., 0., 1.],
            [
                [0.03125, 0., 0., 0.],
                [0., 0.03125, 0., 0.],
                [0., 0., 0.03125, 0.0625],
                [0.; 4],
            ],
            [[1., 0.5], [0.5, 1.]],
        ),
        (
            8.,
            [0., 0., half, half],
            [
                [0., -0.125, 0., 0.],
                [0.125, 0., 0., 0.],
                [0., 0., 0.125, 0.25],
                [0.; 4],
            ],
            [[0.5, 0.], [1., 0.5]],
        ),
    ];
    for (range, rotation, native, expected_uv) in fixtures {
        let pose = my_summer_slop::reference::Transform {
            rotation,
            translation: [0.; 3],
            scale: [1.; 3],
        };
        let matrix = spot_cookie_matrix(&pose, range, 1.);
        let mut scaled = pose.clone();
        scaled.scale = [100., 0.25, 3.];
        assert_eq!(spot_cookie_matrix(&scaled, range, 1.), matrix);
        for column in 0..4 {
            for row in 0..4 {
                close(
                    matrix[column][row],
                    native[column][row] * if column == 2 { -1. } else { 1. },
                );
            }
        }
        let project = |point: [f32; 3]| {
            let input = [point[0], point[1], -point[2], 1.];
            let coord: [f32; 4] = std::array::from_fn(|row| {
                (0..4)
                    .map(|column| matrix[column][row] * input[column])
                    .sum()
            });
            (
                [coord[0] / coord[3] + 0.5, coord[1] / coord[3] + 0.5],
                coord[2] > 0.,
            )
        };
        for (point, uv) in [
            ([1., 0., 1.], expected_uv[0]),
            ([0., 1., 1.], expected_uv[1]),
            ([0., 0., 1.], [0.5, 0.5]),
        ] {
            let (actual, front) = project(point);
            close(actual[0], uv[0]);
            close(actual[1], uv[1]);
            assert!(front);
        }
        assert!(!project([0., 0., -1.]).1);
    }
}
