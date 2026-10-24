mod mass;
mod mutations;
mod presentation_bodies;
mod query_bounds;

use presentation_bodies::{BodyPresentationState, PresentationBodies};
use query_bounds::QueryBounds;
use rapier3d::control::{DynamicRayCastVehicleController, Wheel, WheelTuning};
use rapier3d::math::{Pose, Rotation, Vector};
pub use rapier3d::prelude::ColliderHandle;
use rapier3d::prelude::{
    ColliderBuilder, Group, InteractionGroups, InteractionTestMode, QueryFilter, QueryPipeline,
    QueryPipelineMut, RigidBodyBuilder, RigidBodyHandle,
};

use crate::reference::{
    ColliderGeometry, ReferenceError, ReferenceManifest, VehicleReference, WheelSlotId,
    WheelSlotReference,
};
use crate::simulation::{DriveInput, FIXED_DT};

/// Axle torque after gear ratios and engine engagement. Motor torque has a sign.
#[derive(Clone, Copy, Debug, Default)]
pub struct DriveTorques {
    pub motor_nm: f32,
    pub engine_brake_nm: f32,
}

#[derive(Clone, Copy)]
pub struct InstalledWheel {
    pub radius: f32,
    pub condition: f32,
}
pub const VEHICLE_COLLIDER_FLAG: u128 = 1 << 127;

/// Rapier 0.36 CCD sweeps disabled colliders too. A disabled collider below the
/// ground then clamps its body's motion each step. Empty solver groups stop this.
pub fn solver_groups(enabled: bool) -> InteractionGroups {
    if enabled {
        InteractionGroups::all()
    } else {
        InteractionGroups::none()
    }
}

/// Enables or disables a collider. See [`solver_groups`].
pub fn set_collider_enabled(collider: &mut rapier3d::prelude::Collider, enabled: bool) {
    collider.set_enabled(enabled);
    collider.set_solver_groups(solver_groups(enabled));
}
pub const SLOT_SENSOR_FLAG: u128 = 1 << 126;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VehiclePose {
    pub position: [f32; 3],
    pub rotation: [f32; 4],
}

#[derive(Clone, Copy, Debug)]
pub struct VehicleSnapshot {
    pub pose: VehiclePose,
    pub wheels: [VehiclePose; 4],
    pub speed_mps: f32,
    pub linear_velocity: [f32; 3],
    pub angular_velocity: [f32; 3],
    pub wheels_contact: [bool; 4],
}

impl VehicleSnapshot {
    pub fn absolute_speed_kmh(&self) -> f32 {
        self.linear_velocity
            .iter()
            .map(|v| v * v)
            .sum::<f32>()
            .sqrt()
            * 3.6
    }
}

/// One rigid chassis with its ray-cast wheels.
struct VehicleBody {
    chassis: RigidBodyHandle,
    controller: DynamicRayCastVehicleController,
    resting_wheels: [Wheel; 4],
    wheel_references: [WheelSlotReference; 4],
    suspension_lengths: [f32; 4],
    max_steer: f32,
    brake_torque: f32,
    handbrake_torque: f32,
    active_slots: Vec<usize>,
    installed: [Option<InstalledWheel>; 4],
    wheel_rotations: [f32; 4],
    wheel_rpm: [f32; 4],
    last_handbrake: f32,
}

pub struct PhysicsWorld {
    body_mass_override: Option<mass::BodyMassOverride>,
    pub(crate) inactive_bodies: std::collections::HashSet<RigidBodyHandle>,
    physics: rapier3d::prelude::PhysicsWorld,
    query_bounds: QueryBounds,
    presentation_bodies: PresentationBodies,
    chassis: Vec<RigidBodyHandle>,
    vehicles: Vec<VehicleBody>,
}

