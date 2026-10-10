mod common;

use my_summer_slop::interaction::items::ItemLocation;
use my_summer_slop::interaction::wheels::WheelAssemblies;
use my_summer_slop::reference::{ItemId, WheelSlotId};
use my_summer_slop::simulation::DriveInput;
use my_summer_slop::starting_area::StartingArea;
use my_summer_slop::vehicle::{PhysicsWorld, VehiclePose};
use rapier3d::math::{Rotation, Vector};
use rapier3d::prelude::ColliderHandle;

fn slot_sensor(physics: &PhysicsWorld, slot: WheelSlotId) -> ColliderHandle {
    physics
        .rapier()
        .colliders
        .iter()
        .find(|(_, collider)| {
            collider.is_sensor()
                && (collider.user_data >> 96) as u8 == slot.0
                && (collider.user_data >> 64) as u32 == 0
                && collider.user_data & my_summer_slop::vehicle::VEHICLE_COLLIDER_FLAG != 0
        })
        .unwrap()
        .0
}

fn assert_slot_aim(
    area: &StartingArea,
    physics: &mut PhysicsWorld,
    slot: WheelSlotId,
    item: ItemId,
    empty: bool,
) {
    physics.refresh_queries();
    let collider = &physics.rapier().colliders[slot_sensor(physics, slot)];
    assert!(collider.parent().is_none());
    let center = collider.translation();
    let chassis = physics.rapier().bodies[physics.chassis_handle()].translation();
    let mut outward = center - chassis;
    outward.y = 0.0;
    outward = outward.normalize();
    let view = VehiclePose {
        position: (center + outward * 0.6).to_array(),
        rotation: Rotation::from_rotation_arc(-Vector::Z, -outward).to_array(),
    };
    let aim = common::focus(&area.interaction, physics, view).unwrap();
    assert_eq!(aim.item, item);
    assert_eq!(aim.slot, empty.then_some(slot));
}

fn reference() -> my_summer_slop::reference::ReferenceManifest {
    let mut reference = common::reference();
    reference.area.player_spawn.translation = [20.0, 1.0, 20.0];
    reference
}

#[test]
fn wheel_sensors_are_unparented_and_keep_aim_after_rebase() {
    let reference = reference();
    let mut physics = common::world();
    let mut area = StartingArea::new(&reference, &mut physics, [0.0; 3]).unwrap();
    let mut wheels = WheelAssemblies::new(&reference).unwrap();
    wheels.sync(&mut area.items, &mut physics).unwrap();
    let slot = reference.vehicle.wheel_slots[0];
    let rim = reference.wheel_slots[0].rim;
    assert_slot_aim(&area, &mut physics, slot, rim, false);
    wheels.remove(slot, &mut area.items, &mut physics).unwrap();
    // Move the loose wheel clear of its empty mount.
    let body = area.items.get(rim).unwrap().body.unwrap();
    physics.rapier_mut().bodies[body].set_translation(Vector::new(20.0, 1.0, 20.0), true);
    assert_slot_aim(&area, &mut physics, slot, rim, true);
    physics.rebase([1024.0, 0.0, 0.0]);
    area.items.rebase([1024.0, 0.0, 0.0]);
    wheels.sync(&mut area.items, &mut physics).unwrap();
    assert_slot_aim(&area, &mut physics, slot, rim, true);
    wheels
        .install(rim, slot, &mut area.items, &mut physics)
        .unwrap();
    assert_slot_aim(&area, &mut physics, slot, rim, false);
}

#[test]
fn removed_wheel_has_no_support() {
    let reference = reference();
    let mut physics = common::world();
    physics.insert_box([0.0, -0.25, 0.0], [100.0, 0.25, 100.0]);
    let mut area = StartingArea::new(&reference, &mut physics, [0.0; 3]).unwrap();
    let mut wheels = WheelAssemblies::new(&reference).unwrap();
    wheels.sync(&mut area.items, &mut physics).unwrap();
    for _ in 0..120 {
        physics.apply_vehicle_forces(DriveInput::default(), 0.0);
        physics.step_physics();
        wheels.sync(&mut area.items, &mut physics).unwrap();
    }
    let slot = reference.vehicle.wheel_slots[0];
    let mounted = reference
        .wheel_slots
        .iter()
        .find(|wheel| wheel.id == slot)
        .unwrap();
    assert!(physics.snapshot().wheels_contact[0]);
    let condition = area.items.get(mounted.rim).unwrap().condition;
    let count = area.items.len();
    let id = wheels.remove(slot, &mut area.items, &mut physics).unwrap();
    assert_eq!(id, mounted.rim);
    assert_eq!(physics.wheel_support_count(), 3);
    physics.apply_vehicle_forces(DriveInput::default(), 0.0);
    physics.step_physics();
    assert!(!physics.snapshot().wheels_contact[0]);
    let part = area.items.get(id).unwrap();
    assert_eq!(part.location, ItemLocation::World);
    assert_eq!(part.condition, condition);
    assert!(physics.rapier().bodies[part.body.unwrap()].is_dynamic());
    let body = part.body.unwrap();
    let rotation = *physics.rapier().bodies[body].rotation();
    physics.rapier_mut().bodies[body].apply_torque_impulse(rapier3d::math::Vector::X, true);
    physics.step_physics();
    assert_ne!(*physics.rapier().bodies[body].rotation(), rotation);
    wheels
        .install(id, slot, &mut area.items, &mut physics)
        .unwrap();
    assert_eq!(physics.wheel_support_count(), 4);
    assert_eq!(area.items.len(), count);
    assert_eq!(area.items.get(id).unwrap().condition, condition);
    assert!(
        wheels
            .install(id, slot, &mut area.items, &mut physics)
            .is_err()
    );
}

