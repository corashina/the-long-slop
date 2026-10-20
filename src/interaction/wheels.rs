//! Source wheel parts. Attachment changes reuse collider and item identities.

use super::items::{ItemLocation, ItemStore, node_pose};
use crate::reference::{
    ColliderId, ItemId, ItemKind, PartSlotId, ReferenceError, ReferenceManifest, TankRole,
    Transform, WheelSlotId,
};
use crate::vehicle::{
    InstalledWheel, PhysicsWorld, SLOT_SENSOR_FLAG, VEHICLE_COLLIDER_FLAG, VehiclePose,
    from_rapier, to_rapier,
};
use rapier3d::math::{Pose, Rotation, Vector};
use rapier3d::prelude::{ColliderHandle, RigidBodyBuilder, RigidBodyHandle};
use std::collections::{HashMap, HashSet};

struct Part {
    item: ItemId,
    rim: bool,
    radius: f32,
    quality: f32,
    mass: f32,
    last_mass: f32,
    rotation: Rotation,
    tire_mount: Option<Pose>,
    tire: Option<usize>,
    collider_ids: Vec<ColliderId>,
    colliders: Vec<(ColliderHandle, Pose)>,
    free_body: Option<RigidBodyHandle>,
}
struct Slot {
    id: WheelSlotId,
    vehicle: usize,
    index: usize,
    rim: Option<usize>,
    original_rim: ItemId,
    hub_to_mount: Pose,
    compatible: HashSet<ItemId>,
    last_support: Option<(f32, f32)>,
    sensor: Option<ColliderHandle>,
}
/// Chassis mass terms that this module owns for one vehicle.
struct VehicleMass {
    tanks: Vec<ItemId>,
    base_mass: f32,
    last_mass: f32,
}
pub struct WheelAssemblies {
    rules: Vec<crate::reference::WheelPartReference>,
    parts: Vec<Part>,
    indices: HashMap<ItemId, usize>,
    slots: Vec<Slot>,
    tire_fits: HashSet<(ItemId, ItemId)>,
    vehicles: Vec<VehicleMass>,
    generated_tanks: Vec<ItemId>,
    bound: bool,
    general_mass: Vec<(ItemId, f32)>,
    covers: HashMap<ItemId, PartSlotId>,
}

impl WheelAssemblies {
    pub fn refresh_conditions(
        &mut self,
        conditions: &crate::conditions::ConditionStore,
        items: &ItemStore,
    ) {
        for part in &mut self.parts {
            if let Some(quality) = conditions
                .item_quality(part.item)
                .or_else(|| items.get(part.item).and_then(|i| i.generated_condition))
            {
                part.quality = quality;
            }
        }
    }
    pub(crate) fn restore_save(
        &mut self,
        ids: &crate::save::IdentityMap,
        saved: &crate::save::AttachmentsSnapshot,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
    ) -> Result<(), crate::save::SaveError> {
        self.bind(items, physics)?;
        for part in &mut self.parts {
            if let Some(body) = part.free_body {
                physics.edit_body(body, |body| body.set_enabled(false));
            }
            part.tire = None;
        }
        for record in &saved.tires {
            let rim = self.indices[&ids.item(&record.rim)?];
            self.parts[rim].tire = Some(self.indices[&ids.item(&record.tire)?]);
        }
        for record in &saved.wheels {
            let slot = self
                .slots
                .iter_mut()
                .find(|s| s.id == ids.wheel_slots[&record.slot])
                .unwrap();
            slot.rim = record
                .rim
                .as_ref()
                .map(|id| ids.item(id).map(|id| self.indices[&id]))
                .transpose()?;
        }
        for i in 0..self.parts.len() {
            let state = items.get(self.parts[i].item).unwrap();
            if matches!(state.location, ItemLocation::World | ItemLocation::Held) {
                let body = self.free_body(i, Pose::IDENTITY, physics);
                let local = to_rapier(items.get(self.parts[i].item).unwrap().local_pose);
                self.place_part(i, body, local, true, items, physics)?;
            } else if let ItemLocation::WheelSlot(slot_id) = state.location {
                self.place_part(
                    i,
                    physics.chassis_handle(
                        physics
                            .slot_vehicle(slot_id)
                            .expect("validated wheel owner"),
                    ),
                    to_rapier(state.local_pose),
                    false,
                    items,
                    physics,
                )?;
            }
        }
        for i in 0..self.parts.len() {
            if let Some(tire) = self.parts[i].tire {
                let body = items.get(self.parts[i].item).unwrap().body.unwrap();
                let local = to_rapier(items.get(self.parts[tire].item).unwrap().local_pose);
                self.place_part(
                    tire,
                    body,
                    local,
                    physics.vehicle_of(body).is_none(),
                    items,
                    physics,
                )?;
            }
        }
        self.sync(items, physics)?;
        for record in &saved.wheels {
            let slot = self
                .slots
                .iter()
                .find(|s| s.id == ids.wheel_slots[&record.slot])
                .unwrap();
            physics.restore_wheel_save(slot.vehicle, slot.index, record);
        }
        Ok(())
    }
    pub fn has_external_dependents(&self, owns: impl Fn(ItemId) -> bool) -> bool {
        self.parts.iter().any(|part| {
            owns(part.item) && part.tire.is_some_and(|tire| !owns(self.parts[tire].item))
        })
    }
    pub(crate) fn retained_binding_bytes(&self) -> usize {
        use super::memory::{map, set, strings, vector};
        vector(&self.rules)
            + self
                .rules
                .iter()
                .map(|r| {
                    r.source.file.capacity()
                        + strings(&r.main_types)
                        + strings(&r.part_types)
                        + vector(&r.colliders)
                        + strings(&r.tire_main_types)
                        + strings(&r.tire_types)
                })
                .sum::<usize>()
            + vector(&self.parts)
            + self
                .parts
                .iter()
                .map(|p| vector(&p.collider_ids) + vector(&p.colliders))
                .sum::<usize>()
            + map(&self.indices)
            + vector(&self.slots)
            + self.slots.iter().map(|s| set(&s.compatible)).sum::<usize>()
            + set(&self.tire_fits)
            + vector(&self.generated_tanks)
            + vector(&self.vehicles)
            + self
                .vehicles
                .iter()
                .map(|v| vector(&v.tanks))
                .sum::<usize>()
            + vector(&self.general_mass)
            + map(&self.covers)
    }

