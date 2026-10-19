use super::*;
use crate::interaction::{items::ItemLocation, wheels::WheelAssemblies};
use crate::reference::{BodyId, ItemId, ReferenceManifest};
use crate::starting_area::StartingArea;
use crate::vehicle::{PhysicsWorld, VehiclePose, from_rapier, to_rapier};
use crate::world::{REBASE_LIMIT, WorldOrigin, WorldStream};
use rapier3d::math::{Pose, Rotation, Vector};
use rapier3d::prelude::RigidBodyHandle;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Key {
    Source(BodyId),
    Assembly(ItemId),
}
#[derive(Clone, Copy)]
struct BodyState {
    pose: AbsolutePose,
    linear: [f32; 3],
    angular: [f32; 3],
    sleeping: bool,
    enabled: bool,
}
struct Record {
    key: Key,
    handle: Option<RigidBodyHandle>,
    state: Option<BodyState>,
}

/// Inactive poses use absolute coordinates. Runtime handles never enter a save.
pub struct PersistentWorld {
    records: Vec<Record>,
    processed: Vec<RigidBodyHandle>,
    pending_items: Vec<(ItemId, AbsolutePose)>,
    configured: bool,
}
impl PersistentWorld {
    pub fn new(reference: &ReferenceManifest) -> Self {
        let keys = reference
            .parts
            .iter()
            .map(|part| Key::Assembly(part.item))
            .chain(
                reference
                    .wheel_parts
                    .iter()
                    .map(|part| Key::Assembly(part.item)),
            )
            .chain(reference.bodies.iter().map(|body| Key::Source(body.id)));
        let records: Vec<_> = keys
            .map(|key| Record {
                key,
                handle: None,
                state: None,
            })
            .collect();
        Self {
            processed: Vec::with_capacity(records.len()),
            records,
            pending_items: Vec::new(),
            configured: true,
        }
    }
    pub fn restore(snapshot: &SaveSnapshot, identities: &IdentityMap) -> Result<Self, SaveError> {
        let mut records = Vec::with_capacity(identities.bodies.len() + identities.items.len());
        for body in &snapshot.bodies {
            if let BodyIdentity::Assembly { root } = &body.identity {
                records.push(Record {
                    key: Key::Assembly(identities.item(root)?),
                    handle: None,
                    state: None,
                });
            }
        }
        for &id in identities.bodies.values() {
            records.push(Record {
                key: Key::Source(id),
                handle: None,
                state: None,
            });
        }
        for body in snapshot
            .bodies
            .iter()
            .chain(snapshot.vehicles.iter().map(|v| &v.body))
        {
            let key = match &body.identity {
                BodyIdentity::Source { source } => Key::Source(
                    *identities
                        .bodies
                        .get(source)
                        .ok_or_else(|| SaveError::InvalidLinks("Unknown body".into()))?,
                ),
                BodyIdentity::Assembly { root } => Key::Assembly(identities.item(root)?),
            };
            records
                .iter_mut()
                .find(|record| record.key == key)
                .unwrap()
                .state = Some(BodyState {
                pose: body.pose,
                linear: body.linear_velocity,
                angular: body.angular_velocity,
                sleeping: body.sleeping,
                enabled: true,
            });
        }
        Ok(Self {
            processed: Vec::with_capacity(records.len()),
            records,
            pending_items: snapshot
                .items
                .iter()
                .map(|item| Ok((identities.item(&item.instance)?, item.pose)))
                .collect::<Result<_, SaveError>>()?,
            configured: false,
        })
    }
    pub fn body_snapshot(
        &self,
        handle: RigidBodyHandle,
        identity: BodyIdentity,
    ) -> Option<BodySnapshot> {
        let state = self
            .records
            .iter()
            .find(|r| r.handle == Some(handle) && r.state.is_some())?
            .state?;
        Some(BodySnapshot {
            identity,
            pose: state.pose,
            linear_velocity: state.linear,
            angular_velocity: state.angular,
            sleeping: state.sleeping,
        })
    }
    pub fn recovery_body_pose(
        &self,
        physics: &PhysicsWorld,
        handle: RigidBodyHandle,
        vehicle: usize,
    ) -> Option<VehiclePose> {
        let Some(car) = self
            .records
            .iter()
            .find(|r| r.handle == Some(physics.chassis_handle(vehicle)))
            .and_then(|r| r.state)
        else {
            return Some(from_rapier(physics.rapier().bodies[handle].position()));
        };
        let state = self
            .records
            .iter()
            .find(|r| r.handle == Some(handle))?
            .state?;
        let delta =
            std::array::from_fn::<_, 3, _>(|i| state.pose.position[i] - car.pose.position[i]);
        if delta.iter().any(|v| v.abs() > f64::from(REBASE_LIMIT)) {
            return None;
        }
        Some(VehiclePose {
            position: delta.map(|v| v as f32),
            rotation: state.pose.rotation,
        })
    }
    pub fn recovery_cargo(
        &self,
        physics: &PhysicsWorld,
        handle: RigidBodyHandle,
        vehicle: usize,
        bounds: &crate::reference::Bounds,
    ) -> bool {
        let Some(state) = self
            .records
            .iter()
            .find(|r| r.handle == Some(handle))
            .and_then(|r| r.state)
        else {
            return false;
        };
        let Some(car) = self.recovery_body_pose(physics, physics.chassis_handle(vehicle), vehicle)
        else {
            return false;
        };
        let Some(pose) = self.recovery_body_pose(physics, handle, vehicle) else {
            return false;
        };
        let point = to_rapier(car).inverse() * Vector::from_array(pose.position);
        state.enabled
            && physics.rapier().bodies[handle].is_dynamic()
            && physics.vehicle_of(handle).is_none()
            && (0..3).all(|i| point[i] >= bounds.min[i] && point[i] <= bounds.max[i])
    }
    pub fn activate_recovery_bodies(
        &mut self,
        area: &mut StartingArea,
        physics: &mut PhysicsWorld,
        vehicle: usize,
        bodies: &[RigidBodyHandle],
    ) {
        if physics.body_active(physics.chassis_handle(vehicle)) {
            return;
        }
        // Set the virtual clearance frame only after the target permit succeeds.
        for &handle in bodies {
            let Some(pose) = self.recovery_body_pose(physics, handle, vehicle) else {
                continue;
            };
            let enabled = self
                .records
                .iter()
                .find(|r| r.handle == Some(handle))
                .and_then(|r| r.state)
                .is_some_and(|s| s.enabled);
            physics.edit_body(handle, |body| {
                body.set_position(to_rapier(pose), false);
                body.set_enabled(enabled);
            });
        }
        for &handle in bodies {
            for record in self.records.iter_mut().filter(|r| r.handle == Some(handle)) {
                record.state = None;
            }
            physics.inactive_bodies.remove(&handle);
            for item in area.items.iter_mut().filter(|i| i.body == Some(handle)) {
                item.inactive_pose = None;
            }
        }
    }

