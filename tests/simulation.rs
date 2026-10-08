use desert_drive::simulation::{DriveInput, DriveState, FIXED_DT, FuelState, can_refuel};
use desert_drive::vehicle::{PhysicsWorld, VehiclePose};
use desert_drive::world::{DEFAULT_SEED, WorldStream, nearest_station};

const START: VehiclePose = VehiclePose {
    position: [0.0, 1.1, 0.0],
    rotation: [0.0, 0.0, 0.0, 1.0],
};

#[test]
fn fuel_bounds_and_rates() {
    let mut fuel = FuelState::default();
    fuel.consume(0.0, 10.0);
    assert!((fuel.liters - 11.99).abs() < 0.00001);
    fuel.consume(1.0, 10.0);
    assert!((fuel.liters - 11.89).abs() < 0.00001);
    fuel.consume(1.0, 2000.0);
    assert_eq!(fuel.liters, 0.0);
    fuel.refuel(2.0);
    assert_eq!(fuel.liters, 8.0);
    fuel.refuel(100.0);
    assert_eq!(fuel.liters, 40.0);
}

#[test]
fn refuel_requires_near_station_and_low_speed() {
    assert!(can_refuel(0.0, 6.0));
    assert!(can_refuel(-0.99, 5.99));
    assert!(!can_refuel(1.0, 6.0));
    assert!(!can_refuel(-1.0, 6.0));
    assert!(!can_refuel(0.0, 6.01));
}

#[test]
fn refuel_rejects_motion_outside_the_forward_axis() {
    let mut state = DriveState::default();
    state.started = true;
    state.fuel.liters = 0.0;
    let mut world = PhysicsWorld::new(START);
    for _ in 0..60 {
        world.step(DriveInput::default(), false);
    }
    assert!(world.snapshot().linear_velocity[1].abs() > 1.0);
    state.step(&mut world, DriveInput::default(), true, 0.0);
    assert_eq!(state.fuel.liters, 0.0);
}

#[test]
fn pause_freezes_physics_fuel_and_trip_until_resume() {
    let mut state = DriveState::default();
    let mut world = PhysicsWorld::new(START);
    let input = DriveInput {
        accelerate: true,
        ..Default::default()
    };
    state.step(&mut world, input, false, 10.0);
    assert_eq!(world.snapshot().pose.position, START.position);
    state.started = true;
    state.step(&mut world, input, false, 10.0);
    assert!(state.fuel.liters < 12.0);
    state.paused = true;
    let position = world.snapshot().pose.position;
    let liters = state.fuel.liters;
    for _ in 0..120 {
        state.step(&mut world, input, true, 0.0);
        state.record_travel([0.0; 3], [3.0, 8.0, 4.0], false);
    }
    assert_eq!(world.snapshot().pose.position, position);
    assert_eq!(state.fuel.liters, liters);
    assert_eq!(state.trip_m, 0.0);
    state.paused = false;
    state.step(&mut world, input, false, 10.0);
    assert!(state.fuel.liters < liters);
    state.record_travel([0.0; 3], [3.0, 8.0, 4.0], false);
    assert_eq!(state.trip_m, 5.0);
}

#[test]
fn empty_tank_disables_engine_but_refueling_restores_it() {
    let mut state = DriveState::default();
    state.started = true;
    state.fuel.liters = 0.0;
    let mut world = PhysicsWorld::new(START);
    let mut stream = WorldStream::new(DEFAULT_SEED);
    stream.sync_at([0.0; 3], &mut world);
    for _ in 0..120 {
        world.step(DriveInput::default(), false);
    }
    let input = DriveInput {
        accelerate: true,
        ..Default::default()
    };
    for _ in 0..120 {
        state.step(&mut world, input, false, 0.0);
    }
    assert!(world.snapshot().speed_mps.abs() < 0.1);
    for _ in 0..60 {
        state.step(&mut world, DriveInput::default(), true, 0.0);
    }
    assert!((3.98..4.01).contains(&state.fuel.liters));
    for _ in 0..120 {
        state.step(&mut world, input, false, 10.0);
    }
    assert!(world.snapshot().speed_mps > 2.0);
}

#[test]
fn reset_and_rebase_preserve_fuel_and_trip_restart_restores_them() {
    let mut state = DriveState::default();
    state.started = true;
    state.fuel.liters = 7.0;
    state.record_travel([0.0; 3], [3.0, 99.0, 4.0], false);
    let mut world = PhysicsWorld::new(START);
    let mut stream = WorldStream::new(DEFAULT_SEED);
    let pose = stream.reset_pose(3000.0, false, &mut world);
    world.reset(pose);
    state.record_travel([3.0, 0.0, 4.0], stream.origin.absolute(pose.position), true);
    stream.rebase(&mut world).unwrap();
    assert_eq!(state.fuel.liters, 7.0);
    assert_eq!(state.trip_m, 5.0);
    state.paused = true;
    state.restart();
    assert_eq!(state.fuel.liters, 12.0);
    assert_eq!(state.trip_m, 0.0);
    assert!(state.started && !state.paused);
}

#[test]
fn empty_tank_reset_loads_station_before_refueling() {
    let mut state = DriveState::default();
    state.started = true;
    state.fuel.liters = 0.0;
    state.trip_m = 123.0;
    let mut world = PhysicsWorld::new(START);
    let mut stream = WorldStream::new(DEFAULT_SEED);
    stream.sync_at([0.0; 3], &mut world);
    let station = nearest_station(DEFAULT_SEED, 1800.0);
    assert!(
        stream
            .chunks()
            .all(|c| c.data.stations.iter().all(|s| s.id != station.id))
    );
    let pose = stream.reset_pose(1800.0, state.fuel.liters == 0.0, &mut world);
    world.reset(pose);
    assert!(
        stream
            .chunks()
            .any(|c| c.data.stations.iter().any(|s| s.id == station.id))
    );
    let absolute = stream.origin.absolute(world.snapshot().pose.position);
    let distance = ((absolute[0] - station.absolute_point[0]).powi(2)
        + (absolute[2] - station.absolute_point[2]).powi(2))
    .sqrt() as f32;
    assert!(can_refuel(world.snapshot().speed_mps * 3.6, distance));
    state.step(&mut world, DriveInput::default(), true, distance);
    assert_eq!(state.fuel.liters, 4.0 * FIXED_DT);
    assert_eq!(state.trip_m, 123.0);
}
