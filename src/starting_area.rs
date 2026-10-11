//! Persistent imported home bodies and preparation state.
use crate::interaction::{
    AimTarget, InteractionAction, InteractionState,
    items::{ItemLocation, ItemStore, node_pose},
    parts::PartSlots,
};
use crate::player::PlayerState;
use crate::reference::{BodyId, ItemId, NodeId, ReferenceError, ReferenceManifest};
use crate::simulation::{
    DriveInput, FIXED_DT, FrameInput,
    engine::{EngineInput, EngineOutput, EngineState},
    keys,
};
use crate::vehicle::{
    PhysicsWorld, VEHICLE_COLLIDER_FLAG, VehiclePose, collider_builder, to_rapier,
};
use crate::world::WorldStream;
use rapier3d::prelude::{
    Group, InteractionGroups, InteractionTestMode, RigidBodyBuilder, RigidBodyHandle,
};
use std::collections::HashMap;

mod runtime;
pub use runtime::GarageRuntime;
pub(crate) use runtime::start_sample;

struct CompoundCollider {
    handle: crate::vehicle::ColliderHandle,
    body: BodyId,
    fixed_pose: rapier3d::math::Pose,
    shape_pose: rapier3d::math::Pose,
}

impl CompoundCollider {
    fn local_pose(&self, interaction: &InteractionState) -> rapier3d::math::Pose {
        interaction
            .hinge_local_pose(self.body)
            .unwrap_or(self.fixed_pose)
            * self.shape_pose
    }
}

pub struct StartingArea {
    pub bodies: HashMap<BodyId, RigidBodyHandle>,
    pub items: ItemStore,
    pub interaction: InteractionState,
    pub player: PlayerState,
    pub origin: [f64; 3],
    attachments: Vec<(RigidBodyHandle, RigidBodyHandle, rapier3d::math::Pose)>,
    pub render_poses: Vec<(BodyId, VehiclePose, VehiclePose)>,
    pub item_render_poses: Vec<(ItemId, VehiclePose, VehiclePose)>,
    compound_colliders: Vec<CompoundCollider>,
    portable_mass: Vec<(RigidBodyHandle, f32, Vec<ItemId>, f32)>,
}

