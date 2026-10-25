use crate::reference::WorldPrototypeReference;
use crate::world::{AbsolutePose, GeneratedId, SupplyDefinition, WorldOrigin};
use bevy_platform::collections::HashMap as BevyHashMap;
use std::collections::{HashMap, HashSet};

use rapier3d::math::{Rotation, Vector};
use rapier3d::prelude::RigidBodyHandle;

use crate::fluids::{FluidContainer, FluidEntry, VehicleFluids, validate_fluid_entries};
use crate::reference::{
    BodyId, FoodReference, ItemId, NodeId, PartSlotId, ReferenceError, ReferenceManifest,
    WheelSlotId,
};
use crate::vehicle::{PhysicsWorld, VehiclePose, from_rapier, to_rapier};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemLocation {
    World,
    PartSlot(PartSlotId),
    Held,
    Stored(u8),
    Equipped,
    Attached(ItemId),
    WheelSlot(WheelSlotId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FoodState {
    pub prototype_index: usize,
    pub bite: u8,
}

/// Source `pickupable` hit sound: `collideSound` and `collisionSoundM`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HitSound {
    pub clip: u32,
    pub gain: f32,
}

#[derive(Clone, Debug)]
pub struct ItemState {
    pub id: ItemId,
    pub body: Option<RigidBodyHandle>,
    pub inactive_pose: Option<crate::save::AbsolutePose>,
    pub location: ItemLocation,
    pub pose: VehiclePose,
    pub local_pose: VehiclePose,
    pub generated_condition: Option<f32>,
    pub temperature: Option<f32>,
    pub fluid: Option<FluidContainer>,
    pub food: Option<FoodState>,
    pub hit_sound: Option<HitSound>,
    pub removed: bool,
    pub pending_attachment: bool,
}

pub struct ItemStore {
    dirty: HashSet<ItemId>,
    states: Vec<ItemState>,
    indices: BevyHashMap<ItemId, usize>,
    body_ids: Vec<BodyId>,
    /// Fuel, sump, and coolant state indices per vehicle.
    vehicle_tanks: Vec<[usize; 3]>,
    names: Vec<String>,
    vehicle_items: Vec<ItemId>,
    slot_owners: HashMap<PartSlotId, Option<ItemId>>,
    generated: BevyHashMap<GeneratedId, ItemId>,
    membership_revision: u64,
    initialized_supplies: HashSet<ItemId>,
    next_generated: u32,
    origin: WorldOrigin,
}

/// Physical membership changes less often than item poses.
#[derive(Default)]
pub(crate) struct ItemBodyIndex {
    revision: Option<u64>,
    by_body: BevyHashMap<RigidBodyHandle, Vec<ItemId>>,
    #[cfg(test)]
    visits: usize,
}

impl ItemBodyIndex {
    pub(crate) fn refresh(&mut self, items: &ItemStore) -> bool {
        if self.revision == Some(items.membership_revision()) {
            return false;
        }
        for ids in self.by_body.values_mut() {
            ids.clear();
        }
        for item in items.iter() {
            #[cfg(test)]
            {
                self.visits += 1;
            }
            if let Some(body) = item.body {
                self.by_body.entry(body).or_default().push(item.id);
            }
        }
        self.by_body.retain(|_, ids| !ids.is_empty());
        self.revision = Some(items.membership_revision());
        true
    }

    pub(crate) fn items(&self, body: RigidBodyHandle) -> &[ItemId] {
        self.by_body.get(&body).map_or(&[], Vec::as_slice)
    }
}

impl ItemStore {
    pub(crate) fn restore_save(
        &mut self,
        ids: &crate::save::IdentityMap,
        saved: &[crate::save::ItemSnapshot],
        reference: &ReferenceManifest,
    ) -> Result<(), crate::save::SaveError> {
        use crate::save::ItemLocationSave;
        for record in saved {
            let state = self.get_mut(ids.item(&record.instance)?).unwrap();
            state.location = match &record.location {
                ItemLocationSave::World | ItemLocationSave::GeneratedAttachment { .. } => {
                    ItemLocation::World
                }
                ItemLocationSave::Held => ItemLocation::Held,
                ItemLocationSave::Stored { slot } => ItemLocation::Stored(*slot),
                ItemLocationSave::Equipped => ItemLocation::Equipped,
                ItemLocationSave::Attached { owner } => ItemLocation::Attached(ids.item(owner)?),
                ItemLocationSave::PartSlot { slot } => ItemLocation::PartSlot(ids.part_slots[slot]),
                ItemLocationSave::WheelSlot { slot } => {
                    ItemLocation::WheelSlot(ids.wheel_slots[slot])
                }
            };
            state.pending_attachment = matches!(
                record.location,
                ItemLocationSave::GeneratedAttachment { .. }
            );
            state.local_pose = crate::save::local_pose(&record.local_pose);
            state.food = record.food.as_ref().map(|food| FoodState {
                prototype_index: reference
                    .survival
                    .foods
                    .iter()
                    .position(|p| crate::save::SourceKey::from(&p.source) == food.source)
                    .expect("validated food source"),
                bite: food.bite,
            });
            state.removed = record.removed;
            state.temperature = record.temperature;
            if let (Some(fluid), Some(saved)) = (&mut state.fluid, &record.fluids) {
                fluid.restore_entries(saved);
            }
        }
        Ok(())
    }
    pub(crate) fn cleanup_loaded_fluids(&mut self) {
        for state in &mut self.states {
            if let Some(fluid) = &mut state.fluid {
                fluid.cleanup_one();
            }
        }
    }
    /// Stable generated identity and active physical state for change capture.
    pub fn mark_dirty(&mut self, id: ItemId) {
        self.dirty.insert(id);
    }
    pub fn take_dirty(&mut self) -> Option<ItemId> {
        let id = self.dirty.iter().next().copied()?;
        self.dirty.remove(&id);
        Some(id)
    }
    pub fn object_key(&self, id: ItemId) -> crate::world::ObjectKey {
        self.generated
            .iter()
            .find(|(_, value)| **value == id)
            .map_or(crate::world::ObjectKey::Imported(id), |(key, _)| {
                crate::world::ObjectKey::Generated(key.clone())
            })
    }
    pub fn resolve_key(&self, key: &crate::world::ObjectKey) -> Option<ItemId> {
        match key {
            crate::world::ObjectKey::Imported(id) => self.get(*id).map(|_| *id),
            crate::world::ObjectKey::Generated(id) => self.generated.get(id).copied(),
        }
    }
    pub fn is_generated(&self, id: ItemId) -> bool {
        self.generated.values().any(|value| *value == id)
    }
    pub fn generated_bindings(&self) -> impl Iterator<Item = (&GeneratedId, &ItemState)> {
        self.generated
            .iter()
            .filter_map(|(key, id)| self.get(*id).map(|state| (key, state)))
    }
    pub fn generated_item_id(&self, id: &GeneratedId) -> Option<ItemId> {
        self.generated.get(id).copied()
    }
    pub fn generated_item(&self, id: &GeneratedId) -> Option<&ItemState> {
        self.generated.get(id).and_then(|item| self.get(*item))
    }
    pub(crate) fn retained_binding_bytes(&self) -> usize {
        use super::memory::{map, platform_map, set, strings, vector};
        set(&self.dirty)
            + vector(&self.states)
            + platform_map(&self.indices)
            + vector(&self.body_ids)
            + strings(&self.names)
            + map(&self.slot_owners)
            + platform_map(&self.generated)
            + set(&self.initialized_supplies)
            + self
                .generated
                .keys()
                .map(|id| {
                    id.generator.file.capacity()
                        + vector(&id.children)
                        + id.children
                            .iter()
                            .map(|(s, _)| s.file.capacity())
                            .sum::<usize>()
                })
                .sum::<usize>()
    }

    pub fn new(reference: &ReferenceManifest) -> Result<Self, ReferenceError> {
        let sausage = reference
            .survival
            .foods
            .iter()
            .position(|food| food.kind == crate::reference::FoodKind::Sausage)
            .ok_or_else(|| ReferenceError("Missing sausage prototype".into()))?;
        let mut states = Vec::with_capacity(reference.items.len());
        let mut indices = BevyHashMap::with_capacity(reference.items.len());
        let mut body_ids = Vec::with_capacity(reference.items.len());
        for item in &reference.items {
            let pose = node_pose(reference, item.node)?;
            let body = reference
                .bodies
                .iter()
                .find(|body| body.id == item.body)
                .ok_or_else(|| ReferenceError(format!("Missing item body {}", item.body.0)))?;
            let body_pose = VehiclePose {
                position: body.pose.translation,
                rotation: body.pose.rotation,
            };
            let local_pose = from_rapier(&(to_rapier(body_pose).inverse() * to_rapier(pose)));
            let fluid = reference
                .tanks
                .iter()
                .find(|tank| tank.item == item.id)
                .map(FluidContainer::from);
            if indices.insert(item.id, states.len()).is_some() {
                return Err(ReferenceError(format!("Duplicate item {}", item.id.0)));
            }
            body_ids.push(item.body);
            states.push(ItemState {
                id: item.id,
                body: None,
                inactive_pose: None,
                location: item
                    .attached_to
                    .map_or(ItemLocation::World, ItemLocation::Attached),
                pose,
                local_pose,
                generated_condition: None,
                temperature: reference
                    .vehicles
                    .iter()
                    .find(|vehicle| vehicle.engine.item == item.id)
                    .map(|vehicle| vehicle.engine.initial_temperature),
                fluid,
                food: (item.id == reference.survival.starter_food).then_some(FoodState {
                    prototype_index: sausage,
                    bite: reference.survival.foods[sausage].index,
                }),
                hit_sound: item
                    .pickup
                    .as_ref()
                    .and(item.collision_clip)
                    .map(|clip| HitSound {
                        clip,
                        gain: item.collision_gain,
                    }),
                removed: false,
                pending_attachment: false,
            });
        }
        let mut vehicle_tanks = Vec::with_capacity(reference.vehicles.len());
        for vehicle in &reference.vehicles {
            let ids = [
                vehicle.engine.fuel_tank,
                vehicle.engine.oil_tank,
                vehicle.engine.coolant_tank,
            ];
            let mut tanks = [0; 3];
            for (index, id) in ids.iter().enumerate() {
                tanks[index] = *indices
                    .get(id)
                    .ok_or_else(|| ReferenceError(format!("Missing engine tank {}", id.0)))?;
                if states[tanks[index]].fluid.is_none() {
                    return Err(ReferenceError(format!(
                        "Missing engine fluid container {}",
                        id.0
                    )));
                }
            }
            if tanks[0] == tanks[1] || tanks[0] == tanks[2] || tanks[1] == tanks[2] {
                return Err(ReferenceError(
                    "Engine tanks must have distinct item IDs".into(),
                ));
            }
            vehicle_tanks.push(tanks);
        }
        for slot in &reference.wheel_slots {
            for id in [slot.wheel, slot.rim] {
                if let Some(index) = indices.get(&id) {
                    states[*index].location = ItemLocation::WheelSlot(slot.id);
                }
            }
            if let Some(index) = indices.get(&slot.tire) {
                states[*index].location = ItemLocation::Attached(slot.rim);
            }
        }
        for slot in &reference.part_slots {
            if let Some(id) = slot.part {
                states[*indices
                    .get(&id)
                    .ok_or_else(|| ReferenceError("Missing mounted part".into()))?]
                .location = ItemLocation::PartSlot(slot.id);
            }
        }
        Ok(Self {
            dirty: HashSet::new(),
            generated: BevyHashMap::new(),
            membership_revision: 0,
            initialized_supplies: HashSet::new(),
            next_generated: reference
                .items
                .iter()
                .map(|item| item.id.0)
                .max()
                .unwrap_or(0)
                .checked_add(1)
                .ok_or_else(|| ReferenceError("Item identity space exhausted".into()))?,
            origin: WorldOrigin::default(),
            vehicle_items: reference
                .vehicles
                .iter()
                .map(|vehicle| {
                    reference
                        .items
                        .iter()
                        .find(|i| i.node == vehicle.root)
                        .expect("vehicle item")
                        .id
                })
                .collect(),
            slot_owners: reference
                .part_slots
                .iter()
                .map(|s| (s.id, s.owner))
                .collect(),
            states,
            indices,
            body_ids,
            vehicle_tanks,
            names: reference
                .items
                .iter()
                .map(|item| {
                    reference
                        .nodes
                        .iter()
                        .find(|node| node.id == item.node)
                        .unwrap()
                        .name
                        .clone()
                })
                .collect(),
        })
    }

    pub fn capture_save(
        &self,
        ids: &crate::save::IdentityMap,
        origin: &crate::world::WorldOrigin,
        physics: &PhysicsWorld,
        interaction: &crate::interaction::InteractionState,
        reference: &ReferenceManifest,
    ) -> Result<Vec<crate::save::ItemSnapshot>, crate::save::SaveError> {
        use crate::save::{
            AbsolutePose, ItemLocationSave, ItemSnapshot, LocalPose, identity::source_key,
        };
        let mut records = Vec::with_capacity(ids.items.len());
        for &id in ids.items.values() {
            let item = self.get(id).ok_or_else(|| {
                crate::save::SaveError::InvalidLinks("Missing source item".into())
            })?;
            let (position, rotation) = if let Some(pose) = item.inactive_pose {
                (pose.position, pose.rotation)
            } else if let Some(handle) = item.body {
                let body = physics.rapier().bodies.get(handle).ok_or_else(|| {
                    crate::save::SaveError::InvalidLinks("Missing item body".into())
                })?;
                let offset =
                    body.position().rotation * Vector::from_array(item.local_pose.position);
                // Add local offsets after the absolute conversion to retain their precision.
                let absolute = origin.absolute(body.position().translation.to_array());
                (
                    std::array::from_fn(|axis| absolute[axis] + f64::from(offset[axis])),
                    (body.position().rotation * Rotation::from_array(item.local_pose.rotation))
                        .to_array(),
                )
            } else {
                (origin.absolute(item.pose.position), item.pose.rotation)
            };
            let location = match item.location {
                ItemLocation::World => ItemLocationSave::World,
                ItemLocation::Held => ItemLocationSave::Held,
                ItemLocation::PartSlot(slot) if !ids.part_slots.values().any(|id| *id == slot) => {
                    ItemLocationSave::GeneratedAttachment {
                        attachment: crate::world::changes::capture(
                            id,
                            self,
                            interaction,
                            physics,
                            reference,
                        )
                        .and_then(|s| s.attachment)
                        .ok_or_else(|| {
                            crate::save::SaveError::InvalidLinks(
                                "Missing generated slot identity".into(),
                            )
                        })?,
                    }
                }
                ItemLocation::Stored(slot) => ItemLocationSave::Stored { slot },
                ItemLocation::Equipped => ItemLocationSave::Equipped,
                ItemLocation::PartSlot(slot) => ItemLocationSave::PartSlot {
                    slot: source_key(&ids.part_slots, slot)?,
                },
                ItemLocation::WheelSlot(slot) => ItemLocationSave::WheelSlot {
                    slot: source_key(&ids.wheel_slots, slot)?,
                },
                ItemLocation::Attached(owner)
                    if matches!(
                        self.object_key(owner),
                        crate::world::ObjectKey::Generated(_)
                    ) =>
                {
                    ItemLocationSave::GeneratedAttachment {
                        attachment: crate::world::changes::capture(
                            id,
                            self,
                            interaction,
                            physics,
                            reference,
                        )
                        .and_then(|s| s.attachment)
                        .ok_or_else(|| {
                            crate::save::SaveError::InvalidLinks(
                                "Missing generated owner identity".into(),
                            )
                        })?,
                    }
                }
                ItemLocation::Attached(owner) => ItemLocationSave::Attached {
                    owner: ids.item_instance(owner)?,
                },
            };
            records.push(ItemSnapshot {
                instance: ids.item_instance(id)?,
                pose: AbsolutePose { position, rotation },
                local_pose: LocalPose {
                    position: item.local_pose.position,
                    rotation: item.local_pose.rotation,
                },
                location,
                temperature: item.temperature,
                fluids: item.fluid.as_ref().map(capture_fluids).transpose()?,
                food: item.food.map(|food| crate::save::FoodSnapshot {
                    source: crate::save::SourceKey::from(
                        &reference.survival.foods[food.prototype_index].source,
                    ),
                    bite: food.bite,
                }),
                removed: item.removed,
            });
        }
        records.sort_by(|a, b| a.instance.cmp(&b.instance));
        Ok(records)
    }
    pub fn register_generated_slot(&mut self, id: PartSlotId, owner: Option<ItemId>) {
        self.slot_owners.insert(id, owner);
    }
    pub fn remove_generated_slot(&mut self, id: PartSlotId) {
        self.slot_owners.remove(&id);
    }
    /// Includes imported and generated items. A crossing ownership edge pins its owner.
    pub fn has_external_dependents(&self, owns: impl Fn(ItemId) -> bool) -> bool {
        self.states.iter().any(|item| {
            if owns(item.id) {
                return false;
            }
            match item.location {
                ItemLocation::Attached(parent) => owns(parent),
                ItemLocation::PartSlot(slot) => self
                    .slot_owners
                    .get(&slot)
                    .copied()
                    .flatten()
                    .is_some_and(&owns),
                ItemLocation::WheelSlot(_) => self.vehicle_items.iter().any(|item| owns(*item)),
                _ => false,
            }
        })
    }
    pub fn attached_to(&self, mut id: ItemId, root: ItemId) -> bool {
        for _ in 0..self.states.len() {
            if id == root {
                return true;
            }
            match self.get(id).map(|item| item.location) {
                Some(ItemLocation::Attached(parent)) => id = parent,
                _ => return false,
            }
        }
        false
    }
    pub fn mounted(&self, mut id: ItemId) -> bool {
        for _ in 0..self.states.len() {
            if self.vehicle_items.contains(&id) {
                return true;
            }
            match self.get(id).map(|i| i.location) {
                Some(ItemLocation::PartSlot(slot)) => {
                    if let Some(Some(owner)) = self.slot_owners.get(&slot) {
                        id = *owner;
                    } else {
                        return true;
                    }
                }
                Some(ItemLocation::WheelSlot(_)) => return true,
                Some(ItemLocation::Attached(parent)) => id = parent,
                _ => return false,
            }
        }
        false
    }
    pub fn len(&self) -> usize {
        self.states.len()
    }

    pub fn name(&self, id: ItemId) -> Option<&str> {
        self.indices
            .get(&id)
            .map(|&index| self.names[index].as_str())
    }

    /// Create an instance identity once. Decoding remains owned by the prototype cache.
    pub fn allocate_item_id(&mut self) -> Result<ItemId, ReferenceError> {
        let id = ItemId(self.next_generated);
        self.next_generated = self
            .next_generated
            .checked_add(1)
            .ok_or_else(|| ReferenceError("Item identity space exhausted".into()))?;
        if self.indices.contains_key(&id) {
            return Err(ReferenceError("Item identity collision".into()));
        }
        Ok(id)
    }
    pub fn insert_generated(
        &mut self,
        key: GeneratedId,
        prototype: &WorldPrototypeReference,
        pose: AbsolutePose,
    ) -> Result<ItemId, ReferenceError> {
        if let Some(id) = self.generated.get(&key) {
            return Ok(*id);
        }
        if !pose.position.into_iter().all(f64::is_finite)
            || !pose.rotation.into_iter().all(f32::is_finite)
        {
            return Err(ReferenceError("Nonfinite generated item pose".into()));
        }
        let id = ItemId(self.next_generated);
        let next = self
            .next_generated
            .checked_add(1)
            .ok_or_else(|| ReferenceError("Item identity space exhausted".into()))?;
        if self.indices.contains_key(&id) {
            return Err(ReferenceError("Generated item identity collision".into()));
        }
        let root = prototype
            .nodes
            .iter()
            .find(|node| node.parent.is_none())
            .ok_or_else(|| ReferenceError("Generated prototype has no root".into()))?;
        self.indices.insert(id, self.states.len());
        self.body_ids.push(BodyId(u32::MAX));
        self.names.push(root.name.clone());
        self.states.push(ItemState {
            id,
            body: None,
            inactive_pose: None,
            location: ItemLocation::World,
            pose: VehiclePose {
                position: self.origin.local(pose.position),
                rotation: pose.rotation,
            },
            local_pose: VehiclePose {
                position: [0.0; 3],
                rotation: [0.0, 0.0, 0.0, 1.0],
            },
            generated_condition: Some(1.),
            temperature: None,
            fluid: None,
            food: None,
            hit_sound: None,
            removed: false,
            pending_attachment: false,
        });
        self.membership_revision += 1;
        self.generated.insert(key, id);
        self.next_generated = next;
        Ok(id)
    }

    pub fn insert_consumable(
        &mut self,
        id: ItemId,
        prototype_index: usize,
        reference: &FoodReference,
        pose: VehiclePose,
        body: RigidBodyHandle,
    ) -> Result<(), ReferenceError> {
        if self.indices.contains_key(&id) || reference.index >= reference.bites {
            return Err(ReferenceError("Invalid consumable identity".into()));
        }
        self.indices.insert(id, self.states.len());
        self.body_ids.push(BodyId(u32::MAX));
        self.names.push(reference.kind.name().into());
        self.states.push(ItemState {
            id,
            body: Some(body),
            location: ItemLocation::World,
            pose,
            local_pose: VehiclePose {
                position: [0.; 3],
                rotation: [0., 0., 0., 1.],
            },
            generated_condition: None,
            temperature: None,
            fluid: None,
            inactive_pose: None,
            food: Some(FoodState {
                prototype_index,
                bite: reference.index,
            }),
            hit_sound: None,
            removed: false,
            pending_attachment: false,
        });
        self.membership_revision += 1;
        Ok(())
    }

    /// A runtime copy of an item prototype body. It has no imported or generated identity.
    pub fn insert_spawned(
        &mut self,
        id: ItemId,
        name: &str,
        pose: VehiclePose,
        body: RigidBodyHandle,
    ) -> Result<(), ReferenceError> {
        if self.indices.contains_key(&id) {
            return Err(ReferenceError("Invalid spawned item identity".into()));
        }
        self.indices.insert(id, self.states.len());
        self.body_ids.push(BodyId(u32::MAX));
        self.names.push(name.into());
        self.states.push(ItemState {
            id,
            body: Some(body),
            location: ItemLocation::World,
            pose,
            local_pose: VehiclePose {
                position: [0.; 3],
                rotation: [0., 0., 0., 1.],
            },
            generated_condition: None,
            temperature: None,
            fluid: None,
            inactive_pose: None,
            food: None,
            hit_sound: None,
            removed: false,
            pending_attachment: false,
        });
        self.membership_revision += 1;
        self.dirty.insert(id);
        Ok(())
    }

    pub fn body_id(&self, id: ItemId) -> Option<BodyId> {
        self.indices.get(&id).map(|index| self.body_ids[*index])
    }
    /// Source stock is applied once; later publications cannot refill the container.
    pub fn initialize_generated_supply(
        &mut self,
        id: ItemId,
        source: &SupplyDefinition,
    ) -> Result<(), ReferenceError> {
        if self.initialized_supplies.contains(&id) {
            return Ok(());
        }
        if !self.generated.values().any(|value| *value == id) {
            return Err(ReferenceError("Supply is not a generated item".into()));
        }
        let mut entries = Vec::with_capacity(source.fluids.len());
        for fluid in &source.fluids {
            let fluid_type = u8::try_from(fluid.fluid_type)
                .map_err(|_| ReferenceError("Invalid generated source fluid".into()))?;
            entries.push(FluidEntry {
                fluid_type,
                amount: fluid.amount,
            });
        }
        if !validate_fluid_entries(&entries) || entries.iter().any(|entry| entry.amount < 0.) {
            return Err(ReferenceError("Invalid generated source fluid".into()));
        }
        if !source.capacity.is_finite()
            || source.capacity <= 0.
            || source.capacity < entries.iter().map(|entry| entry.amount).sum::<f32>()
        {
            return Err(ReferenceError("Invalid generated source capacity".into()));
        }
        self.get_mut(id).unwrap().fluid = Some(FluidContainer::new(source.capacity, &entries));
        self.initialized_supplies.insert(id);
        Ok(())
    }

    pub fn generated_snapshot(
        &self,
        id: ItemId,
        valve: Option<f32>,
    ) -> Option<crate::world::ObjectSnapshot> {
        let state = self.get(id)?;
        Some(crate::world::ObjectSnapshot {
            lamp: None,
            dial: None,
            ammo: None,
            glow: None,
            radio: None,
            linear_velocity: [0.; 3],
            angular_velocity: [0.; 3],
            sleeping: false,
            held: false,
            attachment: None,
            mechanism: None,
            pose: AbsolutePose {
                position: self.origin.absolute(state.pose.position),
                rotation: state.pose.rotation,
            },
            condition: state.generated_condition.unwrap_or(1.),
            food: None,
            temperature: state.temperature,
            fluids: state.fluid.as_ref().map(crate::fluids::FluidSnapshot::from),
            valve,
        })
    }
    pub fn restore_generated(
        &mut self,
        id: ItemId,
        snapshot: &crate::world::ObjectSnapshot,
    ) -> Result<(), ReferenceError> {
        if !snapshot.pose.position.iter().all(|v| v.is_finite())
            || !snapshot.pose.rotation.iter().all(|v| v.is_finite())
            || !snapshot.condition.is_finite()
            || !snapshot
                .linear_velocity
                .iter()
                .chain(&snapshot.angular_velocity)
                .all(|v| v.is_finite())
            || snapshot.attachment.as_ref().is_some_and(|a| {
                !a.local_pose.position.iter().all(|v| v.is_finite())
                    || !a.local_pose.rotation.iter().all(|v| v.is_finite())
            })
            || snapshot.temperature.is_some_and(|v| !v.is_finite())
            || snapshot
                .fluids
                .as_ref()
                .is_some_and(|entries| !validate_fluid_entries(entries.as_slice()))
        {
            return Err(ReferenceError("Invalid generated object snapshot".into()));
        }
        let pose = VehiclePose {
            position: self.origin.local(snapshot.pose.position),
            rotation: snapshot.pose.rotation,
        };
        let item = self
            .get_mut(id)
            .ok_or_else(|| ReferenceError("Missing generated snapshot target".into()))?;
        item.pose = pose;
        if let Some(condition) = &mut item.generated_condition {
            *condition = snapshot.condition;
        }
        item.temperature = snapshot.temperature;
        if let Some(entries) = &snapshot.fluids {
            let fluid = item
                .fluid
                .as_mut()
                .ok_or_else(|| ReferenceError("Source snapshot has no container".into()))?;
            fluid.restore_entries(entries.as_slice());
        }
        Ok(())
    }
    pub fn can_remove_generated(&self, id: ItemId) -> bool {
        self.generated.values().any(|value| *value == id)
            && self
                .get(id)
                .is_some_and(|item| item.location == ItemLocation::World && item.body.is_none())
    }

    /// Unbind colliders first. Held, mounted, and attached items retain their identity.
    pub fn remove_generated(&mut self, id: ItemId) -> bool {
        if !self.generated.values().any(|value| *value == id) {
            return false;
        }
        let Some(index) = self.indices.get(&id).copied() else {
            return false;
        };
        if self.states[index].location != ItemLocation::World || self.states[index].body.is_some() {
            return false;
        }
        self.generated.retain(|_, value| *value != id);
        self.initialized_supplies.remove(&id);
        self.remove_state(index);
        true
    }
    /// Remove a spawned item. Unbind its colliders and body first.
    pub fn remove_spawned(&mut self, id: ItemId) -> bool {
        let Some(index) = self.indices.get(&id).copied() else {
            return false;
        };
        if self.is_generated(id)
            || self.body_ids[index].0 != u32::MAX
            || self.states[index].body.is_some()
        {
            return false;
        }
        self.remove_state(index);
        true
    }
    fn remove_state(&mut self, index: usize) {
        let id = self.states[index].id;
        self.membership_revision += 1;
        self.dirty.remove(&id);
        self.indices.remove(&id);
        let last = self.states.len() - 1;
        self.states.swap_remove(index);
        self.names.swap_remove(index);
        self.body_ids.swap_remove(index);
        if index < self.states.len() {
            self.indices.insert(self.states[index].id, index);
        }
        for tank in self.vehicle_tanks.iter_mut().flatten() {
            if *tank == last {
                *tank = index;
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }
    pub fn iter(&self) -> impl Iterator<Item = &ItemState> {
        self.states.iter()
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut ItemState> {
        self.dirty.extend(self.states.iter().map(|s| s.id));
        self.states.iter_mut()
    }

    #[cfg(test)]
    pub(crate) fn from_states(states: Vec<ItemState>) -> Self {
        Self {
            dirty: HashSet::new(),
            indices: states.iter().enumerate().map(|(i, s)| (s.id, i)).collect(),
            states,
            body_ids: Vec::new(),
            vehicle_tanks: Vec::new(),
            names: Vec::new(),
            vehicle_items: Vec::new(),
            slot_owners: HashMap::new(),
            generated: BevyHashMap::new(),
            membership_revision: 0,
            initialized_supplies: HashSet::new(),
            next_generated: 0,
            origin: WorldOrigin::default(),
        }
    }

    pub fn get(&self, id: ItemId) -> Option<&ItemState> {
        self.indices.get(&id).map(|index| &self.states[*index])
    }

    pub fn get_mut(&mut self, id: ItemId) -> Option<&mut ItemState> {
        let index = *self.indices.get(&id)?;
        self.dirty.insert(id);
        self.states.get_mut(index)
    }

    /// Changes only when items enter, leave, or change physical bodies.
    pub fn membership_revision(&self) -> u64 {
        self.membership_revision
    }

    /// Set physical membership before editing the remaining item state.
    pub fn set_body(
        &mut self,
        id: ItemId,
        body: Option<RigidBodyHandle>,
    ) -> Option<&mut ItemState> {
        let index = *self.indices.get(&id)?;
        if self.states[index].body != body {
            self.states[index].body = body;
            self.membership_revision += 1;
        }
        self.dirty.insert(id);
        self.states.get_mut(index)
    }

    pub fn get_pair_mut(
        &mut self,
        first: ItemId,
        second: ItemId,
    ) -> Option<(&mut ItemState, &mut ItemState)> {
        let indices = [*self.indices.get(&first)?, *self.indices.get(&second)?];
        self.dirty.insert(first);
        self.dirty.insert(second);
        let [first, second] = self.states.get_disjoint_mut(indices).ok()?;
        Some((first, second))
    }

    pub fn vehicle_fluids(&mut self, vehicle: usize) -> Result<VehicleFluids<'_>, ReferenceError> {
        for index in self.vehicle_tanks[vehicle] {
            self.dirty.insert(self.states[index].id);
        }
        let [fuel, sump, coolant] = self
            .states
            .get_disjoint_mut(self.vehicle_tanks[vehicle])
            .map_err(|_| ReferenceError("Engine tank bindings overlap".into()))?;
        Ok(VehicleFluids {
            fuel: fuel
                .fluid
                .as_mut()
                .ok_or_else(|| ReferenceError("Missing fuel state".into()))?,
            sump: sump
                .fluid
                .as_mut()
                .ok_or_else(|| ReferenceError("Missing sump state".into()))?,
            coolant: coolant
                .fluid
                .as_mut()
                .ok_or_else(|| ReferenceError("Missing coolant state".into()))?,
        })
    }

    pub fn bind_body(&mut self, id: BodyId, handle: RigidBodyHandle) {
        for (state, body) in self.states.iter_mut().zip(&self.body_ids) {
            if *body == id {
                if state.body != Some(handle) {
                    state.body = Some(handle);
                    self.membership_revision += 1;
                }
            }
        }
    }

    pub fn sync_poses(&mut self, physics: &PhysicsWorld) {
        for state in &mut self.states {
            if state.inactive_pose.is_some() {
                continue;
            }
            if let Some(handle) = state.body
                && let Some(body) = physics.rapier().bodies.get(handle)
            {
                let pose = from_rapier(&(*body.position() * to_rapier(state.local_pose)));
                if state.pose.position != pose.position
                    || state.pose.rotation != pose.rotation
                    || !body.is_sleeping()
                {
                    self.dirty.insert(state.id);
                }
                state.pose = pose;
            }
        }
    }

    pub fn rebase(&mut self, shift: [f32; 3]) {
        self.origin.shift(shift);
        for state in &mut self.states {
            if state.inactive_pose.is_some() {
                continue;
            }
            for (position, shift) in state.pose.position.iter_mut().zip(shift) {
                *position -= shift;
            }
        }
    }
}

/// Capture exact signed entries in source order without changing gameplay.
fn capture_fluids(fluid: &FluidContainer) -> Result<Vec<FluidEntry>, crate::save::SaveError> {
    let entries: Vec<_> = fluid.entries().collect();
    if !fluid.capacity_liters.is_finite()
        || fluid.capacity_liters <= 0.
        || !validate_fluid_entries(&entries)
    {
        return Err(crate::save::SaveError::CorruptData(
            "Nonfinite fluid or invalid capacity".into(),
        ));
    }
    Ok(entries)
}

#[cfg(test)]
mod g04_ordered_state {
    use super::*;

    #[test]
    fn capture_preserves_signed_state_and_rejects_nonfinite_without_mutation() {
        let entries = [
            FluidEntry {
                fluid_type: 3,
                amount: 3.,
            },
            FluidEntry {
                fluid_type: 0,
                amount: 0.,
            },
            FluidEntry {
                fluid_type: 1,
                amount: -0.25,
            },
        ];
        let mut tank = FluidContainer::new(2., &entries);
        assert_eq!(capture_fluids(&tank).unwrap(), entries);
        tank.restore_entries(&[FluidEntry {
            fluid_type: 3,
            amount: f32::INFINITY,
        }]);
        assert!(capture_fluids(&tank).is_err());
        assert_eq!(tank.entries().next().unwrap().amount, f32::INFINITY);
    }
}

#[cfg(test)]
mod g04_nonfinite_capture {
    use super::*;

    #[test]
    fn g04_nonfinite_capture() {
        let mut tank = FluidContainer::new(
            10.,
            &[
                FluidEntry {
                    fluid_type: 0,
                    amount: 1.,
                },
                FluidEntry {
                    fluid_type: 1,
                    amount: 0.,
                },
            ],
        );
        crate::fluids::spill(&mut tank, 1.);
        assert!(tank.mixture().amount(1).is_nan());
        let before: Vec<_> = tank
            .entries()
            .map(|entry| (entry.fluid_type, entry.amount.to_bits()))
            .collect();
        assert!(capture_fluids(&tank).is_err());
        let after: Vec<_> = tank
            .entries()
            .map(|entry| (entry.fluid_type, entry.amount.to_bits()))
            .collect();
        assert_eq!(after, before);
    }
}

pub fn node_pose(reference: &ReferenceManifest, id: NodeId) -> Result<VehiclePose, ReferenceError> {
    let mut position = Vector::ZERO;
    let mut rotation = Rotation::IDENTITY;
    let mut current = Some(id);
    let mut depth = 0;
    while let Some(id) = current {
        let node = reference
            .nodes
            .iter()
            .find(|node| node.id == id)
            .ok_or_else(|| ReferenceError(format!("Missing node {}", id.0)))?;
        let q = Rotation::from_xyzw(
            node.transform.rotation[0],
            node.transform.rotation[1],
            node.transform.rotation[2],
            node.transform.rotation[3],
        );
        position = Vector::from_array(node.transform.translation)
            + q * (Vector::from_array(node.transform.scale) * position);
        rotation = q * rotation;
        current = node.parent;
        depth += 1;
        if depth > reference.nodes.len() {
            return Err(ReferenceError("Cyclic node hierarchy".into()));
        }
    }
    Ok(VehiclePose {
        position: position.to_array(),
        rotation: rotation.to_array(),
    })
}

#[cfg(test)]
mod world_sync_tests {
    use super::*;
    use crate::reference::{AssetId, SourceId, Transform, WorldPrototypeNode, WorldPrototypeRole};
    use crate::world::WorldLayer;

    fn empty_store() -> ItemStore {
        ItemStore::from_states(Vec::new())
    }
    fn key(ordinal: u32) -> GeneratedId {
        GeneratedId {
            layer: WorldLayer::Prop,
            generator: SourceId {
                file: "test".into(),
                path_id: 1,
            },
            unit: [0; 2],
            ordinal,
            children: Vec::new(),
        }
    }
    fn prototype() -> WorldPrototypeReference {
        let transform = Transform {
            translation: [0.; 3],
            rotation: [0., 0., 0., 1.],
            scale: [1.; 3],
        };
        WorldPrototypeReference {
            source: key(0).generator,
            role: WorldPrototypeRole::Prop,
            asset: AssetId(0),
            nodes: vec![WorldPrototypeNode {
                source: key(0).generator,
                name: "item".into(),
                parent: None,
                active: true,
                renderer_enabled: true,
                layer: 0,
                transform: transform.clone(),
                gltf_node: 0,
            }],
            controls: Vec::new(),
            excluded_actions: Vec::new(),
            deferred_materials: Vec::new(),
            skins: Vec::new(),
            bodies: Vec::new(),
            colliders: Vec::new(),
            prefab_pose: transform,
            interiors: Vec::new(),
        }
    }
    fn insert(store: &mut ItemStore, ordinal: u32) -> ItemId {
        store
            .insert_generated(
                key(ordinal),
                &prototype(),
                AbsolutePose {
                    position: [0.; 3],
                    rotation: [0., 0., 0., 1.],
                },
            )
            .unwrap()
    }

    #[test]
    fn world_sync_item_index_tracks_rebind_removal_swap_order_and_body_reuse() {
        let mut store = empty_store();
        let a = insert(&mut store, 0);
        let b = insert(&mut store, 1);
        let c = insert(&mut store, 2);
        let body = RigidBodyHandle::from_raw_parts(3, 0);
        let rebound = RigidBodyHandle::from_raw_parts(4, 0);
        let reused = RigidBodyHandle::from_raw_parts(3, 1);
        store.set_body(a, Some(body)).unwrap();
        store.set_body(b, Some(body)).unwrap();
        store.set_body(c, Some(body)).unwrap();
        let mut index = ItemBodyIndex::default();
        assert!(index.refresh(&store));
        assert_eq!(index.items(body), &[a, b, c]);
        store.set_body(a, Some(rebound)).unwrap();
        assert!(index.refresh(&store));
        assert_eq!(index.items(body), &[b, c]);
        assert_eq!(index.items(rebound), &[a]);
        store.set_body(a, None).unwrap();
        assert!(store.remove_generated(a));
        assert!(index.refresh(&store));
        assert_eq!(index.items(body), &[c, b]);
        assert!(index.items(rebound).is_empty());
        store.set_body(b, Some(reused)).unwrap();
        store.set_body(c, Some(reused)).unwrap();
        assert!(index.refresh(&store));
        assert!(index.items(body).is_empty());
        assert_eq!(index.items(reused), &[c, b]);
        assert_eq!(store.generated_item_id(&key(0)), None);
        assert_eq!(store.generated_item_id(&key(1)), Some(b));
        let revision = store.membership_revision();
        assert!(!store.remove_generated(a));
        assert_eq!(store.membership_revision(), revision);
    }

    #[test]
    fn world_sync_pose_and_repeated_binding_do_not_rebuild_item_index() {
        let mut store = empty_store();
        let item = insert(&mut store, 0);
        let body = RigidBodyHandle::from_raw_parts(2, 0);
        store.set_body(item, Some(body)).unwrap();
        let mut index = ItemBodyIndex::default();
        assert!(index.refresh(&store));
        let revision = store.membership_revision();
        let visits = index.visits;
        for step in 0..100 {
            store.get_mut(item).unwrap().pose.position[0] = step as f32;
            store.set_body(item, Some(body)).unwrap();
            store.iter_mut().next().unwrap().pending_attachment = step % 2 == 0;
            assert!(!index.refresh(&store));
        }
        assert_eq!(store.membership_revision(), revision);
        assert_eq!(index.visits, visits);
        assert_eq!(index.items(body), &[item]);
        let second = insert(&mut store, 1);
        assert!(index.refresh(&store));
        let revision = store.membership_revision();
        assert_eq!(insert(&mut store, 1), second);
        assert_eq!(store.membership_revision(), revision);
        assert!(!index.refresh(&store));
    }
}
