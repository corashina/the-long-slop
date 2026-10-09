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
    assert_eq!(manifest.hinges.len(), 7);
    assert!(manifest.openings.len() >= 8);
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
