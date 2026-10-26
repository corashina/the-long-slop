pub mod collision;
mod head_bob;
mod ladder;
pub(crate) mod normals;
pub mod pose;
mod posture;
mod view;
use collision::PlayerSweepShape;
use posture::{MovementSpeed, PostureState};
use rapier3d::parry::shape::Shape;
pub use view::PlayerView;
use view::ViewState;

use rapier3d::control::{CharacterCollision, KinematicCharacterController};
use rapier3d::math::{Pose, Rotation, Vector};
use rapier3d::prelude::{
    ColliderBuilder, ColliderHandle, Group, InteractionGroups, InteractionTestMode, QueryFilter,
    Ray, RigidBodyBuilder, RigidBodyHandle, SharedShape,
};

use crate::reference::{PlayerPoseReference, PlayerReference, SeatId, SeatReference};
use crate::simulation::{FrameInput, keys};
use crate::vehicle::{PhysicsWorld, VehiclePose};

#[derive(Clone, Copy, Debug)]
pub struct PlayerStepContext {
    pub duck_allowed: bool,
    pub wheel_posture_allowed: bool,
    pub mouse_consumed: bool,
    pub picked_up: bool,
    pub recenter_consumed: bool,
    pub movement_multiplier: f32,
}
impl PlayerStepContext {
    pub fn can_duck(input: FrameInput, tuning_available: bool, picked_up: bool) -> bool {
        !tuning_available
            && (!picked_up
                || (input.player_held & crate::simulation::player_keys::LEAN != 0
                    && input.held & keys::TRANSLATE == 0))
    }
}
impl Default for PlayerStepContext {
    fn default() -> Self {
        Self {
            duck_allowed: false,
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

// Two relative f32 rounding units cover the ten-step source tap accumulation.
pub(crate) fn source_short_hold(duration: f32, limit: f32) -> bool {
    duration < limit - 2. * f32::EPSILON * limit
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
        Self::new(player, seat.exit_distance, seat.min_exit_angle_degrees)
    }
    fn new(player: &PlayerReference, distance: f32, minimum_look_angle_degrees: f32) -> Self {
        Self {
            distance,
            minimum_look_angle_degrees,
            horizontal_step: player.exit_step_horizontal,
            vertical_step: player.exit_step_vertical,
            horizontal_steps: player.exit_steps_horizontal,
            vertical_steps: player.exit_steps_vertical,
        }
    }
}

/// The object that owns the occupied seat. `Vehicle` holds an opaque vehicle index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SeatOwner {
    Vehicle(usize),
    Item(crate::world::ObjectKey),
}

/// Seat values the player keeps while seated.
#[derive(Clone, Copy, Debug)]
pub struct SeatValues {
    pub yaw_limits_degrees: [f32; 2],
    /// Source `limitLessRot`: no yaw clamp.
    pub yaw_unlimited: bool,
    /// Source `lookInsteadOfLean`: lean input turns the camera, and recenter does nothing.
    pub look_instead_of_lean: bool,
    pub pitch_limits_degrees: [f32; 2],
    pub default_pitch_degrees: f32,
    pub driver: bool,
    pub driving2: bool,
    pub show_sleep_text: bool,
    pub exit: ExitReference,
}

impl SeatValues {
    pub fn car(player: &PlayerReference, seat: &SeatReference) -> Self {
        Self {
            yaw_limits_degrees: seat.yaw_limits_degrees,
            yaw_unlimited: false,
            look_instead_of_lean: false,
            pitch_limits_degrees: seat.pitch_limits_degrees,
            default_pitch_degrees: seat.default_pitch_degrees,
            driver: seat.driver,
            driving2: seat.driving2,
            show_sleep_text: seat.show_sleep_text,
            exit: ExitReference::from_reference(player, seat),
        }
    }
    /// Item seats have no driver roles. The exit distance is the player value.
    pub(crate) fn item(
        player: &PlayerReference,
        seat: &crate::world::stream::content_seats::WorldSeatReference,
    ) -> Self {
        Self {
            yaw_limits_degrees: seat.yaw_limits_degrees,
            yaw_unlimited: seat.yaw_unlimited,
            look_instead_of_lean: seat.look_instead_of_lean,
            pitch_limits_degrees: seat.pitch_limits_degrees,
            default_pitch_degrees: seat.default_pitch_degrees,
            driver: false,
            driving2: false,
            show_sleep_text: seat.show_sleep_text,
            exit: ExitReference::new(player, player.exit_distance, seat.min_exit_angle_degrees),
        }
    }
    fn clamp_yaw(&self, yaw: f32) -> f32 {
        if self.yaw_unlimited {
            yaw
        } else {
            yaw.clamp(self.yaw_limits_degrees[0], self.yaw_limits_degrees[1])
        }
    }
}

struct Occupancy {
    /// Car seat record. Item seats have none.
    id: Option<SeatId>,
    /// Main seatscript of an item seat. Car seats have none.
    source: Option<crate::reference::SourceId>,
    owner: SeatOwner,
    /// Owner body, resolved on enter and restore.
    body: RigidBodyHandle,
    local_pose: Pose,
    values: SeatValues,
}

pub struct PlayerState {
    reference: PlayerReference,
    controller: KinematicCharacterController,
    sweep: PlayerSweepShape,
    posture: PostureState,
    movement_speed: MovementSpeed,
    view_state: ViewState,
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
    ladder: ladder::LadderState,
    hazard_damage: f32,
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
        // An item seat waits for its world item binding. See `restore_item_seat`.
        self.occupancy = match saved.seat.as_ref().map(|record| (record, &record.owner)) {
            Some((record, crate::save::SeatOwnerSnapshot::Vehicle { vehicle })) => {
                let id = ids.seats[&record.source];
                let seat = self.seats.iter().find(|s| s.id == id).unwrap();
                let vehicle = ids
                    .vehicles
                    .iter()
                    .position(|v| v == vehicle)
                    .ok_or_else(|| crate::save::SaveError::InvalidLinks("Seat vehicle".into()))?;
                Some(Occupancy {
                    id: Some(id),
                    source: None,
                    owner: SeatOwner::Vehicle(vehicle),
                    body: physics.chassis_handle(vehicle),
                    local_pose: to_rapier(crate::save::local_pose(&record.local_pose)),
                    values: SeatValues::car(&self.reference, seat),
                })
            }
            Some((_, crate::save::SeatOwnerSnapshot::Item { .. })) | None => None,
        };
        if let Some(seat) = &self.occupancy {
            self.pose = *physics.rapier().bodies[seat.body].position() * seat.local_pose;
        }
        self.cancel_view_input();
        self.clear_triggers();
        self.push_gripping = false;
        self.collisions.clear();
        self.grounded = false;
        self.support_body = None;
        physics.edit_body(self.body, |body| {
            body.set_position(self.pose, false);
            body.set_next_kinematic_position(self.pose);
        });
        self.update_geometry(physics);
        if saved.seat.is_none() {
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
    pub fn new(
        reference: &PlayerReference,
        pose_reference: &PlayerPoseReference,
        pose: VehiclePose,
        physics: &mut PhysicsWorld,
    ) -> Self {
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
            view_state: ViewState::new(reference, pose_reference),
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
            ladder: ladder::LadderState::default(),
            hazard_damage: 0.0,
        }
    }

