use my_summer_slop::reference::{MAX_MANIFEST_BYTES, ReferenceManifest};

#[test]
fn package_receipts_preserve_internal_spaces_and_reject_unsafe_paths() {
    use my_summer_slop::reference::FileReceipt;
    let mut receipt = FileReceipt {
        file: "reference/lighting-unity default resources-10001.smip".into(),
        bytes: 3,
        sha256: "a".repeat(64),
        image_size: None,
    };
    receipt.validate().unwrap();
    for path in [
        "reference/a_b-0.1",
        "reference/internal  spaces.bin",
        "shaders/source_mesh.wgsl",
    ] {
        receipt.file = path.into();
        receipt.validate().unwrap();
    }
    for path in [
        "",
        "/reference/file.bin",
        "reference/",
        "reference//file.bin",
        "reference/./file.bin",
        "reference/../file.bin",
        "../file.bin",
        "./reference/file.bin",
        "reference\\file.bin",
        "C:/reference/file.bin",
        "reference/%2e%2e/file.bin",
        "reference/file%20name.bin",
        "reference/file.bin?x=1",
        "reference/file.bin#x",
        "reference/ file.bin",
        "reference/file.bin ",
        " reference/file.bin",
        "reference /file.bin",
        "reference/\tfile.bin",
        "reference/file\tname.bin",
        "reference/file.bin\n",
        "reference/file\rname.bin",
        "reference/file\0.bin",
        "reference/caf\u{e9}.bin",
        "reference/file+name.bin",
        "reference/file~name.bin",
    ] {
        receipt.file = path.into();
        assert_eq!(
            receipt.validate().unwrap_err().to_string(),
            "Invalid reference field: package file path",
            "{path:?}"
        );
    }
}

#[test]
fn receipt_catalog_rejects_oversize_images_and_unreserved_selected_plans() {
    use my_summer_slop::reference::{FileCatalog, FileReceipt};
    let receipt = |file: &str, bytes| serde_json::json!({"file":file,"bytes":bytes,"sha256":"a".repeat(64),"image_size":null});
    let input = receipt("reference/mesh.plan.json", 100);
    let mut catalog: FileCatalog = serde_json::from_value(serde_json::json!({
        "files":[receipt("reference/mesh.gltf", 200), input.clone()],
        "plans":[{"asset_file":"reference/mesh.gltf","input":input}],
        "dependency_staging_reserve":5242960
    }))
    .unwrap();
    catalog.admit().unwrap();
    assert_eq!(catalog.file("reference/mesh.gltf").unwrap().bytes, 200);
    assert!(catalog.file("reference/unused.bin").is_err());
    assert!(
        catalog
            .check_plan_reservation("reference/mesh.gltf", 1799, 200)
            .is_err()
    );
    assert!(
        catalog
            .check_plan_reservation("reference/mesh.gltf", 1800, 199)
            .is_err()
    );
    catalog
        .check_plan_reservation("reference/mesh.gltf", 1800, 200)
        .unwrap();
    assert!(catalog.retained_bytes() > 200);
    let mut png: FileReceipt = serde_json::from_value(receipt("reference/a.png", 30)).unwrap();
    assert!(png.validate().is_err());
    png.image_size = Some([u32::MAX, 2]);
    assert!(png.validate().is_err());
    png.image_size = Some([2, 3]);
    png.validate().unwrap();
    let audio: FileReceipt = serde_json::from_value(receipt("reference/radio.wav", 57_418_028)).unwrap();
    audio.validate().unwrap();
    let mut plan: my_summer_slop::reference::prototype_plan::PrototypePlan = serde_json::from_value(serde_json::json!({
        "json_bytes":1,"buffer_bytes":0,"mesh_bytes":0,"scene_bytes":0,"nodes":[],"primitives":[]
    })).unwrap();
    assert_eq!(plan.reservation().unwrap(), (18, 0));
    plan.mesh_bytes = usize::MAX;
    assert!(plan.reservation().is_err());
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
    let evaluator = my_summer_slop::world::TerrainEvaluator::new(
        &reference.world,
        reference.world.terrain.seed,
    );
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
    runtime.stream.prepare();
    assert_eq!(runtime.stream.metrics().prepared_roads, 256);
    use my_summer_slop::world::{ROAD_SEGMENTS, RoadIndex, RoadSequence};
    let seed = reference.world.terrain.seed;
    let mut small = RoadIndex::new(
        RoadSequence::new(&reference.world, seed).unwrap(),
        my_summer_slop::world::TerrainEvaluator::new(&reference.world, seed),
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
        assert_eq!(small.segment(index), large.segment(index));
    }
    eprintln!(
        "Both road indexes: {:?}; largest 256-section batch: {peak:?}",
        start.elapsed()
    );
}

#[test]
fn two_sided_source_shadows_pass_renderer_validation() {
    let mut record = my_summer_slop::reference::render_lighting::RendererRecord {
        source: my_summer_slop::reference::SourceId {
            file: "source".into(),
            path_id: 1,
        },
        bounds_center: [0.; 3],
        bounds_extents: [0.; 3],
        cast_shadows: 2,
        receive_shadows: true,
        reflection_probe_usage: 0,
        motion_vectors: 1,
        sorting_layer: 0,
        sorting_order: 0,
    };
    record.validate().unwrap();
    record.cast_shadows = 4;
    assert!(record.validate().is_err());
}

#[test]
fn source_world_sized_documents_reach_contract_validation() {
    let mut bytes = b"{}".to_vec();
    bytes.resize(130 * 1024 * 1024, b' ');
    for result in [
        ReferenceManifest::from_runtime_json(&bytes),
        ReferenceManifest::from_json(&bytes),
    ] {
        assert!(result.unwrap_err().to_string().contains("missing field"));
    }
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
