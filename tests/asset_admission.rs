pub use my_summer_slop::{reference, world};
#[path = "../src/presentation/world/preflight/selected.rs"]
mod selected;
use reference::{FileCatalog, ReferenceManifest, SourceId, prototype_plan::PrototypePlan};
use selected::{IoLedger, Reservation, SelectedMetadata, control_bytes, retained_plan_bytes};
use std::sync::Arc;
use world::{PresentationAllocation, WorldMetrics, WorldStream};

type State = SelectedMetadata<u32, u32>;

fn plan() -> PrototypePlan {
    serde_json::from_value(serde_json::json!({
        "json_bytes":100,"buffer_bytes":200,"mesh_bytes":256,"scene_bytes":128,
        "nodes":[],"primitives":[{"mesh":0,"primitive":0,"submesh":0,
        "vertices":4,"indices":6,"bytes":256,"attributes":["POSITION"],
        "has_source_material":false}]
    }))
    .unwrap()
}
fn planned() -> State {
    let mut state = State::PlanLoading(1);
    state.complete_metadata(Arc::new(plan()));
    state
}
fn catalog() -> FileCatalog {
    let receipt = |file: &str, bytes| serde_json::json!({"file":file,"bytes":bytes,"sha256":"a".repeat(64),"image_size":null});
    let input = receipt("reference/selected.plan.json", 100);
    let mut catalog: FileCatalog = serde_json::from_value(serde_json::json!({
        "files":[receipt("reference/selected.gltf",100),input.clone()],
        "plans":[{"asset_file":"reference/selected.gltf","input":input}],
        "dependency_staging_reserve":5242960
    }))
    .unwrap();
    catalog.admit().unwrap();
    catalog
}
fn loading_metadata() -> (State, PresentationAllocation, IoLedger) {
    let reservation = Reservation::metadata(&catalog(), "reference/selected.gltf").unwrap();
    assert_eq!((reservation.cpu, reservation.staging), (1800, 200));
    let state = State::PlanLoading(1);
    let io = state.io(reservation.staging);
    (
        state,
        io.replace(IoLedger::default(), PresentationAllocation::default()),
        io,
    )
}
#[test]
fn metadata_completion_retains_descriptor_without_payload_io() {
    let (mut state, allocation, old) = loading_metadata();
    let produced = Arc::new(plan());
    let pointer = produced.primitives.as_ptr();
    let bytes = retained_plan_bytes(&produced);
    state.complete_metadata(Arc::clone(&produced));
    let allocation = state.io(200).replace(old, allocation);
    let State::Planned(retained) = &state else {
        panic!("retained metadata")
    };
    assert_eq!(retained.handle, 1);
    assert!(Arc::ptr_eq(&retained.data, &produced));
    assert_eq!(retained.primitives.as_ptr(), pointer);
    assert_eq!(retained_plan_bytes(state.plan().unwrap()), bytes);
    assert_eq!(
        (
            allocation.jobs,
            allocation.pending,
            allocation.staging_bytes,
            allocation.dependency_staging_bytes
        ),
        (0, 0, 0, 0)
    );
    assert!(state.fork_ready().is_none());
    // The producer and another selected owner still see the same checked data.
    let mut second = State::PlanLoading(1);
    second.complete_metadata(Arc::clone(&produced));
    drop(state);
    assert_eq!(second.plan().unwrap().primitives.as_ptr(), pointer);
    assert_eq!(produced.primitives.as_ptr(), pointer);
}
#[test]
fn payload_admission_preserves_metadata_and_ledger_while_capacity_waits() {
    let mut state = planned();
    let pointer = state.plan().unwrap().primitives.as_ptr();
    let old_cpu = retained_plan_bytes(state.plan().unwrap());
    let reservation = state.payload_reservation().unwrap();
    assert_eq!(
        reservation.cpu,
        state.plan().unwrap().reservation().unwrap().0 + old_cpu + State::ready_control_bytes()
    );
    let metrics = WorldMetrics {
        prototype_cpu_bytes: old_cpu,
        ..Default::default()
    };
    let allocation = PresentationAllocation::default();
    for (jobs, staging, pending, cpu, gpu) in [
        (0, 600, 0, old_cpu, 0),
        (1, 599, 0, old_cpu, 0),
        (1, 600, 32, old_cpu, 0),
        (1, 600, 0, 256 * 1024 * 1024, 0),
        (1, 600, 0, old_cpu, 512 * 1024 * 1024),
    ] {
        assert!(!reservation.fits(
            &WorldMetrics {
                pending_work: pending,
                prototype_cpu_bytes: cpu,
                prototype_gpu_bytes: gpu,
                ..metrics
            },
            jobs,
            staging,
            old_cpu,
            0,
            false
        ));
        assert_eq!(state.plan().unwrap().primitives.as_ptr(), pointer);
        assert_eq!(state.io(0), IoLedger::default());
        assert_eq!((allocation.jobs, allocation.pending), (0, 0));
    }
    assert!(reservation.fits(&metrics, 1, 600, old_cpu, 0, false));
    let allocation = reservation.allocation(allocation, false);
    // The runtime reserves this ledger and CPU/GPU before starting SourceGltf.
    assert_eq!(
        (
            allocation.jobs,
            allocation.pending,
            allocation.staging_bytes
        ),
        (1, 1, 600)
    );
    state.start_payload(7);
    assert_eq!(state.plan().unwrap().primitives.as_ptr(), pointer);
    assert_eq!(
        state.io(600),
        IoLedger {
            jobs: 1,
            pending: 1,
            staging: 600
        }
    );
    let mut too_large = plan();
    too_large.json_bytes = 33 * 1024 * 1024;
    assert!(Reservation::payload(&too_large).is_err());
    too_large.json_bytes = usize::MAX;
    assert!(Reservation::payload(&too_large).is_err());
}
#[test]
fn full_pending_queue_drains_metadata_before_next_payload_admission() {
    let (mut state, _, old) = loading_metadata();
    let allocation = PresentationAllocation {
        jobs: 1,
        pending: 32,
        staging_bytes: 200,
        dependency_staging_bytes: 200,
        ..Default::default()
    };
    state.complete_metadata(Arc::new(plan()));
    let reservation = state.payload_reservation().unwrap();
    assert!(!reservation.fits(
        &WorldMetrics {
            pending_work: 32,
            ..Default::default()
        },
        1,
        1000,
        0,
        0,
        false
    ));
    let allocation = state.io(0).replace(old, allocation);
    assert_eq!((allocation.pending, allocation.jobs), (31, 0));
    assert!(reservation.fits(
        &WorldMetrics {
            pending_work: allocation.pending,
            ..Default::default()
        },
        2,
        1000,
        0,
        0,
        false
    ));
    let allocation = reservation.allocation(allocation, false);
    state.start_payload(2);
    assert_eq!((allocation.pending, allocation.jobs), (32, 1));
    assert!(!reservation.fits(
        &WorldMetrics {
            pending_work: 32,
            ..Default::default()
        },
        1,
        1000,
        0,
        0,
        false
    ));
}
#[test]
fn retirement_keeps_physical_load_charges_until_terminal_completion() {
    let mut payload = planned();
    payload.start_payload(2);
    for state in [State::PlanLoading(1), payload] {
        let io = state.io(600);
        let allocation = io.replace(IoLedger::default(), PresentationAllocation::default());
        assert!(!state.can_retire(false));
        assert_eq!(state.io(600), io);
        assert_eq!(
            (
                allocation.jobs,
                allocation.pending,
                allocation.staging_bytes
            ),
            (1, 1, 600)
        );
        assert!(state.can_retire(true));
        let allocation = IoLedger::default().replace(io, allocation);
        assert_eq!(
            (
                allocation.jobs,
                allocation.pending,
                allocation.staging_bytes
            ),
            (0, 0, 0)
        );
        let allocation = IoLedger::default().replace(IoLedger::default(), allocation);
        assert_eq!(
            (
                allocation.jobs,
                allocation.pending,
                allocation.staging_bytes
            ),
            (0, 0, 0)
        );
    }
    assert!(planned().can_retire(false));
    assert!(State::Queued.can_retire(false));
}
#[test]
fn ready_fork_shares_descriptor_and_accounts_separate_control_storage() {
    let mut state = planned();
    let pointer = state.plan().unwrap().primitives.as_ptr();
    state.start_payload(8);
    state.complete_payload();
    let copy = state.fork_ready().unwrap();
    let (State::Ready(active), State::Ready(candidate)) = (&state, &copy) else {
        panic!("ready fork")
    };
    assert!(Arc::ptr_eq(active, candidate));
    assert_eq!(candidate.plan.primitives.as_ptr(), pointer);
    assert_eq!((candidate.plan.handle, candidate.asset), (1, 8));
    let active_entries = vec![state];
    let candidate_entries = vec![copy];
    let candidate_retired = Vec::<SourceId>::with_capacity(candidate_entries.len());
    let active_control = control_bytes::<State>(active_entries.capacity(), 7, 1, 0);
    let candidate_control = control_bytes::<State>(
        candidate_entries.capacity(),
        7,
        candidate_retired.capacity(),
        0,
    );
    assert_eq!(
        candidate_control,
        std::mem::size_of::<State>() + 7 + std::mem::size_of::<SourceId>()
    );
    let allocation = PresentationAllocation {
        prototype_cpu: active_control,
        ..Default::default()
    };
    let allocation = PresentationAllocation {
        prototype_cpu: allocation.prototype_cpu + candidate_control,
        ..allocation
    };
    assert_eq!(allocation.prototype_cpu, active_control + candidate_control);
    let State::Ready(active) = &active_entries[0] else {
        unreachable!()
    };
    drop(candidate_entries);
    assert_eq!(Arc::strong_count(active), 1);
    assert_eq!(active.plan.primitives.as_ptr(), pointer);
    let mut ledger = IoLedger::default();
    ledger.add(active_entries[0].io(0));
    assert_eq!(ledger, IoLedger::default());
}
#[test]
#[ignore = "requires the installed checked source export"]
fn installed_stream_retains_and_releases_selected_reservations() {
    let bytes = std::fs::read("web/assets/reference/manifest.json").unwrap();
    let reference = ReferenceManifest::from_runtime_json(&bytes).unwrap();
    let mut stream = WorldStream::new(&reference, reference.world.terrain.seed).unwrap();
    let source = SourceId {
        file: "admission-test".into(),
        path_id: 1,
    };
    stream
        .configure_dependency_staging(
            reference.file_catalog.dependency_staging_reserve as usize,
            0,
        )
        .unwrap();
    assert_eq!(stream.available_external_jobs(), 0);
    let selected = reference
        .assets
        .iter()
        .find(|a| a.usage == reference::AssetUsage::UniqueScene)
        .unwrap();
    let fixed_metadata = Reservation::metadata(&reference.file_catalog, &selected.path).unwrap();
    assert!(
        !reference::fixed_reservation::fixed_room(
            &stream,
            fixed_metadata.cpu,
            0,
            fixed_metadata.staging,
            true
        )
        .unwrap()
    );
    for _ in 0..4096 {
        if stream.available_external_jobs() > 0 {
            break;
        }
        assert_eq!(stream.presentation_allocation().jobs, 0);
        stream.prepare();
        assert!(stream.frame().failure.is_none());
    }
    assert!(
        stream.available_external_jobs() > 0,
        "index preparation must permit metadata admission"
    );
    assert!(
        reference::fixed_reservation::fixed_room(
            &stream,
            fixed_metadata.cpu,
            0,
            fixed_metadata.staging,
            true
        )
        .unwrap()
    );
    let fixed = reference::fixed_reservation::FixedAllocation {
        cpu: fixed_metadata.cpu,
        gpu: 0,
        jobs: 1,
        pending: 1,
        staging: fixed_metadata.staging,
    };
    stream
        .set_presentation_allocation(
            fixed
                .replace(Default::default(), stream.presentation_allocation())
                .unwrap(),
        )
        .unwrap();
    assert!(stream.presentation_allocation().jobs <= 2);
    stream
        .set_presentation_allocation(
            reference::fixed_reservation::FixedAllocation::default()
                .replace(fixed, stream.presentation_allocation())
                .unwrap(),
        )
        .unwrap();
    let baseline = stream.metrics();
    let reservation = Reservation::metadata(&catalog(), "reference/selected.gltf").unwrap();
    assert!(reservation.fits(
        &baseline,
        stream.available_external_jobs(),
        stream.available_dependency_staging_bytes(),
        0,
        0,
        true
    ));
    stream
        .reserve_prototype(source.clone(), reservation.cpu, 0)
        .unwrap();
    let mut state = State::PlanLoading(1);
    let old_io = state.io(reservation.staging);
    stream
        .set_presentation_allocation(
            old_io.replace(IoLedger::default(), stream.presentation_allocation()),
        )
        .unwrap();
    state.complete_metadata(Arc::new(plan()));
    let retained = retained_plan_bytes(state.plan().unwrap());
    stream.resize_prototype(&source, retained, 0).unwrap();
    stream
        .set_presentation_allocation(
            state
                .io(0)
                .replace(old_io, stream.presentation_allocation()),
        )
        .unwrap();
    assert_eq!(
        (
            stream.presentation_allocation().jobs,
            stream.presentation_allocation().pending
        ),
        (0, 0)
    );
    let payload = state.payload_reservation().unwrap();
    stream
        .resize_prototype(&source, payload.cpu, payload.gpu)
        .unwrap();
    stream
        .set_presentation_allocation(payload.allocation(stream.presentation_allocation(), false))
        .unwrap();
    state.start_payload(2);
    assert!(!state.can_retire(false));
    assert_eq!(stream.presentation_allocation().jobs, 1);
    assert!(state.can_retire(true));
    stream
        .set_presentation_allocation(
            IoLedger::default()
                .replace(state.io(payload.staging), stream.presentation_allocation()),
        )
        .unwrap();
    stream.release_prototype(&source).unwrap();
    assert_eq!(
        stream.metrics().prototype_gpu_bytes,
        baseline.prototype_gpu_bytes
    );
    assert_eq!(stream.presentation_allocation().jobs, 0);
}