impl PhysicsWorld {
    /// Poses follow the order of `reference.vehicles`.
    pub fn new(
        reference: &ReferenceManifest,
        poses: &[VehiclePose],
    ) -> Result<Self, ReferenceError> {
        if poses.len() != reference.vehicles.len() {
            return Err(ReferenceError("Expected one pose per vehicle".into()));
        }
        let mut physics = Self {
            physics: rapier3d::prelude::PhysicsWorld::new(),
            body_mass_override: None,
            query_bounds: QueryBounds::default(),
            presentation_bodies: PresentationBodies::default(),
            chassis: Vec::with_capacity(poses.len()),
            vehicles: Vec::with_capacity(poses.len()),
            inactive_bodies: std::collections::HashSet::with_capacity(
                reference.bodies.len() + reference.parts.len() + reference.wheel_parts.len(),
            ),
        };
        physics.physics.integration_parameters.dt = FIXED_DT;
        for (vehicle, pose) in reference.vehicles.iter().zip(poses) {
            let vehicle = VehicleBody::new(reference, vehicle, *pose, &mut physics)?;
            physics.chassis.push(vehicle.chassis);
            physics.vehicles.push(vehicle);
        }
        physics.refresh_queries();
        for vehicle in &mut physics.vehicles {
            // Reserve Rapier's four-wheel scratch buffers before the frame loop.
            vehicle.controller.update_vehicle(
                FIXED_DT,
                QueryPipelineMut {
                    dispatcher: physics.physics.narrow_phase.query_dispatcher(),
                    bvh: &physics.query_bounds.bvh,
                    bodies: &mut physics.physics.bodies,
                    colliders: &mut physics.physics.colliders,
                    filter: QueryFilter::default()
                        .exclude_sensors()
                        .exclude_rigid_body(vehicle.chassis),
                },
            );
            vehicle
                .controller
                .wheels_mut()
                .copy_from_slice(&vehicle.resting_wheels);
        }
        Ok(physics)
    }
}

impl VehicleBody {
    fn new(
        reference: &ReferenceManifest,
        vehicle: &VehicleReference,
        pose: VehiclePose,
        physics: &mut PhysicsWorld,
    ) -> Result<Self, ReferenceError> {
        let source_body = reference
            .bodies
            .iter()
            .find(|body| body.id == vehicle.body)
            .ok_or_else(|| ReferenceError("Missing chassis body".into()))?;
        let chassis = physics.insert_body(
            RigidBodyBuilder::dynamic()
                .pose(to_rapier(pose))
                .linear_damping(source_body.linear_damping)
                .angular_damping(source_body.angular_damping)
                .ccd_enabled(true),
        );
        for id in &vehicle.colliders {
            let collider = reference
                .colliders
                .iter()
                .find(|record| record.id == *id)
                .ok_or_else(|| ReferenceError(format!("Missing vehicle collider {}", id.0)))?;
            let builder = collider_builder(&collider.geometry)?
                .friction(collider.friction)
                .restitution(collider.restitution)
                .sensor(collider.sensor)
                .enabled(collider.enabled)
                .solver_groups(solver_groups(collider.enabled))
                .collision_groups(InteractionGroups::new(
                    Group::from_bits_truncate(collider.membership),
                    Group::from_bits_truncate(collider.filter),
                    InteractionTestMode::And,
                ))
                .density(if collider.sensor { 0.0 } else { 1.0 })
                .user_data(VEHICLE_COLLIDER_FLAG | ((id.0 as u128) << 64));
            physics.insert_collider_with_parent(builder, chassis);
        }
        physics.recompute_body_mass(chassis);
        let geometry_mass = physics.rapier().bodies[chassis].mass();
        if geometry_mass <= 0.0 || geometry_mass >= vehicle.mass_kg {
            return Err(ReferenceError(
                "Invalid car collision volume or mass".into(),
            ));
        }
        physics.edit_body(chassis, |body| {
            body.set_additional_mass(vehicle.mass_kg - geometry_mass, true)
        });
        physics.recompute_body_mass(chassis);
        let mut controller = DynamicRayCastVehicleController::new(chassis);
        controller.index_forward_axis = 2;
        let mut wheel_records = Vec::with_capacity(4);
        let mut suspension_lengths = [0.0; 4];
        for (index, id) in vehicle.wheel_slots.iter().enumerate() {
            let wheel = reference
                .wheel_slots
                .iter()
                .find(|record| record.id == *id)
                .ok_or_else(|| ReferenceError(format!("Missing wheel slot {}", id.0)))?;
            let anchor = reference
                .anchors
                .iter()
                .find(|record| record.id == wheel.anchor)
                .ok_or_else(|| {
                    ReferenceError(format!("Missing wheel anchor {}", wheel.anchor.0))
                })?;
            let rotation = Rotation::from_xyzw(
                anchor.transform.rotation[0],
                anchor.transform.rotation[1],
                anchor.transform.rotation[2],
                anchor.transform.rotation[3],
            );
            let connection = Vector::from_array(anchor.transform.translation)
                + rotation * Vector::from_array(wheel.center);
            let rest_length = wheel.suspension_distance * (1.0 - wheel.target_position);
            suspension_lengths[index] = rest_length;
            let tuning = WheelTuning {
                suspension_stiffness: wheel.spring / vehicle.mass_kg,
                suspension_compression: wheel.damper / vehicle.mass_kg,
                suspension_damping: wheel.damper / vehicle.mass_kg,
                max_suspension_travel: rest_length.min(wheel.suspension_distance - rest_length),
                friction_slip: wheel.sideways_friction.stiffness,
                max_suspension_force: wheel.spring * wheel.suspension_distance,
                ..Default::default()
            };
            controller.add_wheel(
                connection,
                -Vector::Y,
                Vector::X,
                rest_length,
                wheel.radius,
                &tuning,
            );
            wheel_records.push(wheel.clone());
        }
        let resting_wheels = std::array::from_fn(|i| controller.wheels()[i]);
        Ok(Self {
            chassis,
            controller,
            resting_wheels,
            wheel_references: wheel_records
                .try_into()
                .map_err(|_| ReferenceError("Expected four wheels".into()))?,
            suspension_lengths,
            max_steer: vehicle.max_steer_degrees.to_radians(),
            brake_torque: vehicle.brake_torque_nm,
            handbrake_torque: vehicle.handbrake_torque_nm,
            active_slots: (0..4).collect(),
            installed: std::array::from_fn(|index| {
                Some(InstalledWheel {
                    radius: reference
                        .wheel_slots
                        .iter()
                        .find(|wheel| wheel.id == vehicle.wheel_slots[index])
                        .unwrap()
                        .radius,
                    condition: 1.0,
                })
            }),
            wheel_rotations: [0.0; 4],
            wheel_rpm: [0.0; 4],
            last_handbrake: vehicle.handbrake as u8 as f32,
        })
    }
}

