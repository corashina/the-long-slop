use my_summer_slop::{
    reference::{ReferenceManifest, fixed_reservation::*},
    world::WorldStream,
};
#[test]
#[ignore = "requires installed source package"]
fn installed_graph_control_admission_precedes_allocation_and_replaces_bootstrap() {
    let reference = ReferenceManifest::from_runtime_json(
        &std::fs::read("web/assets/reference/manifest.json").unwrap(),
    )
    .unwrap();
    let mut stream = WorldStream::new(&reference, 42).unwrap();
    let mut allocated = false;
    assert!(
        admit_control(&mut stream, 4 * 1024 * 1024 * 1024, || {
            allocated = true;
        })
        .is_err()
    );
    assert!(!allocated);
    let before = stream.presentation_allocation();
    let (bootstrap, ()) = admit_control(&mut stream, 4096, || {
        allocated = true;
    })
    .unwrap();
    assert!(allocated);
    assert_eq!(
        stream.presentation_allocation().prototype_cpu,
        before.prototype_cpu + 4096
    );
    let fixed = FixedAllocation {
        cpu: 12288,
        ..FixedAllocation::default()
    };
    stream
        .set_presentation_allocation(
            fixed
                .replace(bootstrap, stream.presentation_allocation())
                .unwrap(),
        )
        .unwrap();
    assert_eq!(
        stream.presentation_allocation().prototype_cpu,
        before.prototype_cpu + fixed.cpu
    );
    let mut overlap = stream.metrics();
    let active = overlap.prototype_cpu_bytes;
    fixed.exclude_shared(&mut overlap).unwrap();
    assert_eq!(active, overlap.prototype_cpu_bytes + fixed.cpu);
    assert_eq!(
        stream.presentation_allocation().prototype_cpu,
        before.prototype_cpu + fixed.cpu
    );
    let mut candidate = WorldStream::new(&reference, 43).unwrap();
    candidate.set_overlap(overlap).unwrap();
    let candidate_before = candidate.presentation_allocation();
    let candidate_cpu = candidate.metrics().prototype_cpu_bytes;
    candidate
        .set_presentation_allocation(
            fixed
                .replace(FixedAllocation::default(), candidate_before)
                .unwrap(),
        )
        .unwrap();
    assert_eq!(
        candidate.presentation_allocation().prototype_cpu,
        candidate_before.prototype_cpu + fixed.cpu
    );
    assert_eq!(
        candidate.metrics().prototype_cpu_bytes,
        candidate_cpu + fixed.cpu
    );
    let mut runtime = my_summer_slop::starting_area::GarageRuntime::new(&reference).unwrap();
    admit_control(&mut runtime.stream, 4096, || ()).unwrap();
    let retained = runtime.stream.presentation_allocation();
    let discarded =
        my_summer_slop::save::fresh_permit::FreshSessionPermit::prepare(7, &runtime, 1).unwrap();
    assert!(discarded.commit(8, &mut runtime, 1).is_err());
    assert_eq!(
        runtime.stream.presentation_allocation().prototype_cpu,
        retained.prototype_cpu
    );
    let permit =
        my_summer_slop::save::fresh_permit::FreshSessionPermit::prepare(8, &runtime, 1).unwrap();
    permit.commit(8, &mut runtime, 1).unwrap();
    assert_eq!(
        runtime.stream.presentation_allocation().prototype_cpu,
        retained.prototype_cpu
    );
}
