use super::super::*;
use super::WheelAction;
use super::fixture::*;

fn mass(f: &Fixture, item: ItemId) -> f32 {
    f.physics.rapier().bodies[f.body(item)].mass()
}
fn close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.0001, "{actual} != {expected}");
}

#[test]
fn wheel_actions_invalid_install_retains_carry() {
    let mut f = Fixture::new(false, false, true);
    f.hold(SPARE);
    let joint = f.physics.rapier().impulse_joints.iter().next().unwrap().0;
    let body = f.body(SPARE);
    assert!(f.wheels.can_install(SPARE, SLOT, &f.items).is_err());
    assert_eq!(
        f.action(WheelAction::Install {
            item: SPARE,
            target: SLOT
        }),
        Some("This wheel does not fit the slot.")
    );
    assert_eq!(f.interaction.held(), Some(SPARE));
    assert_eq!(f.items.get(SPARE).unwrap().location, ItemLocation::Held);
    assert_eq!(f.body(SPARE), body);
    assert!(f.physics.rapier().impulse_joints.get(joint).is_some());
    assert_eq!(f.physics.rapier().impulse_joints.len(), 1);
    assert!(!f.wheels.installed(SLOT));
    f.no_event();
}

#[test]
fn wheel_actions_install_failure_after_release_keeps_drop() {
    let mut f = Fixture::new(true, false, true);
    f.hold(SPARE);
    assert!(f.wheels.can_install(SPARE, SLOT, &f.items).is_ok());
    f.physics.remove_collider(f.colliders[2]);
    assert_eq!(
        f.action(WheelAction::Install {
            item: SPARE,
            target: SLOT
        }),
        Some("This wheel does not fit the slot.")
    );
    assert_eq!(f.interaction.held(), None);
    assert_eq!(f.items.get(SPARE).unwrap().location, ItemLocation::World);
    assert!(f.physics.rapier().impulse_joints.is_empty());
    assert!(!f.wheels.installed(SLOT));
    f.no_event();
}

#[test]
fn wheel_actions_install_rebinds_sensor() {
    let mut f = Fixture::new(true, true, true);
    assert_eq!(
        f.wheels.remove(SLOT, &mut f.items, &mut f.physics).unwrap(),
        RIM
    );
    let original_body = f.body(RIM);
    let spare_body = f.body(SPARE);
    assert_eq!(
        f.interaction.collider_bindings[&f.sensor].removal_owner,
        Some(RIM)
    );
    f.hold(SPARE);
    assert_eq!(
        f.action(WheelAction::Install {
            item: SPARE,
            target: SLOT
        }),
        Some("")
    );
    assert_eq!(f.interaction.held(), None);
    assert_eq!(
        f.interaction.collider_bindings[&f.sensor].removal_owner,
        Some(SPARE)
    );
    assert_eq!(
        f.physics.rapier().colliders[f.sensor].user_data as u32,
        SPARE.0 + 1
    );
    assert_eq!(f.items.get(SPARE).unwrap().id, SPARE);
    assert_eq!(
        f.items.get(SPARE).unwrap().location,
        ItemLocation::WheelSlot(SLOT)
    );
    let chassis = f.physics.chassis_handle(0);
    assert_eq!(f.body(SPARE), chassis);
    assert_eq!(
        f.physics.rapier().colliders[f.colliders[2]].parent(),
        Some(chassis)
    );
    assert!(!f.physics.rapier().bodies[spare_body].is_enabled());
    assert_eq!(f.body(RIM), original_body);
    assert_eq!(
        f.items.get(TIRE).unwrap().location,
        ItemLocation::Attached(RIM)
    );
    close(mass(&f, SPARE), 103.);
    close(mass(&f, RIM), 12.);
    assert_eq!(
        f.event(),
        InteractionAction::Wheel(WheelAction::Install {
            item: SPARE,
            target: SLOT
        })
    );
    f.no_event();
}

