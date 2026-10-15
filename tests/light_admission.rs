//! Descriptor admission must remain independent from payload construction.
use my_summer_slop::{reference::ReferenceManifest, world::WorldStream};

#[test]
#[ignore = "requires the installed checked source export"]
fn descriptor_readiness_polls_do_not_advance_or_admit_payloads() {
    let reference = ReferenceManifest::from_runtime_json(
        &std::fs::read("web/assets/reference/manifest.json").unwrap(),
    )
    .unwrap();
    let mut stream = WorldStream::new(&reference, reference.world.terrain.seed).unwrap();
    let before = stream.metrics();
    let stamp = stream.frame().request;
    for _ in 0..100 {
        assert!(!stream.descriptor_window_ready());
        assert!(stream.prepared_surface().is_none());
    }
    assert_eq!(stream.metrics().prepared_roads, before.prepared_roads);
    assert_eq!(stream.frame().request, stamp);
    assert!(stream.spawn_boundaries().is_empty());
    stream.prepare_descriptors();
    assert!(stream.metrics().prepared_roads > 0);
    assert_eq!(
        stream.metrics().prototype_cpu_bytes,
        before.prototype_cpu_bytes
    );
    assert_eq!(
        stream.metrics().prototype_gpu_bytes,
        before.prototype_gpu_bytes
    );
    assert!(stream.prepared_surface().is_none());
    stream.request_at([1., 0., 0.], [0.; 3]);
    assert_ne!(stream.frame().request.revision, stamp.revision);
    let stamp = stream.frame().request;
    stream.select_source_car(2);
    assert_ne!(stream.frame().request.revision, stamp.revision);
}
