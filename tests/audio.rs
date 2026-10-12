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

#[test]
fn engine_formula_boundaries() {
    use my_summer_slop::audio::{AudioState, VehicleAudioInput};
    let game = common::reference();
    let mut state = AudioState::new(AudioReference::from_value(&game.audio, &game).unwrap(), 1);
    let e = state.reference.engine.clone();
    for (rpm, idle, motor, ip, mp) in [
        (
            600.,
            0.7,
            0.0875,
            0.9 + 0.3 * 600. / 1400.,
            0.8 + 0.7 * 100. / 6100.,
        ),
        (1400., 0., 0.7, 1.2, 0.8 + 0.7 * 900. / 6100.),
        (6600., 0., 0.7, 1.5, 1.5),
        (13200., 0., 0.7, 1.5, 2.2),
    ] {
        state.vehicle(
            VehicleAudioInput {
                rpm,
                running: true,
                engine_present: true,
                ..Default::default()
            },
            1. / 60.,
            1.,
        );
        let a = state.signals.iter().find(|s| s.id == e.idle_voice).unwrap();
        let b = state
            .signals
            .iter()
            .find(|s| s.id == e.motor_voice)
            .unwrap();
        for (actual, expected) in [
            (a.gain, idle),
            (b.gain, motor),
            (a.pitch, ip),
            (b.pitch, mp),
        ] {
            assert!(
                (actual - expected).abs() < 1e-5,
                "{rpm}: {actual} != {expected}"
            );
        }
        assert!(a.playing && b.playing);
    }
}
#[test]
fn horn_uses_ambient_temperature() {
    use my_summer_slop::audio::{AudioState, VehicleAudioInput};
    let game = common::reference();
    let mut state = AudioState::new(AudioReference::from_value(&game.audio, &game).unwrap(), 1);
    let h = state.reference.horn.clone();
    for (temp, pitch, gain) in [
        (h.temperature_min, h.pitch_min, 0.1),
        (h.temperature_max, h.pitch_max, 0.2),
    ] {
        state.vehicle(
            VehicleAudioInput {
                horn: true,
                ambient_temperature: temp,
                ..Default::default()
            },
            0.01,
            1.,
        );
        let s = state.signals.iter().find(|s| s.id == h.voice).unwrap();
        assert!((s.pitch - pitch).abs() < 1e-6);
        assert!((s.gain - gain).abs() < 1e-6);
        assert!(s.playing);
    }
    state.vehicle(VehicleAudioInput::default(), 0.01, 1.);
    assert!((state.signals.iter().find(|s| s.id == h.voice).unwrap().gain - 0.1).abs() < 1e-6);
    state.vehicle(VehicleAudioInput::default(), 0.01, 1.);
    assert!(
        !state
            .signals
            .iter()
            .find(|s| s.id == h.voice)
            .unwrap()
            .playing
    );
}
#[test]
fn engine_removal_preserves_car_audio() {
    use my_summer_slop::audio::{AudioState, VehicleAudioInput};
    let game = common::reference();
    let mut state = AudioState::new(AudioReference::from_value(&game.audio, &game).unwrap(), 1);
    let radio = state.reference.radio.voices[0];
    state.voice(radio, true, 1., 1.);
    state.vehicle(
        VehicleAudioInput {
            running: true,
            starter: true,
            engine_present: true,
            horn: true,
            ..Default::default()
        },
        0.1,
        1.,
    );
    state.vehicle(
        VehicleAudioInput {
            horn: true,
            ..Default::default()
        },
        0.1,
        2.,
    );
    assert!(!state.starter_enabled);
    assert!(
        !state
            .signals
            .iter()
            .find(|s| s.id == state.reference.engine.idle_voice)
            .unwrap()
            .playing
    );
    assert!(
        state
            .signals
            .iter()
            .find(|s| s.id == state.reference.horn.voice)
            .unwrap()
            .playing
    );
    assert!(
        state
            .signals
            .iter()
            .find(|s| s.id == radio)
            .unwrap()
            .playing
    );
    assert_eq!(
        state.event_count(),
        0,
        "removal must not play an engine stop tail"
    );
}
#[test]
fn ignition_edges_survive_fixed_steps() {
    use my_summer_slop::{
        simulation::{FrameInput, keys},
        starting_area::GarageRuntime,
        vehicle::VehiclePose,
    };
    let mut game = common::reference();
    let car = game
        .items
        .iter()
        .find(|i| i.node == game.vehicle.root)
        .unwrap()
        .id;
    game.items
        .iter_mut()
        .find(|i| i.id == game.engine.item)
        .unwrap()
        .attached_to = Some(car);
    let mut run = GarageRuntime::new(&game).unwrap();
    run.started = true;
    let seat = game.seats.iter().find(|s| s.driver).unwrap();
    assert!(run.area.player.enter(
        seat.id,
        VehiclePose {
            position: [20., 2., 20.],
            rotation: [0., 0., 0., 1.]
        },
        &mut run.physics
    ));
    for expected in [true, false] {
        run.step(
            &game,
            FrameInput {
                held: keys::IGNITION,
                pressed: keys::IGNITION,
                ..Default::default()
            },
        );
        run.step(
            &game,
            FrameInput {
                released: keys::IGNITION,
                ..Default::default()
            },
        );
        assert_eq!(run.engine_output.ignition, expected);
    }
    let state = run.audio.as_mut().unwrap();
    assert_eq!(state.pop_event().unwrap().sequence, 1);
    assert_eq!(state.pop_event().unwrap().sequence, 2);
    assert!(state.pop_event().is_none());
}

