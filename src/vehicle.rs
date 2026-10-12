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

#[derive(Clone, Copy)]
pub struct InstalledWheel {
    pub radius: f32,
    pub condition: f32,
}
pub const VEHICLE_COLLIDER_FLAG: u128 = 1 << 127;
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

pub struct PhysicsWorld {
    physics: rapier3d::prelude::PhysicsWorld,
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
    auto_move_min: f32,
    engine_brake_force: f32,
    last_handbrake: bool,
}

impl PhysicsWorld {
    pub fn new(reference: &ReferenceManifest, pose: VehiclePose) -> Result<Self, ReferenceError> {
        let vehicle = &reference.vehicle;
        let source_body = reference
            .bodies
            .iter()
            .find(|body| body.id == vehicle.body)
            .ok_or_else(|| ReferenceError("Missing chassis body".into()))?;
        let mut physics = rapier3d::prelude::PhysicsWorld::new();
        physics.integration_parameters.dt = FIXED_DT;
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
                .collision_groups(InteractionGroups::new(
                    Group::from_bits_truncate(collider.membership),
                    Group::from_bits_truncate(collider.filter),
                    InteractionTestMode::And,
                ))
                .density(if collider.sensor { 0.0 } else { 1.0 })
                .user_data(VEHICLE_COLLIDER_FLAG | ((id.0 as u128) << 64));
            physics
                .colliders
                .insert_with_parent(builder, chassis, &mut physics.bodies);
        }
        physics.bodies[chassis].recompute_mass_properties_from_colliders(&physics.colliders);
        let geometry_mass = physics.bodies[chassis].mass();
        if geometry_mass <= 0.0 || geometry_mass >= vehicle.mass_kg {
            return Err(ReferenceError(
                "Invalid car collision volume or mass".into(),
            ));
        }
        physics.bodies[chassis].set_additional_mass(vehicle.mass_kg - geometry_mass, true);
        physics.bodies[chassis].recompute_mass_properties_from_colliders(&physics.colliders);
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
                friction_slip: wheel.sideways_friction[4],
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
        refresh_world_queries(&mut physics);
        // Reserve Rapier's four-wheel scratch buffers before the frame loop.
        controller.update_vehicle(
            FIXED_DT,
            physics.broad_phase.as_query_pipeline_mut(
                physics.narrow_phase.query_dispatcher(),
                &mut physics.bodies,
                &mut physics.colliders,
                QueryFilter::default()
                    .exclude_sensors()
                    .exclude_rigid_body(chassis),
            ),
        );
        controller.wheels_mut().copy_from_slice(&resting_wheels);
        Ok(Self {
            physics,
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
            auto_move_min: vehicle.auto_move_min,
            engine_brake_force: 0.0,
            last_handbrake: true,
        })
    }

    pub fn apply_vehicle_forces(&mut self, input: DriveInput, drive_force: f32) {
        if input.accelerate
            || input.brake_reverse
            || input.steer != 0.0
            || input.handbrake != self.last_handbrake
        {
            self.physics.bodies[self.chassis].wake_up(true);
        }
        self.last_handbrake = input.handbrake;
        if self.physics.bodies[self.chassis].is_sleeping() {
            return;
        }
        let chassis = &self.physics.bodies[self.chassis];
        let speed = chassis
            .linvel()
            .dot(chassis.position().rotation * -Vector::Z);
        let steering_limit = self.max_steer;
        let steering = if input.steer.is_finite() {
            input.steer.clamp(-1.0, 1.0) * steering_limit
        } else {
            0.0
        };
        let direction = speed * chassis.linvel().length();
        let braking = crate::simulation::braking(input, direction, self.auto_move_min);
        let drive_force = if drive_force.is_finite() {
            drive_force.max(0.0)
        } else {
            0.0
        };
        let engine_force = if braking || input.handbrake {
            0.0
        } else if input.brake_reverse {
            -drive_force
        } else if input.accelerate {
            drive_force
        } else {
            0.0
        };
        for (i, wheel) in self.controller.wheels_mut().iter_mut().enumerate() {
            let slot = self.active_slots[i];
            let reference = &self.wheel_references[slot];
            wheel.steering = if reference.steer { steering } else { 0.0 };
            wheel.engine_force = if reference.drive {
                engine_force * reference.radius / wheel.radius
            } else {
                0.0
            };
            wheel.brake = if input.handbrake && reference.handbrake {
                self.handbrake_torque / wheel.radius * FIXED_DT
            } else if braking && reference.brake {
                self.brake_torque / wheel.radius * FIXED_DT
            } else if reference.drive {
                self.engine_brake_force * reference.radius / wheel.radius * FIXED_DT
            } else {
                0.0
            };
            wheel.friction_slip =
                0.75 + 0.5 * self.installed[slot].unwrap().condition.clamp(0.0, 1.0);
        }
        let not_vehicle = |_: ColliderHandle, collider: &rapier3d::prelude::Collider| {
            collider.user_data & VEHICLE_COLLIDER_FLAG == 0
        };
        self.controller.update_vehicle(
            FIXED_DT,
            self.physics.broad_phase.as_query_pipeline_mut(
                self.physics.narrow_phase.query_dispatcher(),
                &mut self.physics.bodies,
                &mut self.physics.colliders,
                QueryFilter::default()
                    .exclude_sensors()
                    .exclude_rigid_body(self.chassis)
                    .predicate(&not_vehicle),
            ),
        );
        for (wheel, &slot) in self.controller.wheels().iter().zip(&self.active_slots) {
            self.wheel_rpm[slot] = (wheel.rotation - self.wheel_rotations[slot]) / FIXED_DT * 60.0
                / std::f32::consts::TAU;
            self.wheel_rotations[slot] = wheel.rotation;
        }
    }

    pub fn step_physics(&mut self) {
        self.physics.step();
    }

    pub fn wheel_support_count(&self) -> usize {
        self.active_slots.len()
    }

    pub fn set_engine_brake(&mut self, force: f32) {
        self.engine_brake_force = if force.is_finite() {
            force.max(0.0)
        } else {
            0.0
        };
    }

    pub fn driven_wheel_rpm(&self) -> f32 {
        let mut sum = 0.0;
        let mut count = 0;
        for &slot in &self.active_slots {
            if self.wheel_references[slot].drive {
                sum += self.wheel_rpm[slot];
                count += 1;
            }
        }
        sum / count.max(1) as f32
    }

    pub fn driven_wheel_count(&self) -> usize {
        self.active_slots
            .iter()
            .filter(|&&slot| self.wheel_references[slot].drive)
            .count()
    }

    pub fn set_vehicle_mass(&mut self, mass_kg: f32) {
        let geometry_mass: f32 = self.physics.bodies[self.chassis]
            .colliders()
            .iter()
            .map(|handle| self.physics.colliders[*handle].mass())
            .sum();
        let body = &mut self.physics.bodies[self.chassis];
        body.set_additional_mass((mass_kg - geometry_mass).max(0.0), true);
        body.recompute_mass_properties_from_colliders(&self.physics.colliders);
        let mass = body.mass();
        for (wheel, reference) in self.resting_wheels.iter_mut().zip(&self.wheel_references) {
            wheel.suspension_stiffness = reference.spring / mass;
            wheel.damping_compression = reference.damper / mass;
            wheel.damping_relaxation = reference.damper / mass;
        }
        for (wheel, &slot) in self
            .controller
            .wheels_mut()
            .iter_mut()
            .zip(&self.active_slots)
        {
            let source = &self.resting_wheels[slot];
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
        let index = self
            .wheel_references
            .iter()
            .position(|reference| reference.id == slot)
            .ok_or_else(|| ReferenceError("Unknown wheel slot".into()))?;
        let mut saved = self.resting_wheels;
        for (wheel, &slot) in self.controller.wheels().iter().zip(&self.active_slots) {
            saved[slot] = *wheel;
        }
        self.installed[index] = installed;
        self.wheel_rpm[index] = 0.0;
        let mut controller = DynamicRayCastVehicleController::new(self.chassis);
        controller.index_forward_axis = 2;
        self.active_slots.clear();
        for (slot, installed) in self.installed.iter().enumerate() {
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
            self.active_slots.push(slot);
        }
        self.controller = controller;
        self.physics.bodies[self.chassis].wake_up(true);
        Ok(())
    }

    pub fn rapier(&self) -> &rapier3d::prelude::PhysicsWorld {
        &self.physics
    }

    pub fn rapier_mut(&mut self) -> &mut rapier3d::prelude::PhysicsWorld {
        &mut self.physics
    }

    pub fn chassis_handle(&self) -> RigidBodyHandle {
        self.chassis
    }

    pub fn refresh_queries(&mut self) {
        refresh_world_queries(&mut self.physics);
    }

    pub fn queries<'a>(&'a self, filter: QueryFilter<'a>) -> QueryPipeline<'a> {
        self.physics.broad_phase.as_query_pipeline(
            self.physics.narrow_phase.query_dispatcher(),
            &self.physics.bodies,
            &self.physics.colliders,
            filter,
        )
    }

    pub fn queries_mut<'a>(&'a mut self, filter: QueryFilter<'a>) -> QueryPipelineMut<'a> {
        self.physics.broad_phase.as_query_pipeline_mut(
            self.physics.narrow_phase.query_dispatcher(),
            &mut self.physics.bodies,
            &mut self.physics.colliders,
            filter,
        )
    }

    pub fn snapshot(&self) -> VehicleSnapshot {
        let body = &self.physics.bodies[self.chassis];
        let pose = body.position();
        let wheels = std::array::from_fn(|i| {
            let wheel = self
                .active_slots
                .iter()
                .position(|slot| *slot == i)
                .map_or(&self.resting_wheels[i], |index| {
                    &self.controller.wheels()[index]
                });
            let length = if wheel.raycast_info().is_in_contact {
                wheel.raycast_info().suspension_length
            } else {
                self.suspension_lengths[i]
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
                self.active_slots
                    .iter()
                    .position(|slot| *slot == i)
                    .is_some_and(|index| {
                        self.controller.wheels()[index].raycast_info().is_in_contact
                    })
            }),
        }
    }

    pub fn reset(&mut self, pose: VehiclePose) {
        let body = &mut self.physics.bodies[self.chassis];
        body.set_position(to_rapier(pose), true);
        body.set_linvel(Vector::ZERO, true);
        body.set_angvel(Vector::ZERO, true);
        body.reset_forces(true);
        body.reset_torques(true);
        for (wheel, &slot) in self
            .controller
            .wheels_mut()
            .iter_mut()
            .zip(&self.active_slots)
        {
            *wheel = self.resting_wheels[slot];
            if let Some(installed) = self.installed[slot] {
                wheel.radius = installed.radius;
            }
        }
        self.controller.current_vehicle_speed = 0.0;
        self.wheel_rotations = [0.0; 4];
        self.wheel_rpm = [0.0; 4];
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
        let handle = self.physics.colliders.insert(mesh);
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
    ) -> Result<ColliderHandle, ReferenceError> {
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
        ));
        let handle = self.physics.colliders.insert(mesh);
        self.refresh_queries();
        Ok(handle)
    }

    pub fn remove_collider(&mut self, handle: ColliderHandle) {
        if self
            .controller
            .wheels()
            .iter()
            .any(|wheel| wheel.raycast_info().ground_object == Some(handle))
        {
            self.physics.bodies[self.chassis].wake_up(true);
        }
        self.physics.remove_collider(handle);
        self.refresh_queries();
    }

    pub fn rebase(&mut self, shift: [f32; 3]) {
        let shift = Vector::from_array(shift);
        for (_, body) in self.physics.bodies.iter_mut() {
            body.set_translation(body.translation() - shift, false);
        }
        for (_, collider) in self.physics.colliders.iter_mut() {
            if collider.parent().is_none() {
                collider.set_translation(collider.translation() - shift);
            }
        }
        // Rapier refreshes private wheel ray points before applying forces next step.
        // Keep suspension lengths, wheel rotations, and velocities unchanged here.
        self.refresh_queries();
    }

    pub fn insert_box(&mut self, center: [f32; 3], half_size: [f32; 3]) -> ColliderHandle {
        let handle = self.physics.colliders.insert(
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

/// Returns force per driven wheel. The gear index owns forward/reverse selection.
pub fn drive_force(
    output: crate::simulation::engine::EngineOutput,
    vehicle: &VehicleReference,
    gear_index: usize,
    radius: f32,
    drive_count: usize,
) -> f32 {
    if !output.running {
        return 0.0;
    }
    let Some(ratio) = vehicle.gear_ratios.get(gear_index) else {
        return 0.0;
    };
    if radius <= 0.0 || drive_count == 0 {
        return 0.0;
    }
    (output.torque_nm * ratio.abs() * vehicle.final_drive_ratio / (drive_count as f32 * radius))
        .max(0.0)
}

pub fn select_gear(
    wheel_rpm: f32,
    reverse: bool,
    throttle: bool,
    previous: usize,
    engine: &crate::reference::EngineReference,
    vehicle: &VehicleReference,
) -> usize {
    let neutral = vehicle
        .gear_ratios
        .iter()
        .position(|ratio| *ratio == 0.0)
        .expect("validated neutral gear");
    if !throttle {
        return neutral;
    }
    let mut selected = previous;
    let mut maximum = 0.0;
    for (index, ratio) in vehicle.gear_ratios.iter().enumerate().rev() {
        if *ratio == 0.0 || (*ratio < 0.0) != reverse {
            continue;
        }
        let rpm = wheel_rpm.abs() * vehicle.final_drive_ratio * ratio.abs();
        let mut value = crate::simulation::engine::torque(&engine.torque_curve, rpm);
        if value == 0.0 {
            value = crate::simulation::engine::torque(
                &engine.torque_curve,
                engine.max_rpm * 0.5 * vehicle.final_drive_ratio * ratio.abs(),
            );
        }
        if value > maximum {
            maximum = value;
            selected = index;
        }
    }
    selected
}

fn refresh_world_queries(physics: &mut rapier3d::prelude::PhysicsWorld) {
    // CollisionPipeline consumes body changes before PhysicsPipeline registers islands.
    // Update query bounds while retaining those changes for the next fixed step.
    physics
        .bodies
        .propagate_modified_body_positions_to_colliders(&mut physics.colliders);
    for (handle, collider) in physics.colliders.iter() {
        if collider.is_enabled() {
            physics.broad_phase.set_aabb(
                &physics.integration_parameters,
                handle,
                collider.compute_aabb(),
            );
        }
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
