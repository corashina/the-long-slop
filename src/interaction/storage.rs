//! Source physics locks for portable flashlights.
use super::{
    items::{ItemLocation, ItemStore, node_pose},
    mechanisms::MechanismState,
    parts::PartAttachments,
};
use crate::reference::{
    ColliderGeometry, ColliderId, ItemId, PartSlotId, ReferenceError, ReferenceManifest,
};
use crate::vehicle::{PhysicsWorld, VEHICLE_COLLIDER_FLAG, from_rapier, to_rapier};
use rapier3d::math::{Pose, Vector};
use rapier3d::prelude::{ColliderHandle, QueryFilter, RigidBodyHandle, SharedShape};

struct Shape {
    id: ColliderId,
    handle: Option<ColliderHandle>,
    loose_local: Pose,
    enabled: bool,
}
struct Flashlight {
    item: ItemId,
    mass: f32,
    active: bool,
    body: Option<RigidBodyHandle>,
    owner: Option<ItemId>,
    volume: Option<usize>,
    loose_local: Pose,
    shapes: Vec<Shape>,
    dropped_at: f32,
}
struct Volume {
    owner: ItemId,
    storage_type: u8,
    active: bool,
    slots: Vec<PartSlotId>,
    usables: Vec<ItemId>,
    collider: ColliderId,
    handle: Option<ColliderHandle>,
    shape: SharedShape,
    previous: Vec<ColliderHandle>,
    current: Vec<ColliderHandle>,
}
pub struct StorageLocks {
    flashlights: Vec<Flashlight>,
    volumes: Vec<Volume>,
    part_usables: Vec<(ItemId, ItemId)>,
    mount_mask: u32,
    drop_delay: f32,
    scan_interval: f32,
    elapsed: f32,
    last_scan: f32,
    startup_frame: u8,
    bound: bool,
}
impl StorageLocks {
    pub fn new(reference: &ReferenceManifest) -> Result<Self, ReferenceError> {
        let mut flashlights = Vec::with_capacity(reference.flashlights.instances.len());
        for instance in &reference.flashlights.instances {
            let item = reference
                .items
                .iter()
                .find(|i| i.id == instance.item)
                .ok_or_else(|| error("Missing flashlight item"))?;
            let body = reference
                .bodies
                .iter()
                .find(|b| b.id == item.body)
                .ok_or_else(|| error("Missing flashlight body"))?;
            let body_pose = to_rapier(crate::vehicle::VehiclePose {
                position: body.pose.translation,
                rotation: body.pose.rotation,
            });
            let item_pose = to_rapier(node_pose(reference, item.node)?);
            let shapes = instance
                .colliders
                .iter()
                .map(|id| {
                    let record = reference
                        .colliders
                        .iter()
                        .find(|c| c.id == *id)
                        .ok_or_else(|| error("Missing flashlight shape"))?;
                    Ok(Shape {
                        id: *id,
                        handle: None,
                        loose_local: Pose::IDENTITY,
                        enabled: record.enabled,
                    })
                })
                .collect::<Result<Vec<_>, ReferenceError>>()?;
            flashlights.push(Flashlight {
                item: item.id,
                mass: item.mass_kg,
                active: instance.startup_active,
                body: None,
                owner: None,
                volume: None,
                loose_local: body_pose.inverse() * item_pose,
                shapes,
                dropped_at: 0.0,
            });
        }
        let mut volumes = Vec::with_capacity(reference.flashlights.storage.volumes.len());
        for record in &reference.flashlights.storage.volumes {
            let collider = reference
                .colliders
                .iter()
                .find(|c| c.id == record.collider)
                .ok_or_else(|| error("Missing storage collider"))?;
            let shape = match &collider.geometry {
                ColliderGeometry::Convex { points } => {
                    let vertices: Vec<_> = points.iter().copied().map(Vector::from_array).collect();
                    SharedShape::convex_hull(&vertices)
                        .ok_or_else(|| error("Invalid storage box vertices"))?
                }
                ColliderGeometry::Box { half_extents, .. } => {
                    SharedShape::cuboid(half_extents[0], half_extents[1], half_extents[2])
                }
                _ => return Err(error("Storage bounds must be a box")),
            };
            let usables = record
                .usables
                .iter()
                .map(|source| {
                    reference
                        .mechanisms
                        .iter()
                        .find(|m| m.source == *source)
                        .map(|m| m.item)
                        .ok_or_else(|| error("Missing storage usable"))
                })
                .collect::<Result<Vec<_>, ReferenceError>>()?;
            volumes.push(Volume {
                owner: record.owner,
                storage_type: record.storage_type,
                active: record.active,
                slots: record.slots.clone(),
                usables,
                collider: record.collider,
                handle: None,
                shape,
                previous: Vec::with_capacity(reference.colliders.len()),
                current: Vec::with_capacity(reference.colliders.len()),
            });
        }
        let policy = &reference.flashlights.storage;
        Ok(Self {
            part_usables: reference
                .mechanisms
                .iter()
                .filter_map(|m| m.parent_item.map(|parent| (parent, m.item)))
                .collect(),
            flashlights,
            volumes,
            mount_mask: policy.mount_mask,
            drop_delay: policy.drop_delay_seconds,
            scan_interval: policy.scan_interval_seconds,
            elapsed: 0.0,
            last_scan: 0.0,
            startup_frame: 0,
            bound: false,
        })
    }
    pub fn bind(
        &mut self,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
    ) -> Result<(), ReferenceError> {
        if self.bound {
            return Ok(());
        }
        for light in &mut self.flashlights {
            let body = items
                .get(light.item)
                .and_then(|i| i.body)
                .ok_or_else(|| error("Missing flashlight body binding"))?;
            light.body = Some(body);
            light.loose_local = to_rapier(items.get(light.item).unwrap().local_pose);
            for shape in &mut light.shapes {
                let (handle, collider) = physics
                    .rapier()
                    .colliders
                    .iter()
                    .find(|(_, c)| ColliderId((c.user_data >> 64) as u32) == shape.id)
                    .ok_or_else(|| error("Missing flashlight collider binding"))?;
                shape.handle = Some(handle);
                shape.loose_local =
                    physics.rapier().bodies[body].position().inverse() * *collider.position();
                physics.edit_collider(handle, |collider| collider.set_density(0.0));
            }
            if !light.active {
                physics.edit_body(body, |body| body.set_enabled(false));
            }
        }
        for volume in &mut self.volumes {
            volume.handle = physics
                .rapier()
                .colliders
                .iter()
                .find(|(_, c)| ColliderId((c.user_data >> 64) as u32) == volume.collider)
                .map(|(h, _)| h);
            if volume.handle.is_none() {
                return Err(error("Missing storage volume binding"));
            }
        }
        self.bound = true;
        Ok(())
    }
    pub fn contains(&self, item: ItemId) -> bool {
        self.flashlights.iter().any(|f| f.item == item)
    }
    pub fn active(&self, item: ItemId) -> bool {
        self.flashlights.iter().any(|f| f.item == item && f.active)
    }
    pub fn stored(&self, item: ItemId) -> bool {
        self.flashlights
            .iter()
            .any(|f| f.item == item && f.owner.is_some())
    }
    pub fn loose_body(&self, item: ItemId) -> Option<RigidBodyHandle> {
        self.flashlights
            .iter()
            .find(|f| f.item == item && f.active)?
            .body
    }
    pub fn owns_collider(&self, id: ColliderId) -> bool {
        self.flashlights
            .iter()
            .any(|f| f.shapes.iter().any(|s| s.id == id))
    }
    pub fn mark_dropped(&mut self, item: ItemId) {
        if let Some(light) = self.flashlights.iter_mut().find(|f| f.item == item) {
            light.dropped_at = self.elapsed;
        }
    }
    pub fn rebase(&mut self, _shift: [f32; 3]) {
        // Physics rebases body poses. Volumes follow their parent bodies.
    }
    fn permitted(
        volume: &Volume,
        items: &ItemStore,
        parts: &PartAttachments,
        mechanisms: &MechanismState,
        part_usables: &[(ItemId, ItemId)],
    ) -> bool {
        if !volume.active || !items.mounted(volume.owner) || matches!(volume.storage_type, 5 | 7) {
            return false;
        }
        for slot in &volume.slots {
            let Some(item) = parts.slot_item(*slot) else {
                return false;
            };
            if let Some((_, usable)) = part_usables.iter().find(|(part, _)| *part == item) {
                if mechanisms
                    .value(*usable, false)
                    .is_some_and(|value| value > 0.1)
                {
                    return false;
                }
            }
        }
        for usable in &volume.usables {
            if mechanisms
                .value(*usable, false)
                .is_some_and(|value| value > 0.1)
            {
                return false;
            }
        }
        true
    }
    fn scan_volume(volume: &mut Volume, physics: &PhysicsWorld, mount_mask: u32) {
        volume.current.clear();
        let handle = volume.handle.unwrap();
        let pose = *physics.rapier().colliders[handle].position();
        let predicate = |_, collider: &rapier3d::prelude::Collider| {
            collider.is_enabled()
                && collider.collision_groups().memberships.bits() & mount_mask != 0
        };
        let filter = QueryFilter::default().predicate(&predicate);
        for (handle, _) in physics
            .queries(filter)
            .intersect_shape(pose, volume.shape.as_ref())
        {
            volume.current.push(handle);
        }
    }
    fn lock(
        &mut self,
        index: usize,
        volume_index: usize,
        owner: ItemId,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
    ) -> Result<(), ReferenceError> {
        let light = &mut self.flashlights[index];
        let body = light
            .body
            .ok_or_else(|| error("Flashlight has no loose body"))?;
        let parent = items
            .get(owner)
            .and_then(|i| i.body)
            .ok_or_else(|| error("Storage owner has no body"))?;
        let item_pose = *physics.rapier().bodies[body].position() * light.loose_local;
        let parent_pose = *physics.rapier().bodies[parent].position();
        let vehicle = physics.vehicle_of(parent);
        for shape in &light.shapes {
            let handle = shape.handle.unwrap();
            let world_pose = *physics.rapier().colliders[handle].position();

            physics.set_collider_parent(handle, Some(parent));
            physics.edit_collider(handle, |collider| {
                collider.set_position_wrt_parent(parent_pose.inverse() * world_pose)
            });
            if vehicle.is_some() {
                physics.edit_collider(handle, |collider| {
                    collider.user_data |= VEHICLE_COLLIDER_FLAG;
                });
            } else {
                physics.edit_collider(handle, |collider| {
                    collider.user_data &= !VEHICLE_COLLIDER_FLAG;
                });
            }
        }
        physics.edit_body(body, |body| body.set_enabled(false));
        if let Some(vehicle) = vehicle {
            let mass = physics.rapier().bodies[parent].mass() + light.mass;
            physics.set_vehicle_mass(vehicle, mass);
        }
        let item = items.set_body(light.item, Some(parent)).unwrap();
        item.location = ItemLocation::Attached(owner);
        item.local_pose = from_rapier(&(parent_pose.inverse() * item_pose));
        item.pose = from_rapier(&item_pose);
        light.owner = Some(owner);
        light.volume = Some(volume_index);
        Ok(())
    }
    pub fn unlock(
        &mut self,
        item: ItemId,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
    ) -> Result<(), ReferenceError> {
        let light = self
            .flashlights
            .iter_mut()
            .find(|f| f.item == item && f.owner.is_some())
            .ok_or_else(|| error("Flashlight is not stored"))?;
        let parent = items
            .get(item)
            .and_then(|i| i.body)
            .ok_or_else(|| error("Missing storage parent body"))?;
        let item_pose = *physics.rapier().bodies[parent].position()
            * to_rapier(items.get(item).unwrap().local_pose);
        let loose_pose = item_pose * light.loose_local.inverse();
        let velocity = physics.rapier().bodies[parent].velocity_at_point(loose_pose.translation);
        let angular = physics.rapier().bodies[parent].angvel();
        let body = light.body.unwrap();
        if let Some(vehicle) = physics.vehicle_of(parent) {
            let mass = (physics.rapier().bodies[parent].mass() - light.mass).max(0.001);
            physics.set_vehicle_mass(vehicle, mass);
        }
        {
            physics.edit_body(body, |loose| {
                loose.set_position(loose_pose, true);
                loose.set_enabled(true);
                loose.set_linvel(velocity, true);
                loose.set_angvel(angular, true);
            });
        }
        for shape in &light.shapes {
            let handle = shape.handle.unwrap();

            physics.set_collider_parent(handle, Some(body));
            physics.edit_collider(handle, |collider| {
                collider.set_position_wrt_parent(shape.loose_local)
            });
            physics.edit_collider(handle, |collider| collider.set_enabled(shape.enabled));
            physics.edit_collider(handle, |collider| {
                collider.user_data &= !VEHICLE_COLLIDER_FLAG;
            });
        }
        let state = items.set_body(item, Some(body)).unwrap();
        state.location = ItemLocation::World;
        state.local_pose = from_rapier(&light.loose_local);
        state.pose = from_rapier(&item_pose);
        light.owner = None;
        light.volume = None;
        light.dropped_at = self.elapsed;
        Ok(())
    }
    pub fn initialization_ready(&self) -> bool {
        self.startup_frame == 5
    }