#[test]
fn fixed_mesh_without_normals_reserves_expansion() {
    use reference::fixed_reservation::FixedReservation;
    let mut source = plan();
    source.nodes = serde_json::from_value(serde_json::json!([{
        "index":0,"mesh":0,"skin":null,"children":[],"translation":[0,0,0],
        "rotation":[0,0,0,1],"scale":[1,1,1]
    }]))
    .unwrap();
    let primitive = &mut source.primitives[0];
    primitive.vertices = 3;
    primitive.indices = 12;
    primitive.bytes = 3 * 12 + 12 * 4;
    source.mesh_bytes = primitive.bytes + primitive.vertices * 32;
    let retained = retained_plan_bytes(&source);
    let bound = FixedReservation::mesh(&source, retained).unwrap();
    let expanded = 12 * (12 + 32) + 12 * 4;
    assert_eq!(bound.mesh_bytes, expanded);
    assert_eq!(
        bound.cpu,
        18 * 100 + 2 * 200 + 3 * expanded + 128 + 4096 + retained
    );
    assert_eq!((bound.gpu, bound.staging), (expanded + 8, 600));
    source.nodes.push(
        serde_json::from_value(serde_json::json!({
            "index":1,"mesh":0,"children":[],"translation":[0,0,0],
            "rotation":[0,0,0,1],"scale":[1,1,1]
        }))
        .unwrap(),
    );
    assert_eq!(
        FixedReservation::mesh(&source, retained)
            .unwrap()
            .mesh_bytes,
        2 * expanded
    );
    source.primitives[0].bytes += 1;
    assert!(FixedReservation::mesh(&source, retained).is_err());
    source.primitives[0].indices = usize::MAX;
    assert!(FixedReservation::mesh(&source, retained).is_err());
    source.primitives[0].indices = 12;
    source.primitives[0].bytes -= 1;
    source.json_bytes = usize::MAX;
    assert!(FixedReservation::mesh(&source, retained).is_err());
    source.json_bytes = 100;
    source.primitives.push(serde_json::from_value(serde_json::json!({"mesh":9,"primitive":0,"submesh":0,"vertices":3,"indices":3,"bytes":48,"attributes":["POSITION"],"has_source_material":false})).unwrap());
    assert!(
        FixedReservation::mesh(&source, retained).is_err(),
        "unreferenced Bevy imports still allocate meshes"
    );
}