impl PhysicsWorld {
    pub fn vehicle_count(&self) -> usize {
        self.vehicles.len()
    }

    /// Returns the vehicle whose chassis is `body`.
    pub fn vehicle_of(&self, body: RigidBodyHandle) -> Option<usize> {
        self.chassis.iter().position(|chassis| *chassis == body)
    }

    /// Returns the vehicle that owns the wheel slot.
    pub fn slot_vehicle(&self, slot: WheelSlotId) -> Option<usize> {
        self.vehicles.iter().position(|vehicle| {
            vehicle
                .wheel_references
                .iter()
                .any(|reference| reference.id == slot)
        })
    }

    pub fn apply_vehicle_forces(&mut self, index: usize, input: DriveInput, torques: DriveTorques) {
        let drive_count = self.driven_wheel_count(index).max(1) as f32;
        let vehicle = &mut self.vehicles[index];
        let pedal = |value: f32| {
            if value.is_finite() {
                value.clamp(0.0, 1.0)
            } else {
                0.0
            }
        };
        let brake = pedal(input.brake);
        let handbrake = pedal(input.handbrake);
        let motor_nm = if torques.motor_nm.is_finite() {
            torques.motor_nm
        } else {
            0.0
        };
        let engine_brake_nm = if torques.engine_brake_nm.is_finite() {
            torques.engine_brake_nm.max(0.0)
        } else {
            0.0
        };
        if pedal(input.throttle) > 0.0
            || brake > 0.0
            || motor_nm != 0.0
            || input.steer != 0.0
            || handbrake != vehicle.last_handbrake
        {
            self.query_bounds.mark_body(vehicle.chassis);
            let body = &mut self.physics.bodies[vehicle.chassis];
            let before = BodyPresentationState::capture(body);
            body.wake_up(true);
            self.presentation_bodies
                .mark_if_changed(vehicle.chassis, before, body);
        }
        vehicle.last_handbrake = handbrake;
        if self.physics.bodies[vehicle.chassis].is_sleeping() {
            return;
        }
        let steering = if input.steer.is_finite() {
            input.steer.clamp(-1.0, 1.0) * vehicle.max_steer
        } else {
            0.0
        };
        for (i, wheel) in vehicle.controller.wheels_mut().iter_mut().enumerate() {
            let slot = vehicle.active_slots[i];
            let reference = &vehicle.wheel_references[slot];
            wheel.steering = if reference.steer { steering } else { 0.0 };
            wheel.engine_force = if reference.drive {
                motor_nm / (drive_count * wheel.radius)
            } else {
                0.0
            };
            let mut brake_nm = if reference.drive {
                engine_brake_nm / drive_count
            } else {
                0.0
            };
            if reference.brake {
                brake_nm += vehicle.brake_torque * brake;
            }
            if reference.handbrake {
                brake_nm += vehicle.handbrake_torque * handbrake;
            }
            // Rapier 0.36 uses this impulse only when engine_force is zero.
            wheel.brake = brake_nm / wheel.radius * FIXED_DT;
            wheel.friction_slip =
                0.75 + 0.5 * vehicle.installed[slot].unwrap().condition.clamp(0.0, 1.0);
        }
        let own = vehicle.chassis;
        let chassis = &self.chassis;
        // Wheels hit other vehicles' bodies but never their own vehicle.
        let not_own = |_: ColliderHandle, collider: &rapier3d::prelude::Collider| {
            let parent = collider.parent();
            parent != Some(own)
                && (collider.user_data & VEHICLE_COLLIDER_FLAG == 0
                    || parent.is_some_and(|parent| chassis.contains(&parent)))
        };
        vehicle.controller.update_vehicle(
            FIXED_DT,
            QueryPipelineMut {
                dispatcher: self.physics.narrow_phase.query_dispatcher(),
                bvh: &self.query_bounds.bvh,
                bodies: &mut self.physics.bodies,
                colliders: &mut self.physics.colliders,
                filter: QueryFilter::default()
                    .exclude_sensors()
                    .exclude_rigid_body(own)
                    .predicate(&not_own),
            },
        );
        for (wheel, &slot) in vehicle
            .controller
            .wheels()
            .iter()
            .zip(&vehicle.active_slots)
        {
            vehicle.wheel_rpm[slot] = (wheel.rotation - vehicle.wheel_rotations[slot]) / FIXED_DT
                * 60.0
                / std::f32::consts::TAU;
            vehicle.wheel_rotations[slot] = wheel.rotation;
        }
    }

