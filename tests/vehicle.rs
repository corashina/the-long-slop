mod common;

use my_summer_slop::simulation::{DriveInput, StepClock};
use my_summer_slop::vehicle::{PhysicsWorld, VehicleSnapshot};

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
        world.apply_vehicle_forces(0, DriveInput::default(), 2600.0);
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
fn settles_on_four_wheels() {
    let mut world = flat_track();
    let snapshot = world.snapshot(0);
    finite(snapshot);
    assert!(snapshot.wheels_contact.into_iter().all(|contact| contact));
    assert!(
        (0.4..1.0).contains(&snapshot.pose.position[1]),
        "{snapshot:?}"
    );
    assert!(snapshot.linear_velocity[1].abs() < 0.1, "{snapshot:?}");
    for _ in 0..120 {
        world.apply_vehicle_forces(0, DriveInput::default(), 2600.0);
        world.step_physics();
    }
    assert!((world.snapshot(0).pose.position[1] - snapshot.pose.position[1]).abs() < 0.01);
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
        world.apply_vehicle_forces(0, DriveInput::default(), 0.0);
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
        world.apply_vehicle_forces(0, DriveInput::default(), 0.0);
        world.step_physics();
    }
    assert!(world.snapshot(0).wheels_contact[0]);
    world.rebase([128.0, 0.0, -128.0]);
    world.apply_vehicle_forces(0, DriveInput::default(), 0.0);
    world.step_physics();
    assert!(world.snapshot(0).wheels_contact[0]);
}

#[test]
fn source_springs_keep_their_force_when_vehicle_mass_changes() {
    let reference = common::reference();
    let mut world = flat_track();
    let before = world.snapshot(0).pose.position[1];
    world.set_vehicle_mass(0, reference.vehicles[0].mass_kg * 0.5);
    for _ in 0..180 {
        world.apply_vehicle_forces(0, DriveInput::default(), 0.0);
        world.step_physics();
    }
    let after = world.snapshot(0).pose.position[1];
    assert!(
        after - before > 0.02,
        "spring compression did not decrease: {before} -> {after}"
    );
    assert!(
        world
            .snapshot(0)
            .wheels_contact
            .iter()
            .all(|contact| *contact)
    );
}

#[test]
fn drivetrain_converts_running_torque_with_source_ratio_radius_and_drive_count() {
    use my_summer_slop::simulation::engine::EngineOutput;
    let reference = common::reference();
    let output = EngineOutput {
        running: true,
        torque_nm: 87.0,
        ..EngineOutput::default()
    };
    let force = my_summer_slop::vehicle::drive_force(output, &reference.vehicles[0], 2, 0.3, 2);
    let expected =
        87.0 * reference.vehicles[0].gear_ratios[2].abs() * reference.vehicles[0].final_drive_ratio
            / 0.6;
    assert!((force - expected).abs() < 0.001);
    assert_eq!(
        my_summer_slop::vehicle::drive_force(
            EngineOutput::default(),
            &reference.vehicles[0],
            2,
            0.3,
            2
        ),
        0.0
    );
    assert_eq!(
        my_summer_slop::vehicle::drive_force(output, &reference.vehicles[0], 1, 0.3, 2),
        0.0
    );
}

#[test]
fn automatic_shift_retains_selection_when_curve_candidates_are_zero() {
    let mut reference = common::reference();
    reference.vehicles[0]
        .engine
        .torque_curve
        .keys
        .iter_mut()
        .for_each(|key| {
            key.value = 0.0;
            key.in_tangent = 0.0;
            key.out_tangent = 0.0;
        });
    assert_eq!(
        my_summer_slop::vehicle::select_gear(
            0.0,
            false,
            true,
            2,
            &reference.vehicles[0].engine,
            &reference.vehicles[0]
        ),
        2
    );
    assert_eq!(
        my_summer_slop::vehicle::select_gear(
            0.0,
            false,
            false,
            2,
            &reference.vehicles[0].engine,
            &reference.vehicles[0]
        ),
        1
    );
}

#[test]
fn accelerates_brakes_and_reverses() {
    let mut world = flat_track();
    for _ in 0..180 {
        world.apply_vehicle_forces(
            0,
            DriveInput {
                accelerate: true,
                ..Default::default()
            },
            2600.0,
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
                brake_reverse: true,
                ..Default::default()
            },
            2600.0,
        );
        world.step_physics();
    }
    assert!(world.snapshot(0).speed_mps < driving.speed_mps * 0.5);
    for _ in 0..180 {
        world.apply_vehicle_forces(
            0,
            DriveInput {
                brake_reverse: true,
                ..Default::default()
            },
            2600.0,
        );
        world.step_physics();
    }
    let reversing = world.snapshot(0);
    finite(reversing);
    assert!(reversing.speed_mps < -1.0, "{reversing:?}");
}

