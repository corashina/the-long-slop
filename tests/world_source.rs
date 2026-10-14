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
    use my_summer_slop::reference::{WorldSourceLink, WorldValue};
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
    std::sync::Arc::make_mut(&mut m.world).records[0]
        .fields
        .insert("broken".into(), WorldValue::Link(stale.clone()));
    std::sync::Arc::make_mut(&mut m.world).records[0]
        .links
        .push(WorldSourceLink {
            field: "broken".into(),
            target: stale,
        });
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

#[test]
fn stable_keys_and_absolute_poses_roundtrip() {
    use my_summer_slop::{reference::SourceId, world::*};
    let key = ObjectKey::Generated(GeneratedId {
        layer: WorldLayer::Nested,
        generator: SourceId {
            file: "synthetic".into(),
            path_id: 1,
        },
        unit: [-2, 50000],
        ordinal: 8,
        children: vec![(
            SourceId {
                file: "synthetic".into(),
                path_id: 2,
            },
            9,
        )],
    });
    let pose = AbsolutePose {
        position: [20000001.125, -1., -5000000.5],
        rotation: [0., 0., 0., 1.],
    };
    let text = serde_json::to_vec(&(key.clone(), pose)).unwrap();
    let restored: (ObjectKey, AbsolutePose) = serde_json::from_slice(&text).unwrap();
    assert_eq!(restored, (key, pose));
}
