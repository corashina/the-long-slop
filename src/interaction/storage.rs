//! Source `mountStuff` physics locks. Any loose item can lock to a storage volume.
use super::{
    items::{ItemLocation, ItemStore},
    mechanisms::MechanismState,
    parts::PartAttachments,
};
use crate::reference::{
    ColliderGeometry, ColliderId, ItemId, PartSlotId, ReferenceError, ReferenceManifest,
};
use crate::vehicle::{PhysicsWorld, VEHICLE_COLLIDER_FLAG, from_rapier, to_rapier};
use rapier3d::math::{Pose, Vector};
use rapier3d::prelude::{ColliderHandle, QueryFilter, RigidBodyHandle, SharedShape};
use std::collections::{HashMap, HashSet};

/// Scan buffer capacity per volume. Distinct items seen in one volume.
const SCAN_CAPACITY: usize = 64;
/// Collider scratch capacity for one lock or unlock.
const COLLIDER_CAPACITY: usize = 256;

/// One locked item. Its own body is disabled while its colliders ride on the carrier.
struct Locked {
    item: ItemId,
    volume: u16,
    body: RigidBodyHandle,
    /// The item pose relative to its own body.
    loose_local: Pose,
    /// Mass of the item body, with the items locked to it.
    mass: f32,
}
struct Volume {
    owner: ItemId,
    storage_type: u8,
    active: bool,
    dont_lock_if_owner_picked_up: bool,
    slots: Vec<PartSlotId>,
    usables: Vec<ItemId>,
    collider: ColliderId,
    handle: Option<ColliderHandle>,
    shape: SharedShape,
    /// Gate slots are filled and gate usables are closed. Set before each scan.
    gates_closed: bool,
    previous: Vec<ItemId>,
    current: Vec<ItemId>,
}
pub struct StorageLocks {
    locked: Vec<Locked>,
    volumes: Vec<Volume>,
    part_usables: Vec<(ItemId, ItemId)>,
    dont_mount: HashSet<ItemId>,
    /// Source `dontMountTillPickup`. A pick-up removes the item. Not saved.
    till_pickup: HashSet<ItemId>,
    drop_times: HashMap<ItemId, f32>,
    colliders: Vec<ColliderHandle>,
    riders: Vec<ItemId>,
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
        let mut volumes = Vec::with_capacity(reference.storage.volumes.len());
        for record in &reference.storage.volumes {
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
                dont_lock_if_owner_picked_up: record.dont_lock_if_owner_picked_up,
                slots: record.slots.clone(),
                usables,
                collider: record.collider,
                handle: None,
                shape,
                gates_closed: false,
                previous: Vec::with_capacity(SCAN_CAPACITY),
                current: Vec::with_capacity(SCAN_CAPACITY),
            });
        }
        let flags = &reference.storage.flags;
        let mut locks =
            Self::with_volumes(volumes, reference.items.len(), &reference.storage.policy);
        locks.part_usables = reference
            .mechanisms
            .iter()
            .filter_map(|m| m.parent_item.map(|parent| (parent, m.item)))
            .collect();
        locks.dont_mount = flags
            .iter()
            .filter(|f| f.dont_mount)
            .map(|f| f.item)
            .collect();
        locks.till_pickup = flags
            .iter()
            .filter(|f| f.dont_mount_till_pickup)
            .map(|f| f.item)
            .collect();
        Ok(locks)
    }
    fn with_volumes(
        volumes: Vec<Volume>,
        items: usize,
        policy: &crate::reference::StoragePolicy,
    ) -> Self {
        Self {
            locked: Vec::with_capacity(items),
            volumes,
            part_usables: Vec::new(),
            dont_mount: HashSet::new(),
            till_pickup: HashSet::new(),
            drop_times: HashMap::with_capacity(items),
            colliders: Vec::with_capacity(COLLIDER_CAPACITY),
            riders: Vec::with_capacity(items),
            mount_mask: policy.mount_mask,
            drop_delay: policy.drop_delay_seconds,
            scan_interval: policy.scan_interval_seconds,
            elapsed: 0.0,
            last_scan: 0.0,
            startup_frame: 0,
            bound: false,
        }
    }
    pub fn bind(&mut self, physics: &PhysicsWorld) -> Result<(), ReferenceError> {
        if self.bound {
            return Ok(());
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
    pub fn stored(&self, item: ItemId) -> bool {
        self.locked.iter().any(|l| l.item == item)
    }
    /// The disabled own body of a locked item.
    pub fn locked_body(&self, item: ItemId) -> Option<RigidBodyHandle> {
        self.locked.iter().find(|l| l.item == item).map(|l| l.body)
    }
    /// True if the collider belongs to a locked item or to an item that rides on one.
    pub fn owns_collider(&self, user_data: u128, items: &ItemStore) -> bool {
        if self.locked.is_empty() {
            return false;
        }
        let Some(mut id) = collider_item(user_data) else {
            return false;
        };
        for _ in 0..items.len() {
            match items.get(id).map(|i| i.location) {
                Some(ItemLocation::Locked { .. }) => return true,
                _ => match items.parent(id) {
                    Some(parent) => id = parent,
                    None => return false,
                },
            }
        }
        false
    }
    pub fn mark_dropped(&mut self, item: ItemId) {
        self.drop_times.insert(item, self.elapsed);
    }
    pub fn picked_up(&mut self, item: ItemId) {
        self.till_pickup.remove(&item);
    }
    fn gates_closed(
        volume: &Volume,
        parts: &PartAttachments,
        mechanisms: &MechanismState,
        part_usables: &[(ItemId, ItemId)],
    ) -> bool {
        let open = |usable: ItemId| {
            mechanisms
                .value(usable, false)
                .is_some_and(|value| value > 0.1)
        };
        volume.slots.iter().all(|slot| {
            parts.slot_item(*slot).is_some_and(|item| {
                part_usables
                    .iter()
                    .find(|(part, _)| *part == item)
                    .is_none_or(|(_, usable)| !open(*usable))
            })
        }) && !volume.usables.iter().any(|usable| open(*usable))
    }
    /// Source `CanStore` at setting 2, with owner presence.
    fn permitted(volume: &Volume, items: &ItemStore, physics: &PhysicsWorld) -> bool {
        volume.active
            && volume.gates_closed
            && crate::reference::storage::stores(volume.storage_type)
            && items.get(volume.owner).is_some_and(|owner| {
                !owner.removed
                    && owner.inactive_pose.is_none()
                    && !(volume.dont_lock_if_owner_picked_up
                        && owner.location == ItemLocation::Held)
                    && owner.body.is_some_and(|body| {
                        physics.body_active(body) && physics.rapier().bodies.get(body).is_some()
                    })
            })
    }
    fn scan_volume(volume: &mut Volume, physics: &PhysicsWorld, mount_mask: u32) {
        volume.current.clear();
        let pose = *physics.rapier().colliders[volume.handle.unwrap()].position();
        let predicate = |_, collider: &rapier3d::prelude::Collider| {
            collider.is_enabled()
                && collider.collision_groups().memberships.bits() & mount_mask != 0
        };
        let filter = QueryFilter::default().predicate(&predicate);
        for (_, collider) in physics
            .queries(filter)
            .intersect_shape(pose, volume.shape.as_ref())
        {
            if let Some(item) = collider_item(collider.user_data)
                && !volume.current.contains(&item)
            {
                volume.current.push(item);
            }
        }
    }
    /// Source `FreezeUpd` lock checks, without the two-scan rule.
    fn can_lock(
        &self,
        item: ItemId,
        owner: ItemId,
        items: &ItemStore,
        physics: &PhysicsWorld,
    ) -> bool {
        let Some(state) = items.get(item) else {
            return false;
        };
        let Some(carrier) = items.get(owner).and_then(|o| o.body) else {
            return false;
        };
        state.location == ItemLocation::World
            && !state.removed
            && !state.pending_attachment
            && state.inactive_pose.is_none()
            && !self.dont_mount.contains(&item)
            && !self.till_pickup.contains(&item)
            && self
                .drop_times
                .get(&item)
                .is_none_or(|time| self.elapsed - time >= self.drop_delay)
            && item != owner
            && !items.descends(owner, item)
            && !items.mounted(item)
            && state.body.is_some_and(|body| {
                body != carrier
                    && physics.body_active(body)
                    && physics.vehicle_of(body).is_none()
                    && physics
                        .rapier()
                        .bodies
                        .get(body)
                        .is_some_and(|b| b.is_enabled() && b.is_dynamic())
            })
    }
    /// Adds mass to the carrier and to each locked owner body between them.
    fn add_mass(
        &mut self,
        owner: ItemId,
        carrier: RigidBodyHandle,
        mass: f32,
        items: &ItemStore,
        physics: &mut PhysicsWorld,
    ) {
        physics.add_cargo_mass(carrier, mass);
        let mut id = owner;
        while let Some(ItemLocation::Locked { owner: next, .. }) = items.get(id).map(|i| i.location)
        {
            if let Some(entry) = self.locked.iter_mut().find(|l| l.item == id) {
                entry.mass += mass;
                physics.add_cargo_mass(entry.body, mass);
            }
            id = next;
        }
    }
    /// Moves the item colliders, and the colliders of items locked to it, onto the owner carrier.
    /// Returns false if another loose item shares the item body.
    fn lock(
        &mut self,
        item: ItemId,
        volume: u16,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
    ) -> Result<bool, ReferenceError> {
        let owner = self.volumes[volume as usize].owner;
        let carrier = items
            .get(owner)
            .and_then(|i| i.body)
            .ok_or_else(|| error("Storage owner has no body"))?;
        let state = items
            .get(item)
            .ok_or_else(|| error("Missing storage item"))?;
        let body = state
            .body
            .ok_or_else(|| error("Storage item has no body"))?;
        let loose_local = to_rapier(state.local_pose);
        if items
            .iter()
            .any(|s| s.body == Some(body) && s.id != item && !items.descends(s.id, item))
        {
            return Ok(false);
        }
        let body_pose = *physics.rapier().bodies[body].position();
        let carrier_pose = *physics.rapier().bodies[carrier].position();
        physics.recompute_body_mass(body);
        let mass = physics.authored_body_mass(body);
        let flag = if physics.vehicle_of(carrier).is_some() {
            VEHICLE_COLLIDER_FLAG
        } else {
            0
        };
        self.colliders.clear();
        for handle in physics.rapier().bodies[body].colliders() {
            if collider_item(physics.rapier().colliders[*handle].user_data)
                .is_some_and(|id| id == item || items.descends(id, item))
            {
                self.colliders.push(*handle);
            }
        }
        for &handle in &self.colliders {
            let world = body_pose * local_pose(physics, handle);
            physics.set_collider_parent(handle, Some(carrier));
            physics.edit_collider(handle, |collider| {
                collider.set_position_wrt_parent(carrier_pose.inverse() * world);
                collider.user_data = (collider.user_data & !VEHICLE_COLLIDER_FLAG) | flag;
            });
        }
        physics.edit_body(body, |body| body.set_enabled(false));
        let rebase = carrier_pose.inverse() * body_pose;
        self.riders.clear();
        self.riders
            .extend(items.iter().filter(|s| s.body == Some(body)).map(|s| s.id));
        for &rider in &self.riders {
            let state = items.set_body(rider, Some(carrier)).unwrap();
            state.local_pose = from_rapier(&(rebase * to_rapier(state.local_pose)));
        }
        items.get_mut(item).unwrap().location = ItemLocation::Locked { owner, volume };
        self.add_mass(owner, carrier, mass, items, physics);
        self.locked.push(Locked {
            item,
            volume,
            body,
            loose_local,
            mass,
        });
        Ok(true)
    }
    /// The locked item pose relative to its disabled own body.
    pub fn loose_local(&self, item: ItemId) -> Option<Pose> {
        self.locked
            .iter()
            .find(|l| l.item == item)
            .map(|l| l.loose_local)
    }
    /// Source `Lock(_ignoreTime: true)` for a saved item: no drop delay and no two-scan rule.
    /// `owner_local` is the item pose relative to the owner. The owner must be locked first.
    pub fn lock_saved(
        &mut self,
        item: ItemId,
        volume: u16,
        owner_local: Pose,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
    ) -> Result<(), ReferenceError> {
        let owner = self
            .volumes
            .get(volume as usize)
            .ok_or_else(|| error("Unknown storage volume"))?
            .owner;
        let owner_state = items
            .get(owner)
            .ok_or_else(|| error("Missing storage owner"))?;
        let carrier = owner_state
            .body
            .ok_or_else(|| error("Storage owner has no body"))?;
        let owner_pose =
            *physics.rapier().bodies[carrier].position() * to_rapier(owner_state.local_pose);
        let state = items
            .get(item)
            .ok_or_else(|| error("Missing storage item"))?;
        let body = state
            .body
            .ok_or_else(|| error("Storage item has no body"))?;
        if state.location != ItemLocation::World || body == carrier {
            return Err(error("Saved storage item is not loose"));
        }
        let body_pose = owner_pose * owner_local * to_rapier(state.local_pose).inverse();
        physics.edit_body(body, |body| body.set_position(body_pose, false));
        if !self.lock(item, volume, items, physics)? {
            return Err(error("Saved storage item shares its body"));
        }
        Ok(())
    }
    /// Returns the item to its own body with the carrier velocity. Its locked children stay locked to it.
    pub fn unlock(
        &mut self,
        item: ItemId,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
    ) -> Result<(), ReferenceError> {
        let index = self
            .locked
            .iter()
            .position(|l| l.item == item)
            .ok_or_else(|| error("Item is not stored"))?;
        let state = items
            .get(item)
            .ok_or_else(|| error("Missing storage item"))?;
        let ItemLocation::Locked { owner, .. } = state.location else {
            return Err(error("Item is not stored"));
        };
        let carrier = state.body.ok_or_else(|| error("Missing storage carrier"))?;
        let item_pose = to_rapier(state.local_pose);
        let entry = self.locked.swap_remove(index);
        let carrier_pose = *physics.rapier().bodies[carrier].position();
        let body_pose = carrier_pose * item_pose * entry.loose_local.inverse();
        let velocity = physics.rapier().bodies[carrier].velocity_at_point(body_pose.translation);
        let angular = physics.rapier().bodies[carrier].angvel();
        self.add_mass(owner, carrier, -entry.mass, items, physics);
        self.colliders.clear();
        for handle in physics.rapier().bodies[carrier].colliders() {
            if collider_item(physics.rapier().colliders[*handle].user_data)
                .is_some_and(|id| id == item || items.descends(id, item))
            {
                self.colliders.push(*handle);
            }
        }
        for &handle in &self.colliders {
            let world = carrier_pose * local_pose(physics, handle);
            physics.set_collider_parent(handle, Some(entry.body));
            physics.edit_collider(handle, |collider| {
                collider.set_position_wrt_parent(body_pose.inverse() * world);
                collider.user_data &= !VEHICLE_COLLIDER_FLAG;
            });
        }
        physics.edit_body(entry.body, |body| {
            body.set_position(body_pose, true);
            body.set_enabled(true);
            body.set_linvel(velocity, true);
            body.set_angvel(angular, true);
        });
        let rebase = body_pose.inverse() * carrier_pose;
        self.riders.clear();
        self.riders.extend(
            items
                .iter()
                .filter(|s| s.body == Some(carrier) && (s.id == item || items.descends(s.id, item)))
                .map(|s| s.id),
        );
        for &rider in &self.riders {
            let state = items.set_body(rider, Some(entry.body)).unwrap();
            state.local_pose = from_rapier(&(rebase * to_rapier(state.local_pose)));
        }
        items.get_mut(item).unwrap().location = ItemLocation::World;
        self.drop_times.insert(item, self.elapsed);
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
        for volume in &mut self.volumes {
            volume.gates_closed = Self::gates_closed(volume, parts, mechanisms, &self.part_usables);
        }
        self.scan(items, physics)
    }
    /// One `Upd` pass over every volume. Gate state is already set.
    fn scan(
        &mut self,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
    ) -> Result<bool, ReferenceError> {
        let mut changed = false;
        for index in 0..self.volumes.len() {
            let volume = index as u16;
            let permitted = Self::permitted(&self.volumes[index], items, physics);
            if !permitted {
                while let Some(item) = self
                    .locked
                    .iter()
                    .find(|l| l.volume == volume)
                    .map(|l| l.item)
                {
                    self.unlock(item, items, physics)?;
                    changed = true;
                }
            }
            Self::scan_volume(&mut self.volumes[index], physics, self.mount_mask);
            if permitted {
                let owner = self.volumes[index].owner;
                for slot in 0..self.volumes[index].current.len() {
                    let item = self.volumes[index].current[slot];
                    if self.volumes[index].previous.contains(&item)
                        && self.can_lock(item, owner, items, physics)
                        && self.lock(item, volume, items, physics)?
                    {
                        changed = true;
                    }
                }
            }
            let volume = &mut self.volumes[index];
            std::mem::swap(&mut volume.previous, &mut volume.current);
        }
        if changed {
            physics.refresh_queries();
        }
        Ok(changed)
    }
}
/// Collider world poses update in the next query refresh. The parent pose is current.
fn local_pose(physics: &PhysicsWorld, handle: ColliderHandle) -> Pose {
    physics.rapier().colliders[handle]
        .position_wrt_parent()
        .copied()
        .unwrap_or(Pose::IDENTITY)
}
/// Collider user data low 32 bits hold the item id plus one. Zero is no item.
fn collider_item(user_data: u128) -> Option<ItemId> {
    let value = (user_data & u32::MAX as u128) as u32;
    value.checked_sub(1).map(ItemId)
}
fn error(message: &str) -> ReferenceError {
    ReferenceError(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reference::{SourceId, StoragePolicy};
    use crate::vehicle::VehiclePose;
    use rapier3d::prelude::{ColliderBuilder, RigidBodyBuilder};

    const CARRIER: ItemId = ItemId(1);
    const BOX: ItemId = ItemId(2);
    const CAN: ItemId = ItemId(3);

    struct Rig {
        physics: PhysicsWorld,
        items: ItemStore,
        locks: StorageLocks,
        carrier: RigidBodyHandle,
    }

    fn user_data(item: ItemId, collider: u32) -> u128 {
        ((collider as u128) << 64) | (item.0 as u128 + 1)
    }
    fn body(physics: &mut PhysicsWorld, x: f32, mass: f32, item: ItemId) -> RigidBodyHandle {
        let body = physics.insert_body(
            RigidBodyBuilder::dynamic()
                .translation(Vector::new(x, 0.0, 0.0))
                .additional_mass(mass),
        );
        physics.insert_collider_with_parent(
            ColliderBuilder::cuboid(0.1, 0.1, 0.1)
                .density(0.0)
                .user_data(user_data(item, 100 + item.0)),
            body,
        );
        body
    }
    fn volume(physics: &mut PhysicsWorld, parent: RigidBodyHandle, owner: ItemId) -> Volume {
        let id = 200 + owner.0;
        let handle = physics.insert_collider_with_parent(
            ColliderBuilder::cuboid(1.0, 1.0, 1.0)
                .sensor(true)
                .density(0.0)
                .user_data(user_data(owner, id)),
            parent,
        );
        Volume {
            owner,
            storage_type: 0,
            active: true,
            dont_lock_if_owner_picked_up: false,
            slots: Vec::new(),
            usables: Vec::new(),
            collider: ColliderId(id),
            handle: Some(handle),
            shape: SharedShape::cuboid(1.0, 1.0, 1.0),
            gates_closed: true,
            previous: Vec::with_capacity(SCAN_CAPACITY),
            current: Vec::with_capacity(SCAN_CAPACITY),
        }
    }
    fn item(items: &mut ItemStore, id: ItemId, body: RigidBodyHandle) {
        let pose = VehiclePose {
            position: [0.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
        };
        items.insert_spawned(id, "test", pose, body).unwrap();
    }
    /// Carrier with a 10 kg body and one volume. A 2 kg can inside it.
    /// With `nested`, a 3 kg box with its own volume holds the can. The box volume scans first.
    fn rig(nested: bool) -> Rig {
        let mut physics = PhysicsWorld::empty_for_tests();
        let mut items = ItemStore::empty_for_tests();
        let carrier = body(&mut physics, 0.0, 10.0, CARRIER);
        item(&mut items, CARRIER, carrier);
        let can = body(&mut physics, 0.3, 2.0, CAN);
        item(&mut items, CAN, can);
        let mut volumes = Vec::new();
        if nested {
            let crate_body = body(&mut physics, 0.5, 3.0, BOX);
            item(&mut items, BOX, crate_body);
            volumes.push(volume(&mut physics, crate_body, BOX));
        }
        volumes.push(volume(&mut physics, carrier, CARRIER));
        let policy = StoragePolicy {
            settings_source: SourceId {
                file: "test".into(),
                path_id: 1,
            },
            default_setting: 2,
            mount_mask: 8_388_609,
            drop_delay_seconds: 1.0,
            scan_interval_seconds: 1.5,
        };
        let mut locks = StorageLocks::with_volumes(volumes, 8, &policy);
        locks.bound = true;
        // The carrier body overlaps the box volume. Source heavy owners carry `dontMount`.
        locks.dont_mount.insert(CARRIER);
        Rig {
            physics,
            items,
            locks,
            carrier,
        }
    }
    impl Rig {
        fn scan(&mut self, dt: f32) {
            self.locks.elapsed += dt;
            self.physics.refresh_queries();
            self.locks.scan(&mut self.items, &mut self.physics).unwrap();
        }
        fn location(&self, id: ItemId) -> ItemLocation {
            self.items.get(id).unwrap().location
        }
        fn mass(&self, body: RigidBodyHandle) -> f32 {
            self.physics.rapier().bodies[body].mass()
        }
        fn world(&self, shape: ColliderHandle) -> Pose {
            let collider = &self.physics.rapier().colliders[shape];
            *self.physics.rapier().bodies[collider.parent().unwrap()].position()
                * local_pose(&self.physics, shape)
        }
        fn collider(&self, item: ItemId) -> ColliderHandle {
            self.physics
                .rapier()
                .colliders
                .iter()
                .find(|(_, c)| c.user_data == user_data(item, 100 + item.0))
                .unwrap()
                .0
        }
    }
    fn close(a: &Pose, b: &Pose) -> bool {
        (a.translation - b.translation).length() < 1e-4
            && a.rotation.angle_between(b.rotation) < 1e-4
    }

    #[test]
    fn lock_needs_two_scans_and_drop_delay_then_pickup_releases() {
        let mut rig = rig(false);
        let can_body = rig.items.get(CAN).unwrap().body.unwrap();
        let shape = rig.collider(CAN);
        let world = rig.world(shape);
        rig.locks.mark_dropped(CAN);
        rig.scan(0.0);
        assert_eq!(rig.location(CAN), ItemLocation::World, "one scan");
        rig.scan(0.5);
        assert_eq!(rig.location(CAN), ItemLocation::World, "drop delay");
        rig.scan(0.6);
        assert_eq!(
            rig.location(CAN),
            ItemLocation::Locked {
                owner: CARRIER,
                volume: 0
            }
        );
        assert_eq!(rig.items.get(CAN).unwrap().body, Some(rig.carrier));
        assert_eq!(
            rig.physics.rapier().colliders[shape].parent(),
            Some(rig.carrier)
        );
        assert!(close(&rig.world(shape), &world));
        assert!(!rig.physics.rapier().bodies[can_body].is_enabled());
        assert!((rig.mass(rig.carrier) - 12.0).abs() < 1e-4);

        rig.locks
            .unlock(CAN, &mut rig.items, &mut rig.physics)
            .unwrap();
        assert_eq!(rig.location(CAN), ItemLocation::World);
        assert_eq!(rig.items.get(CAN).unwrap().body, Some(can_body));
        assert_eq!(
            rig.physics.rapier().colliders[shape].parent(),
            Some(can_body)
        );
        assert!(close(&rig.world(shape), &world));
        assert!(rig.physics.rapier().bodies[can_body].is_enabled());
        assert!((rig.mass(rig.carrier) - 10.0).abs() < 1e-4);
        assert!((rig.mass(can_body) - 2.0).abs() < 1e-4);
        rig.scan(0.5);
        assert_eq!(
            rig.location(CAN),
            ItemLocation::World,
            "drop delay after pick-up"
        );
        rig.scan(0.6);
        assert!(matches!(rig.location(CAN), ItemLocation::Locked { .. }));
    }

    #[test]
    fn nested_box_moves_child_colliders_and_gate_close_releases_it() {
        let mut rig = rig(true);
        let box_body = rig.items.get(BOX).unwrap().body.unwrap();
        let can_body = rig.items.get(CAN).unwrap().body.unwrap();
        let can_shape = rig.collider(CAN);
        let can_world = rig.world(can_shape);
        rig.scan(0.0);
        rig.scan(1.5);
        assert_eq!(
            rig.location(CAN),
            ItemLocation::Locked {
                owner: BOX,
                volume: 0
            }
        );
        assert_eq!(
            rig.location(BOX),
            ItemLocation::Locked {
                owner: CARRIER,
                volume: 1
            }
        );
        assert_eq!(
            rig.physics.rapier().colliders[can_shape].parent(),
            Some(rig.carrier)
        );
        assert_eq!(rig.items.get(CAN).unwrap().body, Some(rig.carrier));
        assert!(close(&rig.world(can_shape), &can_world));
        assert!((rig.mass(rig.carrier) - 15.0).abs() < 1e-4);
        assert!((rig.mass(box_body) - 5.0).abs() < 1e-4);

        rig.locks.volumes[1].gates_closed = false;
        rig.scan(1.5);
        assert_eq!(rig.location(BOX), ItemLocation::World);
        assert_eq!(
            rig.location(CAN),
            ItemLocation::Locked {
                owner: BOX,
                volume: 0
            }
        );
        assert_eq!(
            rig.physics.rapier().colliders[can_shape].parent(),
            Some(box_body)
        );
        assert_eq!(rig.items.get(CAN).unwrap().body, Some(box_body));
        assert!(close(&rig.world(can_shape), &can_world));
        assert!((rig.mass(rig.carrier) - 10.0).abs() < 1e-4);

        rig.locks
            .unlock(CAN, &mut rig.items, &mut rig.physics)
            .unwrap();
        assert_eq!(
            rig.physics.rapier().colliders[can_shape].parent(),
            Some(can_body)
        );
        assert!((rig.mass(box_body) - 3.0).abs() < 1e-4);
        assert!((rig.mass(can_body) - 2.0).abs() < 1e-4);
    }

    #[test]
    fn saved_lock_round_trip_restores_owner_pose_and_mass() {
        use crate::save::{InstanceId, ItemLocationSave, LocalPose, SourceKey};
        let mut original = rig(false);
        let shape = original.collider(CAN);
        original.scan(0.0);
        original.scan(1.5);
        assert!(matches!(
            original.location(CAN),
            ItemLocation::Locked { .. }
        ));
        let world = original.world(shape);
        let pose = original
            .items
            .saved_local_pose(CAN, &original.locks)
            .unwrap();
        let source = SourceKey {
            file: "level".into(),
            path_id: "7".into(),
        };
        let saved = (
            ItemLocationSave::Locked {
                owner: InstanceId::Initial {
                    source: source.clone(),
                },
                volume: source,
            },
            LocalPose {
                position: pose.position,
                rotation: pose.rotation,
            },
        );
        let json = serde_json::to_string(&saved).unwrap();
        let parsed: (ItemLocationSave, LocalPose) = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, saved);

        // Load with a moved carrier and the can body elsewhere. No scan and no drop delay.
        let mut loaded = rig(false);
        let carrier_pose = Pose::from_parts(
            Vector::new(0.0, 1.0, 0.0),
            rapier3d::math::Rotation::from_rotation_y(0.5),
        );
        let carrier = loaded.carrier;
        loaded
            .physics
            .edit_body(carrier, |body| body.set_position(carrier_pose, false));
        let can_body = loaded.items.get(CAN).unwrap().body.unwrap();
        loaded.physics.edit_body(can_body, |body| {
            body.set_position(Pose::from_translation(Vector::new(5.0, 0.0, 0.0)), false)
        });
        loaded.locks.mark_dropped(CAN);
        loaded
            .locks
            .lock_saved(
                CAN,
                0,
                to_rapier(crate::save::local_pose(&parsed.1)),
                &mut loaded.items,
                &mut loaded.physics,
            )
            .unwrap();
        assert_eq!(
            loaded.location(CAN),
            ItemLocation::Locked {
                owner: CARRIER,
                volume: 0
            }
        );
        assert!(close(&loaded.world(shape), &(carrier_pose * world)));
        assert!((loaded.mass(carrier) - 12.0).abs() < 1e-4);
    }
}
