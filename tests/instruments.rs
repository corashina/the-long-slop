mod common;
use my_summer_slop::simulation::instruments::{
    InstrumentSample, InstrumentState, needle_angle, source_rotation,
};
use rapier3d::math::{Rotation, Vector};
#[test]
fn source_endpoints_and_reverse_speed() {
    let r = common::reference();
    let refs = &r.instruments;
    assert_eq!(needle_angle(&refs.needles[0], 0.), -64.);
    assert_eq!(needle_angle(&refs.needles[0], 160.), 64.);
    assert_eq!(needle_angle(&refs.needles[1], 10.), -30.);
    assert_eq!(needle_angle(&refs.needles[1], 130.), 30.);
    assert_eq!(needle_angle(&refs.needles[2], 0.), -26.);
    assert_eq!(needle_angle(&refs.needles[2], 39.), 26.);
    let mut state = InstrumentState::new(refs);
    state.update(
        refs,
        InstrumentSample {
            speed_kmh: -80.,
            local_ignition: true,
            ..Default::default()
        },
        1.,
    );
    assert!(state.needle_angles[0].abs() < 0.001);
}
#[test]
fn ignition_gates_fuel_temperature_and_lamps() {
    let r = common::reference();
    let refs = &r.instruments;
    let mut s = InstrumentState::new(refs);
    s.update(
        refs,
        InstrumentSample {
            speed_kmh: 0.,
            fuel_liters: 39.,
            oil_liters: 5.,
            temperature: 130.,
            rpm: 1000.,
            local_ignition: true,
            distance_m: 0.,
            headlights: 0,
        },
        1.,
    );
    assert_eq!(s.needle_angles[1], 30.);
    assert_eq!(s.needle_angles[2], 26.);
    assert_eq!(s.lamp_bits, 0);
    s.update(
        refs,
        InstrumentSample {
            local_ignition: false,
            ..Default::default()
        },
        1.,
    );
    assert_eq!(s.needle_angles[1], -30.);
    assert_eq!(s.needle_angles[2], -26.);
    assert_eq!(s.lamp_bits, 0);
    s.update(
        refs,
        InstrumentSample {
            local_ignition: true,
            fuel_liters: 4.9,
            oil_liters: 1.9,
            rpm: 299.,
            ..Default::default()
        },
        1.,
    );
    assert_eq!(s.lamp_bits & 7, 7);
    assert_eq!(s.lamp_bits & 8, 0);
}
#[test]
fn source_two_stage_smoothing() {
    let r = common::reference();
    let refs = &r.instruments;
    let mut s = InstrumentState::new(refs);
    s.update(
        refs,
        InstrumentSample {
            local_ignition: true,
            fuel_liters: 39.,
            ..Default::default()
        },
        0.1,
    );
    // Signal 39*0.5=19.5; needle 19.5*0.8=15.6; source angle -26+52*0.4=-5.2.
    assert!((s.needle_angles[2] + 5.2).abs() < 0.001);
}
#[test]
fn reflected_parent_preserves_single_handedness_conversion() {
    let q = Rotation::from_array(source_rotation([0., 90., 0.]));
    let point = q * Vector::new(0., 0., -1.);
    let reflected = Vector::new(-2. * point.x, point.y, point.z);
    assert!((reflected - Vector::new(-2., 0., 0.)).length() < 0.00001);
}
#[test]
fn odometer_rollover_repeat_sample_and_new_game() {
    let r = common::reference();
    let refs = &r.instruments;
    let mut s = InstrumentState::new(refs);
    let sample = InstrumentSample {
        distance_m: 10000.,
        ..Default::default()
    };
    s.update(refs, sample, 1.);
    let before = s.digit_angles;
    assert!(before[0].abs() < 0.01 && before[1].abs() < 0.01);
    assert!((before[2] - 36.).abs() < 0.01);
    s.update(refs, sample, 1.);
    assert_eq!(s.digit_angles, before);
    s.reset(refs);
    assert_eq!(s.digit_angles, [0.; 5]);
}

#[test]
fn odometer_half_turn_follows_unity_positive_direction() {
    let r = common::reference();
    let mut s = InstrumentState::new(&r.instruments);
    s.update(
        &r.instruments,
        InstrumentSample {
            distance_m: 5000.,
            ..Default::default()
        },
        0.01,
    );
    assert!((s.digit_angles[1] - 36.).abs() < 0.001);
}

#[test]
fn runtime_recovery_and_rebase_preserve_odometer() {
    let r = common::reference();
    let mut run = my_summer_slop::starting_area::GarageRuntime::new(&r).unwrap();
    run.started = true;
    run.trip_m = 12345.;
    let mut s = InstrumentState::new(&r.instruments);
    s.update(
        &r.instruments,
        InstrumentSample {
            distance_m: run.trip_m,
            ..Default::default()
        },
        1.,
    );
    let angles = s.digit_angles;
    run.recover(&r);
    run.stream.rebase([600., 0., 900.], &mut run.physics);
    s.update(
        &r.instruments,
        InstrumentSample {
            distance_m: run.trip_m,
            ..Default::default()
        },
        1.,
    );
    assert_eq!(s.digit_angles, angles);
    assert_eq!(run.trip_m, 12345.);
    run = my_summer_slop::starting_area::GarageRuntime::new(&r).unwrap();
    run.started = true;
    s.reset(&r.instruments);
    assert_eq!(run.trip_m, 0.);
    assert_eq!(s.digit_angles, [0.; 5]);
}

#[test]
fn headlights_gate_high_beam_indicator_and_digit_illumination() {
    let mut r = common::reference();
    let mut state = InstrumentState::new(&r.instruments);
    for (headlights, ignition, high_beam, digits_lit) in [
        (0, true, false, false),
        (1, true, false, true),
        (2, true, true, true),
        (2, false, true, false),
        (1, false, false, false),
        (0, false, false, false),
    ] {
        state.update(
            &r.instruments,
            InstrumentSample {
                headlights,
                local_ignition: ignition,
                ..Default::default()
            },
            1.,
        );
        assert_eq!(state.lamp_bits & 8 != 0, high_beam);
        assert_eq!(state.digits_lit, digits_lit);
    }
    r.instruments.electronics_output = r.instruments.electronics_off_threshold;
    state.update(
        &r.instruments,
        InstrumentSample {
            headlights: 2,
            local_ignition: false,
            ..Default::default()
        },
        1.,
    );
    assert_eq!(state.lamp_bits & 8, 0);
    assert!(!state.digits_lit);
    state.reset(&r.instruments);
    assert_eq!(state.lamp_bits, 0);
    assert!(!state.digits_lit);
}

mod lighting_merge {
    use my_summer_slop::{reference::ReferenceManifest, simulation::instruments::update_high_beam};
    #[test]
    fn high_beam_selects_checked_dashboard_materials() {
        let r = ReferenceManifest::from_json(include_bytes!("common/reference.json")).unwrap();
        let lamp = &r.lighting.high_beam;
        assert_eq!(lamp.signal, "headlight2");
        let off = update_high_beam(lamp, false);
        let on = update_high_beam(lamp, true);
        assert_eq!(
            (off.renderer, off.asset, off.material_index),
            (lamp.node, lamp.off.asset, lamp.off.gltf_material)
        );
        assert_eq!(
            (on.renderer, on.asset, on.material_index),
            (lamp.node, lamp.on.asset, lamp.on.gltf_material)
        );
    }
}