#[test]
fn fixed_cloud_reservation_uses_checked_counts() {
    use reference::fixed_reservation::FixedReservation;
    let mut cloud: reference::lighting_scene::CloudMesh =
        serde_json::from_value(serde_json::json!({
            "source":{"file":"fixture","path_id":1},
            "file":"reference/source-cloud.gltf","buffer":"reference/source-cloud.bin",
            "vertex_count":829,"index_count":4596,"bounds_center":[0,0,0],"bounds_extents":[1,1,1]
        }))
        .unwrap();
    let receipt = |file, bytes| serde_json::json!({"file":file,"bytes":bytes,"sha256":"a".repeat(64),"image_size":null});
    let mut files: FileCatalog = serde_json::from_value(serde_json::json!({
        "files":[receipt("reference/source-cloud.gltf",100),receipt("reference/source-cloud.bin",829*20+4596*4)],
        "plans":[],"dependency_staging_reserve":5242960
    })).unwrap();
    files.admit().unwrap();
    let bound = FixedReservation::cloud(&cloud, &files).unwrap();
    let mesh = 4596 * (12 + 8 + 32) + 4596 * 4;
    let binary = 829 * 20 + 4596 * 4;
    assert_eq!(bound.mesh_bytes, mesh);
    assert_eq!(bound.cpu, 1800 + 2 * binary + 3 * mesh + 8192);
    assert_eq!((bound.gpu, bound.staging), (mesh + 8, 2 * (100 + binary)));
    cloud.vertex_count += 1;
    assert!(FixedReservation::cloud(&cloud, &files).is_err());
    cloud.vertex_count = 829;
    cloud.buffer = "reference/missing.bin".into();
    assert!(FixedReservation::cloud(&cloud, &files).is_err());
}

