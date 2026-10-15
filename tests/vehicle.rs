mod common;

// The fixture axle has two driven wheels with a 0.3 m radius.

use my_summer_slop::simulation::DriveInput;
use my_summer_slop::vehicle::{DriveTorques, PhysicsWorld, VehicleSnapshot};

fn flat_track() -> PhysicsWorld {
    let mut world = common::world();
    world.insert_mesh(
        &[
            [-500.0, 0.0, -500.0],
            [500.0, 0.0, -500.0],
            [500.0, 0.0, 500.0],
            [-500.0, 0.0, 500.0],
        ],
        &[[0, 2, 1], [0, 3, 2]],
    );
    for _ in 0..120 {
        world.apply_vehicle_forces(0, DriveInput::default(), Default::default());
        world.step_physics();
    }
    world
}

fn finite(snapshot: VehicleSnapshot) {
    for pose in std::iter::once(snapshot.pose).chain(snapshot.wheels) {
        assert!(
            pose.position
                .into_iter()
                .chain(pose.rotation)
                .all(f32::is_finite)
        );
    }
    assert!(snapshot.speed_mps.is_finite());
}

#[test]
fn removed_slot_has_no_raycast_support_and_restores_without_duplicates() {
    use my_summer_slop::vehicle::InstalledWheel;
    let reference = common::reference();
    let mut world = flat_track();
    let slot = reference.vehicles[0].wheel_slots[0];
    world.set_wheel(slot, None).unwrap();
    assert_eq!(world.wheel_support_count(0), 3);
    for _ in 0..30 {
        world.apply_vehicle_forces(0, DriveInput::default(), Default::default());
        world.step_physics();
    }
    assert!(!world.snapshot(0).wheels_contact[0]);
    world
        .set_wheel(
            slot,
            Some(InstalledWheel {
                radius: reference
                    .wheel_slots
                    .iter()
                    .find(|wheel| wheel.id == slot)
                    .unwrap()
                    .radius,
                condition: 0.5,
            }),
        )
        .unwrap();
    assert_eq!(world.wheel_support_count(0), 4);
    for _ in 0..120 {
        world.apply_vehicle_forces(0, DriveInput::default(), Default::default());
        world.step_physics();
    }
    assert!(world.snapshot(0).wheels_contact[0]);
    world.rebase([128.0, 0.0, -128.0]);
    world.apply_vehicle_forces(0, DriveInput::default(), Default::default());
    world.step_physics();
    assert!(world.snapshot(0).wheels_contact[0]);
}

#[test]
fn accelerates_brakes_and_reverses() {
    let mut world = flat_track();
    for _ in 0..180 {
        world.apply_vehicle_forces(
            0,
            DriveInput {
                throttle: 1.0,
                ..Default::default()
            },
            DriveTorques {
                motor_nm: 2600.0 * 2.0 * 0.3,
                ..Default::default()
            },
        );
        world.step_physics();
    }
    let driving = world.snapshot(0);
    finite(driving);
    assert!(driving.speed_mps > 5.0, "{driving:?}");
    assert!(driving.pose.position[2] < -5.0);
    for _ in 0..60 {
        world.apply_vehicle_forces(
            0,
            DriveInput {
                brake: 1.0,
                ..Default::default()
            },
            Default::default(),
        );
        world.step_physics();
    }
    assert!(world.snapshot(0).speed_mps < driving.speed_mps * 0.5);
    for _ in 0..180 {
        world.apply_vehicle_forces(
            0,
            DriveInput {
                throttle: 1.0,
                ..Default::default()
            },
            DriveTorques {
                motor_nm: -2600.0 * 2.0 * 0.3,
                ..Default::default()
            },
        );
        world.step_physics();
    }
    let reversing = world.snapshot(0);
    finite(reversing);
    assert!(reversing.speed_mps < -1.0, "{reversing:?}");
}

#[test]
fn chassis_collides_with_static_wall() {
    let mut world = flat_track();
    world.insert_mesh(
        &[
            [-10.0, 0.0, -15.0],
            [10.0, 0.0, -15.0],
            [10.0, 3.0, -15.0],
            [-10.0, 3.0, -15.0],
        ],
        &[[0, 1, 2], [0, 2, 3]],
    );
    for _ in 0..240 {
        world.apply_vehicle_forces(
            0,
            DriveInput {
                throttle: 1.0,
                ..Default::default()
            },
            DriveTorques {
                motor_nm: 2600.0 * 2.0 * 0.3,
                ..Default::default()
            },
        );
        world.step_physics();
    }
    let snapshot = world.snapshot(0);
    finite(snapshot);
    assert!(snapshot.pose.position[2] > -14.0, "{snapshot:?}");
    assert!(snapshot.speed_mps.abs() < 1.0);
}

#[test]
fn signed_axle_torque_drives_without_second_pedal_scale() {
    for sign in [-1.0, 1.0] {
        let mut low = flat_track();
        let mut high = flat_track();
        for _ in 0..30 {
            low.apply_vehicle_forces(
                0,
                DriveInput {
                    throttle: 0.25,
                    ..Default::default()
                },
                DriveTorques {
                    motor_nm: sign * 1500.0,
                    engine_brake_nm: 0.0,
                },
            );
            high.apply_vehicle_forces(
                0,
                DriveInput {
                    throttle: 1.0,
                    ..Default::default()
                },
                DriveTorques {
                    motor_nm: sign * 1500.0,
                    engine_brake_nm: 0.0,
                },
            );
            low.step_physics();
            high.step_physics();
        }
        let a = low.snapshot(0);
        let b = high.snapshot(0);
        assert_eq!(a.pose, b.pose);
        assert!(a.speed_mps * sign > 0.5, "{a:?}");
    }
}