    pub fn body_active(&self, handle: RigidBodyHandle) -> bool {
        !self.inactive_bodies.contains(&handle)
    }

    pub fn step_physics(&mut self) {
        for body in self.physics.islands.active_bodies() {
            self.query_bounds.mark_simulated_body(body);
            self.presentation_bodies.mark(body);
        }
        self.physics.step();
        for body in self.physics.islands.active_bodies() {
            self.query_bounds.mark_simulated_body(body);
            self.presentation_bodies.mark(body);
        }
        self.query_bounds.requeue_step_mutations();
        self.query_bounds.refresh(&mut self.physics);
        self.query_bounds.finish_step();
    }

    pub fn capture_save(
        &self,
        vehicle: usize,
        identity: crate::save::BodyIdentity,
        origin: &crate::world::WorldOrigin,
    ) -> crate::save::BodySnapshot {
        capture_body_save(
            &self.physics.bodies[self.chassis_handle(vehicle)],
            identity,
            origin,
        )
    }
    pub(crate) fn restore_wheel_save(
        &mut self,
        vehicle: usize,
        index: usize,
        saved: &crate::save::WheelAttachment,
    ) {
        let state = &mut self.vehicles[vehicle];
        state.resting_wheels[index].rotation = saved.rotation;
        if let Some(active) = state.active_slots.iter().position(|slot| *slot == index) {
            state.controller.wheels_mut()[active].rotation = saved.rotation;
        }
        state.wheel_rpm[index] = saved.rpm;
        state.wheel_rotations[index] = saved.previous_rotation;
    }
    pub(crate) fn wheel_save_state(&self, vehicle: usize, index: usize) -> (f32, f32, f32) {
        let state = &self.vehicles[vehicle];
        let wheel = state
            .active_slots
            .iter()
            .position(|slot| *slot == index)
            .map_or(&state.resting_wheels[index], |active| {
                &state.controller.wheels()[active]
            });
        (
            wheel.rotation,
            state.wheel_rpm[index],
            state.wheel_rotations[index],
        )
    }

