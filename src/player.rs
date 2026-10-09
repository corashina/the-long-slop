use rapier3d::control::{CharacterCollision, KinematicCharacterController};
use rapier3d::math::{Pose, Rotation, Vector};
use rapier3d::prelude::{
    ColliderBuilder, ColliderHandle, QueryFilter, Ray, RigidBodyBuilder, RigidBodyHandle,
    SharedShape,
};

use crate::reference::{PlayerReference, SeatId, SeatReference};
use crate::simulation::{FrameInput, keys};
use crate::vehicle::{PhysicsWorld, VehiclePose};

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
    driver: bool,
}

pub struct PlayerState {
    reference: PlayerReference,
    controller: KinematicCharacterController,
    standing: SharedShape,
    crouching: SharedShape,
    body: RigidBodyHandle,
    collider: ColliderHandle,
    pose: Pose,
    seats: Vec<SeatReference>,
    occupancy: Option<Occupancy>,
    collisions: Vec<CharacterCollision>,
    yaw: f32,
    pitch: f32,
    vertical_velocity: f32,
    grounded: bool,
    crouched: bool,
    running: bool,
}

impl PlayerState {
    /// The pose places the capsule center. World up is +Y. View forward is -Z.
    pub fn new(reference: &PlayerReference, pose: VehiclePose, physics: &mut PhysicsWorld) -> Self {
        let standing = SharedShape::capsule_y(
            reference.standing_height * 0.5 - reference.standing_radius,
            reference.standing_radius,
        );
        let crouching = SharedShape::capsule_y(
            reference.crouch_height * 0.5 - reference.crouch_radius,
            reference.crouch_radius,
        );
        let pose = to_rapier(pose);
        let (body, collider) = physics.rapier_mut().insert(
            RigidBodyBuilder::kinematic_position_based().pose(pose),
            ColliderBuilder::new(standing.clone()).mass(reference.mass_kg),
        );
        physics.refresh_queries();
        Self {
            reference: reference.clone(),
            controller: KinematicCharacterController::default(),
            standing,
            crouching,
            body,
            collider,
            pose,
            seats: Vec::new(),
            occupancy: None,
            collisions: Vec::with_capacity(20),
            yaw: yaw_from_rotation(pose.rotation),
            pitch: 0.0,
            vertical_velocity: 0.0,
            grounded: false,
            crouched: false,
            running: false,
        }
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
    pub fn step(&mut self, input: FrameInput, physics: &mut PhysicsWorld, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        if input.mouse[0].is_finite() {
            self.yaw += input.mouse[0];
        }
        if input.mouse[1].is_finite() {
            self.pitch += input.mouse[1];
        }
        self.pitch = self.pitch.clamp(-89.0, 89.0);
        if let Some(seat) = self.occupancy {
            self.yaw = self.yaw.clamp(seat.yaw_limits[0], seat.yaw_limits[1]);
            self.pose =
                *physics.rapier().bodies[physics.chassis_handle()].position() * seat.local_pose;
            physics.rapier_mut().bodies[self.body].set_next_kinematic_position(self.pose);
            return;
        }
        self.yaw = (self.yaw + 180.0).rem_euclid(360.0) - 180.0;
        self.pose.rotation = Rotation::from_rotation_y(-self.yaw.to_radians());
        self.update_crouch(input.held & keys::CROUCH != 0, physics);
        if self.grounded {
            self.running = input.held & keys::RUN != 0;
        }
        if self.grounded && input.pressed & keys::SPACE != 0 {
            // Unity Force mode changes velocity by force / mass * fixed delta.
            self.vertical_velocity = self.reference.jump_force / self.reference.mass_kg * dt;
            self.grounded = false;
        }
        self.vertical_velocity += physics.rapier().gravity.y * dt;
        let x = ((input.held & keys::RIGHT != 0) as u8 as f32)
            - ((input.held & keys::LEFT != 0) as u8 as f32);
        let z = ((input.held & keys::BACK != 0) as u8 as f32)
            - ((input.held & keys::FORWARD != 0) as u8 as f32);
        let direction = Vector::new(x, 0.0, z).normalize_or_zero();
        let speed = self.reference.walk_speed
            * if self.running {
                self.reference.run_multiplier
            } else {
                1.0
            }
            * if self.crouched {
                self.reference.duck_multiplier
            } else {
                1.0
            };
        let desired =
            self.pose.rotation * direction * speed * dt + Vector::Y * self.vertical_velocity * dt;
        self.collisions.clear();
        let shape = if self.crouched {
            &self.crouching
        } else {
            &self.standing
        };
        let filter = QueryFilter::default()
            .exclude_sensors()
            .exclude_rigid_body(self.body);
        let rapier = physics.rapier();
        let movement = self.controller.move_shape(
            dt,
            &physics.queries(filter),
            shape.as_ref(),
            &self.pose,
            desired,
            |collision| {
                let dynamic = rapier.colliders[collision.handle]
                    .parent()
                    .is_some_and(|body| rapier.bodies[body].is_dynamic());
                if dynamic && collision.translation_remaining.dot(collision.hit.normal1) > 1.0e-5 {
                    self.collisions.push(collision);
                }
            },
        );
        if !self.collisions.is_empty() {
            self.controller.solve_character_collision_impulses(
                dt,
                &mut physics.queries_mut(filter),
                shape.as_ref(),
                self.reference.mass_kg,
                &self.collisions,
            );
        }
        self.pose.translation += movement.translation;
        self.grounded = movement.grounded && self.vertical_velocity <= 0.0;
        if self.grounded
            || (self.vertical_velocity > 0.0 && movement.translation.y < desired.y - 1.0e-5)
        {
            self.vertical_velocity = 0.0;
        }
        physics.rapier_mut().bodies[self.body].set_next_kinematic_position(self.pose);
    }

    fn update_crouch(&mut self, want_crouch: bool, physics: &mut PhysicsWorld) {
        if want_crouch == self.crouched {
            return;
        }
        let center_delta = (self.reference.standing_height - self.reference.crouch_height) * 0.5;
        let mut candidate = self.pose;
        candidate.translation.y += if want_crouch {
            -center_delta
        } else {
            center_delta
        };
        if !want_crouch && !self.clearance(candidate, self.standing.as_ref(), physics) {
            return;
        }
        self.crouched = want_crouch;
        self.pose = candidate;
        let shape = if want_crouch {
            self.crouching.clone()
        } else {
            self.standing.clone()
        };
        physics.rapier_mut().colliders[self.collider].set_shape(shape);
    }

    fn clearance(
        &self,
        pose: Pose,
        shape: &dyn rapier3d::prelude::Shape,
        physics: &PhysicsWorld,
    ) -> bool {
        physics
            .queries(
                QueryFilter::default()
                    .exclude_sensors()
                    .exclude_rigid_body(self.body),
            )
            .intersect_shape(pose, shape)
            .next()
            .is_none()
    }

    /// The seat pose places the source sitPos root relative to the chassis.
    pub fn enter(&mut self, id: SeatId, pose: VehiclePose, physics: &mut PhysicsWorld) -> bool {
        if self.occupancy.is_some() {
            return false;
        }
        let Some(seat) = self.seats.iter().find(|seat| seat.id == id) else {
            return false;
        };
        let mut local_pose = to_rapier(pose);
        let center_offset = self.reference.camera_height - self.reference.standing_height * 0.5;
        local_pose.translation += local_pose.rotation * (Vector::Y * center_offset);
        let seated_pose =
            *physics.rapier().bodies[physics.chassis_handle()].position() * local_pose;
        let chassis = physics.chassis_handle();
        let outside_vehicle = |_: ColliderHandle, collider: &rapier3d::prelude::Collider| {
            collider.parent() != Some(chassis)
        };
        let queries = physics.queries(
            QueryFilter::default()
                .exclude_sensors()
                .exclude_rigid_body(self.body)
                .predicate(&outside_vehicle),
        );
        if queries
            .intersect_shape(seated_pose, self.crouching.as_ref())
            .next()
            .is_some()
        {
            return false;
        }
        self.yaw =
            (self.yaw - yaw_from_rotation(seated_pose.rotation) + 180.0).rem_euclid(360.0) - 180.0;
        self.yaw = self
            .yaw
            .clamp(seat.yaw_limits_degrees[0], seat.yaw_limits_degrees[1]);
        self.occupancy = Some(Occupancy {
            id,
            local_pose,
            yaw_limits: seat.yaw_limits_degrees,
            driver: seat.driver,
        });
        self.pose = seated_pose;
        self.crouched = false;
        self.grounded = false;
        self.running = false;
        self.vertical_velocity = 0.0;
        physics.rapier_mut().colliders[self.collider].set_enabled(false);
        physics.rapier_mut().bodies[self.body].set_position(seated_pose, false);
        physics.refresh_queries();
        true
    }

    pub fn try_exit(&mut self, physics: &mut PhysicsWorld, limits: &ExitReference) -> bool {
        if self.occupancy.is_none() {
            return false;
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
            return false;
        }
        // Unity rays ignore colliders containing the ray origin.
        let ray_targets = |_: ColliderHandle, collider: &rapier3d::prelude::Collider| {
            !collider
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
                    + hit.normal * (self.reference.standing_radius + 0.05);
                Vector::new(displacement.x, 0.0, displacement.z)
                    .length()
                    .min(limits.distance)
            });
        let mut vertical = 0.0;
        for horizontal_index in 0..limits.horizontal_steps {
            let distance = horizontal_index as f32 * limits.horizontal_step;
            if distance >= max_distance {
                break;
            }
            for _ in 0..limits.vertical_steps {
                // Source hpoint places the body origin at a point along the head view.
                let mut candidate = Pose::from_translation(
                    view.translation
                        + forward * distance
                        + Vector::Y
                            * (vertical + self.reference.camera_height
                                - self.reference.standing_height * 0.5),
                );
                candidate.rotation =
                    Rotation::from_rotation_y(-yaw_from_rotation(view.rotation).to_radians());
                if self.clearance(candidate, self.standing.as_ref(), physics) {
                    self.pose = candidate;
                    self.yaw = yaw_from_rotation(candidate.rotation);
                    self.vertical_velocity =
                        physics.rapier().bodies[physics.chassis_handle()].linvel().y * 0.3;
                    self.occupancy = None;
                    self.crouched = false;
                    self.grounded = false;
                    self.running = false;
                    let rapier = physics.rapier_mut();
                    rapier.colliders[self.collider].set_shape(self.standing.clone());
                    rapier.colliders[self.collider].set_enabled(true);
                    rapier.bodies[self.body].set_position(candidate, false);
                    physics.refresh_queries();
                    return true;
                }
                vertical += limits.vertical_step;
            }
        }
        false
    }

