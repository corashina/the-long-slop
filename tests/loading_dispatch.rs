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
        admit_control(&mut stream, usize::MAX, || {
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
}
