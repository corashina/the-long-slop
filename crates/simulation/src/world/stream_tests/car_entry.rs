//! On foot, the use ray on the start Lada driver seat offers the seat, and F never carries the car.
use super::drive_out::frame;
use super::*;
use crate::interaction::feedback::ActionMask;
use crate::player::SeatOwner;
use crate::simulation::FIXED_DT;
use crate::vehicle::{VehiclePose, from_rapier};
use rapier3d::math::{Pose, Rotation, Vector};

#[test]
#[ignore = "requires the installed source export"]
fn lada_driver_seat_offers_sit() {
    let (reference, mut run) = garage_controls::load_garage();
    for _ in 0..4000 {
        if !run.vehicles.is_empty() {
            break;
        }
        frame(&mut run, &reference);
    }
    for _ in 0..(2. / FIXED_DT) as usize {
        frame(&mut run, &reference);
    }
    let key = run.vehicles.keys().next().expect("start car").clone();
    let chassis = run.physics.chassis_handle(&key);
    let car = run.area.items.resolve_key(&key).expect("car item");
    let seats: Vec<_> = run
        .physics
        .rapier()
        .colliders
        .iter()
        .filter_map(|(handle, _)| {
            run.area
                .interaction
                .item_seat(handle)
                .filter(|seat| seat.owner == SeatOwner::Vehicle(key.clone()) && seat.seat.driver)
                .map(|seat| (handle, seat))
        })
        .collect();
    assert!(!seats.is_empty(), "no bound driver seat collider");
    let (_, seat) = &seats[0];
    eprintln!(
        "car {car:?} chassis {chassis:?} seat item {:?} seat part {:?} seat body {:?} colliders {}",
        seat.item,
        seat.seat_item,
        seat.body,
        seats.len()
    );
    for item in run.area.items.iter() {
        if run.area.interaction.parts.contains(item.id) {
            let loose = run.area.interaction.parts.loose_body(item.id);
            if loose == Some(chassis) || item.body == Some(chassis) {
                eprintln!(
                    "part {:?} at {:?} body {:?} loose {:?}",
                    item.id, item.location, item.body, loose
                );
            }
        }
    }
    // Aim at each seat collider from a ring of eye points around the car.
    let center = run.physics.rapier().bodies[chassis].translation();
    let mut sit = 0;
    let mut whole_car = Vec::new();
    for (handle, _) in &seats {
        let target = run.physics.rapier().colliders[*handle]
            .position()
            .translation;
        for step in 0..24 {
            let angle = step as f32 * std::f32::consts::TAU / 24.;
            let eye = Vector::new(
                center.x + 2. * angle.cos(),
                target.y + 0.4,
                center.z + 2. * angle.sin(),
            );
            let rotation = Rotation::from_rotation_arc(-Vector::Z, (target - eye).normalize());
            let camera = from_rapier(&Pose::from_parts(eye, rotation));
            let (aim, available) = look(&run, camera);
            let actions = available.actions.0;
            if actions & ActionMask::ENTER != 0 {
                sit += 1;
            }
            let grab = aim.grab_target.map(|t| t.item);
            if actions & (ActionMask::DISMOUNT | ActionMask::PICKUP | ActionMask::PUSH) != 0
                && let Some(item) = grab
                && run
                    .area
                    .items
                    .get(item)
                    .is_some_and(|i| i.body == Some(chassis))
                && run.area.interaction.parts.loose_body(item) == Some(chassis)
            {
                whole_car.push(item);
            }
            eprintln!(
                "seat collider {handle:?} angle {step}: actions {actions:#x} use {:?} grab {:?} {:?} first {}",
                aim.use_target.map(|t| t.item),
                grab,
                grab.and_then(|i| run.area.items.get(i)).map(|i| i.location),
                first_hit(&run, eye, target, chassis),
            );
        }
    }
    assert!(sit > 0, "no eye point offers the driver seat");
    assert!(whole_car.is_empty(), "F carries the car: {whole_car:?}");
}

/// The first collider on the ray, with its body, layers and item.
fn first_hit(
    run: &crate::starting_area::GarageRuntime,
    eye: Vector,
    target: Vector,
    chassis: rapier3d::prelude::RigidBodyHandle,
) -> String {
    let player = run.area.player.body_handle();
    let ray = rapier3d::prelude::Ray::new(eye, (target - eye).normalize());
    let skip = |_: rapier3d::prelude::ColliderHandle, c: &rapier3d::prelude::Collider| {
        c.parent() != Some(player)
    };
    let filter = rapier3d::prelude::QueryFilter::default().predicate(&skip);
    let Some((handle, toi)) = run.physics.queries(filter).cast_ray(&ray, 5., true) else {
        return "none".into();
    };
    let c = &run.physics.rapier().colliders[handle];
    format!(
        "{handle:?} at {toi:.2} m chassis {} sensor {} layers {:#x} item {} use {:?}",
        c.parent() == Some(chassis),
        c.is_sensor(),
        c.collision_groups().memberships.bits(),
        (c.user_data as u32) as i64 - 1,
        run.area.interaction.use_owner(handle),
    )
}

fn look(
    run: &crate::starting_area::GarageRuntime,
    camera: VehiclePose,
) -> (
    crate::interaction::targets::ResolvedAim,
    crate::interaction::feedback::ActionAvailability,
) {
    let aim = run
        .area
        .interaction
        .resolve(&run.physics, camera, &run.area.items);
    let available = run.area.interaction.available_actions(
        &aim,
        &run.area.player,
        &run.area.items,
        &run.physics,
        run.survival.output(),
    );
    (aim, available)
}