#[test]
fn wheel_actions_remove_tire_keeps_completion_after_pickup_failure() {
    let mut f = Fixture::new(true, true, false);
    let chassis = f.physics.chassis_handle(0);
    assert_eq!(
        f.action(WheelAction::Remove(TIRE)),
        Some("Cannot carry this wheel.")
    );
    assert!(!f.wheels.installed(SLOT));
    assert_eq!(f.items.get(RIM).unwrap().location, ItemLocation::World);
    assert_eq!(
        f.items.get(TIRE).unwrap().location,
        ItemLocation::Attached(RIM)
    );
    assert_ne!(f.body(RIM), chassis);
    assert_eq!(f.body(TIRE), f.body(RIM));
    assert_eq!(f.interaction.held(), None);
    close(mass(&f, RIM), 12.);
    assert_eq!(
        f.event(),
        InteractionAction::Wheel(WheelAction::Remove(RIM))
    );
    f.no_event();
}

#[test]
fn wheel_actions_remove_without_slot_preserves_notice() {
    let mut f = Fixture::new(true, false, true);
    let states: Vec<_> = f
        .items
        .iter()
        .map(|i| (i.id, i.location, i.body, i.pose))
        .collect();
    f.interaction
        .completed_action(InteractionAction::PartRemove(SPARE));
    assert_eq!(f.action(WheelAction::Remove(RIM)), None);
    assert_eq!(
        states,
        f.items
            .iter()
            .map(|i| (i.id, i.location, i.body, i.pose))
            .collect::<Vec<_>>()
    );
    assert_eq!(f.event(), InteractionAction::PartRemove(SPARE));
    f.no_event();
}

#[test]
fn wheel_actions_failed_split_and_join_keep_drop() {
    for (action, notice) in [
        (WheelAction::Split(SPARE), "Cannot dismount this tire."),
        (
            WheelAction::Join {
                rim: SPARE,
                tire: TIRE,
            },
            "This tire does not fit the rim.",
        ),
    ] {
        let mut f = Fixture::new(true, false, true);
        f.hold(SPARE);
        assert_eq!(f.action(action), Some(notice));
        assert_eq!(f.interaction.held(), None);
        assert_eq!(f.items.get(SPARE).unwrap().location, ItemLocation::World);
        assert!(f.physics.rapier().impulse_joints.is_empty());
        assert_eq!(
            f.items.get(TIRE).unwrap().location,
            ItemLocation::Attached(RIM)
        );
        f.no_event();
    }
}

#[test]
fn wheel_actions_split_and_join_preserve_state() {
    let mut f = Fixture::new(true, false, true);
    let rim_body = f.body(RIM);
    let linear = Vector::new(1., 2., 3.);
    let angular = Vector::new(0., 1., 0.);
    f.physics.edit_body(rim_body, |body| {
        body.set_linvel(linear, true);
        body.set_angvel(angular, true);
    });
    let tire_pose = f.items.get(TIRE).unwrap().pose;
    let expected_linear = f.physics.rapier().bodies[rim_body]
        .velocity_at_point(Vector::from_array(tire_pose.position));
    f.hold(RIM);
    assert_eq!(f.action(WheelAction::Split(RIM)), Some(""));
    assert_eq!(f.interaction.held(), None);
    let tire_body = f.body(TIRE);
    assert_ne!(tire_body, rim_body);
    assert_eq!(f.body(RIM), rim_body);
    assert_eq!(f.items.get(RIM).unwrap().id, RIM);
    assert_eq!(f.items.get(TIRE).unwrap().id, TIRE);
    assert_eq!(f.items.get(TIRE).unwrap().location, ItemLocation::World);
    assert_eq!(f.items.get(TIRE).unwrap().pose, tire_pose);
    assert_eq!(
        f.physics.rapier().colliders[f.colliders[0]].parent(),
        Some(rim_body)
    );
    assert_eq!(
        f.physics.rapier().colliders[f.colliders[1]].parent(),
        Some(tire_body)
    );
    assert_eq!(
        f.physics.rapier().bodies[tire_body].linvel(),
        expected_linear
    );
    assert_eq!(f.physics.rapier().bodies[tire_body].angvel(), angular);
    assert_eq!(f.physics.rapier().bodies[rim_body].linvel(), linear);
    assert_eq!(f.physics.rapier().bodies[rim_body].angvel(), angular);
    close(mass(&f, RIM), 5.);
    close(mass(&f, TIRE), 7.);
    assert_eq!(f.event(), InteractionAction::PartRemove(TIRE));
    f.no_event();
    assert_eq!(
        f.action(WheelAction::Join {
            rim: RIM,
            tire: TIRE
        }),
        Some("")
    );
    assert_eq!(f.items.get(TIRE).unwrap().id, TIRE);
    assert_eq!(
        f.items.get(TIRE).unwrap().location,
        ItemLocation::Attached(RIM)
    );
    assert_eq!(f.body(TIRE), rim_body);
    assert_eq!(
        f.physics.rapier().colliders[f.colliders[1]].parent(),
        Some(rim_body)
    );
    assert!(!f.physics.rapier().bodies[tire_body].is_enabled());
    close(mass(&f, RIM), 12.);
    assert_eq!(f.physics.rapier().bodies[rim_body].linvel(), linear);
    assert_eq!(f.physics.rapier().bodies[rim_body].angvel(), angular);
    assert_eq!(f.interaction.held(), None);
    assert!(f.physics.rapier().impulse_joints.is_empty());
    assert_eq!(
        f.event(),
        InteractionAction::Wheel(WheelAction::Join {
            rim: RIM,
            tire: TIRE
        })
    );
    f.no_event();
}

