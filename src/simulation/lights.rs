//! Source car lights: `carscript.lampak` and `headlightscript`.

use crate::reference::{HeadlightReference, ItemId};

/// `carscript.InteractSwitchLights` cycles off, low beam, and high beam.
pub const LAMP_STATES: usize = 3;

#[derive(Clone, Debug)]
pub struct Headlight {
    pub item: ItemId,
    pub current: usize,
    switch_to: usize,
    pub energy: f32,
    rate: f32,
}

impl Headlight {
    pub fn new(reference: &HeadlightReference) -> Self {
        Self {
            item: reference.item,
            current: 0,
            switch_to: 0,
            energy: 0.0,
            rate: reference.energy_rate,
        }
    }

    /// `headlightscript.SwitchState(int)`.
    pub fn switch(&mut self, state: usize) {
        if state != self.switch_to && state != self.current {
            self.switch_to = state;
        }
    }

    /// `headlightscript.Update`: the light fades out, changes state, then fades in.
    pub fn step(&mut self, dt: f32) {
        if self.switch_to == self.current {
            self.energy = (self.energy + dt * self.rate).min(1.0);
        } else if self.energy > 0.0 {
            self.energy = (self.energy - dt * self.rate).max(0.0);
        } else {
            self.current = self.switch_to;
        }
    }
}