fn row_texture() -> reference::LightingTextureRecord {
    let resource = |path| {
        serde_json::json!({
            "source":{"file":"fixture","path_id":1},"path":path,"sha256":"a".repeat(64)
        })
    };
    serde_json::from_value(serde_json::json!({
        "chunks":[
            {"data":resource("reference/a.smip"),"mip":0,"row":0,"width":4,"rows":1},
            {"data":resource("reference/b.smip"),"mip":0,"row":1,"width":4,"rows":1},
            {"data":resource("reference/c.smip"),"mip":1,"row":0,"width":2,"rows":1},
            {"data":resource("reference/d.smip"),"mip":2,"row":0,"width":1,"rows":1}],
        "mip_data":resource("reference/full.smip"),"source_data":resource("reference/raw.bin"),
        "id":1,"source":{"file":"fixture","path_id":1},"path":"reference/full.png",
        "sha256":"a".repeat(64),"width":4,"height":2,"source_format":4,"sample_channel":"rgba",
        "color_space":0,"filter_mode":1,"wrap_u":0,"wrap_v":0,"mip_count":3,"aniso":1,"mip_bias":0
    }))
    .unwrap()
}

#[test]
fn fixed_row_ranges_and_copy_preserve_authored_mips() {
    let texture = row_texture();
    assert_eq!(texture.validate_chunks().unwrap(), 44);
    let mut output = vec![0; 44];
    for (index, range) in [0..16, 16..32, 32..40, 40..44].into_iter().enumerate() {
        assert_eq!(texture.chunk_range(index).unwrap(), range);
        texture
            .copy_chunk(index, &vec![index as u8 + 1; range.len()], &mut output)
            .unwrap();
        assert!(output[range].iter().all(|&byte| byte == index as u8 + 1));
    }
    assert!(texture.chunk_range(4).is_err());
    assert!(texture.copy_chunk(0, &[0; 15], &mut output).is_err());
    assert!(texture.copy_chunk(0, &[0; 16], &mut output[..43]).is_err());
}

