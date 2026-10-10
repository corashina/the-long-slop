//! Source carry pose and reusable collision constraint.
use super::items::{ItemLocation, ItemStore};
use crate::player::PlayerState;
use crate::reference::{
    ItemId, PickupReference, PlayerReference, ReferenceError, ReferenceManifest,
};
use crate::simulation::{FrameInput, keys};
use crate::vehicle::{PhysicsWorld, to_rapier};
use rapier3d::math::{Pose, Rotation, Vector};
use rapier3d::parry::query::ShapeCastOptions;
use rapier3d::prelude::{
    ColliderHandle, FixedJointBuilder, ImpulseJointHandle, QueryFilter, RigidBodyBuilder,
    RigidBodyHandle,
};
use std::collections::HashMap;

struct HeldState {
    item: ItemId,
    body: RigidBodyHandle,
    offset: Vector,
    rotation: Rotation,
    joint: ImpulseJointHandle,
    local_pose: Pose,
}

pub struct CarryState {
    reference: PlayerReference,
    rules: HashMap<ItemId, PickupReference>,
    held: Option<HeldState>,
    anchor: Option<RigidBodyHandle>,
    acquisition_held: bool,
}

impl CarryState {
    pub fn new(reference: &ReferenceManifest) -> Result<Self, ReferenceError> {
        let player = &reference.player;
        if !player.carry_min_weight.is_finite()
            || !player.carry_max_weight.is_finite()
            || player.carry_min_weight >= player.carry_max_weight
        {
            return Err(ReferenceError("Invalid carry weight range".into()));
        }
        Ok(Self {
            reference: player.clone(),
            rules: reference
                .items
                .iter()
                .filter_map(|item| item.pickup.clone().map(|rule| (item.id, rule)))
                .collect(),
            held: None,
            anchor: None,
            acquisition_held: false,
        })
    }
    pub fn held(&self) -> Option<ItemId> {
        self.held.as_ref().map(|held| held.item)
    }
    pub fn body(&self) -> Option<RigidBodyHandle> {
        self.held.as_ref().map(|held| held.body)
    }
    pub fn acquisition_held(&self) -> bool {
        self.acquisition_held
    }
    pub fn release_acquisition(&mut self, input: FrameInput) {
        if input.held & keys::GRAB == 0 || input.released & keys::GRAB != 0 {
            self.acquisition_held = false;
        }
    }
    pub fn allows_pickup(&self, item: ItemId, seated: bool, mounted: bool) -> bool {
        self.rules.get(&item).is_some_and(|rule| {
            !rule.cant_pickup
                && !(seated && rule.cant_pickup_while_seated)
                && !(mounted && rule.cant_pickup_while_mounted)
        })
    }
    pub fn drop(&mut self, items: &mut ItemStore, physics: &mut PhysicsWorld) -> Option<ItemId> {
        let held = self.held.take()?;
        physics.rapier_mut().impulse_joints.remove(held.joint, true);
        if let Some(item) = items.get_mut(held.item) {
            item.location = ItemLocation::World;
        }
        self.acquisition_held = false;
        Some(held.item)
    }
    pub fn can_acquire(
        &self,
        id: ItemId,
        point: [f32; 3],
        player: &PlayerState,
        body: RigidBodyHandle,
        physics: &PhysicsWorld,
    ) -> Result<(), ReferenceError> {
        if self.held.is_some()
            || !point.iter().all(|v| v.is_finite())
            || !self.allows_pickup(id, player.seat().is_some(), false)
            || body == physics.chassis_handle()
            || physics
                .rapier()
                .bodies
                .get(body)
                .is_none_or(|body| !body.is_dynamic())
        {
            return Err(ReferenceError("Cannot carry this item".into()));
        }
        Ok(())
    }
    pub fn pickup(
        &mut self,
        id: ItemId,
        point: [f32; 3],
        player: &PlayerState,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
    ) -> Result<(), ReferenceError> {
        let item = items
            .get(id)
            .ok_or_else(|| ReferenceError("Cannot carry this item".into()))?;
        if item.location != ItemLocation::World {
            return Err(ReferenceError("Cannot carry this item".into()));
        }
        let body = item
            .body
            .ok_or_else(|| ReferenceError("Cannot carry this item".into()))?;
        self.can_acquire(id, point, player, body, physics)?;
        let item = items.get_mut(id).unwrap();
        let pose = *physics.rapier().bodies[body].position();
        let anchor = *self.anchor.get_or_insert_with(|| {
            physics
                .rapier_mut()
                .insert_body(RigidBodyBuilder::kinematic_position_based().pose(pose))
        });
        physics.rapier_mut().bodies[anchor].set_position(pose, false);
        let joint = physics.rapier_mut().impulse_joints.insert(
            anchor,
            body,
            FixedJointBuilder::new(),
            true,
        );
        let view = to_rapier(player.view_pose(physics));
        let local_pose = to_rapier(item.local_pose);
        let item_pose = pose * local_pose;
        physics.rapier_mut().bodies[body].enable_ccd(true);
        self.held = Some(HeldState {
            item: id,
            body,
            offset: (view.rotation.inverse() * (item_pose.translation - view.translation)
                - Vector::from_array(self.reference.carry_anchor_offset))
            .clamp_length_max(self.reference.carry_offset_radius),
            rotation: view.rotation.inverse() * item_pose.rotation,
            joint,
            local_pose,
        });
        item.location = ItemLocation::Held;
        self.acquisition_held = true;
        Ok(())
    }

