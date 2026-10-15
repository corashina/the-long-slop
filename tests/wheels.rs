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

fn reference() -> my_summer_slop::reference::ReferenceManifest {
    let mut reference = common::reference();
    reference.area.player_spawn.translation = [20.0, 1.0, 20.0];
    let rim = reference.wheel_slots[0].rim;
    common::add_condition_owner(&mut reference, rim);
    reference
}

fn world(reference: &my_summer_slop::reference::ReferenceManifest) -> PhysicsWorld {
    let mut poses = my_summer_slop::starting_area::initial_vehicle_poses(reference).unwrap();
    poses[0] = VehiclePose {
        position: [0., 1.1, 0.],
        rotation: [0., 0., 0., 1.],
    };
    PhysicsWorld::new(reference, &poses).unwrap()
}

fn remove_cover(area: &mut StartingArea, physics: &mut PhysicsWorld, rim: ItemId) {
    let cover = area.interaction.parts.covering_item(rim).unwrap();
    area.interaction
        .parts
        .remove(cover, &mut area.items, physics)
        .unwrap();
    area.interaction
        .parts
        .detach_mechanism(cover, &mut area.interaction.mechanisms, physics);
    area.interaction
        .sync_parts(&mut area.items, physics)
        .unwrap();
    let body = area.items.get(cover).unwrap().body.unwrap();
    let off = physics.rapier().bodies[body].translation() + Vector::Y * 20.;
    physics.rapier_mut().bodies[body].set_translation(off, true);
    physics.refresh_queries();
}

#[test]
fn split_and_assembly_conserve_parts_and_mass() {
    let reference = reference();
    let mut physics = world(&reference);
    let mut area = StartingArea::new(&reference, &mut physics, [0.0; 3]).unwrap();
    remove_cover(&mut area, &mut physics, reference.wheel_slots[0].rim);
    let mut wheels = WheelAssemblies::new(&reference).unwrap();
    wheels.sync(&mut area.items, &mut physics).unwrap();
    let mounted = &reference.wheel_slots[0];
    let count = area.items.len();
    let initial_mass = physics.rapier().bodies[physics.chassis_handle(0)].mass();
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
        (physics.rapier().bodies[physics.chassis_handle(0)].mass() - initial_mass).abs() < 0.001
    );
    assert_eq!(area.items.len(), count);
}

#[test]
fn rebase_and_reattachment_keep_contact_and_identity() {
    let reference = reference();
    let mut physics = world(&reference);
    physics.insert_box([0.0, -0.25, 0.0], [100.0, 0.25, 100.0]);
    let mut area = StartingArea::new(&reference, &mut physics, [0.0; 3]).unwrap();
    remove_cover(&mut area, &mut physics, reference.wheel_slots[0].rim);
    let mut wheels = WheelAssemblies::new(&reference).unwrap();
    wheels.sync(&mut area.items, &mut physics).unwrap();
    let slot = reference.vehicles[0].wheel_slots[0];
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
        physics.apply_vehicle_forces(0, DriveInput::default(), Default::default());
        physics.step_physics();
        wheels.sync(&mut area.items, &mut physics).unwrap();
    }
    assert!(physics.snapshot(0).wheels_contact[0]);
    assert_eq!(wheels.slot_for_item(id), Some(slot));
    for handle in shape_handles {
        assert!(physics.rapier().colliders.contains(handle));
    }
}

fn runtime_aiming_at_wheel(
    reference: &my_summer_slop::reference::ReferenceManifest,
    slot: WheelSlotId,
) -> my_summer_slop::starting_area::GarageRuntime {
    use my_summer_slop::player::PlayerState;
    use my_summer_slop::starting_area::GarageRuntime;
    let mut runtime = GarageRuntime::new(reference).unwrap();
    let rim = reference
        .wheel_slots
        .iter()
        .find(|wheel| wheel.id == slot)
        .unwrap()
        .rim;
    let center =
        runtime.physics.rapier().colliders[slot_sensor(&runtime.physics, slot)].translation();
    let chassis = runtime.physics.rapier().bodies[runtime.physics.chassis_handle(0)].translation();
    let outward = Vector::new((center - chassis).x, 0.0, (center - chassis).z).normalize();
    let old = runtime.area.player.body_handle();
    runtime.physics.rapier_mut().bodies[old].set_enabled(false);
    runtime.area.player = PlayerState::new(
        &reference.player,
        VehiclePose {
            position: (center + outward * 0.6
                - Vector::Y * (reference.player.standing_height * 0.5))
                .to_array(),
            rotation: Rotation::from_rotation_arc(-Vector::Z, -outward).to_array(),
        },
        &mut runtime.physics,
    );
    runtime
        .area
        .interaction
        .bind_player_body(runtime.area.player.body_handle());
    runtime.physics.refresh_queries();
    let before_aim = runtime.area.interaction.resolve(
        &runtime.physics,
        runtime.area.player.view_pose(&runtime.physics),
        &runtime.area.items,
    );
    assert_eq!(
        before_aim.grab_target.map(|target| target.item),
        Some(
            runtime
                .area
                .interaction
                .parts
                .covering_item(rim)
                .unwrap_or(rim)
        ),
        "{before_aim:?}"
    );
    runtime.started = true;
    runtime
}

#[test]
fn rejected_wheel_installation_keeps_the_carry_constraint() {
    use my_summer_slop::simulation::{FrameInput, keys};
    let mut reference = reference();
    reference
        .nodes
        .iter_mut()
        .find(|node| node.id == reference.vehicles[0].root)
        .unwrap()
        .transform
        .translation[1] = 1.1;
    reference.wheel_slots[0].rim_types = vec!["incompatible".into()];
    let slot = reference.vehicles[0].wheel_slots[0];
    let rim = reference.wheel_slots[0].rim;
    let mut runtime = runtime_aiming_at_wheel(&reference, slot);
    remove_cover(
        &mut runtime.area,
        &mut runtime.physics,
        reference.wheel_slots[0].rim,
    );
    runtime.step(
        &reference,
        FrameInput {
            held: keys::GRAB,
            pressed: keys::GRAB,
            ..Default::default()
        },
    );
    assert_eq!(runtime.area.interaction.held(), Some(rim));
    assert!(
        runtime
            .wheels
            .can_install(rim, slot, &runtime.area.items)
            .is_err()
    );
    runtime.step(
        &reference,
        FrameInput {
            released: keys::GRAB,
            ..Default::default()
        },
    );
    let joints = runtime.physics.rapier().impulse_joints.len();
    runtime.step(
        &reference,
        FrameInput {
            held: keys::GRAB,
            pressed: keys::GRAB,
            ..Default::default()
        },
    );
    runtime.step(
        &reference,
        FrameInput {
            released: keys::GRAB,
            ..Default::default()
        },
    );
    assert_eq!(runtime.notice, "This part does not fit.");
    assert_eq!(runtime.area.interaction.held(), Some(rim));
    assert_eq!(
        runtime.area.items.get(rim).unwrap().location,
        ItemLocation::Held
    );
    assert_eq!(runtime.physics.rapier().impulse_joints.len(), joints);
    let rotation = runtime.area.player.view_pose(&runtime.physics).rotation;
    runtime.step(
        &reference,
        FrameInput {
            held: keys::TRANSLATE,
            mouse: [10.0, 5.0],
            ..Default::default()
        },
    );
    assert_eq!(
        runtime.area.player.view_pose(&runtime.physics).rotation,
        rotation
    );
    assert_eq!(runtime.area.interaction.held(), Some(rim));
}
