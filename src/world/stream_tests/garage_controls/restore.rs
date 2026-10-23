use super::*;
use crate::reference::EndpointReactionReference;
use crate::save::CameraSave;
use crate::simulation::instruments::{InstrumentSample, InstrumentState, needle_angle};

#[test]
fn g10_control_restore_has_no_side_effects() {
    let (reference, mut run) = load_garage();
    let stamp = super::super::roadside_stamp(&reference);
    let control = reference
        .mechanisms
        .iter()
        .find(|m| matches!(m.reaction, EndpointReactionReference::Rotation { .. }))
        .unwrap();
    let mut state = run
        .area
        .interaction
        .mechanisms
        .snapshot(control.item)
        .unwrap();
    state.degrees = control
        .rotation
        .as_ref()
        .unwrap()
        .limits_degrees
        .map(|r| r[0]);
    assert_ne!(
        state.degrees,
        control.rotation.as_ref().unwrap().initial_degrees
    );
    run.area
        .interaction
        .mechanisms
        .restore(control.item, state)
        .unwrap();
    run.lighting.cycle_headlights(&reference.lighting, 0);
    run.vehicles[0].handbrake = !run.vehicles[0].handbrake;
    let vehicle = &mut run.vehicles[0];
    vehicle.ignition.ignition = true;
    let mut engine = vehicle.engine.capture_save();
    engine.rpm = 725.;
    engine.temperature = 65.;
    vehicle.engine_output = vehicle.engine.restore_save(&engine).unwrap();
    vehicle.trip_m = 321.;
    let radio = &mut run.audio.as_mut().unwrap().radio;
    radio.on = true;
    radio.am = true;
    radio.tuning = 0.83;
    radio.volume = 0.65;
    let saved = run
        .capture_save(
            &reference,
            &stamp,
            CameraSave::FirstPerson,
            "2026-10-20T12:00:00Z",
        )
        .unwrap();
    let mut restored = GarageRuntime::from_save(&reference, &stamp, &saved).unwrap();
    super::super::prepare_ready(&mut restored, &reference);
    assert!(restored.advance_saved_restore(&reference).unwrap());
    assert!(!restored.restore_pending());
    let recaptured = restored
        .capture_save(&reference, &stamp, CameraSave::FirstPerson, &saved.saved_at)
        .unwrap();
    assert_eq!(recaptured.mechanisms, saved.mechanisms);
    assert_eq!(recaptured.lighting, saved.lighting);
    assert_eq!(
        recaptured.vehicles[0].requested_headlights,
        saved.vehicles[0].requested_headlights
    );
    assert_eq!(
        recaptured.vehicles[0].handbrake,
        saved.vehicles[0].handbrake
    );
    assert_eq!(recaptured.vehicles[0].engine, saved.vehicles[0].engine);
    assert_eq!(recaptured.vehicles[0].trip_m, saved.vehicles[0].trip_m);
    for expected in saved
        .bodies
        .iter()
        .chain(saved.vehicles.iter().map(|v| &v.body))
    {
        let actual = recaptured
            .bodies
            .iter()
            .chain(recaptured.vehicles.iter().map(|v| &v.body))
            .find(|body| body.identity == expected.identity)
            .unwrap();
        assert_eq!(actual.linear_velocity, expected.linear_velocity);
        assert_eq!(actual.angular_velocity, expected.angular_velocity);
    }
    let motion: Vec<_> = restored
        .physics
        .rapier()
        .bodies
        .iter()
        .map(|(h, b)| (h, b.linvel(), b.angvel()))
        .collect();
    restored
        .area
        .interaction
        .mechanisms
        .apply_reactions(&mut restored.physics);
    for (handle, linear, angular) in motion {
        let body = &restored.physics.rapier().bodies[handle];
        assert_eq!(body.linvel(), linear);
        assert_eq!(body.angvel(), angular);
    }
    let audio = restored.audio.as_mut().unwrap();
    assert_eq!(audio.event_count(), 0);
    audio.fixed_step(
        &restored.area,
        &restored.physics,
        crate::simulation::FIXED_DT,
    );
    let endpoints: Vec<_> = audio
        .reference
        .mechanisms
        .iter()
        .flat_map(|m| m.endpoints.iter().copied())
        .collect();
    while let Some(event) = audio.pop_event() {
        assert!(
            !endpoints.contains(&event.binding),
            "restore replayed an endpoint sound"
        );
    }
    // G13 does not persist radio controls. Its actual restored state owns visuals.
    let initial = run.audio.as_ref().unwrap().reference.radio.clone();
    let defaults = crate::simulation::radio::RadioState::new(
        &initial,
        &run.audio.as_ref().unwrap().reference.clips,
        [10, 20],
    );
    assert_eq!(
        (
            audio.radio.on,
            audio.radio.am,
            audio.radio.tuning,
            audio.radio.volume
        ),
        (defaults.on, defaults.am, defaults.tuning, defaults.volume)
    );
    assert_eq!(
        audio.radio_frame.power,
        audio.radio.on && initial.electronics_output > initial.electronics_cutoff
    );
    let r = &reference.vehicles[0].instruments;
    let sample = restored.instrument_sample(&reference, 0);
    let mut visual = InstrumentState::new(r);
    visual.initialize_from_sample(r, sample);
    assert!(
        visual.digits_lit,
        "saved ignition and low beams must light the dashboard"
    );
    let temperature = r.needles.iter().position(|n| n.signal == "temp").unwrap();
    assert_eq!(
        visual.needle_angles[temperature],
        needle_angle(&r.needles[temperature], sample.temperature)
    );
    let expected = (
        visual.needle_angles,
        visual.digit_angles,
        visual.lamp_bits,
        visual.digits_lit,
    );
    visual.update(
        r,
        InstrumentSample {
            headlights: 2,
            local_ignition: true,
            ..sample
        },
        1.,
    );
    visual.initialize_from_sample(r, sample);
    assert_eq!(
        (
            visual.needle_angles,
            visual.digit_angles,
            visual.lamp_bits,
            visual.digits_lit
        ),
        expected
    );
    assert_eq!(
        visual.digits_lit,
        sample.local_ignition && matches!(sample.headlights, 1 | 2)
    );
}
