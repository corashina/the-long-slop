use my_summer_slop::reference::{AssetUsage, ReferenceManifest};
#[test]
fn prototype_loading_does_not_spawn_a_pole_at_the_origin() {
    let manifest = ReferenceManifest::from_json(include_bytes!("common/reference.json")).unwrap();
    assert_eq!(manifest.expected_instances(), 2);
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
