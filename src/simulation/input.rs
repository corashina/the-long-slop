use crate::simulation::{FIXED_DT, keys};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DrivingControls {
    pub steer: f32,
    pub throttle: f32,
    pub brake: f32,
    pub clutch: f32,
}

/// Source keyboard controls at the fixed gameplay step.
pub struct KeyboardDriving {
    controls: DrivingControls,
    steering_increment: f32,
    pedal_increment: f32,
    driving: bool,
}

impl KeyboardDriving {
    pub fn new(steering_rate: f32, pedal_rate: f32) -> Self {
        Self {
            controls: DrivingControls::default(),
            steering_increment: steering_rate * FIXED_DT,
            pedal_increment: pedal_rate * FIXED_DT,
            driving: false,
        }
    }

    pub fn clear(&mut self) {
        self.controls = DrivingControls::default();
        self.driving = false;
    }

    /// The caller clears the controls on a seat change.
    pub fn step(&mut self, driving: bool, held: u32) -> DrivingControls {
        if self.driving != driving {
            self.clear();
            self.driving = driving;
        }
        if !driving {
            return self.controls;
        }
        let steer =
            ((held & keys::LEFT != 0) as u8 as f32) - ((held & keys::RIGHT != 0) as u8 as f32);
        self.controls.steer +=
            (steer - self.controls.steer).clamp(-self.steering_increment, self.steering_increment);
        for (value, bit) in [
            (&mut self.controls.throttle, keys::FORWARD),
            (&mut self.controls.brake, keys::BACK),
            (&mut self.controls.clutch, keys::CLUTCH),
        ] {
            let target = (held & bit != 0) as u8 as f32;
            *value += (target - *value).clamp(-self.pedal_increment, self.pedal_increment);
        }
        self.controls
    }
}