    pub fn new(reference: &ReferenceManifest) -> Result<Self, ReferenceError> {
        let mut parts = Vec::with_capacity(reference.wheel_parts.len());
        let mut indices = HashMap::with_capacity(reference.wheel_parts.len());
        let mut collider_ids = HashSet::new();
        for record in &reference.wheel_parts {
            if record.colliders.is_empty()
                || record.colliders.iter().any(|id| !collider_ids.insert(*id))
            {
                return Err(error("Missing or shared wheel part collider"));
            }
            let item = reference
                .items
                .iter()
                .find(|item| item.id == record.item)
                .ok_or_else(|| error("Missing wheel part item"))?;
            let rim = match item.kind {
                ItemKind::Rim => true,
                ItemKind::Tire => false,
                _ => return Err(error("Invalid wheel part kind")),
            };
            let tire_mount = if let Some(id) = record.tire_slot_anchor {
                let anchor = reference
                    .anchors
                    .iter()
                    .find(|anchor| anchor.id == id)
                    .ok_or_else(|| error("Missing tire mount anchor"))?;
                let body = reference
                    .bodies
                    .iter()
                    .find(|body| body.id == anchor.body)
                    .ok_or_else(|| error("Missing tire mount body"))?;
                Some(
                    to_rapier(node_pose(reference, item.node)?).inverse()
                        * pose(&body.pose)
                        * pose(&anchor.transform),
                )
            } else {
                None
            };
            indices.insert(item.id, parts.len());
            parts.push(Part {
                item: item.id,
                rim,
                radius: record.radius,
                quality: item.condition,
                mass: item.mass_kg,
                last_mass: f32::NAN,
                rotation: Rotation::from_array(record.local_rotation_after_mount),
                tire_mount,
                tire: None,
                collider_ids: record.colliders.clone(),
                colliders: Vec::with_capacity(record.colliders.len()),
                free_body: None,
            });
        }
        for item in &reference.items {
            if matches!(item.kind, ItemKind::Tire)
                && let Some(parent) = item.attached_to
                && let (Some(&rim), Some(&tire)) = (indices.get(&parent), indices.get(&item.id))
            {
                parts[rim].tire = Some(tire);
            }
        }
        let mut slots = Vec::with_capacity(reference.wheel_slots.len());
        let mut installed_mass = vec![0.0; reference.vehicles.len()];
        for (vehicle, index, id) in reference.vehicles.iter().enumerate().flat_map(|(v, r)| {
            r.wheel_slots
                .iter()
                .enumerate()
                .map(move |(i, id)| (v, i, id))
        }) {
            let chassis = reference
                .bodies
                .iter()
                .find(|body| body.id == reference.vehicles[vehicle].body)
                .ok_or_else(|| error("Missing chassis source pose"))?;
            let chassis_pose = pose(&chassis.pose);
            let record = reference
                .wheel_slots
                .iter()
                .find(|slot| slot.id == *id)
                .ok_or_else(|| error("Missing wheel slot"))?;
            if record.wheel != record.rim {
                return Err(error("Wheel identity must be its rim identity"));
            }
            let rim = *indices
                .get(&record.rim)
                .ok_or_else(|| error("Missing installed rim"))?;
            let tire = *indices
                .get(&record.tire)
                .ok_or_else(|| error("Missing installed tire"))?;
            parts[rim].tire = Some(tire);
            installed_mass[vehicle] += parts[rim].mass + parts[tire].mass;
            let anchor = reference
                .anchors
                .iter()
                .find(|anchor| anchor.id == record.anchor)
                .ok_or_else(|| error("Missing wheel anchor"))?;
            let mount = reference
                .anchors
                .iter()
                .find(|anchor| anchor.id == record.mount_anchor)
                .ok_or_else(|| error("Missing wheel mount"))?;
            let mount_body = reference
                .bodies
                .iter()
                .find(|body| body.id == mount.body)
                .ok_or_else(|| error("Missing wheel mount body"))?;
            let rest = record.suspension_distance * (1.0 - record.target_position);
            let hub = chassis_pose
                * Pose::from_translation(
                    Vector::from_array(anchor.transform.translation)
                        + Rotation::from_array(anchor.transform.rotation)
                            * Vector::from_array(record.center)
                        - Vector::Y * rest,
                );
            let compatible = reference
                .wheel_parts
                .iter()
                .filter(|part| {
                    indices
                        .get(&part.item)
                        .is_some_and(|index| parts[*index].rim)
                        && fits(
                            &record.rim_main_types,
                            &record.rim_types,
                            &part.main_types,
                            &part.part_types,
                        )
                })
                .map(|part| part.item)
                .collect();
            slots.push(Slot {
                id: *id,
                vehicle,
                index,
                rim: Some(rim),
                original_rim: record.rim,
                hub_to_mount: hub.inverse() * pose(&mount_body.pose) * pose(&mount.transform),
                compatible,
                last_support: None,
                sensor: None,
            });
        }
        let mut tire_fits = HashSet::new();
        for rim in &reference.wheel_parts {
            if !parts[indices[&rim.item]].rim {
                continue;
            }
            for tire in &reference.wheel_parts {
                if !parts[indices[&tire.item]].rim
                    && fits(
                        &rim.tire_main_types,
                        &rim.tire_types,
                        &tire.main_types,
                        &tire.part_types,
                    )
                {
                    tire_fits.insert((rim.item, tire.item));
                }
            }
        }
        let general_mass: Vec<_> = reference
            .items
            .iter()
            .filter(|item| {
                if reference
                    .plates
                    .plates
                    .iter()
                    .any(|plate| plate.item == item.id)
                {
                    return true;
                }
                let mut node = Some(item.node);
                while let Some(id) = node {
                    if reference.parts.iter().any(|p| p.node == id) {
                        return true;
                    }
                    node = reference
                        .nodes
                        .iter()
                        .find(|n| n.id == id)
                        .and_then(|n| n.parent);
                }
                false
            })
            .map(|i| (i.id, i.mass_kg))
            .collect();
        // Flashlights start outside the car. Add their mass only while car-owned.
        let mut general_mass = general_mass;
        general_mass.extend(reference.flashlights.instances.iter().map(|instance| {
            let item = reference
                .items
                .iter()
                .find(|item| item.id == instance.item)
                .unwrap();
            (item.id, item.mass_kg)
        }));
        let item_vehicle = |id: ItemId| {
            reference
                .items
                .iter()
                .find(|item| item.id == id)
                .and_then(|item| crate::starting_area::body_vehicle(reference, item.body))
        };
        let vehicles = reference
            .vehicles
            .iter()
            .enumerate()
            .map(|(vehicle, record)| {
                let tanks: Vec<_> = reference
                    .tanks
                    .iter()
                    .filter(|tank| {
                        tank.role == TankRole::Vehicle && item_vehicle(tank.item) == Some(vehicle)
                    })
                    .collect();
                let initial_fluids: f32 = tanks
                    .iter()
                    .map(|tank| tank.fluids.iter().map(|entry| entry.amount).sum::<f32>())
                    .sum();
                let general_dry: f32 = general_mass
                    .iter()
                    .filter(|(id, _)| item_vehicle(*id) == Some(vehicle))
                    .map(|(_, mass)| mass)
                    .sum();
                VehicleMass {
                    tanks: tanks.iter().map(|tank| tank.item).collect(),
                    base_mass: record.mass_kg
                        - installed_mass[vehicle]
                        - initial_fluids
                        - general_dry,
                    last_mass: f32::NAN,
                }
            })
            .collect();
        Ok(Self {
            rules: reference.wheel_parts.clone(),
            parts,
            indices,
            slots,
            tire_fits,
            vehicles,
            generated_tanks: Vec::new(),
            general_mass,
            covers: reference
                .wheel_parts
                .iter()
                .filter_map(|p| p.cover_slot.map(|slot| (p.item, slot)))
                .collect(),
            bound: false,
        })
    }

