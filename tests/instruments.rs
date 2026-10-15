mod common;
use my_summer_slop::simulation::instruments::{InstrumentSample, InstrumentState};

#[test]
fn runtime_recovery_and_rebase_preserve_odometer() {
    let r = common::reference();
    let mut run = my_summer_slop::starting_area::GarageRuntime::new(&r).unwrap();
    run.started = true;
    run.vehicles[0].trip_m = 12345.;
    let mut s = InstrumentState::new(&r.vehicles[0].instruments);
    s.update(
        &r.vehicles[0].instruments,
        InstrumentSample {
            distance_m: run.vehicles[0].trip_m,
            ..Default::default()
        },
        1.,
    );
    let angles = s.digit_angles;
    run.recover(&r);
    run.stream.rebase([600., 0., 900.], &mut run.physics);
    s.update(
        &r.vehicles[0].instruments,
        InstrumentSample {
            distance_m: run.vehicles[0].trip_m,
            ..Default::default()
        },
        1.,
    );
    assert_eq!(s.digit_angles, angles);
    assert_eq!(run.vehicles[0].trip_m, 12345.);
    run = my_summer_slop::starting_area::GarageRuntime::new(&r).unwrap();
    run.started = true;
    s.reset(&r.vehicles[0].instruments);
    assert_eq!(run.vehicles[0].trip_m, 0.);

    assert_eq!(s.digit_angles, [0.; 5]);
}