    pub fn wheel_support_count(&self, index: usize) -> usize {
        self.vehicles[index].active_slots.len()
    }

    pub fn driven_wheel_rpm(&self, index: usize) -> f32 {
        let vehicle = &self.vehicles[index];
        let mut sum = 0.0;
        let mut count = 0;
        for &slot in &vehicle.active_slots {
            if vehicle.wheel_references[slot].drive {
                sum += vehicle.wheel_rpm[slot];
                count += 1;
            }
        }
        sum / count.max(1) as f32
    }

    pub fn driven_wheel_count(&self, index: usize) -> usize {
        let vehicle = &self.vehicles[index];
        vehicle
            .active_slots
            .iter()
            .filter(|&&slot| vehicle.wheel_references[slot].drive)
            .count()
    }

    pub fn set_vehicle_mass(&mut self, index: usize, mass_kg: f32) {
        let vehicle = &mut self.vehicles[index];
        let geometry_mass: f32 = self.physics.bodies[vehicle.chassis]
            .colliders()
            .iter()
            .map(|handle| self.physics.colliders[*handle].mass())
            .sum();
        self.query_bounds.mark_body(vehicle.chassis);
        let body = &mut self.physics.bodies[vehicle.chassis];
        let before = BodyPresentationState::capture(body);
        body.set_additional_mass((mass_kg - geometry_mass).max(0.0), true);
        body.recompute_mass_properties_from_colliders(&self.physics.colliders);
        self.presentation_bodies
            .mark_if_changed(vehicle.chassis, before, body);
        let mass = body.mass();
        for (wheel, reference) in vehicle
            .resting_wheels
            .iter_mut()
            .zip(&vehicle.wheel_references)
        {
            wheel.suspension_stiffness = reference.spring / mass;
            wheel.damping_compression = reference.damper / mass;
            wheel.damping_relaxation = reference.damper / mass;
        }
        for (wheel, &slot) in vehicle
            .controller
            .wheels_mut()
            .iter_mut()
            .zip(&vehicle.active_slots)
        {
            let source = &vehicle.resting_wheels[slot];
            wheel.suspension_stiffness = source.suspension_stiffness;
            wheel.damping_compression = source.damping_compression;
            wheel.damping_relaxation = source.damping_relaxation;
        }
    }

    pub fn set_wheel(
        &mut self,
        slot: WheelSlotId,
        installed: Option<InstalledWheel>,
    ) -> Result<(), ReferenceError> {
        let vehicle = self
            .slot_vehicle(slot)
            .map(|index| &mut self.vehicles[index])
            .ok_or_else(|| ReferenceError("Unknown wheel slot".into()))?;
        let index = vehicle
            .wheel_references
            .iter()
            .position(|reference| reference.id == slot)
            .unwrap();
        let mut saved = vehicle.resting_wheels;
        for (wheel, &slot) in vehicle
            .controller
            .wheels()
            .iter()
            .zip(&vehicle.active_slots)
        {
            saved[slot] = *wheel;
        }
        vehicle.installed[index] = installed;
        vehicle.wheel_rpm[index] = 0.0;
        let mut controller = DynamicRayCastVehicleController::new(vehicle.chassis);
        controller.index_forward_axis = 2;
        vehicle.active_slots.clear();
        for (slot, installed) in vehicle.installed.iter().enumerate() {
            let Some(installed) = installed else { continue };
            let wheel = &saved[slot];
            let added = controller.add_wheel(
                wheel.chassis_connection_point_cs,
                wheel.direction_cs,
                wheel.axle_cs,
                wheel.suspension_rest_length,
                installed.radius,
                &WheelTuning::default(),
            );
            *added = *wheel;
            added.radius = installed.radius;
            added.friction_slip = 0.75 + 0.5 * installed.condition.clamp(0.0, 1.0);
            vehicle.active_slots.push(slot);
        }
        vehicle.controller = controller;
        self.query_bounds.mark_body(vehicle.chassis);
        let body = &mut self.physics.bodies[vehicle.chassis];
        let before = BodyPresentationState::capture(body);
        body.wake_up(true);
        self.presentation_bodies
            .mark_if_changed(vehicle.chassis, before, body);
        Ok(())
    }

