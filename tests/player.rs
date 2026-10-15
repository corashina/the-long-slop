mod common;

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
