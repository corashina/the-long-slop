pub const FIXED_DT: f32 = 0.01999999955;
pub mod car_lights;
pub mod dials;
pub mod drivetrain;
pub mod engine;
pub mod ignition;
pub mod input;
pub mod instruments;
pub mod metal_detector;
pub mod music_box;
pub mod powertrain;
pub mod radio;
pub mod sled;
const MAX_FRAME_SECONDS: f64 = 0.1;
const STEP_SECONDS: f64 = FIXED_DT as f64;
/// Steps per frame at time scale 1. Slow frames drop extra time, so step cost cannot grow frame time.
const STEP_BUDGET: usize = 2;

pub mod keys {
    pub const FORWARD: u32 = 1;
    pub const BACK: u32 = 2;
    pub const LEFT: u32 = 4;
    pub const RIGHT: u32 = 8;
    pub const SPACE: u32 = 16;
    pub const USE: u32 = 32;
    pub const RUN: u32 = 64;
    pub const CROUCH: u32 = 128;
    pub const GRAB: u32 = 256;
    pub const PRIMARY: u32 = 512;
    pub const IGNITION: u32 = 1024;
    pub const CAMERA: u32 = 2048;
    pub const ROTATE: u32 = 4096;
    /// Source `input.reload` (R).
    pub const RELOAD: u32 = 8192;
    pub const TRANSLATE: u32 = 16384;
    pub const RECENTER: u32 = 32768;
    pub const LIGHTS: u32 = 65536;
    pub const HORN: u32 = 1 << 17;
    pub const SLEEP: u32 = 1 << 18;
    pub const URINATE: u32 = 1 << 19;
    pub const BURP: u32 = 1 << 20;
    pub const CLUTCH: u32 = 1 << 21;
    pub const SHIFT_UP: u32 = 1 << 22;
    pub const SHIFT_DOWN: u32 = 1 << 23;
    pub const REVERSE: u32 = 1 << 24;
    pub const NEUTRAL: u32 = 1 << 25;
    pub const GEAR_1: u32 = 1 << 26;
    pub const GEAR_2: u32 = 1 << 27;
    pub const GEAR_3: u32 = 1 << 28;
    pub const GEAR_4: u32 = 1 << 29;
    pub const GEAR_5: u32 = 1 << 30;
    pub const GEAR_6: u32 = 1 << 31;
}

pub mod player_keys {
    pub const LEAN: u8 = 1;
    pub const ZOOM: u8 = 2;
    /// Source `input.smoke` (Caps Lock).
    pub const SMOKE: u8 = 4;
    /// Source `input.removeSunglasses` (X).
    pub const TAKE_OFF: u8 = 8;
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CarryDeltas {
    pub owner: Option<crate::reference::ItemId>,
    pub mouse: [f32; 2],
    pub wheel: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FrameInput {
    pub held: u32,
    pub pressed: u32,
    pub released: u32,
    pub player_held: u8,
    pub player_pressed: u8,
    pub player_released: u8,
    pub inventory_held: u8,
    pub inventory_released: u8,
    pub mouse: [f32; 2],
    pub wheel: f32,
    pub carry: CarryDeltas,
}

impl FrameInput {
    pub fn merge(&mut self, frame: Self) {
        self.held = frame.held;
        self.pressed |= frame.pressed;
        self.released |= frame.released;
        self.player_held = frame.player_held;
        self.player_pressed |= frame.player_pressed;
        self.player_released |= frame.player_released;
        self.inventory_held = frame.inventory_held;
        self.inventory_released |= frame.inventory_released;
        for (pending, delta) in self.mouse.iter_mut().zip(frame.mouse) {
            if delta.is_finite() {
                *pending += delta;
            }
        }
        if frame.wheel.is_finite() {
            self.wheel += frame.wheel;
        }
        if self.carry.owner != frame.carry.owner {
            self.carry = CarryDeltas {
                owner: frame.carry.owner,
                ..CarryDeltas::default()
            };
        }
        for (pending, delta) in self.carry.mouse.iter_mut().zip(frame.carry.mouse) {
            if delta.is_finite() {
                *pending += delta;
            }
        }
        if frame.carry.wheel.is_finite() {
            self.carry.wheel += frame.carry.wheel;
        }
    }

    /// Retain active skipped input, cancel blocked skipped input, or consume a frame.
    #[cfg(any(target_arch = "wasm32", test))]
    pub(crate) fn take_frame(&mut self, skip_elapsed: &mut bool, blocked: bool) -> Option<Self> {
        if std::mem::take(skip_elapsed) {
            if blocked {
                self.clear();
            }
            return None;
        }
        Some(std::mem::take(self))
    }

    pub fn take_step(&mut self) -> Self {
        let step = *self;
        *self = Self {
            held: step.held,
            player_held: step.player_held,
            inventory_held: step.inventory_held,
            ..Self::default()
        };
        step
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveInput {
    pub steer: f32,
    pub throttle: f32,
    pub brake: f32,
    pub handbrake: f32,
}

/// Stores accepted real time. Each step spends its sampled scale once.
/// Below 25 FPS the simulation runs slower than real time.
#[derive(Default)]
pub struct StepClock {
    accumulator: f64,
    steps: usize,
    touched_sleep: bool,
}

impl StepClock {
    pub fn accept_frame(&mut self, elapsed_seconds: f64) {
        self.steps = 0;
        self.touched_sleep = false;
        if elapsed_seconds.is_finite() && elapsed_seconds >= 0. {
            self.accumulator += elapsed_seconds.min(MAX_FRAME_SECONDS);
        }
    }

    pub fn take_step(&mut self, time_scale: f32) -> bool {
        if !time_scale.is_finite() || time_scale <= 0. {
            return false;
        }
        self.touched_sleep |= time_scale > 1.;
        let cost = STEP_SECONDS / f64::from(time_scale);
        let budget = if self.touched_sleep {
            STEP_BUDGET * 2
        } else {
            STEP_BUDGET
        };
        if self.steps >= budget {
            self.accumulator %= cost;
            return false;
        }
        if self.accumulator + 1.0e-12 < cost {
            return false;
        }
        self.accumulator = (self.accumulator - cost).max(0.);
        self.steps += 1;
        true
    }

    pub fn alpha(&self, time_scale: f32) -> f32 {
        (self.accumulator * f64::from(time_scale) / STEP_SECONDS).clamp(0., 1.) as f32
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod step_clock_tests {
    use super::{FIXED_DT, StepClock};

    #[test]
    fn slow_frame_runs_two_steps_and_drops_extra_time() {
        let mut clock = StepClock::default();
        clock.accept_frame(0.1);

        for _ in 0..2 {
            assert!(clock.take_step(1.));
        }
        assert!(!clock.take_step(1.));
        clock.accept_frame(0.);
        assert!(!clock.take_step(1.));
    }

    #[test]
    fn sleep_clock_keeps_double_time_scale_bounded() {
        let mut clock = StepClock::default();
        clock.accept_frame(0.1);

        for _ in 0..4 {
            assert!(clock.take_step(2.));
        }
        assert!(!clock.take_step(2.));
    }

    #[test]
    fn fixed_steps_cover_one_simulation_second() {
        let mut clock = StepClock::default();
        let mut steps = 0;
        for _ in 0..50 {
            clock.accept_frame(f64::from(FIXED_DT));
            steps += usize::from(clock.take_step(1.));
        }

        assert_eq!(steps, 50);
        assert!((f64::from(FIXED_DT) * steps as f64 - 1.).abs() < 1.0e-6);
    }
}
