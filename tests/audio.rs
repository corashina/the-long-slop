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
