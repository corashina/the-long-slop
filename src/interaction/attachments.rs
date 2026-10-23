//! Source plate targets retain multiple members and one collider owner.
use super::{
    InteractionBlock,
    items::{ItemLocation, ItemStore},
};
use crate::reference::*;
use crate::vehicle::{PhysicsWorld, VEHICLE_COLLIDER_FLAG, from_rapier, to_rapier};
use rapier3d::math::{Pose, Vector};
use rapier3d::prelude::{ColliderHandle, RigidBodyBuilder, RigidBodyHandle};
struct Shape {
    id: ColliderId,
    handle: Option<ColliderHandle>,
    local: Pose,
    enabled: bool,
}
struct Plate {
    reference: PlateReference,
    target: Option<usize>,
    body: Option<RigidBodyHandle>,
    shapes: Vec<Shape>,
}
struct Target {
    members: Vec<ItemId>,
    reference: PlateTargetReference,
    local: Pose,
}
pub struct PlateAttachments {
    plates: Vec<Plate>,
    targets: Vec<Target>,
    bound: bool,
}
fn error(message: &str) -> ReferenceError {
    ReferenceError(message.into())
}
impl PlateAttachments {
    pub fn new(r: &ReferenceManifest) -> Result<Self, ReferenceError> {
        let targets = r
            .plates
            .targets
            .iter()
            .map(|t| {
                let owner = r
                    .items
                    .iter()
                    .find(|i| i.id == t.owner)
                    .ok_or_else(|| error("Missing plate target owner"))?;
                Ok(Target {
                    members: r
                        .plates
                        .plates
                        .iter()
                        .filter(|p| p.startup_target == t.id)
                        .map(|p| p.item)
                        .collect(),
                    reference: t.clone(),
                    local: to_rapier(super::items::node_pose(r, owner.node)?).inverse()
                        * to_rapier(super::items::node_pose(r, t.node)?),
                })
            })
            .collect::<Result<Vec<_>, ReferenceError>>()?;
        let plates = r
            .plates
            .plates
            .iter()
            .map(|p| {
                let shapes = p
                    .colliders
                    .iter()
                    .map(|id| {
                        let c = r
                            .colliders
                            .iter()
                            .find(|c| c.id == *id)
                            .ok_or_else(|| error("Missing plate collider"))?;
                        Ok(Shape {
                            id: *id,
                            handle: None,
                            local: Pose::IDENTITY,
                            enabled: c.enabled,
                        })
                    })
                    .collect::<Result<Vec<_>, ReferenceError>>()?;
                Ok(Plate {
                    reference: p.clone(),
                    target: Some(
                        targets
                            .iter()
                            .position(|t| t.reference.id == p.startup_target)
                            .ok_or_else(|| error("Missing plate startup target"))?,
                    ),
                    body: None,
                    shapes,
                })
            })
            .collect::<Result<Vec<_>, ReferenceError>>()?;
        Ok(Self {
            plates,
            targets,
            bound: false,
        })
    }
    pub fn contains(&self, item: ItemId) -> bool {
        self.plates.iter().any(|p| p.reference.item == item)
    }
    pub fn target(&self, item: ItemId) -> Option<AttachmentTargetId> {
        let p = self.plates.iter().find(|p| p.reference.item == item)?;
        p.target.map(|i| self.targets[i].reference.id)
    }
    pub fn members(&self, target: AttachmentTargetId) -> impl Iterator<Item = ItemId> + '_ {
        self.targets
            .iter()
            .find(|t| t.reference.id == target)
            .into_iter()
            .flat_map(|t| t.members.iter().copied())
    }
    pub fn removal_member(&self, target: AttachmentTargetId) -> Option<ItemId> {
        let t = self.targets.iter().find(|t| t.reference.id == target)?;
        if t.reference.dont_remove_attached_when_looking {
            return None;
        }
        self.members(target).next()
    }
    pub fn loose_body(&self, item: ItemId) -> Option<RigidBodyHandle> {
        self.plates.iter().find(|p| p.reference.item == item)?.body
    }
    pub fn owns_collider(&self, id: ColliderId) -> bool {
        self.plates
            .iter()
            .any(|p| p.reference.colliders.contains(&id))
    }
    pub fn collider_target(&self, id: ColliderId) -> Option<AttachmentTargetId> {
        self.targets
            .iter()
            .find(|t| t.reference.colliders.contains(&id))
            .map(|t| t.reference.id)
    }
    pub fn target_owner(&self, id: AttachmentTargetId) -> Option<ItemId> {
        self.targets
            .iter()
            .find(|t| t.reference.id == id)
            .map(|t| t.reference.owner)
    }
    pub fn bind(
        &mut self,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
    ) -> Result<(), ReferenceError> {
        if self.bound {
            return Ok(());
        }
        // Resolve every binding before creating or moving bodies.
        for p in &mut self.plates {
            for s in &mut p.shapes {
                let (handle, collider) = physics
                    .rapier()
                    .colliders
                    .iter()
                    .find(|(_, c)| ColliderId((c.user_data >> 64) as u32) == s.id)
                    .ok_or_else(|| error("Missing plate collider binding"))?;
                s.local = to_rapier(
                    items
                        .get(p.reference.item)
                        .ok_or_else(|| error("Missing plate item"))?
                        .pose,
                )
                .inverse()
                    * *collider.position();
                s.handle = Some(handle);
            }
        }
        for p in &mut self.plates {
            let mut builder = RigidBodyBuilder::dynamic()
                .enabled(false)
                .ccd_enabled(true)
                .additional_mass(p.reference.mass_kg)
                .angular_damping(p.reference.angular_damping.unwrap_or(0.05));
            if let Some(damping) = p.reference.linear_damping {
                builder = builder.linear_damping(damping);
            }
            p.body = Some(physics.insert_body(builder));
            for s in &p.shapes {
                physics.edit_collider(s.handle.unwrap(), |collider| collider.set_density(0.));
            }
        }
        self.bound = true;
        self.sync(items, physics)
    }
    pub fn can_mount(
        &self,
        item: ItemId,
        target: AttachmentTargetId,
        items: &ItemStore,
    ) -> Result<(), InteractionBlock> {
        let p = self
            .plates
            .iter()
            .find(|p| p.reference.item == item)
            .ok_or(InteractionBlock::Unavailable)?;
        let t = self
            .targets
            .iter()
            .find(|t| t.reference.id == target)
            .ok_or(InteractionBlock::Unavailable)?;
        if !self.bound
            || p.target.is_some()
            || !items
                .get(item)
                .is_some_and(|s| matches!(s.location, ItemLocation::World | ItemLocation::Held))
            || items
                .get(t.reference.owner)
                .is_none_or(|s| s.body.is_none())
        {
            return Err(InteractionBlock::Unavailable);
        }
        // Selected source player branch ignores CanAttach when these flags are false.
        if p.reference.only_to_point || p.reference.ignore_points {
            return Err(InteractionBlock::Unavailable);
        }
        Ok(())
    }
    pub fn preflight_mount(
        &self,
        item: ItemId,
        target: AttachmentTargetId,
        items: &ItemStore,
        physics: &PhysicsWorld,
    ) -> Result<(), ReferenceError> {
        self.can_mount(item, target, items)
            .map_err(|_| error("Cannot mount plate"))?;
        let p = self
            .plates
            .iter()
            .find(|p| p.reference.item == item)
            .unwrap();
        let t = self
            .targets
            .iter()
            .find(|t| t.reference.id == target)
            .unwrap();
        let body = items.get(t.reference.owner).and_then(|s| s.body).unwrap();
        if physics.rapier().bodies.get(body).is_none()
            || p.body
                .is_none_or(|b| physics.rapier().bodies.get(b).is_none())
            || p.shapes.iter().any(|s| {
                s.handle
                    .is_none_or(|h| physics.rapier().colliders.get(h).is_none())
            })
        {
            return Err(error("Missing plate physics binding"));
        }
        Ok(())
    }
    pub fn mount(
        &mut self,
        item: ItemId,
        target: AttachmentTargetId,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
    ) -> Result<(), ReferenceError> {
        self.can_mount(item, target, items)
            .map_err(|_| error("Cannot mount plate"))?;
        if items.get(item).unwrap().location != ItemLocation::World {
            return Err(error("Release carry before plate mount"));
        }
        let index = self
            .plates
            .iter()
            .position(|p| p.reference.item == item)
            .unwrap();
        self.plates[index].target = Some(
            self.targets
                .iter()
                .position(|t| t.reference.id == target)
                .unwrap(),
        );
        self.targets[self.plates[index].target.unwrap()]
            .members
            .push(item);
        physics.edit_body(self.plates[index].body.unwrap(), |body| {
            body.set_enabled(false)
        });
        self.sync(items, physics)?;
        physics.refresh_queries();
        Ok(())
    }
    pub fn remove(
        &mut self,
        item: ItemId,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
    ) -> Result<ItemId, ReferenceError> {
        let p = self
            .plates
            .iter_mut()
            .find(|p| p.reference.item == item)
            .ok_or_else(|| error("Unknown plate"))?;
        if !self.bound || p.target.is_none() {
            return Err(error("Plate is not mounted"));
        }
        let state = items
            .get(item)
            .ok_or_else(|| error("Missing plate state"))?;
        let parent = state
            .body
            .ok_or_else(|| error("Missing plate parent body"))?;
        let mut pose = to_rapier(state.pose);
        pose.translation += pose.rotation * Vector::from_array(p.reference.remove_target_pos);
        let body = p.body.unwrap();
        let velocity = physics.rapier().bodies[parent].velocity_at_point(pose.translation);
        let angular = physics.rapier().bodies[parent].angvel();
        physics.edit_body(body, |b| {
            b.set_position(pose, true);
            b.set_enabled(true);
            b.set_linvel(velocity, true);
            b.set_angvel(angular, true);
            b.set_gravity_scale(1., true);
        });
        self.targets[p.target.unwrap()]
            .members
            .retain(|id| *id != item);
        p.target = None;
        let s = items.set_body(item, Some(body)).unwrap();
        s.location = ItemLocation::World;
        s.pose = from_rapier(&pose);
        s.local_pose = from_rapier(&Pose::IDENTITY);
        Self::move_shapes(p, body, pose, physics);

        physics.recompute_body_mass(body);
        physics.refresh_queries();
        Ok(item)
    }
    fn move_shapes(p: &Plate, body: RigidBodyHandle, pose: Pose, physics: &mut PhysicsWorld) {
        let inverse = physics.rapier().bodies[body].position().inverse();
        let vehicle = physics.vehicle_of(body).is_some();
        for s in &p.shapes {
            let h = s.handle.unwrap();

            if physics.rapier().colliders[h].parent() != Some(body) {
                physics.set_collider_parent(h, Some(body));
            }
            physics.edit_collider(h, |collider| {
                collider.set_position_wrt_parent(inverse * pose * s.local)
            });
            physics.edit_collider(h, |collider| collider.set_enabled(s.enabled));
            if vehicle {
                physics.edit_collider(h, |collider| {
                    collider.user_data |= VEHICLE_COLLIDER_FLAG;
                });
            } else {
                physics.edit_collider(h, |collider| {
                    collider.user_data &= !VEHICLE_COLLIDER_FLAG;
                });
            }
        }
    }
    pub fn sync(
        &mut self,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
    ) -> Result<(), ReferenceError> {
        if !self.bound {
            return Err(error("Plate attachments are not bound"));
        }
        for p in &self.plates {
            if let Some(index) = p.target {
                let t = &self.targets[index];
                let owner = items
                    .get(t.reference.owner)
                    .ok_or_else(|| error("Missing plate target owner"))?;
                let body = owner
                    .body
                    .ok_or_else(|| error("Missing plate target body"))?;
                let pose = to_rapier(owner.pose) * t.local;
                let local = physics.rapier().bodies[body].position().inverse() * pose;
                let s = items.set_body(p.reference.item, Some(body)).unwrap();
                s.location = ItemLocation::Attached(t.reference.owner);
                s.pose = from_rapier(&pose);
                s.local_pose = from_rapier(&local);
                Self::move_shapes(p, body, pose, physics);
            }
        }
        Ok(())
    }
    // Target offsets and shape offsets are local. PhysicsWorld rebases retained loose bodies.
    pub fn rebase(&mut self, _shift: [f32; 3]) {}
}
