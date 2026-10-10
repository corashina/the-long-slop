//! General car parts. A part leaves its slot as a loose body and fits back only into its own slot.

use super::items::{ItemLocation, ItemStore, node_pose};
use crate::reference::{ItemId, ReferenceError, ReferenceManifest};
use crate::vehicle::{PhysicsWorld, VEHICLE_COLLIDER_FLAG, from_rapier, to_rapier};
use rapier3d::math::{Pose, Rotation, Vector};
use rapier3d::prelude::{
    ColliderBuilder, ColliderHandle, RigidBodyBuilder, RigidBodyHandle, SharedShape,
};
use std::collections::HashMap;

/// Collider flag for the empty slot of a carried part.
pub const PART_SLOT_FLAG: u128 = 1 << 125;

struct Part {
    item: ItemId,
    parent: ItemId,
    /// Part pose in the rest frame of its parent.
    fitted: Pose,
    /// Pose after removal in the rest frame of its parent.
    detach: Option<Pose>,
    /// Slot sensor pose relative to the fitted part.
    sensor_offset: Pose,
    sensor: ColliderHandle,
    loose: Option<RigidBodyHandle>,
}

struct Member {
    item: ItemId,
    /// Item pose in the source scene.
    rest: Pose,
    home_body: RigidBodyHandle,
    home_local: Pose,
    /// Collider, pose relative to the item, home pose relative to the chassis, and home enable state.
    colliders: Vec<(ColliderHandle, Pose, Pose, bool)>,
}

pub struct PartSlots {
    parts: Vec<Part>,
    members: Vec<Member>,
    indices: HashMap<ItemId, usize>,
    rest: HashMap<ItemId, Pose>,
    subtree: Vec<usize>,
}

impl PartSlots {
    pub fn new(
        reference: &ReferenceManifest,
        items: &ItemStore,
        physics: &mut PhysicsWorld,
    ) -> Result<Self, ReferenceError> {
        let mut rest = HashMap::with_capacity(reference.items.len());
        for item in &reference.items {
            rest.insert(item.id, to_rapier(node_pose(reference, item.node)?));
        }
        let parent_of = |id: ItemId| {
            reference
                .items
                .iter()
                .find(|item| item.id == id)
                .and_then(|item| item.attached_to)
        };
        let below = |mut id: ItemId, root: ItemId| loop {
            if id == root {
                return true;
            }
            match parent_of(id) {
                Some(parent) => id = parent,
                None => return false,
            }
        };
        let mut members = Vec::new();
        let mut indices = HashMap::new();
        for item in &reference.items {
            if !reference.parts.iter().any(|part| below(item.id, part.item)) {
                continue;
            }
            let state = items
                .get(item.id)
                .ok_or_else(|| error("Missing part item"))?;
            let home_body = state.body.ok_or_else(|| error("Missing part body"))?;
            indices.insert(item.id, members.len());
            members.push(Member {
                item: item.id,
                rest: rest[&item.id],
                home_body,
                home_local: to_rapier(state.local_pose),
                colliders: Vec::new(),
            });
        }
        let chassis = physics.chassis_handle();
        let chassis_pose = *physics.rapier().bodies[chassis].position();
        for (handle, collider) in physics.rapier().colliders.iter() {
            let encoded = collider.user_data as u32;
            let Some(&index) = encoded
                .checked_sub(1)
                .and_then(|id| indices.get(&ItemId(id)))
            else {
                continue;
            };
            if collider.parent() != Some(chassis) {
                return Err(error("Part collider must start on the chassis"));
            }
            let member = &mut members[index];
            let state = items.get(member.item).unwrap();
            let item_pose =
                *physics.rapier().bodies[member.home_body].position() * to_rapier(state.local_pose);
            let home = *collider.position_wrt_parent().unwrap();
            member.colliders.push((
                handle,
                item_pose.inverse() * chassis_pose * home,
                home,
                collider.is_enabled(),
            ));
        }
        let mut parts = Vec::with_capacity(reference.parts.len());
        for record in &reference.parts {
            let parent = parent_of(record.item).ok_or_else(|| error("Part has no parent"))?;
            let fitted = rest[&parent].inverse() * rest[&record.item];
            let detach = record
                .detach_node
                .map(|node| {
                    node_pose(reference, node).map(|pose| {
                        let pose = to_rapier(pose);
                        rest[&parent].inverse()
                            * Pose::from_parts(
                                pose.translation,
                                pose.rotation
                                    * Rotation::from_array(record.local_rotation_after_mount),
                            )
                    })
                })
                .transpose()?;
            let mut mins = Vector::splat(f32::MAX);
            let mut maxs = Vector::splat(f32::MIN);
            for member in &members {
                if !below(member.item, record.item) {
                    continue;
                }
                let local = rest[&record.item].inverse() * member.rest;
                for (handle, pose, _, _) in &member.colliders {
                    let aabb = physics.rapier().colliders[*handle]
                        .shape()
                        .compute_aabb(&(local * *pose));
                    mins = mins.min(aabb.mins);
                    maxs = maxs.max(aabb.maxs);
                }
            }
            if mins.x > maxs.x {
                return Err(error("Part has no collider"));
            }
            let half = ((maxs - mins) * 0.5).max(Vector::splat(0.02));
            let sensor = physics.rapier_mut().colliders.insert(
                ColliderBuilder::new(SharedShape::cuboid(half.x, half.y, half.z))
                    .sensor(true)
                    .enabled(false)
                    .density(0.0)
                    .user_data(
                        VEHICLE_COLLIDER_FLAG | PART_SLOT_FLAG | (record.item.0 as u128 + 1),
                    ),
            );
            parts.push(Part {
                item: record.item,
                parent,
                fitted,
                detach,
                sensor_offset: Pose::from_translation((mins + maxs) * 0.5),
                sensor,
                loose: None,
            });
        }
        Ok(Self {
            parts,
            members,
            indices,
            rest,
            subtree: Vec::with_capacity(reference.items.len()),
        })
    }