#[test]
fn split_and_assembly_conserve_parts_and_mass() {
    let reference = reference();
    let mut physics = common::world();
    let mut area = StartingArea::new(&reference, &mut physics, [0.0; 3]).unwrap();
    let mut wheels = WheelAssemblies::new(&reference).unwrap();
    wheels.sync(&mut area.items, &mut physics).unwrap();
    let mounted = &reference.wheel_slots[0];
    let count = area.items.len();
    let initial_mass = physics.rapier().bodies[physics.chassis_handle()].mass();
    let id = wheels
        .remove(mounted.id, &mut area.items, &mut physics)
        .unwrap();
    let (rim, tire) = wheels.split(id, &mut area.items, &mut physics).unwrap();
    assert_eq!((rim, tire), (mounted.rim, mounted.tire));
    assert_eq!(area.items.len(), count);
    assert_eq!(area.items.get(tire).unwrap().location, ItemLocation::World);
    assert_ne!(
        area.items.get(rim).unwrap().body,
        area.items.get(tire).unwrap().body
    );
    assert!(
        wheels
            .assemble(rim, ItemId(4), &mut area.items, &mut physics)
            .is_err()
    );
    assert_eq!(
        wheels
            .assemble(rim, tire, &mut area.items, &mut physics)
            .unwrap(),
        rim
    );
    assert_eq!(
        area.items.get(tire).unwrap().location,
        ItemLocation::Attached(rim)
    );
    wheels
        .install(rim, mounted.id, &mut area.items, &mut physics)
        .unwrap();
    wheels.sync(&mut area.items, &mut physics).unwrap();
    assert!(
        (physics.rapier().bodies[physics.chassis_handle()].mass() - initial_mass).abs() < 0.001
    );
    assert_eq!(area.items.len(), count);
}

#[test]
fn rebase_and_reattachment_keep_contact_and_identity() {
    let reference = reference();
    let mut physics = common::world();
    physics.insert_box([0.0, -0.25, 0.0], [100.0, 0.25, 100.0]);
    let mut area = StartingArea::new(&reference, &mut physics, [0.0; 3]).unwrap();
    let mut wheels = WheelAssemblies::new(&reference).unwrap();
    wheels.sync(&mut area.items, &mut physics).unwrap();
    let slot = reference.vehicle.wheel_slots[0];
    let id = wheels.remove(slot, &mut area.items, &mut physics).unwrap();
    let shape_handles = physics.rapier().bodies[area.items.get(id).unwrap().body.unwrap()]
        .colliders()
        .to_vec();
    physics.rebase([1024.0, 0.0, 0.0]);
    area.items.rebase([1024.0, 0.0, 0.0]);
    wheels
        .install(id, slot, &mut area.items, &mut physics)
        .unwrap();
    for _ in 0..120 {
        physics.apply_vehicle_forces(DriveInput::default(), 0.0);
        physics.step_physics();
        wheels.sync(&mut area.items, &mut physics).unwrap();
    }
    assert!(physics.snapshot().wheels_contact[0]);
    assert_eq!(wheels.slot_for_item(id), Some(slot));
    for handle in shape_handles {
        assert!(physics.rapier().colliders.contains(handle));
    }
}

#[test]
#[ignore = "Requires the local installed-build export"]
fn production_wheel_bindings_detach_and_restore() {
    let reference = my_summer_slop::reference::ReferenceManifest::from_json(
        &std::fs::read("web/assets/reference/manifest.json").unwrap(),
    )
    .unwrap();
    let transform = reference.area.car_pose;
    let mut physics = my_summer_slop::vehicle::PhysicsWorld::new(
        &reference,
        my_summer_slop::vehicle::VehiclePose {
            position: transform.translation,
            rotation: transform.rotation,
        },
    )
    .unwrap();
    let mut area = StartingArea::new(&reference, &mut physics, [0.0; 3]).unwrap();
    let mut wheels = WheelAssemblies::new(&reference).unwrap();
    wheels.sync(&mut area.items, &mut physics).unwrap();
    let slot = reference.vehicle.wheel_slots[0];
    let count = area.items.len();
    let mounted = reference
        .wheel_slots
        .iter()
        .find(|wheel| wheel.id == slot)
        .unwrap();
    assert_slot_aim(&area, &mut physics, slot, mounted.rim, false);
    let item = wheels.remove(slot, &mut area.items, &mut physics).unwrap();
    let body = area.items.get(item).unwrap().body.unwrap();
    let translation = physics.rapier().bodies[body].translation() + Vector::X * 20.0;
    physics.rapier_mut().bodies[body].set_translation(translation, true);
    assert_slot_aim(&area, &mut physics, slot, mounted.rim, true);
    physics.rebase([1024.0, 0.0, 0.0]);
    area.items.rebase([1024.0, 0.0, 0.0]);
    wheels.sync(&mut area.items, &mut physics).unwrap();
    assert_slot_aim(&area, &mut physics, slot, mounted.rim, true);
    let (rim, tire) = wheels.split(item, &mut area.items, &mut physics).unwrap();
    assert_eq!(
        wheels
            .assemble(rim, tire, &mut area.items, &mut physics)
            .unwrap(),
        item
    );
    wheels
        .install(item, slot, &mut area.items, &mut physics)
        .unwrap();
    physics.rebase([1024.0, 0.0, 0.0]);
    area.items.rebase([1024.0, 0.0, 0.0]);
    wheels.sync(&mut area.items, &mut physics).unwrap();
    assert_slot_aim(&area, &mut physics, slot, mounted.rim, false);
    assert_eq!(physics.wheel_support_count(), 4);
    assert_eq!(area.items.len(), count);
}
