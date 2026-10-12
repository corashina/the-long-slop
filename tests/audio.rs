mod common;
use my_summer_slop::reference::{AudioReference, ReferenceManifest};

#[test]
fn audio_failure_does_not_reject_gameplay() {
    let mut value = serde_json::to_value(common::reference()).unwrap();
    value["audio"] = serde_json::json!({"clips": "invalid"});
    let game = ReferenceManifest::from_json(&serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(AudioReference::from_value(&game.audio, &game).is_err());
}

#[test]
fn selected_audio_contract_validates() {
    let game = common::reference();
    let audio = AudioReference::from_value(&game.audio, &game).unwrap();
    assert_eq!(audio.radio.stations.len(), 16);
    let custom = audio.radio.stations.iter().find(|s| s.custom).unwrap();
    assert_eq!(custom.frequency, 1080.0);
    assert!(custom.music.is_empty() && custom.talk.is_empty());
    assert_eq!(audio.shared.audio_manager.real_voices, 128);
    assert_eq!(audio.shared.audio_manager.virtual_voices, 512);
    assert!(audio.shared.enforced_voice_limit.is_none());
    assert_eq!(audio.radio.clock.kind, "scaled");
    assert_eq!(audio.radio.clock.selected.method, "RADIO");
    assert_eq!(audio.radio.clock.multiplayer.method, "RADIO2");
}

#[test]
fn audio_links_and_ranges_are_required() {
    let game = common::reference();
    for (section, field, value) in [
        ("voices", "clip", serde_json::json!(999999)),
        ("voices", "emitter", serde_json::json!(999999)),
        ("voices", "mixer", serde_json::json!(999999)),
        ("voices", "max_distance", serde_json::json!(0)),
        ("emitters", "node", serde_json::json!(999999)),
        ("bindings", "clips", serde_json::json!([999999])),
    ] {
        let mut raw = game.audio.clone();
        raw[section][0][field] = value;
        assert!(
            AudioReference::from_value(&raw, &game).is_err(),
            "{section}.{field}"
        );
    }
}

#[test]
fn missing_mixer_ancestor_returns_audio_error() {
    let game = common::reference();
    let mut raw = game.audio.clone();
    raw["mixers"][0]["parent"] = serde_json::json!(2);
    raw["mixers"][1]["parent"] = serde_json::json!(999999);
    assert!(AudioReference::from_value(&raw, &game).is_err());
}

#[test]
fn sample_metadata_overflow_returns_audio_error() {
    let game = common::reference();
    let mut alignment = game.audio.clone();
    alignment["clips"][0]["channels"] = serde_json::json!(65535);
    alignment["clips"][0]["bits_per_sample"] = serde_json::json!(64);
    assert!(AudioReference::from_value(&alignment, &game).is_err());

    let mut data_size = game.audio.clone();
    data_size["clips"][0]["frames"] = serde_json::json!(u64::MAX);
    assert!(AudioReference::from_value(&data_size, &game).is_err());
}

#[test]
fn shared_source_settings_require_voice_ranges_and_curves() {
    let game = common::reference();
    for (field, value) in [
        ("gain", serde_json::json!(-1)),
        ("pitch", serde_json::json!(0)),
        ("min_distance", serde_json::json!(-1)),
        ("max_distance", serde_json::json!(0)),
        ("spatial_blend", serde_json::json!(2)),
        ("rolloff_mode", serde_json::json!(3)),
    ] {
        let mut raw = game.audio.clone();
        raw["shared"]["settings"][field] = value;
        assert!(AudioReference::from_value(&raw, &game).is_err(), "{field}");
    }
    for field in ["rolloff_curve", "spatial_blend_curve"] {
        let mut raw = game.audio.clone();
        raw["shared"]["settings"][field]["keys"] = serde_json::json!([]);
        assert!(AudioReference::from_value(&raw, &game).is_err(), "{field}");
    }
}

#[test]
fn bounded_events_keep_priority_and_reset_session() {
    use my_summer_slop::audio::{AudioEvent, AudioState};
    let game = common::reference();
    let mut state = AudioState::new(AudioReference::from_value(&game.audio, &game).unwrap(), 1);
    let make = |sequence, priority| AudioEvent {
        session: 1,
        sequence,
        binding: 29,
        emitter: 56,
        position: [0.; 3],
        gain: 1.,
        pitch: 1.,
        time_seconds: 0.,
        priority,
    };
    state.push_event(make(1, 25));
    state.push_event(make(2, 25));
    assert_eq!(state.pop_event().unwrap().sequence, 1);
    assert_eq!(state.pop_event().unwrap().sequence, 2);
    assert!(state.pop_event().is_none());
    for sequence in 1..=65 {
        state.push_event(make(sequence, if sequence == 1 { 255 } else { 10 }));
    }
    assert_eq!(state.event_count(), 64);
    assert_eq!(state.pop_event().unwrap().sequence, 2);
    state.reset(2);
    assert!(state.pop_event().is_none());
    state.push_event(make(66, 10));
    assert!(state.pop_event().is_none());
}

#[test]
fn source_interior_mixer_and_sensor_latch() {
    use my_summer_slop::audio::{SensorLatch, interior_transmission, source_gain_db};
    assert!((source_gain_db(0.5) - 20. * 0.5_f32.ln()).abs() < 1e-6);
    assert_eq!(source_gain_db(2.), 0.);
    assert_eq!(interior_transmission(0., 4, 2, 0.6), 0.);
    assert_eq!(interior_transmission(1., 4, 2, 0.6), 0.25);
    assert_eq!(interior_transmission(2., 4, 2, 0.6), 0.6);
    let mut latch = SensorLatch::new(false, 0, 6);
    latch.begin_step();
    latch.hits.push(Some(5));
    latch.hits.push(Some(4));
    latch.hit();
    assert_eq!(latch.hits[0], Some(5));
    for count in [2, 1, 0] {
        latch.begin_step();
        assert!(latch.sensed);
        assert_eq!(latch.count, count);
    }
    latch.begin_step();
    assert!(!latch.sensed);
    assert!(latch.hits.is_empty());
    // Source closed/open door counts control the Outside gain before dB conversion.
    let closed = 0.5 + (1. - 0.8) * interior_transmission(0., 4, 2, 0.6);
    let open = 0.5 + (1. - 0.8) * interior_transmission(2., 4, 2, 0.6);
    assert!(open > closed);
    assert!(source_gain_db(open) > source_gain_db(closed));
}

#[test]
fn ignition_event_requires_a_successful_change_and_restart_clears_it() {
    use my_summer_slop::{
        simulation::{FrameInput, keys},
        starting_area::GarageRuntime,
        vehicle::VehiclePose,
    };
    let mut game = common::reference();
    let vehicle = game
        .items
        .iter()
        .find(|i| i.node == game.vehicle.root)
        .unwrap()
        .id;
    game.items
        .iter_mut()
        .find(|i| i.id == game.engine.item)
        .unwrap()
        .attached_to = Some(vehicle);
    let mut run = GarageRuntime::new(&game).unwrap();
    run.started = true;
    let input = FrameInput {
        held: keys::IGNITION,
        pressed: keys::IGNITION,
        ..Default::default()
    };
    run.step(&game, input);
    assert!(run.audio.as_mut().unwrap().pop_event().is_none());
    let seat = game.seats.iter().find(|s| s.driver).unwrap();
    assert!(run.area.player.enter(
        seat.id,
        VehiclePose {
            position: [20., 2., 20.],
            rotation: [0., 0., 0., 1.]
        },
        &mut run.physics
    ));
    run.step(&game, input);
    run.step(
        &game,
        FrameInput {
            released: keys::IGNITION,
            ..Default::default()
        },
    );
    let event = run
        .audio
        .as_mut()
        .unwrap()
        .pop_event()
        .expect("successful ignition");
    assert_eq!(event.sequence, 1);
    assert_eq!(
        run.audio
            .as_ref()
            .unwrap()
            .binding(event.binding)
            .unwrap()
            .trigger,
        "ignition"
    );
    run.step(
        &game,
        FrameInput {
            held: keys::IGNITION,
            ..Default::default()
        },
    );
    assert!(run.audio.as_mut().unwrap().pop_event().is_none());
    run.restart(&game).unwrap();
    assert_eq!(run.audio.as_ref().unwrap().session, 2);
    assert!(run.audio.as_mut().unwrap().pop_event().is_none());
}

#[test]
#[ignore = "requires installed source export"]
fn installed_doors_change_source_interior_mixer() {
    use my_summer_slop::{audio::AudioState, starting_area::GarageRuntime};
    let game =
        ReferenceManifest::from_json(&std::fs::read("web/assets/reference/manifest.json").unwrap())
            .unwrap();
    let mut run = GarageRuntime::new(&game).unwrap();
    let mut state = AudioState::new(AudioReference::from_value(&game.audio, &game).unwrap(), 1);
    state.bind_game(&game, &run.area, &run.physics);
    let interior = state
        .reference
        .interiors
        .iter()
        .position(|i| i.id == 5)
        .unwrap();
    state.sensor.sensed = true;
    state.sensor.hits.push(Some(interior));
    state.update_mixers(&run.area, 0.5);
    assert!((state.outside_gain - 0.5).abs() < 1e-6);
    let slots = state.reference.interiors[interior].door_slots.clone();
    for slot in slots.iter().take(2) {
        let item = run.area.interaction.parts.slot_item(*slot).unwrap();
        run.area
            .interaction
            .parts
            .remove(item, &mut run.area.items, &mut run.physics)
            .unwrap();
    }
    state.update_mixers(&run.area, 0.5);
    assert!((state.outside_gain - 0.62).abs() < 1e-6);
    assert_eq!(
        state.route_mixers[0],
        state.reference.routes[0].car_mixer.unwrap()
    );
}
