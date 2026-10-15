#[path = "common/world.rs"]
mod world_fixture;
use my_summer_slop::reference::ReferenceManifest;

#[test]
fn rejects_missing_world_broken_weights_and_wrong_source_hashes() {
    assert_eq!(world_fixture::synthetic_world().road.prototypes.len(), 3);
    let fixture = include_bytes!("common/reference.json");
    let mut missing: serde_json::Value = serde_json::from_slice(fixture).unwrap();
    missing.as_object_mut().unwrap().remove("world");
    assert!(ReferenceManifest::from_json(&serde_json::to_vec(&missing).unwrap()).is_err());
    let mut manifest = ReferenceManifest::from_json(fixture).unwrap();
    std::sync::Arc::make_mut(&mut manifest.world).prototypes[0].skins[0].weights[0][0] = -1.0;
    assert!(manifest.world.validate(&manifest).is_err());
    let mut manifest = ReferenceManifest::from_json(fixture).unwrap();
    std::sync::Arc::make_mut(&mut manifest.world).source_hashes[0].sha256 = "0".repeat(64);
    assert!(manifest.world.validate(&manifest).is_err());
}

#[test]
fn rejects_missing_prototype_dependencies_and_corrupt_local_bindings() {
    use my_summer_slop::reference::WorldValue;
    let original = ReferenceManifest::from_json(include_bytes!("common/reference.json")).unwrap();
    assert_eq!(original.expected_instances(), 2);
    let mut m = original.clone();
    m.assets.retain(|a| a.id != m.world.prototypes[0].asset);
    assert!(m.validate().is_err());
    let mut m = original.clone();
    std::sync::Arc::make_mut(&mut m.world).prototypes[0].skins[0]
        .bind_poses
        .pop();
    assert!(m.validate().is_err());
    let mut m = original.clone();
    std::sync::Arc::make_mut(&mut m.world).prototypes[0].skins[0].joints[0][0] = 999;
    assert!(m.validate().is_err());
    let mut m = original.clone();
    std::sync::Arc::make_mut(&mut m.world).prototypes[0].nodes[1].parent = Some(999);
    assert!(m.validate().is_err());
    let mut m = original.clone();
    std::sync::Arc::make_mut(&mut m.world).prototypes[0].skins[0].weights[0] = [0.; 4];
    assert!(m.validate().is_err());
    let mut m = original.clone();
    let mut stale = m.world.records[0].source.clone();
    stale.path_id = -999;
    let mut fields = serde_json::to_value(&m.world.records[0].fields).unwrap();
    fields.as_array_mut().unwrap().push(serde_json::json!({
        "name": "broken",
        "value": serde_json::to_value(WorldValue::Link(stale)).unwrap(),
    }));
    fields
        .as_array_mut()
        .unwrap()
        .sort_by(|a, b| a["name"].as_str().unwrap().cmp(b["name"].as_str().unwrap()));
    std::sync::Arc::make_mut(&mut m.world).records[0].fields =
        serde_json::from_value(fields).unwrap();
    assert!(m.validate().is_err());
    let mut m = original.clone();
    let source = m.world.records[0].source.clone();
    std::sync::Arc::make_mut(&mut m.world).prototypes[0]
        .colliders
        .push(my_summer_slop::reference::WorldColliderReference {
            source: source.clone(),
            node: 0,
            body: None,
            enabled: true,
            trigger: false,
            material: None,
            deformed_skin: None,
            source_empty_mesh: Some(source),
            geometry: None,
        });
    assert!(m.validate().is_err());
    let mut m = original;
    std::sync::Arc::make_mut(&mut m.world)
        .distances
        .remove("RoadGenDist");
    assert!(m.validate().is_err());
}
