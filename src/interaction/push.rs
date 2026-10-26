//! Source push force and a reusable steering grip.
use super::AimTarget;
use crate::player::PlayerState;
use crate::reference::{BodyId, ItemId, PlayerReference, PushReference, ReferenceManifest};
use crate::simulation::{FrameInput, keys};
use crate::vehicle::{PhysicsWorld, to_rapier};
use rapier3d::math::{Pose, Rotation, Vector};
use rapier3d::prelude::{GenericJointBuilder, ImpulseJointHandle, JointAxesMask, RigidBodyHandle};
use std::collections::HashMap;

struct Grip {
    owner: ItemId,
    body: RigidBodyHandle,
    collider: rapier3d::prelude::ColliderHandle,
    joint: ImpulseJointHandle,
    previous_pose: Pose,
}
pub struct PushState {
    rules: HashMap<ItemId, PushReference>,
    player: PlayerReference,
    /// Maximum steering angle per vehicle, in radians.
    steering_angles: Vec<f32>,
    grip: Option<Grip>,
    cancelled: bool,
    steer: Option<f32>,
}
impl PushState {
    pub fn new(reference: &ReferenceManifest) -> Self {
        Self {
            rules: reference
                .pushables
                .iter()
                .map(|r| (r.owner, r.clone()))
                .collect(),
            player: reference.player.clone(),
            steering_angles: reference
                .vehicles
                .iter()
                .map(|vehicle| vehicle.max_steer_degrees.to_radians())
                .collect(),
            grip: None,
            cancelled: false,
            steer: None,
        }
    }
    pub fn is_grip(&self, item: ItemId, collider: crate::reference::ColliderId) -> bool {
        self.rules
            .get(&item)
            .and_then(|rule| rule.grip.as_ref())
            .is_some_and(|grip| grip.colliders.contains(&collider))
    }
    pub fn supports(&self, item: ItemId) -> bool {
        self.rules.contains_key(&item)
    }
    pub(super) fn can_start(
        &self,
        target: AimTarget,
        bodies: &HashMap<BodyId, RigidBodyHandle>,
        player: &PlayerState,
        physics: &PhysicsWorld,
    ) -> bool {
        let Some(rule) = self.rules.get(&target.item) else {
            return false;
        };
        let Some(&body) = bodies.get(&rule.body) else {
            return false;
        };
        let Some(collider) = physics.rapier().colliders.get(target.collider) else {
            return false;
        };
        if target.slot.is_some()
            || player.seat().is_some()
            || !target.point.iter().all(|v| v.is_finite())
            || physics
                .rapier()
                .bodies
                .get(body)
                .is_none_or(|b| !b.is_dynamic() || !b.is_enabled())
            || !collider.is_enabled()
        {
            return false;
        }
        let id = crate::reference::ColliderId((collider.user_data >> 64) as u32);
        if self.is_grip(target.item, id) {
            let grip = rule.grip.as_ref().unwrap();
            player.grounded()
                && !grip.in_use
                && player.support_body() != Some(body)
                && self.grip_in_range(grip, body, player, physics)
        } else {
            rule.push_even_if_same_parent
                || !player.grounded()
                || player.support_body() != Some(body)
        }
    }
    fn grip_in_range(
        &self,
        rule: &crate::reference::PushGripReference,
        body: RigidBodyHandle,
        player: &PlayerState,
        physics: &PhysicsWorld,
    ) -> bool {
        let point = *physics.rapier().bodies[body].position()
            * Vector::from_array(rule.local_pose.translation);
        Vector::from_array(player.body_pose(physics).position).distance_squared(point)
            <= self.player.grip_distance.powi(2)
    }
    pub fn cancel_input(&mut self) {
        self.cancelled = true;
        self.steer = None;
    }
    pub fn steering(&self) -> Option<(RigidBodyHandle, f32)> {
        Some((self.grip.as_ref()?.body, self.steer?))
    }
    pub fn gripping(&self) -> bool {
        self.grip.is_some() && !self.cancelled
    }
    pub fn stop(&mut self, player: &mut PlayerState, physics: &mut PhysicsWorld) {
        if let Some(grip) = self.grip.take() {
            physics.remove_impulse_joint(grip.joint, true);
        }
        player.set_push_gripping(false, physics);
        self.steer = None;
        self.cancelled = false;
    }
    pub(super) fn finish_cancelled_input(
        &mut self,
        player: &mut PlayerState,
        physics: &mut PhysicsWorld,
    ) {
        if self.cancelled {
            self.stop(player, physics);
        }
    }
    pub fn step(
        &mut self,
        input: FrameInput,
        target: Option<AimTarget>,
        bodies: &HashMap<BodyId, RigidBodyHandle>,
        player: &mut PlayerState,
        physics: &mut PhysicsWorld,
        dt: f32,
    ) -> Option<ItemId> {
        if self.grip.is_none() && (input.held & keys::GRAB == 0 || input.released & keys::GRAB != 0)
        {
            self.stop(player, physics);
            return None;
        }
        let target = if let Some(grip) = &self.grip {
            if physics
                .rapier()
                .bodies
                .get(grip.body)
                .is_none_or(|body| !body.is_dynamic() || !body.is_enabled())
                || physics
                    .rapier()
                    .colliders
                    .get(grip.collider)
                    .is_none_or(|collider| {
                        !collider.is_enabled() || collider.parent() != Some(grip.body)
                    })
            {
                self.stop(player, physics);
                return None;
            }
            Some(AimTarget {
                item: grip.owner,
                collider: grip.collider,
                distance: 0.0,
                point: physics.rapier().colliders[grip.collider]
                    .position()
                    .translation
                    .to_array(),
                slot: None,
            })
        } else {
            target
        };
        let Some(target) = target else {
            self.stop(player, physics);
            return None;
        };
        if input.pressed & keys::GRAB != 0 && self.grip.is_some() {
            self.stop(player, physics);
            return None;
        }
        if self.grip.is_none() && !self.can_start(target, bodies, player, physics) {
            self.stop(player, physics);
            return None;
        }
        let Some(rule) = self.rules.get(&target.item) else {
            self.stop(player, physics);
            return None;
        };
        let Some(&body_handle) = bodies.get(&rule.body) else {
            self.stop(player, physics);
            return None;
        };
        if target.slot.is_some()
            || player.seat().is_some()
            || !target.point.iter().all(|v| v.is_finite())
        {
            self.stop(player, physics);
            return None;
        }
        let is_grip = rule.grip.as_ref().is_some_and(|grip| {
            let id = crate::reference::ColliderId(
                (physics
                    .rapier()
                    .colliders
                    .get(target.collider)
                    .map_or(0, |c| c.user_data)
                    >> 64) as u32,
            );
            grip.colliders.contains(&id)
        });
        if is_grip {
            let grip_rule = rule.grip.as_ref().unwrap();
            let car_pose = *physics.rapier().bodies[body_handle].position();
            let position = Vector::from_array(player.body_pose(physics).position);
            if !self.grip_in_range(grip_rule, body_handle, player, physics)
                || self
                    .grip
                    .as_ref()
                    .is_some_and(|g| g.owner != target.item || g.collider != target.collider)
            {
                self.stop(player, physics);
                return None;
            }
            if self.grip.is_none() {
                if input.pressed & keys::GRAB == 0 {
                    return None;
                }
                let player_pose = *physics.rapier().bodies[player.body_handle()].position();
                let joint = GenericJointBuilder::new(JointAxesMask::LIN_X | JointAxesMask::LIN_Z)
                    .local_frame1(Pose::IDENTITY)
                    .local_frame2(car_pose.inverse() * player_pose);
                let joint =
                    physics.insert_impulse_joint(player.body_handle(), body_handle, joint, true);
                self.grip = Some(Grip {
                    owner: target.item,
                    body: body_handle,
                    collider: target.collider,
                    joint,
                    previous_pose: car_pose,
                });
            }
            let grip = self.grip.as_mut().unwrap();
            debug_assert_eq!(grip.body, body_handle);
            let before = grip.previous_pose.rotation * -Vector::Z;
            let after = car_pose.rotation * -Vector::Z;
            let turn = before.angle_between(after).to_degrees() * before.cross(after).y.signum();
            player.follow_push_grip(turn);
            grip.previous_pose = car_pose;
            player.set_push_gripping(true, physics);
            self.steer = Some(
                ((input.held & keys::LEFT != 0) as u8 as f32)
                    - ((input.held & keys::RIGHT != 0) as u8 as f32),
            );
            let forward = ((input.held & keys::FORWARD != 0) as u8 as f32)
                - ((input.held & keys::BACK != 0) as u8 as f32);
            let angle = physics
                .vehicle_of(body_handle)
                .and_then(crate::vehicle::garage_vehicle_index)
                .map_or(0.0, |vehicle| self.steering_angles[vehicle]);
            let direction = Rotation::from_rotation_y(self.steer.unwrap() * angle)
                * (car_pose.rotation * -Vector::Z)
                * forward;
            let up = car_pose.rotation * Vector::Y;
            let projected = (direction - up * direction.dot(up)).normalize_or_zero();
            physics.edit_body(body_handle, |body| {
                body.apply_impulse_at_point(
                    projected
                        * self.player.grip_speed
                        * self.player.grip_force
                        * self.player.source_force_interval,
                    position,
                    true,
                )
            });
            return Some(target.item);
        }
        if self.grip.is_some() {
            self.stop(player, physics);
            return None;
        }
        let view = to_rapier(player.view(physics).camera_pose);
        let forward = view.rotation * -Vector::Z;
        let mut direction = forward;
        let projected = Vector::new(direction.x, 0.0, direction.z);
        if (rule.both && projected.angle_between(direction) < 45.0_f32.to_radians())
            || (!rule.both && rule.only_projected)
        {
            direction = projected;
        }
        let mass = physics.rapier().bodies[body_handle].mass();
        physics.edit_body(body_handle, |body| {
            body.apply_impulse_at_point(
                direction
                    * mass
                    * self.player.push_force
                    * dt
                    * rule.modifier
                    * self.player.source_force_interval,
                Vector::from_array(target.point),
                true,
            )
        });
        if !player.grounded() {
            player.apply_recoil(
                -forward * self.player.airborne_push_force * self.player.source_force_interval,
            );
        }
        Some(target.item)
    }
}
