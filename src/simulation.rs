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
    pub const FILL: u32 = 512;
    pub const IGNITION: u32 = 1024;
    pub const CAMERA: u32 = 2048;
    pub const ROTATE: u32 = 4096;
    pub const ASSEMBLE: u32 = 8192;
    pub const TRANSLATE: u32 = 16384;
    pub const RECENTER: u32 = 32768;
    pub const LIGHTS: u32 = 65536;
    pub const HORN: u32 = 131072;
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

/// Retains only the fraction after at most six fixed steps per frame.
#[derive(Default)]
pub struct StepClock {
    accumulator: f64,
}

impl StepClock {
    pub fn advance(&mut self, elapsed_seconds: f64) -> usize {
        if elapsed_seconds.is_finite() {
            self.accumulator += elapsed_seconds.clamp(0.0, MAX_FRAME_SECONDS);
        }
        let steps = ((self.accumulator + 1.0e-12) / STEP_SECONDS).floor() as usize;
        self.accumulator = (self.accumulator - steps as f64 * STEP_SECONDS).max(0.0);
        steps.min(6)
    }

    pub fn alpha(&self) -> f32 {
        (self.accumulator / STEP_SECONDS) as f32
    }
}