    pub fn seat(&self) -> Option<SeatId> {
        self.occupancy.map(|seat| seat.id)
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
    pub fn crouched(&self) -> bool {
        self.crouched
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
    pub fn body_handle(&self) -> RigidBodyHandle {
        self.body
    }
    pub fn body_pose(&self, physics: &PhysicsWorld) -> VehiclePose {
        let pose = self.occupancy.map_or(self.pose, |seat| {
            *physics.rapier().bodies[physics.chassis_handle()].position() * seat.local_pose
        });
        from_rapier(pose)
    }
    pub fn view_pose(&self, physics: &PhysicsWorld) -> VehiclePose {
        let mut pose = to_rapier(self.body_pose(physics));
        let head_offset = if self.crouched {
            self.reference.crouch_height * 0.5
        } else {
            self.reference.standing_height * 0.5
        };
        pose.translation += pose.rotation * (Vector::Y * head_offset);
        if self.occupancy.is_some() {
            pose.rotation *= Rotation::from_rotation_y(-self.yaw.to_radians());
        }
        pose.rotation *= Rotation::from_rotation_x(-self.pitch.to_radians());
        from_rapier(pose)
    }
    pub fn focus_position(&self, physics: &PhysicsWorld) -> [f32; 3] {
        self.view_pose(physics).position
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