impl StartingArea {
    pub fn new(
        reference: &ReferenceManifest,
        physics: &mut PhysicsWorld,
        origin: [f64; 3],
    ) -> Result<Self, ReferenceError> {
        let mut items = ItemStore::new(reference)?;
        let mut interaction = InteractionState::new(reference)?;
        let mut bodies = HashMap::with_capacity(reference.bodies.len());
        let mut compound_colliders = Vec::new();
        bodies.insert(reference.vehicle.body, physics.chassis_handle());
        for body in &reference.bodies {
            if body.id == reference.vehicle.body {
                continue;
            }
            let pose = VehiclePose {
                position: std::array::from_fn(|i| body.pose.translation[i] - origin[i] as f32),
                rotation: body.pose.rotation,
            };
            let builder = if body.parent_body.is_some() {
                RigidBodyBuilder::kinematic_position_based()
            } else if body.dynamic {
                RigidBodyBuilder::dynamic()
                    .ccd_enabled(true)
                    .additional_mass(body.mass_kg)
                    .linear_damping(body.linear_damping)
                    .angular_damping(body.angular_damping)
            } else {
                RigidBodyBuilder::fixed()
            };
            let handle = physics
                .rapier_mut()
                .insert_body(builder.pose(to_rapier(pose)));
            bodies.insert(body.id, handle);
        }
        for record in &reference.colliders {
            if reference.vehicle.colliders.contains(&record.id) {
                continue;
            }
            let item = collider_item(reference, record.node);
            let flag = if belongs_to_vehicle(reference, record.body) {
                VEHICLE_COLLIDER_FLAG
            } else {
                0
            };
            let builder = collider_builder(&record.geometry)?
                .friction(record.friction)
                .restitution(record.restitution)
                .sensor(record.sensor)
                .enabled(record.enabled)
                .density(0.0)
                .collision_groups(InteractionGroups::new(
                    Group::from_bits_truncate(record.membership),
                    Group::from_bits_truncate(record.filter),
                    InteractionTestMode::And,
                ))
                .active_events(rapier3d::prelude::ActiveEvents::COLLISION_EVENTS)
                .user_data(
                    flag | ((record.id.0 as u128) << 64)
                        | item.map_or(0, |item| item.0 as u128 + 1),
                );
            let owner = bodies[&record.body];
            let chassis = bodies[&reference.vehicle.body];
            let mut collider = builder.build();
            let local = *collider.position();
            let physical_parent = if flag != 0 && owner != chassis {
                collider.set_position(
                    physics.rapier().bodies[chassis].position().inverse()
                        * *physics.rapier().bodies[owner].position()
                        * local,
                );
                chassis
            } else {
                owner
            };
            let rapier = physics.rapier_mut();
            let handle =
                rapier
                    .colliders
                    .insert_with_parent(collider, physical_parent, &mut rapier.bodies);
            if physical_parent != owner {
                let source = reference
                    .bodies
                    .iter()
                    .find(|body| body.id == record.body)
                    .unwrap();
                let parent = reference
                    .bodies
                    .iter()
                    .find(|body| body.id == reference.vehicle.body)
                    .unwrap();
                compound_colliders.push(CompoundCollider {
                    handle,
                    body: record.body,
                    fixed_pose: to_rapier(VehiclePose {
                        position: parent.pose.translation,
                        rotation: parent.pose.rotation,
                    })
                    .inverse()
                        * to_rapier(VehiclePose {
                            position: source.pose.translation,
                            rotation: source.pose.rotation,
                        }),
                    shape_pose: local,
                });
            }
        }
        // The vehicle constructor owns these shapes. Add only their aim identity.
        for (_, collider) in physics.rapier_mut().colliders.iter_mut() {
            if collider.parent() == Some(bodies[&reference.vehicle.body]) {
                let source = (collider.user_data >> 64) as u32;
                if let Some(record) = reference
                    .colliders
                    .iter()
                    .find(|record| record.id.0 == source)
                {
                    collider.user_data |=
                        collider_item(reference, record.node).map_or(0, |id| id.0 as u128 + 1);
                }
            }
        }
        for (&id, &handle) in &bodies {
            items.bind_body(id, handle);
            interaction.bind_body(id, handle);
        }
        for slot in &reference.wheel_slots {
            let builder = rapier3d::prelude::ColliderBuilder::ball(slot.radius)
                .sensor(true)
                .enabled(false)
                .density(0.0)
                .user_data(
                    VEHICLE_COLLIDER_FLAG
                        | crate::vehicle::SLOT_SENSOR_FLAG
                        | ((slot.id.0 as u128) << 96)
                        | (slot.wheel.0 as u128 + 1),
                );
            let rapier = physics.rapier_mut();
            rapier.colliders.insert(builder);
        }
        interaction.rebase(origin.map(|v| v as f32));
        interaction.sync_hinges(physics);
        for collider in &compound_colliders {
            physics.rapier_mut().colliders[collider.handle]
                .set_position_wrt_parent(collider.local_pose(&interaction));
        }
        physics.refresh_queries();
        items.sync_poses(physics);
        let spawn = &reference.area.player_spawn;
        let mut player = PlayerState::new(
            &reference.player,
            VehiclePose {
                position: std::array::from_fn(|i| spawn.translation[i] - origin[i] as f32),
                rotation: spawn.rotation,
            },
            physics,
        );
        player.configure_seats(&reference.seats);
        interaction.bind_player_body(player.body_handle());
        let attachments = reference
            .bodies
            .iter()
            .filter_map(|record| {
                let parent = record.parent_body?;
                if reference.hinges.iter().any(|hinge| hinge.body == record.id) {
                    return None;
                }
                let parent_record = reference
                    .bodies
                    .iter()
                    .find(|body| body.id == parent)
                    .expect("validated parent");
                let source = to_rapier(VehiclePose {
                    position: record.pose.translation,
                    rotation: record.pose.rotation,
                });
                let parent_source = to_rapier(VehiclePose {
                    position: parent_record.pose.translation,
                    rotation: parent_record.pose.rotation,
                });
                Some((
                    bodies[&record.id],
                    bodies[&parent],
                    parent_source.inverse() * source,
                ))
            })
            .collect();
        let render_poses = bodies
            .iter()
            .map(|(&id, &handle)| {
                let pose = crate::vehicle::from_rapier(physics.rapier().bodies[handle].position());
                (id, pose, pose)
            })
            .collect();
        let item_render_poses = items
            .iter()
            .map(|item| (item.id, item.pose, item.pose))
            .collect();
        let portable_mass = reference
            .bodies
            .iter()
            .filter_map(|body| {
                if !body.dynamic || body.id == reference.vehicle.body {
                    return None;
                }
                let tanks: Vec<_> = reference
                    .tanks
                    .iter()
                    .filter(|tank| {
                        tank.role == crate::reference::TankRole::Portable
                            && reference
                                .items
                                .iter()
                                .any(|item| item.id == tank.item && item.body == body.id)
                    })
                    .map(|tank| tank.item)
                    .collect();
                if tanks.is_empty() {
                    return None;
                }
                let volume: f32 = tanks
                    .iter()
                    .map(|id| {
                        items
                            .get(*id)
                            .unwrap()
                            .fluid
                            .as_ref()
                            .unwrap()
                            .mixture
                            .total()
                    })
                    .sum();
                Some((bodies[&body.id], body.mass_kg - volume, tanks, body.mass_kg))
            })
            .collect();
        Ok(Self {
            bodies,
            items,
            interaction,
            player,
            origin,
            attachments,
            render_poses,
            item_render_poses,
            compound_colliders,
            portable_mass,
        })
    }

