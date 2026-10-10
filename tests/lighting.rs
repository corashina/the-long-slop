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
