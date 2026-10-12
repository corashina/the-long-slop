use my_summer_slop::reference::{AssetUsage, ReferenceManifest};
#[test]
fn prototype_loading_does_not_spawn_a_pole_at_the_origin() {
    let manifest = ReferenceManifest::from_json(include_bytes!("common/reference.json")).unwrap();
    assert_eq!(manifest.expected_instances(), 3);
    let prototype = manifest
        .assets
        .iter()
        .find(|a| a.usage == AssetUsage::RoadLampPrototype)
        .unwrap();
    assert!(
        manifest
            .bodies
            .iter()
            .all(|b| manifest.nodes[b.node.0 as usize - 1].asset != prototype.id)
    );
    assert!(
        manifest
            .items
            .iter()
            .all(|b| manifest.nodes[b.node.0 as usize - 1].asset != prototype.id)
    );
    assert_eq!(
        manifest
            .assets
            .iter()
            .filter(|a| a.usage == AssetUsage::RoadLampPrototype)
            .count(),
        1
    );
}
#[test]
fn rejects_missing_cookie_and_invalid_controller_links() {
    let original = ReferenceManifest::from_json(include_bytes!("common/reference.json")).unwrap();
    let mut m = original.clone();
    let cookie = m.lighting.lights.iter().find_map(|l| l.cookie).unwrap();
    m.lighting.textures.retain(|t| t.id != cookie);
    assert!(m.validate().is_err());
    let mut m = original.clone();
    m.lighting.controllers[0].node.0 = 999999;
    assert!(m.validate().is_err());
    let mut m = original.clone();
    m.lighting
        .controllers
        .push(m.lighting.controllers[0].clone());
    assert!(m.validate().is_err());
    let mut m = original;
    m.lighting.cycle.day_angle_min = f32::NAN;
    assert!(m.validate().is_err());
}

#[test]
#[ignore = "Requires the local installed-build export"]
fn production_lighting_has_checked_source_links() {
    let bytes = std::fs::read("web/assets/reference/manifest.json").unwrap();
    let manifest = ReferenceManifest::from_json(&bytes).unwrap();
    assert_eq!(manifest.expected_instances(), 3);
    assert_eq!(manifest.lighting.controllers.len(), 10);
    assert_eq!(manifest.lighting.lights.len(), 17);
    assert_eq!(manifest.lighting.cycle.startup_offset_seconds, 240.);
}

#[test]
fn rejects_missing_cabin_input_and_incomplete_door_links() {
    let original = ReferenceManifest::from_json(include_bytes!("common/reference.json")).unwrap();
    let index = original
        .lighting
        .controllers
        .iter()
        .position(|c| c.kind == my_summer_slop::reference::LightControllerKind::Cabin)
        .unwrap();
    let mut m = original.clone();
    m.lighting.controllers[index].cabin_input = None;
    assert!(m.validate().is_err());
    for field in 0..3 {
        for empty in [true, false] {
            let mut m = original.clone();
            let input = m.lighting.controllers[index].cabin_input.as_mut().unwrap();
            match field {
                0 => {
                    if empty {
                        input.door_slots.clear();
                    } else {
                        input.door_slots.pop();
                    }
                }
                1 => {
                    if empty {
                        input.door_usable_sources.clear();
                    } else {
                        input.door_usable_sources.pop();
                    }
                }
                _ => {
                    if empty {
                        input.door_nodes.clear();
                    } else {
                        input.door_nodes.pop();
                    }
                }
            }
            assert!(m.validate().is_err());
        }
        let mut m = original.clone();
        let input = m.lighting.controllers[index].cabin_input.as_mut().unwrap();
        match field {
            0 => input.door_slots.reverse(),
            1 => input.door_usable_sources.reverse(),
            _ => input.door_nodes.reverse(),
        }
        assert!(m.validate().is_err());
    }
    let mut m = original;
    m.lighting.controllers[index]
        .cabin_input
        .as_mut()
        .unwrap()
        .modes = vec![0, 2, 1];
    assert!(m.validate().is_err());
}

#[test]
fn rejects_missing_flare_payload_and_atlas() {
    let original = ReferenceManifest::from_json(include_bytes!("common/reference.json")).unwrap();
    assert_eq!(original.lighting.cycle.sun_flares.len(), 3);
    let mut m = original.clone();
    m.lighting.cycle.sun_flares.clear();
    assert!(m.validate().is_err());
    let mut m = original.clone();
    m.lighting.cycle.sun_flares[0].flare.sha256 = "0".repeat(64);
    assert!(m.validate().is_err());
    let mut m = original.clone();
    let atlas = m.lighting.cycle.sun_flares[0].atlas;
    m.lighting.textures.retain(|t| t.id != atlas);
    assert!(m.validate().is_err());
    let mut m = original;
    m.lighting.cycle.sun_flares[0].atlas = 999999;
    assert!(m.validate().is_err());
}