    pub fn step(
        &mut self,
        input: FrameInput,
        player: &PlayerState,
        physics: &mut PhysicsWorld,
        dt: f32,
    ) -> bool {
        if !dt.is_finite() || dt <= 0.0 {
            return false;
        }
        self.release_acquisition(input);
        let Some(held) = self.held.as_mut() else {
            return false;
        };
        let Some(anchor) = self.anchor else {
            return false;
        };
        let view = to_rapier(player.view_pose(physics));
        let rule = &self.rules[&held.item];
        let mass = physics.rapier().bodies[held.body].mass();
        let movement_gain = ((mass - self.reference.carry_max_weight)
            / (self.reference.carry_min_weight - self.reference.carry_max_weight))
            .clamp(0.0, 1.0);
        let rotation_gain = ((mass - self.reference.carry_max_weight * 2.0)
            / (self.reference.carry_min_weight - self.reference.carry_max_weight * 2.0))
            .clamp(0.0, 1.0);
        if (input.held | input.pressed) & keys::RECENTER != 0 {
            held.offset = if rule.use_recenter_position {
                Vector::from_array(rule.recenter_position)
            } else {
                Vector::from_array(self.reference.carry_anchor_offset) * rule.recenter_distance
            };
            held.rotation = if rule.use_recenter_rotation {
                Rotation::from_array(rule.recenter_rotation)
            } else {
                Rotation::IDENTITY
            };
        } else {
            let rotating = input.held & keys::ROTATE != 0;
            let moving = input.held & keys::TRANSLATE != 0;
            if input.wheel.is_finite() && !rotating {
                let direction = if rule.move_with_rotation {
                    (Vector::from_array(self.reference.carry_anchor_offset) + held.offset)
                        .normalize_or_zero()
                } else {
                    -Vector::Z
                };
                held.offset += direction
                    * input.wheel
                    * self.reference.scroll_sensitivity
                    * dt
                    * self.reference.obj_move_sensitivity;
            }
            if input.mouse.iter().all(|delta| delta.is_finite()) {
                if rotating && !rule.no_rotation {
                    let sensitivity =
                        dt * 10.0 * self.reference.obj_rotation_sensitivity * rotation_gain;
                    let x = input.mouse[0] * sensitivity;
                    let y = input.mouse[1] * sensitivity;
                    let rotation = if moving {
                        if rule.move_with_rotation {
                            held.rotation * Rotation::from_rotation_z(-x.to_radians())
                        } else {
                            Rotation::from_rotation_z(-x.to_radians()) * held.rotation
                        }
                    } else {
                        Rotation::from_rotation_y(x.to_radians()) * held.rotation
                    };
                    held.rotation = if !rule.move_with_rotation || !moving {
                        Rotation::from_rotation_x(y.to_radians()) * rotation
                    } else {
                        rotation
                    };
                } else if moving && !rotating {
                    if rule.move_with_rotation {
                        let sensitivity = dt * 20.0 * self.reference.obj_move_sensitivity;
                        let rotation =
                            Rotation::from_rotation_y((-input.mouse[0] * sensitivity).to_radians())
                                * Rotation::from_rotation_x(
                                    (-input.mouse[1] * sensitivity).to_radians(),
                                );
                        let position =
                            Vector::from_array(self.reference.carry_anchor_offset) + held.offset;
                        held.offset = rotation * position
                            - Vector::from_array(self.reference.carry_anchor_offset);
                    } else {
                        held.offset += Vector::new(input.mouse[0], -input.mouse[1], 0.0)
                            * dt
                            * 0.25
                            * self.reference.obj_move_sensitivity;
                    }
                }
            }
        }
        held.offset = held
            .offset
            .clamp_length_max(self.reference.carry_offset_radius);
        let current = *physics.rapier().bodies[held.body].position();
        let mut desired = Pose::from_parts(
            view.translation
                + view.rotation
                    * (Vector::from_array(self.reference.carry_anchor_offset) + held.offset)
                        .clamp_length(
                            self.reference.carry_min_distance,
                            self.reference.carry_max_distance,
                        ),
            view.rotation * held.rotation,
        );
        desired *= held.local_pose.inverse();
        if (desired.translation - current.translation).length()
            >= self.reference.carry_drop_distance
        {
            return true;
        }
        desired.translation = current.translation
            + ((desired.translation - current.translation) * self.reference.carry_force_multiplier)
                .clamp_length_max(self.reference.carry_max_force * movement_gain)
                * physics.rapier().integration_parameters.dt;
        let outside_player = |_: ColliderHandle, collider: &rapier3d::prelude::Collider| {
            collider.parent() != Some(player.body_handle())
        };
        let queries = physics.queries(
            QueryFilter::default()
                .exclude_sensors()
                .exclude_rigid_body(held.body)
                .predicate(&outside_player),
        );
        for collider_handle in physics.rapier().bodies[held.body].colliders() {
            let collider = &physics.rapier().colliders[*collider_handle];
            let local = collider
                .position_wrt_parent()
                .copied()
                .unwrap_or(Pose::IDENTITY);
            let velocity = desired.translation - current.translation;
            if let Some((_, hit)) = queries.cast_shape(
                &(current * local),
                velocity,
                collider.shape(),
                ShapeCastOptions {
                    max_time_of_impact: 1.0,
                    ..Default::default()
                },
            ) {
                desired.translation =
                    current.translation + velocity * (hit.time_of_impact - 0.01).max(0.0);
            }
            if queries
                .intersect_shape(desired * local, collider.shape())
                .next()
                .is_some()
            {
                desired.rotation = current.rotation;
            }
        }
        physics.rapier_mut().bodies[anchor].set_next_kinematic_position(desired);
        false
    }
}