#[test]
fn wheel_actions_split_uses_original_input_and_aim() {
    let mut f = Fixture::new(true, false, true);
    let grab = f.target(RIM, [f32::NAN; 3]);
    assert_eq!(
        f.complete(
            WheelAction::Split(RIM),
            FrameInput {
                pressed: keys::ASSEMBLE,
                ..FrameInput::default()
            },
            ResolvedAim {
                grab_target: Some(grab),
                ..ResolvedAim::default()
            }
        ),
        Some("")
    );
    assert_eq!(f.interaction.held(), None);
    assert_eq!(f.event(), InteractionAction::PartRemove(TIRE));
    f.no_event();
    // Invalid points prove focus, grab, then tire-pose precedence through real pickup validation.
    for (focus, grab, succeeds) in [
        (Some([0.; 3]), Some([f32::NAN; 3]), true),
        (Some([f32::NAN; 3]), Some([0.; 3]), false),
        (None, Some([f32::NAN; 3]), false),
        (None, None, true),
    ] {
        let mut f = Fixture::new(true, false, true);
        let aim = ResolvedAim {
            focus_target: focus.map(|point| f.target(RIM, point)),
            grab_target: grab.map(|point| f.target(RIM, point)),
            ..ResolvedAim::default()
        };
        assert_eq!(
            f.complete(
                WheelAction::Split(RIM),
                FrameInput {
                    pressed: keys::GRAB,
                    ..FrameInput::default()
                },
                aim
            ),
            Some(if succeeds {
                ""
            } else {
                "Cannot carry this tire."
            })
        );
        assert_eq!(f.interaction.held(), succeeds.then_some(TIRE));
        assert_eq!(
            f.items.get(TIRE).unwrap().location,
            if succeeds {
                ItemLocation::Held
            } else {
                ItemLocation::World
            }
        );
        assert_ne!(f.body(TIRE), f.body(RIM));
        assert_eq!(f.event(), InteractionAction::PartRemove(TIRE));
        f.no_event();
    }
}

#[test]
fn wheel_actions_late_drop_replaces_split() {
    let mut f = Fixture::new(true, false, true);
    f.hold(RIM);
    let input = FrameInput {
        pressed: keys::ASSEMBLE,
        ..FrameInput::default()
    };
    let aim = ResolvedAim::default();
    assert_ne!(
        f.interaction
            .available_actions(
                &aim,
                &f.player,
                &f.items,
                &f.physics,
                crate::survival::NeedsOutput::default()
            )
            .actions
            .0
            & feedback::ActionMask::SPLIT,
        0
    );
    let rim_body = f.body(RIM);
    f.physics.edit_body(rim_body, |body| {
        body.set_translation(Vector::new(30., 0., 0.), true)
    });
    let action = f.step(input, aim);
    assert_eq!(action, Some(InteractionAction::Drop(RIM)));
    if let Some(InteractionAction::Wheel(action)) = action {
        f.complete(action, input, aim);
    }
    assert_eq!(f.items.get(RIM).unwrap().location, ItemLocation::World);
    assert_eq!(
        f.items.get(TIRE).unwrap().location,
        ItemLocation::Attached(RIM)
    );
    assert_eq!(f.body(TIRE), rim_body);
    assert!(f.physics.rapier().impulse_joints.is_empty());
    f.no_event();
}
