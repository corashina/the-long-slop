use crate::common;
use my_summer_slop::simulation::day::{advance, sky};
use rapier3d::math::{Rotation, Vector};

#[test]
fn day_and_night_follow_source_phases() {
    let reference = common::reference();
    let rules = &reference.day_cycle;
    let grass = [0.0, 0.0, 0.0, 1.0];
    let noon = sky(rules, rules.day_seconds * 0.5, grass);
    let night = sky(rules, rules.day_seconds + rules.night_seconds * 0.5, grass);
    // The light shines along local -Z. At noon it shines down. The moon shines down too.
    let down = |rotation: [f32; 4]| {
        let [x, y, z, w] = rotation;
        (Rotation::from_xyzw(x, y, z, w) * -Vector::Z).y
    };
    assert!(down(noon.sun_rotation) < -0.9);
    assert!(down(night.sun_rotation) < 0.0);
    assert!((noon.sun_intensity - rules.noon_intensity).abs() < 1e-4);
    assert_eq!(night.sun_intensity, rules.night_intensity);
    assert_eq!(noon.ambient, rules.day_ambient);
    assert_eq!(night.ambient, rules.night_ambient);
    assert_eq!(noon.fog_color, rules.day_fog_colors[2]);
    assert_eq!(night.fog_color, rules.night_fog_colors[2]);
    // Dusk ends dark at the horizon and the cycle wraps to dawn.
    let dusk = sky(rules, rules.day_seconds, grass);
    assert_eq!(dusk.sun_intensity, 0.0);
    let total = rules.day_seconds + rules.night_seconds;
    assert!(advance(rules, total - 1.0, 2.0) < 1.5);
}