#[test]
fn engine_stop_edge_emits_once() {
    use my_summer_slop::audio::{AudioState, VehicleAudioInput};
    let game = common::reference();
    let mut state = AudioState::new(AudioReference::from_value(&game.audio, &game).unwrap(), 1);
    state.vehicle(
        VehicleAudioInput {
            running: true,
            engine_present: true,
            ..Default::default()
        },
        0.1,
        1.,
    );
    state.vehicle(
        VehicleAudioInput {
            engine_present: true,
            ..Default::default()
        },
        0.1,
        2.,
    );
    state.vehicle(
        VehicleAudioInput {
            engine_present: true,
            ..Default::default()
        },
        0.1,
        3.,
    );
    let event = state.pop_event().unwrap();
    assert_eq!(event.binding, state.reference.engine.stop_binding.unwrap());
    assert_eq!(event.time_seconds, 2.);
    assert!(state.pop_event().is_none());
}

fn interaction_audio() -> (ReferenceManifest, my_summer_slop::audio::AudioState) {
    use my_summer_slop::{audio::AudioState, starting_area::GarageRuntime};
    let mut game = common::reference();
    let source = AudioReference::from_value(&game.audio, &game)
        .unwrap()
        .bindings
        .iter()
        .find(|b| b.trigger == "crafted")
        .unwrap()
        .source
        .clone();
    game.items[0].pickup.as_mut().unwrap().source = source;
    let run = GarageRuntime::new(&game).unwrap();
    let mut state = AudioState::new(AudioReference::from_value(&game.audio, &game).unwrap(), 1);
    state.bind_game(&game, &run.area, &run.physics);
    (game, state)
}
#[test]
fn failed_attachment_is_silent() {
    use my_summer_slop::interaction::InteractionAction;
    let (game, mut state) = interaction_audio();
    let run = my_summer_slop::starting_area::GarageRuntime::new(&game).unwrap();
    // A failed operation is never submitted as a completed action.
    assert!(
        run.area
            .interaction
            .parts
            .can_install(
                game.items[0].id,
                my_summer_slop::reference::PartSlotId(999),
                &run.area.items
            )
            .is_err()
    );
    assert_eq!(state.event_count(), 0);
    state.action(InteractionAction::PartInstall(game.items[0].id), 1.);
    let e = state.pop_event().unwrap();
    assert_eq!(e.gain * state.binding(e.binding).unwrap().gain, 0.25);
    assert!((0.85..=1.15).contains(&e.pitch));
}
#[test]
fn silent_pickup_has_no_substitute() {
    use my_summer_slop::interaction::InteractionAction;
    let (game, mut state) = interaction_audio();
    state.action(InteractionAction::Pickup(game.items[0].id), 1.);
    state.action(InteractionAction::Drop(game.items[0].id), 1.);
    assert_eq!(state.event_count(), 0);
}
#[test]
fn multiple_steps_preserve_actions() {
    use my_summer_slop::interaction::InteractionAction;
    let (game, mut state) = interaction_audio();
    for step in 0..6 {
        state.action(
            InteractionAction::PartInstall(game.items[0].id),
            step as f64 / 60.,
        );
    }
    for step in 0..6 {
        let e = state.pop_event().unwrap();
        assert_eq!(e.sequence, step + 1);
        assert_eq!(e.time_seconds, step as f64 / 60.);
    }
    assert!(state.pop_event().is_none());
}
#[test]
fn removal_discards_pending_owned_events() {
    use my_summer_slop::interaction::InteractionAction;
    let (game, mut state) = interaction_audio();
    state.action(InteractionAction::PartInstall(game.items[0].id), 1.);
    let id = state
        .reference
        .bindings
        .iter()
        .find(|b| b.trigger == "crafted")
        .unwrap()
        .emitter;
    state.remove_owner(id);
    state.action(InteractionAction::PartInstall(game.items[0].id), 2.);
    assert_eq!(state.event_count(), 0);
}

