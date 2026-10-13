use crate::reference::WorldPrototypeReference;
use crate::world::{AbsolutePose, GeneratedId, SupplyDefinition, WorldOrigin};
use std::collections::{HashMap, HashSet};

use rapier3d::math::{Rotation, Vector};
use rapier3d::prelude::RigidBodyHandle;

use crate::fluids::{FluidContainer, VehicleFluids};
use crate::reference::{
    BodyId, ItemId, NodeId, PartSlotId, ReferenceError, ReferenceManifest, WheelSlotId,
};
use crate::vehicle::{PhysicsWorld, VehiclePose, from_rapier, to_rapier};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemLocation {
    World,
    PartSlot(PartSlotId),
    Held,
    Attached(ItemId),
    WheelSlot(WheelSlotId),
}

#[derive(Clone, Debug)]
pub struct ItemState {
    pub id: ItemId,
    pub body: Option<RigidBodyHandle>,
    pub location: ItemLocation,
    pub pose: VehiclePose,
    pub local_pose: VehiclePose,
    /// One is new; zero is the most worn source state.
    pub condition: f32,
    pub temperature: Option<f32>,
    pub fluid: Option<FluidContainer>,
}

pub struct ItemStore {
    dirty: HashSet<ItemId>,
    states: Vec<ItemState>,
    indices: HashMap<ItemId, usize>,
    body_ids: Vec<BodyId>,
    vehicle_tanks: [usize; 3],
    names: Vec<String>,
    vehicle_item: ItemId,
    slot_owners: HashMap<PartSlotId, Option<ItemId>>,
    generated: HashMap<GeneratedId, ItemId>,
    initialized_supplies: HashSet<ItemId>,
    next_generated: u32,
    origin: WorldOrigin,
}

impl ItemStore {
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
    pub fn generated_bindings(&self) -> impl Iterator<Item = (&GeneratedId, &ItemState)> {
        self.generated
            .iter()
            .filter_map(|(key, id)| self.get(*id).map(|state| (key, state)))
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
                location: item
                    .attached_to
                    .map_or(ItemLocation::World, ItemLocation::Attached),
                pose,
                local_pose,
                condition: item.condition,
                temperature: (item.id == reference.engine.item)
                    .then_some(reference.engine.initial_temperature),
                fluid,
            });
        }
        let ids = [
            reference.engine.fuel_tank,
            reference.engine.oil_tank,
            reference.engine.coolant_tank,
        ];
        let mut vehicle_tanks = [0; 3];
        for (index, id) in ids.iter().enumerate() {
            vehicle_tanks[index] = *indices
                .get(id)
                .ok_or_else(|| ReferenceError(format!("Missing engine tank {}", id.0)))?;
            if states[vehicle_tanks[index]].fluid.is_none() {
                return Err(ReferenceError(format!(
                    "Missing engine fluid container {}",
                    id.0
                )));
            }
        }
        if vehicle_tanks[0] == vehicle_tanks[1]
            || vehicle_tanks[0] == vehicle_tanks[2]
            || vehicle_tanks[1] == vehicle_tanks[2]
        {
            return Err(ReferenceError(
                "Engine tanks must have distinct item IDs".into(),
            ));
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
            vehicle_item: reference
                .items
                .iter()
                .find(|i| i.node == reference.vehicle.root)
                .expect("vehicle item")
                .id,
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
                ItemLocation::WheelSlot(_) => owns(self.vehicle_item),
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
            if id == self.vehicle_item {
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
            location: ItemLocation::World,
            pose: VehiclePose {
                position: self.origin.local(pose.position),
                rotation: pose.rotation,
            },
            local_pose: VehiclePose {
                position: [0.0; 3],
                rotation: [0.0, 0.0, 0.0, 1.0],
            },
            condition: 1.0,
            temperature: None,
            fluid: None,
        });
        self.generated.insert(key, id);
        self.next_generated = next;
        Ok(id)
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
        self.get_mut(id).unwrap().fluid = Some(FluidContainer {
            capacity_liters: source.capacity,
            mixture,
        });
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
            condition: state.condition,
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
        item.condition = snapshot.condition;
        item.temperature = snapshot.temperature;
        if let Some(quantities) = snapshot.fluid_quantities {
            let fluid = item
                .fluid
                .as_mut()
                .ok_or_else(|| ReferenceError("Source snapshot has no container".into()))?;
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
        for tank in &mut self.vehicle_tanks {
            if *tank == last {
                *tank = index;
            }
        }
        true
    }

    pub fn insert_supply(&mut self, id: ItemId, pose: VehiclePose) {
        assert!(!self.indices.contains_key(&id), "Duplicate station supply");
        self.indices.insert(id, self.states.len());
        self.body_ids.push(BodyId(u32::MAX));
        self.names.push("Gasoline pump".into());
        self.states.push(ItemState {
            id,
            body: None,
            location: ItemLocation::World,
            pose,
            local_pose: VehiclePose {
                position: [0.0; 3],
                rotation: [0.0, 0.0, 0.0, 1.0],
            },
            condition: 1.0,
            temperature: None,
            fluid: Some(FluidContainer {
                capacity_liters: 1000.0,
                mixture: crate::fluids::FluidMixture {
                    quantities_liters: [1000.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                },
            }),
        });
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

    pub fn vehicle_fluids(&mut self) -> Result<VehicleFluids<'_>, ReferenceError> {
        for index in self.vehicle_tanks {
            self.dirty.insert(self.states[index].id);
        }
        let [fuel, sump, coolant] = self
            .states
            .get_disjoint_mut(self.vehicle_tanks)
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
            for (position, shift) in state.pose.position.iter_mut().zip(shift) {
                *position -= shift;
            }
        }
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
