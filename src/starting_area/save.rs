use super::{GarageRuntime, StartingArea};
use crate::reference::ReferenceManifest;
use crate::save::*;
use crate::vehicle::{PhysicsWorld, capture_body_save};
use crate::world::WorldOrigin;
use std::collections::HashSet;

impl GarageRuntime {
    pub fn capture_save(
        &self,
        reference: &ReferenceManifest,
        source: &SourceStamp,
        view: CameraSave,
        saved_at: &str,
    ) -> Result<SaveSnapshot, SaveError> {
        let ids = IdentityMap::new(reference)?;
        let origin = &self.stream.origin;
        let (wheels, tires) = self.wheels.capture_save(&ids, &self.physics)?;
        let mut world = self
            .persistent_world
            .capture(&self.area, &self.physics, origin, &ids)?;
        for record in self.stream.capture_save(&self.area.interaction).records {
            let WorldRecord::Pump { instance, .. } = &record;
            if let Some(old) = world.records.iter_mut().find(|old| {
                let WorldRecord::Pump {
                    instance: other, ..
                } = old;
                instance == other
            }) {
                *old = record;
            } else {
                world.records.push(record);
            }
        }
        world.seed = self.stream.seed.to_string();
        world.records.sort_by(|a, b| {
            let WorldRecord::Pump { instance: a, .. } = a;
            let WorldRecord::Pump { instance: b, .. } = b;
            a.cmp(b)
        });
        let mut snapshot = SaveSnapshot {
            schema_version: SAVE_SCHEMA_VERSION,
            saved_at: saved_at.into(),
            source: source.clone(),
            world,
            player: self.area.player.capture_save(
                &ids,
                origin,
                &self.physics,
                self.area.interaction.capture_carry_save(&ids)?,
            )?,
            vehicles: vec![VehicleSnapshot {
                instance: ids.vehicle.clone(),
                body: self.physics.capture_save(
                    BodyIdentity::Source {
                        source: crate::save::identity::source_key(
                            &ids.bodies,
                            reference.vehicle.body,
                        )?,
                    },
                    origin,
                ),
                engine: self.engine.capture_save(),
                auto_gear: self.auto_gear,
                handbrake: self.handbrake,
            }],
            bodies: self.area.capture_save(
                reference,
                &ids,
                origin,
                &self.physics,
                &self.persistent_world,
            )?,
            items: self.area.items.capture_save(&ids, origin, &self.physics)?,
            attachments: AttachmentsSnapshot {
                parts: self.area.interaction.parts.capture_save(&ids)?,
                wheels,
                tires,
            },
            mechanisms: self.area.interaction.capture_save(&ids)?,
            progress: ProgressSnapshot {
                trip_m: self.trip_m,
                start_attempts: self.attempts.to_string(),
            },
            lighting: self.lighting.capture_save(&reference.lighting, &ids)?,
            view: ViewSnapshot { camera: view },
        };
        if let Some(saved) = self.persistent_world.body_snapshot(
            self.physics.chassis_handle(),
            snapshot.vehicles[0].body.identity.clone(),
        ) {
            snapshot.vehicles[0].body = saved;
        }
        snapshot.validate(reference, source)?;
        Ok(snapshot)
    }
}
impl StartingArea {
    pub fn capture_save(
        &self,
        reference: &ReferenceManifest,
        ids: &IdentityMap,
        origin: &WorldOrigin,
        physics: &PhysicsWorld,
        world: &PersistentWorld,
    ) -> Result<Vec<BodySnapshot>, SaveError> {
        let mut records = Vec::new();
        let mut captured = HashSet::new();
        captured.insert(physics.chassis_handle());
        for item in self.items.iter() {
            if !matches!(
                item.location,
                crate::interaction::items::ItemLocation::World
                    | crate::interaction::items::ItemLocation::Held
            ) {
                continue;
            }
            let detachable = reference.parts.iter().any(|part| part.item == item.id)
                || reference
                    .wheel_parts
                    .iter()
                    .any(|part| part.item == item.id);
            if !detachable {
                continue;
            }
            let handle = item
                .body
                .ok_or_else(|| SaveError::InvalidLinks("Missing detached assembly body".into()))?;
            if !captured.insert(handle) {
                continue;
            }
            let body = physics
                .rapier()
                .bodies
                .get(handle)
                .ok_or_else(|| SaveError::InvalidLinks("Missing detached body".into()))?;
            let identity = BodyIdentity::Assembly {
                root: ids.item_instance(item.id)?,
            };
            records.push(
                world
                    .body_snapshot(handle, identity.clone())
                    .unwrap_or_else(|| capture_body_save(body, identity, origin)),
            );
        }
        for record in &reference.bodies {
            if record.parent_body.is_some() {
                continue;
            }
            let handle = *self
                .bodies
                .get(&record.id)
                .ok_or_else(|| SaveError::InvalidLinks("Missing source body".into()))?;
            if !captured.insert(handle) {
                continue;
            }
            let body = physics
                .rapier()
                .bodies
                .get(handle)
                .ok_or_else(|| SaveError::InvalidLinks("Missing source body".into()))?;
            let identity = BodyIdentity::Source {
                source: SourceKey::from(&record.source),
            };
            let saved = world
                .body_snapshot(handle, identity.clone())
                .unwrap_or_else(|| capture_body_save(body, identity, origin));
            if saved.pose.position != record.pose.translation.map(f64::from)
                || saved.pose.rotation != record.pose.rotation
                || saved.linear_velocity != [0.; 3]
                || saved.angular_velocity != [0.; 3]
                || (record.dynamic && saved.sleeping)
            {
                records.push(saved);
            }
        }
        records.sort_by(|a, b| match (&a.identity, &b.identity) {
            (BodyIdentity::Source { source: a }, BodyIdentity::Source { source: b }) => a.cmp(b),
            (BodyIdentity::Assembly { root: a }, BodyIdentity::Assembly { root: b }) => a.cmp(b),
            (BodyIdentity::Source { .. }, _) => std::cmp::Ordering::Less,
            _ => std::cmp::Ordering::Greater,
        });
        Ok(records)
    }
}
