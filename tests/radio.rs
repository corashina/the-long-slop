mod common;
use my_summer_slop::reference::AudioReference;

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