    pub fn contains(&self, item: ItemId) -> bool {
        self.parts.iter().any(|part| part.item == item)
    }

    /// Removes a fitted part with everything attached to it.
    pub fn detach(
        &mut self,
        item: ItemId,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
    ) -> Result<RigidBodyHandle, ReferenceError> {
        let index = self.part(item)?;
        let part = &self.parts[index];
        if items.get(item).map(|state| state.location) != Some(ItemLocation::Attached(part.parent))
        {
            return Err(error("Part is not fitted"));
        }
        let current = item_pose(items, item, physics)?;
        let world = match part.detach {
            Some(detach) => item_pose(items, part.parent, physics)? * detach,
            None => current,
        };
        let carrier = items.get(item).unwrap().body.unwrap();
        let velocity = physics.rapier().bodies[carrier].velocity_at_point(current.translation);
        let angular = physics.rapier().bodies[carrier].angvel();
        let body = *self.parts[index].loose.get_or_insert_with(|| {
            physics
                .rapier_mut()
                .insert_body(RigidBodyBuilder::dynamic().ccd_enabled(true))
        });
        let state = &mut physics.rapier_mut().bodies[body];
        state.set_enabled(true);
        state.set_position(world, true);
        state.set_linvel(velocity, true);
        state.set_angvel(angular, true);
        state.reset_forces(true);
        state.reset_torques(true);
        self.collect(item, items);
        let root = self.rest[&item].inverse();
        for &member in &self.subtree {
            let local = root * self.members[member].rest;
            self.place(member, body, local, items, physics);
        }
        items.get_mut(item).unwrap().location = ItemLocation::World;
        update_mass(carrier, items, physics);
        update_mass(body, items, physics);
        physics.refresh_queries();
        Ok(body)
    }

    /// Fits a loose part into its own slot.
    pub fn fit(
        &mut self,
        item: ItemId,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
    ) -> Result<(), ReferenceError> {
        let index = self.part(item)?;
        if items.get(item).map(|state| state.location) != Some(ItemLocation::World) {
            return Err(error("Part must be loose before fitting"));
        }
        let parent = self.parts[index].parent;
        let parent_state = items
            .get(parent)
            .ok_or_else(|| error("Missing part parent"))?;
        let parent_body = parent_state
            .body
            .ok_or_else(|| error("Missing part parent body"))?;
        let parent_local = to_rapier(parent_state.local_pose);
        let home = self
            .indices
            .get(&parent)
            .is_none_or(|&member| self.members[member].home_body == parent_body);
        self.collect(item, items);
        for i in 0..self.subtree.len() {
            let member = self.subtree[i];
            if home {
                self.restore(member, items, physics);
            } else {
                let local = parent_local * self.rest[&parent].inverse() * self.members[member].rest;
                self.place(member, parent_body, local, items, physics);
            }
        }
        if let Some(body) = self.parts[index].loose {
            physics.rapier_mut().bodies[body].set_enabled(false);
        }
        items.get_mut(item).unwrap().location = ItemLocation::Attached(parent);
        if !home {
            update_mass(parent_body, items, physics);
        }
        physics.refresh_queries();
        Ok(())
    }

