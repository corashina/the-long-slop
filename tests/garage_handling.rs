//! Source build 11140890 action order and input ownership.
use my_summer_slop::{
    interaction::{
        AimTarget, InteractionAction, InteractionBlock, InteractionOutcome, ResolvedAim,
        feedback::ActionMask,
    },
    reference::ReferenceManifest,
    simulation::{FIXED_DT, FrameInput, keys},
    starting_area::GarageRuntime,
    vehicle::VehiclePose,
};
use rapier3d::math::{Rotation, Vector};

fn source_scene() -> (ReferenceManifest, GarageRuntime) {
    let reference = ReferenceManifest::from_runtime_json(
        &std::fs::read("web/assets/reference/manifest.json").unwrap(),
    )
    .unwrap();
    assert_eq!(reference.build_id, 11140890);
    let run = GarageRuntime::new(&reference).unwrap();
    (reference, run)
}
fn step_input(run: &mut GarageRuntime, input: FrameInput, aim: ResolvedAim) -> InteractionOutcome {
    run.area.interaction.step(
        input,
        aim,
        &mut run.area.player,
        &mut run.area.items,
        &mut run.physics,
        FIXED_DT,
        run.survival.output(),
        1.,
    )
}
fn ray_aim(
    run: &GarageRuntime,
    collider: rapier3d::prelude::ColliderHandle,
    accepts: impl Fn(&ResolvedAim) -> bool,
) -> ResolvedAim {
    let center = run.physics.rapier().colliders[collider]
        .compute_aabb()
        .center();
    [
        Vector::X,
        -Vector::X,
        Vector::Y,
        -Vector::Y,
        Vector::Z,
        -Vector::Z,
    ]
    .into_iter()
    .find_map(|axis| {
        let view = VehiclePose {
            position: (center + axis * 1.25).to_array(),
            rotation: Rotation::from_rotation_arc(-Vector::Z, -axis).to_array(),
        };
        let aim = run
            .area
            .interaction
            .resolve(&run.physics, view, &run.area.items);
        accepts(&aim).then_some(aim)
    })
    .expect("source collider must resolve separate action owners")
}
#[test]
#[ignore = "requires build 11140890 export"]
fn action_priority_and_input_ownership() {
    let (reference, mut run) = source_scene();
    let part = reference
        .items
        .iter()
        .find(|i| i.name == "FRDoor")
        .unwrap()
        .id;
    let door = reference
        .items
        .iter()
        .find(|i| i.name == "Collider" && i.attached_to == Some(part))
        .unwrap()
        .id;
    let binding = reference
        .action_bindings
        .iter()
        .find(|b| b.use_owner == Some(door) && b.pickup_owner == Some(part))
        .unwrap();
    let collider = run
        .physics
        .rapier()
        .colliders
        .iter()
        .find(|(_, c)| (c.user_data >> 64) as u32 == binding.collider.0)
        .unwrap()
        .0;
    let aim = ray_aim(&run, collider, |a| {
        a.use_target.is_some_and(|t| t.item == door)
            && a.grab_target.is_some_and(|t| t.item == part)
    });
    assert_ne!(aim.use_target.unwrap().item, aim.grab_target.unwrap().item);
    // RayStuff retains an active usable before the later mount and pickup passes.
    step_input(
        &mut run,
        FrameInput {
            held: keys::PRIMARY,
            mouse: [0.2, 0.1],
            ..Default::default()
        },
        aim,
    );
    let available = run.area.interaction.available_actions(
        &aim,
        &run.area.player,
        &run.area.items,
        &run.physics,
        run.survival.output(),
    );
    assert_eq!(
        available.actions.0 & ActionMask::DISMOUNT,
        0,
        "active drag owns F in feedback"
    );
    let outcome = step_input(
        &mut run,
        FrameInput {
            held: keys::PRIMARY | keys::GRAB,
            pressed: keys::GRAB,
            mouse: [0.3, 0.2],
            ..Default::default()
        },
        aim,
    );
    assert_eq!(outcome.action, Some(InteractionAction::Mechanism(door)));
    assert_eq!(outcome.look_mouse, [0.0; 2]);
    assert!(run.area.interaction.held().is_none());
    assert!(run.area.interaction.parts.contains(part));
    // Without an active drag, the earlier part pass claims simultaneous F and LMB.
    step_input(&mut run, FrameInput::default(), aim);
    let available = run.area.interaction.available_actions(
        &aim,
        &run.area.player,
        &run.area.items,
        &run.physics,
        run.survival.output(),
    );
    assert_ne!(
        available.actions.0 & ActionMask::DISMOUNT,
        0,
        "ending drag restores removal feedback"
    );
    let outcome = step_input(
        &mut run,
        FrameInput {
            held: keys::PRIMARY | keys::GRAB,
            pressed: keys::GRAB,
            ..Default::default()
        },
        aim,
    );
    assert_eq!(outcome.action, Some(InteractionAction::PartRemove(part)));
    assert_eq!(run.area.interaction.held(), Some(part));

    let (reference, mut run) = source_scene();
    let loose = reference
        .items
        .iter()
        .find(|i| i.name == "Kolbasz")
        .unwrap()
        .id;
    let view = run.area.player.view_pose(&run.physics);
    let item = run.area.items.get(loose).unwrap();
    let shift =
        Vector::from_array(view.position) - Vector::Z - Vector::from_array(item.pose.position);
    run.physics.edit_body(item.body.unwrap(), |body| {
        body.set_translation(body.translation() + shift, true)
    });
    run.area.items.sync_poses(&run.physics);
    run.physics.refresh_queries();
    let binding = reference
        .action_bindings
        .iter()
        .find(|b| b.pickup_owner == Some(loose) && b.use_owner.is_none())
        .unwrap();
    let collider = run
        .physics
        .rapier()
        .colliders
        .iter()
        .find(|(_, c)| (c.user_data >> 64) as u32 == binding.collider.0)
        .unwrap()
        .0;
    let point = run.physics.rapier().colliders[collider]
        .compute_aabb()
        .center()
        .to_array();
    let aim = ResolvedAim {
        grab_target: Some(AimTarget {
            collider,
            item: loose,
            distance: 1.,
            point,
            slot: None,
        }),
        ..Default::default()
    };
    let cap = reference.openings.iter().find(|o| o.can_tune).unwrap().cap;
    let cap_binding = reference
        .action_bindings
        .iter()
        .find(|b| b.use_owner == Some(cap))
        .unwrap();
    let cap_collider = run
        .physics
        .rapier()
        .colliders
        .iter()
        .find(|(_, c)| (c.user_data >> 64) as u32 == cap_binding.collider.0)
        .unwrap()
        .0;
    let tune_aim = ResolvedAim {
        use_target: Some(AimTarget {
            item: cap,
            collider: cap_collider,
            point,
            distance: 1.,
            slot: None,
        }),
        ..Default::default()
    };
    let both = ResolvedAim {
        use_target: tune_aim.use_target,
        ..aim
    };
    let outcome = step_input(
        &mut run,
        FrameInput {
            held: keys::USE | keys::GRAB,
            pressed: keys::USE,
            mouse: [0., 0.2],
            ..Default::default()
        },
        both,
    );
    assert!(matches!(outcome.action, Some(InteractionAction::Cap(_))));
    assert!(
        run.area.interaction.held().is_none(),
        "later F pickup cannot overwrite use"
    );
    let outcome = step_input(
        &mut run,
        FrameInput {
            held: keys::GRAB,
            mouse: [0.5, -0.25],
            ..Default::default()
        },
        aim,
    );
    assert_eq!(
        outcome.action,
        Some(InteractionAction::Pickup(loose)),
        "ordinary pickup accepts held F"
    );
    assert_eq!(outcome.look_mouse, [0.5, -0.25]);
    step_input(&mut run, FrameInput::default(), ResolvedAim::default());
    let blocked = ResolvedAim {
        installation_block: Some(InteractionBlock::Incompatible),
        ..Default::default()
    };
    let available = run.area.interaction.available_actions(
        &blocked,
        &run.area.player,
        &run.area.items,
        &run.physics,
        run.survival.output(),
    );
    assert_ne!(available.actions.0 & ActionMask::DROP, 0);
    step_input(
        &mut run,
        FrameInput {
            held: keys::GRAB,
            pressed: keys::GRAB,
            ..Default::default()
        },
        blocked,
    );
    let outcome = step_input(
        &mut run,
        FrameInput {
            released: keys::GRAB,
            ..Default::default()
        },
        blocked,
    );
    assert_eq!(outcome.action, Some(InteractionAction::Drop(loose)));
    // Reset must suppress held controls and their later releases until neutral.
    run.area.interaction.cancel_input();
    let held = keys::GRAB | keys::RECENTER | keys::TRANSLATE | keys::ROTATE;
    let outcome = step_input(
        &mut run,
        FrameInput {
            held,
            pressed: held,
            mouse: [1., 2.],
            wheel: 1.,
            ..Default::default()
        },
        aim,
    );
    assert!(outcome.action.is_none());
    assert!(run.area.interaction.held().is_none());
    step_input(
        &mut run,
        FrameInput {
            released: held,
            ..Default::default()
        },
        aim,
    );
    assert!(run.area.interaction.held().is_none());
    let outcome = step_input(
        &mut run,
        FrameInput {
            held: keys::GRAB,
            ..Default::default()
        },
        aim,
    );
    assert_eq!(outcome.action, Some(InteractionAction::Pickup(loose)));
    step_input(&mut run, FrameInput::default(), ResolvedAim::default());
    let ids = my_summer_slop::save::IdentityMap::new(&reference).unwrap();
    let before = run.area.interaction.capture_carry_save(&ids).unwrap();
    run.area.interaction.cancel_input();
    let outcome = step_input(
        &mut run,
        FrameInput {
            held,
            pressed: held,
            mouse: [1., 2.],
            wheel: 1.,
            ..Default::default()
        },
        ResolvedAim::default(),
    );
    assert!(outcome.action.is_none());
    assert_eq!(run.area.interaction.held(), Some(loose));
    let outcome = step_input(
        &mut run,
        FrameInput {
            released: held,
            ..Default::default()
        },
        ResolvedAim::default(),
    );
    assert!(outcome.action.is_none());
    assert_eq!(run.area.interaction.held(), Some(loose));
    assert_eq!(
        run.area.interaction.capture_carry_save(&ids).unwrap(),
        before
    );
    for modifier in [keys::RECENTER, keys::TRANSLATE, keys::ROTATE] {
        run.area.interaction.cancel_input();
        let before = run.area.interaction.capture_carry_save(&ids).unwrap();
        let outcome = step_input(
            &mut run,
            FrameInput {
                held: modifier,
                pressed: modifier,
                mouse: [1., 2.],
                wheel: 0.02,
                ..Default::default()
            },
            ResolvedAim::default(),
        );
        assert!(outcome.action.is_none());
        step_input(
            &mut run,
            FrameInput {
                released: modifier,
                ..Default::default()
            },
            ResolvedAim::default(),
        );
        assert_eq!(
            run.area.interaction.capture_carry_save(&ids).unwrap(),
            before,
            "reset suppresses each held handling control"
        );
    }
    let body = run.area.items.get(loose).unwrap().body.unwrap();
    let anchor = run
        .physics
        .rapier()
        .impulse_joints
        .iter()
        .find_map(|(_, joint)| {
            (joint.body2() == body && run.physics.rapier().bodies[joint.body1()].is_kinematic())
                .then_some(joint.body1())
        })
        .unwrap();
    let anchor_before = run.physics.rapier().bodies[anchor]
        .next_position()
        .translation;
    run.area.player.rebase([0.05, 0., 0.]);
    let outcome = step_input(
        &mut run,
        FrameInput {
            held: keys::USE,
            mouse: [0.3, 0.2],
            ..Default::default()
        },
        tune_aim,
    );
    assert_eq!(outcome.look_mouse, [0.; 2]);
    assert_ne!(
        run.physics.rapier().bodies[anchor]
            .next_position()
            .translation,
        anchor_before,
        "carry constraint still follows the player while tuning owns the mouse"
    );
    // Source wheel distance blocks on tuning. RMB overrides that block. MMB always blocks.
    for modifiers in [0, keys::TRANSLATE, keys::TRANSLATE | keys::ROTATE] {
        step_input(&mut run, FrameInput::default(), ResolvedAim::default());
        let before = run
            .area
            .interaction
            .capture_carry_save(&ids)
            .unwrap()
            .unwrap();
        let outcome = step_input(
            &mut run,
            FrameInput {
                held: keys::USE | modifiers,
                mouse: [0.3, 0.2],
                wheel: 0.02,
                ..Default::default()
            },
            tune_aim,
        );
        let after = run
            .area
            .interaction
            .capture_carry_save(&ids)
            .unwrap()
            .unwrap();
        assert_eq!(outcome.look_mouse, [0.; 2]);
        assert_eq!(after.rotation, before.rotation, "tuning owns the mouse");
        if modifiers == keys::TRANSLATE {
            assert_ne!(
                after.offset, before.offset,
                "RMB permits carry wheel distance"
            );
        } else {
            assert_eq!(
                after.offset, before.offset,
                "tuning or MMB blocks carry wheel distance"
            );
        }
    }
    step_input(&mut run, FrameInput::default(), ResolvedAim::default());
    let door = reference
        .items
        .iter()
        .find(|i| i.name == "FRDoor")
        .unwrap()
        .id;
    let slide = reference
        .items
        .iter()
        .find(|i| i.name == "Handle" && i.attached_to == Some(door))
        .unwrap()
        .id;
    let binding = reference
        .action_bindings
        .iter()
        .find(|b| b.use_owner == Some(slide))
        .unwrap();
    let collider = run
        .physics
        .rapier()
        .colliders
        .iter()
        .find(|(_, c)| (c.user_data >> 64) as u32 == binding.collider.0)
        .unwrap()
        .0;
    let slide_aim = ResolvedAim {
        use_target: Some(AimTarget {
            item: slide,
            collider,
            point,
            distance: 1.,
            slot: None,
        }),
        ..Default::default()
    };
    let before = run
        .area
        .interaction
        .capture_carry_save(&ids)
        .unwrap()
        .unwrap();
    let outcome = step_input(
        &mut run,
        FrameInput {
            wheel: 0.02,
            mouse: [0.3, 0.2],
            ..Default::default()
        },
        slide_aim,
    );
    let after = run
        .area
        .interaction
        .capture_carry_save(&ids)
        .unwrap()
        .unwrap();
    assert_eq!(outcome.action, Some(InteractionAction::Mechanism(slide)));
    assert_eq!(outcome.look_mouse, [0.; 2]);
    assert_ne!(
        before.offset, after.offset,
        "source slide scroll shares wheel distance when tuning is absent"
    );
    assert_eq!(before.rotation, after.rotation);
}