    pub fn set_generated_cover(&mut self, item: ItemId, slot: PartSlotId) {
        self.covers.insert(item, slot);
    }
    pub fn rules(&self) -> &[crate::reference::WheelPartReference] {
        &self.rules
    }
    pub fn register_generated(
        &mut self,
        rule: crate::reference::WheelPartReference,
        rim: bool,
        mass: f32,
        colliders: &[(ColliderId, ColliderHandle)],
        items: &ItemStore,
        physics: &mut PhysicsWorld,
        reference: &ReferenceManifest,
    ) -> Result<(), ReferenceError> {
        self.bind(items, physics)?;
        if self.indices.contains_key(&rule.item) {
            return Err(error("Generated wheel already bound"));
        }
        let item = items
            .get(rule.item)
            .ok_or_else(|| error("Generated wheel item missing"))?;
        let inverse = to_rapier(item.pose).inverse();
        let shapes = colliders
            .iter()
            .map(|&(id, handle)| {
                let _ = id;
                (
                    handle,
                    inverse * *physics.rapier().colliders[handle].position(),
                )
            })
            .collect();
        for slot in &mut self.slots {
            let r = reference
                .wheel_slots
                .iter()
                .find(|r| r.id == slot.id)
                .unwrap();
            if rim
                && fits(
                    &r.rim_main_types,
                    &r.rim_types,
                    &rule.main_types,
                    &rule.part_types,
                )
            {
                slot.compatible.insert(rule.item);
            }
        }
        for old in &self.rules {
            let old_rim = self.parts[self.indices[&old.item]].rim;
            if rim
                && !old_rim
                && fits(
                    &rule.tire_main_types,
                    &rule.tire_types,
                    &old.main_types,
                    &old.part_types,
                )
            {
                self.tire_fits.insert((rule.item, old.item));
            }
            if !rim
                && old_rim
                && fits(
                    &old.tire_main_types,
                    &old.tire_types,
                    &rule.main_types,
                    &rule.part_types,
                )
            {
                self.tire_fits.insert((old.item, rule.item));
            }
        }
        self.indices.insert(rule.item, self.parts.len());
        self.parts.push(Part {
            quality: item.generated_condition.expect("generated wheel condition"),
            item: rule.item,
            rim,
            radius: rule.radius,
            mass,
            last_mass: f32::NAN,
            rotation: Rotation::from_array(rule.local_rotation_after_mount),
            tire_mount: rule.tire_mount.as_ref().map(pose),
            tire: None,
            collider_ids: rule.colliders.clone(),
            colliders: shapes,
            free_body: (item.location == ItemLocation::World)
                .then_some(item.body)
                .flatten(),
        });
        self.rules.push(rule);
        Ok(())
    }
    pub fn register_generated_general(&mut self, item: ItemId, mass: f32, fluid: bool) {
        self.general_mass.push((item, mass));
        if fluid {
            self.generated_tanks.push(item);
        }
    }
    pub fn remove_generated(&mut self, item: ItemId) {
        self.covers.remove(&item);
        self.general_mass.retain(|(id, _)| *id != item);
        self.generated_tanks.retain(|id| *id != item);
        let Some(index) = self.indices.remove(&item) else {
            return;
        };
        self.parts.remove(index);
        self.rules.retain(|r| r.item != item);
        self.tire_fits
            .retain(|(rim, tire)| *rim != item && *tire != item);
        for slot in &mut self.slots {
            slot.compatible.remove(&item);
            slot.rim = slot
                .rim
                .and_then(|rim| (rim != index).then_some(rim - usize::from(rim > index)));
        }
        for (i, part) in self.parts.iter_mut().enumerate() {
            self.indices.insert(part.item, i);
            if let Some(tire) = &mut part.tire {
                if *tire > index {
                    *tire -= 1;
                }
            }
        }
    }
    fn covered(&self, item: ItemId, items: &ItemStore) -> bool {
        self.covers.get(&item).is_some_and(|slot| {
            items
                .iter()
                .any(|state| state.location == ItemLocation::PartSlot(*slot))
        })
    }

