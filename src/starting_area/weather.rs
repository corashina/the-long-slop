//! Weather on the fixed step: wind roots and the live interior volumes.
use super::GarageRuntime;
use crate::audio::interiors::WorldInteriors;
use crate::interaction::items::ItemStore;
use crate::reference::{ColliderId, ItemId, ReferenceManifest};
use crate::vehicle::PhysicsWorld;
use crate::weather::InteriorVolume;
use rapier3d::prelude::{ColliderHandle, RigidBodyHandle};
use std::collections::HashMap;

/// Retained wind bindings. Item and interior changes rebuild them, not each step.
pub(super) struct WeatherBindings {
    player: RigidBodyHandle,
    /// The first live item of each physical body.
    items: HashMap<RigidBodyHandle, ItemId>,
    item_revision: Option<u64>,
    garage: Vec<ColliderHandle>,
    world: WorldInteriors,
    volumes: Vec<InteriorVolume>,
}

impl WeatherBindings {
    /// `interiors` are the garage interior colliders of the live interior list.
    pub(super) fn new(
        physics: &PhysicsWorld,
        player: RigidBodyHandle,
        interiors: &[ColliderId],
    ) -> Self {
        let garage: Vec<_> = physics
            .rapier()
            .colliders
            .iter()
            .filter(|(_, c)| interiors.contains(&ColliderId((c.user_data >> 64) as u32)))
            .map(|(handle, _)| handle)
            .collect();
        Self {
            player,
            items: HashMap::new(),
            item_revision: None,
            world: WorldInteriors::new(garage.len()),
            volumes: Vec::with_capacity(garage.len() + 256),
            garage,
        }
    }

    fn sync(
        &mut self,
        items: &ItemStore,
        physics: &PhysicsWorld,
        stream: &crate::world::WorldStream,
    ) {
        let mut changed = self.world.sync(
            stream.structure_revision(),
            stream.frame().structures,
            &stream.world_reference().prototypes,
        );
        if self.item_revision != Some(items.membership_revision()) {
            self.item_revision = Some(items.membership_revision());
            self.items.clear();
            for item in items.iter().filter(|item| !item.removed) {
                if let Some(body) = item.body {
                    self.items.entry(body).or_insert(item.id);
                }
            }
            changed = true;
        }
        if !changed {
            return;
        }
        self.volumes.clear();
        for collider in self.garage.iter().copied().chain(self.world.colliders()) {
            let root = physics
                .rapier()
                .colliders
                .get(collider)
                .and_then(|c| c.parent())
                .filter(|body| {
                    physics
                        .rapier()
                        .bodies
                        .get(*body)
                        .is_some_and(|b| !b.is_fixed())
                })
                .map(|body| root(&self.items, items, body));
            self.volumes.push(InteriorVolume { collider, root });
        }
    }

    /// The root of a body that wind moves. The player body has none.
    fn wind_root(&self, items: &ItemStore, body: RigidBodyHandle) -> Option<RigidBodyHandle> {
        (body != self.player).then(|| root(&self.items, items, body))
    }
}

/// The body of the item at the top of the parent chain: attachment parent, lock owner or slot owner.
/// Installed parts and wheels already share the body of their vehicle.
fn root(
    bodies: &HashMap<RigidBodyHandle, ItemId>,
    items: &ItemStore,
    body: RigidBodyHandle,
) -> RigidBodyHandle {
    let Some(&(mut id)) = bodies.get(&body) else {
        return body;
    };
    for _ in 0..items.len() {
        match items.parent(id) {
            Some(parent) => id = parent,
            None => break,
        }
    }
    items.get(id).and_then(|item| item.body).unwrap_or(body)
}

impl GarageRuntime {
    /// Runs once before physics, with normal simulation time. Sleep does not speed storms up.
    pub(super) fn advance_weather(&mut self, reference: &ReferenceManifest, dt: f32) {
        let bindings = &mut self.weather_bindings;
        bindings.sync(&self.area.items, &self.physics, &self.stream);
        self.weather
            .reserve_bodies(self.physics.rapier().bodies.len());
        let player = self
            .stream
            .origin
            .absolute(self.area.player.body_pose(&self.physics).position);
        let stream = &mut self.stream;
        self.weather.advance(
            &reference.weather,
            self.simulation_seconds,
            dt,
            player,
            |point| stream.terrain_sample(point),
        );
        let (bindings, items) = (&self.weather_bindings, &self.area.items);
        // WaitForSeconds resumes after Update: apply cached targets before later rescans.
        self.weather.apply_wind(
            &mut self.physics,
            &reference.weather,
            &bindings.volumes,
            dt,
            |body| bindings.wind_root(items, body),
        );
        let physics = &self.physics;
        self.weather.scan_wind(
            physics,
            &reference.weather,
            self.simulation_seconds,
            |body| {
                physics
                    .rapier()
                    .bodies
                    .get(body)
                    .is_some_and(|b| b.is_dynamic() && b.is_enabled())
                    .then(|| bindings.wind_root(items, body))
                    .flatten()
            },
        );
        self.lighting
            .cycle
            .compose_weather(&reference.weather, self.weather.influence());
        let Some(audio) = &mut self.audio else {
            return;
        };
        use crate::audio::AudioKind;
        audio.remove_storm_loops(self.weather.storms());
        // sandstormscript.Update: source.volume = defVolume * llerp.
        let volume = audio
            .reference
            .survival
            .iter()
            .find(|b| b.kind == AudioKind::Storm as u32)
            .expect("validated storm audio")
            .gain[0];
        for storm in self.weather.storms() {
            audio.source_loop(
                AudioKind::Storm,
                storm.id.0,
                volume * storm.life_strength,
                storm.absolute_position,
            );
        }
    }
}