#[test]
fn mechanism_endpoints_emit_once() {
    use my_summer_slop::{audio::AudioState, starting_area::GarageRuntime};
    let mut game = common::reference();
    let body = game.bodies[1].id.0;
    common::add_rotation(&mut game, 1, body, [0., 10.]);
    let audio = AudioReference::from_value(&game.audio, &game).unwrap();
    game.mechanisms[0].source = audio.mechanisms[0].source.clone();
    let run = GarageRuntime::new(&game).unwrap();
    let mut state = AudioState::new(audio, 1);
    state.bind_game(&game, &run.area, &run.physics);
    let item = game.mechanisms[0].item;
    state.mechanism(item, 0., 10., 1. / 60.);
    let e = state.pop_event().unwrap();
    assert_eq!(state.binding(e.binding).unwrap().trigger, "endPosEnd");
    assert!((e.gain - 6.).abs() < 1e-6);
    for _ in 0..6 {
        state.mechanism(item, 10., 10., 1. / 60.);
    }
    assert_eq!(state.event_count(), 0);
    let voice = state.reference.mechanisms[0].movement_voice.unwrap();
    assert!(
        !state
            .signals
            .iter()
            .find(|s| s.id == voice)
            .unwrap()
            .playing
    );
}
#[test]
fn fluid_events_require_actual_flow() {
    use my_summer_slop::{
        audio::AudioState, interaction::InteractionAction, reference::*,
        starting_area::GarageRuntime,
    };
    let mut game = common::reference();
    let audio = AudioReference::from_value(&game.audio, &game).unwrap();
    let opening = game.openings[0].clone();
    let run = GarageRuntime::new(&game).unwrap();
    let mut state = AudioState::new(audio, 1);
    state.bind_game(&game, &run.area, &run.physics);
    let voice = state.reference.fluids[0].fill_voice.unwrap();
    let fill = |liters| InteractionAction::Fill {
        source: game.items[1].id,
        destination: opening.tank,
        liters,
    };
    state.action(fill(0.), 1.);
    assert!(
        !state
            .signals
            .iter()
            .find(|s| s.id == voice)
            .unwrap()
            .playing
    );
    state.cap(OpeningId(1), 0., 0.5, Some(0.5));
    assert_eq!(state.event_count(), 0);
    state.cap(OpeningId(1), 0.5, 1., Some(2.));
    let e = state.pop_event().unwrap();
    assert_eq!(e.gain, 2.);
    assert_eq!(state.binding(e.binding).unwrap().trigger, "cap_tune");
    state.cap(OpeningId(1), 1., 0., None);
    let e = state.pop_event().unwrap();
    assert_eq!(state.binding(e.binding).unwrap().trigger, "cap_close");
}
#[test]
fn room_turn_emits_authored_lamp_edges_only() {
    use my_summer_slop::interaction::InteractionAction;
    let (game, mut state) = interaction_audio();
    let control = state.reference.lamps[0].control;
    let initial = game
        .lighting
        .controllers
        .iter()
        .find(|c| c.id == control)
        .unwrap()
        .startup_on;
    for turn in 0..2 {
        state.action(InteractionAction::LightControl(control), turn as f64);
        let e = state.pop_event().unwrap();
        let on = if turn == 0 { !initial } else { initial };
        assert_eq!(
            state.binding(e.binding).unwrap().trigger,
            if on { "lamp_on" } else { "lamp_off" }
        );
    }
}

