mod common;

// The fixture axle has two driven wheels with a 0.3 m radius.

use my_summer_slop::simulation::DriveInput;
use my_summer_slop::vehicle::{DriveTorques, PhysicsWorld, VehiclePose};
use my_summer_slop::world::WorldOrigin;

/// These streaming tests drive one car.
fn lada_only() -> my_summer_slop::reference::ReferenceManifest {
    let mut reference = common::reference();
    reference.vehicles.truncate(1);
    reference
}

#[test]
fn rebase_preserves_wheels_velocities_and_absolute_position() {
    let mut physics = PhysicsWorld::new(
        &lada_only(),
        &[VehiclePose {
            position: [0.0, 1.1, 0.0],
            rotation: [0.0, 0.0, 0.0, 1.0],
        }],
    )
    .expect("fixture physics");
    physics.insert_mesh(
        &[
            [-500.0, 0.0, -500.0],
            [500.0, 0.0, -500.0],
            [500.0, 0.0, 500.0],
            [-500.0, 0.0, 500.0],
        ],
        &[[0, 2, 1], [0, 3, 2]],
    );
    for _ in 0..180 {
        physics.apply_vehicle_forces(
            0,
            DriveInput {
                throttle: 1.0,
                steer: 0.05,
                ..Default::default()
            },
            DriveTorques {
                motor_nm: 2600.0 * 2.0 * 0.3,
                ..Default::default()
            },
        );
        physics.step_physics();
    }
    let before = physics.snapshot(0);
    let mut origin = WorldOrigin::default();
    let absolute = origin.absolute(before.pose.position);
    let shift = [0.0, 0.0, -4096.0];
    physics.rebase(shift);
    origin.shift(shift);
    let after = physics.snapshot(0);
    assert!((after.pose.position[2] - before.pose.position[2] - 4096.0).abs() < 0.001);
    assert_eq!(before.linear_velocity, after.linear_velocity);
    assert_eq!(before.angular_velocity, after.angular_velocity);
    assert_eq!(before.pose.rotation, after.pose.rotation);
    for (p, q) in origin
        .absolute(after.pose.position)
        .into_iter()
        .zip(absolute)
    {
        assert!((p - q).abs() < 0.001);
    }
    for i in 0..4 {
        assert_eq!(before.wheels[i].rotation, after.wheels[i].rotation);
        for axis in 0..3 {
            let relative_before = before.wheels[i].position[axis] - before.pose.position[axis];
            let relative_after = after.wheels[i].position[axis] - after.pose.position[axis];
            assert!((relative_before - relative_after).abs() < 0.001);
        }
    }
    physics.apply_vehicle_forces(0, DriveInput::default(), Default::default());
    physics.step_physics();
    assert!(physics.snapshot(0).wheels_contact.into_iter().all(|c| c));
    assert!(physics.snapshot(0).speed_mps > 1.0);
}
