//! Switch requests and source fades advance in the fixed simulation step.
use crate::interaction::items::ItemStore;
use crate::interaction::mechanisms::{ControlAction, MechanismState};
use crate::reference::{LampControl, LightingReference};

#[derive(Clone, Copy, Debug)]
pub struct VehicleControlState {
    pub headlights: u8,
    pub interior_light_mode: u8,
    pub interior_light: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct LampState {
    pub current: u8,
    pub energy: f32,
    pub electronics_output: f32,
    pub on: bool,
}
pub struct LightingState {
    reference: LightingReference,
    pub controls: VehicleControlState,
    pub groups: Vec<LampState>,
}
impl LightingState {
    pub fn new(reference: &LightingReference) -> Self {
        Self {
            reference: reference.clone(),
            controls: VehicleControlState {
                headlights: reference.headlights,
                interior_light_mode: reference.interior_light_mode,
                interior_light: false,
            },
            groups: reference
                .groups
                .iter()
                .map(|g| LampState {
                    current: g.initial_state,
                    energy: g.initial_energy,
                    electronics_output: g.electronics_output,
                    on: g.initial_on,
                })
                .collect(),
        }
    }
    pub fn apply(&mut self, action: &ControlAction) {
        match action {
            ControlAction::CycleHeadlights => {
                self.controls.headlights = (self.controls.headlights + 1) % 3
            }
            ControlAction::InteriorLight(mode) => self.controls.interior_light_mode = *mode,
            ControlAction::ToggleHomeLight(source) => {
                if let Some(i) = self
                    .reference
                    .groups
                    .iter()
                    .position(|g| g.source == *source)
                {
                    self.groups[i].on = !self.groups[i].on;
                }
            }
            ControlAction::ToggleHandbrake => {}
        }
    }
    pub fn step(&mut self, mechanisms: &MechanismState, items: &ItemStore, dt: f32) {
        let doors_open = self.reference.door_items.iter().any(|id| {
            items.mounted(*id)
                && mechanisms
                    .value(*id, false)
                    .is_some_and(|value| value > self.reference.door_limit)
        });
        let interior_requested = match self.controls.interior_light_mode {
            0 => doors_open,
            1 => true,
            _ => false,
        };
        for (g, state) in self.reference.groups.iter().zip(&mut self.groups) {
            let installed = g.owner.is_none_or(|id| items.mounted(id));
            if g.control == LampControl::Headlights && !installed {
                state.current = 0;
                state.energy = 0.;
                continue;
            }
            let target = match g.control {
                LampControl::Headlights => {
                    if installed {
                        self.controls.headlights
                    } else {
                        0
                    }
                }
                LampControl::Interior => u8::from(interior_requested),
                LampControl::Home => u8::from(
                    state.on
                        && state.electronics_output > self.reference.home_electronics_threshold,
                ),
            };
            if g.control == LampControl::Home {
                state.current = target;
            } else if state.current != target {
                if state.energy > 0.0 {
                    state.energy = (state.energy - g.energy_move * dt).max(0.0);
                } else {
                    state.current = target;
                }
            } else {
                state.energy = (state.energy + g.energy_move * dt).min(1.0);
            }
        }
        self.controls.interior_light =
            self.reference
                .groups
                .iter()
                .zip(&self.groups)
                .any(|(reference, state)| {
                    reference.control == LampControl::Interior
                        && state.current == 1
                        && state.energy * state.electronics_output > 0.0
                });
    }
    /// The renderer uses this same gate for both lamps and material states.
    pub fn output(&self, group: usize, state: usize) -> f32 {
        let current = &self.groups[group];
        if usize::from(current.current) == state {
            current.energy
                * if self.reference.groups[group].control == LampControl::Home {
                    1.0
                } else {
                    current.electronics_output
                }
        } else {
            0.0
        }
    }
}