#[test]
fn fixed_row_coverage_and_overflow_reject() {
    let valid = row_texture();
    let mut bad = valid.clone();
    bad.chunks.pop();
    assert!(bad.validate_chunks().is_err());
    bad = valid.clone();
    bad.chunks.swap(0, 1);
    assert!(bad.validate_chunks().is_err());
    bad = valid.clone();
    bad.chunks.push(bad.chunks[0].clone());
    assert!(bad.validate_chunks().is_err());
    bad = valid.clone();
    bad.chunks[0].width = 2;
    assert!(bad.validate_chunks().is_err());
    bad = valid.clone();
    bad.chunks[0].data.source.path_id += 1;
    assert!(bad.validate_chunks().is_err());
    bad = valid.clone();
    bad.chunks[0].rows = 0;
    assert!(bad.validate_chunks().is_err());
    bad = valid.clone();
    bad.width = 1024;
    bad.height = 512;
    bad.mip_count = 1;
    bad.chunks.truncate(1);
    bad.chunks[0].width = 1024;
    bad.chunks[0].rows = 512;
    assert!(bad.validate_chunks().is_err());
    bad = valid.clone();
    bad.width = u32::MAX;
    bad.height = u32::MAX;
    bad.mip_count = 32;
    assert!(bad.validate_chunks().is_err());
    bad = valid.clone();
    bad.mip_count = 33;
    assert!(bad.validate_chunks().is_err());
    bad = valid.clone();
    bad.width = 0;
    assert!(bad.validate_chunks().is_err());
    bad = valid;
    bad.chunks[1].row = 0;
    assert!(bad.chunk_range(0).is_err());
}

