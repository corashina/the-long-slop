pub const FIXED_DT: f32 = 1.0 / 60.0;
const MAX_FRAME_SECONDS: f64 = 0.1;
const STEP_SECONDS: f64 = 1.0 / 60.0;

#[derive(Clone, Copy, Debug, Default)]
pub struct DriveInput {
    pub accelerate: bool,
    pub brake_reverse: bool,
    pub steer: f32,
    pub handbrake: bool,
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
