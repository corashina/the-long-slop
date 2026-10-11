mod common;
use my_summer_slop::interaction::feedback::FeedbackContext;
use my_summer_slop::reference::ReferenceManifest;

#[test]
fn switch_feedback_contract_requires_source_sprite_and_tank_guard() {
    assert_eq!(FeedbackContext::LightSwitch as u32, 7);
    let original: serde_json::Value =
        serde_json::from_slice(include_bytes!("common/reference.json")).unwrap();
    for missing in ["switch_icon", "no_drink"] {
        let mut json = original.clone();
        if missing == "switch_icon" {
            json["lighting"].as_object_mut().unwrap().remove(missing);
        } else {
            json["tanks"][0].as_object_mut().unwrap().remove(missing);
        }
        assert!(ReferenceManifest::from_json(&serde_json::to_vec(&json).unwrap()).is_err());
    }
    let mut r = common::reference();
    r.lighting.switch_icon.source.path_id = 4593;
    assert!(r.validate().is_err());
}
