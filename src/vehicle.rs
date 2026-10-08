use rapier3d::control::{DynamicRayCastVehicleController, Wheel, WheelTuning};
use rapier3d::math::{Pose, Rotation, Vector};
pub use rapier3d::prelude::ColliderHandle;
use rapier3d::prelude::{ColliderBuilder, QueryFilter, RigidBodyBuilder, RigidBodyHandle};

use crate::simulation::{DriveInput, FIXED_DT};

pub const WHEEL_RADIUS: f32 = 0.32;
pub const WHEEL_CONNECTIONS: [[f32; 3]; 4] = [
    [-0.78, -0.1, -1.12],
    [0.78, -0.1, -1.12],
    [-0.78, -0.1, 1.12],
    [0.78, -0.1, 1.12],
];
const SUSPENSION_LENGTH: f32 = 0.34;
const MASS_KG: f32 = 900.0;

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

pub struct PhysicsWorld {
    physics: rapier3d::prelude::PhysicsWorld,
    chassis: RigidBodyHandle,
    controller: DynamicRayCastVehicleController,
    resting_wheels: [Wheel; 4],
}

impl PhysicsWorld {
    pub fn new(pose: VehiclePose) -> Self {
        let mut physics = rapier3d::prelude::PhysicsWorld::new();
        physics.integration_parameters.dt = FIXED_DT;
        let (chassis, _) = physics.insert(
            RigidBodyBuilder::dynamic()
                .pose(to_rapier(pose))
                .linear_damping(0.06)
                .angular_damping(0.5)
                .ccd_enabled(true)
                .can_sleep(false),
            ColliderBuilder::cuboid(0.76, 0.24, 1.55)
                .mass(MASS_KG)
                .friction(0.4),
        );
        physics.colliders.insert_with_parent(
            ColliderBuilder::cuboid(0.65, 0.36, 0.72)
                .translation(Vector::new(0.0, 0.57, 0.2))
                .mass(0.0),
            chassis,
            &mut physics.bodies,
        );
        let mut controller = DynamicRayCastVehicleController::new(chassis);
        controller.index_forward_axis = 2;
        let tuning = WheelTuning {
            suspension_stiffness: 32.0,
            suspension_compression: 2.8,
            suspension_damping: 3.2,
            max_suspension_travel: 0.22,
            friction_slip: 2.0,
            max_suspension_force: 12_000.0,
            ..Default::default()
        };
        for connection in WHEEL_CONNECTIONS {
            controller.add_wheel(
                Vector::from_array(connection),
                -Vector::Y,
                Vector::X,
                SUSPENSION_LENGTH,
                WHEEL_RADIUS,
                &tuning,
            );
        }
        let resting_wheels = std::array::from_fn(|i| controller.wheels()[i]);
        physics.detect_collisions(&(), &());
        // Reserve Rapier's four-wheel scratch buffers before the frame loop.
        controller.update_vehicle(
            FIXED_DT,
            physics.broad_phase.as_query_pipeline_mut(
                physics.narrow_phase.query_dispatcher(),
                &mut physics.bodies,
                &mut physics.colliders,
                QueryFilter::default().exclude_rigid_body(chassis),
            ),
        );
        controller.wheels_mut().copy_from_slice(&resting_wheels);
        Self {
            physics,
            chassis,
            controller,
            resting_wheels,
        }
    }

    pub fn step(&mut self, input: DriveInput, engine_enabled: bool) {
        let chassis = &self.physics.bodies[self.chassis];
        let speed = chassis
            .linvel()
            .dot(chassis.position().rotation * -Vector::Z);
        let steering_limit = 0.46 / (1.0 + speed.abs() * 0.055);
        let steering = if input.steer.is_finite() {
            input.steer.clamp(-1.0, 1.0) * steering_limit
        } else {
            0.0
        };
        let braking = input.brake_reverse && speed > 0.4;
        let engine_force = if !engine_enabled || braking || input.handbrake {
            0.0
        } else if input.brake_reverse {
            if speed > -8.0 { -1_600.0 } else { 0.0 }
        } else if input.accelerate && speed < 42.0 {
            2_600.0
        } else {
            0.0
        };
        for (i, wheel) in self.controller.wheels_mut().iter_mut().enumerate() {
            wheel.steering = if i < 2 { steering } else { 0.0 };
            wheel.engine_force = if i >= 2 { engine_force } else { 0.0 };
            wheel.brake = if input.handbrake && i >= 2 {
                55.0
            } else if braking {
                35.0
            } else {
                2.0
            };
            wheel.friction_slip = if input.handbrake && i >= 2 { 0.8 } else { 2.0 };
        }
        self.controller.update_vehicle(
            FIXED_DT,
            self.physics.broad_phase.as_query_pipeline_mut(
                self.physics.narrow_phase.query_dispatcher(),
                &mut self.physics.bodies,
                &mut self.physics.colliders,
                QueryFilter::default().exclude_rigid_body(self.chassis),
            ),
        );
        self.physics.step();
    }

    pub fn snapshot(&self) -> VehicleSnapshot {
        let body = &self.physics.bodies[self.chassis];
        let pose = body.position();
        let wheels = std::array::from_fn(|i| {
            let wheel = &self.controller.wheels()[i];
            let length = if wheel.raycast_info().is_in_contact {
                wheel.raycast_info().suspension_length
            } else {
                SUSPENSION_LENGTH
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
                self.controller.wheels()[i].raycast_info().is_in_contact
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
        self.controller
            .wheels_mut()
            .copy_from_slice(&self.resting_wheels);
        self.controller.current_vehicle_speed = 0.0;
        self.physics.detect_collisions(&(), &());
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
        self.physics.detect_collisions(&(), &());
        handle
    }

    pub fn remove_collider(&mut self, handle: ColliderHandle) {
        self.physics.remove_collider(handle);
        self.physics.detect_collisions(&(), &());
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
        self.physics.detect_collisions(&(), &());
    }

    pub fn insert_box(&mut self, center: [f32; 3], half_size: [f32; 3]) -> ColliderHandle {
        let handle = self.physics.colliders.insert(
            ColliderBuilder::cuboid(half_size[0], half_size[1], half_size[2])
                .translation(Vector::from_array(center))
                .friction(0.8),
        );
        self.physics.detect_collisions(&(), &());
        handle
    }

    pub fn collider_count(&self) -> usize {
        self.physics.colliders.len()
    }
}

fn to_rapier(pose: VehiclePose) -> Pose {
    Pose::from_parts(
        Vector::from_array(pose.position),
        Rotation::from_array(pose.rotation),
    )
}

fn from_rapier(pose: &Pose) -> VehiclePose {
    VehiclePose {
        position: pose.translation.to_array(),
        rotation: pose.rotation.to_array(),
    }
}