    pub fn capture_save(
        &self,
        ids: &crate::save::IdentityMap,
        physics: &PhysicsWorld,
    ) -> Result<
        (
            Vec<crate::save::WheelAttachment>,
            Vec<crate::save::TireAttachment>,
        ),
        crate::save::SaveError,
    > {
        let mut slots = self
            .slots
            .iter()
            .map(|slot| {
                let (rotation, rpm, previous_rotation) =
                    physics.wheel_save_state(slot.vehicle, slot.index);
                Ok(crate::save::WheelAttachment {
                    slot: crate::save::identity::source_key(&ids.wheel_slots, slot.id)?,
                    rim: slot
                        .rim
                        .filter(|rim| ids.items.values().any(|id| *id == self.parts[*rim].item))
                        .map(|rim| ids.item_instance(self.parts[rim].item))
                        .transpose()?,
                    rotation,
                    rpm,
                    previous_rotation,
                })
            })
            .collect::<Result<Vec<_>, crate::save::SaveError>>()?;
        let mut tires = self
            .parts
            .iter()
            .filter(|part| part.rim)
            .filter_map(|part| part.tire.map(|tire| (part, tire)))
            .filter(|(part, tire)| {
                ids.items.values().any(|id| *id == part.item)
                    && ids.items.values().any(|id| *id == self.parts[*tire].item)
            })
            .map(|(part, tire)| {
                Ok(crate::save::TireAttachment {
                    rim: ids.item_instance(part.item)?,
                    tire: ids.item_instance(self.parts[tire].item)?,
                })
            })
            .collect::<Result<Vec<_>, crate::save::SaveError>>()?;
        slots.sort_by(|a, b| a.slot.cmp(&b.slot));
        tires.sort_by(|a, b| a.rim.cmp(&b.rim));
        Ok((slots, tires))
    }
    pub fn slot_for_item(&self, item: ItemId) -> Option<WheelSlotId> {
        self.slots
            .iter()
            .find(|slot| {
                slot.rim.is_some_and(|rim| {
                    self.parts[rim].item == item
                        || self.parts[rim]
                            .tire
                            .is_some_and(|tire| self.parts[tire].item == item)
                })
            })
            .map(|slot| slot.id)
    }
    pub fn installed(&self, slot: WheelSlotId) -> bool {
        self.slots
            .iter()
            .any(|candidate| candidate.id == slot && candidate.rim.is_some())
    }

