use my_summer_slop::reference::{BUILD_ID, ReferenceManifest};

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
    let car = &manifest.bodies[manifest.vehicle.body.0 as usize - 1];
    assert_eq!(car.linear_damping, 0.005);
    assert_eq!(car.angular_damping, 0.05);
}

#[test]
fn rejects_missing_negative_and_nonfinite_body_damping() {
    for field in ["linear_damping", "angular_damping"] {
        let mut input = fixture();
        input["bodies"][0].as_object_mut().unwrap().remove(field);
        assert!(ReferenceManifest::from_json(&serde_json::to_vec(&input).unwrap()).is_err());
        let mut input = fixture();
        input["bodies"][0][field] = serde_json::json!(-0.01);
        assert!(ReferenceManifest::from_json(&serde_json::to_vec(&input).unwrap()).is_err());
    }
    for value in [f32::NAN, f32::INFINITY] {
        let mut manifest =
            ReferenceManifest::from_json(include_bytes!("common/reference.json")).unwrap();
        manifest.bodies[0].linear_damping = value;
        assert!(manifest.validate().is_err());
        manifest.bodies[0].linear_damping = 0.0;
        manifest.bodies[0].angular_damping = value;
        assert!(manifest.validate().is_err());
    }
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
    if let my_summer_slop::reference::ColliderGeometry::Convex { points } =
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
fn rejects_invalid_seat_hit_nodes_and_empty_use_mask() {
    for nodes in [
        serde_json::json!([]),
        serde_json::json!([999999]),
        serde_json::json!([1, 1]),
    ] {
        let mut input = fixture();
        input["seats"][0]["hit_nodes"] = nodes;
        assert!(ReferenceManifest::from_json(&serde_json::to_vec(&input).unwrap()).is_err());
    }
    let mut input = fixture();
    input["player"]["use_layer_mask"] = serde_json::json!(0);
    assert!(ReferenceManifest::from_json(&serde_json::to_vec(&input).unwrap()).is_err());
    let mut input = fixture();
    input["player"]["get_out_layer_mask"] = serde_json::json!(0);
    assert!(ReferenceManifest::from_json(&serde_json::to_vec(&input).unwrap()).is_err());
    let mut input = fixture();
    input["player"]["exit_capsule_radius"] = serde_json::json!(0);
    assert!(ReferenceManifest::from_json(&serde_json::to_vec(&input).unwrap()).is_err());
}

#[test]
#[ignore = "Requires the local installed-build export"]
fn production_export_matches_installed_contract() {
    let bytes = std::fs::read("web/assets/reference/manifest.json").unwrap();
    let manifest = ReferenceManifest::from_json(&bytes).unwrap();
    fn digest(path: &str) -> String {
        let output = std::process::Command::new("sha256sum")
            .arg(path)
            .output()
            .unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout)
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
            .to_owned()
    }
    assert_eq!(
        manifest.library.manifest_sha256,
        digest("web/assets/library/manifest.json")
    );
    assert_eq!(
        manifest.library.catalog_sha256,
        digest(".cache/reference-library/catalog.json")
    );
    assert_eq!(
        manifest.library.source_manifest_sha256,
        digest(".cache/reference-library/source-manifest.json")
    );
    for asset in &manifest.assets {
        assert_eq!(asset.sha256, digest(&format!("web/assets/{}", asset.path)));
        for dependency in &asset.dependencies {
            assert_eq!(
                dependency.sha256,
                digest(&format!("web/assets/{}", dependency.file))
            );
        }
    }

    assert_eq!(
        manifest.nodes.iter().filter(|n| n.asset.0 == 1).count(),
        434
    );
    assert_eq!(
        manifest.nodes.iter().filter(|n| n.asset.0 == 2).count(),
        697
    );
    assert!((manifest.vehicle.mass_kg - 961.6).abs() < 0.01);
    for (source, mass, linear, angular) in [
        (50613, 961.6, 0.005, 0.05),
        (50394, 7.0, 0.0, 0.05),
        (50395, 10.0, 0.0, 0.05),
        (50411, 4.0, 0.0, 0.05),
        (50393, 25.0, 0.0, 0.05),
        (50513, 20.0, 0.0, 0.05),
        (50540, 20.0, 0.0, 0.05),
    ] {
        let body = manifest
            .bodies
            .iter()
            .find(|body| body.source.path_id == source)
            .unwrap();
        assert!(body.dynamic);
        assert!((body.mass_kg - mass).abs() < 0.001);
        assert_eq!(body.linear_damping, linear);
        assert_eq!(body.angular_damping, angular);
    }
    let spare = &manifest.bodies[7];
    assert_eq!(spare.id.0, 8);
    assert_eq!(spare.source.path_id, 50540);
    for body in manifest.bodies.iter().filter(|body| !body.dynamic) {
        assert_eq!(body.linear_damping, 0.0);
        assert_eq!(body.angular_damping, 0.0);
    }
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
    let driver = manifest.seats.iter().find(|seat| seat.driver).unwrap();
    assert_eq!(driver.source.path_id, 69296);
    let hit_sources: Vec<_> = driver
        .hit_nodes
        .iter()
        .map(|id| {
            manifest
                .nodes
                .iter()
                .find(|node| node.id == *id)
                .unwrap()
                .source
                .path_id
        })
        .collect();
    assert_eq!(hit_sources, [33209, 31216, 39134]);
    assert_eq!(manifest.player.use_layer_mask, 12747265);
    assert_eq!(manifest.player.use_layer_mask & (1 << 13), 0);
    assert_eq!(manifest.player.get_out_layer_mask, 10125313);
    assert_eq!(manifest.player.get_out_layer_mask & (1 << 13), 0);
    assert_eq!(manifest.player.exit_capsule_half_height, 0.568);
    assert_eq!(manifest.player.exit_capsule_radius, 0.25);
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
        if let my_summer_slop::reference::ColliderGeometry::Convex { points } = &collider.geometry {
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

#[test]
fn requires_current_library_provenance() {
    let mut input = fixture();
    input.as_object_mut().unwrap().remove("library");
    assert!(ReferenceManifest::from_json(&serde_json::to_vec(&input).unwrap()).is_err());
    for value in [
        serde_json::json!({"schema_version": 0, "manifest_sha256": "a".repeat(64), "catalog_sha256": "a".repeat(64), "source_manifest_sha256": "a".repeat(64)}),
        serde_json::json!({"schema_version": 1, "manifest_sha256": "z".repeat(64), "catalog_sha256": "a".repeat(64), "source_manifest_sha256": "a".repeat(64)}),
    ] {
        input["library"] = value;
        assert!(ReferenceManifest::from_json(&serde_json::to_vec(&input).unwrap()).is_err());
    }
}
