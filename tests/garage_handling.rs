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

fn place_for_carry(
    run: &mut GarageRuntime,
    item: my_summer_slop::reference::ItemId,
) -> ResolvedAim {
    let view = run.area.player.view_pose(&run.physics);
    let state = run.area.items.get(item).unwrap();
    let body = state.body.unwrap();
    let desired =
        Vector::from_array(view.position) + Rotation::from_array(view.rotation) * -Vector::Z;
    let shift = desired - Vector::from_array(state.pose.position);
    run.physics.edit_body(body, |body| {
        body.set_translation(body.translation() + shift, true);
        body.set_linvel(Vector::ZERO, true);
    });
    run.physics.recompute_body_mass(body);
    run.area.items.sync_poses(&run.physics);
    run.physics.refresh_queries();
    let collider = run.physics.rapier().bodies[body].colliders()[0];
    ResolvedAim {
        grab_target: Some(AimTarget {
            item,
            collider,
            distance: 1.,
            point: desired.to_array(),
            slot: None,
        }),
        ..Default::default()
    }
}
fn acquire(run: &mut GarageRuntime, item: my_summer_slop::reference::ItemId) {
    let aim = place_for_carry(run, item);
    let out = step_input(
        run,
        FrameInput {
            held: keys::GRAB,
            pressed: keys::GRAB,
            released: keys::GRAB,
            ..Default::default()
        },
        aim,
    );
    assert_eq!(out.action, Some(InteractionAction::Pickup(item)));
    assert_eq!(
        run.area.interaction.held(),
        Some(item),
        "same-frame F release retains acquisition"
    );
    let out = step_input(
        run,
        FrameInput {
            released: keys::GRAB,
            ..Default::default()
        },
        ResolvedAim::default(),
    );
    assert_eq!(out.action, None);
    assert_eq!(run.area.interaction.held(), Some(item));
}
fn anchor_handle(
    run: &GarageRuntime,
    item: my_summer_slop::reference::ItemId,
) -> rapier3d::prelude::RigidBodyHandle {
    let body = run.area.items.get(item).unwrap().body.unwrap();
    run.physics
        .rapier()
        .impulse_joints
        .iter()
        .find_map(|(_, joint)| {
            (joint.body2() == body && run.physics.rapier().bodies[joint.body1()].is_kinematic())
                .then_some(joint.body1())
        })
        .unwrap()
}
fn near_rotation(actual: Rotation, expected: Rotation) {
    assert!(
        actual.dot(expected).abs() > 0.99999,
        "{actual:?} != {expected:?}"
    );
}

