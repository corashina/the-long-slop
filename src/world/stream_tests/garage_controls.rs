use crate::{reference::ReferenceManifest, starting_area::GarageRuntime};

#[path = "garage_controls/reactions.rs"]
mod reactions;

pub(super) fn load_garage() -> (ReferenceManifest, GarageRuntime) {
    let reference = super::reference();
    assert_eq!(reference.build_id, 11140890);
    let mut run = GarageRuntime::new_default(&reference).unwrap();
    super::prepare_ready(&mut run, &reference);
    assert!(run.world_supported());
    run.started = true;
    run.paused = false;
    (reference, run)
}
