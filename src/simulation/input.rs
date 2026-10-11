use crate::reference::SeatId;
use crate::simulation::{FIXED_DT, keys};

/// Source keyboard steering at the fixed gameplay step.
pub struct KeyboardSteering {
    value: f32,
    increment: f32,
    seat: Option<SeatId>,
}

impl KeyboardSteering {
    pub fn new(rate: f32) -> Self {
        Self {
            value: 0.0,
            increment: rate * FIXED_DT,
            seat: None,
        }
    }

    pub fn clear(&mut self) {
        self.value = 0.0;
        self.seat = None;
    }

    pub fn step(&mut self, seat: Option<SeatId>, held: u32) -> f32 {
        if self.seat != seat {
            self.clear();
            self.seat = seat;
        }
        if seat.is_none() {
            return 0.0;
        }
        let target =
            ((held & keys::LEFT != 0) as u8 as f32) - ((held & keys::RIGHT != 0) as u8 as f32);
        self.value += (target - self.value).clamp(-self.increment, self.increment);
        self.value
    }
}
