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
    input["player"]["action_masks"]["use_mask"] = serde_json::json!(0);
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
        if !body.dynamic || body.id == manifest.vehicle.body {
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
fn rejects_missing_masks_and_invalid_action_owners() {
    for field in ["use_mask", "pickup_mask", "attach_mask", "push_mask"] {
        let mut input = fixture();
        input["player"]["action_masks"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(ReferenceManifest::from_json(&serde_json::to_vec(&input).unwrap()).is_err());
        let mut input = fixture();
        input["player"]["action_masks"][field] = serde_json::json!(0);
        assert!(ReferenceManifest::from_json(&serde_json::to_vec(&input).unwrap()).is_err());
    }
    for field in ["use_owner", "pickup_owner", "removal_owner", "push_owner"] {
        let mut input = fixture();
        input["action_bindings"][0][field] = serde_json::json!(999999);
        assert!(ReferenceManifest::from_json(&serde_json::to_vec(&input).unwrap()).is_err());
    }
    let mut input = fixture();
    input["action_bindings"].as_array_mut().unwrap().pop();
    assert!(ReferenceManifest::from_json(&serde_json::to_vec(&input).unwrap()).is_err());
}

#[test]
#[ignore = "requires installed reference export"]
fn installed_mechanisms_keep_source_controls_and_window_tracks() {
    let json: serde_json::Value =
        serde_json::from_slice(&std::fs::read("web/assets/reference/manifest.json").unwrap())
            .unwrap();
    let mechanisms = json["mechanisms"]
        .as_array()
        .expect("source rotation and slide mechanisms");
    for source in [
        64783, 64777, 64811, 64916, 64797, 75324, 77425, 71545, 71291, 66558, 73523, 67612, 70647,
        79151, 74645, 76322,
    ] {
        assert!(
            mechanisms.iter().any(|m| m["source"]["path_id"] == source),
            "missing source mechanism {source}"
        );
    }
    for source in [70647, 79151, 74645, 76322] {
        let window = mechanisms
            .iter()
            .find(|m| m["source"]["path_id"] == source)
            .unwrap();
        assert_eq!(window["slide"]["paths"].as_array().unwrap().len(), 1);
        assert_eq!(window["slide"]["rotations"][0]["angle_degrees"], -810.0);
        assert!(window["parent_item"].is_number());
    }
}

#[test]
fn lighting_state_counts_match_reachable_control_states() {
    use my_summer_slop::reference::{LampControl, LampGroupReference, LampStateReference};
    for (control, expected) in [
        (LampControl::Headlights, 3),
        (LampControl::Interior, 2),
        (LampControl::Home, 2),
    ] {
        let mut manifest =
            ReferenceManifest::from_json(include_bytes!("common/reference.json")).unwrap();
        manifest.lighting.groups.push(LampGroupReference {
            source: manifest.player.source.clone(),
            owner: None,
            control,
            electronics_source: None,
            electronics_output: 1.0,
            initial_on: false,
            initial_state: 0,
            initial_target: 0,
            initial_energy: 0.0,
            energy_move: 10.0,
            button: None,
            states: vec![
                LampStateReference {
                    intensity: 1.0,
                    lamps: vec![],
                    renderers: vec![],
                    material: None
                };
                expected
            ],
        });
        assert!(manifest.validate().is_ok());
        for count in [1, expected - 1, expected + 1] {
            let mut invalid = manifest.clone();
            let state = invalid.lighting.groups[0].states[0].clone();
            invalid.lighting.groups[0].states.resize(count, state);
            let bytes = serde_json::to_vec(&invalid).unwrap();
            let error = ReferenceManifest::from_json(&bytes)
                .err()
                .expect("Malformed state count must fail before Start");
            assert!(
                error.to_string().contains("lamp state count"),
                "{control:?}: {error}"
            );
        }
    }
}