    pub fn sensor(&self, slot: WheelSlotId) -> Option<ColliderHandle> {
        self.slots
            .iter()
            .find(|candidate| candidate.id == slot)
            .and_then(|slot| slot.sensor)
    }

    fn bind(
        &mut self,
        items: &ItemStore,
        physics: &mut PhysicsWorld,
    ) -> Result<(), ReferenceError> {
        if self.bound {
            return Ok(());
        }
        let handles: HashMap<_, _> = physics
            .rapier()
            .colliders
            .iter()
            .filter_map(|(handle, collider)| {
                let id = (collider.user_data >> 64) as u32;
                (id != 0).then_some((ColliderId(id), handle))
            })
            .collect();
        for part in &mut self.parts {
            let item = items
                .get(part.item)
                .ok_or_else(|| error("Missing wheel item state"))?;
            let inverse = to_rapier(item.pose).inverse();
            if item.location == ItemLocation::World
                && let Some(body) = item.body
                && physics.rapier().bodies[body].is_dynamic()
            {
                part.free_body = Some(body);
            }
            for id in &part.collider_ids {
                let handle = *handles
                    .get(id)
                    .ok_or_else(|| error("Missing recovered wheel collider binding"))?;
                part.colliders.push((
                    handle,
                    inverse * *physics.rapier().colliders[handle].position(),
                ));
                physics.edit_collider(handle, |collider| {
                    collider.user_data =
                        (collider.user_data & !(u32::MAX as u128)) | (part.item.0 as u128 + 1);
                });
            }
        }
        for (handle, collider) in physics.rapier().colliders.iter() {
            if collider.user_data & SLOT_SENSOR_FLAG != 0 {
                let id = WheelSlotId((collider.user_data >> 96) as u8);
                if let Some(slot) = self.slots.iter_mut().find(|slot| slot.id == id) {
                    slot.sensor = Some(handle);
                }
            }
        }
        self.bound = true;
        Ok(())
    }

    pub fn remove(
        &mut self,
        id: WheelSlotId,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
    ) -> Result<ItemId, ReferenceError> {
        self.bind(items, physics)?;
        let slot = self
            .slots
            .iter()
            .position(|slot| slot.id == id)
            .ok_or_else(|| error("Unknown wheel slot"))?;
        let rim = self.slots[slot]
            .rim
            .ok_or_else(|| error("Wheel slot is empty"))?;
        if self.covered(self.parts[rim].item, items) {
            return Err(error("Remove the wheel cover first"));
        }
        let world = item_pose(items, self.parts[rim].item, physics)?;
        let body = self.free_body(rim, world, physics);
        let chassis = physics.chassis_handle(self.slots[slot].vehicle);
        let velocity = physics.rapier().bodies[chassis].velocity_at_point(world.translation);
        let angular = physics.rapier().bodies[chassis].angvel();
        physics.edit_body(body, |body| body.set_linvel(velocity, true));
        physics.edit_body(body, |body| body.set_angvel(angular, true));
        self.place_part(rim, body, Pose::IDENTITY, true, items, physics)?;
        items.get_mut(self.parts[rim].item).unwrap().location = ItemLocation::World;
        if let Some(tire) = self.parts[rim].tire {
            let local = world.inverse() * item_pose(items, self.parts[tire].item, physics)?;
            self.place_part(tire, body, local, true, items, physics)?;
            items.get_mut(self.parts[tire].item).unwrap().location =
                ItemLocation::Attached(self.parts[rim].item);
        }
        self.slots[slot].rim = None;
        self.update_body_mass(rim, body, items, physics);
        self.sync(items, physics)?;
        physics.refresh_queries();
        Ok(self.parts[rim].item)
    }

    pub fn can_install(
        &self,
        item: ItemId,
        id: WheelSlotId,
        items: &ItemStore,
    ) -> Result<(), ReferenceError> {
        let slot = self
            .slots
            .iter()
            .position(|slot| slot.id == id)
            .ok_or_else(|| error("Unknown wheel slot"))?;
        if self.slots[slot].rim.is_some() {
            return Err(error("Wheel slot is occupied"));
        }
        let rim = *self
            .indices
            .get(&item)
            .ok_or_else(|| error("Unknown rim"))?;
        if self.covered(item, items) {
            return Err(error("Remove the wheel cover first"));
        }
        if !self.parts[rim].rim || !self.slots[slot].compatible.contains(&item) {
            return Err(error("Rim does not fit wheel slot"));
        }
        if items
            .get(item)
            .is_none_or(|item| !matches!(item.location, ItemLocation::World | ItemLocation::Held))
        {
            return Err(error("Rim must be loose or held before installation"));
        }
        Ok(())
    }

