//! Source spricniscript trigger and ammoscript count.
use crate::reference::{AcquisitionAmmo, SprayBottleReference};

/// Mathf.Lerp clamps its factor.
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0., 1.)
}

/// spricniscript currentAngle, lastAngle, triggered, and pressing.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SprayTrigger {
    pub angle: f32,
    last: f32,
    triggered: bool,
    pressing: bool,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TriggerEvents {
    /// Fire runs. It still checks the ammo.
    pub fire: bool,
    /// The trigger re-arms and plays the reset clip.
    pub reset: bool,
}
impl SprayTrigger {
    /// Pressing: the trigger eases toward its maximum angle.
    pub fn press(&mut self, r: &SprayBottleReference, dt: f32) {
        self.pressing = true;
        self.angle = lerp(self.angle, r.trigger_max_angle, r.angle_speed * dt);
    }
    /// Update: release easing, the fire edge, and the re-arm.
    pub fn update(&mut self, r: &SprayBottleReference, dt: f32) -> TriggerEvents {
        if !self.pressing {
            self.angle = lerp(self.angle, 0., r.angle_speed * dt);
        }
        let mut events = TriggerEvents::default();
        if self.angle >= r.trigger_set_angle && self.last < self.angle && !self.triggered {
            self.triggered = true;
            events.fire = true;
        }
        if self.triggered && self.angle <= r.trigger_reset_angle {
            self.triggered = false;
            events.reset = true;
        }
        self.last = self.angle;
        self.pressing = false;
        events
    }
    /// The trigger node local Euler X angle in degrees.
    pub fn euler_x(&self, r: &SprayBottleReference) -> f32 {
        if r.invert { -self.angle } else { self.angle }
    }
}

