//! Siphon hose rules (`tubescript`).

use super::level::{inverse_lerp, lerp};

/// Fluid moved this frame between two attached ends.
#[allow(clippy::too_many_arguments)]
pub(crate) fn siphon_rate(
    height_diff: f32,
    min_h: f32,
    max_h: f32,
    valve1: f32,
    flow1: f32,
    valve2: f32,
    flow2: f32,
    pour_speed: f32,
    dt: f32,
) -> f32 {
    (valve1 * flow1).min(valve2 * flow2)
        * pour_speed
        * dt
        * inverse_lerp(min_h, max_h, height_diff.abs())
}

/// Returns `(source, receiver)` end indices. The higher end is the source.
pub(crate) fn siphon_direction(y1: f32, y2: f32) -> (usize, usize) {
    if y1 > y2 { (0, 1) } else { (1, 0) }
}

/// End index that attaches to the cap, as `TryAttach`.
pub(crate) fn pick_end(
    dist_sq_t1: f32,
    dist_sq_t2: f32,
    t1_attached: bool,
    t2_attached: bool,
    t1_cap_is_target: bool,
    t2_cap_is_target: bool,
) -> Option<usize> {
    if t1_cap_is_target || t2_cap_is_target {
        return None;
    }
    let (near, far, near_free, far_free) = if dist_sq_t1 < dist_sq_t2 {
        (0, 1, !t1_attached, !t2_attached)
    } else {
        (1, 0, !t2_attached, !t1_attached)
    };
    if near_free {
        Some(near)
    } else if far_free {
        Some(far)
    } else {
        None
    }
}

/// A cap accepts a hose end.
pub(crate) fn cap_accepts(valve: f32, cant_tube: bool, tube_flag: bool) -> bool {
    valve > 0.0 && !cant_tube && !tube_flag
}

/// Owncap valve step toward `target`.
pub(crate) fn ease_valve(v: f32, target: f32, dt: f32) -> f32 {
    lerp(v, target, dt * 5.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rate(h: f32) -> f32 {
        siphon_rate(h, 0.05, 0.15, 1.0, 2.0, 0.5, 3.0, 4.0, 0.1)
    }

    #[test]
    fn siphon_rate_scales_with_height() {
        // min(1*2, 0.5*3) * 4 * 0.1 = 0.6
        assert_eq!(rate(0.04), 0.0);
        assert!((rate(0.10) - 0.3).abs() < 1e-6);
        assert!((rate(0.20) - 0.6).abs() < 1e-6);
        assert!((rate(-0.20) - 0.6).abs() < 1e-6);
    }

    #[test]
    fn siphon_direction_higher_end_is_source() {
        assert_eq!(siphon_direction(1.0, 0.5), (0, 1));
        assert_eq!(siphon_direction(0.5, 1.0), (1, 0));
    }

    #[test]
    fn pick_end_rules() {
        assert_eq!(pick_end(1.0, 2.0, false, false, false, false), Some(0));
        assert_eq!(pick_end(2.0, 1.0, false, false, false, false), Some(1));
        assert_eq!(pick_end(1.0, 2.0, true, false, false, false), Some(1));
        assert_eq!(pick_end(2.0, 1.0, false, true, false, false), Some(0));
        assert_eq!(pick_end(1.0, 2.0, true, true, false, false), None);
        assert_eq!(pick_end(1.0, 2.0, false, true, false, true), None);
        assert_eq!(pick_end(1.0, 2.0, true, false, true, false), None);
    }

    #[test]
    fn cap_accepts_rules() {
        assert!(cap_accepts(1.0, false, false));
        assert!(!cap_accepts(0.0, false, false));
        assert!(!cap_accepts(1.0, true, false));
        assert!(!cap_accepts(1.0, false, true));
    }

    #[test]
    fn ease_valve_steps_and_clamps() {
        assert!((ease_valve(0.0, 1.0, 0.1) - 0.5).abs() < 1e-6);
        assert_eq!(ease_valve(0.0, 1.0, 1.0), 1.0);
    }
}
