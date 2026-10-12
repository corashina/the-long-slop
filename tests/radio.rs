mod common;
use my_summer_slop::reference::AudioReference;
use my_summer_slop::simulation::radio::{RadioControl, RadioState};

fn radio(tuning: f32, am: bool) -> RadioState {
    let game = common::reference();
    let mut audio = AudioReference::from_value(&game.audio, &game).unwrap();
    audio.radio.initial_tuning = tuning;
    audio.radio.initial_am = am;
    RadioState::new(&audio.radio, &audio.clips, [10, 12])
}
#[test]
fn volume_controls_power() {
    let mut r = radio(0., false);
    assert!(!r.advance(0., 1.).power);
    r.control(RadioControl::VolumeUp);
    assert_eq!(r.advance(0., 1.).volume, 0.1);
    assert!(r.advance(0., 1.).power);
    for _ in 0..20 {
        r.control(RadioControl::VolumeUp);
    }
    assert_eq!(r.advance(0., 1.).volume, 1.);
    for _ in 0..20 {
        r.control(RadioControl::VolumeDown);
    }
    assert!(!r.advance(0., 1.).power);
}
#[test]
fn tuning_uses_authored_ranges() {
    for (am, min, max, step) in [(false, 88., 108., 0.1), (true, 540., 1600., 5.)] {
        let mut r = radio(0., am);
        r.control(RadioControl::TuneDown);
        assert_eq!(r.advance(0., 1.).frequency, min);
        r.control(RadioControl::TuneUp);
        assert!((r.advance(0., 1.).frequency - min - step).abs() < 0.001);
        for _ in 0..220 {
            r.control(RadioControl::TuneUp);
        }
        assert!((r.advance(0., 1.).frequency - max).abs() < 0.001);
    }
    assert!((radio(0.0125, false).advance(0., 1.).frequency - 88.2).abs() < 0.00001);
}
#[test]
fn secondary_station_preserves_source_order() {
    let game = common::reference();
    let mut a = AudioReference::from_value(&game.audio, &game).unwrap();
    a.radio.initial_tuning = 0.5;
    a.radio.stations.retain(|s| s.band == "fm");
    for (s, f) in a.radio.stations.iter_mut().zip([
        100., 98.1, 98.2, 98., 97.9, 99., 99., 99., 99., 99., 99., 99.,
    ]) {
        s.frequency = f;
    }
    let mut r = RadioState::new(&a.radio, &a.clips, [10, 12]);
    let f = r.advance(0., 1.);
    assert_eq!(f.primary.station, Some(a.radio.stations[3].id));
    assert_eq!(f.secondary.station, Some(a.radio.stations[1].id));
}
#[test]
fn radio_signal_uses_electronics() {
    for (am, tuning, freq, tolerance) in [
        (false, 0.06, 89.2, 0.4000000059604645_f32),
        (true, 90. / 1060., 630., 20.),
    ] {
        let mut r = radio(tuning, am);
        r.control(RadioControl::VolumeUp);
        let f = r.advance(0., 0.5);
        assert!((f.primary.gain - 0.05).abs() < 0.0001);
        assert!((f.static_gain - 0.05).abs() < 0.0001);
        assert!((f.frequency - freq).abs() < 0.001);
        r.control(RadioControl::TuneUp);
        let weak = r.advance(0., 0.5);
        let distance = (weak.frequency - freq).abs();
        let expected = (1. - distance / tolerance).clamp(0., 1.) * 0.5 * 0.1;
        assert!((weak.primary.gain - expected).abs() < 0.00001);
        assert!((weak.static_gain - (0.1 - expected)).abs() < 0.00001);
        let off = r.advance(1., 0.2);
        assert!(!off.power);
        assert_eq!(off.static_gain, 0.);
    }
}
#[test]
fn custom_1080_has_no_recording() {
    let mut r = radio(540. / 1060., true);
    r.control(RadioControl::VolumeUp);
    let f = r.advance(0., 1.);
    assert_eq!(f.frequency, 1080.);
    assert_eq!(f.primary.clip, None);
    assert_eq!(f.primary.next_clip, None);
    assert_eq!(f.static_gain, 0.);
    assert!((f.primary.gain - 0.05).abs() < 0.0001);
}
#[test]
fn scaled_schedule_follows_game_time() {
    let mut r = radio(0.06, false);
    let a = r.advance(0., 1.);
    let b = r.advance(5., 1.);
    assert_eq!(a.primary.clip, b.primary.clip);
    assert_eq!(b.primary.offset_seconds, 5.);
    assert!(!b.power);
    r.control(RadioControl::VolumeUp);
    assert_eq!(r.advance(5., 1.).primary.offset_seconds, 5.);
    assert_eq!(r.advance(5., 1.).primary.offset_seconds, 5.);
    assert_eq!(r.advance(6., 1.).primary.offset_seconds, 6.);
}