#[test]
fn slot_uncraft_uses_crafted_sound() {
    use my_summer_slop::interaction::InteractionAction;
    let (game, mut state) = interaction_audio();
    for action in [
        InteractionAction::PartRemove(game.items[0].id),
        InteractionAction::WheelRemove(game.items[0].id),
        InteractionAction::WheelJoin {
            rim: game.items[1].id,
            tire: game.items[0].id,
        },
    ] {
        state.action(action, 1.);
        let e = state.pop_event().unwrap();
        assert_eq!(state.binding(e.binding).unwrap().trigger, "crafted");
        assert_eq!(state.binding(e.binding).unwrap().gain * e.gain, 0.25);
    }
}
#[test]
#[ignore = "requires installed source export"]
fn installed_lamp_turn_updates_source_hum_voices() {
    use my_summer_slop::{interaction::InteractionAction, starting_area::GarageRuntime};
    let game =
        ReferenceManifest::from_json(&std::fs::read("web/assets/reference/manifest.json").unwrap())
            .unwrap();
    let mut run = GarageRuntime::new(&game).unwrap();
    let state = run.audio.as_mut().unwrap();
    let lamps = state.reference.lamps.clone();
    for lamp in lamps {
        assert_eq!(
            game.lighting
                .controllers
                .iter()
                .find(|c| c.id == lamp.control)
                .unwrap()
                .kind,
            my_summer_slop::reference::LightControllerKind::House
        );
        state.action(InteractionAction::LightControl(lamp.control), 1.);
        let e = state.pop_event().unwrap();
        assert_eq!(state.binding(e.binding).unwrap().trigger, "lamp_on");
        for &id in &lamp.voices {
            assert!(state.signals.iter().find(|s| s.id == id).unwrap().playing);
        }
        state.action(InteractionAction::LightControl(lamp.control), 2.);
        let e = state.pop_event().unwrap();
        assert_eq!(state.binding(e.binding).unwrap().trigger, "lamp_off");
        for &id in &lamp.voices {
            assert!(!state.signals.iter().find(|s| s.id == id).unwrap().playing);
        }
    }
}

