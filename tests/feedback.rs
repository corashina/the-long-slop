mod common;
use my_summer_slop::interaction::AimTarget;
use my_summer_slop::interaction::feedback::ActionMask;
use my_summer_slop::starting_area::GarageRuntime;

#[test]
fn closed_destination_has_no_fill_action() {
    let r = common::reference();
    let run = GarageRuntime::new(&r).unwrap();
    let cap = r.openings[0].cap;
    let a = run.area.interaction.available_actions(
        Some(AimTarget {
            item: cap,
            distance: 0.5,
            point: [0.; 3],
            slot: None,
        }),
        &run.area.player,
        &run.area.items,
        &run.physics,
    );
    assert_eq!(a.actions.0 & ActionMask::FILL, 0);
}
#[test]
fn occupied_slot_has_no_mount_action() {
    use my_summer_slop::interaction::items::ItemLocation;
    use my_summer_slop::simulation::{FrameInput, keys};
    use my_summer_slop::vehicle::to_rapier;
    use rapier3d::math::{Pose, Vector};
    use rapier3d::prelude::RigidBodyBuilder;
    let r = common::reference();
    let mut run = GarageRuntime::new(&r).unwrap();
    let slot = &r.wheel_slots[0];
    let spare = slot.rim;
    let pose = to_rapier(run.area.player.view_pose(&run.physics))
        * Pose::from_translation(Vector::new(0., 0., -0.5));
    let body = run
        .physics
        .rapier_mut()
        .insert_body(RigidBodyBuilder::dynamic().pose(pose));
    let item = run.area.items.get_mut(spare).unwrap();
    item.body = Some(body);
    item.location = ItemLocation::World;
    run.area
        .items
        .get_mut(r.wheel_slots[1].rim)
        .unwrap()
        .location = ItemLocation::WheelSlot(slot.id);
    run.area.interaction.step(
        FrameInput {
            pressed: keys::GRAB,
            ..Default::default()
        },
        Some(AimTarget {
            item: spare,
            distance: 0.5,
            point: pose.translation.to_array(),
            slot: None,
        }),
        &mut run.area.player,
        &mut run.area.items,
        &mut run.physics,
        1. / 60.,
    );
    assert_eq!(run.area.interaction.held(), Some(spare));
    let a = run.area.interaction.available_actions(
        Some(AimTarget {
            item: slot.rim,
            distance: 0.5,
            point: [0.; 3],
            slot: Some(slot.id),
        }),
        &run.area.player,
        &run.area.items,
        &run.physics,
    );
    assert_eq!(a.actions.0 & ActionMask::MOUNT, 0);
}

#[test]
fn attached_tire_selects_tire_dismount() {
    let r = common::reference();
    let run = GarageRuntime::new(&r).unwrap();
    let tire = r.wheel_slots[0].tire;
    let a = run.area.interaction.available_actions(
        Some(AimTarget {
            item: tire,
            distance: 0.5,
            point: [0.; 3],
            slot: None,
        }),
        &run.area.player,
        &run.area.items,
        &run.physics,
    );
    assert_ne!(a.actions.0 & ActionMask::DISMOUNT, 0);
    assert_eq!(a.part, Some(tire));
}

#[test]
fn blocked_exit_has_no_exit_action() {
    let r = common::reference();
    let mut run = GarageRuntime::new(&r).unwrap();
    let seat = r.seats[0].id;
    assert!(run.area.player.enter(
        seat,
        my_summer_slop::vehicle::VehiclePose {
            position: [0., 2., 0.],
            rotation: [0., 0., 0., 1.]
        },
        &mut run.physics
    ));
    let a = run.area.interaction.available_actions(
        None,
        &run.area.player,
        &run.area.items,
        &run.physics,
    );
    assert_eq!(a.actions.0 & ActionMask::EXIT, 0);
    assert_eq!(run.area.player.seat(), Some(seat));
}