#[test]
fn source_control_nodes_keep_eligibility_and_repeat_gate() {
    use my_summer_slop::interaction::mechanisms::{ControlAction, MechanismState};
    use my_summer_slop::simulation::{FrameInput, keys};
    use my_summer_slop::vehicle::VehiclePose;
    let mut game = common::reference();
    let audio = AudioReference::from_value(&game.audio, &game).unwrap();
    let volume = audio.radio.controls.volume.as_ref().unwrap().item;
    let tuning = audio.radio.controls.tuning.as_ref().unwrap().item;
    let band = audio.radio.controls.band.as_ref().unwrap().item;
    let mut mechanisms = MechanismState::new(&game).unwrap();
    assert!(mechanisms.contains(volume));
    assert!(mechanisms.scroll_eligible(tuning));
    assert!(mechanisms.primary_switch(band));
    let pose = VehiclePose {
        position: [0.; 3],
        rotation: [0., 0., 0., 1.],
    };
    mechanisms.step(
        volume,
        FrameInput {
            wheel: 1.,
            ..Default::default()
        },
        pose,
        1. / 60.,
    );
    assert!(matches!(
        mechanisms.take_control_action(),
        Some(ControlAction::Radio(RadioControl::VolumeUp))
    ));
    mechanisms.step(
        tuning,
        FrameInput {
            held: keys::USE,
            mouse: [0., -0.01],
            ..Default::default()
        },
        pose,
        1. / 60.,
    );
    assert!(mechanisms.take_control_action().is_none());
    for _ in 0..12 {
        mechanisms.begin_audio_step();
    }
    mechanisms.step(
        tuning,
        FrameInput {
            held: keys::USE,
            mouse: [0., -0.01],
            ..Default::default()
        },
        pose,
        1. / 60.,
    );
    assert!(matches!(
        mechanisms.take_control_action(),
        Some(ControlAction::Radio(RadioControl::TuneUp))
    ));
    mechanisms.step(
        band,
        FrameInput {
            pressed: keys::FILL,
            ..Default::default()
        },
        pose,
        1. / 60.,
    );
    assert!(matches!(
        mechanisms.take_control_action(),
        Some(ControlAction::Radio(RadioControl::Band))
    ));
    game.audio["radio"]["controls"]["volume"]["enabled"] = serde_json::json!(false);
    game.audio["radio"]["controls"]["band"] = serde_json::Value::Null;
    let mut disabled = MechanismState::new(&game).unwrap();
    assert!(!disabled.contains(volume));
    assert!(!disabled.contains(band));
    disabled.step(
        volume,
        FrameInput {
            wheel: 1.,
            ..Default::default()
        },
        pose,
        1. / 60.,
    );
    assert!(disabled.take_control_action().is_none());
}
#[test]
fn schedule_preserves_music_talk_gains_and_decks_until_reset() {
    let game = common::reference();
    let mut a = AudioReference::from_value(&game.audio, &game).unwrap();
    a.radio.initial_tuning = 0.;
    a.radio.stations.truncate(1);
    let s = &mut a.radio.stations[0];
    s.frequency = 88.;
    s.music = vec![30, 31];
    s.music_holidays = vec![0, 1];
    s.music_gains = vec![0., 0.2];
    s.talk = vec![70];
    s.talk_holidays = vec![0];
    s.talk_gains = vec![0.3];
    for c in &mut a.clips {
        c.source_duration_seconds = 1.;
    }
    let mut r = RadioState::new(&a.radio, &a.clips, [10, 15]);
    r.control(RadioControl::VolumeUp);
    let first = r.advance(0., 1.);
    let first_clip = first.primary.clip.unwrap();
    let expected = if first_clip == 30 { 0.1 } else { 0.02 };
    assert!((first.primary.gain - expected).abs() < 0.0001);
    assert_eq!(r.advance(1., 1.).primary.clip, Some(70));
    assert!((r.advance(1., 1.).primary.gain - 0.03).abs() < 0.0001);
    r.calendar(11, 16);
    let second = r.advance(2., 1.).primary.clip.unwrap();
    assert_ne!(first_clip, second, "remaining deck survives holiday end");
    assert_eq!(r.advance(3., 1.).primary.clip, Some(70));
    assert_eq!(
        r.advance(4., 1.).primary.clip,
        Some(30),
        "deck reset uses current local day"
    );
}
#[test]
fn radio_clicks_only_follow_power_and_band_changes() {
    use my_summer_slop::audio::AudioState;
    let game = common::reference();
    let a = AudioReference::from_value(&game.audio, &game).unwrap();
    let click = a.radio.click_binding.unwrap();
    let mut audio = AudioState::new(a, 1);
    audio.radio_control(RadioControl::TuneUp);
    assert_eq!(audio.event_count(), 0);
    audio.radio_control(RadioControl::VolumeUp);
    assert_eq!(audio.pop_event().unwrap().binding, click);
    audio.radio_control(RadioControl::VolumeUp);
    assert_eq!(audio.event_count(), 0);
    audio.radio_control(RadioControl::Band);
    assert_eq!(audio.pop_event().unwrap().binding, click);
    audio.radio_control(RadioControl::VolumeDown);
    assert_eq!(audio.event_count(), 0);
    audio.radio_control(RadioControl::VolumeDown);
    assert_eq!(audio.pop_event().unwrap().binding, click);
}

