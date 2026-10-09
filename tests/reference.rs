use desert_drive::reference::{BUILD_ID, ReferenceManifest};

fn fixture() -> serde_json::Value {
    serde_json::from_slice(include_bytes!("common/reference.json")).unwrap()
}

#[test]
fn accepts_recovered_scalars_with_synthetic_geometry() {
    let manifest = ReferenceManifest::from_json(include_bytes!("common/reference.json")).unwrap();
    assert_eq!(manifest.build_id, BUILD_ID);
    assert_eq!(manifest.vehicle.serialized_mass_kg, 717.0);
    assert_eq!(manifest.engine.max_torque_nm, 87.0);
    assert_eq!(manifest.tanks[0].capacity_liters, 39.0);
    assert_eq!(manifest.tanks[0].quantities_liters[0], 0.5);
    assert_eq!(manifest.player.mass_kg, 72.0);
    assert_eq!(manifest.player.camera_culling_mask, 0x14db8f33);
}

#[test]
fn fixture_preserves_tunable_faucet_metadata() {
    let input = fixture();
    let faucet = input["openings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|opening| opening["source"]["path_id"] == 64723)
        .expect("source faucet metadata");
    assert_eq!(faucet["can_toggle"], false);
    assert_eq!(faucet["can_tune"], true);
    assert_eq!(faucet["valve"], 0.0);
    assert!((faucet["tune_rate"].as_f64().unwrap() - 0.525).abs() < 0.000001);
    assert!(ReferenceManifest::from_json(&serde_json::to_vec(&input).unwrap()).is_ok());
    let mut invalid = input.clone();
    invalid["openings"][0]["valve"] = serde_json::json!(1.5);
    assert!(ReferenceManifest::from_json(&serde_json::to_vec(&invalid).unwrap()).is_err());
    let mut invalid = input;
    invalid["openings"][0]["visuals"]["cap_node"] = serde_json::json!(999999);
    assert!(ReferenceManifest::from_json(&serde_json::to_vec(&invalid).unwrap()).is_err());
}

#[test]
fn rejects_mismatch_unknown_fields_duplicate_ids_and_missing_bindings() {
    let cases = [
        ("build_id", serde_json::json!(0)),
        ("obsolete", serde_json::json!(true)),
    ];
    for (field, value) in cases {
        let mut input = fixture();
        input[field] = value;
        assert!(ReferenceManifest::from_json(&serde_json::to_vec(&input).unwrap()).is_err());
    }
    let mut input = fixture();
    let duplicate = input["items"][0].clone();
    input["items"].as_array_mut().unwrap().push(duplicate);
    assert!(ReferenceManifest::from_json(&serde_json::to_vec(&input).unwrap()).is_err());
    let mut input = fixture();
    input["colliders"][0]["body"] = serde_json::json!(99);
    assert!(ReferenceManifest::from_json(&serde_json::to_vec(&input).unwrap()).is_err());
    let mut input = fixture();
    input["wheel_slots"][1]["rim"] = input["wheel_slots"][0]["rim"].clone();
    assert!(ReferenceManifest::from_json(&serde_json::to_vec(&input).unwrap()).is_err());
    let mut input = fixture();
    input["wheel_slots"][1]["tire"] = input["wheel_slots"][0]["tire"].clone();
    assert!(ReferenceManifest::from_json(&serde_json::to_vec(&input).unwrap()).is_err());
}

#[test]
fn rejects_invalid_geometry_quantities_and_hierarchy_cycles() {
    let mut manifest =
        ReferenceManifest::from_json(include_bytes!("common/reference.json")).unwrap();
    if let desert_drive::reference::ColliderGeometry::Convex { points } =
        &mut manifest.colliders[0].geometry
    {
        points[0][0] = f32::NAN;
    }
    assert!(manifest.validate().is_err());
    let mut input = fixture();
    input["tanks"][0]["quantities_liters"][0] = serde_json::json!(40);
    assert!(ReferenceManifest::from_json(&serde_json::to_vec(&input).unwrap()).is_err());
    let mut input = fixture();
    input["nodes"][0]["parent"] = serde_json::json!(2);
    input["nodes"][1]["parent"] = serde_json::json!(1);
    assert!(ReferenceManifest::from_json(&serde_json::to_vec(&input).unwrap()).is_err());
}

#[test]
fn rejects_incomplete_transmission_before_startup() {
    let mut input = fixture();
    input["vehicle"]["gear_ratios"] = serde_json::json!([-3.87, 3.75, 2.3, 1.49, 1.0]);
    assert!(ReferenceManifest::from_json(&serde_json::to_vec(&input).unwrap()).is_err());
    let mut input = fixture();
    input["vehicle"]["gear_free_run"] = serde_json::json!([]);
    assert!(ReferenceManifest::from_json(&serde_json::to_vec(&input).unwrap()).is_err());
}

#[test]
#[ignore = "Requires the local installed-build export"]
fn production_export_matches_installed_contract() {
    let bytes = std::fs::read("web/assets/reference/manifest.json").unwrap();
    let manifest = ReferenceManifest::from_json(&bytes).unwrap();
    assert_eq!(
        manifest.nodes.iter().filter(|n| n.asset.0 == 1).count(),
        434
    );
    assert_eq!(
        manifest.nodes.iter().filter(|n| n.asset.0 == 2).count(),
        697
    );
    assert!((manifest.vehicle.mass_kg - 961.6).abs() < 0.01);
    assert_eq!(manifest.wheel_slots.len(), 4);
    assert_eq!(manifest.hinges.len(), 9);
    for (usable, pivot, collider_node) in [(64783, 21482, 21566), (64777, 21464, 21548)] {
        let hinge = manifest
            .hinges
            .iter()
            .find(|hinge| hinge.source.path_id == usable)
            .expect("source garage leaf hinge");
        let body = manifest
            .bodies
            .iter()
            .find(|body| body.id == hinge.body)
            .unwrap();
        assert_eq!(body.source.path_id, pivot);
        assert_ne!(body.node, manifest.area.home_root);
        let node = manifest
            .nodes
            .iter()
            .find(|node| node.source.path_id == collider_node)
            .unwrap();
        assert!(
            manifest
                .colliders
                .iter()
                .any(|collider| collider.node == node.id && collider.body == hinge.body)
        );
    }
    assert!(manifest.openings.len() >= 8);
    assert_eq!(manifest.player.camera_culling_mask & (1 << 18), 0);
    let gltf: serde_json::Value =
        serde_json::from_slice(&std::fs::read("web/assets/reference/lada.gltf").unwrap()).unwrap();
    let helper = gltf["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["extras"]["reference_id"]["path_id"] == 37124)
        .unwrap();
    assert_eq!(helper["extras"]["layer"], 18);
    assert_eq!(helper["extras"]["renderer_enabled"], true);
    assert!(helper.get("mesh").is_some());
    let faucet = manifest
        .openings
        .iter()
        .find(|opening| opening.source.path_id == 64723)
        .unwrap();
    assert!(faucet.can_tune && !faucet.can_toggle);
    assert_eq!(faucet.valve, 0.0);
    assert!((faucet.tune_rate - 0.525).abs() < 0.000001);
    assert_eq!(
        manifest.nodes[faucet.visuals.cap_node.unwrap().0 as usize - 1]
            .source
            .path_id,
        30290
    );
    assert_eq!(faucet.visuals.open_euler_degrees, [0., 0., -405.]);
    for opening in &manifest.openings {
        let visual = &opening.visuals;
        if let (Some(cap), Some(closed), Some(open)) =
            (visual.cap_node, visual.closed_node, visual.open_node)
        {
            assert_eq!(
                manifest.nodes[cap.0 as usize - 1].parent,
                manifest.nodes[closed.0 as usize - 1].parent
            );
            assert_eq!(
                manifest.nodes[cap.0 as usize - 1].parent,
                manifest.nodes[open.0 as usize - 1].parent
            );
        }
    }
    for collider in &manifest.colliders {
        if let desert_drive::reference::ColliderGeometry::Convex { points } = &collider.geometry {
            let points: Vec<_> = points
                .iter()
                .copied()
                .map(rapier3d::prelude::Vector::from_array)
                .collect();
            assert!(
                rapier3d::prelude::ColliderBuilder::convex_hull(&points).is_some(),
                "Invalid source collider {:?}",
                collider.source
            );
        }
    }
}
