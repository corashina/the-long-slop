mod common;
use my_summer_slop::interaction::feedback::ActionMask;
use my_summer_slop::interaction::{AimTarget, ResolvedAim, SlotTarget};
use my_summer_slop::starting_area::GarageRuntime;

#[test]
fn charged_throw_over_valid_slot_does_not_mount() {
    use my_summer_slop::interaction::InteractionAction;
    use my_summer_slop::interaction::items::ItemLocation;
    use my_summer_slop::simulation::{FrameInput, keys};
    use my_summer_slop::vehicle::to_rapier;
    use rapier3d::math::{Pose, Vector};
    use rapier3d::prelude::{ColliderBuilder, RigidBodyBuilder};
    let r = common::reference();
    let mut run = GarageRuntime::new(&r).unwrap();
    let slot = &r.wheel_slots[0];
    let rim = slot.rim;
    let pose = to_rapier(run.area.player.view_pose(&run.physics))
        * Pose::from_translation(Vector::new(0., 0., -0.5));
    let mass = r.items.iter().find(|item| item.id == rim).unwrap().mass_kg;
    let (body, _) = run.physics.rapier_mut().insert(
        RigidBodyBuilder::dynamic().pose(pose),
        ColliderBuilder::ball(0.1).mass(mass),
    );
    let item = run.area.items.get_mut(rim).unwrap();
    item.body = Some(body);
    item.location = ItemLocation::World;
    run.area
        .interaction
        .pickup_item(
            rim,
            pose.translation.to_array(),
            &run.area.player,
            &mut run.area.items,
            &mut run.physics,
        )
        .unwrap();
    run.area.interaction.step(
        FrameInput::default(),
        ResolvedAim::default(),
        &mut run.area.player,
        &mut run.area.items,
        &mut run.physics,
        0.02,
        Default::default(),
        1.,
    );
    run.area.interaction.step(
        FrameInput {
            pressed: keys::GRAB,
            held: keys::GRAB,
            ..Default::default()
        },
        ResolvedAim::default(),
        &mut run.area.player,
        &mut run.area.items,
        &mut run.physics,
        0.2,
        Default::default(),
        1.,
    );
    assert!(run.area.interaction.throw_charge() > r.player.throw_minimum);
    let aim = ResolvedAim {
        installation_target: Some(AimTarget {
            collider: run.wheels.sensor(slot.id).unwrap(),
            item: rim,
            distance: 0.5,
            point: pose.translation.to_array(),
            slot: Some(SlotTarget::Wheel(slot.id)),
        }),
        ..Default::default()
    };
    let available = run.area.interaction.available_actions(
        &aim,
        &run.area.player,
        &run.area.items,
        &run.physics,
        Default::default(),
    );
    assert_eq!(available.slot, Some(SlotTarget::Wheel(slot.id)));
    assert_eq!(available.actions.0 & ActionMask::MOUNT, 0);
    let action = run.area.interaction.step(
        FrameInput {
            released: keys::GRAB,
            ..Default::default()
        },
        aim,
        &mut run.area.player,
        &mut run.area.items,
        &mut run.physics,
        0.02,
        Default::default(),
        1.,
    );
    assert_eq!(action, Some(InteractionAction::Throw(rim)));
    assert_eq!(run.area.interaction.held(), None);
    assert_eq!(
        run.area.items.get(rim).unwrap().location,
        ItemLocation::World
    );
    assert!(run.physics.rapier().bodies[body].linvel().length() > 0.);
}
