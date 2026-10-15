use my_summer_slop::lighting::{CycleAdvance, CycleState};
use my_summer_slop::reference::{CycleReference, ReferenceManifest};

fn reference() -> CycleReference {
    ReferenceManifest::from_json(include_bytes!("common/reference.json"))
        .unwrap()
        .lighting
        .cycle
}

#[test]
fn invalid_elapsed_does_not_corrupt_state() {
    let reference = reference();
    let mut state = CycleState::new(&reference);
    state.advance(
        &reference,
        CycleAdvance {
            simulation_delta: (3.) as f32,
            cycle_extra: 0.,
            cloud_scale: 1.,
            sleep_progress: None,
        },
    );
    let previous = state.output().clone();
    let clock = state.elapsed_seconds();
    for invalid in [-1., f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        state.advance(
            &reference,
            CycleAdvance {
                simulation_delta: (invalid) as f32,
                cycle_extra: 0.,
                cloud_scale: 1.,
                sleep_progress: None,
            },
        );
        assert_eq!(*state.output(), previous);
        assert_eq!(state.elapsed_seconds(), clock);
    }
}
