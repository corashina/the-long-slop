use my_summer_slop::reference::{MAX_MANIFEST_BYTES, ReferenceManifest};

#[test]
#[ignore = "requires the installed source export"]
fn installed_startup_selects_only_garage_lada() {
    let bytes = std::fs::read("web/assets/reference/manifest.json").unwrap();
    let reference = ReferenceManifest::from_runtime_json(&bytes).unwrap();
    assert_eq!(reference.vehicles.len(), 1);
    assert_eq!(reference.vehicles[0].source.path_id, 77839);
    let mut manifest: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(
        manifest["assets"]
            .as_array()
            .unwrap()
            .iter()
            .all(|asset| { asset["path"] != "reference/skoda.gltf" })
    );
    let packs = manifest["acquisition"]["startup_packs"]
        .as_array_mut()
        .unwrap();
    assert_eq!(packs.len(), 1);
    assert_eq!(packs[0]["vehicle"]["path_id"], 77839);
    packs.clear();
    let bytes = serde_json::to_vec(&manifest).unwrap();
    assert!(
        ReferenceManifest::from_runtime_json(&bytes)
            .unwrap_err()
            .to_string()
            .contains("selected packs")
    );
}

#[test]
#[ignore = "requires the installed source export"]
fn installed_manifest_checks_lod_bounds_and_initializes_runtime() {
    let bytes = std::fs::read("web/assets/reference/manifest.json").unwrap();
    let mut reference = ReferenceManifest::from_runtime_json(&bytes).unwrap();
    let index = reference
        .world
        .records
        .iter()
        .position(|r| r.class_name == "LODGroup")
        .unwrap();
    let fields = reference.world.records[index].fields.clone();
    let mut value = serde_json::to_value(&fields).unwrap();
    value
        .as_array_mut()
        .unwrap()
        .retain(|f| f["name"] != "m_LocalReferencePoint");
    std::sync::Arc::make_mut(&mut reference.world).records[index].fields =
        serde_json::from_value(value).unwrap();
    assert!(
        reference
            .validate()
            .unwrap_err()
            .to_string()
            .contains("m_LocalReferencePoint")
    );
    std::sync::Arc::make_mut(&mut reference.world).records[index].fields = fields;
    let root = &reference.world.startup[0].source;
    let index = reference
        .world
        .records
        .iter()
        .position(|r| &r.source == root)
        .unwrap();
    let root = std::sync::Arc::make_mut(&mut reference.world)
        .records
        .swap_remove(index);
    assert!(reference.validate().is_err());
    std::sync::Arc::make_mut(&mut reference.world)
        .records
        .push(root);
    let evaluator =
        my_summer_slop::world::terrain_evaluator(&reference.world, reference.world.terrain.seed);
    my_summer_slop::world::StartupTerrain::new(&reference.world, &evaluator, &[])
        .expect("installed startup frames initialize");
    let mut runtime = my_summer_slop::starting_area::GarageRuntime::new(&reference)
        .expect("installed world initializes");
    use my_summer_slop::world::PresentationAllocation;
    let allocation = runtime.stream.presentation_allocation();
    let mib = 1024 * 1024;
    for (name, next) in [
        (
            "World CPU",
            PresentationAllocation {
                active_cpu: 128 * mib + 1,
                ..allocation
            },
        ),
        (
            "World GPU",
            PresentationAllocation {
                active_gpu: 128 * mib + 1,
                ..allocation
            },
        ),
        (
            "Asset CPU",
            PresentationAllocation {
                prototype_cpu: 256 * mib + 1,
                ..allocation
            },
        ),
        (
            "Asset GPU",
            PresentationAllocation {
                prototype_gpu: 512 * mib + 1,
                ..allocation
            },
        ),
        (
            "Objects",
            PresentationAllocation {
                objects: 8193,
                ..allocation
            },
        ),
        (
            "Pending work",
            PresentationAllocation {
                pending: 33,
                ..allocation
            },
        ),
        (
            "Staging",
            PresentationAllocation {
                staging_bytes: 32 * mib + 1,
                ..allocation
            },
        ),
        (
            "Dependency staging",
            PresentationAllocation {
                dependency_staging_bytes: 1,
                ..allocation
            },
        ),
        (
            "Jobs",
            PresentationAllocation {
                jobs: 3,
                ..allocation
            },
        ),
    ] {
        let error = runtime
            .stream
            .set_presentation_allocation(next)
            .unwrap_err()
            .to_string();
        assert!(
            error.starts_with(&format!("{name} budget exceeded: requested ")),
            "{error}"
        );
        assert!(error.contains(", limit "), "{error}");
        assert_eq!(
            runtime.stream.presentation_allocation().jobs,
            allocation.jobs
        );
        assert_eq!(
            runtime.stream.presentation_allocation().active_cpu,
            allocation.active_cpu
        );
    }
    runtime.stream.prepare(0.);
    assert_eq!(runtime.stream.metrics().prepared_roads, 256);
    use my_summer_slop::world::{ROAD_SEGMENTS, RoadIndex, RoadSequence};
    let seed = reference.world.terrain.seed;
    let mut small = RoadIndex::new(
        RoadSequence::new(&reference.world, seed).unwrap(),
        my_summer_slop::world::terrain_evaluator(&reference.world, seed),
    );
    let mut large = RoadIndex::new(
        RoadSequence::new(&reference.world, seed).unwrap(),
        evaluator,
    );
    let start = std::time::Instant::now();
    while !small.prepare(8) {}
    let mut batches = 0;
    let mut peak = std::time::Duration::ZERO;
    loop {
        let batch_start = std::time::Instant::now();
        let ready = large.prepare(256);
        peak = peak.max(batch_start.elapsed());
        batches += 1;
        if ready {
            break;
        }
    }
    assert_eq!(batches, ROAD_SEGMENTS.div_ceil(256));
    assert_eq!(small.prepared_segments(), ROAD_SEGMENTS);
    assert_eq!(large.prepared_segments(), ROAD_SEGMENTS);
    assert_eq!(small.checkpoint_count(), large.checkpoint_count());
    for index in [0, 255, 256, 1031, ROAD_SEGMENTS - 1] {
        assert_eq!(small.segment(index).unwrap(), large.segment(index).unwrap());
    }
    eprintln!(
        "Both road indexes: {:?}; largest 256-section batch: {peak:?}",
        start.elapsed()
    );
}

