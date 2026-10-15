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
        .configure_dependency_staging(catalog().dependency_staging_reserve as usize, 0)
        .unwrap();
    assert_eq!(stream.available_external_jobs(), 0);
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
