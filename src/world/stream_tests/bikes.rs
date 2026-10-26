//! B12: the Bike01 copy kick starts at 10 m/s, then rides with balance on flat ground.
use super::drive_out::{frame, frame_with, settle};
use super::*;
use crate::player::SeatOwner;
use crate::reference::{BalanceReference, DrivingAssists};
use crate::simulation::{FIXED_DT, FrameInput, keys};
use crate::world::ObjectKey;
use rapier3d::math::Vector;

/// The reference with a Bike01 startup entry 30 m above `StartCar02`.
/// Returns the startup ordinal and the balance record.
fn bike_reference() -> (ReferenceManifest, u32, BalanceReference) {
    let mut reference = reference();
    let world = std::sync::Arc::make_mut(&mut reference.world);
    let category = world
        .catalog
        .categories
        .iter()
        .find(|c| c.name == "bike01")
        .expect("bike01 category");
    let prototype = category
        .items
        .iter()
        .flatten()
        .filter_map(|&i| world.catalog.items[i as usize].prototype.as_ref())
        .filter_map(|source| world.prototypes.iter().find(|p| &p.source == source))
        .filter(|p| p.vehicle.as_ref().is_some_and(|v| v.balance.is_some()))
        .max_by_key(|p| p.engines.len())
        .expect("Bike01 prototype with balance");
    let (source, balance) = (
        prototype.source.clone(),
        prototype.vehicle.as_ref().unwrap().balance.clone().unwrap(),
    );
    let start = world
        .startup
        .iter()
        .position(|s| s.group == "StartCar02" && s.prototype.is_some())
        .expect("StartCar02 placement");
    let mut copy = world.startup[start].clone();
    copy.pose.translation[1] += 30.;
    copy.prototype = Some(source);
    world.startup.push(copy);
    let ordinal = world.startup.len() as u32 - 1;
    (reference, ordinal, balance)
}

/// Source roll in degrees: the chassis up around its forward.
fn roll(run: &GarageRuntime, key: &ObjectKey) -> f32 {
    let rotation = *run.physics.rapier().bodies[run.physics.chassis_handle(key)].rotation();
    let (up, right) = (rotation * Vector::Y, rotation * Vector::X);
    right.y.atan2(up.y).to_degrees()
}

fn running(run: &GarageRuntime, key: &ObjectKey) -> bool {
    run.engine_output(key).running
}

/// One game frame with `held` keys.
fn step(run: &mut GarageRuntime, reference: &ReferenceManifest, held: u32) {
    let input = FrameInput {
        held,
        ..Default::default()
    };
    frame_with(run, reference, input);
}

#[test]
#[ignore = "requires the installed source export"]
fn bike01_kick_starts_and_rides_upright() {
    let (reference, ordinal, balance) = bike_reference();
    let mut run = GarageRuntime::new_default(&reference).unwrap();
    prepare_ready(&mut run, &reference);
    run.started = true;
    run.paused = false;
    run.driving_assists = DrivingAssists {
        automatic_clutch: true,
        automatic_gears: true,
    };
    let key = (0..2000)
        .find_map(|_| {
            let key = run.vehicles.keys().find(|key| {
                matches!(key, ObjectKey::Generated(id)
                    if id.layer == WorldLayer::Startup && id.ordinal == ordinal)
            });
            let key = key.cloned();
            if key.is_none() {
                frame(&mut run, &reference);
            }
            key
        })
        .expect("Bike01 copy");

    // A flat deck under the wheels, then the driver seat.
    let pose = run.physics.snapshot(&key).pose;
    let lowest = (0..run.physics.wheel_contacts(&key).count())
        .map(|i| run.physics.wheel_pose(&key, i).position[1])
        .fold(f32::INFINITY, f32::min);
    run.physics.insert_box(
        [pose.position[0], lowest - 1.5, pose.position[2]],
        [40., 0.5, 200.],
    );
    run.physics.reset_preserving_wheels(&key, pose);
    run.physics.refresh_queries();
    let seat = run
        .physics
        .rapier()
        .colliders
        .iter()
        .find_map(|(handle, _)| {
            run.area
                .interaction
                .item_seat(handle)
                .filter(|s| s.owner == SeatOwner::Vehicle(key.clone()) && s.seat.driver)
        })
        .expect("Bike01 driver seat");
    assert!(run.area.player.enter_item_seat(&seat, &mut run.physics));
    settle(&mut run, &reference, 2.);
    assert!(run.physics.wheel_contacts(&key).all(|c| c), "wheels down");
    assert!(!running(&run, &key));

    // Ignition on and 10 m/s forward: the kick start runs the engine.
    run.vehicles.get_mut(&key).unwrap().ignition.ignition = true;
    let chassis = run.physics.chassis_handle(&key);
    run.physics.edit_body(chassis, |body| {
        let forward = *body.rotation() * Vector::NEG_Z;
        body.set_linvel(forward * 10., true);
    });
    let started = (0..(3. / FIXED_DT) as usize).any(|_| {
        step(&mut run, &reference, keys::FORWARD);
        running(&run, &key)
    });
    assert!(started, "kick start runs the engine");

    // Ride 10 s with throttle and a steer change. The roll stays inside the balance limits.
    let start = run.physics.snapshot(&key).pose.position;
    for i in 0..(10. / FIXED_DT) as usize {
        let steer = if i * 2 < (10. / FIXED_DT) as usize {
            keys::LEFT
        } else {
            keys::RIGHT
        };
        step(&mut run, &reference, keys::FORWARD | steer);
        let roll = roll(&run, &key);
        assert!(
            roll > balance.min_angle && roll < balance.max_angle,
            "step {i}: roll {roll} outside {}..{}",
            balance.min_angle,
            balance.max_angle
        );
    }
    let end = run.physics.snapshot(&key).pose.position;
    let moved = ((end[0] - start[0]).powi(2) + (end[2] - start[2]).powi(2)).sqrt();
    assert!(moved > 20., "rode {moved} m");
    assert!(running(&run, &key));
}
