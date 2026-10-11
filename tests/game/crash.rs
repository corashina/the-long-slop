use crate::common;
use my_summer_slop::simulation::crash::{crash, falls_off};

#[test]
fn worn_cars_and_parts_break_at_source_thresholds() {
    let rules = common::reference().vehicle.crash;
    // A new car needs more than 15 km/h at the lowest threshold sample.
    assert_eq!(crash(&rules, 15.0, 0, [0.0, 0.0]), None);
    // A rusty car scales both speeds by 0.2: 3 to 8 km/h.
    let worn = crash(&rules, 9.0, 4, [0.99, 0.0]).expect("rusty car crash");
    assert_eq!(worn.lerp, 0.0);
    assert_eq!(worn.count, 1);
    let hard = crash(&rules, 120.0, 0, [0.99, 0.75]).expect("hard crash");
    assert_eq!(hard.lerp, 1.0);
    assert_eq!(hard.count, 6);
    // A wheel needs 0.6 when new and 0.12 when rusty.
    assert!(!falls_off(0.6, 0, 0.5));
    assert!(falls_off(0.6, 4, 0.13));
    assert!(!falls_off(0.6, 4, 0.12));
    // Power zero parts fall off in any real crash.
    assert!(falls_off(0.0, 0, 0.01));
}