#[test]
fn cap_steps_preserve_completed_source_order() {
    use my_summer_slop::{
        interaction::{AimTarget, InteractionSound},
        simulation::{FrameInput, keys},
        starting_area::GarageRuntime,
    };
    let mut game = common::reference();
    game.openings[0].can_toggle = true;
    game.openings[0].can_tune = false;
    let mut run = GarageRuntime::new(&game).unwrap();
    let target = AimTarget {
        collider: rapier3d::prelude::ColliderHandle::invalid(),
        item: game.openings[0].cap,
        distance: 1.,
        point: [0.; 3],
        slot: None,
    };
    for step in 0..6 {
        let action = run.area.interaction.step(
            FrameInput {
                pressed: keys::USE,
                ..Default::default()
            },
            common::resolved(Some(target)),
            &mut run.area.player,
            &mut run.area.items,
            &mut run.physics,
            1. / 60.,
        );
        assert!(matches!(
            action,
            Some(my_summer_slop::interaction::InteractionAction::Cap(_))
        ));
        let audio = run.audio.as_mut().unwrap();
        audio.fixed_step(&run.area, &run.physics, 1. / 60.);
        while let Some(sound) = run.area.interaction.pop_sound() {
            if let InteractionSound::Cap {
                id,
                previous,
                current,
                tune,
            } = sound
            {
                audio.cap(id, previous, current, tune);
            }
        }
        assert_eq!(audio.event_count(), step + 1);
    }
    let audio = run.audio.as_mut().unwrap();
    for step in 0..6 {
        let e = audio.pop_event().unwrap();
        assert_eq!(e.sequence, step + 1);
        assert_eq!(
            audio.binding(e.binding).unwrap().trigger,
            if step % 2 == 0 {
                "cap_open"
            } else {
                "cap_close"
            }
        );
    }
    assert!(audio.pop_event().is_none());
    assert!(run.area.interaction.pop_sound().is_none());
}
#[test]
fn actual_spill_uses_source_speed_gain_and_empty_tank_stops() {
    use my_summer_slop::{simulation::FrameInput, starting_area::GarageRuntime};
    let mut game = common::reference();
    game.openings[0].angle_disabled = true;
    game.openings[0].valve = 1.;
    game.openings[0].flow_liters_per_second = 0.05;
    game.openings[0].spill_disabled = false;
    game.openings[0].tube_connected = false;
    let tank = game.openings[0].tank;
    let opening = game.openings[0].id;
    let mut run = GarageRuntime::new(&game).unwrap();
    run.area
        .items
        .get_mut(tank)
        .unwrap()
        .fluid
        .as_mut()
        .unwrap()
        .mixture
        .quantities_liters = [1.; 6];
    run.area.interaction.step(
        FrameInput::default(),
        Default::default(),
        &mut run.area.player,
        &mut run.area.items,
        &mut run.physics,
        1. / 60.,
    );
    assert_eq!(
        run.area.interaction.opening_flow(opening),
        Some((false, 0.05))
    );
    let audio = run.audio.as_mut().unwrap();
    audio.fixed_step(&run.area, &run.physics, 1. / 60.);
    let rule = &audio.reference.fluids[0];
    let signal = audio
        .signals
        .iter()
        .find(|s| Some(s.id) == rule.fill_voice)
        .unwrap();
    assert!(signal.playing);
    assert!(signal.looping);
    assert!(
        (signal.gain - (0.05 - rule.pour_speed_min) / (rule.pour_speed_max - rule.pour_speed_min))
            .abs()
            < 1e-6
    );
    run.area
        .items
        .get_mut(tank)
        .unwrap()
        .fluid
        .as_mut()
        .unwrap()
        .mixture
        .quantities_liters = [0.; 6];
    run.area.interaction.step(
        FrameInput::default(),
        Default::default(),
        &mut run.area.player,
        &mut run.area.items,
        &mut run.physics,
        1. / 60.,
    );
    audio.fixed_step(&run.area, &run.physics, 1. / 60.);
    let rule = &audio.reference.fluids[0];
    assert!(
        !audio
            .signals
            .iter()
            .find(|s| Some(s.id) == rule.fill_voice)
            .unwrap()
            .playing
    );
}

