//! Source `cigiscript` packs, `ediblescript` smoke burn, `emberScript`, and player `smokeCharged`.
use crate::reference::{
    CigaretteConsumptionReference, ConsumptionEffect, ExhaleReference, Transform,
};

/// `emberScript` values of the checked cigarette record.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EmberRates {
    /// Start, min, max, and end values.
    pub colors: [[f32; 4]; 4],
    pub light_intensities: [f32; 4],
    pub particle_rates: [f32; 4],
    pub speed_up: f32,
    pub speed_down: f32,
    pub lerp: f32,
}

/// `ediblescript` values of the checked cigarette record.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BurnRates {
    pub startup_hp: f32,
    pub ate_once: bool,
    pub automatic: bool,
    pub ember_in_auto: bool,
    pub eat_speed: f32,
    pub auto_eat_speed: f32,
    pub effect: ConsumptionEffect,
    pub parts: usize,
    pub ember: EmberRates,
}

impl From<&CigaretteConsumptionReference> for BurnRates {
    fn from(r: &CigaretteConsumptionReference) -> Self {
        let e = &r.ember;
        Self {
            startup_hp: r.startup_hp,
            ate_once: r.ate_once,
            automatic: r.automatic,
            ember_in_auto: r.ember_in_auto,
            eat_speed: r.eat_speed,
            auto_eat_speed: r.auto_eat_speed,
            effect: r.effect,
            parts: r.parts.len(),
            ember: EmberRates {
                colors: e.colors,
                light_intensities: e.light_intensities,
                particle_rates: e.particle_rates,
                speed_up: e.speed_up,
                speed_down: e.speed_down,
                lerp: e.lerp,
            },
        }
    }
}

/// Player mouth smoke values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExhaleRates {
    pub max_rate: f32,
    pub charge_speed: f32,
}

impl From<&ExhaleReference> for ExhaleRates {
    fn from(r: &ExhaleReference) -> Self {
        Self {
            max_rate: r.max_rate,
            charge_speed: r.charge_speed,
        }
    }
}

