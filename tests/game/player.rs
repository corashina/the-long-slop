use crate::common;

use my_summer_slop::player::{ExitReference, PlayerMode, PlayerState};
use my_summer_slop::simulation::{FIXED_DT, FrameInput, keys};
use my_summer_slop::vehicle::{PhysicsWorld, VehiclePose};

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

struct CountingAllocator;
thread_local! {
    static ALLOCATIONS: Cell<Option<(usize, usize)>> = const { Cell::new(None) };
}

fn count_allocation(bytes: usize) {
    let _ = ALLOCATIONS.try_with(|count| {
        if let Some((allocations, total)) = count.get() {
            count.set(Some((allocations + 1, total + bytes)));
        }
    });
}

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count_allocation(layout.size());
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        count_allocation(size);
        unsafe { System.realloc(pointer, layout, size) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn measure_allocations(action: impl FnOnce()) -> (usize, usize) {
    ALLOCATIONS.with(|count| count.set(Some((0, 0))));
    action();
    ALLOCATIONS.with(|count| count.replace(None).unwrap())
}

const START: VehiclePose = VehiclePose {
    position: [20.0, 1.0, 20.0],
    rotation: [0.0, 0.0, 0.0, 1.0],
};

fn player(world: &mut PhysicsWorld) -> PlayerState {
    let reference = common::reference();
    let mut player = PlayerState::new(&reference.player, START, world);
    player.configure_seats(&reference.seats);
    player
}

fn step(player: &mut PlayerState, world: &mut PhysicsWorld, held: u32) {
    player.step(
        FrameInput {
            held,
            ..FrameInput::default()
        },
        world,
        FIXED_DT,
    );
    world.step_physics();
}

fn floor(world: &mut PhysicsWorld) {
    world.insert_box([20.0, -0.5, 20.0], [20.0, 0.5, 20.0]);
}

fn exit_limits() -> ExitReference {
    let reference = common::reference();
    ExitReference::from_reference(
        &reference.player,
        reference.seats.iter().find(|seat| seat.driver).unwrap(),
    )
}

#[test]
fn capsule_cannot_cross_wall() {
    let mut world = common::world();
    floor(&mut world);
    world.insert_box([20.0, 2.0, 18.0], [5.0, 2.0, 0.1]);
    let mut player = player(&mut world);
    for _ in 0..180 {
        step(&mut player, &mut world, keys::FORWARD);
    }
    assert!(player.body_pose(&world).position[2] > 18.1);
    assert!(player.body_pose(&world).position[2] < 19.0);
}

#[test]
fn crouch_cannot_stand_under_roof() {
    let mut world = common::world();
    floor(&mut world);
    let mut player = player(&mut world);
    for _ in 0..60 {
        step(&mut player, &mut world, keys::CROUCH);
    }
    assert!(player.crouched());
    let foot =
        player.body_pose(&world).position[1] - common::reference().player.crouch_height * 0.5;
    let roof = world.insert_box([20.0, foot + 1.0, 20.0], [2.0, 0.1, 2.0]);
    for _ in 0..30 {
        step(&mut player, &mut world, 0);
    }
    assert!(player.crouched());
    world.remove_collider(roof);
    step(&mut player, &mut world, 0);
    assert!(!player.crouched());
}

#[test]
fn blocked_exit_retains_seat() {
    let mut world = common::world();
    let mut player = player(&mut world);
    let driver = common::reference()
        .seats
        .into_iter()
        .find(|s| s.driver)
        .unwrap();
    assert!(player.enter(
        driver.id,
        VehiclePose {
            position: [0.0, 1.0, 0.0],
            ..START
        },
        &mut world
    ));
    let view = player.view_pose(&world);
    world.insert_box(view.position, [5.0; 3]);
    let limits = ExitReference {
        minimum_look_angle_degrees: 0.0,
        ..exit_limits()
    };
    assert!(!player.try_exit(&mut world, &limits));
    assert_eq!(player.seat(), Some(driver.id));
    assert_eq!(player.mode(), PlayerMode::Seated);
}

#[test]
fn exit_ignores_hidden_cabin_layer_but_keeps_solid_wall() {
    use rapier3d::math::Vector;
    use rapier3d::prelude::{ColliderBuilder, Group, InteractionGroups, InteractionTestMode};
    let mut world = common::world();
    let mut player = player(&mut world);
    let driver = common::reference()
        .seats
        .into_iter()
        .find(|seat| seat.driver)
        .unwrap();
    assert!(player.enter(driver.id, START, &mut world));
    let view = player.view_pose(&world);
    world.rapier_mut().colliders.insert(
        ColliderBuilder::cuboid(5.0, 5.0, 5.0)
            .translation(Vector::from_array(view.position))
            .collision_groups(InteractionGroups::new(
                Group::from_bits_retain(1 << 13),
                Group::ALL,
                InteractionTestMode::And,
            )),
    );
    let wall = world.insert_box(view.position, [5.0; 3]);
    let limits = ExitReference {
        minimum_look_angle_degrees: 0.0,
        ..exit_limits()
    };
    assert!(!player.try_exit(&mut world, &limits));
    assert_eq!(player.seat(), Some(driver.id));
    world.remove_collider(wall);
    assert!(player.try_exit(&mut world, &limits));
    assert_eq!(player.mode(), PlayerMode::Walking);
    let reference = common::reference().player;
    let expected_y = view.position[1] + reference.camera_height - reference.standing_height * 0.5;
    assert!((player.body_pose(&world).position[1] - expected_y).abs() < 1e-5);
}

#[test]
fn exit_uses_source_capsule_at_the_head_point() {
    let mut world = common::world();
    let mut player = player(&mut world);
    let driver = common::reference()
        .seats
        .into_iter()
        .find(|seat| seat.driver)
        .unwrap();
    assert!(player.enter(driver.id, START, &mut world));
    let view = player.view_pose(&world);
    world.insert_box(
        [view.position[0], view.position[1] + 0.75, view.position[2]],
        [5.0, 0.02, 5.0],
    );
    let limits = ExitReference {
        minimum_look_angle_degrees: 0.0,
        ..exit_limits()
    };
    assert!(!player.try_exit(&mut world, &limits));
    assert_eq!(player.seat(), Some(driver.id));
}

#[test]
fn look_clamps_walking_and_seated() {
    let mut world = common::world();
    let mut player = player(&mut world);
    player.step(
        FrameInput {
            mouse: [0.0, 1000.0],
            ..FrameInput::default()
        },
        &mut world,
        FIXED_DT,
    );
    assert_eq!(player.pitch_degrees().abs(), 89.0);
    let driver = common::reference()
        .seats
        .into_iter()
        .find(|s| s.driver)
        .unwrap();
    player.enter(
        driver.id,
        VehiclePose {
            position: [0.0, 1.0, 0.0],
            ..START
        },
        &mut world,
    );
    player.step(
        FrameInput {
            mouse: [1000.0, -1000.0],
            ..FrameInput::default()
        },
        &mut world,
        FIXED_DT,
    );
    assert_eq!(player.pitch_degrees().abs(), 89.0);
    assert_eq!(player.yaw_degrees(), driver.yaw_limits_degrees[1]);
    player.step(
        FrameInput {
            mouse: [-2000.0, 2000.0],
            ..FrameInput::default()
        },
        &mut world,
        FIXED_DT,
    );
    assert_eq!(player.pitch_degrees().abs(), 89.0);
    assert_eq!(player.yaw_degrees(), driver.yaw_limits_degrees[0]);
}

#[test]
fn exit_uses_current_view_and_source_angle() {
    let mut world = common::world();
    let mut player = player(&mut world);
    let driver = common::reference()
        .seats
        .into_iter()
        .find(|seat| seat.driver)
        .unwrap();
    let local = VehiclePose {
        position: [20.0, 2.0, 20.0],
        ..START
    };
    assert!(player.enter(driver.id, local, &mut world));
    assert!(player.driving());
    assert!(!player.try_exit(&mut world, &exit_limits()));
    let view = player.view_pose(&world);
    // Block the first scan positions. A rightward view clears the block.
    world.insert_box(
        [view.position[0], view.position[1], view.position[2]],
        [0.05, 2.0, 1.0],
    );
    player.step(
        FrameInput {
            mouse: [90.0, 0.0],
            ..FrameInput::default()
        },
        &mut world,
        FIXED_DT,
    );
    assert!(player.try_exit(&mut world, &exit_limits()));
    assert_eq!(player.mode(), PlayerMode::Walking);
    assert!(!player.driving());
    assert!(player.body_pose(&world).position[0] > view.position[0] + 0.3);
}

#[test]
fn seated_view_follows_chassis_and_rebase() {
    use rapier3d::math::{Pose, Rotation, Vector};
    let mut world = common::world();
    let mut player = player(&mut world);
    let driver = common::reference()
        .seats
        .into_iter()
        .find(|seat| seat.driver)
        .unwrap();
    let local = VehiclePose {
        position: [2.0, 2.0, 1.0],
        ..START
    };
    assert!(player.enter(driver.id, local, &mut world));
    let chassis = world.chassis_handle();
    world.rapier_mut().bodies[chassis].set_position(
        Pose::from_parts(Vector::new(5.0, 1.0, 5.0), Rotation::from_rotation_y(0.5)),
        false,
    );
    let before = player.view_pose(&world);
    let expected = *world.rapier().bodies[chassis].position()
        * Vector::new(2.0, 2.0 + common::reference().player.camera_height, 1.0);
    for (actual, expected) in before.position.into_iter().zip(expected.to_array()) {
        assert!((actual - expected).abs() < 1.0e-5);
    }
    let shift = [10.0, 0.0, 7.0];
    world.rebase(shift);
    player.rebase(shift);
    let after = player.view_pose(&world);
    for (i, shift) in shift.iter().enumerate() {
        assert!((before.position[i] - shift - after.position[i]).abs() < 1.0e-5);
    }
    assert_eq!(before.rotation, after.rotation);
}

#[test]
fn grounded_jump_uses_pressed_edge() {
    let mut world = common::world();
    floor(&mut world);
    let mut player = player(&mut world);
    for _ in 0..60 {
        step(&mut player, &mut world, 0);
    }
    assert!(player.grounded());
    let start = player.body_pose(&world).position[1];
    player.step(
        FrameInput {
            held: keys::SPACE,
            pressed: keys::SPACE,
            ..FrameInput::default()
        },
        &mut world,
        FIXED_DT,
    );
    world.step_physics();
    assert!(!player.grounded());
    assert!(player.body_pose(&world).position[1] > start);
    for _ in 0..180 {
        step(&mut player, &mut world, keys::SPACE);
    }
    assert!(player.grounded());
    assert!((player.body_pose(&world).position[1] - start).abs() < 0.02);
}

#[test]
fn run_speed_and_crouched_eye_match_reference() {
    let reference = common::reference().player;
    let mut world = common::world();
    floor(&mut world);
    let mut player = player(&mut world);
    for _ in 0..60 {
        step(&mut player, &mut world, 0);
    }
    let standing = player.body_pose(&world);
    let standing_eye = player.view_pose(&world).position[1];
    assert!((standing_eye - standing.position[1] - reference.standing_height * 0.5).abs() < 1.0e-5);
    for _ in 0..60 {
        step(&mut player, &mut world, keys::FORWARD | keys::RUN);
    }
    assert!(player.running());
    let run_position = player.body_pose(&world).position[2];
    let run_distance = standing.position[2] - run_position;
    for _ in 0..60 {
        step(&mut player, &mut world, keys::BACK);
    }
    let walk_distance = player.body_pose(&world).position[2] - run_position;
    assert!(!player.running());
    // Shape casts shorten floor movement. Compare the recovered speed ratio.
    assert!(
        (run_distance / walk_distance - reference.run_multiplier).abs()
            < reference.run_multiplier * 0.05,
        "run distance={run_distance}, walk distance={walk_distance}"
    );
    step(&mut player, &mut world, keys::CROUCH);
    let crouched = player.body_pose(&world);
    let crouched_eye = player.view_pose(&world).position[1];
    assert!(
        (standing.position[1] - reference.standing_height * 0.5 - crouched.position[1]
            + reference.crouch_height * 0.5)
            .abs()
            < 0.02
    );
    assert!((crouched_eye - crouched.position[1] - reference.crouch_height * 0.5).abs() < 1.0e-5);
    assert!(
        (standing_eye - crouched_eye - reference.standing_height + reference.crouch_height).abs()
            < 0.02
    );
}

#[test]
fn static_steps_reuse_shapes_and_scratch() {
    use rapier3d::control::KinematicCharacterController;
    use rapier3d::math::{Pose, Rotation, Vector};
    use rapier3d::prelude::{QueryFilter, SharedShape};
    let mut world = common::world();
    floor(&mut world);
    let mut player = player(&mut world);
    for _ in 0..60 {
        step(&mut player, &mut world, 0);
    }
    player.step(FrameInput::default(), &mut world, FIXED_DT);
    let reference = common::reference().player;
    let shape = SharedShape::capsule_y(
        reference.standing_height * 0.5 - reference.standing_radius,
        reference.standing_radius,
    );
    let pose = player.body_pose(&world);
    let pose = Pose::from_parts(
        Vector::from_array(pose.position),
        Rotation::from_array(pose.rotation),
    );
    let controller = KinematicCharacterController::default();
    let library_allocations = measure_allocations(|| {
        for _ in 0..120 {
            controller.move_shape(
                FIXED_DT,
                &world.queries(
                    QueryFilter::default()
                        .exclude_sensors()
                        .exclude_rigid_body(player.body_handle()),
                ),
                shape.as_ref(),
                &pose,
                Vector::Y * world.rapier().gravity.y * FIXED_DT * FIXED_DT,
                |_| {},
            );
        }
    });
    let allocations = measure_allocations(|| {
        for _ in 0..120 {
            player.step(FrameInput::default(), &mut world, FIXED_DT);
        }
    });
    eprintln!("120 static steps: library={library_allocations:?}, player={allocations:?}");
    assert!(library_allocations.0 > 0);
    assert_eq!(allocations, library_allocations);
}

#[test]
fn player_pushes_dynamic_box() {
    use rapier3d::math::Vector;
    use rapier3d::prelude::{ColliderBuilder, RigidBodyBuilder};
    let mut world = common::world();
    floor(&mut world);
    let mut player = player(&mut world);
    let (body, _) = world.rapier_mut().insert(
        RigidBodyBuilder::dynamic().translation(Vector::new(20.0, 1.0, 19.38)),
        ColliderBuilder::cuboid(0.3, 0.5, 0.3).mass(10.0),
    );
    world.refresh_queries();
    let before = world.rapier().bodies[body].translation().z;
    player.step(
        FrameInput {
            held: keys::FORWARD,
            ..FrameInput::default()
        },
        &mut world,
        FIXED_DT,
    );
    // Check the player impulse before the shared physics solver can move the box.
    assert!(
        world.rapier().bodies[body].linvel().z < -0.01,
        "box velocity={}",
        world.rapier().bodies[body].linvel().z
    );
    world.step_physics();
    assert!(world.rapier().bodies[body].translation().z < before);
}

#[test]
fn rapier_dynamic_impulse_allocation_cost() {
    use rapier3d::control::KinematicCharacterController;
    use rapier3d::math::{Pose, Vector};
    use rapier3d::prelude::{ColliderBuilder, QueryFilter, RigidBodyBuilder, SharedShape};
    let reference = common::reference().player;
    let mut world = common::world();
    let (body, _) = world.rapier_mut().insert(
        RigidBodyBuilder::dynamic().translation(Vector::new(20.0, 1.0, 19.3)),
        ColliderBuilder::cuboid(0.3, 0.5, 0.3).mass(10.0),
    );
    world.refresh_queries();
    let shape = SharedShape::capsule_y(
        reference.standing_height * 0.5 - reference.standing_radius,
        reference.standing_radius,
    );
    let controller = KinematicCharacterController::default();
    let mut collisions = Vec::with_capacity(20);
    controller.move_shape(
        FIXED_DT,
        &world.queries(QueryFilter::default()),
        shape.as_ref(),
        &Pose::from_translation(Vector::new(20.0, 1.0, 20.0)),
        Vector::new(0.0, 0.0, -0.2),
        |collision| collisions.push(collision),
    );
    assert!(!collisions.is_empty());
    let allocations = measure_allocations(|| {
        controller.solve_character_collision_impulses(
            FIXED_DT,
            &mut world.queries_mut(QueryFilter::default()),
            shape.as_ref(),
            reference.mass_kg,
            &collisions,
        );
    });
    eprintln!(
        "Rapier dynamic solver: {} allocations, {} bytes",
        allocations.0, allocations.1
    );
    assert!(allocations.0 > 0);
    assert!(world.rapier().bodies[body].linvel().z < 0.0);
}