    pub fn sync_origin(&mut self, origin: [f64; 3]) {
        let shift = std::array::from_fn(|i| (origin[i] - self.origin[i]) as f32);
        self.player.rebase(shift);
        self.interaction.rebase(shift);
        self.items.rebase(shift);
        for (_, previous, current) in &mut self.render_poses {
            for (i, shift) in shift.iter().enumerate() {
                previous.position[i] -= shift;
                current.position[i] -= shift;
            }
        }
        for (_, previous, current) in &mut self.item_render_poses {
            for (i, shift) in shift.iter().enumerate() {
                previous.position[i] -= shift;
                current.position[i] -= shift;
            }
        }
        self.origin = origin;
    }

    pub fn begin_step(&mut self) {
        for (_, previous, current) in &mut self.render_poses {
            *previous = *current;
        }
        for (_, previous, current) in &mut self.item_render_poses {
            *previous = *current;
        }
    }
    pub fn sync_portable_mass(&mut self, physics: &mut PhysicsWorld) {
        for (handle, dry, tanks, current) in &mut self.portable_mass {
            let mass = *dry
                + tanks
                    .iter()
                    .map(|id| {
                        self.items
                            .get(*id)
                            .expect("portable tank")
                            .fluid
                            .as_ref()
                            .expect("portable fluid")
                            .mixture
                            .total()
                    })
                    .sum::<f32>();
            if (mass - *current).abs() > 0.0001 {
                physics.rapier_mut().bodies[*handle].set_additional_mass(mass, true);
                *current = mass;
            }
        }
    }
    pub fn sync_compound(&self, physics: &mut PhysicsWorld) {
        let chassis = physics.chassis_handle();
        for record in &self.compound_colliders {
            let local = record.local_pose(&self.interaction);
            let collider = &mut physics.rapier_mut().colliders[record.handle];
            // A removed part carries its shapes on its own body.
            if collider.parent() != Some(chassis) {
                continue;
            }
            if collider.position_wrt_parent() != Some(&local) {
                collider.set_position_wrt_parent(local);
            }
        }
    }
    pub fn commit_poses(&mut self, physics: &PhysicsWorld, teleport: bool) {
        for (id, previous, current) in &mut self.render_poses {
            *current =
                crate::vehicle::from_rapier(physics.rapier().bodies[self.bodies[id]].position());
            if teleport {
                *previous = *current;
            }
        }
        for (id, previous, current) in &mut self.item_render_poses {
            *current = self.items.get(*id).expect("bound visual item").pose;
            if teleport {
                *previous = *current;
            }
        }
    }

    /// Attached bodies without hinges follow the authoritative vehicle pose.
    pub fn sync_attachments(&self, physics: &mut PhysicsWorld, next: bool) {
        for &(handle, parent, local) in &self.attachments {
            let parent_pose = if next {
                *physics.rapier().bodies[parent].next_position()
            } else {
                *physics.rapier().bodies[parent].position()
            };
            let body = &mut physics.rapier_mut().bodies[handle];
            let pose = parent_pose * local;
            if !next {
                body.set_position(pose, false);
            }
            body.set_next_kinematic_position(pose);
        }
    }
}

pub fn belongs_to_vehicle(reference: &ReferenceManifest, mut id: BodyId) -> bool {
    loop {
        if id == reference.vehicle.body {
            return true;
        }
        match reference
            .bodies
            .iter()
            .find(|body| body.id == id)
            .and_then(|body| body.parent_body)
        {
            Some(parent) => id = parent,
            None => return false,
        }
    }
}

pub fn collider_item(reference: &ReferenceManifest, mut node: NodeId) -> Option<ItemId> {
    if let Some(seat) = reference
        .seats
        .iter()
        .find(|seat| seat.hit_nodes.contains(&node))
    {
        return Some(seat.item);
    }
    loop {
        if let Some(item) = reference.items.iter().find(|item| item.node == node) {
            return Some(item.id);
        }
        node = reference
            .nodes
            .iter()
            .find(|record| record.id == node)?
            .parent?;
    }
}

pub fn initial_vehicle_pose(reference: &ReferenceManifest) -> Result<VehiclePose, ReferenceError> {
    node_pose(reference, reference.vehicle.root)
}