    pub fn install(
        &mut self,
        item: ItemId,
        id: WheelSlotId,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
    ) -> Result<(), ReferenceError> {
        self.bind(items, physics)?;
        self.can_install(item, id, items)?;
        if items
            .get(item)
            .is_none_or(|item| item.location != ItemLocation::World)
        {
            return Err(error("Release the carry constraint before installation"));
        }
        let slot = self.slots.iter().position(|slot| slot.id == id).unwrap();
        let rim = self.indices[&item];
        let chassis = physics.chassis_handle(self.slots[slot].vehicle);
        let snapshot = physics.snapshot(self.slots[slot].vehicle);
        let rim_pose = to_rapier(snapshot.wheels[self.slots[slot].index])
            * self.slots[slot].hub_to_mount
            * Pose::from_rotation(self.parts[rim].rotation);
        let local = to_rapier(snapshot.pose).inverse() * rim_pose;
        self.place_part(rim, chassis, local, false, items, physics)?;
        if let Some(tire) = self.parts[rim].tire {
            let tire_local = local
                * self.parts[rim].tire_mount.unwrap()
                * Pose::from_rotation(self.parts[tire].rotation);
            self.place_part(tire, chassis, tire_local, false, items, physics)?;
        }
        if let Some(body) = self.parts[rim].free_body {
            physics.edit_body(body, |body| body.set_enabled(false));
        }
        self.slots[slot].rim = Some(rim);
        items.get_mut(item).unwrap().location = ItemLocation::WheelSlot(id);
        self.sync(items, physics)?;
        physics.refresh_queries();
        Ok(())
    }

    pub fn can_split(&self, item: ItemId, items: &ItemStore) -> Result<ItemId, ReferenceError> {
        let rim = *self
            .indices
            .get(&item)
            .ok_or_else(|| error("Unknown rim"))?;
        if !self.parts[rim].rim
            || items.get(item).is_none_or(|item| {
                !matches!(
                    item.location,
                    ItemLocation::World | ItemLocation::WheelSlot(_)
                )
            })
        {
            return Err(error("Split requires a loose or mounted rim"));
        }
        let tire = self.parts[rim]
            .tire
            .ok_or_else(|| error("Rim has no tire"))?;
        Ok(self.parts[tire].item)
    }

    pub fn split(
        &mut self,
        item: ItemId,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
    ) -> Result<(ItemId, ItemId), ReferenceError> {
        self.can_split(item, items)?;
        self.bind(items, physics)?;
        let rim = self.indices[&item];
        let tire = self.parts[rim].tire.unwrap();
        let world = item_pose(items, self.parts[tire].item, physics)?;
        let body = self.free_body(tire, world, physics);
        let rim_body = items
            .get(item)
            .unwrap()
            .body
            .ok_or_else(|| error("Missing loose rim body"))?;
        let velocity = physics.rapier().bodies[rim_body].velocity_at_point(world.translation);
        let angular = physics.rapier().bodies[rim_body].angvel();
        physics.edit_body(body, |body| body.set_linvel(velocity, true));
        physics.edit_body(body, |body| body.set_angvel(angular, true));
        self.place_part(tire, body, Pose::IDENTITY, true, items, physics)?;
        items.get_mut(self.parts[tire].item).unwrap().location = ItemLocation::World;
        self.parts[rim].tire = None;
        self.update_body_mass(rim, rim_body, items, physics);
        self.update_body_mass(tire, body, items, physics);
        self.sync(items, physics)?;
        physics.refresh_queries();
        Ok((item, self.parts[tire].item))
    }

    pub fn can_assemble(
        &self,
        rim_id: ItemId,
        tire_id: ItemId,
        items: &ItemStore,
    ) -> Result<(), ReferenceError> {
        if !self.tire_fits.contains(&(rim_id, tire_id)) {
            return Err(error("Tire does not fit rim"));
        }
        let rim = self.indices[&rim_id];
        if self.parts[rim].tire.is_some() {
            return Err(error("Rim already has a tire"));
        }
        if items
            .get(rim_id)
            .is_none_or(|i| !matches!(i.location, ItemLocation::World | ItemLocation::WheelSlot(_)))
            || items
                .get(tire_id)
                .is_none_or(|i| !matches!(i.location, ItemLocation::World | ItemLocation::Held))
        {
            return Err(error("Assembly requires loose parts"));
        }
        if self.parts[rim].tire_mount.is_none() || items.get(rim_id).unwrap().body.is_none() {
            return Err(error("Missing rim mount or body"));
        }
        Ok(())
    }

