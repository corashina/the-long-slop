pub const FIXED_DT: f32 = 0.01999999955;
pub mod drivetrain;
pub mod engine;
pub mod input;
pub mod instruments;
pub mod radio;
const MAX_FRAME_SECONDS: f64 = 0.1;
const STEP_SECONDS: f64 = FIXED_DT as f64;

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
    pub const ASSEMBLE: u32 = 8192;
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

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FrameInput {
    pub held: u32,
    pub pressed: u32,
    pub released: u32,
    pub inventory_held: u8,
    pub inventory_released: u8,
    pub mouse: [f32; 2],
    pub wheel: f32,
}

impl FrameInput {
    pub fn merge(&mut self, frame: Self) {
        self.held = frame.held;
        self.pressed |= frame.pressed;
        self.released |= frame.released;
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
    }

    pub fn take_step(&mut self) -> Self {
        let step = *self;
        *self = Self {
            held: step.held,
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
        let budget = if self.touched_sleep { 12 } else { 5 };
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
    fn normal_clock_advances_five_steps_per_tenth_second() {
        let mut clock = StepClock::default();
        clock.accept_frame(0.1);

        for _ in 0..5 {
            assert!(clock.take_step(1.));
        }
        assert!(!clock.take_step(1.));
    }

    #[test]
    fn sleep_clock_keeps_double_time_scale_bounded() {
        let mut clock = StepClock::default();
        clock.accept_frame(0.1);

        for _ in 0..10 {
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