    pub fn rapier(&self) -> &rapier3d::prelude::PhysicsWorld {
        &self.physics
    }

    pub fn chassis_handle(&self, index: usize) -> RigidBodyHandle {
        self.chassis[index]
    }

    pub fn refresh_queries(&mut self) {
        self.query_bounds.refresh(&mut self.physics);
    }

    pub(crate) fn query_storage_bytes(&self) -> usize {
        self.query_bounds.retained_bytes()
    }

    pub fn queries<'a>(&'a self, filter: QueryFilter<'a>) -> QueryPipeline<'a> {
        QueryPipeline {
            dispatcher: self.physics.narrow_phase.query_dispatcher(),
            bvh: &self.query_bounds.bvh,
            bodies: &self.physics.bodies,
            colliders: &self.physics.colliders,
            filter,
        }
    }

    /// Player movement queries. Shape-cast normals come from contact manifolds.
    pub fn character_queries<'a>(&'a self, filter: QueryFilter<'a>) -> QueryPipeline<'a> {
        QueryPipeline {
            dispatcher: &crate::player::normals::CharacterQueryDispatcher,
            ..self.queries(filter)
        }
    }

    pub fn snapshot(&self, index: usize) -> VehicleSnapshot {
        let vehicle = &self.vehicles[index];
        let body = &self.physics.bodies[vehicle.chassis];
        let pose = body.position();
        let wheels = std::array::from_fn(|i| {
            let wheel = vehicle
                .active_slots
                .iter()
                .position(|slot| *slot == i)
                .map_or(&vehicle.resting_wheels[i], |index| {
                    &vehicle.controller.wheels()[index]
                });
            let length = if wheel.raycast_info().is_in_contact {
                wheel.raycast_info().suspension_length
            } else {
                vehicle.suspension_lengths[i]
            };
            let local = wheel.chassis_connection_point_cs + wheel.direction_cs * length;
            VehiclePose {
                position: (pose * local).to_array(),
                rotation: (pose.rotation
                    * Rotation::from_rotation_y(wheel.steering)
                    * Rotation::from_rotation_x(-wheel.rotation))
                .to_array(),
            }
        });
        VehicleSnapshot {
            pose: from_rapier(pose),
            wheels,
            speed_mps: body.linvel().dot(pose.rotation * -Vector::Z),
            linear_velocity: body.linvel().to_array(),
            angular_velocity: body.angvel().to_array(),
            wheels_contact: std::array::from_fn(|i| {
                vehicle
                    .active_slots
                    .iter()
                    .position(|slot| *slot == i)
                    .is_some_and(|index| {
                        vehicle.controller.wheels()[index]
                            .raycast_info()
                            .is_in_contact
                    })
            }),
        }
    }

    pub fn reset_preserving_wheels(&mut self, vehicle: usize, pose: VehiclePose) {
        let rotations =
            std::array::from_fn::<_, 4, _>(|index| self.wheel_save_state(vehicle, index).0);
        let previous = self.vehicles[vehicle].wheel_rotations;
        let rpm = self.vehicles[vehicle].wheel_rpm;
        self.reset(vehicle, pose);
        let state = &mut self.vehicles[vehicle];
        for (wheel, &slot) in state
            .controller
            .wheels_mut()
            .iter_mut()
            .zip(&state.active_slots)
        {
            wheel.rotation = rotations[slot];
        }
        for (wheel, rotation) in self.vehicles[vehicle]
            .resting_wheels
            .iter_mut()
            .zip(rotations)
        {
            wheel.rotation = rotation;
        }
        self.vehicles[vehicle].wheel_rotations = previous;
        self.vehicles[vehicle].wheel_rpm = rpm;
    }

