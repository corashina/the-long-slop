use std::collections::HashMap;

use rapier3d::math::{Rotation, Vector};
use rapier3d::prelude::RigidBodyHandle;

use crate::fluids::{FluidContainer, VehicleFluids};
use crate::reference::{BodyId, ItemId, NodeId, ReferenceError, ReferenceManifest, WheelSlotId};
use crate::vehicle::{PhysicsWorld, VehiclePose, from_rapier, to_rapier};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemLocation {
    World,
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
    pub fluid: Option<FluidContainer>,
}

pub struct ItemStore {
    states: Vec<ItemState>,
    indices: HashMap<ItemId, usize>,
    body_ids: Vec<BodyId>,
    vehicle_tanks: [usize; 3],
    names: Vec<String>,
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
        Ok(Self {
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

    pub fn vehicle_fluids(&mut self) -> Result<VehicleFluids<'_>, ReferenceError> {
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
