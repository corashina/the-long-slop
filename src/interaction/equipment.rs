//! Minimal inventory retrieval into the source right hand.
use super::items::{ItemLocation, ItemStore};
use crate::{
    player::pose::PlayerPoseState,
    reference::{ItemId, NodeId, PickupReference, ReferenceManifest},
    vehicle::{PhysicsWorld, VehiclePose, to_rapier},
};
use rapier3d::prelude::{ColliderHandle, RigidBodyType};
pub struct EquipmentState {
    item: Option<ItemId>,
    anchor: NodeId,
    colliders: Vec<(ColliderHandle, bool)>,
}
impl EquipmentState {
    pub fn new(r: &ReferenceManifest) -> Self {
        Self {
            item: None,
            anchor: r.clothing.pose.anchors["RH"].node,
            colliders: Vec::with_capacity(r.colliders.len()),
        }
    }
    pub fn item(&self) -> Option<ItemId> {
        self.item
    }
    pub(crate) fn retrieve(
        &mut self,
        id: ItemId,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
    ) {
        assert!(self.item.is_none());
        let item = items.get_mut(id).expect("validated equipment item");
        let body = item.body.expect("validated equipment body");
        self.colliders.clear();
        for &handle in physics.rapier().bodies[body].colliders() {
            self.colliders
                .push((handle, physics.rapier().colliders[handle].is_enabled()));
        }
        for &(handle, _) in &self.colliders {
            physics.edit_collider(handle, |collider| collider.set_enabled(false));
        }
        physics.edit_body(body, |rb| {
            rb.set_body_type(RigidBodyType::KinematicPositionBased, true);
            rb.set_linvel(rapier3d::math::Vector::ZERO, false);
            rb.set_angvel(rapier3d::math::Vector::ZERO, false);
        });
        item.location = ItemLocation::Equipped;
        self.item = Some(id);
    }
    pub(crate) fn release(
        &mut self,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
    ) -> Option<ItemId> {
        let id = self.item.take()?;
        let item = items.get_mut(id).unwrap();
        physics.edit_body(item.body.unwrap(), |body| {
            body.set_body_type(RigidBodyType::Dynamic, true)
        });
        for &(handle, enabled) in &self.colliders {
            physics.edit_collider(handle, |collider| collider.set_enabled(enabled));
        }
        self.colliders.clear();
        item.location = ItemLocation::World;
        Some(id)
    }
    pub fn sync(
        &self,
        pose: &PlayerPoseState,
        items: &ItemStore,
        rule: &PickupReference,
        physics: &mut PhysicsWorld,
    ) {
        let Some(id) = self.item else { return };
        let item = items.get(id).unwrap();
        let offset = if rule.disable_rh_fix {
            VehiclePose {
                position: [0.; 3],
                rotation: [0., 0., 0., 1.],
            }
        } else {
            VehiclePose {
                position: rule.right_hand_position,
                rotation: rule.right_hand_rotation,
            }
        };
        let target = to_rapier(pose.anchor(self.anchor).expect("retained source hand"))
            * to_rapier(offset)
            * to_rapier(item.local_pose).inverse();
        physics.edit_body(item.body.unwrap(), |body| {
            body.set_position(target, false);
            body.set_next_kinematic_position(target);
        });
    }
}

#[cfg(test)]
impl EquipmentState {
    pub(super) fn empty_for_wheel_actions() -> Self {
        Self {
            item: None,
            anchor: NodeId(0),
            colliders: Vec::new(),
        }
    }
}