#[test]
#[ignore = "requires build 11140890 export"]
fn carry_release_and_pose_follow_source() {
    use my_summer_slop::interaction::items::ItemLocation;
    let (reference, mut run) = source_scene();
    // Keep collision-free release assertions separate from the collision case below.
    run.area.player.rebase([0., -50., 0.]);
    let item = reference
        .items
        .iter()
        .find(|i| i.name == "Kolbasz")
        .unwrap()
        .id;
    let body = run.area.items.get(item).unwrap().body.unwrap();
    let blocked = ResolvedAim {
        installation_block: Some(InteractionBlock::Incompatible),
        ..Default::default()
    };
    for _ in 0..3 {
        acquire(&mut run, item);
        let available = run.area.interaction.available_actions(
            &blocked,
            &run.area.player,
            &run.area.items,
            &run.physics,
            run.survival.output(),
        );
        assert_ne!(available.actions.0 & ActionMask::DROP, 0);
        assert_ne!(available.actions.0 & ActionMask::THROW, 0);
        step_input(
            &mut run,
            FrameInput {
                held: keys::GRAB,
                pressed: keys::GRAB,
                ..Default::default()
            },
            blocked,
        );
        let out = step_input(
            &mut run,
            FrameInput {
                released: keys::GRAB,
                ..Default::default()
            },
            blocked,
        );
        assert_eq!(out.action, Some(InteractionAction::Drop(item)));
        assert_eq!(run.area.items.get(item).unwrap().body, Some(body));
        assert_eq!(
            run.area.items.get(item).unwrap().location,
            ItemLocation::World
        );
    }
    acquire(&mut run, item);
    let mut maximum = 0f32;
    let mut previous = 0f32;
    let mut fell = false;
    for frame in 0..100 {
        step_input(
            &mut run,
            FrameInput {
                held: keys::GRAB,
                pressed: if frame == 0 { keys::GRAB } else { 0 },
                ..Default::default()
            },
            blocked,
        );
        let charge = run.area.interaction.throw_charge();
        maximum = maximum.max(charge);
        fell |= charge < previous;
        previous = charge;
    }
    assert!(maximum >= 1. && fell);
    assert!(previous <= reference.player.throw_minimum);
    let outcome = step_input(
        &mut run,
        FrameInput {
            released: keys::GRAB,
            ..Default::default()
        },
        blocked,
    );
    assert_eq!(outcome.action, None);
    assert_eq!(run.area.interaction.held(), Some(item));
    // Neutral re-arms a new gesture after the canceled falling cycle.
    step_input(&mut run, FrameInput::default(), blocked);
    for frame in 0..12 {
        step_input(
            &mut run,
            FrameInput {
                held: keys::GRAB,
                pressed: if frame == 0 { keys::GRAB } else { 0 },
                ..Default::default()
            },
            blocked,
        );
    }
    let velocity = Vector::new(2., 3., 4.);
    run.physics
        .edit_body(body, |body| body.set_linvel(velocity, true));
    let charge = run.area.interaction.throw_charge();
    let direction =
        Rotation::from_array(run.area.player.view_pose(&run.physics).rotation) * -Vector::Z;
    let mass = run.physics.rapier().bodies[body].mass();
    let out = step_input(
        &mut run,
        FrameInput {
            released: keys::GRAB,
            ..Default::default()
        },
        blocked,
    );
    assert_eq!(out.action, Some(InteractionAction::Throw(item)));
    let expected = velocity
        + direction
            * charge
            * reference.player.throw_multiplier
            * reference.player.source_force_interval
            / mass;
    assert!(
        (run.physics.rapier().bodies[body].linvel() - expected).length() < 0.001,
        "actual {:?}, expected {expected:?}, mass {mass}, charge {charge}",
        run.physics.rapier().bodies[body].linvel()
    );
    acquire(&mut run, item);
    let target = *run.physics.rapier().bodies[anchor_handle(&run, item)].next_position();
    // The source comparison includes equality at the authored drop distance.
    run.physics.edit_body(body, |body| {
        body.set_translation(
            target.translation + Vector::X * reference.player.carry_drop_distance,
            true,
        )
    });
    let out = step_input(&mut run, FrameInput::default(), ResolvedAim::default());
    assert_eq!(out.action, Some(InteractionAction::Drop(item)));
    assert_eq!(
        run.area.items.get(item).unwrap().location,
        ItemLocation::World
    );
    acquire(&mut run, item);
    let anchor = anchor_handle(&run, item);
    let collider = run.physics.rapier().bodies[body].colliders()[0];
    let shape = &run.physics.rapier().colliders[collider];
    let groups = shape.collision_groups();
    let bounds = shape.compute_aabb();
    let view = Rotation::from_array(run.area.player.view_pose(&run.physics).rotation);
    let right = view * Vector::X;
    let center = bounds.center() + right * (bounds.half_extents().dot(right.abs()) + 0.04);
    let wall = run.physics.insert_collider(
        rapier3d::prelude::ColliderBuilder::cuboid(0.005, 2., 2.)
            .translation(center)
            .rotation(view.to_scaled_axis())
            .collision_groups(groups),
    );
    run.physics.refresh_queries();
    let current = *run.physics.rapier().bodies[body].position();
    step_input(
        &mut run,
        FrameInput {
            held: keys::TRANSLATE,
            mouse: [100., 0.],
            ..Default::default()
        },
        ResolvedAim::default(),
    );
    let desired = *run.physics.rapier().bodies[anchor].next_position();
    let shift = (desired.translation - current.translation).dot(right);
    assert!(
        shift > 0. && shift < 0.04,
        "wall must constrain carry translation: {shift}"
    );
    let shape = &run.physics.rapier().colliders[collider];
    let wall = &run.physics.rapier().colliders[wall];
    let local = shape.position_wrt_parent().copied().unwrap();
    assert!(
        !rapier3d::parry::query::intersection_test(
            &(desired * local),
            shape.shape(),
            wall.position(),
            wall.shape()
        )
        .unwrap()
        .intersecting
    );
    assert_eq!(shape.collision_groups(), groups);
    assert!(shape.is_enabled());
    assert!(run.physics.rapier().bodies[body].is_ccd_enabled());
}