    pub fn reset(&mut self, index: usize, pose: VehiclePose) {
        let vehicle = &mut self.vehicles[index];
        self.query_bounds.mark_body(vehicle.chassis);
        let body = &mut self.physics.bodies[vehicle.chassis];
        let before = BodyPresentationState::capture(body);

        body.set_position(to_rapier(pose), true);
        body.set_linvel(Vector::ZERO, true);
        body.set_angvel(Vector::ZERO, true);
        body.reset_forces(true);
        body.reset_torques(true);
        self.presentation_bodies
            .mark_if_changed(vehicle.chassis, before, body);
        for (wheel, &slot) in vehicle
            .controller
            .wheels_mut()
            .iter_mut()
            .zip(&vehicle.active_slots)
        {
            *wheel = vehicle.resting_wheels[slot];
            if let Some(installed) = vehicle.installed[slot] {
                wheel.radius = installed.radius;
            }
        }
        vehicle.controller.current_vehicle_speed = 0.0;
        vehicle.wheel_rotations = [0.0; 4];
        vehicle.wheel_rpm = [0.0; 4];
        self.refresh_queries();
    }

    pub fn insert_mesh(&mut self, vertices: &[[f32; 3]], indices: &[[u32; 3]]) -> ColliderHandle {
        self.insert_mesh_at(vertices, indices, [0.0; 3])
    }

    pub fn insert_mesh_at(
        &mut self,
        vertices: &[[f32; 3]],
        indices: &[[u32; 3]],
        position: [f32; 3],
    ) -> ColliderHandle {
        let mesh = ColliderBuilder::trimesh(
            vertices.iter().copied().map(Vector::from_array).collect(),
            indices.to_vec(),
        )
        .expect("Static mesh must contain valid triangles")
        .friction(0.8)
        .translation(Vector::from_array(position));
        let handle = self.insert_collider(mesh);
        self.refresh_queries();
        handle
    }

    /// Fallible source mesh publication leaves existing collision untouched on failure.
    pub fn insert_source_mesh(
        &mut self,
        surface: &crate::world::WorldSurface,
        position: [f32; 3],
        layer: u8,
        filter: u32,
        friction: f32,
        restitution: f32,
        available_cpu: usize,
    ) -> Result<(ColliderHandle, usize), ReferenceError> {
        // Reserve vertices, indices, BVH construction and retained collider storage before allocation.
        let reservation = surface
            .vertices
            .len()
            .saturating_mul(48)
            .saturating_add(surface.triangles.len().saturating_mul(256))
            .saturating_add(std::mem::size_of::<rapier3d::prelude::Collider>());
        if reservation > available_cpu {
            return Err(ReferenceError(format!(
                "Source collision reservation exceeds budget: {reservation}"
            )));
        }
        let mesh = ColliderBuilder::trimesh(
            surface
                .vertices
                .iter()
                .copied()
                .map(Vector::from_array)
                .collect(),
            surface.triangles.clone(),
        )
        .map_err(|error| ReferenceError(format!("Source collision mesh: {error}")))?
        .friction(friction)
        .restitution(restitution)
        .translation(Vector::from_array(position))
        .collision_groups(InteractionGroups::new(
            Group::from_bits_truncate(1u32 << layer),
            Group::from_bits_truncate(filter),
            InteractionTestMode::And,
        ))
        .build();
        let bytes = std::mem::size_of::<rapier3d::prelude::Collider>()
            + mesh
                .shape()
                .as_trimesh()
                .expect("source triangle mesh")
                .total_memory_size();
        if bytes > available_cpu {
            return Err(ReferenceError(format!(
                "Source collision allocation exceeds budget: {bytes}"
            )));
        }
        let handle = self.insert_collider(mesh);
        self.refresh_queries();
        Ok((handle, bytes))
    }