#[test]
fn radio_source_metadata_is_required_and_linked() {
    let game = common::reference();
    for (field, value) in [
        ("electronics_output", serde_json::json!(-1)),
        ("tuning_repeat_seconds", serde_json::json!(0)),
        (
            "holiday_ranges",
            serde_json::json!([[0, 15, 11, 15], [12, 15, 1, 15]]),
        ),
    ] {
        let mut audio = game.audio.clone();
        audio["radio"][field] = value;
        assert!(AudioReference::from_value(&audio, &game).is_err());
    }
    let mut audio = game.audio.clone();
    audio["radio"]["controls"]["volume"]["item"] = serde_json::json!(9999);
    assert!(AudioReference::from_value(&audio, &game).is_err());
}

#[test]
#[ignore = "requires installed source export"]
fn installed_radio_controls_and_electronics_keep_source_owners() {
    use my_summer_slop::audio::AudioState;
    use my_summer_slop::interaction::mechanisms::{ControlAction, MechanismState};
    use my_summer_slop::reference::ItemId;
    use my_summer_slop::simulation::FrameInput;
    use my_summer_slop::vehicle::VehiclePose;
    let game = my_summer_slop::reference::ReferenceManifest::from_json(
        &std::fs::read("web/assets/reference/manifest.json").unwrap(),
    )
    .unwrap();
    let a = AudioReference::from_value(&game.audio, &game).unwrap();
    assert_eq!(
        a.radio.controls.volume.as_ref().unwrap().source.path_id,
        77810
    );
    assert_eq!(
        a.radio.controls.tuning.as_ref().unwrap().source.path_id,
        70967
    );
    assert_eq!(
        a.radio.controls.band.as_ref().unwrap().source.path_id,
        78052
    );
    assert_eq!(a.radio.electronics_source.path_id, 68491);
    assert_eq!(a.radio.electronics_output, 1.);
    let mut m = MechanismState::new(&game).unwrap();
    assert!(m.contains(ItemId(94)));
    let mut audio = AudioState::new(a, 1);
    let run = my_summer_slop::starting_area::GarageRuntime::new(&game).unwrap();
    audio.bind_game(&game, &run.area, &run.physics);
    audio.fixed_step(&run.area, &run.physics, 1. / 60.);
    m.step(
        ItemId(94),
        FrameInput {
            wheel: 1.,
            ..Default::default()
        },
        VehiclePose {
            position: [0.; 3],
            rotation: [0., 0., 0., 1.],
        },
        1. / 60.,
    );
    let Some(ControlAction::Radio(control)) = m.take_control_action() else {
        panic!("source volume control");
    };
    audio.radio_control(control);
    let click = audio.pop_event().unwrap();
    assert_eq!(click.emitter, audio.reference.radio.emitter);
    let ss = audio
        .reference
        .voices
        .iter()
        .find(|v| v.id == audio.reference.radio.voices[0])
        .unwrap();
    let node = audio
        .reference
        .emitters
        .iter()
        .find(|e| e.id == ss.emitter)
        .unwrap()
        .node;
    let expected = my_summer_slop::interaction::items::node_pose(&game, node)
        .unwrap()
        .position;
    for (actual, expected) in click.position.into_iter().zip(expected) {
        assert!((actual - expected).abs() < 0.0001);
    }
    assert!(audio.radio.advance(0., 1.).power);
    audio.remove_owner(audio.reference.engine.emitter);
    assert!(audio.owner_alive(audio.reference.radio.emitter));
    assert!(audio.radio.advance(5., 1.).power);
}