/// Values that `emberScript` last wrote.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmberPhase {
    /// `STart`: start color, light, and particle values.
    Start,
    /// `Upd`: values lerped from min to max.
    Lerp,
    /// `End`: end values. The script is disabled.
    End,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EmberState {
    pub lerp: f32,
    pub phase: EmberPhase,
    up: bool,
    done: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EmberOutput {
    /// `_EmissionColor` as `SetColor` receives it.
    pub color: [f32; 4],
    pub light_intensity: f32,
    pub particle_rate: f32,
}

fn mix(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

impl EmberState {
    pub fn new(r: &EmberRates) -> Self {
        Self {
            lerp: r.lerp,
            phase: EmberPhase::Start,
            up: false,
            done: true,
        }
    }
    /// `emberScript.Up`.
    pub fn up(&mut self, r: &EmberRates, dt: f32) {
        if self.phase == EmberPhase::End {
            return;
        }
        self.done = false;
        self.up = true;
        self.lerp = (self.lerp + r.speed_up * dt).clamp(0., 1.);
        self.phase = EmberPhase::Lerp;
    }
    /// `emberScript.End`.
    pub fn end(&mut self) {
        self.phase = EmberPhase::End;
    }
    /// `emberScript.Update`.
    pub fn update(&mut self, r: &EmberRates, dt: f32) {
        if self.done || self.phase == EmberPhase::End {
            return;
        }
        if self.up {
            self.up = false;
            return;
        }
        self.lerp = (self.lerp - r.speed_down * dt).clamp(0., 1.);
        self.phase = EmberPhase::Lerp;
        if self.lerp <= 0. {
            self.done = true;
        }
    }
    pub fn output(&self, r: &EmberRates) -> EmberOutput {
        // Records hold start, min, max, and end values in that order.
        let pick = |v: [f32; 4]| match self.phase {
            EmberPhase::Start => v[0],
            EmberPhase::Lerp => mix(v[1], v[2], self.lerp),
            EmberPhase::End => v[3],
        };
        EmberOutput {
            color: match self.phase {
                EmberPhase::Start => r.colors[0],
                EmberPhase::Lerp => {
                    std::array::from_fn(|i| mix(r.colors[1][i], r.colors[2][i], self.lerp))
                }
                EmberPhase::End => r.colors[3],
            },
            light_intensity: pick(r.light_intensities),
            particle_rate: pick(r.particle_rates),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CigaretteState {
    pub hp: f32,
    pub ate_once: bool,
    pub ember: EmberState,
}

impl CigaretteState {
    pub fn new(r: &BurnRates) -> Self {
        Self {
            hp: r.startup_hp,
            ate_once: r.ate_once,
            ember: EmberState::new(&r.ember),
        }
    }
    /// Saved `hp` and `ateOnce`. The source load path calls `ember.Up` once for a lit cigarette.
    pub fn restored(r: &BurnRates, hp: f32, ate_once: bool, dt: f32) -> Self {
        let mut state = Self {
            hp: hp.clamp(0., 1.),
            ate_once,
            ember: EmberState::new(&r.ember),
        };
        if ate_once {
            state.ember.up(&r.ember, dt);
        }
        state
    }
    /// `ediblescript.Eat` for a continuous item. Returns the survival effect of this call.
    pub fn eat(&mut self, r: &BurnRates, dt: f32, auto: bool) -> ConsumptionEffect {
        if self.hp <= 0. {
            return ConsumptionEffect::default();
        }
        self.ate_once = true;
        let speed = if auto { r.auto_eat_speed } else { r.eat_speed };
        self.hp = (self.hp - speed * dt).clamp(0., 1.);
        if !auto || r.ember_in_auto {
            self.ember.up(&r.ember, dt);
            if self.hp <= 0. {
                self.ember.end();
            }
        }
        // Each call returns the record values divided by the part count.
        let parts = r.parts as f32;
        ConsumptionEffect {
            food: r.effect.food / parts,
            water: r.effect.water / parts,
            bowel: r.effect.bowel / parts,
            hp: r.effect.hp / parts,
        }
    }
    /// `ediblescript.Update` and `emberScript.Update`. The automatic burn has no survival effect.
    pub fn update(&mut self, r: &BurnRates, dt: f32) {
        if r.automatic && self.ate_once {
            self.eat(r, dt, true);
        }
        self.ember.update(&r.ember, dt);
    }
    /// `UpdParts`: part poses lerp from end to start by hp.
    pub fn part_blend(&self) -> f32 {
        self.hp
    }
}

/// `UpdParts`: Lerp(end, start, hp) of position and scale, and Quaternion.Lerp of rotation.
pub fn part_pose(start: &Transform, end: &Transform, hp: f32) -> Transform {
    let lerp3 = |a: [f32; 3], b: [f32; 3]| std::array::from_fn(|i| mix(a[i], b[i], hp));
    let (a, mut b) = (end.rotation, start.rotation);
    // Quaternion.Lerp takes the shorter arc and normalizes.
    if (0..4).map(|i| a[i] * b[i]).sum::<f32>() < 0. {
        b = b.map(|v| -v);
    }
    let q: [f32; 4] = std::array::from_fn(|i| mix(a[i], b[i], hp));
    let length = q.iter().map(|v| v * v).sum::<f32>().sqrt();
    Transform {
        translation: lerp3(end.translation, start.translation),
        rotation: q.map(|v| v / length),
        scale: lerp3(end.scale, start.scale),
    }
}

/// `ammoscript` integer count of a cigarette pack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PackState {
    pub count: u32,
}

impl PackState {
    /// The "take one" action shows while the count is above 0.
    pub fn can_take(&self) -> bool {
        self.count > 0
    }
    /// `SetIntAmmo(GetAmmo() - 1)`. The caller then spawns a cigarette at the pack pose.
    pub fn take_one(&mut self) -> bool {
        if !self.can_take() {
            return false;
        }
        self.count -= 1;
        true
    }
}

/// Player `smokeCharged` and mouth smoke emission.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ExhaleState {
    pub charged: f32,
}

impl ExhaleState {
    /// Charge while a lit cigarette is smoked.
    pub fn charge(&mut self, r: &ExhaleRates, dt: f32) {
        self.charged += dt * r.charge_speed;
    }
    /// Emission after this frame's input. `None` disables the mouth smoke.
    /// `blocked` is `isSmoking || input.smoke || input.lmb`.
    pub fn emit(&mut self, r: &ExhaleRates, blocked: bool, dt: f32) -> Option<f32> {
        if blocked || self.charged <= 0. {
            return None;
        }
        self.charged = self.charged.min(1.);
        let rate = self.charged * r.max_rate;
        self.charged -= dt * r.charge_speed * 2.;
        Some(rate)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// Item 230 values from the checked cigarette record.
    pub fn cigarette() -> BurnRates {
        BurnRates {
            startup_hp: 1.,
            ate_once: false,
            automatic: true,
            ember_in_auto: false,
            eat_speed: 0.019999999552965164,
            auto_eat_speed: 0.0020833334419876337,
            effect: ConsumptionEffect {
                food: -0.00019999999494757503,
                water: -0.00019999999494757503,
                bowel: 0.00019999999494757503,
                hp: 0.,
            },
            parts: 2,
            ember: EmberRates {
                colors: [[0.; 4], [1., 0.2, 0., 1.], [4., 1., 0., 1.], [0.; 4]],
                light_intensities: [0., 0.10000000149011612, 0.25, 0.],
                particle_rates: [0., 100., 0., 0.],
                speed_up: 1.,
                speed_down: 1.,
                lerp: 0.,
            },
        }
    }

    #[test]
    fn manual_burn_uses_eat_speed_and_part_divided_effects() {
        let r = cigarette();
        let mut c = CigaretteState::new(&r);
        assert_eq!(c.hp, 1.);
        let effect = c.eat(&r, 0.5, false);
        assert!((c.hp - (1. - 0.019999999552965164 * 0.5)).abs() < 1e-6);
        assert!(c.ate_once);
        assert_eq!(
            effect,
            ConsumptionEffect {
                food: -0.00019999999494757503 / 2.,
                water: -0.00019999999494757503 / 2.,
                bowel: 0.00019999999494757503 / 2.,
                hp: 0.,
            }
        );
        assert_eq!(c.ember.phase, EmberPhase::Lerp);
        assert!((c.ember.lerp - 0.5).abs() < 1e-6);
    }

    #[test]
    fn automatic_burn_starts_after_first_puff_without_ember() {
        let r = cigarette();
        let mut c = CigaretteState::new(&r);
        c.update(&r, 10.);
        assert_eq!(c.hp, 1.);
        c.eat(&r, 0.02, false);
        let hp = c.hp;
        let lerp = c.ember.lerp;
        c.update(&r, 1.);
        assert!((c.hp - (hp - 0.0020833334419876337)).abs() < 1e-6);
        // The update after Up skips the decay once.
        assert_eq!(c.ember.lerp, lerp);
        c.update(&r, 0.01);
        assert!(c.ember.lerp < lerp);
    }

    #[test]
    fn burnt_cigarette_stays_as_butt_with_end_ember() {
        let r = cigarette();
        let mut c = CigaretteState::new(&r);
        c.eat(&r, 100., false);
        assert_eq!(c.hp, 0.);
        assert_eq!(c.ember.phase, EmberPhase::End);
        assert_eq!(c.eat(&r, 1., false), ConsumptionEffect::default());
        c.update(&r, 1.);
        assert_eq!(c.hp, 0.);
        let out = c.ember.output(&r.ember);
        assert_eq!(out.light_intensity, r.ember.light_intensities[3]);
        assert_eq!(out.color, r.ember.colors[3]);
    }

    #[test]
    fn ember_lerps_between_min_and_max_and_decays_to_min() {
        let e = cigarette().ember;
        let mut s = EmberState::new(&e);
        assert_eq!(s.output(&e).light_intensity, e.light_intensities[0]);
        s.up(&e, 0.5);
        let out = s.output(&e);
        assert!((out.light_intensity - (0.1 + (0.25 - 0.1) * 0.5)).abs() < 1e-6);
        assert!((out.particle_rate - 50.).abs() < 1e-4);
        s.update(&e, 0.1);
        s.update(&e, 2.);
        assert_eq!(s.lerp, 0.);
        assert_eq!(s.output(&e).light_intensity, e.light_intensities[1]);
        s.update(&e, 1.);
        assert_eq!(s.phase, EmberPhase::Lerp);
    }

    #[test]
    fn ember_output_and_parts_follow_burn() {
        let r = cigarette();
        let mut c = CigaretteState::new(&r);
        let start = c.ember.output(&r.ember);
        assert_eq!(start.color, r.ember.colors[0]);
        assert_eq!(start.particle_rate, r.ember.particle_rates[0]);
        c.eat(&r, 0.25, false);
        let lerp = c.ember.output(&r.ember);
        assert!((lerp.color[0] - (1. + (4. - 1.) * 0.25)).abs() < 1e-6);
        assert!((lerp.particle_rate - 75.).abs() < 1e-4);
        let full = Transform {
            translation: [0., 0., -0.03],
            rotation: [0., -0.70710677, 0.70710677, 0.],
            scale: [0.008, 0.03, 0.008],
        };
        let empty = Transform {
            translation: [0., 0., 0.03],
            scale: [0.008, 0.0007, 0.008],
            ..full
        };
        assert_eq!(part_pose(&full, &empty, 1.).translation, full.translation);
        let half = part_pose(&full, &empty, 0.5);
        assert!(half.translation[2].abs() < 1e-6);
        assert!((half.scale[1] - (0.03 + 0.0007) / 2.).abs() < 1e-6);
        assert!((half.rotation[2] - 0.70710677).abs() < 1e-5);
        assert_eq!(part_pose(&full, &empty, 0.).scale, empty.scale);
        c.eat(&r, 100., false);
        assert_eq!(c.part_blend(), 0.);
        let end = c.ember.output(&r.ember);
        assert_eq!(end.color, r.ember.colors[3]);
        assert_eq!(end.light_intensity, r.ember.light_intensities[3]);
    }

    #[test]
    fn pack_count_decrements_to_zero() {
        let mut pack = PackState { count: 2 };
        assert!(pack.take_one() && pack.take_one());
        assert_eq!(pack.count, 0);
        assert!(!pack.can_take() && !pack.take_one());
        assert_eq!(pack.count, 0);
    }

    fn exhale() -> ExhaleRates {
        ExhaleRates {
            max_rate: 30.,
            charge_speed: 0.5,
        }
    }

    #[test]
    fn smoke_charge_clamps_and_decays_at_twice_the_rate() {
        let r = exhale();
        let mut s = ExhaleState::default();
        s.charge(&r, 4.);
        assert_eq!(s.charged, 2.);
        assert_eq!(s.emit(&r, true, 0.1), None);
        assert_eq!(s.charged, 2.);
        assert_eq!(s.emit(&r, false, 0.1), Some(30.));
        assert!((s.charged - 0.9).abs() < 1e-6);
        assert_eq!(s.emit(&r, false, 1.), Some(0.9 * 30.));
        assert!(s.charged < 0.);
        assert_eq!(s.emit(&r, false, 1.), None);
    }
}
