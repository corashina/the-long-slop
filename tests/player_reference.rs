mod common;
use my_summer_slop::reference::{AssetUsage, ReferenceManifest};
use serde_json::{Value, json};

#[test]
fn inactive_player_has_intact_models_and_source_pose_links() {
    let m = common::reference();
    assert_eq!(m.expected_instances(), 4);
    assert_eq!(
        m.assets
            .iter()
            .filter(|a| a.usage == AssetUsage::PlayerPrototype)
            .count(),
        1
    );
    assert_eq!(
        m.clothing
            .characters
            .iter()
            .map(|c| c.skins.len())
            .collect::<Vec<_>>(),
        vec![2, 16]
    );
    assert_eq!(m.clothing.pose.ik_chains.len(), 5);
    assert_eq!(m.clothing.pose.follow.nodes.len(), 66);
    assert!(!m.clothing.characters[0].segments[0].visible());
    assert!(m.clothing.characters[0].segments[4].visible());
    assert!(m.clothing.characters[1].segments[4].visible());
    assert_ne!(
        m.clothing.characters[0].skins[0].joints,
        m.clothing.characters[1].skins[0].joints
    );
}

#[test]
fn malformed_player_contracts_fail_strict_and_runtime_validation() {
    let original: Value = serde_json::from_slice(include_bytes!("common/reference.json")).unwrap();
    for (pointer, value) in [
        ("/clothing/selected_character", json!(1)),
        ("/clothing/pose/character_meshes/0", json!(3532)),
        ("/clothing/pose/animation_scalars/headBackM", json!(2)),
        ("/clothing/characters/0/root", json!(1)),
        ("/clothing/characters/0/renderers/0/node", json!(1)),
        (
            "/clothing/characters/0/renderers/0/materials/0/path_id",
            json!(57),
        ),
        (
            "/clothing/characters/0/segments/0/renderers/0/slot",
            json!(99),
        ),
        (
            "/clothing/characters/0/segments/4/default_enabled",
            json!(true),
        ),
        ("/clothing/characters/0/skins/0/joints/0", json!(1)),
        ("/clothing/characters/0/skins/0/skeleton", json!(1)),
        (
            "/clothing/characters/0/skins/0/inverse_bind/index",
            json!(999999),
        ),
        (
            "/clothing/characters/0/skins/0/inverse_bind/count",
            json!(52),
        ),
        (
            "/clothing/characters/0/skins/0/inverse_bind/shape",
            json!("VEC4"),
        ),
        (
            "/clothing/characters/0/skins/0/attributes/0/weights/count",
            json!(1),
        ),
        ("/clothing/pose/targets/LHTarget", json!(1)),
        ("/clothing/pose/runtime_targets/nearestHand", json!(1)),
        ("/clothing/pose/walk_paths/0/0", json!(1)),
        ("/clothing/pose/ik_chains/0/bones/1", json!(1)),
        ("/clothing/pose/ik_chains/0/pole", Value::Null),
        ("/clothing/pose/follow/targets/0", json!(1)),
        ("/clothing/pose/follow/inverse", json!(true)),
        ("/clothing/pose/follow/nodes", json!([])),
        (
            "/clothing/pose/anchors/RH/transform/rotation",
            json!([0, 0, 0, 0]),
        ),
    ] {
        let mut bad = original.clone();
        *bad.pointer_mut(pointer).unwrap() = value;
        let bytes = serde_json::to_vec(&bad).unwrap();
        assert!(
            ReferenceManifest::from_json(&bytes).is_err(),
            "strict {pointer}"
        );
        assert!(
            ReferenceManifest::from_runtime_json(&bytes).is_err(),
            "runtime {pointer}"
        );
    }
    for field in ["characters", "pose", "selected_character"] {
        let mut bad = original.clone();
        bad["clothing"].as_object_mut().unwrap().remove(field);
        assert!(ReferenceManifest::from_runtime_json(&serde_json::to_vec(&bad).unwrap()).is_err());
    }
}

#[test]
fn authored_ik_hierarchy_and_nonzero_segments_are_required() {
    for change in 0..3 {
        let mut m = common::reference();
        let bone = m.clothing.pose.ik_chains[0].bones[1];
        let n = m.nodes.iter_mut().find(|n| n.id == bone).unwrap();
        match change {
            0 => n.parent = None,
            1 => n.transform.translation = [0.; 3],
            _ => n.transform.translation[0] = f32::NAN,
        }
        assert!(m.validate().is_err());
        assert!(m.clothing.pose.validate(&m).is_err() || change == 2);
    }
}

#[test]
fn extra_player_prototype_is_rejected() {
    let mut m = common::reference();
    let mut asset = m
        .assets
        .iter()
        .find(|a| a.usage == AssetUsage::PlayerPrototype)
        .unwrap()
        .clone();
    asset.id = my_summer_slop::reference::AssetId(9999);
    m.assets.push(asset);
    assert!(m.validate().is_err());
}