#[test]
fn rejects_missing_incomplete_and_nonfinite_visual_temperature_arrays() {
    let original: serde_json::Value =
        serde_json::from_slice(include_bytes!("common/reference.json")).unwrap();
    for field in ["temp_days", "temp_nights", "temp_add_noons"] {
        let mut value = original.clone();
        value["lighting"]["cycle"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(ReferenceManifest::from_json(&serde_json::to_vec(&value).unwrap()).is_err());
        for array in [
            serde_json::json!([1., 2.]),
            serde_json::json!([1., 2., 3., 4.]),
        ] {
            let mut value = original.clone();
            value["lighting"]["cycle"][field] = array;
            assert!(ReferenceManifest::from_json(&serde_json::to_vec(&value).unwrap()).is_err());
        }
    }
    let original = ReferenceManifest::from_json(include_bytes!("common/reference.json")).unwrap();
    for field in 0..3 {
        let mut m = original.clone();
        match field {
            0 => m.lighting.cycle.temp_days[0] = f32::NAN,
            1 => m.lighting.cycle.temp_nights[0] = f32::NAN,
            _ => m.lighting.cycle.temp_add_noons[0] = f32::NAN,
        }
        assert!(m.validate().is_err());
    }
}

#[test]
fn required_sky_dependency_is_validated() {
    let mut manifest =
        ReferenceManifest::from_json(include_bytes!("common/reference.json")).unwrap();
    let texture = manifest.lighting.cycle.clouds[0].material.textures[0].texture;
    manifest
        .lighting
        .textures
        .retain(|record| record.id != texture);
    assert!(
        manifest
            .validate()
            .unwrap_err()
            .to_string()
            .contains("sky properties")
    );
}

fn assert_missing_field(
    manifest: &ReferenceManifest,
    source: &my_summer_slop::reference::SourceId,
    field: &str,
) {
    let error = manifest
        .validate()
        .expect_err("missing setup binding must fail validation")
        .to_string();
    assert!(
        error.contains(&format!("{}:{}", source.file, source.path_id)),
        "{error}"
    );
    assert!(error.contains(field), "{error}");
}

#[test]
fn rejects_missing_required_sky_float() {
    let original = ReferenceManifest::from_json(include_bytes!("common/reference.json")).unwrap();
    for property in ["_SunSize", "_SunSizeConvergence"] {
        let mut manifest = original.clone();
        manifest.lighting.skybox.floats.remove(property);
        assert_missing_field(&manifest, &manifest.lighting.skybox.source, property);
    }
}

#[test]
fn rejects_missing_required_star_binding() {
    let original = ReferenceManifest::from_json(include_bytes!("common/reference.json")).unwrap();
    for renamed in [false, true] {
        let mut manifest = original.clone();
        let material = &mut manifest.lighting.cycle.star_material;
        if renamed {
            material.textures[0].property = "_UnusedTex".into();
        } else {
            material.textures.clear();
        }
        assert_missing_field(
            &manifest,
            &manifest.lighting.cycle.star_material.source,
            "textures[0]._MainTex",
        );
    }
}

#[test]
fn rejects_missing_required_cloud_binding() {
    let original = ReferenceManifest::from_json(include_bytes!("common/reference.json")).unwrap();
    for index in 0..original.lighting.cycle.clouds.len() {
        for property in ["_ScatterMap0", "_ScatterMap1", "_DensityMap", "_TextureMap"] {
            let mut manifest = original.clone();
            manifest.lighting.cycle.clouds[index]
                .material
                .textures
                .retain(|link| link.property != property);
            assert_missing_field(
                &manifest,
                &manifest.lighting.cycle.clouds[index].material.source,
                property,
            );
        }
    }
}

#[test]
fn rejects_missing_required_camera_role() {
    let original = ReferenceManifest::from_json(include_bytes!("common/reference.json")).unwrap();
    for role in ["Cam", "farCamera"] {
        let mut manifest = original.clone();
        manifest
            .lighting
            .cameras
            .iter_mut()
            .find(|camera| camera.role == role)
            .unwrap()
            .role = "unusedCamera".into();
        assert_missing_field(
            &manifest,
            &manifest.lighting.source,
            &format!("cameras.{role}"),
        );
    }
}

#[test]
fn rejects_missing_required_controller_family() {
    use my_summer_slop::reference::LightControllerKind;
    let original = ReferenceManifest::from_json(include_bytes!("common/reference.json")).unwrap();
    for (kind, field) in [
        (LightControllerKind::Headlight, "controllers.headlight"),
        (LightControllerKind::Rear, "controllers.rear"),
    ] {
        let mut manifest = original.clone();
        manifest
            .lighting
            .controllers
            .retain(|controller| controller.kind != kind);
        assert_missing_field(&manifest, &manifest.lighting.source, field);
    }
}

#[test]
fn rejects_missing_required_road_light() {
    let mut manifest =
        ReferenceManifest::from_json(include_bytes!("common/reference.json")).unwrap();
    manifest.lighting.road_lamp.lights.clear();
    assert_missing_field(
        &manifest,
        &manifest.lighting.road_lamp.source,
        "road lights[0]",
    );
}

#[test]
fn rejects_missing_or_invalid_source_mips() {
    let original = ReferenceManifest::from_json(include_bytes!("common/reference.json")).unwrap();
    let mut manifest = original.clone();
    manifest.lighting.textures[0].mip_count = 32;
    assert!(manifest.validate().is_err());
    let mut manifest = original;
    manifest.lighting.textures[0].mip_data.sha256.clear();
    assert!(manifest.validate().is_err());
}

#[test]
fn rejects_incomplete_flare_geometry() {
    let original = ReferenceManifest::from_json(include_bytes!("common/reference.json")).unwrap();
    for mutation in 0..4 {
        let mut manifest = original.clone();
        let flare = &mut manifest.lighting.cycle.sun_flares[0];
        match mutation {
            0 => flare.elements.clear(),
            1 => flare.elements[0].image = 11,
            2 => flare.elements[0].size = f32::NAN,
            _ => flare.fade_speed = -1.,
        }
        assert!(manifest.validate().is_err());
    }
}

#[test]
fn dashboard_contract_rejects_missing_and_wrong_source_links() {
    let mut value: serde_json::Value =
        serde_json::from_slice(include_bytes!("common/reference.json")).unwrap();
    value["lighting"]
        .as_object_mut()
        .unwrap()
        .remove("high_beam");
    assert!(ReferenceManifest::from_json(&serde_json::to_vec(&value).unwrap()).is_err());
    let original = ReferenceManifest::from_json(include_bytes!("common/reference.json")).unwrap();
    let mut bad = original.clone();
    bad.lighting.high_beam.node = bad.vehicle.root;
    assert!(bad.validate().is_err());
    let mut bad = original.clone();
    bad.lighting.high_beam.on = bad.lighting.high_beam.off.clone();
    assert!(bad.validate().is_err());
    let mut bad = original;
    bad.lighting.high_beam.max_value = f32::NAN;
    assert!(bad.validate().is_err());
}

#[test]
#[ignore = "Requires the installed export and Docker sha256sum"]
fn production_lighting_records_match_selected_scene() {
    let bytes = std::fs::read("web/assets/reference/manifest.json").unwrap();
    let manifest = ReferenceManifest::from_json(&bytes).unwrap();
    let actual: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let checked: serde_json::Value =
        serde_json::from_slice(include_bytes!("common/reference.json")).unwrap();
    assert_eq!(actual["source_hashes"], checked["source_hashes"]);
    let a = &actual["lighting"];
    let b = &checked["lighting"];
    for field in [
        "source",
        "day_seconds",
        "night_seconds",
        "dawn_seconds",
        "dusk_seconds",
        "startup_offset_seconds",
    ] {
        assert_eq!(a["cycle"][field], b["cycle"][field], "cycle {field}");
    }
    assert_eq!(manifest.lighting.controllers.len(), 10);
    assert_eq!(manifest.lighting.lights.len(), 17);
    for (light, checked) in a["lights"]
        .as_array()
        .unwrap()
        .iter()
        .zip(b["lights"].as_array().unwrap())
    {
        for field in [
            "id",
            "source",
            "kind",
            "pose",
            "range_meters",
            "full_cone_degrees",
            "inner_cone_degrees",
            "shadows",
            "culling_mask",
            "cookie",
        ] {
            assert_eq!(
                light[field], checked[field],
                "light {} {field}",
                light["id"]
            );
        }
    }
    for (control, checked) in a["controllers"]
        .as_array()
        .unwrap()
        .iter()
        .zip(b["controllers"].as_array().unwrap())
    {
        for field in [
            "id",
            "source",
            "kind",
            "startup_on",
            "requested_state",
            "current_state",
            "energy_move",
            "electronics_threshold",
        ] {
            assert_eq!(
                control[field], checked[field],
                "controller {} {field}",
                control["id"]
            );
        }
        assert_eq!(
            control["states"].as_array().unwrap().len(),
            checked["states"].as_array().unwrap().len()
        );
        for (state, expected) in control["states"]
            .as_array()
            .unwrap()
            .iter()
            .zip(checked["states"].as_array().unwrap())
        {
            for field in ["on_material", "off_material"] {
                assert_eq!(state[field]["source"], expected[field]["source"]);
                if !state[field].is_null() {
                    let asset = manifest
                        .assets
                        .iter()
                        .find(|asset| asset.id.0 as u64 == state[field]["asset"].as_u64().unwrap())
                        .unwrap();
                    let gltf: serde_json::Value = serde_json::from_slice(
                        &std::fs::read(format!("web/assets/{}", asset.path)).unwrap(),
                    )
                    .unwrap();
                    let index = state[field]["gltf_material"].as_u64().unwrap() as usize;
                    assert_eq!(
                        gltf["materials"][index]["extras"]["source"],
                        state[field]["source"]
                    );
                }
            }
        }
    }
    for field in ["source", "signal", "min_value", "max_value", "stay_on"] {
        assert_eq!(
            a["high_beam"][field], b["high_beam"][field],
            "dashboard {field}"
        );
    }
    for field in ["on", "off"] {
        let choice = &a["high_beam"][field];
        assert_eq!(choice["source"], b["high_beam"][field]["source"]);
        let asset = manifest
            .assets
            .iter()
            .find(|asset| asset.id.0 as u64 == choice["asset"].as_u64().unwrap())
            .unwrap();
        let gltf: serde_json::Value =
            serde_json::from_slice(&std::fs::read(format!("web/assets/{}", asset.path)).unwrap())
                .unwrap();
        let index = choice["gltf_material"].as_u64().unwrap() as usize;
        assert_eq!(
            gltf["materials"][index]["extras"]["source"],
            choice["source"]
        );
    }
    assert_eq!(a["default_spot_cookie"], b["default_spot_cookie"]);
    assert_eq!(
        a["cycle"]["cloud_shadow_cookie"],
        b["cycle"]["cloud_shadow_cookie"]
    );
    assert_eq!(manifest.lighting.textures.len(), 11);
    for (texture, expected) in a["textures"]
        .as_array()
        .unwrap()
        .iter()
        .zip(b["textures"].as_array().unwrap())
    {
        for field in [
            "id",
            "source",
            "sha256",
            "source_data",
            "width",
            "height",
            "mip_count",
        ] {
            assert_eq!(
                texture[field], expected[field],
                "texture {} {field}",
                texture["id"]
            );
        }
    }
    // Use the toolchain's digest utility rather than add a runtime hash package.
    for asset in &manifest.assets {
        check_file_hash(&asset.path, &asset.sha256);
        for dependency in &asset.dependencies {
            check_file_hash(&dependency.file, &dependency.sha256);
        }
    }
    for texture in &manifest.lighting.textures {
        check_file_hash(&texture.path, &texture.sha256);
        check_file_hash(&texture.mip_data.path, &texture.mip_data.sha256);
        check_file_hash(&texture.source_data.path, &texture.source_data.sha256);
    }
}

fn check_file_hash(path: &str, expected: &str) {
    let output = std::process::Command::new("sha256sum")
        .arg(format!("web/assets/{path}"))
        .output()
        .unwrap();
    assert!(output.status.success(), "hash {path}");
    assert_eq!(
        std::str::from_utf8(&output.stdout)
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap(),
        expected,
        "hash {path}"
    );
}

#[test]
fn lighting_resource_provenance_uses_the_complete_archive_path() {
    let mut manifest =
        ReferenceManifest::from_json(include_bytes!("common/reference.json")).unwrap();
    let texture = manifest
        .lighting
        .textures
        .iter_mut()
        .find(|t| t.source.file == "Resources/unity default resources")
        .unwrap();
    texture.source.file = "unity default resources".into();
    assert!(manifest.validate().is_err());
}
