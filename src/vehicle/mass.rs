//! Authored mass and the single carried-body override.
use super::PhysicsWorld;
use rapier3d::prelude::RigidBodyHandle;

pub(super) struct BodyMassOverride {
    pub body: RigidBodyHandle,
    pub authored_mass_kg: f32,
    pub physical_mass_kg: f32,
}

impl PhysicsWorld {
    /// Returns authored mass while carry uses temporary physical mass.
    pub fn authored_body_mass(&self, body: RigidBodyHandle) -> f32 {
        self.body_mass_override
            .as_ref()
            .filter(|state| state.body == body)
            .map_or_else(
                || self.rapier().bodies[body].mass(),
                |state| state.authored_mass_kg,
            )
    }

    /// Updates zero-density portable or assembly mass before any impulse.
    pub fn set_body_mass(&mut self, body: RigidBodyHandle, mass_kg: f32) {
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
            self.set_body_mass(body, authored_mass_kg);
        } else if self
            .body_mass_override
            .as_ref()
            .is_some_and(|state| state.body == body)
        {
            let state = self.body_mass_override.take().unwrap();
            self.set_body_mass(body, state.authored_mass_kg);
        }
    }
}