    pub fn assemble(
        &mut self,
        rim_id: ItemId,
        tire_id: ItemId,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
    ) -> Result<ItemId, ReferenceError> {
        self.can_assemble(rim_id, tire_id, items)?;
        if items.get(tire_id).unwrap().location != ItemLocation::World {
            return Err(error("Release the carry constraint before assembly"));
        }
        self.bind(items, physics)?;
        let rim = self.indices[&rim_id];
        let tire = self.indices[&tire_id];
        let body = items
            .get(rim_id)
            .unwrap()
            .body
            .ok_or_else(|| error("Missing rim body"))?;
        let local = to_rapier(items.get(rim_id).unwrap().local_pose)
            * self.parts[rim]
                .tire_mount
                .ok_or_else(|| error("Missing tire mount"))?
            * Pose::from_rotation(self.parts[tire].rotation);
        self.place_part(
            tire,
            body,
            local,
            physics.vehicle_of(body).is_none(),
            items,
            physics,
        )?;
        if let Some(old) = self.parts[tire].free_body
            && old != body
        {
            physics.edit_body(old, |body| body.set_enabled(false));
        }
        items.get_mut(tire_id).unwrap().location = ItemLocation::Attached(rim_id);
        self.parts[rim].tire = Some(tire);
        self.update_body_mass(rim, body, items, physics);
        self.sync(items, physics)?;
        physics.refresh_queries();
        Ok(rim_id)
    }

    fn free_body(
        &mut self,
        part: usize,
        world: Pose,
        physics: &mut PhysicsWorld,
    ) -> RigidBodyHandle {
        let handle = *self.parts[part].free_body.get_or_insert_with(|| {
            physics.insert_body(RigidBodyBuilder::dynamic().pose(world).ccd_enabled(true))
        });
        physics.edit_body(handle, |body| {
            body.set_enabled(true);
            body.set_position(world, true);
            body.reset_forces(true);
            body.reset_torques(true);
        });
        handle
    }
    fn place_part(
        &self,
        part: usize,
        body: RigidBodyHandle,
        local: Pose,
        enabled: bool,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
    ) -> Result<(), ReferenceError> {
        let state = items
            .get_mut(self.parts[part].item)
            .ok_or_else(|| error("Missing wheel item"))?;
        state.body = Some(body);
        state.local_pose = from_rapier(&local);
        state.pose = from_rapier(&(*physics.rapier().bodies[body].position() * local));
        let vehicle = physics.vehicle_of(body).is_some();
        for &(handle, pose) in &self.parts[part].colliders {
            physics.set_collider_parent(handle, Some(body));
            physics.edit_collider(handle, |collider| {
                collider.set_position_wrt_parent(local * pose);
                crate::vehicle::set_collider_enabled(collider, enabled);
                collider.set_density(0.0);
                if vehicle {
                    collider.user_data |= VEHICLE_COLLIDER_FLAG;
                } else {
                    collider.user_data &= !VEHICLE_COLLIDER_FLAG;
                }
            });
        }
        Ok(())
    }
    // This module owns complete loose-wheel mass, including general part descendants.
    // The source member list is cached at construction; transactions move their body ownership.
    fn update_body_mass(
        &mut self,
        rim: usize,
        body: RigidBodyHandle,
        items: &ItemStore,
        physics: &mut PhysicsWorld,
    ) {
        if physics.vehicle_of(body).is_some() {
            return;
        }
        let mass = self.parts[rim].mass
            + self.parts[rim]
                .tire
                .map_or(0.0, |tire| self.parts[tire].mass)
            + self
                .general_mass
                .iter()
                .filter_map(|(id, dry)| {
                    let item = items.get(*id)?;
                    (item.body == Some(body))
                        .then(|| dry + item.fluid.as_ref().map_or(0., |f| f.mixture().total()))
                })
                .sum::<f32>();
        if self.parts[rim].last_mass != mass {
            physics.edit_body(body, |body| body.set_additional_mass(mass, true));
            physics.recompute_body_mass(body);
            self.parts[rim].last_mass = mass;
        }
    }

    pub fn sync_activity(&mut self, physics: &mut PhysicsWorld) {
        for slot in &self.slots {
            let active = physics.body_active(physics.chassis_handle(slot.vehicle));
            if let Some(handle) = slot.sensor {
                physics.edit_collider(handle, |collider| collider.set_enabled(active));
            }
        }
    }

