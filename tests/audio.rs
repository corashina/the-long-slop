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
