mod common;
use my_summer_slop::reference::ReferenceManifest;
use serde_json::{Value, json};

#[test]
fn checked_inventory_contract() {
    let r = common::reference();
    let i = &r.inventory;
    assert_eq!(
        (i.source.file.as_str(), i.source.path_id),
        ("level1", 80125)
    );
    assert_eq!(i.slot_count, 4);
    assert!(i.initial_slots.iter().all(Option::is_none));
    assert_eq!(i.selected_slot, 0);
    assert_eq!(i.swap_gain, 5.0);
    assert_eq!(i.starter_groups, [false; 3]);
    assert_eq!(i.equip_hold_seconds, 0.25);
    assert_eq!(i.hand_interpolation_factor, 20.0);
    assert_eq!(i.hand_interpolation_seconds, 1.0);
    assert_eq!(
        (
            i.right_hand_anchor.file.as_str(),
            i.right_hand_anchor.path_id
        ),
        ("level1", 23118)
    );
    assert_eq!(
        i.inputs
            .iter()
            .map(|v| (v.action.as_str(), v.index, v.key.as_str()))
            .collect::<Vec<_>>(),
        [
            ("equiprecenter", 19, "Tab"),
            ("removeSunglasses", 30, "X"),
            ("inventory1", 31, "Alpha1"),
            ("inventory2", 32, "Alpha2"),
            ("inventory3", 33, "Alpha3"),
            ("inventory4", 34, "Alpha4"),
            ("smoke", 29, "CapsLock"),
        ]
    );
}

#[test]
fn rejects_changed_startup_anchor_controls_and_missing_contract() {
    let fixture: Value = serde_json::from_slice(include_bytes!("common/reference.json")).unwrap();
    let mutations = [
        ("/inventory/slot_count", json!(3)),
        ("/inventory/initial_slots", json!([null, null, null])),
        (
            "/inventory/initial_slots/0",
            json!({"file":"level1","path_id":1}),
        ),
        ("/inventory/selected_slot", json!(1)),
        ("/inventory/swap_gain", json!(4.)),
        ("/inventory/starter_groups/0", json!(true)),
        ("/inventory/equip_hold_seconds", json!(0.3)),
        ("/inventory/hand_interpolation_factor", json!(19.)),
        ("/inventory/hand_interpolation_seconds", json!(2.)),
        ("/inventory/source/path_id", json!(1)),
        ("/inventory/right_hand_anchor/file", json!("other")),
        ("/inventory/right_hand_anchor/path_id", json!(23119)),
        ("/inventory/inputs/6/action", json!("hum")),
        ("/inventory/inputs/6/index", json!(28)),
        ("/inventory/inputs/6/key", json!("H")),
        ("/inventory/evidence/assembly_sha256", json!("0".repeat(64))),
        ("/inventory/evidence/methods", json!([])),
    ];
    for (path, value) in mutations {
        let mut broken = fixture.clone();
        *broken.pointer_mut(path).unwrap() = value;
        let bytes = serde_json::to_vec(&broken).unwrap();
        assert!(ReferenceManifest::from_json(&bytes).is_err(), "{path}");
        assert!(
            ReferenceManifest::from_runtime_json(&bytes).is_err(),
            "{path}"
        );
    }
    for field in fixture["inventory"].as_object().unwrap().keys() {
        let mut broken = fixture.clone();
        broken["inventory"].as_object_mut().unwrap().remove(field);
        assert!(
            ReferenceManifest::from_runtime_json(&serde_json::to_vec(&broken).unwrap()).is_err(),
            "{field}"
        );
    }
    let mut old = fixture;
    old.as_object_mut().unwrap().remove("inventory");
    assert!(ReferenceManifest::from_runtime_json(&serde_json::to_vec(&old).unwrap()).is_err());
}

#[test]
fn original_parent_inventory_clothing_contract() {
    let r = common::reference();
    assert_eq!(r.clothing.wearable_type_count, 5);
    assert_eq!(
        r.clothing
            .characters
            .iter()
            .map(|c| c.source.path_id)
            .collect::<Vec<_>>(),
        [71215, 67763]
    );
    assert_eq!(
        r.acquisition
            .stations
            .iter()
            .map(|s| s.misc4.len())
            .collect::<Vec<_>>(),
        [18, 6, 38]
    );
    assert_eq!(r.acquisition.pack_item_id, 74);
    assert_eq!(r.acquisition.cigarette_item_id, 230);
    assert_eq!(r.acquisition.holiday_pumpkin.file, "level1");
    assert_eq!(r.acquisition.holiday_pumpkin.path_id, 82);
}
#[test]
fn original_parent_rejects_broken_skin_or_wearable_anchor_and_missing_owners() {
    let fixture: Value = serde_json::from_slice(include_bytes!("common/reference.json")).unwrap();
    for path in [
        "/clothing/characters/0/skins/0/joints/0",
        "/clothing/characters/0/skins/0/node",
        "/clothing/anchors/0/path_id",
        "/clothing/wearables/0/selectors/0/variants/0/nodes/0",
        "/clothing/wearables/0/selectors/0/variants/2/profile",
    ] {
        let mut broken = fixture.clone();
        *broken.pointer_mut(path).unwrap() = json!(u32::MAX);
        let bytes = serde_json::to_vec(&broken).unwrap();
        assert!(ReferenceManifest::from_json(&bytes).is_err(), "{path}");
        assert!(
            ReferenceManifest::from_runtime_json(&bytes).is_err(),
            "{path}"
        );
    }
    for owner in ["inventory", "clothing", "acquisition"] {
        let mut broken = fixture.clone();
        broken.as_object_mut().unwrap().remove(owner);
        let bytes = serde_json::to_vec(&broken).unwrap();
        assert!(
            ReferenceManifest::from_json(&bytes).is_err(),
            "missing {owner}"
        );
        assert!(
            ReferenceManifest::from_runtime_json(&bytes).is_err(),
            "missing {owner}"
        );
    }
}
