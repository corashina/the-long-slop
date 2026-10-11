//! Carrying, moving, rotating, and dropping a held item.

use super::*;

impl InteractionState {
    pub fn drop_held(
        &mut self,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
    ) -> Option<ItemId> {
        let held = self.held.take()?;
        physics.rapier_mut().impulse_joints.remove(held.joint, true);
        if let Some(item) = items.get_mut(held.item) {
            item.location = ItemLocation::World;
        }
        Some(held.item)
    }
    pub(super) fn update_carry(
        &mut self,
        input: FrameInput,
        player: &PlayerState,
        physics: &mut PhysicsWorld,
        dt: f32,
    ) -> bool {
        let Some(held) = self.held.as_mut() else {
            return false;
        };
        let Some(anchor) = self.carry_anchor else {
            return false;
        };
        let view = to_rapier(player.view_pose(physics));
        if input.wheel.is_finite() {
            held.offset = (held.offset
                - Vector::Z
                    * input.wheel
                    * self.carry.scroll_sensitivity
                    * dt
                    * self.carry.obj_move_sensitivity)
                .clamp_length_max(self.carry.carry_offset_radius);
        }
        if input.held & keys::MOVE != 0 && input.mouse.iter().all(|n| n.is_finite()) {
            held.offset += Vector::new(input.mouse[0], -input.mouse[1], 0.)
                * dt
                * self.carry.obj_move_sensitivity;
            held.offset = held.offset.clamp_length_max(self.carry.carry_offset_radius);
        }
        if input.held & keys::ROTATE != 0 && input.mouse.iter().all(|delta| delta.is_finite()) {
            let sensitivity = dt * 10.0 * self.carry.obj_rotation_sensitivity;
            held.rotation = Rotation::from_rotation_y((-input.mouse[0] * sensitivity).to_radians())
                * held.rotation
                * Rotation::from_rotation_x((input.mouse[1] * sensitivity).to_radians());
        }
        let current = *physics.rapier().bodies[held.body].position();
        let mut desired = Pose::from_parts(
            view.translation
                + view.rotation
                    * (Vector::from_array(self.carry.carry_anchor_offset) + held.offset)
                        .clamp_length(self.carry.carry_min_distance, self.carry.carry_max_distance),
            view.rotation * held.rotation,
        );
        if (desired.translation - current.translation).length() >= self.carry.carry_drop_distance {
            return true;
        }
        desired.translation = current.translation
            + ((desired.translation - current.translation) * self.carry.carry_force_multiplier)
                .clamp_length_max(self.carry.carry_max_force)
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