#[test]
fn oversized_documents_stop_before_parsing() {
    let bytes = vec![b' '; MAX_MANIFEST_BYTES + 1];
    for result in [
        ReferenceManifest::from_runtime_json(&bytes),
        ReferenceManifest::from_json(&bytes),
    ] {
        assert_eq!(
            result.unwrap_err().to_string(),
            "Invalid reference field: manifest size"
        );
    }
}

#[test]
#[ignore = "requires the installed source export"]
fn installed_player_movement_contract() {
    let bytes = std::fs::read("web/assets/reference/manifest.json").unwrap();
    let reference = ReferenceManifest::from_runtime_json(&bytes).unwrap();
    let player = &reference.player;
    assert_eq!(player.source.path_id, 80125);
    assert_eq!(player.posture.feet.source.path_id, 57258);
    assert_eq!(player.posture.feet.source.file, "level1");
    assert_eq!(player.posture.feet.filter, 4_172_190_207);
    assert_eq!(player.posture.feet.membership, 1 << 12);
    assert!((player.posture.feet.center[1] + 0.573).abs() < 0.00001);
    assert_eq!(player.posture.sensor_hold_steps, 3);
    assert_eq!(player.view.foot_fov_degrees, 65.0);
    assert_eq!(player.view.car_fov_degrees, 65.0);
    for (index, sensor) in player.posture.clearance.iter().enumerate() {
        assert!((sensor.posture - (index + 1) as f32 / 10.0).abs() < 0.00001);
        assert_eq!(sensor.sphere.source.file, "sharedassets1.assets");
        assert_eq!(sensor.sphere.source.path_id, 124758);
        assert_eq!(sensor.sphere.membership, 1 << 14);
        assert_eq!(sensor.sphere.filter, 4_037_970_431);
        assert!((sensor.sphere.radius - 0.15).abs() < 0.00001);
        assert!((sensor.sphere.center[1] - (-0.685 + 1.335 * sensor.posture)).abs() < 0.00001);
    }
    let mut seat_sources: Vec<_> = reference.seats.iter().map(|s| s.source.path_id).collect();
    seat_sources.sort_unstable();
    assert_eq!(seat_sources, [65840, 66466, 69296, 76381, 77843]);
    for seat in &reference.seats {
        assert_eq!(seat.yaw_limits_degrees, [-179.0, 179.0]);
        assert_eq!(seat.pitch_limits_degrees, [-89.0, 89.0]);
        assert_eq!(seat.default_pitch_degrees, 0.0);
        assert_eq!(seat.min_exit_angle_degrees, 65.0);
    }
    for mutate in [
        |r: &mut ReferenceManifest| r.player.posture.clearance.reverse(),
        |r: &mut ReferenceManifest| r.player.posture.clearance[0].sphere.radius = 0.2,
        |r: &mut ReferenceManifest| r.player.posture.clearance[0].sphere.filter = 1,
        |r: &mut ReferenceManifest| r.player.posture.feet.center[0] = f32::NAN,
        |r: &mut ReferenceManifest| r.player.posture.standing_head.translation[1] = f32::INFINITY,
        |r: &mut ReferenceManifest| r.player.view.tap_seconds = 0.3,
        |r: &mut ReferenceManifest| r.seats[0].default_pitch_degrees = 1.0,
        |r: &mut ReferenceManifest| r.seats[0].pitch_limits_degrees = [-90.0, 90.0],
    ] {
        let mut invalid = reference.clone();
        mutate(&mut invalid);
        assert!(invalid.validate().is_err());
    }
    for mutate in [
        |r: &mut ReferenceManifest| r.world.capture_library.schema_version = 2,
        |r: &mut ReferenceManifest| r.world.capture_library.manifest_sha256 = "f".repeat(64),
        |r: &mut ReferenceManifest| r.world.capture_library.catalog_sha256 = "f".repeat(64),
        |r: &mut ReferenceManifest| r.world.capture_library.source_manifest_sha256 = "f".repeat(64),
        |r: &mut ReferenceManifest| r.world.library.manifest_sha256 = "f".repeat(64),
    ] {
        let mut invalid = reference.clone();
        mutate(&mut invalid);
        assert!(invalid.validate().is_err());
    }
    let mut missing_capture: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    missing_capture["world"]
        .as_object_mut()
        .unwrap()
        .remove("capture_library");
    assert!(
        ReferenceManifest::from_runtime_json(&serde_json::to_vec(&missing_capture).unwrap())
            .is_err()
    );
    let mut missing: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    missing["player"].as_object_mut().unwrap().remove("view");
    assert!(ReferenceManifest::from_runtime_json(&serde_json::to_vec(&missing).unwrap()).is_err());
}
