use super::{GarageRuntime, StartingArea};
use crate::reference::ReferenceManifest;
use crate::save::*;
use crate::vehicle::{PhysicsWorld, capture_body_save};
use crate::world::WorldOrigin;
use std::collections::HashSet;

impl GarageRuntime {
    pub fn from_save(
        reference: &ReferenceManifest,
        expected: &SourceStamp,
        saved: &SaveSnapshot,
    ) -> Result<Self, SaveError> {
        saved.validate(reference, expected)?;
        let ids = IdentityMap::new(reference)?;
        let mut candidate = Self::new_candidate(reference, false)?;
        candidate.area.items.restore_save(&ids, &saved.items)?;
        candidate.wheels.restore_save(
            &ids,
            &saved.attachments,
            &mut candidate.area.items,
            &mut candidate.physics,
        )?;
        candidate.area.interaction.parts.restore_save(
            &ids,
            &saved.attachments.parts,
            &mut candidate.area.items,
            &mut candidate.area.interaction.mechanisms,
            &mut candidate.physics,
        )?;
        candidate
            .area
            .interaction
            .restore_save(&saved.mechanisms, &ids);
        candidate
            .area
            .interaction
            .sync_parts(&mut candidate.area.items, &mut candidate.physics)?;
        // Assembly setup may change local poses. The snapshot owns their saved frames.
        candidate.area.items.restore_save(&ids, &saved.items)?;
        candidate.stream.seed = crate::save::identity::unsigned(&saved.world.seed)?;
        candidate.stream.origin.offset = saved.player.pose.position;
        candidate.stream.focus = saved.player.pose.position;
        candidate.area.origin = candidate.stream.origin.offset;
        candidate.persistent_world = PersistentWorld::restore(saved, &ids)?;
        candidate.persistent_world.sync(
            reference,
            &mut candidate.area,
            &mut candidate.wheels,
            &candidate.stream,
            &mut candidate.physics,
        )?;
        candidate
            .stream
            .sync_at(saved.player.pose.position, &mut candidate.physics);
        crate::world::sync_station_supplies(
            &mut candidate.stream,
            &mut candidate.area.items,
            &mut candidate.area.interaction,
            &mut candidate.physics,
        );
        candidate.persistent_world.sync(
            reference,
            &mut candidate.area,
            &mut candidate.wheels,
            &candidate.stream,
            &mut candidate.physics,
        )?;
        candidate
            .persistent_world
            .sync_pumps(&candidate.stream, &mut candidate.area.interaction);
        candidate
            .area
            .interaction
            .sync_mechanisms(&mut candidate.physics);
        candidate
            .area
            .sync_attachments(&mut candidate.physics, false);
        candidate.area.sync_compound(&mut candidate.physics);
        candidate
            .area
            .interaction
            .sync_parts(&mut candidate.area.items, &mut candidate.physics)?;
        candidate.area.sync_portable_mass(&mut candidate.physics);
        candidate
            .wheels
            .sync(&mut candidate.area.items, &mut candidate.physics)?;
        candidate.area.items.restore_save(&ids, &saved.items)?;
        candidate.area.items.sync_poses(&candidate.physics);
        // Rapier defers collider and additional-mass updates until a step or recomputation.
        // Commit them while paused, before restoring motion and sleep.
        let world = candidate.physics.rapier_mut();
        for (_, body) in world.bodies.iter_mut() {
            if body.is_dynamic() {
                body.recompute_mass_properties_from_colliders(&world.colliders);
            }
        }
        // Mass and collider assembly may wake bodies. Restore velocities and sleep last.
        for record in saved
            .bodies
            .iter()
            .chain(saved.vehicles.iter().map(|v| &v.body))
        {
            let handle = match &record.identity {
                BodyIdentity::Source { source } => candidate.area.bodies[&ids.bodies[source]],
                BodyIdentity::Assembly { root } => candidate
                    .area
                    .items
                    .get(ids.item(root)?)
                    .and_then(|i| i.body)
                    .ok_or_else(|| {
                        SaveError::ReconstructionFailure("Missing assembly body".into())
                    })?,
            };
            if candidate.physics.body_active(handle) {
                let body = &mut candidate.physics.rapier_mut().bodies[handle];
                body.set_linvel(
                    rapier3d::math::Vector::from_array(record.linear_velocity),
                    false,
                );
                body.set_angvel(
                    rapier3d::math::Vector::from_array(record.angular_velocity),
                    false,
                );
                if record.sleeping {
                    body.sleep();
                } else {
                    body.wake_up(true);
                }
            }
        }
        candidate.physics.refresh_queries();
        candidate.area.player.restore_save(
            &saved.player,
            &ids,
            &candidate.stream.origin,
            &mut candidate.physics,
        )?;
        if let Some(carry) = &saved.player.carry {
            candidate.area.interaction.restore_carry_save(
                carry,
                &ids,
                &candidate.area.items,
                &mut candidate.physics,
            )?;
        }
        candidate.vehicle = ids
            .vehicles
            .iter()
            .position(|id| *id == saved.progress.selected_vehicle)
            .expect("validated selected vehicle");
        for (index, vehicle) in candidate.vehicles.iter_mut().enumerate() {
            let record = saved
                .vehicles
                .iter()
                .find(|v| v.instance == ids.vehicles[index])
                .expect("validated vehicle");
            vehicle.engine_output = vehicle.engine.restore_save(&record.engine);
            vehicle.drivetrain.restore_gear(record.gear);
            vehicle.handbrake = record.handbrake;
            vehicle.trip_m = record.trip_m;
        }
        candidate.driving_assists = saved.driving_assists;
        candidate.attempts = crate::save::identity::unsigned(&saved.progress.start_attempts)?;
        let mounted: Vec<_> = candidate
            .light_owners
            .iter()
            .map(|owner| owner.is_none_or(|item| candidate.area.items.mounted(item)))
            .collect();
        let mut signals = Vec::with_capacity(candidate.vehicles.len());
        let mut headlights = Vec::with_capacity(candidate.vehicles.len());
        for (index, vehicle) in candidate.vehicles.iter().enumerate() {
            let electronics = reference
                .lighting
                .controllers
                .iter()
                .find(|c| {
                    c.kind == crate::reference::LightControllerKind::Rear
                        && c.vehicle == Some(index)
                })
                .ok_or_else(|| {
                    SaveError::ReconstructionFailure("Missing vehicle electronics".into())
                })?;
            signals.push(crate::lighting::VehicleLightSample {
                local_ignition: vehicle.engine_output.ignition
                    && (electronics.electronics_source.is_none()
                        || electronics.electronics_output > electronics.electronics_threshold),
                brake: 0.,
                gear_ratio: reference.vehicles[index].gear_ratios[vehicle.drivetrain.gear()],
                electronics_output: electronics.electronics_output,
            });
            headlights.push(
                saved
                    .vehicles
                    .iter()
                    .find(|v| v.instance == ids.vehicles[index])
                    .unwrap()
                    .requested_headlights,
            );
        }
        candidate.lighting.restore_save(
            &reference.lighting,
            &ids,
            &saved.lighting,
            &mounted,
            &signals,
            &headlights,
        )?;
        candidate.area.commit_poses(&candidate.physics, true);
        if let Some(audio) = &mut candidate.audio {
            audio.bind_game(reference, &candidate.area, &candidate.physics);
            audio.sync_lamps(&reference.lighting, &candidate.lighting);
        }
        crate::world::sync_station_food(
            &mut candidate.stream,
            reference,
            &mut candidate.area,
            &mut candidate.physics,
        )?;
        candidate.sync_weather_bindings(reference);
        candidate.clear_input();
        candidate.paused = true;
        candidate.started = true;
        Ok(candidate)
    }
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
            driving_assists: self.driving_assists,
            schema_version: SAVE_SCHEMA_VERSION,
            saved_at: saved_at.into(),
            source: source.clone(),
            world,
            player: self.area.player.capture_save(
                &ids,
                origin,
                &self.physics,
                if self
                    .area
                    .interaction
                    .held()
                    .is_some_and(|id| !reference.items.iter().any(|r| r.id == id))
                {
                    None
                } else {
                    self.area.interaction.capture_carry_save(&ids)?
                },
            )?,
            vehicles: self
                .vehicles
                .iter()
                .enumerate()
                .map(|(index, vehicle)| {
                    Ok(VehicleSnapshot {
                        instance: ids.vehicles[index].clone(),
                        body: self.physics.capture_save(
                            index,
                            BodyIdentity::Source {
                                source: crate::save::identity::source_key(
                                    &ids.bodies,
                                    reference.vehicles[index].body,
                                )?,
                            },
                            origin,
                        ),
                        engine: vehicle.engine.capture_save(),
                        gear: vehicle.drivetrain.gear(),
                        handbrake: vehicle.handbrake,
                        trip_m: vehicle.trip_m,
                        requested_headlights: self.lighting.headlights(index),
                    })
                })
                .collect::<Result<Vec<_>, SaveError>>()?,
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
                selected_vehicle: ids.vehicles[self.vehicle].clone(),
                start_attempts: self.attempts.to_string(),
            },
            lighting: self.lighting.capture_save(&reference.lighting, &ids)?,
            view: ViewSnapshot { camera: view },
        };
        for (index, vehicle) in snapshot.vehicles.iter_mut().enumerate() {
            if let Some(body) = self.persistent_world.body_snapshot(
                self.physics.chassis_handle(index),
                vehicle.body.identity.clone(),
            ) {
                vehicle.body = body;
            }
        }
        snapshot
            .vehicles
            .sort_by(|a, b| a.instance.cmp(&b.instance));
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
        for index in 0..physics.vehicle_count() {
            captured.insert(physics.chassis_handle(index));
        }
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
            if reference
                .items
                .iter()
                .any(|i| i.body == record.id && i.id == reference.survival.starter_food)
                && self
                    .items
                    .get(reference.survival.starter_food)
                    .is_some_and(|i| i.removed)
            {
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
            let at_source = saved.pose.position == record.pose.translation.map(f64::from)
                || (physics.body_active(handle)
                    && body.translation().to_array()
                        == origin.local(record.pose.translation.map(f64::from)));
            if !at_source
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
