use super::GarageRuntime;
use crate::interaction::items::{ItemLocation, ItemStore};
use crate::reference::{BodyId, Bounds, ItemId, ReferenceError, ReferenceManifest};
use crate::vehicle::{PhysicsWorld, VehiclePose, collider_builder, to_rapier};
use crate::weather::ShelterVolume;
use rapier3d::math::Pose;
use rapier3d::prelude::{RigidBodyHandle, SharedShape};

struct ShelterBinding {
    root: RigidBodyHandle,
    pose: Pose,
    shape: SharedShape,
}

/// Retained source hierarchy bindings. Only lifecycle changes rebuild ownership.
pub(super) struct WeatherBindings {
    shelters: Vec<ShelterBinding>,
    volumes: Vec<ShelterVolume>,
    roots: Vec<(RigidBodyHandle, RigidBodyHandle)>,
    items: Vec<(ItemId, Option<RigidBodyHandle>, ItemLocation, bool)>,
}

impl WeatherBindings {
    pub(super) fn new(
        reference: &ReferenceManifest,
        bodies: &std::collections::HashMap<BodyId, RigidBodyHandle>,
    ) -> Result<Self, ReferenceError> {
        let mut shelters = Vec::with_capacity(reference.weather.shelters.len());
        for r in &reference.weather.shelters {
            let collider = collider_builder(&r.geometry)?.build();
            shelters.push(ShelterBinding {
                root: bodies[&r.root],
                pose: to_rapier(VehiclePose {
                    position: r.local_pose.translation,
                    rotation: r.local_pose.rotation,
                }) * *collider.position(),
                shape: collider.shared_shape().clone(),
            });
        }
        Ok(Self {
            volumes: Vec::with_capacity(shelters.len()),
            shelters,
            roots: Vec::new(),
            items: Vec::new(),
        })
    }

    fn update_shelters(&mut self, physics: &PhysicsWorld) {
        self.volumes.clear();
        for binding in &self.shelters {
            let Some(body) = physics.rapier().bodies.get(binding.root) else {
                continue;
            };
            if !body.is_enabled() {
                continue;
            }
            let bounds = binding
                .shape
                .compute_aabb(&(*body.position() * binding.pose));
            self.volumes.push(ShelterVolume {
                root: binding.root,
                bounds: Bounds {
                    min: bounds.mins.to_array(),
                    max: bounds.maxs.to_array(),
                },
            });
        }
    }
}

impl GarageRuntime {
    /// Refresh after item/body lifecycle changes and origin changes. This does not advance weather.
    pub fn sync_weather_bindings(&mut self, reference: &ReferenceManifest) {
        let bindings = &mut self.weather_bindings;
        let changed = bindings.items.len() != self.area.items.len()
            || bindings
                .items
                .iter()
                .zip(self.area.items.iter())
                .any(|(old, item)| *old != (item.id, item.body, item.location, item.removed))
            || bindings.roots.len() != self.physics.rapier().bodies.len()
            || bindings
                .roots
                .iter()
                .any(|(body, _)| !self.physics.rapier().bodies.contains(*body));
        if changed {
            let count = self.physics.rapier().bodies.len();
            bindings.roots.clear();
            bindings.roots.reserve(count);
            bindings.items.clear();
            bindings.items.reserve(self.area.items.len());
            bindings.items.extend(
                self.area
                    .items
                    .iter()
                    .map(|item| (item.id, item.body, item.location, item.removed)),
            );
            for (handle, _) in self.physics.rapier().bodies.iter() {
                let mut root = handle;
                if let Some(record) = reference
                    .bodies
                    .iter()
                    .find(|r| self.area.bodies.get(&r.id) == Some(&handle))
                {
                    let mut parent = record.parent_body;
                    while let Some(id) = parent {
                        let Some(body) = reference.bodies.iter().find(|b| b.id == id) else {
                            break;
                        };
                        if let Some(bound) = self.area.bodies.get(&id) {
                            root = *bound;
                        }
                        parent = body.parent_body;
                    }
                }
                // Item ownership is current: detached parts may reuse a different physical body.
                if let Some(item) = self
                    .area
                    .items
                    .iter()
                    .find(|item| !item.removed && item.body == Some(handle))
                {
                    root = item_root(&self.area.items, reference, item.id, &self.physics)
                        .unwrap_or(handle);
                }
                bindings.roots.push((handle, root));
            }
            self.weather.reserve_bodies(count);
        }
        self.weather.set_origin(self.stream.origin.offset);
        self.weather.refresh_roots(&self.physics, &bindings.roots);
        // Current body poses keep moving vehicle shelters valid between one-second wind scans.
        bindings.update_shelters(&self.physics);
    }

