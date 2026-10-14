mod common;
use my_summer_slop::reference::ReferenceManifest;
use serde_json::{Value, json};

#[test]
fn checked_wearables_remain_local_templates() {
    let m = common::reference();
    m.clothing.validate(&m).unwrap();
    assert_eq!(m.clothing.wearable_type_count, 5);
    assert_eq!(m.clothing.wearables.len(), 6);
    assert_eq!(m.clothing.glasses_profiles.len(), 6);
    assert_eq!(m.clothing.player_root.path_id, 39843);
    assert_eq!(m.item_prototypes.len(), 11);
    assert_eq!(m.expected_instances(), 4);
    assert_eq!(
        m.clothing.wearables[0]
            .selectors
            .iter()
            .flat_map(|s| &s.variants)
            .filter(|v| v.lens.is_some())
            .count(),
        19
    );
    let shadows: Vec<_> = m
        .item_prototypes
        .iter()
        .flat_map(|p| &p.renderers)
        .filter(|r| r.state.cast_shadows == 3)
        .collect();
    assert_eq!(shadows.len(), 5);
    for renderer in shadows {
        renderer.state.validate_template().unwrap();
        assert!(renderer.state.validate().is_err());
        let mut changed = renderer.state.clone();
        changed.cast_shadows = 1;
        assert!(changed.validate_template().is_err());
        changed = renderer.state.clone();
        changed.source.path_id = 1;
        assert!(changed.validate_template().is_err());
    }
}

#[test]
fn strict_and_runtime_reject_broken_wearable_contracts() {
    let original: Value = serde_json::from_slice(include_bytes!("common/reference.json")).unwrap();
    for (path, value) in [
        ("/clothing/anchors/0/path_id", json!(1)),
        ("/clothing/player_root/path_id", json!(80125)),
        ("/clothing/wearables/0/prototype/path_id", json!(75443)),
        ("/clothing/wearables/0/renderers", json!([999999])),
        ("/clothing/glasses_profiles", json!([])),
        ("/clothing/anchor_nodes/1/parent", json!(null)),
        (
            "/clothing/wearables/0/selectors/0/variants/2/lens",
            json!(null),
        ),
        (
            "/clothing/wearables/0/selectors/2/variants/0/nodes",
            json!([999999]),
        ),
        (
            "/clothing/wearables/0/selectors/1/variants/0/renderers/0/slot",
            json!(99),
        ),
        (
            "/clothing/wearables/0/selectors/1/variants/0/material/path_id",
            json!(1),
        ),
        (
            "/clothing/wearables/0/selectors/2/variants/0/profile",
            json!(5),
        ),
        (
            "/clothing/glasses_profiles/0/effects/0/kind",
            json!("unknown"),
        ),
        (
            "/clothing/glasses_profiles/0/effects/0/parameters/ldrLut/texture",
            json!(999999),
        ),
        (
            "/clothing/glasses_profiles/0/effects/0/parameters/contrast/override_state",
            json!(true),
        ),
        ("/item_prototypes/10/consumable/min_particle", json!(0)),
        ("/item_prototypes/10/consumable/continuous", json!(false)),
        ("/item_prototypes/10/consumable/destroy_on_end", json!(true)),
        ("/item_prototypes/10/consumable/parts", json!([999999])),
        (
            "/item_prototypes/10/consumable/ember/particle/simulation_space",
            json!(2),
        ),
        (
            "/item_prototypes/10/consumable/ember/particle/module_flags/NoiseModule",
            json!(true),
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
}

#[test]
fn local_cigarette_effects_cannot_enter_active_tables() {
    let original: Value = serde_json::from_slice(include_bytes!("common/reference.json")).unwrap();
    for (target, source) in [
        (
            "/lighting/lights",
            "/item_prototypes/10/consumable/ember/light",
        ),
        (
            "/weather/particles",
            "/item_prototypes/10/consumable/ember/particle",
        ),
    ] {
        let mut m = original.clone();
        let local = m.pointer(source).unwrap().clone();
        m.pointer_mut(target)
            .unwrap()
            .as_array_mut()
            .unwrap()
            .push(local);
        let bytes = serde_json::to_vec(&m).unwrap();
        assert!(ReferenceManifest::from_json(&bytes).is_err());
        assert!(ReferenceManifest::from_runtime_json(&bytes).is_err());
    }
}
