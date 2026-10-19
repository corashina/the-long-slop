use crate::reference::WorldPrototypeReference;
use crate::world::{AbsolutePose, GeneratedId, SupplyDefinition, WorldOrigin};
use std::collections::{HashMap, HashSet};

use rapier3d::math::{Rotation, Vector};
use rapier3d::prelude::RigidBodyHandle;

use crate::fluids::{FluidContainer, VehicleFluids};
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
    pub removed: bool,
    pub pending_attachment: bool,
}

pub struct ItemStore {
    dirty: HashSet<ItemId>,
    states: Vec<ItemState>,
    indices: HashMap<ItemId, usize>,
    body_ids: Vec<BodyId>,
    /// Fuel, sump, and coolant state indices per vehicle.
    vehicle_tanks: Vec<[usize; 3]>,
    names: Vec<String>,
    vehicle_items: Vec<ItemId>,
    slot_owners: HashMap<PartSlotId, Option<ItemId>>,
    generated: HashMap<GeneratedId, ItemId>,
    initialized_supplies: HashSet<ItemId>,
    next_generated: u32,
    origin: WorldOrigin,
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
            if let (Some(fluid), Some(quantities)) = (&mut state.fluid, record.fluids) {
                fluid.mixture.quantities_liters = quantities;
            }
        }
        Ok(())
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
    pub fn generated_item(&self, id: &GeneratedId) -> Option<&ItemState> {
        self.generated.get(id).and_then(|item| self.get(*item))
    }
    pub(crate) fn retained_binding_bytes(&self) -> usize {
        use super::memory::{map, set, strings, vector};
        set(&self.dirty)
            + vector(&self.states)
            + map(&self.indices)
            + vector(&self.body_ids)
            + strings(&self.names)
            + map(&self.slot_owners)
            + map(&self.generated)
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
        let mut indices = HashMap::with_capacity(reference.items.len());
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
            generated: HashMap::new(),
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
            removed: false,
            pending_attachment: false,
        });
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
        self.names.push(match reference.kind {
            crate::reference::FoodKind::Sausage => "Sausage".into(),
            crate::reference::FoodKind::Turd => "Turd".into(),
        });
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
            removed: false,
            pending_attachment: false,
        });
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
        let mut mixture = crate::fluids::FluidMixture::default();
        for fluid in &source.fluids {
            if !(0..6).contains(&fluid.fluid_type)
                || !fluid.amount.is_finite()
                || fluid.amount < 0.0
            {
                return Err(ReferenceError("Invalid generated source fluid".into()));
            }
            mixture.quantities_liters[fluid.fluid_type as usize] += fluid.amount;
        }
        if !source.capacity.is_finite() || source.capacity < mixture.total() {
            return Err(ReferenceError("Invalid generated source capacity".into()));
        }
        self.get_mut(id).unwrap().fluid = Some(FluidContainer::new(source.capacity, mixture));
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
            fluid_quantities: state
                .fluid
                .as_ref()
                .map(|fluid| fluid.mixture.quantities_liters),
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
                .fluid_quantities
                .is_some_and(|values| values.iter().any(|v| !v.is_finite() || *v < 0.))
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
        if let Some(quantities) = snapshot.fluid_quantities {
            let fluid = item
                .fluid
                .as_mut()
                .ok_or_else(|| ReferenceError("Source snapshot has no container".into()))?;
            if quantities.iter().map(|v| f64::from(*v)).sum::<f64>()
                > f64::from(fluid.capacity_liters) + 0.001
            {
                return Err(ReferenceError(
                    "Restored fluid exceeds source capacity".into(),
                ));
            }
            fluid.mixture.quantities_liters = quantities;
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
        true
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

    pub fn get(&self, id: ItemId) -> Option<&ItemState> {
        self.indices.get(&id).map(|index| &self.states[*index])
    }

    pub fn get_mut(&mut self, id: ItemId) -> Option<&mut ItemState> {
        let index = *self.indices.get(&id)?;
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
                state.body = Some(handle);
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

/// Project source signed state into the nonnegative core save format without changing gameplay.
fn capture_fluids(fluid: &FluidContainer) -> Result<[f32; 6], crate::save::SaveError> {
    let mut quantities = fluid.mixture.quantities_liters;
    if !fluid.capacity_liters.is_finite()
        || fluid.capacity_liters < 0.
        || quantities.iter().any(|amount| !amount.is_finite())
    {
        return Err(crate::save::SaveError::CorruptData(
            "Nonfinite fluid or invalid capacity".into(),
        ));
    }
    for amount in &mut quantities {
        *amount = amount.max(0.);
    }
    let total: f64 = quantities.iter().map(|amount| f64::from(*amount)).sum();
    let capacity = f64::from(fluid.capacity_liters);
    if total > capacity {
        let scale = capacity / total;
        let mut remaining = capacity;
        for amount in &mut quantities {
            let scaled = (f64::from(*amount) * scale).min(remaining);
            let rounded = scaled as f32;
            // Round toward zero so the saved positive sum stays within capacity.
            *amount = if f64::from(rounded) > scaled {
                rounded.next_down()
            } else {
                rounded
            };
            remaining = (remaining - f64::from(*amount)).max(0.);
        }
    }
    Ok(quantities)
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
impl ItemStore {
    pub(super) fn wheel_actions_fixture(states: Vec<ItemState>) -> Self {
        Self {
            indices: states
                .iter()
                .enumerate()
                .map(|(i, state)| (state.id, i))
                .collect(),
            body_ids: states.iter().map(|state| BodyId(state.id.0)).collect(),
            names: states
                .iter()
                .map(|state| format!("wheel fixture {}", state.id.0))
                .collect(),
            next_generated: states.iter().map(|state| state.id.0).max().unwrap() + 1,
            states,
            dirty: HashSet::new(),
            vehicle_tanks: Vec::new(),
            vehicle_items: Vec::new(),
            slot_owners: HashMap::new(),
            generated: HashMap::new(),
            initialized_supplies: HashSet::new(),
            origin: WorldOrigin::default(),
        }
    }
}
