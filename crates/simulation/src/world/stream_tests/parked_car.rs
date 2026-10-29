//! A parked start Lada rests: it stays in place and plays no hits.
use super::drive_out::frame;
use super::*;
use crate::simulation::FIXED_DT;

#[test]
#[ignore = "requires the installed source export"]
fn parked_lada_rests() {
    let (reference, mut run) = garage_controls::load_garage();
    for _ in 0..4000 {
        if !run.vehicles.is_empty() {
            break;
        }
        frame(&mut run, &reference);
    }
    let key = run.vehicles.keys().next().expect("start car").clone();
    let chassis = run.physics.chassis_handle(&key);
    // The car lands on its wheels first.
    for _ in 0..(5. / FIXED_DT) as usize {
        frame(&mut run, &reference);
    }
    let start = run.physics.snapshot(&key).pose.position;
    let mut hits = 0;
    for _ in 0..(10. / FIXED_DT) as usize {
        frame(&mut run, &reference);
        let colliders = &run.physics.rapier().colliders;
        hits += run
            .physics
            .hits()
            .iter()
            .filter(|h| {
                h.colliders
                    .iter()
                    .any(|c| colliders[*c].parent() == Some(chassis))
            })
            .count();
    }
    let end = run.physics.snapshot(&key).pose.position;
    let drift = (0..3)
        .map(|i| (end[i] - start[i]).powi(2))
        .sum::<f32>()
        .sqrt();
    assert!(drift < 0.005, "parked car moved {drift} m");
    assert_eq!(hits, 0, "parked car hits");
}