#[test]
#[ignore = "requires build 11140890 export"]
fn carry_pose_uses_source_frames_and_gestures() {
    use my_summer_slop::{reference::ItemId, save::IdentityMap};
    let (reference, mut run) = source_scene();
    run.area.player.rebase([0., -50., 0.]);
    let ids = IdentityMap::new(&reference).unwrap();
    let item = ItemId(120);
    assert_eq!(
        reference.items.iter().find(|i| i.id == item).unwrap().name,
        "OilCan"
    );
    acquire(&mut run, item);
    let anchor = anchor_handle(&run, item);
    let before = run.physics.rapier().bodies[anchor].next_position().rotation;
    let available = run.area.interaction.available_actions(
        &ResolvedAim::default(),
        &run.area.player,
        &run.area.items,
        &run.physics,
        run.survival.output(),
    );

    run.area.player.step(
        FrameInput {
            mouse: [0., 12.],
            ..Default::default()
        },
        &mut run.physics,
        FIXED_DT,
    );
    step_input(&mut run, FrameInput::default(), ResolvedAim::default());
    near_rotation(
        run.physics.rapier().bodies[anchor].next_position().rotation,
        before,
    );
    assert_ne!(
        available.actions.0 & ActionMask::ROTATE,
        0,
        "noRot still permits manual rotation"
    );
    let root_before = Rotation::from_array(run.area.player.body_pose(&run.physics).rotation);
    run.area.player.follow_push_grip(8.);
    let root_after = Rotation::from_array(run.area.player.body_pose(&run.physics).rotation);
    step_input(&mut run, FrameInput::default(), ResolvedAim::default());
    near_rotation(
        run.physics.rapier().bodies[anchor].next_position().rotation,
        root_after * root_before.inverse() * before,
    );
    // Manual rotations use head axes, even when the stored frame is the Player root.
    let body = run.area.items.get(item).unwrap().body.unwrap();
    let mass = run.physics.rapier().bodies[body].mass();
    let gain = ((mass - reference.player.carry_max_weight * 2.)
        / (reference.player.carry_min_weight - reference.player.carry_max_weight * 2.))
        .clamp(0., 1.);
    let angle = (FIXED_DT * 10. * reference.player.obj_rotation_sensitivity * gain).to_radians();
    for (buttons, mouse, axis, sign) in [
        (keys::ROTATE, [1., 0.], Vector::Y, 1.),
        (keys::ROTATE, [0., 1.], Vector::X, 1.),
        (keys::ROTATE | keys::TRANSLATE, [1., 0.], Vector::Z, -1.),
    ] {
        let before = run
            .area
            .interaction
            .capture_carry_save(&ids)
            .unwrap()
            .unwrap();
        let rotation_before = run.physics.rapier().bodies[anchor].next_position().rotation;
        let view = Rotation::from_array(run.area.player.view_pose(&run.physics).rotation);
        let out = step_input(
            &mut run,
            FrameInput {
                held: buttons,
                mouse,
                ..Default::default()
            },
            ResolvedAim::default(),
        );
        let after = run
            .area
            .interaction
            .capture_carry_save(&ids)
            .unwrap()
            .unwrap();
        assert_eq!(out.look_mouse, [0.; 2]);
        assert_eq!(after.offset, before.offset, "MMB wins over RMB translation");
        near_rotation(
            run.physics.rapier().bodies[anchor].next_position().rotation,
            Rotation::from_axis_angle(view * axis, angle * sign) * rotation_before,
        );
    }
    let before = run
        .area
        .interaction
        .capture_carry_save(&ids)
        .unwrap()
        .unwrap();
    step_input(
        &mut run,
        FrameInput {
            held: keys::TRANSLATE,
            mouse: [1., -1.],
            ..Default::default()
        },
        ResolvedAim::default(),
    );
    let after = run
        .area
        .interaction
        .capture_carry_save(&ids)
        .unwrap()
        .unwrap();
    assert_ne!(after.offset, before.offset);
    assert_eq!(after.rotation, before.rotation);
    // The authored radius bounds all distance and translation input.
    step_input(
        &mut run,
        FrameInput {
            held: keys::TRANSLATE,
            mouse: [10000., -10000.],
            wheel: 10000.,
            ..Default::default()
        },
        ResolvedAim::default(),
    );
    let saved = run
        .area
        .interaction
        .capture_carry_save(&ids)
        .unwrap()
        .unwrap();
    assert!(
        (Vector::from_array(saved.offset).length() - reference.player.carry_offset_radius).abs()
            < 0.0001
    );
    // Recenter uses the view pose and preserves the existing offset formula.
    step_input(
        &mut run,
        FrameInput {
            held: keys::RECENTER,
            ..Default::default()
        },
        ResolvedAim::default(),
    );
    let recentered = run
        .area
        .interaction
        .capture_carry_save(&ids)
        .unwrap()
        .unwrap();
    let rule = reference
        .items
        .iter()
        .find(|i| i.id == item)
        .unwrap()
        .pickup
        .as_ref()
        .unwrap();
    assert_eq!(
        recentered.offset,
        (Vector::from_array(reference.player.carry_anchor_offset) * rule.recenter_distance)
            .to_array()
    );
    let grip = Rotation::from_array(recentered.local_grip.rotation);
    let view = Rotation::from_array(run.area.player.view_pose(&run.physics).rotation);
    near_rotation(
        run.physics.rapier().bodies[anchor].next_position().rotation,
        view * grip.inverse(),
    );
    run.area
        .interaction
        .drop_held(&mut run.area.items, &mut run.physics);
    let ordinary = reference
        .items
        .iter()
        .find(|i| i.name == "Kolbasz")
        .unwrap()
        .id;
    acquire(&mut run, ordinary);
    let ordinary_anchor = anchor_handle(&run, ordinary);
    let before = run.physics.rapier().bodies[ordinary_anchor]
        .next_position()
        .rotation;
    let view_before = Rotation::from_array(run.area.player.view_pose(&run.physics).rotation);
    run.area.player.step(
        FrameInput {
            mouse: [0., 5.],
            ..Default::default()
        },
        &mut run.physics,
        FIXED_DT,
    );
    let view_after = Rotation::from_array(run.area.player.view_pose(&run.physics).rotation);
    step_input(&mut run, FrameInput::default(), ResolvedAim::default());
    near_rotation(
        run.physics.rapier().bodies[ordinary_anchor]
            .next_position()
            .rotation,
        view_after * view_before.inverse() * before,
    );
}