    pub fn sync(
        &mut self,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
    ) -> Result<(), ReferenceError> {
        self.bind(items, physics)?;
        self.sync_activity(physics);
        for vehicle in 0..self.vehicles.len() {
            self.sync_vehicle(vehicle, items, physics)?;
        }
        for index in 0..self.parts.len() {
            if let Some(item) = items.get(self.parts[index].item)
                && matches!(item.location, ItemLocation::World | ItemLocation::Held)
                && let Some(body) = item.body
                && physics.body_active(body)
            {
                self.update_body_mass(index, body, items, physics);
            }
        }
        for vehicle in 0..self.vehicles.len() {
            let chassis = physics.chassis_handle(vehicle);
            let state = &self.vehicles[vehicle];
            let mut mass = state.base_mass;
            for slot in self.slots.iter().filter(|slot| slot.vehicle == vehicle) {
                if let Some(rim) = slot.rim {
                    mass += self.parts[rim].mass
                        + self.parts[rim]
                            .tire
                            .map_or(0.0, |tire| self.parts[tire].mass);
                }
            }
            for &(id, dry) in &self.general_mass {
                if items.get(id).is_some_and(|i| i.body == Some(chassis)) {
                    mass += dry;
                }
            }
            for id in state.tanks.iter().chain(&self.generated_tanks) {
                if items.get(*id).is_some_and(|i| i.body != Some(chassis)) {
                    continue;
                }
                mass += items
                    .get(*id)
                    .and_then(|item| item.fluid.as_ref())
                    .ok_or_else(|| error("Missing vehicle tank"))?
                    .mixture()
                    .total();
            }
            if !state.last_mass.is_finite() || (mass - state.last_mass).abs() > 0.0001 {
                physics.set_vehicle_mass(vehicle, mass);
                self.vehicles[vehicle].last_mass = mass;
            }
        }
        Ok(())
    }

    /// Moves mounted wheels with the suspension and updates wheel supports.
    fn sync_vehicle(
        &mut self,
        vehicle: usize,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
    ) -> Result<(), ReferenceError> {
        if !physics.body_active(physics.chassis_handle(vehicle)) {
            return Ok(());
        }
        let snapshot = physics.snapshot(vehicle);
        let chassis = physics.chassis_handle(vehicle);

        let inverse = to_rapier(snapshot.pose).inverse();
        for slot in self.slots.iter_mut().filter(|slot| slot.vehicle == vehicle) {
            let support = if let Some(rim) = slot.rim {
                let part = &self.parts[rim];
                let rim_pose = to_rapier(snapshot.wheels[slot.index])
                    * slot.hub_to_mount
                    * Pose::from_rotation(part.rotation);
                let state = items
                    .get_mut(part.item)
                    .ok_or_else(|| error("Missing mounted rim"))?;
                state.body = Some(chassis);
                state.local_pose = from_rapier(&(inverse * rim_pose));
                state.pose = from_rapier(&rim_pose);
                let (radius, quality) = if let Some(tire) = part.tire {
                    let tire_part = &self.parts[tire];
                    let tire_pose = rim_pose
                        * part
                            .tire_mount
                            .ok_or_else(|| error("Missing rim tire mount"))?
                        * Pose::from_rotation(tire_part.rotation);
                    let tire_state = items
                        .get_mut(tire_part.item)
                        .ok_or_else(|| error("Missing mounted tire"))?;
                    tire_state.body = Some(chassis);
                    tire_state.local_pose = from_rapier(&(inverse * tire_pose));
                    tire_state.pose = from_rapier(&tire_pose);
                    tire_state.location = ItemLocation::Attached(part.item);
                    (tire_part.radius, tire_part.quality)
                } else {
                    (part.radius, 1.0)
                };
                Some((radius, quality))
            } else {
                None
            };
            if slot.last_support != support {
                physics.set_wheel(
                    slot.id,
                    support.map(|(radius, condition)| InstalledWheel { radius, condition }),
                )?;
                slot.last_support = support;
            }
            if let Some(handle) = slot.sensor {
                let world = to_rapier(snapshot.wheels[slot.index]) * slot.hub_to_mount;
                physics.edit_collider(handle, |collider| {
                    if !collider.is_enabled() {
                        collider.set_enabled(true);
                    }
                    let item = slot
                        .rim
                        .map_or(slot.original_rim, |rim| self.parts[rim].item);
                    let mut data = (collider.user_data & !(SLOT_SENSOR_FLAG | u32::MAX as u128))
                        | (item.0 as u128 + 1);
                    if slot.rim.is_none() {
                        data |= SLOT_SENSOR_FLAG;
                    }
                    if collider.user_data != data {
                        collider.user_data = data;
                    }
                    if *collider.position() != world {
                        collider.set_position(world);
                    }
                });
            }
        }

        Ok(())
    }
}

pub(super) fn fits(
    slot_main: &[String],
    slot_types: &[String],
    part_main: &[String],
    part_types: &[String],
) -> bool {
    (slot_main.is_empty()
        || part_main.is_empty()
        || slot_main.iter().any(|tag| part_main.contains(tag)))
        && slot_types.iter().any(|tag| part_types.contains(tag))
}
fn pose(transform: &Transform) -> Pose {
    to_rapier(VehiclePose {
        position: transform.translation,
        rotation: transform.rotation,
    })
}
fn item_pose(
    items: &ItemStore,
    id: ItemId,
    physics: &PhysicsWorld,
) -> Result<Pose, ReferenceError> {
    let item = items.get(id).ok_or_else(|| error("Missing wheel item"))?;
    Ok(item
        .body
        .and_then(|handle| physics.rapier().bodies.get(handle))
        .map_or(to_rapier(item.pose), |body| {
            *body.position() * to_rapier(item.local_pose)
        }))
}
fn error(message: &str) -> ReferenceError {
    ReferenceError(message.into())
}
