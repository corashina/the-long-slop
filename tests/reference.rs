use my_summer_slop::reference::ReferenceManifest;

fn fixture() -> serde_json::Value {
    serde_json::from_slice(include_bytes!("common/reference.json")).unwrap()
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
    let tank = input["tanks"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|tank| tank["source"]["path_id"] == 77838)
        .unwrap();
    tank["quantities_liters"][0] = serde_json::json!(40);
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
    fn digest(path: impl AsRef<std::path::Path>) -> String {
        let output = std::process::Command::new("sha256sum")
            .arg(path.as_ref())
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
    let library_root = std::env::var_os("REFERENCE_LIBRARY_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("web/assets/library"));
    let library: serde_json::Value =
        serde_json::from_slice(&std::fs::read(library_root.join("manifest.json")).unwrap())
            .unwrap();
    let records = library_root.join(library["records_root"].as_str().unwrap());
    let stage = records.parent().unwrap();
    assert_eq!(
        manifest.library.manifest_sha256,
        digest(library_root.join("manifest.json"))
    );
    assert_eq!(
        manifest.library.catalog_sha256,
        digest(stage.join("catalog.json"))
    );
    assert_eq!(
        manifest.library.source_manifest_sha256,
        digest(stage.join("source-manifest.json"))
    );
    assert_eq!(manifest.world.generators.len(), 4);
    assert_eq!(manifest.world.buildings.types.len(), 11);
    assert_eq!(manifest.world.catalog.items.len(), 402);
    assert_eq!(manifest.world.catalog.categories.len(), 90);
    for prototype in manifest
        .world
        .prototypes
        .iter()
        .filter(|p| p.role == my_summer_slop::reference::WorldPrototypeRole::Road)
    {
        assert_eq!(prototype.controls.len(), 10);
        assert_eq!(prototype.skins[0].positions.len(), 88);
        assert_eq!(prototype.skins[0].bones.len(), 11);
        assert_eq!(prototype.skins[0].bind_poses.len(), 11);
    }
    for asset in &manifest.assets {
        assert_eq!(asset.sha256, digest(&format!("web/assets/{}", asset.path)));
        for dependency in &asset.dependencies {
            assert_eq!(
                dependency.sha256,
                digest(&format!("web/assets/{}", dependency.file))
            );
        }
    }

    assert_eq!(manifest.vehicles[0].keyboard_steering_rate, 1.25);
    assert_eq!(
        manifest.nodes.iter().filter(|n| n.asset.0 == 1).count(),
        434
    );
    assert_eq!(
        manifest.nodes.iter().filter(|n| n.asset.0 == 2).count(),
        697
    );
    assert_eq!(
        manifest.nodes.iter().filter(|n| n.asset.0 == 3).count(),
        730
    );
    assert!((manifest.vehicles[0].mass_kg - 961.6).abs() < 0.01);
    assert!((manifest.vehicles[1].mass_kg - 811.6).abs() < 0.01);
    assert!((manifest.vehicles[1].engine.max_torque_nm - 70.3).abs() < 0.001);
    assert_eq!(manifest.vehicles[1].final_drive_ratio, 4.44);
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
    for body in manifest.bodies.iter().filter(|body| !body.dynamic) {
        assert_eq!(body.linear_damping, 0.0);
        assert_eq!(body.angular_damping, 0.0);
    }
    assert_eq!(manifest.wheel_slots.len(), 8);
    for (usable, pivot, collider_node) in [(64783, 21482, 21566), (64777, 21464, 21548)] {
        let hinge = manifest
            .mechanisms
            .iter()
            .find(|hinge| hinge.source.path_id == usable)
            .expect("source garage leaf hinge");
        let body = manifest
            .bodies
            .iter()
            .find(|body| hinge.driven_nodes.contains(&body.node))
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
                .any(|collider| collider.node == node.id && collider.body == body.id)
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
    assert_eq!(manifest.player.action_masks.use_mask, 12747265);
    assert_eq!(manifest.player.action_masks.pickup_mask, 8388609);
    assert_eq!(manifest.player.action_masks.attach_mask, 8552449);
    assert_eq!(manifest.player.action_masks.push_mask, 163841);
    for opening in &manifest.openings {
        let tank = manifest
            .items
            .iter()
            .find(|item| item.id == opening.tank)
            .unwrap();
        let body = manifest
            .bodies
            .iter()
            .find(|body| body.id == tank.body)
            .unwrap();
        if !body.dynamic || manifest.vehicles.iter().any(|v| v.body == body.id) {
            continue;
        }
        let bindings: Vec<_> = manifest
            .action_bindings
            .iter()
            .filter(|binding| binding.use_owner == Some(opening.cap))
            .collect();
        assert!(
            !bindings.is_empty(),
            "cap must have a source action collider"
        );
        assert!(
            bindings
                .iter()
                .all(|binding| binding.pickup_owner == Some(opening.tank)),
            "cap must resolve its source container"
        );
    }
    assert_eq!(manifest.player.action_masks.use_mask & (1 << 13), 0);
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
    let home: serde_json::Value =
        serde_json::from_slice(&std::fs::read("web/assets/reference/home.gltf").unwrap()).unwrap();
    let hole = home["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| {
            node["extras"]["reference_id"]["file"] == "level1"
                && node["extras"]["reference_id"]["path_id"] == 35014
        })
        .expect("source terrain hole node");
    assert_eq!(hole["extras"]["renderer_enabled"], false);
    assert!(hole.get("mesh").is_none());
    for source in [38712, 23613, 28128] {
        let marker = home["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|node| {
                node["extras"]["reference_id"]["file"] == "level1"
                    && node["extras"]["reference_id"]["path_id"] == source
            })
            .expect("source terrain alignment marker");
        assert_eq!(marker["extras"]["active"], false);
        assert_eq!(marker["extras"]["renderer_enabled"], false);
        assert!(marker.get("mesh").is_none());
        let node = manifest
            .nodes
            .iter()
            .find(|node| node.source.file == "level1" && node.source.path_id == source)
            .expect("retained terrain alignment node");
        assert!(!node.active);
        assert_eq!(
            serde_json::to_value(node.transform.scale).unwrap(),
            marker["scale"]
        );
    }
    let cover = manifest
        .nodes
        .iter()
        .find(|node| node.source.file == "level1" && node.source.path_id == 28128)
        .unwrap();
    assert_eq!(cover.transform.scale, [20.0, 0.1, 20.0]);
    for (actual, expected) in cover
        .transform
        .translation
        .into_iter()
        .zip([9.3, -1.013, 2.34])
    {
        assert!((actual - expected).abs() < 0.000001);
    }

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
fn driving_reference_rejects_nonfinite_values_and_unresolved_bindings() {
    let original = ReferenceManifest::from_json(include_bytes!("common/reference.json")).unwrap();
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        for field in 0..12 {
            let mut manifest = original.clone();
            let wheel = &mut manifest.wheel_slots[0];
            match field {
                0 => manifest.driving.pedal_rate = value,
                1 => manifest.driving.min_motor_torque_nm = value,
                2 => wheel.wheel_mass_kg = value,
                3 => wheel.rotational_damping = value,
                4 => wheel.force_application_distance_m = value,
                5 => wheel.sideways_stiffness_modifier = value,
                6 => wheel.hub.radius_m = value,
                7 => wheel.hub.mass_kg = value,
                8 => wheel.hub.forward_stiffness = value,
                9 => wheel.hub.sideways_stiffness = value,
                10 => wheel.forward_friction.extremum_slip = value,
                _ => wheel.sideways_friction.stiffness = value,
            }
            assert!(manifest.validate().is_err(), "nonfinite field {field}");
        }
    }
    let mut manifest = original.clone();
    manifest.driving.default_assists.automatic_clutch = false;
    manifest.driving.default_assists.automatic_gears = true;
    assert!(manifest.validate().is_err());
    let mut manifest = original.clone();
    manifest.driving.bindings[0].codes = vec!["Unresolved".into()];
    assert!(manifest.validate().is_err());
    let mut manifest = original;
    manifest.wheel_slots[0].forward_friction.asymptote_slip = 0.1;
    assert!(manifest.validate().is_err());
}
