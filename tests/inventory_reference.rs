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