#[test]
fn reset_stops_the_chassis() {
    let mut world = flat_track();
    let start = world.snapshot(0).pose;
    let input = DriveInput {
        accelerate: true,
        ..DriveInput::default()
    };
    for _ in 0..180 {
        world.apply_vehicle_forces(0, input, 2600.0);
        world.step_physics();
    }
    assert!(world.snapshot(0).speed_mps.abs() > 1.0);
    world.reset(0, start);
    let snapshot = world.snapshot(0);
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
        world.apply_vehicle_forces(
            0,
            DriveInput {
                accelerate: true,
                ..Default::default()
            },
            0.0,
        );
        world.step_physics();
    }
    assert!(world.snapshot(0).speed_mps.abs() < 0.1);
}

#[test]
fn handbrake_slows_the_car() {
    let mut world = flat_track();
    for _ in 0..120 {
        world.apply_vehicle_forces(
            0,
            DriveInput {
                accelerate: true,
                ..Default::default()
            },
            2600.0,
        );
        world.step_physics();
    }
    let before = world.snapshot(0).speed_mps;
    // Recovered torque and mass require more than one second to stop this speed.
    for _ in 0..180 {
        world.apply_vehicle_forces(
            0,
            DriveInput {
                handbrake: true,
                ..Default::default()
            },
            2600.0,
        );
        world.step_physics();
    }
    assert!(
        world.snapshot(0).speed_mps.abs() < before * 0.1,
        "before {before}, after {}",
        world.snapshot(0).speed_mps
    );
    finite(world.snapshot(0));
}

#[test]
fn removed_ground_cannot_support_wheels() {
    let mut world = common::world();
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
        world.apply_vehicle_forces(0, DriveInput::default(), 2600.0);
        world.step_physics();
    }
    for _ in 0..600 {
        world.apply_vehicle_forces(0, DriveInput::default(), 0.0);
        world.step_physics();
    }
    assert!(world.rapier().bodies[world.chassis_handle(0)].is_sleeping());
    world.remove_collider(ground);
    for _ in 0..60 {
        world.apply_vehicle_forces(0, DriveInput::default(), 2600.0);
        world.step_physics();
    }
    assert_eq!(world.snapshot(0).wheels_contact, [false; 4]);
    assert!(world.snapshot(0).pose.position[1] < -1.0);
}

#[test]
fn one_second_stall_advances_at_most_six_steps() {
    let mut clock = StepClock::default();
    clock.accept_frame(1.0);
    let mut steps = 0;
    while clock.take_step(1.) {
        steps += 1;
    }
    assert_eq!(steps, 6);
    assert!((0.0..1.0).contains(&clock.alpha(1.)));
    clock.accept_frame(0.0);
    assert!(!clock.take_step(1.));
}

#[test]
fn steering_turns_the_car_left() {
    let mut world = flat_track();
    for _ in 0..120 {
        world.apply_vehicle_forces(
            0,
            DriveInput {
                accelerate: true,
                steer: 1.0,
                ..Default::default()
            },
            2600.0,
        );
        world.step_physics();
    }
    let snapshot = world.snapshot(0);
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
        world.apply_vehicle_forces(
            0,
            DriveInput {
                accelerate: true,
                ..Default::default()
            },
            2600.0,
        );
        world.step_physics();
    }
    let snapshot = world.snapshot(0);
    finite(snapshot);
    assert!(snapshot.pose.position[2] > -14.0, "{snapshot:?}");
    assert!(snapshot.speed_mps.abs() < 1.0);
}

#[test]
fn force_application_does_not_step_the_body() {
    let mut world = flat_track();
    let before = world.snapshot(0).pose;
    world.apply_vehicle_forces(
        0,
        DriveInput {
            accelerate: true,
            ..Default::default()
        },
        2600.0,
    );
    assert_eq!(world.snapshot(0).pose, before);
    world.step_physics();
    assert_ne!(world.snapshot(0).pose, before);
}

#[test]
fn query_refresh_preserves_new_kinematic_body_registration() {
    use rapier3d::math::Vector;
    use rapier3d::prelude::{ColliderBuilder, QueryFilter, Ray, RigidBodyBuilder};

    let mut world = flat_track();
    let pose = world.snapshot(0).pose;
    let position = Vector::from_array(pose.position) + Vector::new(0.7, 0.3, 0.0);
    let (body, collider) = world.rapier_mut().insert(
        RigidBodyBuilder::kinematic_position_based().translation(position),
        ColliderBuilder::cuboid(0.5, 0.5, 0.5),
    );
    world.refresh_queries();
    let ray = Ray::new(position + Vector::Y * 3.0, -Vector::Y);
    let hit = world
        .queries(QueryFilter::default())
        .cast_ray(&ray, 5.0, true);
    assert_eq!(hit.map(|hit| hit.0), Some(collider));
    world.step_physics();
    assert!(
        world.rapier().bodies[body]
            .position()
            .translation
            .is_finite()
    );
    finite(world.snapshot(0));
}
