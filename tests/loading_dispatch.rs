use the_long_slop::{
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
    let shared = 4096 + std::mem::size_of::<Option<the_long_slop::world::RequiredViews>>();
    let (bootstrap, ()) = admit_control(&mut stream, shared, || {
        allocated = true;
    })
    .unwrap();
    assert!(allocated);
    assert_eq!(
        stream.presentation_allocation().prototype_cpu,
        before.prototype_cpu + shared
    );
    let fixed = FixedAllocation {
        cpu: shared + 8192,
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
    // Candidate setup bytes survive fixed accounting and overlap reconciliation.
    let mut setup = candidate.presentation_allocation();
    setup.active_cpu += std::mem::size_of::<the_long_slop::world::RequiredViews>();
    candidate.set_presentation_allocation(setup).unwrap();
    candidate.set_overlap(overlap).unwrap();
    let replaced = fixed
        .replace(fixed, candidate.presentation_allocation())
        .unwrap();
    candidate.set_presentation_allocation(replaced).unwrap();
    assert_eq!(
        candidate.presentation_allocation().active_cpu,
        setup.active_cpu
    );
    drop(candidate);
    assert_eq!(
        stream.presentation_allocation().prototype_cpu,
        before.prototype_cpu + fixed.cpu
    );
    let mut runtime = the_long_slop::starting_area::GarageRuntime::new(&reference).unwrap();
    admit_control(&mut runtime.stream, shared, || ()).unwrap();
    let retained = runtime.stream.presentation_allocation();
    let discarded =
        the_long_slop::save::fresh_permit::FreshSessionPermit::prepare(7, &runtime, 1).unwrap();
    assert!(discarded.commit(8, &mut runtime, 1).is_err());
    assert_eq!(
        runtime.stream.presentation_allocation().prototype_cpu,
        retained.prototype_cpu
    );
    let permit =
        the_long_slop::save::fresh_permit::FreshSessionPermit::prepare(8, &runtime, 1).unwrap();
    permit.commit(8, &mut runtime, 1).unwrap();
    assert_eq!(
        runtime.stream.presentation_allocation().prototype_cpu,
        retained.prototype_cpu
    );
}