#[test]
fn fixed_row_cancel_and_upload_keep_charge() {
    use reference::fixed_reservation::FixedImageOwnership;
    let mut row = FixedImageOwnership::new(44, true, 5242960).unwrap();
    assert_eq!(row.bytes(), (0, 0));
    assert!(row.begin_row().unwrap());
    assert_eq!(row.bytes(), (132 + 5242960, 44));
    row.cancel();
    assert!(!row.finish_row(false));
    assert_eq!(row.bytes(), (44 + 5242960, 44));
    assert!(row.finish_row(true));
    assert_eq!(row.bytes(), (44, 44));
    assert!(!row.begin_row().unwrap());
    row.discard().unwrap();
    assert_eq!(row.bytes(), (0, 0));
    let mut upload = FixedImageOwnership::new(44, true, 5242960).unwrap();
    assert!(upload.begin_row().unwrap());
    assert!(upload.finish_row(true));
    upload.assemble().unwrap();
    upload.cancel();
    assert_eq!(upload.bytes(), (132, 44));
    assert!(upload.discard().is_err());
    upload.uploaded().unwrap();
    assert_eq!(upload.bytes(), (44, 44));
    assert!(FixedImageOwnership::new(usize::MAX, true, 1).is_err());
}

#[test]
fn fixed_payloads_share_one_overlap_charge() {
    use reference::fixed_reservation::FixedAllocation;
    let fixed = FixedAllocation {
        cpu: 100,
        gpu: 200,
        ..Default::default()
    };
    let active = WorldMetrics {
        prototype_cpu_bytes: 300,
        prototype_gpu_bytes: 500,
        ..Default::default()
    };
    let mut overlap = active;
    fixed.exclude_shared(&mut overlap).unwrap();
    let candidate = fixed
        .replace(
            FixedAllocation::default(),
            PresentationAllocation {
                prototype_cpu: 17,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(
        overlap.prototype_cpu_bytes + candidate.prototype_cpu,
        active.prototype_cpu_bytes + 17
    );
    assert_eq!(
        overlap.prototype_gpu_bytes + candidate.prototype_gpu,
        active.prototype_gpu_bytes
    );
    assert_eq!(active.prototype_cpu_bytes, 300);
    let committed = fixed.replace(fixed, candidate).unwrap();
    assert_eq!(
        (committed.prototype_cpu, committed.prototype_gpu),
        (117, 200)
    );
    assert!(
        fixed
            .replace(fixed, PresentationAllocation::default())
            .is_err()
    );
}

#[test]
fn fixed_png_reservation_checks_input_and_decode_arithmetic() {
    use reference::fixed_reservation::{png_cpu, png_staging};
    assert_eq!(png_staging(100, 4).unwrap(), 300 + 2 * 1024 * 1024 + 512);
    assert_eq!(png_cpu(44, 1000).unwrap(), 2088);
    assert!(png_staging(usize::MAX, 4).is_err());
    assert!(png_cpu(usize::MAX, 1).is_err());
    assert!(png_cpu(1, usize::MAX).is_err());
}

#[test]
#[ignore = "requires the installed checked source export"]
fn fixed_row_assembly_preserves_installed_source1065_and_fixed_mesh_contracts() {
    let reference =
        ReferenceManifest::from_json(&std::fs::read("web/assets/reference/manifest.json").unwrap())
            .unwrap();
    let texture = reference
        .lighting
        .textures
        .iter()
        .find(|t| t.source.path_id == 1065)
        .expect("source1065");
    let bytes = texture
        .validate_chunk_receipts(&reference.file_catalog)
        .unwrap();
    assert_eq!(bytes, 22_369_620);
    assert_eq!(texture.chunks.len(), 30);
    assert!(texture.filter_mode == 1 && texture.aniso <= 1 && texture.mip_bias == 0.);
    let mut output = vec![0; bytes];
    for (index, chunk) in texture.chunks.iter().enumerate() {
        let input =
            std::fs::read(std::path::Path::new("web/assets").join(&chunk.data.path)).unwrap();
        assert_eq!(
            input.len(),
            reference.file_catalog.file(&chunk.data.path).unwrap().bytes as usize
        );
        texture.copy_smip_chunk(index, &input, &mut output).unwrap();
    }
    let full =
        std::fs::read(std::path::Path::new("web/assets").join(&texture.mip_data.path)).unwrap();
    assert_eq!(full.len(), bytes + 16);
    assert_eq!(&full[..4], b"SMIP");
    assert_eq!(
        u32::from_le_bytes(full[4..8].try_into().unwrap()),
        texture.width
    );
    assert_eq!(
        u32::from_le_bytes(full[8..12].try_into().unwrap()),
        texture.height
    );
    assert_eq!(
        u32::from_le_bytes(full[12..16].try_into().unwrap()),
        texture.mip_count
    );
    assert_eq!(output, &full[16..]);
    let mut cpu = 0usize;
    let mut gpu = 0usize;
    for asset in reference.assets.iter().filter(|a| {
        matches!(
            a.usage,
            reference::AssetUsage::UniqueScene
                | reference::AssetUsage::RoadLampPrototype
                | reference::AssetUsage::SkyPrototype
        )
    }) {
        let receipt = reference.file_catalog.plan(&asset.path).unwrap();
        let plan: PrototypePlan = serde_json::from_slice(
            &std::fs::read(std::path::Path::new("web/assets").join(&receipt.file)).unwrap(),
        )
        .unwrap();
        let document: serde_json::Value = serde_json::from_slice(
            &std::fs::read(std::path::Path::new("web/assets").join(&asset.path)).unwrap(),
        )
        .unwrap();
        assert!(
            document
                .get("images")
                .is_none_or(|v| v.as_array().unwrap().is_empty())
        );
        assert!(
            document
                .get("skins")
                .is_none_or(|v| v.as_array().unwrap().is_empty())
        );
        assert_eq!(document["buffers"].as_array().unwrap().len(), 1);
        assert!(
            document["meshes"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|m| m["primitives"].as_array().unwrap())
                .all(|p| p
                    .get("targets")
                    .is_none_or(|v| v.as_array().unwrap().is_empty()))
        );
        let bound =
            reference::fixed_reservation::FixedReservation::mesh(&plan, retained_plan_bytes(&plan))
                .unwrap();
        cpu += bound.cpu;
        gpu += bound.gpu;
    }
    let cloud = reference::fixed_reservation::FixedReservation::cloud(
        &reference.render_lighting.sky.cloud_mesh,
        &reference.file_catalog,
    )
    .unwrap();
    assert!(cpu + cloud.cpu < 256 * 1024 * 1024);
    assert!(gpu + cloud.gpu < 512 * 1024 * 1024);
    println!(
        "installed fixed package {}, mesh reservation CPU={} GPU={}",
        reference.content_base,
        cpu + cloud.cpu,
        gpu + cloud.gpu
    );
}

#[test]
fn fixed_row_headers_and_receipts_reject_before_copy() {
    let t = row_texture();
    let mut output = vec![0; t.validate_chunks().unwrap()];
    let mut input = b"SMIP".to_vec();
    for value in [4u32, 1, 1] {
        input.extend(value.to_le_bytes());
    }
    input.extend([7u8; 16]);
    t.copy_smip_chunk(0, &input, &mut output).unwrap();
    input[0] = 0;
    assert!(t.copy_smip_chunk(0, &input, &mut output).is_err());
    input[0] = b'S';
    input[12] = 2;
    assert!(t.copy_smip_chunk(0, &input, &mut output).is_err());
    input[12] = 1;
    input[4] = 3;
    assert!(t.copy_smip_chunk(0, &input, &mut output).is_err());
    let receipt = |chunk: &reference::LightingTextureChunk| serde_json::json!({"file":chunk.data.path,"bytes":16+chunk.width*chunk.rows*4,"sha256":chunk.data.sha256,"image_size":null});
    let mut files:FileCatalog=serde_json::from_value(serde_json::json!({"files":t.chunks.iter().map(receipt).collect::<Vec<_>>(),"plans":[],"dependency_staging_reserve":5242960})).unwrap();
    files.admit().unwrap();
    assert_eq!(t.validate_chunk_receipts(&files).unwrap(), 44);
    files.files[0].bytes -= 1;
    assert!(t.validate_chunk_receipts(&files).is_err());
    files.files[0].bytes += 1;
    files.files[0].sha256 = "b".repeat(64);
    assert!(t.validate_chunk_receipts(&files).is_err());
}

#[test]
fn selected_shader_inputs_have_one_checked_owner() {
    use reference::fixed_reservation::{FIXED_SHADER_PATHS, shader_input};
    let files = FIXED_SHADER_PATHS
        .iter()
        .map(|path| {
            serde_json::json!({
                "file":path, "bytes":100, "sha256":"a".repeat(64), "image_size":null
            })
        })
        .collect::<Vec<_>>();
    let mut catalog: FileCatalog = serde_json::from_value(serde_json::json!({
        "files":files,"plans":[],"dependency_staging_reserve":5242960
    }))
    .unwrap();
    catalog.admit().unwrap();
    assert_eq!(FIXED_SHADER_PATHS.len(), 36);
    let mut unique = std::collections::HashSet::new();
    for path in FIXED_SHADER_PATHS {
        assert!(unique.insert(path));
        let bound = shader_input(&catalog, path).unwrap();
        assert_eq!((bound.cpu, bound.gpu, bound.staging), (5896, 0, 200));
    }
    assert!(shader_input(&catalog, "shaders/unselected.wgsl").is_err());
    assert!(shader_input(&crate::catalog(), FIXED_SHADER_PATHS[0]).is_err());
    // Every selected file import and custom import must already have its one owner.
    let root = std::path::Path::new("web/assets");
    let mut libraries = std::collections::HashMap::new();
    for (index, path) in FIXED_SHADER_PATHS.iter().enumerate() {
        let text = std::fs::read_to_string(root.join(path)).unwrap();
        for line in text.lines().map(str::trim) {
            if let Some(name) = line.strip_prefix("#define_import_path ") {
                assert!(libraries.insert(name.to_owned(), index).is_none());
            }
        }
    }
    for (index, path) in FIXED_SHADER_PATHS.iter().enumerate() {
        let text = std::fs::read_to_string(root.join(path)).unwrap();
        for line in text.lines().map(str::trim) {
            let Some(import) = line.strip_prefix("#import ") else {
                continue;
            };
            if let Some(quoted) = import.strip_prefix('"') {
                let file = quoted.split('"').next().unwrap();
                let dependency = FIXED_SHADER_PATHS
                    .iter()
                    .position(|p| *p == file)
                    .expect("owned shader file import");
                assert!(dependency < index, "{path} imports {file} before admission");
            } else if let Some(name) = import
                .split("::")
                .next()
                .filter(|n| n.starts_with("source_"))
            {
                assert!(
                    libraries
                        .get(name)
                        .is_some_and(|dependency| *dependency < index),
                    "unowned or unordered custom import {name}"
                );
            }
        }
    }
}

#[test]
fn retained_descriptor_set_exceeds_request_window() {
    let mut states = Vec::<State>::new();
    let mut allocation = PresentationAllocation::default();
    let mut retained_cpu = 0;
    for i in 0..40 {
        let reservation = Reservation::metadata(&catalog(), "reference/selected.gltf").unwrap();
        let mut state = State::PlanLoading(i);
        let io = state.io(reservation.staging);
        allocation = io.replace(IoLedger::default(), allocation);
        allocation.prototype_cpu += reservation.cpu;
        assert_eq!((allocation.pending, allocation.jobs), (1, 1));
        state.complete_metadata(Arc::new(plan()));
        let cpu = retained_plan_bytes(state.plan().unwrap()) + control_bytes::<State>(1, 0, 0, 0);
        retained_cpu += cpu;
        allocation.prototype_cpu = allocation.prototype_cpu - reservation.cpu + cpu;
        allocation = state.io(0).replace(io, allocation);
        states.push(state);
        assert_eq!(
            (
                allocation.pending,
                allocation.jobs,
                allocation.staging_bytes
            ),
            (0, 0, 0)
        );
    }
    assert_eq!(states.len(), 40);
    assert!(retained_cpu > 0);
    assert_eq!(allocation.prototype_cpu, retained_cpu);
    assert!(states.iter().all(|s| s.plan().is_some()));
    let reservation = states[39].payload_reservation().unwrap();
    assert!(reservation.fits(
        &WorldMetrics {
            prototype_cpu_bytes: retained_cpu,
            ..Default::default()
        },
        2,
        32 * 1024 * 1024,
        0,
        0,
        false
    ));
    let allocation = reservation.allocation(allocation, false);
    states[39].start_payload(40);
    assert_eq!((allocation.pending, allocation.jobs), (1, 1));
    assert!(states.iter().all(|s| s.plan().is_some()));
}
