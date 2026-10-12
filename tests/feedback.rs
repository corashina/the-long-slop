mod common;
use my_summer_slop::interaction::feedback::ActionMask;
use my_summer_slop::interaction::{AimTarget, ResolvedAim, SlotTarget};
use my_summer_slop::starting_area::GarageRuntime;

#[test]
fn closed_destination_has_no_fill_action() {
    let r = common::reference();
    let mut run = GarageRuntime::new(&r).unwrap();
    let cap = r.openings[0].cap;
    // The native fixture omits the tap hit surface. Bind a real cap proxy.
    let body = run.area.items.get(cap).unwrap().body.unwrap();
    let collider = run.physics.rapier_mut().insert_collider(
        rapier3d::prelude::ColliderBuilder::ball(0.05)
            .sensor(true)
            .user_data(cap.0 as u128 + 1),
        Some(body),
    );
    let a = run.area.interaction.available_actions(
        &ResolvedAim {
            use_target: Some(AimTarget {
                collider,
                item: cap,
                distance: 0.5,
                point: [0.; 3],
                slot: None,
            }),
            ..Default::default()
        },
        &run.area.player,
        &run.area.items,
        &run.physics,
        Default::default(),
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
    let collider = item_collider(&run, spare);
    run.area.interaction.step(
        FrameInput {
            pressed: keys::GRAB,
            ..Default::default()
        },
        ResolvedAim {
            grab_target: Some(AimTarget {
                collider,
                item: spare,
                distance: 0.5,
                point: pose.translation.to_array(),
                slot: None,
            }),
            ..Default::default()
        },
        &mut run.area.player,
        &mut run.area.items,
        &mut run.physics,
        1. / 60.,
        Default::default(),
    );
    assert_eq!(run.area.interaction.held(), Some(spare));
    let a = run.area.interaction.available_actions(
        &ResolvedAim {
            installation_target: Some(AimTarget {
                collider: run.wheels.sensor(slot.id).unwrap(),
                item: slot.rim,
                distance: 0.5,
                point: [0.; 3],
                slot: Some(SlotTarget::Wheel(slot.id)),
            }),
            ..Default::default()
        },
        &run.area.player,
        &run.area.items,
        &run.physics,
        Default::default(),
    );
    assert_eq!(a.actions.0 & ActionMask::MOUNT, 0);
}

#[test]
fn attached_tire_selects_tire_dismount() {
    let r = common::reference();
    let run = GarageRuntime::new(&r).unwrap();
    let tire = r.wheel_slots[0].tire;
    let a = run.area.interaction.available_actions(
        &ResolvedAim {
            focus_target: Some(AimTarget {
                collider: item_collider(&run, tire),
                item: tire,
                distance: 0.5,
                point: [0.; 3],
                slot: None,
            }),
            ..Default::default()
        },
        &run.area.player,
        &run.area.items,
        &run.physics,
        Default::default(),
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
        &ResolvedAim::default(),
        &run.area.player,
        &run.area.items,
        &run.physics,
        Default::default(),
    );
    assert_eq!(a.actions.0 & ActionMask::EXIT, 0);
    assert_eq!(run.area.player.seat(), Some(seat));
}

#[test]
fn mount_action_hides_drop_until_target_changes() {
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

    let collider = item_collider(&run, spare);
    run.area.interaction.step(
        FrameInput {
            pressed: keys::GRAB,
            ..Default::default()
        },
        ResolvedAim {
            grab_target: Some(AimTarget {
                collider,
                item: spare,
                distance: 0.5,
                point: pose.translation.to_array(),
                slot: None,
            }),
            ..Default::default()
        },
        &mut run.area.player,
        &mut run.area.items,
        &mut run.physics,
        1. / 60.,
        Default::default(),
    );
    assert_eq!(run.area.interaction.held(), Some(spare));
    let a = run.area.interaction.available_actions(
        &ResolvedAim {
            installation_target: Some(AimTarget {
                collider: run.wheels.sensor(slot.id).unwrap(),
                item: slot.rim,
                distance: 0.5,
                point: [0.; 3],
                slot: Some(SlotTarget::Wheel(slot.id)),
            }),
            ..Default::default()
        },
        &run.area.player,
        &run.area.items,
        &run.physics,
        Default::default(),
    );
    assert_ne!(a.actions.0 & ActionMask::MOUNT, 0);
    assert_eq!(a.actions.0 & ActionMask::DROP, 0);
}

fn item_collider(
    run: &GarageRuntime,
    item: my_summer_slop::reference::ItemId,
) -> rapier3d::prelude::ColliderHandle {
    run.physics
        .rapier()
        .colliders
        .iter()
        .find(|(_, c)| (c.user_data as u32) == item.0 + 1)
        .map(|(handle, _)| handle)
        .unwrap_or_else(|| {
            let body = run.area.items.get(item).unwrap().body.unwrap();
            *run.physics.rapier().bodies[body]
                .colliders()
                .first()
                .unwrap()
        })
}

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
    );
    assert_eq!(action, Some(InteractionAction::Throw(rim)));
    assert_eq!(run.area.interaction.held(), None);
    assert_eq!(
        run.area.items.get(rim).unwrap().location,
        ItemLocation::World
    );
    assert!(run.physics.rapier().bodies[body].linvel().length() > 0.);
}

mod lighting_merge {
    use super::common;
    use my_summer_slop::interaction::feedback::FeedbackContext;
    use my_summer_slop::reference::ReferenceManifest;

    #[test]
    fn switch_feedback_contract_requires_source_sprite_and_tank_guard() {
        assert_eq!(FeedbackContext::LightSwitch as u32, 10);
        let original: serde_json::Value =
            serde_json::from_slice(include_bytes!("common/reference.json")).unwrap();
        for missing in ["switch_icon", "no_drink"] {
            let mut json = original.clone();
            if missing == "switch_icon" {
                json["lighting"].as_object_mut().unwrap().remove(missing);
            } else {
                json["tanks"][0].as_object_mut().unwrap().remove(missing);
            }
            assert!(ReferenceManifest::from_json(&serde_json::to_vec(&json).unwrap()).is_err());
        }
        let mut r = common::reference();
        r.lighting.switch_icon.source.path_id = 4593;
        assert!(r.validate().is_err());
    }
}