#[test]
fn actual_fill_targets_one_cap_and_preserves_non_looping_pour() {
    use my_summer_slop::{
        interaction::AimTarget,
        reference::*,
        simulation::{FrameInput, keys},
        starting_area::GarageRuntime,
    };
    let mut game = common::reference();
    let tank = game.openings[0].tank;
    let cap = game.openings[0].cap;
    let source = game.items[0].id;
    game.tanks.iter_mut().find(|t| t.item == tank).unwrap().role = TankRole::Fixed;
    let mut body = game.bodies[1].clone();
    body.id = BodyId(90);
    body.parent_body = None;
    body.pose.translation = [10., 2., -1.];
    game.bodies.push(body);
    game.items[0].body = BodyId(90);
    game.openings[0].valve = 1.;
    game.openings[0].angle_disabled = false;
    game.openings[0].tilt_degrees = 100.;
    game.openings[0].spill_disabled = false;
    let mut other = game.openings[0].clone();
    other.id = OpeningId(2);
    other.source = AudioReference::from_value(&game.audio, &game)
        .unwrap()
        .fluids[1]
        .source
        .clone();
    other.cap = game.items[1].id;
    game.openings.push(other);
    let mut run = GarageRuntime::new(&game).unwrap();
    run.area
        .items
        .get_mut(source)
        .unwrap()
        .fluid
        .as_mut()
        .unwrap()
        .mixture
        .quantities_liters = [1.; 6];
    run.area
        .items
        .get_mut(tank)
        .unwrap()
        .fluid
        .as_mut()
        .unwrap()
        .mixture
        .quantities_liters = [0.; 6];
    let point = run.area.items.get(source).unwrap().pose.position;
    run.area
        .interaction
        .pickup_item(
            source,
            point,
            &run.area.player,
            &mut run.area.items,
            &mut run.physics,
        )
        .unwrap();
    let target = AimTarget {
        collider: rapier3d::prelude::ColliderHandle::invalid(),
        item: cap,
        distance: 1.,
        point: [0.; 3],
        slot: None,
    };
    let action = run.area.interaction.step(
        FrameInput {
            held: keys::FILL,
            ..Default::default()
        },
        common::resolved(Some(target)),
        &mut run.area.player,
        &mut run.area.items,
        &mut run.physics,
        1. / 60.,
    );
    assert!(
        matches!(action,Some(my_summer_slop::interaction::InteractionAction::Fill{liters,..}) if liters>0.)
    );
    assert_eq!(
        run.area.interaction.opening_flow(OpeningId(1)),
        Some((true, 1.))
    );
    assert_eq!(run.area.interaction.opening_flow(OpeningId(2)), None);
    let audio = run.audio.as_mut().unwrap();
    audio.fixed_step(&run.area, &run.physics, 1. / 60.);
    let first = audio.reference.fluids[0].fill_voice.unwrap();
    let second = audio.reference.fluids[1].fill_voice.unwrap();
    let signal = audio.signals.iter().find(|s| s.id == first).unwrap();
    assert!(signal.playing);
    assert!(!signal.looping);
    assert_eq!(signal.gain, audio.reference.fluids[0].max_gain);
    assert!(
        !audio
            .signals
            .iter()
            .find(|s| s.id == second)
            .unwrap()
            .playing
    );
    for step in 0..4 {
        run.area.interaction.step(
            FrameInput::default(),
            Default::default(),
            &mut run.area.player,
            &mut run.area.items,
            &mut run.physics,
            1. / 60.,
        );
        audio.fixed_step(&run.area, &run.physics, 1. / 60.);
        assert_eq!(
            audio
                .signals
                .iter()
                .find(|s| s.id == first)
                .unwrap()
                .playing,
            step < 3
        );
    }
    run.area
        .items
        .get_mut(tank)
        .unwrap()
        .fluid
        .as_mut()
        .unwrap()
        .mixture
        .quantities_liters = [1.; 6];
    let body = run.area.items.get(tank).unwrap().body.unwrap();
    let mut pose = *run.physics.rapier().bodies[body].position();
    pose.rotation = rapier3d::math::Rotation::from_rotation_x(-std::f32::consts::FRAC_PI_2);
    run.physics.rapier_mut().bodies[body].set_position(pose, true);
    run.area.interaction.step(
        FrameInput::default(),
        Default::default(),
        &mut run.area.player,
        &mut run.area.items,
        &mut run.physics,
        1. / 60.,
    );
    assert!(
        matches!(run.area.interaction.opening_flow(OpeningId(1)),Some((false,speed)) if speed>0.)
    );
    audio.fixed_step(&run.area, &run.physics, 1. / 60.);
    assert!(
        !audio
            .signals
            .iter()
            .find(|s| s.id == first)
            .unwrap()
            .looping
    );
}

#[test]
fn movement_reset_starts_at_source_zero_gain() {
    let (_, mut state) = interaction_audio();
    let id = state.reference.mechanisms[0].movement_voice.unwrap();
    state.voice(id, true, 0.5, 1.);
    state.reset(2);
    let signal = state.signals.iter().find(|s| s.id == id).unwrap();
    assert!(!signal.playing);
    assert_eq!(signal.gain, 0.);
}