    pub fn capture_save(
        &self,
        ids: &crate::save::IdentityMap,
        origin: &crate::world::WorldOrigin,
        physics: &PhysicsWorld,
        carry: Option<crate::save::CarrySnapshot>,
    ) -> Result<crate::save::PlayerSnapshot, crate::save::SaveError> {
        let (position, rotation) = if let Some(seat) = &self.occupancy {
            let chassis = physics.rapier().bodies[seat.body].position();
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
                .as_ref()
                .map(|seat| {
                    let (owner, source) = match (&seat.owner, seat.id) {
                        (SeatOwner::Vehicle(vehicle), Some(id)) => (
                            crate::save::SeatOwnerSnapshot::Vehicle {
                                vehicle: ids.vehicles[*vehicle].clone(),
                            },
                            crate::save::identity::source_key(&ids.seats, id)?,
                        ),
                        (SeatOwner::Item(item), None) => (
                            crate::save::SeatOwnerSnapshot::Item { item: item.clone() },
                            seat.source
                                .as_ref()
                                .map(crate::save::SourceKey::from)
                                .ok_or_else(|| {
                                    crate::save::SaveError::InvalidLinks("Item seat source".into())
                                })?,
                        ),
                        _ => {
                            return Err(crate::save::SaveError::InvalidLinks("Seat owner".into()));
                        }
                    };
                    Ok(crate::save::SeatSnapshot {
                        owner,
                        source,
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
    pub fn step_view(&mut self, input: FrameInput, context: PlayerStepContext, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        let seated = self.occupancy.is_some();
        let look_mode = self
            .occupancy
            .as_ref()
            .is_some_and(|seat| seat.values.look_instead_of_lean);
        if let Some(seat) = &self.occupancy
            && !seat.values.look_instead_of_lean
            && !context.recenter_consumed
            && input.held & keys::RECENTER != 0
        {
            self.view_state.recenter();
            let amount = (dt * self.reference.posture.rise_rate).min(1.);
            self.yaw += (0. - self.yaw) * amount;
            self.pitch += (seat.values.default_pitch_degrees - self.pitch) * amount;
        }
        let head_rotation = self.head_rotation();
        let ordinary_look = self.view_state.step(
            input,
            context,
            self.posture.value(),
            seated,
            look_mode,
            head_rotation,
            dt,
        );
        if let Some(multiplier) = ordinary_look {
            if input.mouse[0].is_finite() {
                self.yaw += input.mouse[0] * multiplier;
            }
            if input.mouse[1].is_finite() {
                self.pitch += input.mouse[1] * multiplier;
            }
        }
        self.pitch = self.pitch.clamp(-89.0, 89.0);
        if let Some(seat) = &self.occupancy {
            let [pitch_min, pitch_max] = seat.values.pitch_limits_degrees;
            self.yaw = seat.values.clamp_yaw(self.yaw);
            self.pitch = self.pitch.clamp(pitch_min, pitch_max);
        } else {
            self.yaw = (self.yaw + 180.0).rem_euclid(360.0) - 180.0;
            self.pose.rotation = Rotation::from_rotation_y(-self.yaw.to_radians());
        }
    }
    pub fn step_movement(
        &mut self,
        input: FrameInput,
        context: PlayerStepContext,
        physics: &mut PhysicsWorld,
        dt: f32,
    ) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        if let Some(seat) = &self.occupancy {
            self.pose = *physics.rapier().bodies[seat.body].position() * seat.local_pose;
            physics.edit_body(self.body, |body| {
                body.set_next_kinematic_position(self.pose)
            });
            return;
        }
        if self.push_gripping {
            self.pose.translation = physics.rapier().bodies[self.body].position().translation;
        }
        let was_grounded = self.grounded;
        let vertical_before_jump = self.vertical_velocity;
        self.ladder.tick(input);
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
        if self.grounded || self.ladder.ladder().is_some() {
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
        let head = self.pose.rotation * self.head_rotation();
        let climb = self.ladder.climb_velocity(
            input,
            head * -Vector::Z,
            head * Vector::X,
            self.grounded,
            self.reference.ladder_speed,
            self.running.then_some(self.reference.run_multiplier),
        );
        if climb.is_some() {
            // Source LadderMove replaces the body velocity.
            self.vertical_velocity = 0.0;
            self.recoil_velocity = Vector::ZERO;
        } else if self.ladder.gravity() {
            self.vertical_velocity = (self.vertical_velocity + physics.rapier().gravity.y * dt)
                .max(-self.reference.max_fall_speed);
        }
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
        let desired = climb.map_or(
            self.pose.rotation * direction * speed * dt
                + self.recoil_velocity * dt
                + Vector::Y * self.vertical_velocity * dt,
            |velocity| velocity * dt,
        );
        self.collisions.clear();
        let shape = &self.sweep;
        let filter = self.movement_filter();
        let rapier = physics.rapier();
        let movement = self.controller.move_shape(
            dt,
            &physics.character_queries(filter),
            shape,
            &self.pose,
            desired,
            |collision| {
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
        self.support_body = if self.grounded {
            physics
                .queries(filter)
                .cast_ray(
                    &Ray::new(self.pose.translation, -Vector::Y),
                    -self.sweep.compute_local_aabb().mins.y + 0.1,
                    true,
                )
                .and_then(|(collider, _)| physics.rapier().colliders[collider].parent())
        } else {
            None
        };
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

    /// Source trigger stays. Run before `step_movement`. Seated, only hazards count.
    pub fn scan_triggers(&mut self, physics: &PhysicsWorld, roles: &crate::world::TriggerRoles) {
        use crate::reference::TriggerRole;
        self.hazard_damage = 0.0;
        if roles.is_empty() {
            return;
        }
        let seated = self.occupancy.is_some();
        let filter = QueryFilter::default()
            .exclude_solids()
            .exclude_rigid_body(self.body)
            .groups(self.collision_groups());
        for (handle, collider) in physics
            .queries(filter)
            .intersect_shape(self.pose, &self.sweep)
        {
            match roles.get(handle, collider) {
                Some(TriggerRole::Ladder { speed, up }) if !seated => {
                    self.ladder.touch(ladder::LadderContact {
                        collider: handle,
                        speed,
                        up: Vector::from_array(up),
                    })
                }
                Some(TriggerRole::Hazard { damage_player, .. }) => {
                    self.hazard_damage += damage_player
                }
                _ => {}
            }
        }
    }
    /// Sum of touched hazard damage per second.
    pub fn hazard_damage(&self) -> f32 {
        self.hazard_damage
    }
    pub fn clear_triggers(&mut self) {
        self.ladder = ladder::LadderState::default();
        self.hazard_damage = 0.0;
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
        // Source climbing sets BcanJump.
        let force = self.posture.step(
            posture_input,
            context.wheel_posture_allowed,
            self.grounded || self.ladder.climbed(),
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
        let occupancy = Occupancy {
            id: Some(id),
            source: None,
            owner: SeatOwner::Vehicle(seat.vehicle),
            body: physics.chassis_handle(seat.vehicle),
            local_pose: to_rapier(pose),
            values: SeatValues::car(&self.reference, seat),
        };
        self.occupy(occupancy, physics)
    }
    /// Sit on a world item seat. The pose places sitPos relative to the item body.
    pub(crate) fn enter_item_seat(
        &mut self,
        owner: crate::world::ObjectKey,
        body: RigidBodyHandle,
        seat: &crate::world::stream::content_seats::WorldSeatReference,
        pose: VehiclePose,
        physics: &mut PhysicsWorld,
    ) -> bool {
        if self.occupancy.is_some() {
            return false;
        }
        let occupancy = self.item_occupancy(owner, body, seat, pose);
        self.occupy(occupancy, physics)
    }
    /// Finish a saved item seat after the world item binds. View and posture keep their saved values.
    pub(crate) fn restore_item_seat(
        &mut self,
        owner: crate::world::ObjectKey,
        body: RigidBodyHandle,
        seat: &crate::world::stream::content_seats::WorldSeatReference,
        local_pose: VehiclePose,
        physics: &mut PhysicsWorld,
    ) {
        let occupancy = self.item_occupancy(owner, body, seat, local_pose);
        self.pose = *physics.rapier().bodies[body].position() * occupancy.local_pose;
        self.occupancy = Some(occupancy);
        self.update_geometry(physics);
        physics.edit_body(self.body, |body| {
            body.set_position(self.pose, false);
            body.set_next_kinematic_position(self.pose);
        });
        physics.refresh_queries();
    }
    fn item_occupancy(
        &self,
        owner: crate::world::ObjectKey,
        body: RigidBodyHandle,
        seat: &crate::world::stream::content_seats::WorldSeatReference,
        pose: VehiclePose,
    ) -> Occupancy {
        Occupancy {
            id: None,
            source: Some(seat.source.clone()),
            owner: SeatOwner::Item(owner),
            body,
            local_pose: to_rapier(pose),
            values: SeatValues::item(&self.reference, seat),
        }
    }
    fn occupy(&mut self, occupancy: Occupancy, physics: &mut PhysicsWorld) -> bool {
        let old_head = to_rapier(self.view(physics).head_pose);
        let seated_pose =
            *physics.rapier().bodies[occupancy.body].position() * occupancy.local_pose;
        let head_in_seat = seated_pose.inverse() * old_head;
        self.yaw = occupancy
            .values
            .clamp_yaw(yaw_from_rotation(head_in_seat.rotation));
        self.occupancy = Some(occupancy);
        self.pose = seated_pose;
        self.posture.restore(1.0);
        self.clear_triggers();
        self.view_state.enter(head_in_seat);
        self.grounded = false;
        self.running = false;
        self.vertical_velocity = 0.0;
        self.update_geometry(physics);
        physics.edit_body(self.body, |body| body.set_position(seated_pose, false));
        physics.refresh_queries();
        true
    }

    fn exit_pose(&self, physics: &PhysicsWorld) -> Option<Pose> {
        let limits = &self.occupancy.as_ref()?.values.exit;
        let resolved = self.view(physics);
        let view = to_rapier(resolved.head_pose);
        let camera = to_rapier(resolved.camera_pose);
        let body = to_rapier(self.body_pose(physics));
        if !view::exit_permission(camera, body, limits.minimum_look_angle_degrees) {
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
        view::search_exit(
            view,
            limits,
            self.reference.posture.maximum_radius + 0.05,
            self.exit_capsule.as_ref(),
            &queries,
            &exit_queries,
        )
    }
    /// Exit rules come from the occupied seat values.
    pub fn can_exit(&self, physics: &PhysicsWorld) -> bool {
        self.exit_pose(physics).is_some()
    }
    pub fn try_exit(&mut self, physics: &mut PhysicsWorld) -> bool {
        let Some(candidate) = self.exit_pose(physics) else {
            return false;
        };
        let Some(seat) = self.occupancy.take() else {
            return false;
        };
        self.pose = candidate;
        self.yaw = yaw_from_rotation(candidate.rotation);
        let inherited = physics.rapier().bodies[seat.body].linvel() * 0.3;
        self.vertical_velocity = inherited.y;
        self.recoil_velocity = Vector::new(inherited.x, 0.0, inherited.z);
        self.posture.restore(1.0);
        self.cancel_view_input();
        self.grounded = false;
        self.running = false;

        self.update_geometry(physics);
        physics.edit_body(self.body, |body| body.set_position(candidate, false));
        physics.refresh_queries();
        true
    }
    /// The occupied car seat. Item seats return `None`.
    pub fn seat(&self) -> Option<SeatId> {
        self.occupancy.as_ref().and_then(|seat| seat.id)
    }
    pub fn seated(&self) -> bool {
        self.occupancy.is_some()
    }
    pub fn seat_owner(&self) -> Option<&SeatOwner> {
        self.occupancy.as_ref().map(|seat| &seat.owner)
    }
    /// Returns the vehicle that owns the occupied seat. Item seats return `None`.
    pub fn seat_vehicle(&self) -> Option<usize> {
        match self.seat_owner()? {
            SeatOwner::Vehicle(vehicle) => Some(*vehicle),
            SeatOwner::Item(_) => None,
        }
    }
    /// The body of the seat owner: car chassis or item body.
    pub fn seat_body(&self) -> Option<RigidBodyHandle> {
        self.occupancy.as_ref().map(|seat| seat.body)
    }
    /// The body of the occupied item seat. Car seats return `None`.
    pub fn item_seat_body(&self) -> Option<RigidBodyHandle> {
        self.occupancy
            .as_ref()
            .filter(|seat| matches!(seat.owner, SeatOwner::Item(_)))
            .map(|seat| seat.body)
    }
    /// The main seatscript of the occupied item seat. Car seats return `None`.
    pub fn item_seat_source(&self) -> Option<&crate::reference::SourceId> {
        self.occupancy
            .as_ref()
            .and_then(|seat| seat.source.as_ref())
    }
    /// seatscript.showSleepText of the occupied seat.
    pub fn show_sleep_text(&self) -> bool {
        self.occupancy
            .as_ref()
            .is_some_and(|seat| seat.values.show_sleep_text)
    }
    pub fn mode(&self) -> PlayerMode {
        if self.occupancy.is_some() {
            PlayerMode::Seated
        } else {
            PlayerMode::Walking
        }
    }
    pub fn driving(&self) -> bool {
        self.occupancy
            .as_ref()
            .is_some_and(|seat| seat.values.driver)
    }
    pub fn driving2(&self) -> bool {
        self.occupancy
            .as_ref()
            .is_some_and(|seat| seat.values.driving2)
    }
    pub fn posture(&self) -> f32 {
        self.posture.value()
    }
    pub fn support_body(&self) -> Option<RigidBodyHandle> {
        self.support_body
    }
    /// Run after the physics step, so body velocities hold this step's motion.
    pub fn step_head_bob(
        &mut self,
        input: FrameInput,
        dead: bool,
        physics: &PhysicsWorld,
        dt: f32,
    ) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        let bodies = &physics.rapier().bodies;
        if let Some(seat) = &self.occupancy {
            let head = to_rapier(self.view(physics).head_pose).rotation;
            let velocity = bodies[seat.body].linvel();
            self.view_state.step_car_bob(dt, velocity, dead, head);
            return;
        }
        let axis =
            |positive, negative| (input.held & positive != 0) != (input.held & negative != 0);
        let moving = axis(keys::RIGHT, keys::LEFT) || axis(keys::BACK, keys::FORWARD);
        let speed = bodies[self.body].linvel().length()
            - self
                .support_body
                .map_or(0.0, |body| bodies[body].linvel().length());
        self.view_state.step_walk_bob(
            (self.grounded || self.ladder.climbed()) && !self.push_gripping && moving,
            speed,
            dt,
        );
    }
    /// True on the step a source footstep fires.
    pub fn footstep_beat(&self) -> bool {
        self.view_state.footstep_beat()
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
        let pose = self.occupancy.as_ref().map_or(walking_pose, |seat| {
            *physics.rapier().bodies[seat.body].position() * seat.local_pose
        });
        from_rapier(pose)
    }
    fn head_rotation(&self) -> Rotation {
        let yaw = if self.occupancy.is_some() {
            Rotation::from_rotation_y(-self.yaw.to_radians())
        } else {
            Rotation::IDENTITY
        };
        yaw * Rotation::from_rotation_x(-self.pitch.to_radians())
    }
    /// Source THeadDir/TProneHeadPos midpoint. Seated THeadDir follows unsmoothed lean.
    pub(crate) fn source_anchor_midpoint(&self, physics: &PhysicsWorld) -> Vector {
        let body = to_rapier(self.body_pose(physics));
        body.translation
            + body.rotation
                * self
                    .view_state
                    .source_anchor_midpoint(self.posture.value(), self.occupancy.is_some())
    }
    pub fn neutral_view_at(&self, body_pose: VehiclePose) -> PlayerView {
        self.view_state.resolve(
            to_rapier(body_pose),
            self.head_rotation(),
            self.occupancy.is_some(),
            Some(self.posture.value()),
        )
    }
    pub fn view(&self, physics: &PhysicsWorld) -> PlayerView {
        self.view_state.resolve(
            to_rapier(self.body_pose(physics)),
            self.head_rotation(),
            self.occupancy.is_some(),
            None,
        )
    }
    pub(crate) fn view_owns_look_input(&self) -> bool {
        self.view_state.looking()
    }
    pub fn cancel_view_input(&mut self) {
        self.view_state.cancel(self.posture.value());
    }
    /// Place the player on foot. An item seat ends here.
    pub(crate) fn recover(&mut self, pose: VehiclePose, physics: &mut PhysicsWorld) {
        self.occupancy = None;
        self.pose = to_rapier(pose);
        self.yaw = yaw_from_rotation(self.pose.rotation);
        self.posture.restore(self.posture.value());
        self.cancel_view_input();
        self.clear_triggers();
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
        self.view(physics).camera_pose.position
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

#[cfg(test)]
mod seat_values_tests {
    use super::{ExitReference, SeatValues};

    #[test]
    fn unlimited_yaw_skips_the_clamp() {
        let mut values = SeatValues {
            yaw_limits_degrees: [-60., 60.],
            yaw_unlimited: false,
            look_instead_of_lean: false,
            pitch_limits_degrees: [-89., 89.],
            default_pitch_degrees: 0.,
            driver: false,
            driving2: false,
            show_sleep_text: false,
            exit: ExitReference {
                distance: 1.,
                minimum_look_angle_degrees: 0.,
                horizontal_step: 0.,
                vertical_step: 0.,
                horizontal_steps: 0,
                vertical_steps: 0,
            },
        };
        assert_eq!(values.clamp_yaw(200.), 60.);
        values.yaw_unlimited = true;
        assert_eq!(values.clamp_yaw(200.), 200.);
    }
}
