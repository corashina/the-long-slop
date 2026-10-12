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
    seed: u64,
    pumps: Vec<(i64, f32, Option<ItemId>)>,
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
            seed: crate::world::DEFAULT_SEED,
            pumps: Vec::new(),
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
        let seed = super::identity::unsigned(&snapshot.world.seed)?;
        let pumps = snapshot
            .world
            .records
            .iter()
            .map(|WorldRecord::Pump { instance, valve }| {
                let (record_seed, index) = instance.station()?;
                if seed != record_seed {
                    return Err(SaveError::InvalidLinks("Station seed".into()));
                }
                Ok((index, *valve, None))
            })
            .collect::<Result<Vec<_>, SaveError>>()?;
        Ok(Self {
            processed: Vec::with_capacity(records.len()),
            records,
            seed,
            pumps,
            pending_items: snapshot
                .items
                .iter()
                .map(|item| Ok((identities.item(&item.instance)?, item.pose)))
                .collect::<Result<_, SaveError>>()?,
            configured: false,
        })
    }
    pub fn capture(
        &self,
        _area: &StartingArea,
        _physics: &PhysicsWorld,
        _origin: &WorldOrigin,
        _identities: &IdentityMap,
    ) -> Result<WorldSnapshot, SaveError> {
        let mut records = self
            .pumps
            .iter()
            .map(|(index, valve, _)| WorldRecord::Pump {
                instance: InstanceId::Station {
                    seed: self.seed.to_string(),
                    index: index.to_string(),
                    role: "pump".into(),
                },
                valve: *valve,
            })
            .collect::<Vec<_>>();
        records.sort_by(|a, b| {
            let WorldRecord::Pump { instance: a, .. } = a;
            let WorldRecord::Pump { instance: b, .. } = b;
            a.cmp(b)
        });
        Ok(WorldSnapshot {
            seed: self.seed.to_string(),
            records,
        })
    }
    pub fn sync_pumps(
        &mut self,
        stream: &WorldStream,
        interaction: &mut crate::interaction::InteractionState,
    ) {
        self.seed = stream.seed;
        for (index, item) in stream.station_bindings() {
            if let Some((_, valve, binding)) = self.pumps.iter_mut().find(|(id, _, _)| id == index)
            {
                if binding.is_none() {
                    interaction.restore_pump_valve(*item, *valve);
                    *binding = Some(*item);
                } else {
                    *valve = interaction.opening_valve(crate::reference::OpeningId(item.0));
                }
            } else {
                self.pumps.push((
                    *index,
                    interaction.opening_valve(crate::reference::OpeningId(item.0)),
                    Some(*item),
                ));
            }
        }
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
    pub fn relocate_dormant_vehicle(
        &mut self,
        reference: &ReferenceManifest,
        area: &mut StartingArea,
        physics: &PhysicsWorld,
        vehicle: usize,
        target: [f64; 3],
        bounds: &crate::reference::Bounds,
        cargo_bodies: &mut Vec<RigidBodyHandle>,
    ) {
        cargo_bodies.clear();
        let Some(car) = self
            .records
            .iter()
            .find(|r| r.handle == Some(physics.chassis_handle(vehicle)))
            .and_then(|r| r.state)
        else {
            return;
        };
        let inverse = Rotation::from_array(car.pose.rotation).inverse();
        for record in &mut self.records {
            let Some(state) = &mut record.state else {
                continue;
            };
            let delta = std::array::from_fn::<_, 3, _>(|axis| {
                state.pose.position[axis] - car.pose.position[axis]
            });
            let near = delta
                .iter()
                .all(|v| v.abs() <= crate::world::REBASE_LIMIT as f64);
            let point = if near {
                inverse * Vector::from_array(delta.map(|v| v as f32))
            } else {
                Vector::splat(f32::INFINITY)
            };
            let cargo = state.enabled
                && record.handle.is_some_and(|handle| {
                    physics.rapier().bodies[handle].is_dynamic()
                        && physics.vehicle_of(handle).is_none()
                })
                && (0..3)
                    .all(|axis| point[axis] >= bounds.min[axis] && point[axis] <= bounds.max[axis]);
            let attached = match record.key {
                Key::Source(id) => {
                    crate::starting_area::body_vehicle(reference, id) == Some(vehicle)
                }
                Key::Assembly(_) => false,
            };
            if cargo && !attached {
                if let Some(handle) = record.handle {
                    if !cargo_bodies.contains(&handle) {
                        cargo_bodies.push(handle);
                    }
                }
            }
            if !attached && !cargo {
                continue;
            }
            let shift =
                std::array::from_fn::<_, 3, _>(|axis| target[axis] - car.pose.position[axis]);
            for axis in 0..3 {
                state.pose.position[axis] += shift[axis];
            }
            for item in area
                .items
                .iter_mut()
                .filter(|item| item.body == record.handle)
            {
                if let Some(pose) = &mut item.inactive_pose {
                    for axis in 0..3 {
                        pose.position[axis] += shift[axis];
                    }
                }
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
        self.seed = stream.seed;
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
                            matches!(item.location, ItemLocation::World | ItemLocation::Held)
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
                    (absolute[axis] - stream.focus[axis]).abs() + extent <= REBASE_LIMIT as f64
                });
                let active = if let Some(parent) = parent {
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
                            && (physics.vehicle_of(handle).is_some()
                                || stream.contains_active(absolute)))
                };
                if active {
                    if let Some(state) = self.records[index].state.take() {
                        changed = true;
                        let body = &mut physics.rapier_mut().bodies[handle];
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
                        physics.inactive_bodies.remove(&handle);
                        for item in area
                            .items
                            .iter_mut()
                            .filter(|item| item.body == Some(handle))
                        {
                            item.inactive_pose = None;
                        }
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
                    let body = &mut physics.rapier_mut().bodies[handle];
                    body.set_enabled(false);
                    body.set_translation(Vector::ZERO, false);
                    body.set_linvel(Vector::ZERO, false);
                    body.set_angvel(Vector::ZERO, false);
                }
            }
        }
        self.pending_items.clear();
        self.sync_pumps(stream, &mut area.interaction);
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
