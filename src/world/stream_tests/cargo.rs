//! R05: a loose item locks in the Lada trunk and rides with the car.
use super::drive_out::{enter, park, place_car, road_points, settle};
use crate::{
    interaction::items::ItemLocation,
    reference::TankRole,
    starting_area::GarageRuntime,
    vehicle::{VehiclePose, to_rapier},
};
use rapier3d::math::{Pose, Vector};

const DISTANCE: f64 = 200.;

fn relative(
    run: &GarageRuntime,
    carrier: rapier3d::prelude::RigidBodyHandle,
    pose: VehiclePose,
) -> Pose {
    run.physics.rapier().bodies[carrier].position().inverse() * to_rapier(pose)
}

#[test]
#[ignore = "requires the installed source export"]
fn r05_cargo_locks_in_trunk() {
    let (reference, mut run) = super::garage_controls::load_garage();
    settle(&mut run, &reference, 2.);
    let volume = reference
        .storage
        .volumes
        .iter()
        .position(|v| v.storage_type == 3)
        .expect("Lada trunk volume");
    let trunk = &reference.storage.volumes[volume];
    for slot in &trunk.slots {
        assert!(
            run.area.interaction.parts.slot_item(*slot).is_some(),
            "trunk slot {slot:?} is empty"
        );
    }
    let carrier = run.area.items.get(trunk.owner).unwrap().body.unwrap();
    let can = reference
        .tanks
        .iter()
        .find(|t| t.role == TankRole::Portable && t.fluids.iter().any(|f| f.fluid_type == 0))
        .expect("petrol can")
        .item;
    assert_eq!(
        run.area.items.get(can).unwrap().location,
        ItemLocation::World
    );
    let body = run.area.items.get(can).unwrap().body.unwrap();
    let can_mass = run.physics.authored_body_mass(body);
    let empty = run.physics.rapier().bodies[carrier].mass();

    // Put the can in the middle of the trunk box.
    let target = *run
        .physics
        .rapier()
        .colliders
        .iter()
        .find(|(_, c)| (c.user_data >> 64) as u32 == trunk.collider.0)
        .expect("bound trunk collider")
        .1
        .position();
    run.physics.edit_body(body, |b| {
        b.set_position(target, true);
        b.set_linvel(Vector::ZERO, true);
        b.set_angvel(Vector::ZERO, true);
    });
    run.physics.refresh_queries();
    // Two 1.5 s scans must see the can.
    settle(&mut run, &reference, 4.);
    let locked = ItemLocation::Locked {
        owner: trunk.owner,
        volume: volume as u16,
    };
    assert_eq!(run.area.items.get(can).unwrap().location, locked);
    let loaded = run.physics.rapier().bodies[carrier].mass();
    assert!(
        (loaded - empty - can_mass).abs() < 0.01,
        "mass {empty} -> {loaded}, can {can_mass}"
    );
    let before = relative(&run, carrier, run.area.items.get(can).unwrap().pose);

    // Drive out along the road for about 200 m.
    assert!(enter(&mut run, &reference));
    let start = run.stream.origin.absolute(
        run.physics
            .snapshot(run.vehicle.as_ref().unwrap())
            .pose
            .position,
    );
    let points = road_points(&run, &reference);
    for pair in points.windows(2) {
        place_car(&mut run, &reference, pair[0], pair[1], 0.8);
        park(&mut run, &reference, pair[0]);
        if ((pair[0][0] - start[0]).powi(2) + (pair[0][2] - start[2]).powi(2)).sqrt() > DISTANCE {
            break;
        }
    }
    settle(&mut run, &reference, 2.);
    assert_eq!(run.area.items.get(can).unwrap().location, locked);
    let after = relative(&run, carrier, run.area.items.get(can).unwrap().pose);
    let moved = (after.translation - before.translation).length();
    let turned = after.rotation.angle_between(before.rotation);
    assert!(
        moved < 0.01 && turned < 0.01,
        "can moved {moved} m and turned {turned} rad relative to the chassis"
    );

    // Pick the can up from the trunk.
    assert!(run.area.player.try_exit(&mut run.physics));
    let point = run.area.items.get(can).unwrap().pose.position;
    run.area
        .interaction
        .pickup_stored_item(
            can,
            point,
            &run.area.player,
            &mut run.area.items,
            &mut run.physics,
        )
        .unwrap();
    assert_eq!(
        run.area.items.get(can).unwrap().location,
        ItemLocation::Held
    );
    let unloaded = run.physics.rapier().bodies[carrier].mass();
    assert!(
        (unloaded - empty).abs() < 0.01,
        "mass {empty} -> {unloaded} after pick-up"
    );
}
