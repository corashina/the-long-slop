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