/// ammoscript: an int or float count of one ammo type.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ammo {
    pub count: f32,
    pub maximum: f32,
    pub float: bool,
    pub kind: u32,
}
impl Ammo {
    pub fn new(r: &AcquisitionAmmo) -> Self {
        Self {
            count: if r.float_or_int {
                r.float_count
            } else {
                r.count as f32
            },
            maximum: r.maximum,
            float: r.float_or_int,
            kind: r.r#type,
        }
    }
    /// The count after UpdAmmoFromMulti clamping, or None when it is not valid.
    pub fn with_count(self, count: f32) -> Option<Self> {
        (count.is_finite()
            && (0. ..=self.maximum).contains(&count)
            && (self.float || count.fract() == 0.))
            .then_some(Self { count, ..self })
    }
    /// Decrease: the int count drops by the truncated value.
    pub fn decrease(&mut self, value: f32) -> f32 {
        let value = value.clamp(0., self.count.max(0.));
        self.count -= if self.float { value } else { value.trunc() };
        value
    }
    /// CanReload without the weapon and magazine paths.
    pub fn can_reload(&self, from: &Ammo) -> bool {
        self.kind == from.kind && self.count < self.maximum && from.count > 0.
    }
    /// Reload: take up to `value` from a source of the same type.
    pub fn reload(&mut self, from: &mut Ammo, value: f32) {
        if from.kind != self.kind || from.count <= 0. || self.count >= self.maximum {
            return;
        }
        let value = value
            .clamp(0., self.maximum - self.count)
            .clamp(0., from.count);
        self.count += if self.float { value } else { value.trunc() };
        from.decrease(value);
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::reference::{ParticleReference, SourceId};

    fn source(path_id: i64) -> SourceId {
        SourceId {
            file: "sharedassets1.assets".into(),
            path_id,
        }
    }
    /// Representative spricniscript values. No export with small_items is in the cloud.
    pub fn bottle(particle: ParticleReference) -> SprayBottleReference {
        SprayBottleReference {
            item: 121,
            prototype: source(1),
            source: source(142096),
            ammo: AcquisitionAmmo {
                source: source(2),
                count: 100,
                float_count: 0.,
                maximum: 100.,
                float_or_int: false,
                r#type: 7,
                random_range: [10., 100.],
            },
            can_pour: false,
            pour_time: 0.,
            cleaning_type: 0,
            text: 185,
            angle_speed: 10.,
            trigger_max_angle: 30.,
            trigger_set_angle: 20.,
            trigger_reset_angle: 5.,
            invert: true,
            trigger: source(3),
            hole: source(4),
            set_clip: source(5),
            reset_clip: source(6),
            set_volume: 1.,
            reset_volume: 1.,
            mask: 1,
            gain: 0.2,
            distance: 2.,
            wash_radius: 0.2,
            wash_mask: 1,
            spray: particle.clone(),
            splash: particle,
            startup: Vec::new(),
        }
    }
    pub fn particle() -> ParticleReference {
        ParticleReference {
            source: source(9),
            role: "spray_bottle_spray".into(),
            capacity: 8,
            rate_over_time: 0.,
            rate_over_distance: 0.,
            duration: 1.,
            simulation_space: 1,
            local_pose: crate::reference::Transform {
                translation: [0.; 3],
                rotation: [0., 0., 0., 1.],
                scale: [1.; 3],
            },
            module_flags: Default::default(),
            curves: Default::default(),
            colors: Default::default(),
            vectors: Default::default(),
            scalars: Default::default(),
            flags: Default::default(),
            links: Default::default(),
            materials: Vec::new(),
            limits: Vec::new(),
        }
    }
    fn test_bottle() -> SprayBottleReference {
        bottle(particle())
    }

    #[test]
    fn one_shot_per_press() {
        let r = test_bottle();
        let mut t = SprayTrigger::default();
        let (mut fires, mut resets) = (0, 0);
        for press in 0..3 {
            for _ in 0..60 {
                t.press(&r, 1. / 60.);
                let e = t.update(&r, 1. / 60.);
                fires += e.fire as u32;
                resets += e.reset as u32;
            }
            assert_eq!(fires, press + 1, "held trigger fires once");
            assert!(t.euler_x(&r) < -r.trigger_set_angle);
            for _ in 0..120 {
                let e = t.update(&r, 1. / 60.);
                assert!(!e.fire);
                resets += e.reset as u32;
            }
            assert_eq!(resets, press + 1, "release re-arms once");
        }
        // A short tap that stays below the set angle never fires.
        let mut t = SprayTrigger::default();
        t.press(&r, 1. / 60.);
        assert!(!t.update(&r, 1. / 60.).fire);
    }

    #[test]
    fn ammo_follows_ammoscript() {
        let r = test_bottle();
        let mut a = Ammo::new(&r.ammo);
        assert_eq!(a.count, 100.);
        a.count = 0.;
        assert_eq!(a.decrease(1.), 0., "an empty bottle does not shoot");
        assert_eq!(a.count, 0.);
        let mut source = Ammo { count: 3., ..a };
        assert!(a.can_reload(&source));
        a.reload(&mut source, 1.);
        assert_eq!((a.count, source.count), (1., 2.));
        let mut other = Ammo { kind: 5, ..source };
        a.reload(&mut other, 1.);
        assert_eq!(a.count, 1., "other ammo types do not reload");
        let mut full = Ammo { count: 100., ..a };
        assert!(!full.can_reload(&source));
        full.reload(&mut source, 1.);
        assert_eq!(
            (full.count, source.count),
            (100., 2.),
            "a full bottle takes nothing"
        );
        let mut almost = Ammo {
            count: 99.5,
            float: true,
            ..a
        };
        let mut float_source = Ammo {
            float: true,
            ..source
        };
        almost.reload(&mut float_source, 1.);
        assert_eq!(
            (almost.count, float_source.count),
            (100., 1.5),
            "the move clamps at max"
        );
        assert!(a.with_count(0.5).is_none() && a.with_count(101.).is_none());
        assert_eq!(a.with_count(40.).unwrap().count, 40.);
    }
}