    pub fn shelter_volumes(&self) -> &[ShelterVolume] {
        &self.weather_bindings.volumes
    }

    /// Task 8 calls this once before physics, with normal simulation time and FIXED_DT.
    /// Normal play does not call it until weather presentation and audio are connected.
    pub fn advance_weather(&mut self, reference: &ReferenceManifest, dt: f32) {
        self.sync_weather_bindings(reference);
        self.physics.refresh_queries();
        let player = self
            .stream
            .origin
            .absolute(self.area.player.body_pose(&self.physics).position);
        self.weather.advance(
            &reference.weather,
            self.simulation_seconds,
            dt,
            player,
            |point| self.stream.terrain_sample(point),
        );
        // WaitForSeconds resumes after Update: apply cached targets before later rescans.
        self.weather
            .apply_wind(&mut self.physics, &self.weather_bindings.volumes, dt);
        // Initial source distanceLerp is zero. Its pre-Move scan prepares the next Wind call.
        self.weather.scan_wind(
            &self.physics,
            &self.weather_bindings.roots,
            &reference.weather,
            self.simulation_seconds,
        );
        let cycle = self.lighting.cycle.output();
        let night = match cycle.phase {
            crate::lighting::CyclePhase::Day => 0.,
            crate::lighting::CyclePhase::Night => 1.,
            crate::lighting::CyclePhase::Dawn => 1. - cycle.progress,
            crate::lighting::CyclePhase::Dusk => cycle.progress,
        };
        self.weather.advance_particles(
            &reference.weather,
            dt,
            player,
            night,
            &self.weather_bindings.volumes,
            &self.physics,
        );
        self.lighting
            .cycle
            .compose_weather(&reference.weather, self.weather.influence());
        let gains =
            self.weather
                .ambient_gains(&reference.weather, self.lighting.cycle.output(), dt);
        use crate::audio::AudioKind;
        let Some(audio) = &mut self.audio else {
            return;
        };
        audio.remove_storm_loops(self.weather.storms());
        {
            let source_gain = audio
                .reference
                .survival
                .iter()
                .find(|b| b.kind == AudioKind::Storm as u32)
                .unwrap()
                .gain[0];
            for storm in self.weather.storms() {
                audio.source_loop(
                    AudioKind::Storm,
                    storm.id.0,
                    source_gain * storm.life_strength,
                    storm.absolute_position,
                );
            }
        }
        audio.source_loop(
            AudioKind::StormInside,
            0,
            reference.weather.storm.volume * self.weather.influence().strength,
            player,
        );
        audio.source_loop(AudioKind::AmbientDay, 0, gains[0], player);
        audio.source_loop(AudioKind::AmbientNight, 0, gains[1], player);
    }
}

fn item_root(
    items: &ItemStore,
    reference: &ReferenceManifest,
    mut id: ItemId,
    physics: &PhysicsWorld,
) -> Option<RigidBodyHandle> {
    for _ in 0..items.len() {
        let item = items.get(id)?;
        if item.removed {
            return None;
        }
        match item.location {
            ItemLocation::Attached(parent) => id = parent,
            ItemLocation::PartSlot(slot) => {
                match reference.part_slots.iter().find(|s| s.id == slot)?.owner {
                    Some(parent) => id = parent,
                    None => {
                        return Some(physics.chassis_handle(slot_vehicle(reference, slot)?));
                    }
                }
            }
            ItemLocation::WheelSlot(slot) => {
                return Some(
                    physics.chassis_handle(
                        reference
                            .vehicles
                            .iter()
                            .position(|v| v.wheel_slots.contains(&slot))?,
                    ),
                );
            }
            // Source ordinary Pickup keeps its root. Right-hand parenting is outside current controls.
            ItemLocation::World
            | ItemLocation::Held
            | ItemLocation::Stored(_)
            | ItemLocation::Equipped => return item.body,
        }
    }
    None
}

fn slot_vehicle(
    reference: &ReferenceManifest,
    slot: crate::reference::PartSlotId,
) -> Option<usize> {
    let mut node = Some(reference.part_slots.iter().find(|s| s.id == slot)?.node);
    while let Some(id) = node {
        if let Some(index) = reference.vehicles.iter().position(|v| v.root == id) {
            return Some(index);
        }
        node = reference.nodes.iter().find(|n| n.id == id)?.parent;
    }
    None
}
