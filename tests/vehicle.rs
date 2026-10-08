use desert_drive::simulation::{DriveInput, StepClock};
use desert_drive::vehicle::{PhysicsWorld, VehiclePose, VehicleSnapshot};

const START: VehiclePose = VehiclePose {
    position: [0.0, 1.1, 0.0],
    rotation: [0.0, 0.0, 0.0, 1.0],
};

fn flat_track() -> PhysicsWorld {
    let mut world = PhysicsWorld::new(START);
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
        world.step(DriveInput::default(), true);
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
fn settles_on_four_wheels() {
    let mut world = flat_track();
    let snapshot = world.snapshot();
    finite(snapshot);
    assert!(snapshot.wheels_contact.into_iter().all(|contact| contact));
    assert!(
        (0.4..1.0).contains(&snapshot.pose.position[1]),
        "{snapshot:?}"
    );
    assert!(snapshot.linear_velocity[1].abs() < 0.1, "{snapshot:?}");
    for _ in 0..120 {
        world.step(DriveInput::default(), true);
    }
    assert!((world.snapshot().pose.position[1] - snapshot.pose.position[1]).abs() < 0.01);
}

#[test]
fn accelerates_brakes_and_reverses() {
    let mut world = flat_track();
    for _ in 0..180 {
        world.step(
            DriveInput {
                accelerate: true,
                ..Default::default()
            },
            true,
        );
    }
    let driving = world.snapshot();
    finite(driving);
    assert!(driving.speed_mps > 5.0, "{driving:?}");
    assert!(driving.pose.position[2] < -5.0);
    for _ in 0..60 {
        world.step(
            DriveInput {
                brake_reverse: true,
                ..Default::default()
            },
            true,
        );
    }
    assert!(world.snapshot().speed_mps < driving.speed_mps * 0.5);
    for _ in 0..180 {
        world.step(
            DriveInput {
                brake_reverse: true,
                ..Default::default()
            },
            true,
        );
    }
    let reversing = world.snapshot();
    finite(reversing);
    assert!(reversing.speed_mps < -1.0, "{reversing:?}");
}

#[test]
fn reset_stops_the_chassis() {
    let mut world = flat_track();
    let start = world.snapshot().pose;
    let input = DriveInput {
        accelerate: true,
        ..DriveInput::default()
    };
    for _ in 0..180 {
        world.step(input, true);
    }
    assert!(world.snapshot().speed_mps.abs() > 1.0);
    world.reset(start);
    let snapshot = world.snapshot();
    assert_eq!(snapshot.pose, start);
    assert_eq!(snapshot.linear_velocity, [0.0; 3]);
    assert_eq!(snapshot.angular_velocity, [0.0; 3]);
    assert_eq!(snapshot.speed_mps, 0.0);
    assert_eq!(snapshot.wheels_contact, [false; 4]);
    finite(snapshot);
}

#[test]
fn disabled_engine_produces_no_thrust() {
    let mut world = flat_track();
    for _ in 0..180 {
        world.step(
            DriveInput {
                accelerate: true,
                ..Default::default()
            },
            false,
        );
    }
    assert!(world.snapshot().speed_mps.abs() < 0.1);
}

#[test]
fn handbrake_slows_the_car() {
    let mut world = flat_track();
    for _ in 0..120 {
        world.step(
            DriveInput {
                accelerate: true,
                ..Default::default()
            },
            true,
        );
    }
    let before = world.snapshot().speed_mps;
    for _ in 0..60 {
        world.step(
            DriveInput {
                handbrake: true,
                ..Default::default()
            },
            true,
        );
    }
    assert!(world.snapshot().speed_mps.abs() < before * 0.5);
    finite(world.snapshot());
}

#[test]
fn removed_ground_cannot_support_wheels() {
    let mut world = PhysicsWorld::new(START);
    let ground = world.insert_mesh(
        &[
            [-10.0, 0.0, -10.0],
            [10.0, 0.0, -10.0],
            [10.0, 0.0, 10.0],
            [-10.0, 0.0, 10.0],
        ],
        &[[0, 2, 1], [0, 3, 2]],
    );
    for _ in 0..120 {
        world.step(DriveInput::default(), true);
    }
    world.remove_collider(ground);
    for _ in 0..60 {
        world.step(DriveInput::default(), true);
    }
    assert_eq!(world.snapshot().wheels_contact, [false; 4]);
    assert!(world.snapshot().pose.position[1] < -1.0);
}

#[test]
fn one_second_stall_advances_at_most_six_steps() {
    let mut clock = StepClock::default();
    assert_eq!(clock.advance(1.0), 6);
    assert!((0.0..1.0).contains(&clock.alpha()));
    assert_eq!(clock.advance(0.0), 0);
}

#[test]
fn steering_turns_the_car_left() {
    let mut world = flat_track();
    for _ in 0..120 {
        world.step(
            DriveInput {
                accelerate: true,
                steer: 1.0,
                ..Default::default()
            },
            true,
        );
    }
    let snapshot = world.snapshot();
    finite(snapshot);
    assert!(snapshot.pose.position[0] < -1.0, "{snapshot:?}");
    assert!(snapshot.wheels_contact.into_iter().all(|contact| contact));
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
        world.step(
            DriveInput {
                accelerate: true,
                ..Default::default()
            },
            true,
        );
    }
    let snapshot = world.snapshot();
    finite(snapshot);
    assert!(snapshot.pose.position[2] > -14.0, "{snapshot:?}");
    assert!(snapshot.speed_mps.abs() < 1.0);
}