#[test]
#[ignore = "requires build 11140890 export"]
fn carry_release_shares_tuning_input() {
    let (reference, mut run) = source_scene();
    run.area.player.rebase([0., -50., 0.]);
    let item = reference
        .items
        .iter()
        .find(|i| i.name == "Kolbasz")
        .unwrap()
        .id;
    acquire(&mut run, item);
    let opening = reference.openings.iter().find(|o| o.can_tune).unwrap();
    let cap = opening.cap;
    let tune_mouse = [
        0.,
        if run.area.interaction.opening_valve(opening.id) < 0.5 {
            0.2
        } else {
            -0.2
        },
    ];
    let binding = reference
        .action_bindings
        .iter()
        .find(|b| b.use_owner == Some(cap))
        .unwrap();
    let collider = run
        .physics
        .rapier()
        .colliders
        .iter()
        .find(|(_, c)| (c.user_data >> 64) as u32 == binding.collider.0)
        .unwrap()
        .0;
    let aim = ResolvedAim {
        use_target: Some(AimTarget {
            item: cap,
            collider,
            distance: 1.,
            point: [0.; 3],
            slot: None,
        }),
        ..Default::default()
    };
    for frame in 0..12 {
        let out = step_input(
            &mut run,
            FrameInput {
                held: keys::USE | keys::GRAB,
                pressed: if frame == 0 { keys::GRAB } else { 0 },
                mouse: tune_mouse,
                ..Default::default()
            },
            aim,
        );
        assert!(matches!(out.action, Some(InteractionAction::Cap(_))));
        assert_eq!(out.look_mouse, [0.; 2]);
    }
    assert!(
        run.area.interaction.throw_charge() > reference.player.throw_minimum,
        "tuning must not consume the distinct F gesture"
    );
    while run.area.interaction.pop_sound().is_some() {}
    let valve_before = run.area.interaction.opening_valve(opening.id);
    let out = step_input(
        &mut run,
        FrameInput {
            held: keys::USE,
            released: keys::GRAB,
            mouse: tune_mouse,
            ..Default::default()
        },
        aim,
    );
    assert_eq!(out.action, Some(InteractionAction::Throw(item)));
    assert_eq!(run.area.interaction.held(), None);
    assert_ne!(run.area.interaction.opening_valve(opening.id), valve_before);
    assert!(
        matches!(run.area.interaction.pop_sound(), Some(my_summer_slop::interaction::InteractionSound::Cap { id, tune: Some(_), .. }) if id == opening.id)
    );
}