    /// Run the two source yields, then one QuickFreeze pass per loading frame.
    pub fn initialize_frame(
        &mut self,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
        parts: &PartAttachments,
        mechanisms: &MechanismState,
        dt: f32,
    ) -> Result<bool, ReferenceError> {
        if !self.bound || !dt.is_finite() || dt < 0.0 {
            return Err(error("Invalid storage initialization frame"));
        }
        if self.initialization_ready() {
            return Ok(false);
        }
        self.elapsed += dt;
        if self.startup_frame < 2 {
            self.startup_frame += 1;
            return Ok(false);
        }
        physics.refresh_queries();
        if self.startup_frame == 2 {
            for volume in &mut self.volumes {
                Self::scan_volume(volume, physics, self.mount_mask);
                std::mem::swap(&mut volume.previous, &mut volume.current);
            }
        }
        let changed = self.advance(items, physics, parts, mechanisms, 0.0)?;
        self.startup_frame += 1;
        Ok(changed)
    }
    pub fn advance(
        &mut self,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
        parts: &PartAttachments,
        mechanisms: &MechanismState,
        dt: f32,
    ) -> Result<bool, ReferenceError> {
        if !self.bound || !dt.is_finite() || dt < 0.0 {
            return Err(error("Invalid storage step"));
        }
        self.elapsed += dt;
        if dt > 0.0 && self.elapsed - self.last_scan <= self.scan_interval {
            return Ok(false);
        }
        self.last_scan = self.elapsed;
        let mut changed = false;
        for volume_index in 0..self.volumes.len() {
            let eligible = Self::permitted(
                &self.volumes[volume_index],
                items,
                parts,
                mechanisms,
                &self.part_usables,
            );
            let owner = self.volumes[volume_index].owner;
            if !eligible {
                for i in 0..self.flashlights.len() {
                    if self.flashlights[i].volume == Some(volume_index) {
                        self.unlock(self.flashlights[i].item, items, physics)?;
                        changed = true;
                    }
                }
            }
            {
                let volume = &mut self.volumes[volume_index];
                Self::scan_volume(volume, physics, self.mount_mask);
            }
            if eligible {
                for i in 0..self.flashlights.len() {
                    let light = &self.flashlights[i];
                    if !light.active
                        || light.owner.is_some()
                        || self.elapsed - light.dropped_at < self.drop_delay
                        || items
                            .get(light.item)
                            .is_none_or(|item| item.location != ItemLocation::World)
                        || owner == light.item
                        || items.mounted(light.item)
                    {
                        continue;
                    }
                    let retained = light.shapes.iter().any(|s| {
                        let h = s.handle.unwrap();
                        self.volumes[volume_index].current.contains(&h)
                            && self.volumes[volume_index].previous.contains(&h)
                    });
                    if retained {
                        self.lock(i, volume_index, owner, items, physics)?;
                        changed = true;
                    }
                }
            }
            let volume = &mut self.volumes[volume_index];
            std::mem::swap(&mut volume.previous, &mut volume.current);
        }
        if changed {
            physics.refresh_queries();
        }
        Ok(changed)
    }
}
fn error(message: &str) -> ReferenceError {
    ReferenceError(message.into())
}
