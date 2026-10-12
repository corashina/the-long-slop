pub const FIXED_DT: f32 = 1.0 / 60.0;
pub mod engine;
pub mod input;
pub mod instruments;
const MAX_FRAME_SECONDS: f64 = 0.1;
const STEP_SECONDS: f64 = 1.0 / 60.0;

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
    pub const SLEEP: u32 = 1 << 17;
    pub const URINATE: u32 = 1 << 18;
    pub const BURP: u32 = 1 << 19;
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FrameInput {
    pub held: u32,
    pub pressed: u32,
    pub released: u32,
    pub mouse: [f32; 2],
    pub wheel: f32,
}

impl FrameInput {
    pub fn merge(&mut self, frame: Self) {
        self.held = frame.held;
        self.pressed |= frame.pressed;
        self.released |= frame.released;
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
    pub accelerate: bool,
    pub brake_reverse: bool,
    pub steer: f32,
    pub handbrake: bool,
}

pub fn braking(input: DriveInput, direction: f32, auto_move_min: f32) -> bool {
    (input.brake_reverse && direction > auto_move_min)
        || (input.accelerate && direction < -auto_move_min)
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
        let budget = if self.touched_sleep { 12 } else { 6 };
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
