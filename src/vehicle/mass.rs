//! Authored mass, locked cargo mass, and the single carried-body override.
use super::PhysicsWorld;
use rapier3d::prelude::RigidBodyHandle;

/// Preallocated non-vehicle cargo bodies.
pub(super) const CARGO_BODIES: usize = 64;

pub(super) struct BodyMassOverride {
    pub body: RigidBodyHandle,
    pub authored_mass_kg: f32,
    pub physical_mass_kg: f32,
}

impl PhysicsWorld {
    /// Returns authored mass with cargo while carry uses temporary physical mass.
    pub fn authored_body_mass(&self, body: RigidBodyHandle) -> f32 {
        self.body_mass_override
            .as_ref()
            .filter(|state| state.body == body)
            .map_or_else(
                || self.rapier().bodies[body].mass(),
                |state| state.authored_mass_kg,
            )
    }

    fn body_cargo(&self, body: RigidBodyHandle) -> f32 {
        self.body_cargo
            .iter()
            .find(|(handle, _)| *handle == body)
            .map_or(0.0, |(_, mass)| *mass)
    }

    /// Updates zero-density portable or assembly mass before any impulse.
    /// `mass_kg` excludes locked cargo. The body keeps its cargo.
    pub fn set_body_mass(&mut self, body: RigidBodyHandle, mass_kg: f32) {
        self.apply_body_mass(body, mass_kg + self.body_cargo(body));
    }

    /// Sets the authored mass with cargo. A carried body keeps its physical mass.
    fn apply_body_mass(&mut self, body: RigidBodyHandle, mass_kg: f32) {
        let physical_mass_kg = if let Some(state) = self
            .body_mass_override
            .as_mut()
            .filter(|state| state.body == body)
        {
            state.authored_mass_kg = mass_kg;
            state.physical_mass_kg
        } else {
            mass_kg
        };
        self.edit_body(body, |body| {
            body.set_additional_mass(physical_mass_kg, true)
        });
        self.recompute_body_mass(body);
    }

    /// Applies or clears the single carried-body mass override.
    pub fn set_body_mass_override(&mut self, body: RigidBodyHandle, mass_kg: Option<f32>) {
        if let Some(physical_mass_kg) = mass_kg {
            self.recompute_body_mass(body);
            let authored_mass_kg = self.authored_body_mass(body);
            assert!(
                self.body_mass_override
                    .as_ref()
                    .is_none_or(|state| state.body == body)
            );
            self.body_mass_override = Some(BodyMassOverride {
                body,
                authored_mass_kg,
                physical_mass_kg,
            });
            self.apply_body_mass(body, authored_mass_kg);
        } else if self
            .body_mass_override
            .as_ref()
            .is_some_and(|state| state.body == body)
        {
            let state = self.body_mass_override.take().unwrap();
            self.apply_body_mass(body, state.authored_mass_kg);
        }
    }

    /// Adds locked item mass to a carrier body. Vehicle cargo stays through `set_vehicle_mass`.
    /// Other bodies keep it through `set_body_mass`.
    pub fn add_cargo_mass(&mut self, body: RigidBodyHandle, mass_kg: f32) {
        self.recompute_body_mass(body);
        if let Some(index) = self.vehicle_of(body) {
            let base = self.physics.bodies[body].mass() - self.vehicles[index].cargo_mass;
            self.vehicles[index].cargo_mass = (self.vehicles[index].cargo_mass + mass_kg).max(0.0);
            self.set_vehicle_mass(index, base);
            return;
        }
        let cargo = self.body_cargo(body);
        let base = self.authored_body_mass(body) - cargo;
        let cargo = (cargo + mass_kg).max(0.0);
        match self
            .body_cargo
            .iter()
            .position(|(handle, _)| *handle == body)
        {
            Some(index) if cargo <= 1e-6 => {
                self.body_cargo.swap_remove(index);
            }
            Some(index) => self.body_cargo[index].1 = cargo,
            None if cargo > 1e-6 => self.body_cargo.push((body, cargo)),
            None => {}
        }
        self.apply_body_mass(body, (base + cargo).max(0.001));
    }

    /// Forgets the cargo of a removed body.
    pub(super) fn forget_body_cargo(&mut self, body: RigidBodyHandle) {
        self.body_cargo.retain(|(handle, _)| *handle != body);
    }
}
