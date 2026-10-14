mod common;
use my_summer_slop::reference::ReferenceManifest;
use serde_json::{Value, json};

#[test]
fn complete_templates_keep_source_owners_and_local_physics() {
    let m = common::reference();
    assert_eq!(
        m.item_prototypes[..5]
            .iter()
            .map(|p| p.id.0.path_id)
            .collect::<Vec<_>>(),
        [75443, 51722, 61587, 81642, 81645]
    );
    assert_eq!(
        m.item_prototypes[..5]
            .iter()
            .map(|p| p.pickups.len())
            .collect::<Vec<_>>(),
        [1, 1, 1, 56, 14]
    );
    assert_eq!(
        m.item_prototypes[..5]
            .iter()
            .map(|p| p.colliders.len())
            .collect::<Vec<_>>(),
        [1, 2, 2, 58, 15]
    );
    assert_eq!(
        m.item_prototypes[3]
            .pickups
            .iter()
            .filter(|p| !p.pickup.can_inventory)
            .map(|p| p.pickup.source.path_id)
            .collect::<Vec<_>>(),
        [135313, 140775]
    );
    assert_eq!(
        m.item_prototypes[4]
            .pickups
            .iter()
            .filter(|p| !p.pickup.can_inventory)
            .map(|p| p.pickup.source.path_id)
            .collect::<Vec<_>>(),
        [135314]
    );
    assert_eq!(m.expected_instances(), 4);
    for p in &m.item_prototypes {
        p.validate(&m).unwrap();
    }
}

#[test]
fn strict_and_runtime_reject_missing_contract_and_broken_owners() {
    let original: Value = serde_json::from_slice(include_bytes!("common/reference.json")).unwrap();
    for (path, value) in [
        ("/item_prototypes/0/root", json!(1)),
        ("/item_prototypes/0/pickups/0/body", json!(999999)),
        ("/item_prototypes/3/colliders/0/body", json!(2)),
        (
            "/item_prototypes/0/pickups/0/pickup/inventory_icon/path_id",
            json!(1),
        ),
        ("/item_prototypes/1/behavior/clips/0/path_id", json!(1)),
        ("/item_prototypes/0/variants/0/nodes", json!([])),
        ("/item_prototypes/3/pickups", json!([])),
        (
            "/item_prototypes/3/pickups/10/pickup/can_inventory",
            json!(true),
        ),
        ("/item_prototypes/0/bodies/0/parent_body", json!(1)),
        ("/item_prototypes/0/renderers/0/node", json!(1)),
        ("/item_prototypes/0/renderers/0/materials", json!([])),
        (
            "/item_prototypes/2/behavior/contact_clips/4/0/path_id",
            json!(1),
        ),
        (
            "/item_prototypes/0/pickups/0/pickup/disable_when_stored",
            json!({"file":"level1","path_id":21400}),
        ),
    ] {
        let mut m = original.clone();
        *m.pointer_mut(path).unwrap() = value;
        let bytes = serde_json::to_vec(&m).unwrap();
        assert!(ReferenceManifest::from_json(&bytes).is_err(), "{path}");
        assert!(
            ReferenceManifest::from_runtime_json(&bytes).is_err(),
            "{path}"
        );
    }
    for field in ["bodies", "colliders", "pickups"] {
        let mut duplicate = original.clone();
        let first = duplicate["item_prototypes"][0][field][0].clone();
        duplicate["item_prototypes"][0][field]
            .as_array_mut()
            .unwrap()
            .push(first);
        let bytes = serde_json::to_vec(&duplicate).unwrap();
        assert!(
            ReferenceManifest::from_json(&bytes).is_err(),
            "duplicate {field}"
        );
        assert!(
            ReferenceManifest::from_runtime_json(&bytes).is_err(),
            "duplicate {field}"
        );
    }
    let mut old = original;
    old.as_object_mut().unwrap().remove("item_prototypes");
    let bytes = serde_json::to_vec(&old).unwrap();
    assert!(ReferenceManifest::from_json(&bytes).is_err());
    assert!(ReferenceManifest::from_runtime_json(&bytes).is_err());
}

fn reject_prototype_manifest(m: &Value) {
    let bytes = serde_json::to_vec(m).unwrap();
    assert!(
        ReferenceManifest::from_json(&bytes).is_err(),
        "strict rejection"
    );
    assert!(
        ReferenceManifest::from_runtime_json(&bytes).is_err(),
        "runtime rejection"
    );
}

#[test]
fn strict_and_runtime_reject_appended_variant_nodes() {
    let original: Value = serde_json::from_slice(include_bytes!("common/reference.json")).unwrap();
    for node in [
        json!(999999),
        json!(1),
        original["item_prototypes"][0]["variants"][0]["nodes"][0].clone(),
    ] {
        let mut m = original.clone();
        m["item_prototypes"][0]["variants"][0]["nodes"]
            .as_array_mut()
            .unwrap()
            .push(node);
        reject_prototype_manifest(&m);
    }
}

#[test]
fn strict_and_runtime_reject_active_prototype_collider() {
    let original: Value = serde_json::from_slice(include_bytes!("common/reference.json")).unwrap();
    let mut m = original.clone();
    let mut collider = m["item_prototypes"][0]["colliders"][0].clone();
    collider["id"] = json!(
        m["colliders"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["id"].as_u64().unwrap())
            .max()
            .unwrap()
            + 1
    );
    collider["body"] = m["bodies"][0]["id"].clone();
    let mut binding = m["action_bindings"][0].clone();
    binding["collider"] = collider["id"].clone();
    for field in ["use_owner", "pickup_owner", "removal_owner", "push_owner"] {
        binding[field] = Value::Null;
    }
    m["colliders"].as_array_mut().unwrap().push(collider);
    m["action_bindings"].as_array_mut().unwrap().push(binding);
    reject_prototype_manifest(&m);
}