    pub fn remove_collider(&mut self, handle: ColliderHandle) {
        for vehicle in &self.vehicles {
            if vehicle
                .controller
                .wheels()
                .iter()
                .any(|wheel| wheel.raycast_info().ground_object == Some(handle))
            {
                self.query_bounds.mark_body(vehicle.chassis);
                let body = &mut self.physics.bodies[vehicle.chassis];
                let before = BodyPresentationState::capture(body);
                body.wake_up(true);
                self.presentation_bodies
                    .mark_if_changed(vehicle.chassis, before, body);
            }
        }
        if self.physics.colliders.contains(handle) {
            self.query_bounds.retire_collider(handle);
        }
        if let Some(parent) = self
            .physics
            .colliders
            .get(handle)
            .and_then(|collider| collider.parent())
        {
            self.query_bounds.mark_body(parent);
        }
        self.physics.remove_collider(handle);
        self.refresh_queries();
    }

    pub fn rebase(&mut self, shift: [f32; 3]) {
        let shift = Vector::from_array(shift);
        for (handle, body) in self.physics.bodies.iter_mut() {
            if self.inactive_bodies.contains(&handle) {
                continue;
            }
            self.query_bounds.mark_body(handle);
            let before = BodyPresentationState::capture(body);
            body.set_translation(body.translation() - shift, false);
            self.presentation_bodies
                .mark_if_changed(handle, before, body);
        }
        for (handle, collider) in self.physics.colliders.iter_mut() {
            if collider.parent().is_none() {
                self.query_bounds.mark_collider(handle, false);
                collider.set_translation(collider.translation() - shift);
            }
        }
        // Rapier refreshes private wheel ray points before applying forces next step.
        // Keep suspension lengths, wheel rotations, and velocities unchanged here.
        self.refresh_queries();
    }

    pub fn insert_box(&mut self, center: [f32; 3], half_size: [f32; 3]) -> ColliderHandle {
        let handle = self.insert_collider(
            ColliderBuilder::cuboid(half_size[0], half_size[1], half_size[2])
                .translation(Vector::from_array(center))
                .friction(0.8),
        );
        self.refresh_queries();
        handle
    }

    pub fn collider_count(&self) -> usize {
        self.physics.colliders.len()
    }
}

pub(crate) fn capture_body_save(
    body: &rapier3d::prelude::RigidBody,
    identity: crate::save::BodyIdentity,
    origin: &crate::world::WorldOrigin,
) -> crate::save::BodySnapshot {
    let pose = from_rapier(body.position());
    crate::save::BodySnapshot {
        identity,
        pose: crate::save::AbsolutePose {
            position: origin.absolute(pose.position),
            rotation: pose.rotation,
        },
        linear_velocity: body.linvel().to_array(),
        angular_velocity: body.angvel().to_array(),
        sleeping: body.is_sleeping(),
    }
}

pub fn collider_builder(geometry: &ColliderGeometry) -> Result<ColliderBuilder, ReferenceError> {
    match geometry {
        ColliderGeometry::Box {
            center,
            half_extents,
            rotation,
        } => Ok(
            ColliderBuilder::cuboid(half_extents[0], half_extents[1], half_extents[2]).position(
                to_rapier(VehiclePose {
                    position: *center,
                    rotation: *rotation,
                }),
            ),
        ),
        ColliderGeometry::Sphere { center, radius } => {
            Ok(ColliderBuilder::ball(*radius).translation(Vector::from_array(*center)))
        }
        ColliderGeometry::Capsule { a, b, radius } => Ok(ColliderBuilder::capsule_from_endpoints(
            Vector::from_array(*a),
            Vector::from_array(*b),
            *radius,
        )),
        ColliderGeometry::Convex { points } => {
            let points: Vec<_> = points.iter().copied().map(Vector::from_array).collect();
            ColliderBuilder::convex_hull(&points)
                .ok_or_else(|| ReferenceError("Invalid convex collision geometry".into()))
        }
        ColliderGeometry::Trimesh { points, triangles } => ColliderBuilder::trimesh(
            points.iter().copied().map(Vector::from_array).collect(),
            triangles.clone(),
        )
        .map_err(|error| ReferenceError(format!("Invalid collision mesh: {error}"))),
    }
}

pub fn to_rapier(pose: VehiclePose) -> Pose {
    Pose::from_parts(
        Vector::from_array(pose.position),
        Rotation::from_array(pose.rotation),
    )
}

pub fn from_rapier(pose: &Pose) -> VehiclePose {
    VehiclePose {
        position: pose.translation.to_array(),
        rotation: pose.rotation.to_array(),
    }
}
