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

#[derive(Clone, Copy, Debug)]
pub struct FuelState {
    pub liters: f32,
    pub capacity_liters: f32,
}

impl Default for FuelState {
    fn default() -> Self {
        Self {
            liters: 12.0,
            capacity_liters: 40.0,
        }
    }
}

impl FuelState {
    pub fn consume(&mut self, throttle: f32, dt: f32) {
        self.liters = (self.liters - (0.001 + 0.009 * throttle.clamp(0.0, 1.0)) * dt.max(0.0))
            .clamp(0.0, self.capacity_liters);
    }

    pub fn refuel(&mut self, dt: f32) {
        self.liters = (self.liters + 4.0 * dt.max(0.0)).clamp(0.0, self.capacity_liters);
    }
}

pub fn can_refuel(speed_kmh: f32, distance_m: f32) -> bool {
    speed_kmh.abs() < 1.0 && distance_m <= 6.0
}

#[derive(Default, Debug)]
pub struct DriveState {
    pub fuel: FuelState,
    pub trip_m: f64,
    pub paused: bool,
    pub started: bool,
}

impl DriveState {
    pub fn active(&self) -> bool {
        self.started && !self.paused
    }

    pub fn step(
        &mut self,
        world: &mut crate::vehicle::PhysicsWorld,
        input: DriveInput,
        refuel: bool,
        station_distance: f32,
    ) {
        if !self.active() {
            return;
        }
        let snapshot = world.snapshot();
        let speed_kmh = snapshot.speed_mps * 3.6;
        if self.fuel.liters > 0.0 {
            let throttle = input.accelerate || (input.brake_reverse && speed_kmh < 1.44);
            self.fuel.consume(throttle as u8 as f32, FIXED_DT);
        }
        if refuel && can_refuel(snapshot.absolute_speed_kmh(), station_distance) {
            self.fuel.refuel(FIXED_DT);
        }
        world.step(input, self.fuel.liters > 0.0);
    }

    pub fn record_travel(&mut self, previous: [f64; 3], current: [f64; 3], teleported: bool) {
        if self.active() && !teleported {
            self.trip_m +=
                ((current[0] - previous[0]).powi(2) + (current[2] - previous[2]).powi(2)).sqrt();
        }
    }

    pub fn restart(&mut self) {
        *self = Self {
            started: true,
            ..Self::default()
        };
    }
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
