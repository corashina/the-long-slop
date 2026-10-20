pub mod collision;
pub mod pose;
mod posture;
use collision::PlayerSweepShape;
use posture::{MovementSpeed, PostureState};
use rapier3d::parry::shape::Shape;

use rapier3d::control::{CharacterCollision, KinematicCharacterController};
use rapier3d::math::{Pose, Rotation, Vector};
use rapier3d::prelude::{
    ColliderBuilder, ColliderHandle, Group, InteractionGroups, InteractionTestMode, QueryFilter,
    Ray, RigidBodyBuilder, RigidBodyHandle, SharedShape,
};

use crate::reference::{PlayerReference, SeatId, SeatReference};
use crate::simulation::{FrameInput, keys};
use crate::vehicle::{PhysicsWorld, VehiclePose};

#[derive(Clone, Copy, Debug)]
pub struct PlayerStepContext {
    pub wheel_posture_allowed: bool,
    pub mouse_consumed: bool,
    pub picked_up: bool,
    pub recenter_consumed: bool,
    pub movement_multiplier: f32,
}
impl Default for PlayerStepContext {
    fn default() -> Self {
        Self {
            wheel_posture_allowed: false,
            mouse_consumed: false,
            picked_up: false,
            recenter_consumed: false,
            movement_multiplier: 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerMode {
    Walking,
    Seated,
}

#[derive(Clone, Copy, Debug)]
pub struct ExitReference {
    pub distance: f32,
    pub minimum_look_angle_degrees: f32,
    pub horizontal_step: f32,
    pub vertical_step: f32,
    pub horizontal_steps: u32,
    pub vertical_steps: u32,
}

impl ExitReference {
    pub fn from_reference(player: &PlayerReference, seat: &SeatReference) -> Self {
        Self {
            distance: seat.exit_distance,
            minimum_look_angle_degrees: seat.min_exit_angle_degrees,
            horizontal_step: player.exit_step_horizontal,
            vertical_step: player.exit_step_vertical,
            horizontal_steps: player.exit_steps_horizontal,
            vertical_steps: player.exit_steps_vertical,
        }
    }
}

#[derive(Clone, Copy)]
struct Occupancy {
    id: SeatId,
    local_pose: Pose,
    yaw_limits: [f32; 2],
    vehicle: usize,
    driver: bool,
    driving2: bool,
}

pub struct PlayerState {
    reference: PlayerReference,
    controller: KinematicCharacterController,
    sweep: PlayerSweepShape,
    posture: PostureState,
    movement_speed: MovementSpeed,
    exit_capsule: SharedShape,
    body: RigidBodyHandle,
    collider: ColliderHandle,
    feet: ColliderHandle,
    clearance_shapes: [rapier3d::parry::shape::Ball; 10],
    pose: Pose,
    seats: Vec<SeatReference>,
    occupancy: Option<Occupancy>,
    collisions: Vec<CharacterCollision>,
    yaw: f32,
    pitch: f32,
    vertical_velocity: f32,
    grounded: bool,
    support_body: Option<RigidBodyHandle>,
    recoil_velocity: Vector,
    push_gripping: bool,
    running: bool,
}

impl PlayerState {
    pub(crate) fn restore_save(
        &mut self,
        saved: &crate::save::PlayerSnapshot,
        ids: &crate::save::IdentityMap,
        origin: &crate::world::WorldOrigin,
        physics: &mut PhysicsWorld,
    ) -> Result<(), crate::save::SaveError> {
        self.pose = Pose::from_parts(
            Vector::from_array(origin.local(saved.pose.position)),
            Rotation::from_array(saved.pose.rotation),
        );
        self.yaw = saved.yaw;
        self.pitch = saved.pitch;
        self.vertical_velocity = saved.vertical_velocity;
        self.recoil_velocity = Vector::from_array(saved.recoil_velocity);
        self.posture.restore(saved.posture);
        self.movement_speed = MovementSpeed::default();
        self.running = saved.running;
        self.occupancy = if let Some(record) = &saved.seat {
            let id = ids.seats[&record.source];
            let seat = self.seats.iter().find(|s| s.id == id).unwrap();
            Some(Occupancy {
                id,
                vehicle: ids
                    .vehicles
                    .iter()
                    .position(|v| v == &record.vehicle)
                    .ok_or_else(|| crate::save::SaveError::InvalidLinks("Seat vehicle".into()))?,
                local_pose: to_rapier(crate::save::local_pose(&record.local_pose)),
                yaw_limits: seat.yaw_limits_degrees,
                driver: seat.driver,
                driving2: seat.driving2,
            })
        } else {
            None
        };
        if let Some(seat) = self.occupancy {
            self.pose = *physics.rapier().bodies[physics.chassis_handle(seat.vehicle)].position()
                * seat.local_pose;
        }
        self.push_gripping = false;
        self.collisions.clear();
        self.grounded = false;
        self.support_body = None;
        physics.edit_body(self.body, |body| {
            body.set_position(self.pose, false);
            body.set_next_kinematic_position(self.pose);
        });
        self.update_geometry(physics);
        if self.occupancy.is_none() {
            let hit = physics.queries(self.movement_filter()).cast_ray(
                &Ray::new(self.pose.translation, -Vector::Y),
                -self.sweep.compute_local_aabb().mins.y + 0.1,
                true,
            );
            self.grounded = hit.is_some() && self.vertical_velocity <= 0.0;
            if self.grounded {
                self.support_body = hit.and_then(|(h, _)| physics.rapier().colliders[h].parent());
            }
        }
        Ok(())
    }
    /// Poses place the source body root. World up is +Y. View forward is -Z.
    pub fn new(reference: &PlayerReference, pose: VehiclePose, physics: &mut PhysicsWorld) -> Self {
        let sweep = PlayerSweepShape::new(reference);
        let exit_capsule = SharedShape::capsule_y(
            reference.exit_capsule_half_height,
            reference.exit_capsule_radius,
        );
        let pose = to_rapier(pose);
        let (body, collider) = physics.insert(
            RigidBodyBuilder::kinematic_position_based().pose(pose),
            ColliderBuilder::new(SharedShape::new(*sweep.capsule()))
                .position(sweep.capsule_pose())
                .density(0.0)
                .collision_groups(groups(
                    reference.posture.capsule_membership,
                    reference.posture.capsule_filter,
                )),
        );
        physics.edit_body(body, |body| {
            body.set_additional_mass(reference.mass_kg, false)
        });
        let feet = physics.insert_collider_with_parent(
            ColliderBuilder::new(SharedShape::new(sweep.feet()))
                .position(sweep.feet_pose())
                .density(0.0)
                .collision_groups(groups(
                    reference.posture.feet.membership,
                    reference.posture.feet.filter,
                )),
            body,
        );
        physics.recompute_body_mass(body);
        physics.refresh_queries();
        Self {
            reference: reference.clone(),
            controller: KinematicCharacterController::default(),
            sweep,
            posture: PostureState::new(1.0),
            movement_speed: MovementSpeed::default(),
            exit_capsule,
            body,
            collider,
            feet,
            clearance_shapes: std::array::from_fn(|i| {
                rapier3d::parry::shape::Ball::new(reference.posture.clearance[i].sphere.radius)
            }),
            pose,
            seats: Vec::new(),
            occupancy: None,
            collisions: Vec::with_capacity(20),
            yaw: yaw_from_rotation(pose.rotation),
            pitch: 0.0,
            vertical_velocity: 0.0,
            grounded: false,
            support_body: None,
            recoil_velocity: Vector::ZERO,
            push_gripping: false,
            running: false,
        }
    }

    pub fn capture_save(
        &self,
        ids: &crate::save::IdentityMap,
        origin: &crate::world::WorldOrigin,
        physics: &PhysicsWorld,
        carry: Option<crate::save::CarrySnapshot>,
    ) -> Result<crate::save::PlayerSnapshot, crate::save::SaveError> {
        let (position, rotation) = if let Some(seat) = self.occupancy {
            let chassis = physics.rapier().bodies[physics.chassis_handle(seat.vehicle)].position();
            let absolute = origin.absolute(chassis.translation.to_array());
            let offset = chassis.rotation * seat.local_pose.translation;
            (
                std::array::from_fn(|axis| absolute[axis] + f64::from(offset[axis])),
                (chassis.rotation * seat.local_pose.rotation).to_array(),
            )
        } else {
            let pose = self.body_pose(physics);
            (origin.absolute(pose.position), pose.rotation)
        };
        Ok(crate::save::PlayerSnapshot {
            pose: crate::save::AbsolutePose { position, rotation },
            yaw: self.yaw,
            pitch: self.pitch,
            seat: self
                .occupancy
                .map(|seat| {
                    Ok::<_, crate::save::SaveError>(crate::save::SeatSnapshot {
                        vehicle: ids.vehicles[seat.vehicle].clone(),
                        source: crate::save::identity::source_key(&ids.seats, seat.id)?,
                        local_pose: crate::save::LocalPose {
                            position: seat.local_pose.translation.to_array(),
                            rotation: seat.local_pose.rotation.to_array(),
                        },
                    })
                })
                .transpose()?,
            posture: self.posture.value(),
            vertical_velocity: self.vertical_velocity,
            recoil_velocity: self.recoil_velocity.to_array(),
            running: self.running,
            carry,
        })
    }
    /// Bind seats once at setup. Driver identity comes from the recovered seat record.
    pub fn configure_seats(&mut self, seats: &[SeatReference]) {
        assert!(
            self.occupancy.is_none(),
            "cannot replace occupied seat records"
        );
        self.seats.clear();
        self.seats.extend_from_slice(seats);
    }

    /// Mouse deltas contain degrees. The browser applies mouse sensitivity.
    pub fn step(
        &mut self,
        input: FrameInput,
        context: PlayerStepContext,
        physics: &mut PhysicsWorld,
        dt: f32,
    ) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        if !context.mouse_consumed && input.mouse[0].is_finite() {
            self.yaw += input.mouse[0];
        }
        if !context.mouse_consumed && input.mouse[1].is_finite() {
            self.pitch += input.mouse[1];
        }
        self.pitch = self.pitch.clamp(-89.0, 89.0);
        if let Some(seat) = self.occupancy {
            self.yaw = self.yaw.clamp(seat.yaw_limits[0], seat.yaw_limits[1]);
            self.pose = *physics.rapier().bodies[physics.chassis_handle(seat.vehicle)].position()
                * seat.local_pose;
            physics.edit_body(self.body, |body| {
                body.set_next_kinematic_position(self.pose)
            });
            return;
        }
        self.yaw = (self.yaw + 180.0).rem_euclid(360.0) - 180.0;
        self.pose.rotation = Rotation::from_rotation_y(-self.yaw.to_radians());
        if self.push_gripping {
            self.pose.translation = physics.rapier().bodies[self.body].position().translation;
        }
        let was_grounded = self.grounded;
        let vertical_before_jump = self.vertical_velocity;
        let jump_force = self.update_posture(input, context, physics, dt);
        if self.push_gripping {
            let filter = self.movement_filter();
            let ground = physics.queries(filter).cast_ray(
                &Ray::new(self.pose.translation, -Vector::Y),
                -self.sweep.compute_local_aabb().mins.y + 0.1,
                true,
            );
            self.grounded =
                ground.is_some() && physics.rapier().bodies[self.body].linvel().y <= 0.1;
            self.support_body =
                ground.and_then(|(collider, _)| physics.rapier().colliders[collider].parent());
            if jump_force != 0.0 {
                physics.edit_body(self.body, |body| {
                    body.apply_impulse(
                        Vector::Y
                            * self.reference.jump_force
                            * self.reference.source_force_interval
                            * jump_force,
                        true,
                    )
                });
                self.grounded = false;
            }
            return;
        }
        if self.grounded {
            self.running = input.held & keys::RUN != 0;
        }
        if jump_force != 0.0 {
            self.vertical_velocity += jump_force * self.reference.jump_force
                / self.reference.mass_kg
                * self.reference.source_force_interval;
            if jump_force > 0.0 {
                self.grounded = false;
            }
        }
        self.vertical_velocity = (self.vertical_velocity + physics.rapier().gravity.y * dt)
            .max(-self.reference.max_fall_speed);
        let x = ((input.held & keys::RIGHT != 0) as u8 as f32)
            - ((input.held & keys::LEFT != 0) as u8 as f32);
        let z = ((input.held & keys::BACK != 0) as u8 as f32)
            - ((input.held & keys::FORWARD != 0) as u8 as f32);
        let direction = Vector::new(x, 0.0, z).normalize_or_zero();
        let requested_speed = self.reference.walk_speed
            * if self.running {
                self.reference.run_multiplier
            } else {
                1.0
            }
            * posture::speed_multiplier(
                self.posture.value(),
                self.reference.posture.ctrl_value,
                self.reference.prone_multiplier,
                self.reference.duck_multiplier,
            )
            * context.movement_multiplier;
        let speed = self.movement_speed.limit(was_grounded, requested_speed);
        let desired = self.pose.rotation * direction * speed * dt
            + self.recoil_velocity * dt
            + Vector::Y * self.vertical_velocity * dt;
        let mut support_body = None;
        self.collisions.clear();
        let shape = &self.sweep;
        let filter = self.movement_filter();
        let rapier = physics.rapier();
        let movement = self.controller.move_shape(
            dt,
            &physics.queries(filter),
            shape,
            &self.pose,
            desired,
            |collision| {
                if collision.hit.normal1.y > 0.5 {
                    support_body = rapier.colliders[collision.handle].parent();
                }
                let dynamic = rapier.colliders[collision.handle]
                    .parent()
                    .is_some_and(|body| rapier.bodies[body].is_dynamic());
                // The hit normal points out of the obstacle. Approaching movement has a negative dot.
                if dynamic && collision.translation_remaining.dot(collision.hit.normal1) < -1.0e-5 {
                    self.collisions.push(collision);
                }
            },
        );
        if !self.collisions.is_empty() {
            physics.solve_character_collision_impulses(
                &self.controller,
                dt,
                filter,
                shape,
                self.reference.mass_kg,
                &self.collisions,
            );
        }
        if was_grounded {
            self.movement_speed.grounded_velocity(
                Vector::new(
                    movement.translation.x / dt,
                    vertical_before_jump,
                    movement.translation.z / dt,
                )
                .length(),
            );
        }
        self.pose.translation += movement.translation;
        self.grounded = movement.grounded && self.vertical_velocity <= 0.0;
        self.support_body = if self.grounded { support_body } else { None };
        if self.grounded {
            self.recoil_velocity = Vector::ZERO;
        }
        if self.grounded
            || (self.vertical_velocity > 0.0 && movement.translation.y < desired.y - 1.0e-5)
        {
            self.vertical_velocity = 0.0;
        }
        physics.edit_body(self.body, |body| {
            body.set_next_kinematic_position(self.pose)
        });
    }

    pub(crate) fn collision_groups(&self) -> InteractionGroups {
        groups(
            self.reference.posture.capsule_membership,
            self.reference.posture.capsule_filter,
        )
    }
    fn movement_filter(&self) -> QueryFilter<'static> {
        QueryFilter::default()
            .exclude_sensors()
            .exclude_rigid_body(self.body)
            .groups(self.collision_groups())
    }
    fn update_posture(
        &mut self,
        input: FrameInput,
        context: PlayerStepContext,
        physics: &mut PhysicsWorld,
        dt: f32,
    ) -> f32 {
        let hits = std::array::from_fn(|index| {
            let sensor = &self.reference.posture.clearance[index].sphere;
            let pose = self.pose * Pose::from_translation(Vector::from_array(sensor.center));
            physics
                .queries(
                    QueryFilter::default()
                        .exclude_rigid_body(self.body)
                        .groups(groups(sensor.membership, sensor.filter)),
                )
                .intersect_shape(pose, &self.clearance_shapes[index])
                .next()
                .is_some()
        });
        self.posture
            .contacts(hits, self.reference.posture.sensor_hold_steps);
        let before = self.posture.value();
        let mut posture_input = input;
        posture_input.wheel *= self.reference.scroll_sensitivity;
        let force = self.posture.step(
            posture_input,
            context.wheel_posture_allowed,
            self.grounded,
            &self.reference.posture,
            dt,
            self.reference.duck_jump_multiplier,
            self.reference.inverse_duck_multiplier,
        );
        if self.posture.value() != before {
            self.update_geometry(physics);
        }
        force
    }
    fn update_geometry(&mut self, physics: &mut PhysicsWorld) {
        self.sweep
            .set_posture(&self.reference, self.posture.value());
        physics.edit_collider(self.collider, |collider| {
            *collider
                .shape_mut()
                .as_capsule_mut()
                .expect("Owned player capsule") = *self.sweep.capsule();
            collider.set_position_wrt_parent(self.sweep.capsule_pose());
            crate::vehicle::set_collider_enabled(collider, self.occupancy.is_none());
        });
        physics.edit_collider(self.feet, |collider| {
            *collider
                .shape_mut()
                .as_ball_mut()
                .expect("Owned player foot sphere") = self.sweep.feet();
            crate::vehicle::set_collider_enabled(
                collider,
                self.occupancy.is_none() && self.sweep.feet_enabled(),
            );
        });
        physics.refresh_queries();
    }
    /// The seat pose places the source sitPos root relative to the chassis.
    pub fn enter(&mut self, id: SeatId, pose: VehiclePose, physics: &mut PhysicsWorld) -> bool {
        if self.occupancy.is_some() {
            return false;
        }
        let Some(seat) = self.seats.iter().find(|seat| seat.id == id) else {
            return false;
        };
        let local_pose = to_rapier(pose);
        let chassis = physics.chassis_handle(seat.vehicle);
        let seated_pose = *physics.rapier().bodies[chassis].position() * local_pose;
        self.yaw =
            (self.yaw - yaw_from_rotation(seated_pose.rotation) + 180.0).rem_euclid(360.0) - 180.0;
        self.yaw = self
            .yaw
            .clamp(seat.yaw_limits_degrees[0], seat.yaw_limits_degrees[1]);
        self.occupancy = Some(Occupancy {
            id,
            local_pose,
            yaw_limits: seat.yaw_limits_degrees,
            vehicle: seat.vehicle,
            driver: seat.driver,
            driving2: seat.driving2,
        });
        self.pose = seated_pose;
        self.posture.restore(1.0);
        self.grounded = false;
        self.running = false;
        self.vertical_velocity = 0.0;
        self.update_geometry(physics);
        physics.edit_body(self.body, |body| body.set_position(seated_pose, false));
        physics.refresh_queries();
        true
    }

    fn exit_pose(&self, physics: &PhysicsWorld, limits: &ExitReference) -> Option<Pose> {
        if self.occupancy.is_none() {
            return None;
        }
        let view = to_rapier(self.view_pose(physics));
        let body = to_rapier(self.body_pose(physics));
        let forward = view.rotation * -Vector::Z;
        let seat_forward = body.rotation * -Vector::Z;
        let angle = forward
            .dot(seat_forward)
            .clamp(-1.0, 1.0)
            .acos()
            .to_degrees();
        if angle < limits.minimum_look_angle_degrees {
            return None;
        }
        // Unity rays ignore colliders containing the ray origin.
        let use_mask = self.reference.action_masks.use_mask;
        let ray_targets = |_: ColliderHandle, collider: &rapier3d::prelude::Collider| {
            collider.is_enabled()
                && collider.collision_groups().memberships.bits() & use_mask != 0
                && !collider
                    .shape()
                    .contains_point(collider.position(), view.translation)
        };
        let queries = physics.queries(
            QueryFilter::default()
                .exclude_sensors()
                .exclude_rigid_body(self.body)
                .predicate(&ray_targets),
        );
        let max_distance = queries
            .cast_ray_and_get_normal(&Ray::new(view.translation, forward), limits.distance, true)
            .map_or(limits.distance, |(_, hit)| {
                let displacement = forward * hit.time_of_impact
                    + hit.normal * (self.reference.posture.maximum_radius + 0.05);
                Vector::new(displacement.x, 0.0, displacement.z)
                    .length()
                    .min(limits.distance)
            });
        let exit_mask = self.reference.get_out_layer_mask;
        let exit_targets = |_: ColliderHandle, collider: &rapier3d::prelude::Collider| {
            collider.is_enabled() && collider.collision_groups().memberships.bits() & exit_mask != 0
        };
        let exit_queries = physics.queries(
            QueryFilter::default()
                .exclude_sensors()
                .exclude_rigid_body(self.body)
                .predicate(&exit_targets),
        );
        let mut vertical = 0.0;
        for horizontal_index in 0..limits.horizontal_steps {
            let distance = horizontal_index as f32 * limits.horizontal_step;
            if distance >= max_distance {
                break;
            }
            for _ in 0..limits.vertical_steps {
                let head = view.translation + forward * distance + Vector::Y * vertical;
                if exit_queries
                    .intersect_shape(Pose::from_translation(head), self.exit_capsule.as_ref())
                    .next()
                    .is_none()
                {
                    // Source GetOut places the body origin at the checked head point.
                    let mut candidate = Pose::from_translation(head);
                    candidate.rotation =
                        Rotation::from_rotation_y(-yaw_from_rotation(view.rotation).to_radians());
                    return Some(candidate);
                }
                vertical += limits.vertical_step;
            }
        }
        None
    }
    pub fn can_exit(&self, physics: &PhysicsWorld, limits: &ExitReference) -> bool {
        self.exit_pose(physics, limits).is_some()
    }
    pub fn try_exit(&mut self, physics: &mut PhysicsWorld, limits: &ExitReference) -> bool {
        let Some(candidate) = self.exit_pose(physics, limits) else {
            return false;
        };
        let chassis = physics.chassis_handle(self.occupancy.unwrap().vehicle);
        self.pose = candidate;
        self.yaw = yaw_from_rotation(candidate.rotation);
        let inherited = physics.rapier().bodies[chassis].linvel() * 0.3;
        self.vertical_velocity = inherited.y;
        self.recoil_velocity = Vector::new(inherited.x, 0.0, inherited.z);
        self.occupancy = None;
        self.posture.restore(1.0);
        self.grounded = false;
        self.running = false;

        self.update_geometry(physics);
        physics.edit_body(self.body, |body| body.set_position(candidate, false));
        physics.refresh_queries();
        true
    }
    pub fn seat(&self) -> Option<SeatId> {
        self.occupancy.map(|seat| seat.id)
    }
    /// Returns the vehicle that owns the occupied seat.
    pub fn seat_vehicle(&self) -> Option<usize> {
        self.occupancy.map(|seat| seat.vehicle)
    }
    pub fn mode(&self) -> PlayerMode {
        if self.occupancy.is_some() {
            PlayerMode::Seated
        } else {
            PlayerMode::Walking
        }
    }
    pub fn driving(&self) -> bool {
        self.occupancy.is_some_and(|seat| seat.driver)
    }
    pub fn driving2(&self) -> bool {
        self.occupancy.is_some_and(|seat| seat.driving2)
    }
    pub fn posture(&self) -> f32 {
        self.posture.value()
    }
    pub fn support_body(&self) -> Option<RigidBodyHandle> {
        self.support_body
    }
    pub fn apply_recoil(&mut self, impulse: Vector) {
        self.recoil_velocity += impulse / self.reference.mass_kg;
    }
    pub fn set_push_gripping(&mut self, active: bool, physics: &mut PhysicsWorld) {
        if self.push_gripping == active {
            return;
        }
        physics.edit_body(self.body, |body| {
            if active {
                body.set_body_type(rapier3d::prelude::RigidBodyType::Dynamic, true);
                body.set_enabled_rotations(false, false, false, true);
            } else {
                self.pose.translation = body.position().translation;
                let velocity = body.linvel();
                self.recoil_velocity = Vector::new(velocity.x, 0.0, velocity.z);
                self.vertical_velocity = velocity.y;
                body.set_body_type(
                    rapier3d::prelude::RigidBodyType::KinematicPositionBased,
                    true,
                );
                body.set_enabled_rotations(true, true, true, true);
                body.set_next_kinematic_position(self.pose);
            }
        });
        self.push_gripping = active;
    }
    pub fn follow_push_grip(&mut self, turn_degrees: f32) {
        self.yaw -= turn_degrees;
        self.pose.rotation = Rotation::from_rotation_y(-self.yaw.to_radians());
    }
    pub fn grounded(&self) -> bool {
        self.grounded
    }
    pub fn running(&self) -> bool {
        self.running
    }
    pub fn yaw_degrees(&self) -> f32 {
        self.yaw
    }
    pub fn pitch_degrees(&self) -> f32 {
        self.pitch
    }
    pub fn sweep_shape(&self) -> &PlayerSweepShape {
        &self.sweep
    }
    pub fn body_handle(&self) -> RigidBodyHandle {
        self.body
    }
    pub fn body_pose(&self, physics: &PhysicsWorld) -> VehiclePose {
        let mut walking_pose = self.pose;
        if self.push_gripping {
            walking_pose.translation = physics.rapier().bodies[self.body].position().translation;
        }
        let pose = self.occupancy.map_or(walking_pose, |seat| {
            *physics.rapier().bodies[physics.chassis_handle(seat.vehicle)].position()
                * seat.local_pose
        });
        from_rapier(pose)
    }
    pub fn head_pose_at(&self, body_pose: VehiclePose) -> VehiclePose {
        let mut pose = to_rapier(body_pose);
        let lower = Vector::from_array(self.reference.posture.prone_head.translation);
        let head = lower.lerp(
            Vector::from_array(self.reference.posture.standing_head.translation),
            self.posture.value(),
        );
        pose.translation += pose.rotation * head;
        if self.occupancy.is_some() {
            pose.rotation *= Rotation::from_rotation_y(-self.yaw.to_radians());
        }
        pose.rotation *= Rotation::from_rotation_x(-self.pitch.to_radians());
        from_rapier(pose)
    }
    pub fn view_pose(&self, physics: &PhysicsWorld) -> VehiclePose {
        self.head_pose_at(self.body_pose(physics))
    }
    pub(crate) fn recover(&mut self, pose: VehiclePose, physics: &mut PhysicsWorld) {
        self.pose = to_rapier(pose);
        self.yaw = yaw_from_rotation(self.pose.rotation);
        self.posture.restore(self.posture.value());
        self.movement_speed = MovementSpeed::default();
        self.vertical_velocity = 0.0;
        self.recoil_velocity = Vector::ZERO;
        self.grounded = false;
        self.support_body = None;
        self.running = false;
        physics.edit_body(self.body, |body| {
            body.set_position(self.pose, false);
            body.set_next_kinematic_position(self.pose);
        });
        self.update_geometry(physics);
    }
    pub fn focus_position(&self, physics: &PhysicsWorld) -> [f32; 3] {
        self.view_pose(physics).position
    }
    pub(crate) fn align_startup(&mut self, offset: f32, physics: &mut PhysicsWorld) {
        self.pose.translation.y += offset;
        physics.edit_body(self.body, |body| body.set_position(self.pose, false));
    }
    pub fn rebase(&mut self, shift: [f32; 3]) {
        self.pose.translation -= Vector::from_array(shift);
    }
}

fn yaw_from_rotation(rotation: Rotation) -> f32 {
    let forward = rotation * -Vector::Z;
    forward.x.atan2(-forward.z).to_degrees()
}
fn to_rapier(pose: VehiclePose) -> Pose {
    Pose::from_parts(
        Vector::from_array(pose.position),
        Rotation::from_array(pose.rotation),
    )
}
fn from_rapier(pose: Pose) -> VehiclePose {
    VehiclePose {
        position: pose.translation.to_array(),
        rotation: pose.rotation.to_array(),
    }
}

fn groups(membership: u32, filter: u32) -> InteractionGroups {
    InteractionGroups::new(
        Group::from_bits_retain(membership),
        Group::from_bits_retain(filter),
        InteractionTestMode::And,
    )
}