    pub fn absolute_vehicle_position(
        &self,
        physics: &PhysicsWorld,
        origin: &WorldOrigin,
        vehicle: usize,
    ) -> [f64; 3] {
        self.records
            .iter()
            .find(|r| r.handle == Some(physics.chassis_handle(vehicle)) && r.state.is_some())
            .and_then(|r| r.state)
            .map_or_else(
                || origin.absolute(physics.snapshot(vehicle).pose.position),
                |s| s.pose.position,
            )
    }
    pub fn sync(
        &mut self,
        reference: &ReferenceManifest,
        area: &mut StartingArea,
        wheels: &mut WheelAssemblies,
        stream: &WorldStream,
        physics: &mut PhysicsWorld,
    ) -> Result<(), SaveError> {
        if !self.configured {
            for key in reference
                .parts
                .iter()
                .map(|p| Key::Assembly(p.item))
                .chain(reference.wheel_parts.iter().map(|p| Key::Assembly(p.item)))
            {
                if !self.records.iter().any(|r| r.key == key) {
                    self.records.push(Record {
                        key,
                        handle: None,
                        state: None,
                    });
                }
            }
            self.records
                .sort_by_key(|r| matches!(r.key, Key::Source(_)));
            for record in &mut self.records {
                // Attachment owners disable source bodies whose shapes moved to another root.
                if let Key::Source(id) = record.key {
                    if let Some(state) = &mut record.state {
                        state.enabled = physics.rapier().bodies[area.bodies[&id]].is_enabled();
                    }
                }
                if record.state.is_some() {
                    continue;
                }
                if let Key::Source(id) = record.key {
                    let source = reference
                        .bodies
                        .iter()
                        .find(|body| body.id == id)
                        .expect("source body identity");
                    if source.parent_body.is_none() {
                        record.state = Some(BodyState {
                            pose: AbsolutePose {
                                position: source.pose.translation.map(f64::from),
                                rotation: source.pose.rotation,
                            },
                            linear: [0.; 3],
                            angular: [0.; 3],
                            sleeping: false,
                            enabled: physics.rapier().bodies[area.bodies[&id]].is_enabled(),
                        });
                    }
                }
            }
            self.configured = true;
        }
        for record in &mut self.records {
            match record.key {
                Key::Assembly(id)
                    if area.items.get(id).is_some_and(|i| {
                        !matches!(i.location, ItemLocation::World | ItemLocation::Held)
                    }) =>
                {
                    record.state = None;
                    record.handle = None;
                }
                Key::Source(id) => {
                    let handle = area.bodies[&id];
                    if reference.items.iter().filter(|i| i.body == id).any(|i| {
                        area.items
                            .get(i.id)
                            .is_some_and(|item| item.body.is_some() && item.body != Some(handle))
                    }) {
                        if let Some(state) = &mut record.state {
                            state.enabled = false;
                        }
                    }
                }
                _ => (),
            }
        }
        let mut changed = false;
        self.processed.clear();
        let held = area
            .items
            .iter()
            .find(|item| item.location == ItemLocation::Held)
            .and_then(|item| item.body);
        // Independent bodies decide activity first. Source children follow their physical root.
        for children in [false, true] {
            for index in 0..self.records.len() {
                let key = self.records[index].key;
                let parent = if let Key::Source(id) = key {
                    let mut parent = reference
                        .bodies
                        .iter()
                        .find(|b| b.id == id)
                        .and_then(|b| b.parent_body);
                    let mut root = parent;
                    while let Some(id) = parent {
                        root = Some(id);
                        parent = reference
                            .bodies
                            .iter()
                            .find(|body| body.id == id)
                            .and_then(|body| body.parent_body);
                    }
                    root
                } else {
                    None
                };
                if parent.is_some() != children {
                    continue;
                }
                let handle = match key {
                    Key::Source(id) => area.bodies.get(&id).copied(),
                    Key::Assembly(id) => area
                        .items
                        .get(id)
                        .filter(|item| {
                            matches!(
                                item.location,
                                ItemLocation::World
                                    | ItemLocation::Held
                                    | ItemLocation::Stored(_)
                                    | ItemLocation::Equipped
                            )
                        })
                        .and_then(|item| item.body),
                };
                let Some(handle) = handle else {
                    continue;
                };
                if self.processed.contains(&handle) {
                    continue;
                }
                self.processed.push(handle);
                self.records[index].handle = Some(handle);
                let body = &physics.rapier().bodies[handle];
                let absolute = self.records[index].state.map_or_else(
                    || stream.origin.absolute(body.translation().to_array()),
                    |state| state.pose.position,
                );
                let rotation = self.records[index].state.map_or_else(
                    || *body.rotation(),
                    |state| Rotation::from_array(state.pose.rotation),
                );
                let body_rotation = Pose::from_parts(Vector::ZERO, rotation);
                let extent = body
                    .colliders()
                    .iter()
                    .map(|h| {
                        let collider = &physics.rapier().colliders[*h];
                        let pose =
                            body_rotation * *collider.position_wrt_parent().expect("body collider");
                        let bounds = collider.shape().compute_aabb(&pose);
                        bounds.mins.abs().max(bounds.maxs.abs()).max_element()
                    })
                    .fold(0_f32, f32::max) as f64;
                let bounded = [0, 1, 2].iter().all(|&axis| {
                    (absolute[axis] - stream.origin.offset[axis]).abs() + extent
                        <= REBASE_LIMIT as f64
                }) && [0, 2].iter().all(|&axis| {
                    (absolute[axis] - stream.focus()[axis]).abs() + extent <= REBASE_LIMIT as f64
                });
                let pending_attachment = area
                    .items
                    .iter()
                    .any(|item| item.body == Some(handle) && item.pending_attachment);
                let inventory = area.inventory_body(handle);
                let stored = matches!(inventory, Some(ItemLocation::Stored(_)));
                let active = if pending_attachment {
                    false
                } else if matches!(
                    inventory,
                    Some(ItemLocation::Stored(_) | ItemLocation::Equipped | ItemLocation::Held)
                ) {
                    true
                } else if let Some(parent) = parent {
                    area.bodies
                        .get(&parent)
                        .is_some_and(|h| physics.body_active(*h))
                } else {
                    Some(handle) == held
                        || area
                            .player
                            .seat_vehicle()
                            .is_some_and(|vehicle| handle == physics.chassis_handle(vehicle))
                        || (bounded
                            && stream.supports(crate::world::WorldBounds {
                                min: absolute.map(|v| v - extent - 2.),
                                max: absolute.map(|v| v + extent + 2.),
                            }))
                };
                if active {
                    if let Some(state) = self.records[index].state.take() {
                        changed = true;
                        physics.edit_body(handle, |body| {
                            let pose = to_rapier(VehiclePose {
                                position: stream.origin.local(state.pose.position),
                                rotation: state.pose.rotation,
                            });
                            body.set_position(pose, false);
                            body.set_next_kinematic_position(pose);
                            body.set_enabled(state.enabled);
                            body.set_linvel(Vector::from_array(state.linear), false);
                            body.set_angvel(Vector::from_array(state.angular), false);
                            if state.sleeping {
                                body.sleep();
                            } else {
                                body.wake_up(true);
                            }
                        });
                        physics.inactive_bodies.remove(&handle);
                        for item in area
                            .items
                            .iter_mut()
                            .filter(|item| item.body == Some(handle))
                        {
                            item.inactive_pose = None;
                        }
                    }
                    if stored {
                        physics.edit_body(handle, |body| body.set_enabled(false));
                    }
                } else if physics.body_active(handle) {
                    changed = true;
                    let pose = from_rapier(body.position());
                    let state = self.records[index].state.unwrap_or(BodyState {
                        pose: AbsolutePose {
                            position: absolute,
                            rotation: pose.rotation,
                        },
                        linear: body.linvel().to_array(),
                        angular: body.angvel().to_array(),
                        sleeping: body.is_sleeping(),
                        enabled: body.is_enabled(),
                    });
                    for item in area
                        .items
                        .iter_mut()
                        .filter(|item| item.body == Some(handle))
                    {
                        let offset = Rotation::from_array(state.pose.rotation)
                            * Vector::from_array(item.local_pose.position);
                        item.inactive_pose = Some(
                            self.pending_items
                                .iter()
                                .find(|(id, _)| *id == item.id)
                                .map_or(
                                    AbsolutePose {
                                        position: std::array::from_fn(|axis| {
                                            absolute[axis] + offset[axis] as f64
                                        }),
                                        rotation: (Rotation::from_array(state.pose.rotation)
                                            * Rotation::from_array(item.local_pose.rotation))
                                        .to_array(),
                                    },
                                    |(_, pose)| *pose,
                                ),
                        );
                    }
                    self.records[index].state = Some(state);
                    physics.inactive_bodies.insert(handle);
                    physics.edit_body(handle, |body| {
                        body.set_enabled(false);
                        body.set_translation(Vector::ZERO, false);
                        body.set_linvel(Vector::ZERO, false);
                        body.set_angvel(Vector::ZERO, false);
                    });
                }
            }
        }
        self.pending_items.clear();
        if changed {
            wheels.sync_activity(physics);
            area.items.sync_poses(physics);
            area.interaction.sync_mechanisms(physics);
            area.sync_attachments(physics, false);
            area.commit_poses(physics, true);
            physics.refresh_queries();
        }
        Ok(())
    }
}
