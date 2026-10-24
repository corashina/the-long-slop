//! Source ginlampscript: rubbing its wash trigger lights the lamp.
use crate::reference::GenieLampReference;

/// Seconds after the last rub while the lamp still counts as rubbed.
const RUB_HOLD: f64 = 1.;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GenieLamp {
    pub value: f32,
    pub rubbing: bool,
    last_rubbed: f64,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LampOutput {
    pub intensity: f32,
    pub range: f32,
    pub emission: [f32; 4],
}
impl GenieLamp {
    pub fn new(value: f32) -> Self {
        Self {
            value,
            rubbing: false,
            last_rubbed: f64::NEG_INFINITY,
        }
    }
    /// Update. `rubbed` is a positive wash trigger repairValue, which the caller resets to 0.
    pub fn update(
        &mut self,
        r: &GenieLampReference,
        rubbed: bool,
        time: f64,
        dt: f32,
        sleep_modifier: f32,
    ) -> LampOutput {
        if rubbed {
            self.rubbing = true;
            self.last_rubbed = time;
        } else if time - self.last_rubbed >= RUB_HOLD {
            self.rubbing = false;
        }
        if self.rubbing {
            if self.value < 1. {
                self.value = (self.value + r.gain * dt * sleep_modifier).min(1.);
            }
        } else if self.value > 0. {
            self.value = (self.value - r.lose * dt * sleep_modifier).max(0.);
        }
        output(r, self.value)
    }
}
pub fn output(r: &GenieLampReference, value: f32) -> LampOutput {
    LampOutput {
        intensity: crate::simulation::engine::torque(&r.intensity, value),
        range: crate::simulation::engine::torque(&r.range, value),
        emission: crate::lighting::gradient(&r.emission, value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reference::{
        CurveKey, GradientAlpha, GradientColor, GradientReference, SourceId, TorqueCurve,
    };

    fn line(to: f32) -> TorqueCurve {
        let key = |time: f32, value: f32| CurveKey {
            time,
            value,
            in_tangent: to,
            out_tangent: to,
            in_weight: 0.,
            out_weight: 0.,
            weighted_mode: 0,
        };
        TorqueCurve {
            keys: vec![key(0., 0.), key(1., to)],
            pre_wrap_mode: 8,
            post_wrap_mode: 8,
        }
    }
    /// Representative ginlampscript values. No export with small_items is in the cloud.
    fn lamp() -> GenieLampReference {
        let s = |path_id| SourceId {
            file: "sharedassets1.assets".into(),
            path_id,
        };
        GenieLampReference {
            item: 211,
            prototype: s(1),
            source: s(152760),
            light: s(2),
            renderer: s(3),
            material: s(4),
            wash_trigger: s(5),
            gain: 0.5,
            lose: 0.1,
            intensity: line(2.),
            range: line(5.),
            emission: GradientReference {
                mode: 0,
                colors: vec![
                    GradientColor {
                        time: 0.,
                        value: [0., 0., 0., 1.],
                    },
                    GradientColor {
                        time: 1.,
                        value: [1., 0.5, 0., 1.],
                    },
                ],
                alphas: vec![GradientAlpha {
                    time: 0.,
                    value: 1.,
                }],
            },
            startup_value: 0.,
        }
    }

    #[test]
    fn lamp_rises_holds_and_falls() {
        let r = lamp();
        let mut l = GenieLamp::new(r.startup_value);
        let dt = 0.1;
        let mut time = 0.;
        // One rub: the lamp counts as rubbed for one second.
        l.update(&r, true, time, dt, 1.);
        assert!((l.value - 0.05).abs() < 1e-6);
        for _ in 0..9 {
            time += 0.1;
            l.update(&r, false, time, dt, 1.);
            assert!(l.rubbing);
        }
        assert!((l.value - 0.5).abs() < 1e-5);
        time += 0.1001;
        l.update(&r, false, time, dt, 1.);
        assert!(!l.rubbing);
        assert!((l.value - 0.49).abs() < 1e-5);
        // Rubbing saturates at 1; sleep scales the rate.
        for _ in 0..30 {
            time += 0.1;
            l.update(&r, true, time, dt, 2.);
        }
        assert_eq!(l.value, 1.);
        let out = output(&r, l.value);
        assert_eq!((out.intensity, out.range), (2., 5.));
        assert_eq!(out.emission, [1., 0.5, 0., 1.]);
        time += 2.;
        for _ in 0..200 {
            time += 0.1;
            l.update(&r, false, time, dt, 1.);
        }
        assert_eq!(l.value, 0.);
        assert_eq!(output(&r, 0.).intensity, 0.);
    }
}
