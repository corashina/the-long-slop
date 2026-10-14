use crate::reference::SeatId;
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
    seat: Option<SeatId>,
}

impl KeyboardDriving {
    pub fn new(steering_rate: f32, pedal_rate: f32) -> Self {
        Self {
            controls: DrivingControls::default(),
            steering_increment: steering_rate * FIXED_DT,
            pedal_increment: pedal_rate * FIXED_DT,
            seat: None,
        }
    }

    pub fn clear(&mut self) {
        self.controls = DrivingControls::default();
        self.seat = None;
    }

    pub fn step(&mut self, seat: Option<SeatId>, held: u32) -> DrivingControls {
        if self.seat != seat {
            self.clear();
            self.seat = seat;
        }
        if seat.is_none() {
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