#[test]
#[ignore = "requires build 11140890 export"]
fn carry_release_preserves_held_switch() {
    use my_summer_slop::{interaction::InteractionSound, save::IdentityMap};
    let (reference, mut run) = source_scene();
    run.area.player.rebase([0., -50., 0.]);
    let item = reference
        .flashlights
        .instances
        .iter()
        .find(|f| f.startup_active)
        .unwrap()
        .item;
    if run.area.interaction.storage.stored(item) {
        let point = run.area.items.get(item).unwrap().pose.position;
        run.area
            .interaction
            .pickup_stored_item(
                item,
                point,
                &run.area.player,
                &mut run.area.items,
                &mut run.physics,
            )
            .unwrap();
        run.area
            .interaction
            .drop_held(&mut run.area.items, &mut run.physics);
    }
    let ids = IdentityMap::new(&reference).unwrap();
    for forced in [false, true] {
        acquire(&mut run, item);
        let control = run.area.interaction.held_light_control().unwrap();
        let source = ids
            .controllers
            .iter()
            .find(|(_, id)| **id == control)
            .unwrap()
            .0;
        let before = run
            .lighting
            .capture_save(&reference.lighting, &ids)
            .unwrap()
            .controllers
            .into_iter()
            .find(|c| &c.source == source)
            .unwrap()
            .on;
        if forced {
            let anchor = anchor_handle(&run, item);
            let target = run.physics.rapier().bodies[anchor]
                .next_position()
                .translation;
            let body = run.area.items.get(item).unwrap().body.unwrap();
            run.physics.edit_body(body, |body| {
                body.set_translation(
                    target + Vector::X * reference.player.carry_drop_distance,
                    true,
                )
            });
        } else {
            step_input(
                &mut run,
                FrameInput {
                    held: keys::GRAB,
                    pressed: keys::GRAB,
                    ..Default::default()
                },
                ResolvedAim::default(),
            );
        }
        let out = step_input(
            &mut run,
            FrameInput {
                pressed: keys::PRIMARY,
                released: if forced { 0 } else { keys::GRAB },
                ..Default::default()
            },
            ResolvedAim::default(),
        );
        assert_eq!(run.area.interaction.held(), None);
        assert_eq!(out.action, Some(InteractionAction::LightControl(control)));
        // Execute the external owner from GarageRuntime's action match.
        if let Some(InteractionAction::LightControl(control)) = out.action {
            run.lighting.use_control(&reference.lighting, control);
        }
        let after = run
            .lighting
            .capture_save(&reference.lighting, &ids)
            .unwrap()
            .controllers
            .into_iter()
            .find(|c| &c.source == source)
            .unwrap()
            .on;
        assert_ne!(before, after);
        assert!(
            matches!(run.area.interaction.pop_sound(), Some(InteractionSound::Action(InteractionAction::Drop(dropped))) if dropped == item)
        );
        assert!(run.area.interaction.pop_sound().is_none());
    }
}