    /// Shows the empty slot of the carried part.
    pub fn sync(
        &self,
        held: Option<ItemId>,
        items: &ItemStore,
        physics: &mut PhysicsWorld,
    ) -> Result<(), ReferenceError> {
        for part in &self.parts {
            let enabled = held == Some(part.item);
            let pose = if enabled {
                Some(item_pose(items, part.parent, physics)? * part.fitted * part.sensor_offset)
            } else {
                None
            };
            let collider = &mut physics.rapier_mut().colliders[part.sensor];
            if collider.is_enabled() != enabled {
                collider.set_enabled(enabled);
            }
            if let Some(pose) = pose
                && *collider.position() != pose
            {
                collider.set_position(pose);
            }
        }
        Ok(())
    }

    fn part(&self, item: ItemId) -> Result<usize, ReferenceError> {
        self.parts
            .iter()
            .position(|part| part.item == item)
            .ok_or_else(|| error("Unknown part"))
    }

    /// Collects the part and the items attached to it.
    fn collect(&mut self, root: ItemId, items: &ItemStore) {
        self.subtree.clear();
        for (index, member) in self.members.iter().enumerate() {
            let mut id = member.item;
            loop {
                if id == root {
                    self.subtree.push(index);
                    break;
                }
                match items.get(id).map(|item| item.location) {
                    Some(ItemLocation::Attached(parent)) => id = parent,
                    _ => break,
                }
            }
        }
    }

    fn place(
        &self,
        member: usize,
        body: RigidBodyHandle,
        local: Pose,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
    ) {
        let member = &self.members[member];
        let state = items.get_mut(member.item).unwrap();
        state.body = Some(body);
        state.local_pose = from_rapier(&local);
        state.pose = from_rapier(&(*physics.rapier().bodies[body].position() * local));
        for &(handle, pose, _, _) in &member.colliders {
            let world = physics.rapier_mut();
            world
                .colliders
                .set_parent(handle, Some(body), &mut world.bodies);
            let collider = &mut world.colliders[handle];
            collider.set_position_wrt_parent(local * pose);
            collider.set_enabled(true);
            collider.user_data &= !VEHICLE_COLLIDER_FLAG;
        }
    }

    fn restore(&self, member: usize, items: &mut ItemStore, physics: &mut PhysicsWorld) {
        let member = &self.members[member];
        let state = items.get_mut(member.item).unwrap();
        state.body = Some(member.home_body);
        state.local_pose = from_rapier(&member.home_local);
        state.pose = from_rapier(
            &(*physics.rapier().bodies[member.home_body].position() * member.home_local),
        );
        let chassis = physics.chassis_handle();
        for &(handle, _, home, enabled) in &member.colliders {
            let world = physics.rapier_mut();
            world
                .colliders
                .set_parent(handle, Some(chassis), &mut world.bodies);
            let collider = &mut world.colliders[handle];
            collider.set_position_wrt_parent(home);
            collider.set_enabled(enabled);
            collider.user_data |= VEHICLE_COLLIDER_FLAG;
        }
    }
}

fn update_mass(body: RigidBodyHandle, items: &ItemStore, physics: &mut PhysicsWorld) {
    if !physics.rapier().bodies[body].is_dynamic() || body == physics.chassis_handle() {
        return;
    }
    let mass = items.body_mass(body);
    let world = physics.rapier_mut();
    world.bodies[body].set_additional_mass(mass, true);
    world.bodies[body].recompute_mass_properties_from_colliders(&world.colliders);
}

fn item_pose(
    items: &ItemStore,
    id: ItemId,
    physics: &PhysicsWorld,
) -> Result<Pose, ReferenceError> {
    let item = items.get(id).ok_or_else(|| error("Missing part item"))?;
    let body = item.body.ok_or_else(|| error("Missing part body"))?;
    Ok(*physics.rapier().bodies[body].position() * to_rapier(item.local_pose))
}

fn error(message: &str) -> ReferenceError {
    ReferenceError(message.into())
}
