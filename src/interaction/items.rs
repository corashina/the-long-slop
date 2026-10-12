use std::collections::HashMap;

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
    states: Vec<ItemState>,
    indices: HashMap<ItemId, usize>,
    body_ids: Vec<BodyId>,
    /// Fuel, sump, and coolant state indices per vehicle.
    vehicle_tanks: Vec<[usize; 3]>,
    names: Vec<String>,
    vehicle_items: Vec<ItemId>,
    slot_owners: HashMap<PartSlotId, Option<ItemId>>,
}

impl ItemStore {
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
                temperature: reference
                    .vehicles
                    .iter()
                    .find(|vehicle| vehicle.engine.item == item.id)
                    .map(|vehicle| vehicle.engine.initial_temperature),
                fluid,
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
        self.states.iter_mut()
    }

    pub fn get(&self, id: ItemId) -> Option<&ItemState> {
        self.indices.get(&id).map(|index| &self.states[*index])
    }

    pub fn get_mut(&mut self, id: ItemId) -> Option<&mut ItemState> {
        let index = *self.indices.get(&id)?;
        self.states.get_mut(index)
    }

    pub fn get_pair_mut(
        &mut self,
        first: ItemId,
        second: ItemId,
    ) -> Option<(&mut ItemState, &mut ItemState)> {
        let indices = [*self.indices.get(&first)?, *self.indices.get(&second)?];
        let [first, second] = self.states.get_disjoint_mut(indices).ok()?;
        Some((first, second))
    }

    pub fn vehicle_fluids(&mut self, vehicle: usize) -> Result<VehicleFluids<'_>, ReferenceError> {
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
            if let Some(handle) = state.body
                && let Some(body) = physics.rapier().bodies.get(handle)
            {
                state.pose = from_rapier(&(*body.position() * to_rapier(state.local_pose)));
            }
        }
    }

    pub fn rebase(&mut self, shift: [f32; 3]) {
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
