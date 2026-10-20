//! Source build 11140890 action order and input ownership.
use crate::{
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
fn source_reference() -> ReferenceManifest {
    ReferenceManifest::from_runtime_json(
        &std::fs::read("web/assets/reference/manifest.json").unwrap(),
    )
    .unwrap()
}
fn handling_frame(
    run: &mut GarageRuntime,
    reference: &ReferenceManifest,
    input: FrameInput,
    dt: f32,
) -> FrameInput {
    crate::world::stream::tests::prepare_ready(run, reference);
    run.sync_world_activity(reference).unwrap();
    run.started = true;
    assert!(run.world_supported());
    let view = run.area.player.view_pose(&run.physics);
    let frame = run.advance_inventory_frame(reference, input, dt, view);
    assert!(!frame.actions_blocked);
    frame.fixed_input
}
fn step_input(run: &mut GarageRuntime, input: FrameInput, aim: ResolvedAim) -> InteractionOutcome {
    let input = if (input.held | input.pressed | input.released) & keys::RECENTER != 0 {
        let reference = source_reference();
        run.aim = aim;
        handling_frame(run, &reference, input, FIXED_DT)
    } else {
        let input = run.area.interaction.filter_cancelled_input(input);
        let available = run.area.interaction.available_actions(
            &aim,
            &run.area.player,
            &run.area.items,
            &run.physics,
            run.survival.output(),
        );
        run.area.interaction.route_frame_input(input, &available)
    };
    step_fixed_input(run, input, aim)
}
fn step_fixed_input(
    run: &mut GarageRuntime,
    input: FrameInput,
    aim: ResolvedAim,
) -> InteractionOutcome {
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
    // Reset suppresses stale held input and release. A fresh pressed edge is separate.
    run.area.interaction.cancel_input();
    let held =
        keys::GRAB | keys::RECENTER | keys::TRANSLATE | keys::ROTATE | keys::USE | keys::PRIMARY;
    let outcome = step_input(
        &mut run,
        FrameInput {
            held,
            mouse: [1., 2.],
            wheel: 1.,
            ..Default::default()
        },
        both,
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
    let ids = crate::save::IdentityMap::new(&reference).unwrap();
    let before = run.area.interaction.capture_carry_save(&ids).unwrap();
    run.area.interaction.cancel_input();
    let outcome = step_input(
        &mut run,
        FrameInput {
            held,
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

fn place_for_carry(run: &mut GarageRuntime, item: crate::reference::ItemId) -> ResolvedAim {
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
#[track_caller]
fn acquire(run: &mut GarageRuntime, item: crate::reference::ItemId) {
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
    item: crate::reference::ItemId,
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
    use crate::interaction::items::ItemLocation;
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
    let mass = run.physics.authored_body_mass(body);
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
    use crate::{reference::ItemId, save::IdentityMap};
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
    let mass = run.physics.authored_body_mass(body);
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
        matches!(run.area.interaction.pop_sound(), Some(crate::interaction::InteractionSound::Cap { id, tune: Some(_), .. }) if id == opening.id)
    );
}

#[test]
#[ignore = "requires build 11140890 export"]
fn carry_release_preserves_held_switch() {
    use crate::{interaction::InteractionSound, save::IdentityMap};
    let (reference, mut run) = source_scene();
    handling_frame(&mut run, &reference, FrameInput::default(), FIXED_DT);
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
        let input = handling_frame(
            &mut run,
            &reference,
            FrameInput {
                pressed: keys::PRIMARY,
                released: if forced { 0 } else { keys::GRAB },
                ..Default::default()
            },
            FIXED_DT,
        );
        let out = step_fixed_input(&mut run, input, ResolvedAim::default());
        assert_eq!(out.action, Some(InteractionAction::Drop(item)));
        assert_eq!(run.area.interaction.held(), None);
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
            matches!(run.area.interaction.pop_sound(), Some(InteractionSound::Action(InteractionAction::LightControl(used))) if used == control)
        );
        assert!(run.area.interaction.pop_sound().is_none());
    }
}

#[test]
#[ignore = "requires build 11140890 export"]
fn carry_mass_save_and_assembly_lifecycle() {
    use crate::{
        interaction::items::ItemLocation,
        reference::ItemKind,
        save::{CameraSave, SourceHash, SourceStamp},
    };
    let (reference, mut run) = source_scene();
    let oil = reference
        .items
        .iter()
        .find(|i| i.name == "OilCan")
        .unwrap()
        .id;
    let oil_body = run.area.items.get(oil).unwrap().body.unwrap();
    let source_body = reference.items.iter().find(|i| i.id == oil).unwrap().body;
    let initial_mass = reference
        .bodies
        .iter()
        .find(|b| b.id == source_body)
        .unwrap()
        .mass_kg;
    let parents: Vec<_> = run.physics.rapier().bodies[oil_body].colliders().to_vec();
    let quality = run.area.conditions.item_quality(oil);
    run.physics
        .edit_body(oil_body, |body| body.set_gravity_scale(0.35, true));
    let mut authored_mass = initial_mass;
    for _ in 0..3 {
        // Acquisition commits pending Rapier mass before recording authored mass.
        let point = run.area.items.get(oil).unwrap().pose.position;
        run.area
            .interaction
            .pickup_item(
                oil,
                point,
                &run.area.player,
                &mut run.area.items,
                &mut run.physics,
            )
            .unwrap();
        assert_eq!(run.physics.rapier().bodies[oil_body].mass(), 1.0);
        assert_eq!(run.physics.rapier().bodies[oil_body].gravity_scale(), 0.0);
        near_mass(run.physics.authored_body_mass(oil_body), authored_mass);
        drain_tank(&mut run, oil, 0.2);
        authored_mass -= 0.2;
        sync_handling_mass(&mut run);
        assert_eq!(run.physics.rapier().bodies[oil_body].mass(), 1.0);
        near_mass(run.physics.authored_body_mass(oil_body), authored_mass);
        let fluid = run.area.items.get(oil).unwrap().fluid.clone();
        assert_eq!(
            run.area
                .interaction
                .drop_held(&mut run.area.items, &mut run.physics),
            Some(oil)
        );
        near_mass(run.physics.rapier().bodies[oil_body].mass(), authored_mass);
        assert_eq!(run.physics.rapier().bodies[oil_body].gravity_scale(), 0.35);
        assert_eq!(run.area.items.get(oil).unwrap().body, Some(oil_body));
        assert_eq!(run.area.items.get(oil).unwrap().fluid, fluid);
        assert_eq!(run.area.conditions.item_quality(oil), quality);
        assert_eq!(run.physics.rapier().bodies[oil_body].colliders(), parents);
        for &handle in &parents {
            assert_eq!(
                run.physics.rapier().colliders[handle].parent(),
                Some(oil_body)
            );
        }
    }
    // General part mass includes the current contents of retained tank descendants.
    let engine = reference.vehicles[0].engine.item;
    let sump = reference.vehicles[0].engine.oil_tank;
    run.area
        .interaction
        .parts
        .remove(engine, &mut run.area.items, &mut run.physics)
        .unwrap();
    run.area.interaction.parts.detach_mechanism(
        engine,
        &mut run.area.interaction.mechanisms,
        &mut run.physics,
    );
    sync_handling_mass(&mut run);
    let engine_body = run.area.items.get(engine).unwrap().body.unwrap();
    assert_eq!(run.area.items.get(sump).unwrap().body, Some(engine_body));
    let members: Vec<_> = run
        .area
        .items
        .iter()
        .filter(|i| i.body == Some(engine_body))
        .map(|i| (i.id, i.location, run.area.conditions.item_quality(i.id)))
        .collect();
    let engine_colliders = run.physics.rapier().bodies[engine_body]
        .colliders()
        .to_vec();
    for _ in 0..3 {
        let mass = run.physics.authored_body_mass(engine_body);
        let point = run.area.items.get(engine).unwrap().pose.position;
        run.area
            .interaction
            .pickup_item(
                engine,
                point,
                &run.area.player,
                &mut run.area.items,
                &mut run.physics,
            )
            .unwrap();
        drain_tank(&mut run, sump, 0.1);
        sync_handling_mass(&mut run);
        assert_eq!(run.physics.rapier().bodies[engine_body].mass(), 1.0);
        near_mass(run.physics.authored_body_mass(engine_body), mass - 0.1);
        let fluids: Vec<_> = members
            .iter()
            .map(|(id, _, _)| run.area.items.get(*id).unwrap().fluid.clone())
            .collect();
        run.area
            .interaction
            .drop_held(&mut run.area.items, &mut run.physics);
        near_mass(run.physics.rapier().bodies[engine_body].mass(), mass - 0.1);
        assert_eq!(
            run.physics.rapier().bodies[engine_body].gravity_scale(),
            1.0
        );
        for ((id, location, quality), fluid) in members.iter().zip(fluids) {
            let state = run.area.items.get(*id).unwrap();
            assert_eq!(state.body, Some(engine_body));
            assert_eq!(state.location, *location);
            assert_eq!(state.fluid, fluid);
            assert_eq!(run.area.conditions.item_quality(*id), *quality);
        }
        assert_eq!(
            run.physics.rapier().bodies[engine_body].colliders(),
            engine_colliders
        );
        for &handle in &engine_colliders {
            assert_eq!(
                run.physics.rapier().colliders[handle].parent(),
                Some(engine_body)
            );
        }
    }
    // A held loose wheel retains its override when a mounted cover changes its mass.
    let slot = &reference.wheel_slots[0];
    let rim = slot.rim;
    let tire = slot.tire;
    let cover_slot = reference
        .wheel_parts
        .iter()
        .find(|p| p.item == rim)
        .unwrap()
        .cover_slot
        .unwrap();
    let cover = run.area.interaction.parts.slot_item(cover_slot).unwrap();
    run.area
        .interaction
        .parts
        .remove(cover, &mut run.area.items, &mut run.physics)
        .unwrap();
    run.wheels
        .remove(slot.id, &mut run.area.items, &mut run.physics)
        .unwrap();
    run.area
        .interaction
        .parts
        .install(cover, cover_slot, &mut run.area.items, &mut run.physics)
        .unwrap();
    sync_handling_mass(&mut run);
    let rim_body = run.area.items.get(rim).unwrap().body.unwrap();
    let cover_mass = run
        .physics
        .authored_body_mass(run.area.interaction.parts.loose_body(cover).unwrap());
    let wheel_colliders = run.physics.rapier().bodies[rim_body].colliders().to_vec();
    let qualities = [rim, tire, cover].map(|id| run.area.conditions.item_quality(id));
    assert!(matches!(
        reference.items.iter().find(|i| i.id == rim).unwrap().kind,
        ItemKind::Rim
    ));
    for _ in 0..3 {
        let mass = run.physics.authored_body_mass(rim_body);
        let point = run.area.items.get(rim).unwrap().pose.position;
        run.area
            .interaction
            .pickup_item(
                rim,
                point,
                &run.area.player,
                &mut run.area.items,
                &mut run.physics,
            )
            .unwrap();
        run.area
            .interaction
            .parts
            .remove(cover, &mut run.area.items, &mut run.physics)
            .unwrap();
        sync_handling_mass(&mut run);
        assert_eq!(run.physics.rapier().bodies[rim_body].mass(), 1.0);
        near_mass(run.physics.authored_body_mass(rim_body), mass - cover_mass);
        assert_eq!(run.area.items.get(tire).unwrap().body, Some(rim_body));
        assert_eq!(
            run.area.items.get(tire).unwrap().location,
            ItemLocation::Attached(rim)
        );
        run.area
            .interaction
            .drop_held(&mut run.area.items, &mut run.physics);
        near_mass(
            run.physics.rapier().bodies[rim_body].mass(),
            mass - cover_mass,
        );
        assert_eq!(run.physics.rapier().bodies[rim_body].gravity_scale(), 1.0);
        run.area
            .interaction
            .parts
            .install(cover, cover_slot, &mut run.area.items, &mut run.physics)
            .unwrap();
        sync_handling_mass(&mut run);
        near_mass(run.physics.authored_body_mass(rim_body), mass);
        assert_eq!(run.area.items.get(rim).unwrap().body, Some(rim_body));
        assert_eq!(run.area.items.get(cover).unwrap().body, Some(rim_body));
        assert_eq!(
            run.area.interaction.parts.slot_item(cover_slot),
            Some(cover)
        );
        assert_eq!(
            [rim, tire, cover].map(|id| run.area.conditions.item_quality(id)),
            qualities
        );
        assert_eq!(
            run.physics.rapier().bodies[rim_body].colliders().len(),
            wheel_colliders.len()
        );
        for &handle in &wheel_colliders {
            assert_eq!(
                run.physics.rapier().colliders[handle].parent(),
                Some(rim_body)
            );
        }
    }
    // Save current contents and a root-relative held pose, while a throw is armed.
    run.physics
        .edit_body(oil_body, |body| body.set_gravity_scale(1.0, true));
    run.area.player.rebase([0., -50., 0.]);
    acquire(&mut run, oil);
    run.area.player.follow_push_grip(15.);
    for _ in 0..8 {
        step_input(
            &mut run,
            FrameInput {
                held: keys::ROTATE,
                mouse: [1., 0.5],
                ..Default::default()
            },
            ResolvedAim::default(),
        );
        run.physics.step_physics();
    }
    for _ in 0..12 {
        step_input(&mut run, FrameInput::default(), ResolvedAim::default());
        run.physics.step_physics();
    }
    step_input(
        &mut run,
        FrameInput {
            pressed: keys::GRAB,
            held: keys::GRAB,
            ..Default::default()
        },
        ResolvedAim::default(),
    );
    assert!(run.area.interaction.throw_active());
    let digest = std::process::Command::new("sha256sum")
        .arg("web/assets/reference/manifest.json")
        .output()
        .unwrap();
    assert!(digest.status.success());
    let stamp = SourceStamp {
        app_id: reference.app_id,
        build_id: reference.build_id,
        manifest_sha256: String::from_utf8(digest.stdout)
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
            .into(),
        source_hashes: reference
            .source_hashes
            .iter()
            .map(|h| SourceHash {
                file: h.file.clone(),
                sha256: h.sha256.clone(),
            })
            .collect(),
    };
    let saved = run
        .capture_save(
            &reference,
            &stamp,
            CameraSave::FirstPerson,
            "2026-10-20T12:00:00Z",
        )
        .unwrap();
    let mut restored = GarageRuntime::from_save(&reference, &stamp, &saved).unwrap();
    let restored_body = restored.area.items.get(oil).unwrap().body.unwrap();
    assert_eq!(restored.area.interaction.held(), Some(oil));
    assert_eq!(restored.physics.rapier().bodies[restored_body].mass(), 1.0);
    assert_eq!(
        restored.physics.rapier().bodies[restored_body].gravity_scale(),
        0.0
    );
    near_mass(
        restored.physics.authored_body_mass(restored_body),
        authored_mass,
    );
    assert_eq!(
        restored.area.items.get(oil).unwrap().fluid,
        run.area.items.get(oil).unwrap().fluid
    );
    assert!(!restored.area.interaction.throw_active());
    assert_eq!(restored.area.interaction.throw_charge(), 0.0);
    assert_eq!(
        restored
            .physics
            .rapier()
            .impulse_joints
            .iter()
            .filter(|(_, j)| j.body2() == restored_body)
            .count(),
        1
    );
    let ids = crate::save::IdentityMap::new(&reference).unwrap();
    assert_eq!(
        saved.player.carry,
        restored.area.interaction.capture_carry_save(&ids).unwrap()
    );
    let oil_instance = &saved.player.carry.as_ref().unwrap().item;
    let before = &saved
        .items
        .iter()
        .find(|i| &i.instance == oil_instance)
        .unwrap()
        .pose;
    let after = *restored.physics.rapier().bodies[restored_body].position()
        * crate::vehicle::to_rapier(restored.area.items.get(oil).unwrap().local_pose);
    let absolute = restored
        .stream
        .origin
        .absolute(after.translation.to_array());
    for axis in 0..3 {
        assert!((before.position[axis] - absolute[axis]).abs() < 0.0001);
    }
    near_rotation(Rotation::from_array(before.rotation), after.rotation);
    // A later publication updates authored mass without removing the carried override.
    restored
        .physics
        .set_body_mass(restored_body, authored_mass + 0.4);
    assert_eq!(restored.physics.rapier().bodies[restored_body].mass(), 1.0);
    near_mass(
        restored.physics.authored_body_mass(restored_body),
        authored_mass + 0.4,
    );
    let out = step_input(
        &mut restored,
        FrameInput {
            released: keys::GRAB,
            ..Default::default()
        },
        ResolvedAim::default(),
    );
    assert_eq!(out.action, None);
    assert_eq!(restored.area.interaction.held(), Some(oil));
    restored
        .area
        .interaction
        .drop_held(&mut restored.area.items, &mut restored.physics);
    near_mass(
        restored.physics.rapier().bodies[restored_body].mass(),
        authored_mass + 0.4,
    );
    assert_eq!(
        restored.physics.rapier().bodies[restored_body].gravity_scale(),
        1.0
    );
    assert!(
        restored
            .physics
            .rapier()
            .impulse_joints
            .iter()
            .all(|(_, j)| j.body2() != restored_body)
    );
    assert_eq!(
        restored.physics.rapier().bodies[restored_body]
            .colliders()
            .len(),
        parents.len()
    );
    for &handle in restored.physics.rapier().bodies[restored_body].colliders() {
        assert_eq!(
            restored.physics.rapier().colliders[handle].parent(),
            Some(restored_body)
        );
    }
}

fn drain_tank(run: &mut GarageRuntime, item: crate::reference::ItemId, amount: f32) {
    let fluid = run
        .area
        .items
        .get_mut(item)
        .unwrap()
        .fluid
        .as_mut()
        .unwrap();
    let index = fluid
        .entries()
        .find(|entry| entry.amount > amount)
        .unwrap()
        .fluid_type as usize;
    fluid.change_one(-amount, index);
}
fn near_mass(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.0001,
        "mass {actual} != {expected}"
    );
}
fn sync_handling_mass(run: &mut GarageRuntime) {
    run.area.sync_portable_mass(&mut run.physics);
    run.area
        .interaction
        .sync_parts(&mut run.area.items, &mut run.physics)
        .unwrap();
    run.wheels
        .sync(&mut run.area.items, &mut run.physics)
        .unwrap();
}
#[test]
#[ignore = "requires build 11140890 export"]
fn carry_body_follows_solver_pose_and_collision() {
    let (reference, mut run) = source_scene();
    let oil = reference
        .items
        .iter()
        .find(|i| i.name == "OilCan")
        .unwrap()
        .id;
    run.area.player.rebase([0., -50., 0.]);
    acquire(&mut run, oil);
    let body = run.area.items.get(oil).unwrap().body.unwrap();
    assert_eq!(run.physics.rapier().bodies[body].mass(), 1.0);
    let initial = *run.physics.rapier().bodies[body].position();
    run.area.player.rebase([0.2, 0., 0.]);
    for _ in 0..20 {
        step_input(&mut run, FrameInput::default(), ResolvedAim::default());
        run.physics.step_physics();
    }
    let moved = *run.physics.rapier().bodies[body].position();
    assert!(
        initial.translation.x - moved.translation.x > 0.1,
        "actual held body follows translation: {initial:?} -> {moved:?}"
    );
    for _ in 0..8 {
        step_input(
            &mut run,
            FrameInput {
                held: keys::ROTATE,
                mouse: [2., 1.],
                ..Default::default()
            },
            ResolvedAim::default(),
        );
        run.physics.step_physics();
    }
    for _ in 0..12 {
        step_input(&mut run, FrameInput::default(), ResolvedAim::default());
        run.physics.step_physics();
    }
    let rotated = *run.physics.rapier().bodies[body].position();
    assert!(
        rotated.rotation.dot(moved.rotation).abs() < 0.999,
        "actual held body rotates: {moved:?} -> {rotated:?}; mass properties {:?}",
        run.physics.rapier().bodies[body].mass_properties()
    );
    let colliders: Vec<_> = run.physics.rapier().bodies[body]
        .colliders()
        .iter()
        .copied()
        .filter(|h| !run.physics.rapier().colliders[*h].is_sensor())
        .collect();
    assert!(!colliders.is_empty());
    let edge = colliders
        .iter()
        .map(|h| run.physics.rapier().colliders[*h].compute_aabb().maxs.x)
        .fold(f32::NEG_INFINITY, f32::max);
    let wall_x = edge + 0.25;
    let (_, wall) = run.physics.insert(
        rapier3d::prelude::RigidBodyBuilder::fixed().translation(Vector::new(
            wall_x,
            rotated.translation.y,
            rotated.translation.z,
        )),
        rapier3d::prelude::ColliderBuilder::cuboid(0.1, 2., 2.),
    );
    run.physics.refresh_queries();
    run.area.player.rebase([-1., 0., 0.]);
    for _ in 0..50 {
        step_input(&mut run, FrameInput::default(), ResolvedAim::default());
        run.physics.step_physics();
    }
    let blocked = run.physics.rapier().bodies[body].translation();
    assert!(
        blocked.x > rotated.translation.x + 0.05,
        "actual body reaches the wall"
    );
    assert!(
        blocked.x < rotated.translation.x + 0.3,
        "actual body cannot pass the wall"
    );
    let wall = &run.physics.rapier().colliders[wall];
    for handle in colliders {
        let collider = &run.physics.rapier().colliders[handle];
        assert_eq!(collider.parent(), Some(body));
        assert!(collider.is_enabled());
        assert_eq!(collider.density(), 0.0);
        assert!(
            !rapier3d::parry::query::intersection_test(
                collider.position(),
                collider.shape(),
                wall.position(),
                wall.shape()
            )
            .unwrap()
            .intersecting
        );
    }
    assert!(run.physics.rapier().bodies[body].is_ccd_enabled());
}
#[test]
#[ignore = "requires build 11140890 export"]
fn tab_recenter_has_one_owner() {
    use crate::{interaction::items::ItemLocation, reference::ItemId, save::IdentityMap};

    fn frame(
        reference: &ReferenceManifest,
        run: &mut GarageRuntime,
        held: u32,
        pressed: u32,
        released: u32,
        dt: f32,
    ) -> InteractionOutcome {
        let input = handling_frame(
            run,
            reference,
            FrameInput {
                held,
                pressed,
                released,
                mouse: [0.2, 0.1],
                ..Default::default()
            },
            dt,
        );
        let out = run.area.interaction.step(
            input,
            ResolvedAim::default(),
            &mut run.area.player,
            &mut run.area.items,
            &mut run.physics,
            dt,
            run.survival.output(),
            1.,
        );
        run.area.sync_inventory();
        out
    }
    fn offset(run: &GarageRuntime, ids: &IdentityMap) -> [f32; 3] {
        run.area
            .interaction
            .capture_carry_save(ids)
            .unwrap()
            .unwrap()
            .offset
    }
    fn pickup(run: &mut GarageRuntime, id: ItemId) {
        if run.area.interaction.storage.stored(id) {
            let point = run.area.items.get(id).unwrap().pose.position;
            run.area
                .interaction
                .pickup_stored_item(
                    id,
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
        place_for_carry(run, id);
        let point = run.area.items.get(id).unwrap().pose.position;
        run.area
            .interaction
            .pickup_item(
                id,
                point,
                &run.area.player,
                &mut run.area.items,
                &mut run.physics,
            )
            .unwrap();
        let item = run.area.items.get(id).unwrap();
        assert!(run.physics.rapier().bodies[item.body.unwrap()].is_enabled());
        let rule = run.area.interaction.pickup_rule(id).unwrap();
        assert!(rule.can_hold_right_hand && rule.can_inventory);
        run.area.sync_inventory();
    }
    fn equip(reference: &ReferenceManifest, run: &mut GarageRuntime, id: ItemId) {
        handling_frame(run, reference, FrameInput::default(), FIXED_DT);
        pickup(run, id);
        frame(reference, run, keys::RECENTER, keys::RECENTER, 0, 0.125);
        frame(reference, run, 0, 0, keys::RECENTER, 0.125);
        assert_eq!(run.area.interaction.equipment.item(), Some(id));
        assert_eq!(run.area.interaction.held(), None);
    }

    let (reference, mut run) = source_scene();
    handling_frame(&mut run, &reference, FrameInput::default(), FIXED_DT);
    assert_eq!(reference.inventory.equip_hold_seconds, 0.25);
    let ids = IdentityMap::new(&reference).unwrap();
    let lamp = ItemId(126);
    let other = ItemId(132);
    assert!(
        reference
            .flashlights
            .instances
            .iter()
            .any(|f| f.item == lamp && f.startup_active)
    );
    let spray = reference.items.iter().find(|i| i.id == other).unwrap();
    assert_eq!(spray.source.file, "level1");
    assert_eq!(spray.source.path_id, 30986);
    let oil = ItemId(120);
    assert_eq!(
        run.area
            .interaction
            .pickup_rule(lamp)
            .unwrap()
            .recenter_distance,
        -0.5
    );
    assert_eq!(
        run.area
            .interaction
            .pickup_rule(oil)
            .unwrap()
            .recenter_distance,
        0.
    );
    pickup(&mut run, lamp);
    let body = run.area.items.get(lamp).unwrap().body.unwrap();
    let authored = run.physics.authored_body_mass(body);
    let slots_before = run.area.inventory.slots;
    let before = offset(&run, &ids);
    frame(
        &reference,
        &mut run,
        keys::RECENTER,
        keys::RECENTER,
        0,
        0.125,
    );
    assert_eq!(
        offset(&run, &ids),
        before,
        "short Tab must not recenter on press"
    );
    frame(&reference, &mut run, 0, 0, keys::RECENTER, 0.125);
    assert_eq!(run.area.interaction.equipment.item(), Some(lamp));
    assert_eq!(run.area.interaction.held(), None);
    assert_eq!(
        run.area.items.get(lamp).unwrap().location,
        ItemLocation::Equipped
    );
    assert_eq!(run.area.inventory.slots, slots_before);
    near_mass(run.physics.rapier().bodies[body].mass(), authored);
    assert_eq!(run.physics.rapier().bodies[body].gravity_scale(), 1.);
    assert!(run.physics.rapier().bodies[body].is_kinematic());
    assert!(
        run.physics
            .rapier()
            .impulse_joints
            .iter()
            .all(|(_, j)| j.body2() != body)
    );
    // Tab-down releases equipment, but its release cannot equip it again.
    let outcome = frame(
        &reference,
        &mut run,
        keys::RECENTER | keys::ROTATE,
        keys::RECENTER,
        0,
        0.125,
    );
    assert_eq!(
        outcome.look_mouse, [0.; 2],
        "resolve input after hand transfer"
    );
    assert_eq!(run.area.interaction.equipment.item(), None);
    assert_eq!(run.area.interaction.held(), Some(lamp));
    near_mass(run.physics.rapier().bodies[body].mass(), 1.);
    assert_eq!(run.physics.rapier().bodies[body].gravity_scale(), 0.);
    frame(&reference, &mut run, 0, 0, keys::RECENTER, 0.125);
    assert_eq!(run.area.interaction.held(), Some(lamp));
    assert_eq!(run.area.inventory.slots, slots_before);

    // Both comparisons are strict at the source threshold.
    let before = offset(&run, &ids);
    frame(
        &reference,
        &mut run,
        keys::RECENTER,
        keys::RECENTER,
        0,
        0.125,
    );
    frame(&reference, &mut run, keys::RECENTER, 0, 0, 0.25);
    assert_eq!(offset(&run, &ids), before);
    frame(&reference, &mut run, keys::RECENTER, 0, 0, 0.001);
    let centered = (Vector::from_array(reference.player.carry_anchor_offset) * -0.5).to_array();
    assert_eq!(offset(&run, &ids), centered);
    frame(&reference, &mut run, 0, 0, keys::RECENTER, 0.01);
    assert_eq!(run.area.interaction.held(), Some(lamp));
    frame(
        &reference,
        &mut run,
        keys::RECENTER,
        keys::RECENTER,
        0,
        0.125,
    );
    frame(&reference, &mut run, 0, 0, keys::RECENTER, 0.25);
    assert_eq!(run.area.interaction.equipment.item(), None);

    // Reset discards the timer and blocks held input until release.
    step_input(
        &mut run,
        FrameInput {
            wheel: 1.,
            ..Default::default()
        },
        ResolvedAim::default(),
    );
    let cancelled_offset = offset(&run, &ids);
    frame(
        &reference,
        &mut run,
        keys::RECENTER,
        keys::RECENTER,
        0,
        0.125,
    );
    run.area.interaction.cancel_input();
    frame(&reference, &mut run, keys::RECENTER, 0, 0, 0.5);
    frame(&reference, &mut run, 0, 0, keys::RECENTER, 0.01);
    assert_eq!(run.area.interaction.held(), Some(lamp));
    assert_eq!(run.area.interaction.equipment.item(), None);
    assert_eq!(offset(&run, &ids), cancelled_offset);
    // A new carried identity cannot finish the previous gesture.
    frame(
        &reference,
        &mut run,
        keys::RECENTER,
        keys::RECENTER,
        0,
        0.125,
    );
    run.area
        .interaction
        .drop_held(&mut run.area.items, &mut run.physics);
    run.area.player.rebase([0., -50., 0.]);
    place_for_carry(&mut run, oil);
    let point = run.area.items.get(oil).unwrap().pose.position;
    run.area
        .interaction
        .pickup_item(
            oil,
            point,
            &run.area.player,
            &mut run.area.items,
            &mut run.physics,
        )
        .unwrap();
    let before = offset(&run, &ids);
    frame(&reference, &mut run, keys::RECENTER, 0, 0, 0.5);
    assert_eq!(
        offset(&run, &ids),
        before,
        "item change suppresses stale recenter"
    );
    frame(&reference, &mut run, 0, 0, keys::RECENTER, 0.01);
    assert_eq!(run.area.interaction.equipment.item(), None);
    frame(
        &reference,
        &mut run,
        keys::RECENTER,
        keys::RECENTER,
        0,
        0.125,
    );
    assert_eq!(offset(&run, &ids), [0.; 3], "OilCan recenters immediately");
    let anchor = anchor_handle(&run, oil);
    let saved = run
        .area
        .interaction
        .capture_carry_save(&ids)
        .unwrap()
        .unwrap();
    near_rotation(
        run.physics.rapier().bodies[anchor].next_position().rotation,
        Rotation::from_array(run.area.player.view_pose(&run.physics).rotation)
            * Rotation::from_array(saved.local_grip.rotation).inverse(),
    );
    frame(&reference, &mut run, 0, 0, keys::RECENTER, 0.125);
    assert_eq!(run.area.interaction.held(), Some(oil));
    frame(
        &reference,
        &mut run,
        keys::RECENTER,
        keys::RECENTER,
        0,
        0.125,
    );
    run.area
        .interaction
        .drop_held(&mut run.area.items, &mut run.physics);
    pickup(&mut run, lamp);
    frame(&reference, &mut run, 0, 0, keys::RECENTER, 0.125);
    assert_eq!(
        run.area.interaction.equipment.item(),
        None,
        "item change suppresses delayed equip"
    );
    assert_eq!(run.area.interaction.held(), Some(lamp));
    run.area
        .interaction
        .drop_held(&mut run.area.items, &mut run.physics);
    near_mass(run.physics.rapier().bodies[body].mass(), authored);
    assert_eq!(run.physics.rapier().bodies[body].gravity_scale(), 1.);

    let (_, mut run) = source_scene();
    equip(&reference, &mut run, other);
    pickup(&mut run, lamp);
    let slots_before = run.area.inventory.slots;
    let before = offset(&run, &ids);
    frame(
        &reference,
        &mut run,
        keys::RECENTER,
        keys::RECENTER,
        0,
        0.125,
    );
    frame(&reference, &mut run, 0, 0, keys::RECENTER, 0.125);
    assert_eq!(offset(&run, &ids), before);
    assert_eq!(run.area.interaction.equipment.item(), Some(other));
    assert_eq!(run.area.interaction.held(), Some(lamp));
    assert_eq!(run.area.inventory.slots, slots_before);
    frame(
        &reference,
        &mut run,
        keys::RECENTER,
        keys::RECENTER,
        0,
        0.125,
    );
    frame(&reference, &mut run, keys::RECENTER, 0, 0, 0.251);
    assert_eq!(offset(&run, &ids), centered);
    frame(&reference, &mut run, 0, 0, keys::RECENTER, 0.01);
    assert_eq!(run.area.interaction.equipment.item(), Some(other));
    assert_eq!(run.area.interaction.held(), Some(lamp));
    assert_eq!(run.area.inventory.slots, slots_before);

    let (_, mut run) = source_scene();
    equip(&reference, &mut run, lamp);
    let slots_before = run.area.inventory.slots;
    let seat = &reference.seats[0];
    assert!(run.area.player.enter(
        seat.id,
        VehiclePose {
            position: [0., 10., 0.],
            rotation: [0., 0., 0., 1.],
        },
        &mut run.physics
    ));
    frame(
        &reference,
        &mut run,
        keys::RECENTER,
        keys::RECENTER,
        0,
        0.125,
    );
    frame(&reference, &mut run, 0, 0, keys::RECENTER, 0.125);
    assert_eq!(run.area.interaction.equipment.item(), Some(lamp));
    assert_eq!(run.area.interaction.held(), None);
    run.area.sync_equipment(&reference, &mut run.physics);
    // Held MMB transfers equipment even while seated and claims look input.
    let out = frame(&reference, &mut run, keys::ROTATE, 0, 0, 0.125);
    assert_eq!(out.look_mouse, [0.; 2]);
    assert_eq!(run.area.interaction.equipment.item(), None);
    assert_eq!(run.area.interaction.held(), Some(lamp));
    assert_eq!(run.area.inventory.slots, slots_before);
    assert_eq!(
        run.physics
            .rapier()
            .impulse_joints
            .iter()
            .filter(|(_, j)| j.body2() == run.area.items.get(lamp).unwrap().body.unwrap())
            .count(),
        1
    );
    frame(&reference, &mut run, 0, 0, keys::RECENTER, 0.125);
    assert_eq!(run.area.interaction.equipment.item(), None);
}

#[test]
#[ignore = "requires build 11140890 export"]
fn push_force_grip_and_cancellation() {
    use crate::reference::{ColliderId, ItemId};
    let (reference, mut run) = source_scene();
    let rule = reference
        .pushables
        .iter()
        .find(|r| r.owner == ItemId(27))
        .unwrap();
    assert_eq!(rule.body.0, 2);
    assert_eq!(rule.modifier, 1.);
    assert!(rule.only_projected);
    assert!(!rule.both);
    assert!(!rule.push_even_if_same_parent);
    assert_eq!(rule.grip.as_ref().unwrap().colliders, [ColliderId(398)]);
    assert_eq!(reference.player.push_force, 500.);
    assert_eq!(reference.player.airborne_push_force, 2500.);
    let binding = reference
        .action_bindings
        .iter()
        .find(|b| b.push_owner == Some(rule.owner) && b.collider == ColliderId(303))
        .unwrap();
    assert_eq!(binding.use_owner, None);
    assert_eq!(binding.pickup_owner, None);
    let collider = run
        .physics
        .rapier()
        .colliders
        .iter()
        .find(|(_, c)| (c.user_data >> 64) as u32 == binding.collider.0)
        .unwrap()
        .0;
    let body = run.physics.rapier().colliders[collider].parent().unwrap();
    assert_eq!(body, run.physics.chassis_handle(0));
    let aim = ray_aim(&run, collider, |a| {
        a.push_target.is_some_and(|t| t.item == rule.owner)
    });
    step_input(
        &mut run,
        FrameInput {
            held: keys::GRAB,
            pressed: keys::GRAB,
            ..Default::default()
        },
        ResolvedAim::default(),
    );
    let out = step_input(
        &mut run,
        FrameInput {
            held: keys::GRAB,
            ..Default::default()
        },
        aim,
    );
    assert_eq!(
        out.action,
        Some(InteractionAction::Push(rule.owner)),
        "held F starts ordinary push after aim changes"
    );
    // Authored force is mass-scaled. Horizontal projection keeps its source magnitude.
    run.area.player.rebase([0., -50., 0.]);
    run.area.player.step(
        FrameInput {
            mouse: [0., 30.],
            ..Default::default()
        },
        &mut run.physics,
        FIXED_DT,
    );
    assert!(!run.area.player.grounded());
    run.physics.recompute_body_mass(body);
    let mass = run.physics.rapier().bodies[body].mass();
    assert!(mass > 100.);
    let ids = crate::save::IdentityMap::new(&reference).unwrap();
    let recoil = |r: &GarageRuntime| {
        Vector::from_array(
            r.area
                .player
                .capture_save(&ids, &r.stream.origin, &r.physics, None)
                .unwrap()
                .recoil_velocity,
        )
    };
    let forward =
        Rotation::from_array(run.area.player.view_pose(&run.physics).rotation) * -Vector::Z;
    let projected = Vector::new(forward.x, 0., forward.z);
    let impulse =
        projected * mass * 500. * FIXED_DT * rule.modifier * reference.player.source_force_interval;
    let mut current_aim = aim;
    for side in [-1., 1.] {
        let com = run.physics.rapier().bodies[body].center_of_mass();
        let point = com + Vector::X * side;
        current_aim.push_target.as_mut().unwrap().point = point.to_array();
        run.physics.edit_body(body, |b| {
            b.set_linvel(Vector::ZERO, true);
            b.set_angvel(Vector::ZERO, true);
        });
        let before = recoil(&run);
        let out = step_input(
            &mut run,
            FrameInput {
                held: keys::GRAB,
                ..Default::default()
            },
            current_aim,
        );
        assert_eq!(out.action, Some(InteractionAction::Push(rule.owner)));
        let pushed = &run.physics.rapier().bodies[body];
        near_vector(pushed.linvel(), impulse / mass);
        let torque = (point - com).cross(impulse);
        assert!(
            pushed.angvel().dot(torque) > 0.,
            "current hit point must produce its torque"
        );
        near_vector(
            recoil(&run) - before,
            -forward * 2500. * reference.player.source_force_interval / reference.player.mass_kg,
        );
    }
    let before = run.physics.rapier().bodies[body].linvel();
    step_input(
        &mut run,
        FrameInput {
            released: keys::GRAB,
            ..Default::default()
        },
        current_aim,
    );
    near_vector(run.physics.rapier().bodies[body].linvel(), before);
    let baseline_joints = run.physics.rapier().impulse_joints.len();
    assert_push_released(&run, baseline_joints);

    // The actual source ray bindings reserve use and pickup before push.
    let use_binding = reference
        .action_bindings
        .iter()
        .find(|b| {
            b.push_owner == Some(rule.owner) && b.use_owner.is_some() && b.pickup_owner.is_some()
        })
        .unwrap();
    let handle = run
        .physics
        .rapier()
        .colliders
        .iter()
        .find(|(_, c)| (c.user_data >> 64) as u32 == use_binding.collider.0)
        .unwrap()
        .0;
    let reserved = ray_aim(&run, handle, |a| {
        a.use_target.is_some() || a.grab_target.is_some()
    });
    assert!(
        reserved.push_target.is_none(),
        "use or pickup reservation blocks push"
    );
    let before = run.physics.rapier().bodies[body].linvel();
    let out = step_input(
        &mut run,
        FrameInput {
            held: keys::GRAB,
            ..Default::default()
        },
        reserved,
    );
    assert!(!matches!(out.action, Some(InteractionAction::Push(_))));
    near_vector(run.physics.rapier().bodies[body].linvel(), before);

    // A supported grip requires F-down. Held F over a new grip is insufficient.
    push_support(&reference, &mut run, false);
    let ordinary_aim = source_push_aim(&reference, &run, false);
    let before_recoil = recoil(&run);
    let out = step_input(
        &mut run,
        FrameInput {
            held: keys::GRAB,
            ..Default::default()
        },
        ordinary_aim,
    );
    assert_eq!(out.action, Some(InteractionAction::Push(rule.owner)));
    near_vector(recoil(&run), before_recoil);
    let grip_aim = source_push_aim(&reference, &run, true);
    let available = run.area.interaction.available_actions(
        &grip_aim,
        &run.area.player,
        &run.area.items,
        &run.physics,
        run.survival.output(),
    );
    assert_ne!(available.actions.0 & ActionMask::GRIP, 0);
    step_input(
        &mut run,
        FrameInput {
            held: keys::GRAB,
            ..Default::default()
        },
        grip_aim,
    );
    assert_push_released(&run, baseline_joints);
    for _ in 0..3 {
        let out = step_input(
            &mut run,
            FrameInput {
                held: keys::GRAB,
                pressed: keys::GRAB,
                ..Default::default()
            },
            grip_aim,
        );
        assert_eq!(out.action, Some(InteractionAction::Push(rule.owner)));
        assert!(run.area.interaction.push_gripping());
        assert_eq!(
            run.physics.rapier().impulse_joints.len(),
            baseline_joints + 1
        );
        // Looking away and releasing F retain the grip.
        step_input(
            &mut run,
            FrameInput {
                released: keys::GRAB,
                ..Default::default()
            },
            ResolvedAim::default(),
        );
        assert!(run.area.interaction.push_gripping());
        for (movement, steer, sign) in [
            (keys::FORWARD | keys::LEFT, 1., 1.),
            (keys::BACK | keys::RIGHT, -1., -1.),
            (
                keys::FORWARD | keys::BACK | keys::LEFT | keys::RIGHT,
                0.,
                0.,
            ),
        ] {
            run.physics
                .edit_body(body, |b| b.set_linvel(Vector::ZERO, true));
            step_input(
                &mut run,
                FrameInput {
                    held: movement,
                    ..Default::default()
                },
                ResolvedAim::default(),
            );
            assert_eq!(run.area.interaction.push_steering(), Some((body, steer)));
            let car = run.physics.rapier().bodies[body].position();
            let direction = Rotation::from_rotation_y(steer * 40_f32.to_radians())
                * (car.rotation * -Vector::Z)
                * sign;
            let up = car.rotation * Vector::Y;
            let expected = (direction - up * direction.dot(up)).normalize_or_zero()
                * 10.
                * 300.
                * reference.player.source_force_interval
                / mass;
            near_vector(run.physics.rapier().bodies[body].linvel(), expected);
            assert_eq!(
                run.physics.rapier().impulse_joints.len(),
                baseline_joints + 1
            );
        }
        step_input(
            &mut run,
            FrameInput {
                held: keys::GRAB,
                pressed: keys::GRAB,
                ..Default::default()
            },
            ResolvedAim::default(),
        );
        assert_push_released(&run, baseline_joints);
        step_input(
            &mut run,
            FrameInput {
                held: keys::GRAB,
                ..Default::default()
            },
            grip_aim,
        );
        assert_push_released(&run, baseline_joints);
        step_input(&mut run, FrameInput::default(), ResolvedAim::default());
    }
    step_input(
        &mut run,
        FrameInput {
            held: keys::GRAB,
            pressed: keys::GRAB,
            ..Default::default()
        },
        grip_aim,
    );
    run.physics.edit_body(run.area.player.body_handle(), |b| {
        b.set_translation(b.translation() + Vector::X * 10., true)
    });
    step_input(
        &mut run,
        FrameInput {
            held: keys::FORWARD,
            ..Default::default()
        },
        ResolvedAim::default(),
    );
    assert_push_released(&run, baseline_joints);
    run.area.player.rebase([10., 0., 0.]);
    run.physics.edit_body(run.area.player.body_handle(), |b| {
        let mut pose = *b.position();
        pose.translation -= Vector::X * 10.;
        b.set_position(pose, true);
        b.set_next_kinematic_position(pose);
    });
    step_input(
        &mut run,
        FrameInput {
            held: keys::GRAB,
            pressed: keys::GRAB,
            ..Default::default()
        },
        grip_aim,
    );
    assert!(run.area.interaction.push_gripping());
    run.clear_input();
    assert!(!run.area.interaction.push_gripping());
    assert!(run.area.interaction.push_steering().is_none());
    run.area
        .interaction
        .finish_cancelled_input(&mut run.area.player, &mut run.physics);
    assert_push_released(&run, baseline_joints);
    let before = run.physics.rapier().bodies[body].linvel();
    let ordinary_aim = source_push_aim(&reference, &run, false);
    for held in [keys::GRAB | keys::FORWARD, 0] {
        step_input(
            &mut run,
            FrameInput {
                held,
                ..Default::default()
            },
            ordinary_aim,
        );
        assert_push_released(&run, baseline_joints);
        near_vector(run.physics.rapier().bodies[body].linvel(), before);
    }
    step_input(
        &mut run,
        FrameInput {
            held: keys::GRAB,
            pressed: keys::GRAB,
            ..Default::default()
        },
        grip_aim,
    );
    assert!(run.area.interaction.push_gripping());
    let grip_collider = grip_aim.push_target.unwrap().collider;
    run.physics
        .edit_collider(grip_collider, |c| c.set_enabled(false));
    step_input(&mut run, FrameInput::default(), ResolvedAim::default());
    assert_push_released(&run, baseline_joints);
    run.physics
        .edit_collider(grip_collider, |c| c.set_enabled(true));
    step_input(
        &mut run,
        FrameInput {
            held: keys::GRAB,
            pressed: keys::GRAB,
            ..Default::default()
        },
        grip_aim,
    );
    assert!(run.area.interaction.push_gripping());
    run.physics.edit_body(body, |b| b.set_enabled(false));
    step_input(&mut run, FrameInput::default(), ResolvedAim::default());
    assert_push_released(&run, baseline_joints);

    // Same-support rejection and its authored override use the real Lada body.
    for override_support in [false, true] {
        let mut authored = reference.clone();
        authored
            .pushables
            .iter_mut()
            .find(|r| r.owner == rule.owner)
            .unwrap()
            .push_even_if_same_parent = override_support;
        let mut run = GarageRuntime::new(&authored).unwrap();
        push_support(&authored, &mut run, true);
        let aim = source_push_aim(&authored, &run, false);
        let available = run.area.interaction.available_actions(
            &aim,
            &run.area.player,
            &run.area.items,
            &run.physics,
            run.survival.output(),
        );
        assert_eq!(
            available.actions.0 & ActionMask::PUSH,
            if override_support {
                ActionMask::PUSH
            } else {
                0
            }
        );
        let before = run.physics.rapier().bodies[run.physics.chassis_handle(0)].linvel();
        let out = step_input(
            &mut run,
            FrameInput {
                held: keys::GRAB,
                pressed: keys::GRAB,
                ..Default::default()
            },
            aim,
        );
        assert_eq!(
            out.action,
            override_support.then_some(InteractionAction::Push(rule.owner))
        );
        if !override_support {
            near_vector(
                run.physics.rapier().bodies[run.physics.chassis_handle(0)].linvel(),
                before,
            );
        }
        let grip_aim = source_push_aim(&authored, &run, true);
        let baseline_joints = run.physics.rapier().impulse_joints.len();
        step_input(
            &mut run,
            FrameInput {
                held: keys::GRAB,
                pressed: keys::GRAB,
                ..Default::default()
            },
            grip_aim,
        );
        assert_push_released(&run, baseline_joints);
    }
}

fn near_vector(actual: Vector, expected: Vector) {
    assert!(
        (actual - expected).length() < 0.0002,
        "{actual:?} != {expected:?}"
    );
}
fn push_support(reference: &ReferenceManifest, run: &mut GarageRuntime, same_body: bool) {
    use crate::player::PlayerState;
    use rapier3d::prelude::ColliderBuilder;
    let rule = reference
        .pushables
        .iter()
        .find(|r| r.owner.0 == 27)
        .unwrap();
    let body = run.physics.chassis_handle(0);
    // Keep the source chassis and grip. Move the chassis into clear space.
    let shift = Vector::Y * 30.;
    run.physics.edit_body(body, |b| {
        b.set_translation(b.translation() + shift, true);
        b.set_linvel(Vector::ZERO, true);
        b.set_angvel(Vector::ZERO, true);
    });
    run.physics.refresh_queries();
    let grip = *run.physics.rapier().bodies[body].position()
        * Vector::from_array(rule.grip.as_ref().unwrap().local_pose.translation);
    let player_position = grip
        + run.physics.rapier().bodies[body].position().rotation * -Vector::X * 1.2
        + Vector::Y * 0.5;
    let floor = player_position - Vector::Y * (reference.player.standing_height * 0.5 + 0.12);
    let platform = ColliderBuilder::cuboid(0.4, 0.1, 0.4);
    let platform_handle = if same_body {
        let local = run.physics.rapier().bodies[body].position().inverse() * floor;
        run.physics
            .insert_collider_with_parent(platform.translation(local), body)
    } else {
        run.physics.insert_collider(platform.translation(floor))
    };
    run.physics
        .edit_body(run.area.player.body_handle(), |b| b.set_enabled(false));
    run.area.player = PlayerState::new(
        &reference.player,
        VehiclePose {
            position: player_position.to_array(),
            rotation: [0., 0., 0., 1.],
        },
        &mut run.physics,
    );
    run.physics.refresh_queries();
    assert!(!run.area.player.grounded());
    let aim = source_push_aim(reference, run, true);
    let joints = run.physics.rapier().impulse_joints.len();
    step_input(
        run,
        FrameInput {
            held: keys::GRAB,
            pressed: keys::GRAB,
            ..Default::default()
        },
        aim,
    );
    assert_push_released(run, joints);
    for _ in 0..60 {
        run.area
            .player
            .step(FrameInput::default(), &mut run.physics, FIXED_DT);
        if run.area.player.grounded() {
            break;
        }
    }
    assert!(
        run.area.player.grounded(),
        "source grip needs real ground contact"
    );
    let position = Vector::from_array(run.area.player.body_pose(&run.physics).position);
    let ground = run
        .physics
        .queries(
            rapier3d::prelude::QueryFilter::default()
                .exclude_sensors()
                .exclude_rigid_body(run.area.player.body_handle()),
        )
        .cast_ray(
            &rapier3d::prelude::Ray::new(position, -Vector::Y),
            reference.player.standing_height * 0.5 + 0.1,
            true,
        );
    assert_eq!(run.area.player.support_body(), same_body.then_some(body));
    let (ground_collider, toi) = ground.unwrap();
    assert_eq!(ground_collider, platform_handle);
    assert!(toi > 0.);
    assert_eq!(
        run.physics.rapier().colliders[ground_collider].parent(),
        same_body.then_some(body)
    );
    for _ in 0..4 {
        run.area
            .player
            .step(FrameInput::default(), &mut run.physics, FIXED_DT);
        assert!(run.area.player.grounded());
        assert_eq!(run.area.player.support_body(), same_body.then_some(body));
    }
}
fn source_push_aim(reference: &ReferenceManifest, run: &GarageRuntime, grip: bool) -> ResolvedAim {
    use crate::reference::ColliderId;
    let id = ColliderId(if grip { 398 } else { 303 });
    let binding = reference
        .action_bindings
        .iter()
        .find(|b| b.collider == id)
        .unwrap();
    assert_eq!(binding.push_owner.unwrap().0, 27);
    let collider = run
        .physics
        .rapier()
        .colliders
        .iter()
        .find(|(_, c)| (c.user_data >> 64) as u32 == id.0)
        .unwrap()
        .0;
    assert_eq!(
        run.physics.rapier().colliders[collider].parent(),
        Some(run.physics.chassis_handle(0))
    );
    ResolvedAim {
        push_target: Some(AimTarget {
            item: binding.push_owner.unwrap(),
            collider,
            distance: 1.,
            point: run.physics.rapier().colliders[collider]
                .compute_aabb()
                .center()
                .to_array(),
            slot: None,
        }),
        ..Default::default()
    }
}
#[track_caller]
fn assert_push_released(run: &GarageRuntime, baseline_joints: usize) {
    assert!(!run.area.interaction.push_gripping());
    assert!(run.area.interaction.push_steering().is_none());
    assert_eq!(run.physics.rapier().impulse_joints.len(), baseline_joints);
}

#[test]
#[ignore = "requires build 11140890 export"]
fn garage_handling_combined_save_restore() {
    use crate::{
        interaction::{SlotTarget, items::ItemLocation},
        reference::ItemId,
        save::{CameraSave, SourceHash, SourceStamp},
    };
    let (reference, mut run) = source_scene();
    let original_items = run.area.items.iter().count();
    let baseline_joints = run.physics.rapier().impulse_joints.len();
    let chassis = run.physics.chassis_handle(0);
    // Installation owns F-release and cannot also push the Lada.
    let part = reference
        .items
        .iter()
        .find(|i| i.name == "FRDoor")
        .unwrap()
        .id;
    assert_eq!(part, ItemId(61));
    let binding = reference
        .action_bindings
        .iter()
        .find(|b| b.pickup_owner == Some(part) && b.use_owner.is_some())
        .unwrap();
    let collider = run
        .physics
        .rapier()
        .colliders
        .iter()
        .find(|(_, c)| (c.user_data >> 64) as u32 == binding.collider.0)
        .unwrap()
        .0;
    let target = AimTarget {
        item: part,
        collider,
        distance: 1.,
        point: run.area.items.get(part).unwrap().pose.position,
        slot: None,
    };
    let out = step_input(
        &mut run,
        FrameInput {
            held: keys::GRAB,
            pressed: keys::GRAB,
            ..Default::default()
        },
        ResolvedAim {
            grab_target: Some(target),
            ..Default::default()
        },
    );
    assert_eq!(out.action, Some(InteractionAction::PartRemove(part)));
    step_input(
        &mut run,
        FrameInput {
            released: keys::GRAB,
            ..Default::default()
        },
        ResolvedAim::default(),
    );
    let slot = reference
        .part_slots
        .iter()
        .find(|s| s.part == Some(part))
        .unwrap();
    let collider = run
        .physics
        .rapier()
        .colliders
        .iter()
        .find(|(_, c)| {
            slot.colliders
                .iter()
                .any(|id| id.0 == (c.user_data >> 64) as u32)
        })
        .unwrap()
        .0;
    let mut aim = source_push_aim(&reference, &run, false);
    aim.installation_target = Some(AimTarget {
        item: part,
        collider,
        distance: 1.,
        point: target.point,
        slot: Some(SlotTarget::Part(slot.id)),
    });
    let available = run.area.interaction.available_actions(
        &aim,
        &run.area.player,
        &run.area.items,
        &run.physics,
        run.survival.output(),
    );
    assert_ne!(available.actions.0 & ActionMask::MOUNT, 0);
    assert_eq!(
        available.actions.0 & (ActionMask::PUSH | ActionMask::GRIP),
        0
    );
    let before = run.physics.rapier().bodies[chassis].linvel();
    let out = step_input(
        &mut run,
        FrameInput {
            held: keys::GRAB,
            pressed: keys::GRAB,
            ..Default::default()
        },
        aim,
    );
    assert!(!matches!(out.action, Some(InteractionAction::Push(_))));
    let out = step_input(
        &mut run,
        FrameInput {
            released: keys::GRAB,
            ..Default::default()
        },
        aim,
    );
    assert_eq!(out.action, Some(InteractionAction::PartInstall(part)));
    near_vector(run.physics.rapier().bodies[chassis].linvel(), before);
    assert_eq!(
        run.area.items.get(part).unwrap().location,
        ItemLocation::PartSlot(slot.id)
    );
    assert_push_released(&run, baseline_joints);

    run.area.player.rebase([0., -50., 0.]);
    let oil = reference
        .items
        .iter()
        .find(|i| i.name == "OilCan")
        .unwrap()
        .id;
    acquire(&mut run, oil);
    let body = run.area.items.get(oil).unwrap().body.unwrap();
    let authored = run.physics.authored_body_mass(body);
    let before = *run.physics.rapier().bodies[body].position();
    run.area.player.rebase([0.2, 0., 0.]);
    for _ in 0..20 {
        step_input(
            &mut run,
            FrameInput {
                held: keys::ROTATE,
                mouse: [1., 0.5],
                wheel: 0.01,
                ..Default::default()
            },
            ResolvedAim::default(),
        );
        run.physics.step_physics();
    }
    let handled = *run.physics.rapier().bodies[body].position();
    assert!(handled.translation.distance(before.translation) > 0.1);
    assert!(handled.rotation.dot(before.rotation).abs() < 0.999);
    assert_eq!(run.area.interaction.held(), Some(oil));
    assert_eq!(
        run.physics.rapier().impulse_joints.len(),
        baseline_joints + 1
    );
    step_input(
        &mut run,
        FrameInput {
            held: keys::GRAB,
            pressed: keys::GRAB,
            ..Default::default()
        },
        ResolvedAim::default(),
    );
    let out = step_input(
        &mut run,
        FrameInput {
            released: keys::GRAB,
            ..Default::default()
        },
        ResolvedAim::default(),
    );
    assert_eq!(out.action, Some(InteractionAction::Drop(oil)));
    near_mass(run.physics.rapier().bodies[body].mass(), authored);
    assert_eq!(run.physics.rapier().bodies[body].gravity_scale(), 1.);
    assert_push_released(&run, baseline_joints);
    let aim = source_push_aim(&reference, &run, false);
    let before = run.physics.rapier().bodies[chassis].linvel();
    let out = step_input(
        &mut run,
        FrameInput {
            held: keys::GRAB,
            ..Default::default()
        },
        aim,
    );
    assert_eq!(out.action, Some(InteractionAction::Push(ItemId(27))));
    assert!(
        run.physics.rapier().bodies[chassis]
            .linvel()
            .distance(before)
            > 0.01
    );
    run.physics.step_physics();
    step_input(
        &mut run,
        FrameInput {
            released: keys::GRAB,
            ..Default::default()
        },
        ResolvedAim::default(),
    );
    run.area.items.sync_poses(&run.physics);
    let digest = std::process::Command::new("sha256sum")
        .arg("web/assets/reference/manifest.json")
        .output()
        .unwrap();
    assert!(digest.status.success());
    let stamp = SourceStamp {
        app_id: reference.app_id,
        build_id: reference.build_id,
        manifest_sha256: String::from_utf8(digest.stdout)
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
            .into(),
        source_hashes: reference
            .source_hashes
            .iter()
            .map(|h| SourceHash {
                file: h.file.clone(),
                sha256: h.sha256.clone(),
            })
            .collect(),
    };
    acquire(&mut run, oil);
    let saved = run
        .capture_save(
            &reference,
            &stamp,
            CameraSave::FirstPerson,
            "2026-10-20T15:00:00Z",
        )
        .unwrap();
    let mut restored = GarageRuntime::from_save(&reference, &stamp, &saved).unwrap();
    assert_eq!(run.area.items.iter().count(), original_items);
    assert_eq!(restored.area.items.iter().count(), original_items);
    for item in run.area.items.iter() {
        let restored_item = restored.area.items.get(item.id).unwrap();
        assert_eq!(restored_item.location, item.location);
        assert_eq!(restored_item.fluid, item.fluid);
    }
    assert_eq!(restored.physics.vehicle_count(), 1);
    assert_push_released(&restored, baseline_joints + 1);
    step_input(&mut restored, FrameInput::default(), ResolvedAim::default());
    assert_eq!(restored.area.interaction.held(), Some(oil));
    let body = restored.area.items.get(oil).unwrap().body.unwrap();
    assert_eq!(restored.physics.rapier().bodies[body].mass(), 1.);
    let before = *restored.physics.rapier().bodies[body].position();
    restored.area.player.rebase([0.2, 0., 0.]);
    for _ in 0..20 {
        step_input(
            &mut restored,
            FrameInput {
                held: keys::ROTATE,
                mouse: [1., 0.5],
                ..Default::default()
            },
            ResolvedAim::default(),
        );
        restored.physics.step_physics();
    }
    let after = *restored.physics.rapier().bodies[body].position();
    assert!(after.translation.distance(before.translation) > 0.1);
    assert!(after.rotation.dot(before.rotation).abs() < 0.999);
    restored
        .area
        .interaction
        .drop_held(&mut restored.area.items, &mut restored.physics);
    near_mass(restored.physics.rapier().bodies[body].mass(), authored);
    assert_push_released(&restored, baseline_joints);
    assert_eq!(restored.area.items.iter().count(), original_items);
    // Direct owners execute real gameplay here. Normal Runtime::step remains unverified.
}

#[test]
#[ignore = "requires build 11140890 export"]
fn push_feedback_matches_source_support() {
    let (reference, mut run) = source_scene();
    push_support(&reference, &mut run, true);
    let ordinary = source_push_aim(&reference, &run, false);
    let grip = source_push_aim(&reference, &run, true);
    let masks = |run: &GarageRuntime, aim: &ResolvedAim| {
        run.area
            .interaction
            .available_actions(
                aim,
                &run.area.player,
                &run.area.items,
                &run.physics,
                run.survival.output(),
            )
            .actions
            .0
    };
    let same_support_push = masks(&run, &ordinary) & ActionMask::PUSH;
    let same_support_grip = masks(&run, &grip) & ActionMask::GRIP;
    let pose = run.area.player.body_pose(&run.physics);
    run.physics
        .edit_body(run.area.player.body_handle(), |b| b.set_enabled(false));
    run.area.player = crate::player::PlayerState::new(&reference.player, pose, &mut run.physics);
    assert!(!run.area.player.grounded());
    let airborne_grip = masks(&run, &grip) & ActionMask::GRIP;
    assert_eq!(
        (same_support_push, same_support_grip, airborne_grip),
        (0, 0, 0),
        "feedback must share source push and grip eligibility"
    );
}

fn source_door_aim(reference: &ReferenceManifest, run: &GarageRuntime) -> ResolvedAim {
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
    ray_aim(run, collider, |a| {
        a.use_target.is_some_and(|t| t.item == door)
    })
}

#[test]
#[ignore = "requires build 11140890 export"]
fn active_drag_independent_carry_release() {
    use crate::interaction::items::ItemLocation;
    let (reference, mut run) = source_scene();
    let item = reference
        .items
        .iter()
        .find(|i| i.name == "Kolbasz")
        .unwrap()
        .id;
    // Build 11140890: RayStuff returns for drag. Update charges and releases carry separately.
    for frames in [1, 12] {
        acquire(&mut run, item);
        let aim = source_door_aim(&reference, &run);
        let door = aim.use_target.unwrap().item;
        step_input(
            &mut run,
            FrameInput {
                held: keys::USE,
                ..Default::default()
            },
            aim,
        );
        assert_eq!(run.area.interaction.dragging_mechanism(), Some(door));
        let mask = run
            .area
            .interaction
            .available_actions(
                &aim,
                &run.area.player,
                &run.area.items,
                &run.physics,
                run.survival.output(),
            )
            .actions
            .0;
        let before = run
            .area
            .interaction
            .mechanisms
            .rotation_degrees(door)
            .unwrap();
        for frame in 0..frames {
            let out = step_input(
                &mut run,
                FrameInput {
                    held: keys::USE | keys::GRAB,
                    pressed: if frame == 0 { keys::GRAB } else { 0 },
                    mouse: [0.3, 0.2],
                    ..Default::default()
                },
                aim,
            );
            assert_eq!(out.action, Some(InteractionAction::Mechanism(door)));
            assert_eq!(out.look_mouse, [0.; 2]);
            assert_eq!(run.area.interaction.dragging_mechanism(), Some(door));
        }
        let charge = run.area.interaction.throw_charge();
        assert!(
            charge > 0.,
            "active drag must allow independent carried F charge"
        );
        assert_eq!(
            mask & (ActionMask::DROP | ActionMask::THROW),
            ActionMask::DROP | ActionMask::THROW
        );
        assert_eq!(
            mask & (ActionMask::MOUNT
                | ActionMask::DISMOUNT
                | ActionMask::PICKUP
                | ActionMask::PUSH
                | ActionMask::GRIP),
            0
        );
        let during = run
            .area
            .interaction
            .mechanisms
            .rotation_degrees(door)
            .unwrap();
        let out = step_input(
            &mut run,
            FrameInput {
                held: keys::USE,
                released: keys::GRAB,
                mouse: [-0.3, -0.2],
                ..Default::default()
            },
            aim,
        );
        assert_eq!(
            out.action,
            Some(if frames == 1 {
                InteractionAction::Drop(item)
            } else {
                InteractionAction::Throw(item)
            })
        );
        assert_eq!(out.look_mouse, [0.; 2]);
        assert_eq!(run.area.interaction.dragging_mechanism(), Some(door));
        let after = run
            .area
            .interaction
            .mechanisms
            .rotation_degrees(door)
            .unwrap();
        assert!(
            before != during || during != after,
            "drag must still change the source door"
        );
        assert!(run.area.interaction.held().is_none());
        assert_eq!(
            run.area.items.get(item).unwrap().location,
            ItemLocation::World
        );
        step_input(&mut run, FrameInput::default(), ResolvedAim::default());
    }
}

#[test]
#[ignore = "requires build 11140890 export"]
fn active_drag_independent_grip_release() {
    let (reference, mut run) = source_scene();
    push_support(&reference, &mut run, false);
    let baseline_joints = run.physics.rapier().impulse_joints.len();
    let aim = source_push_aim(&reference, &run, true);
    step_input(
        &mut run,
        FrameInput {
            held: keys::GRAB,
            pressed: keys::GRAB,
            ..Default::default()
        },
        aim,
    );
    assert!(run.area.interaction.push_gripping());
    // Existing grip release follows RayStuff in source Update and remains independent.
    let aim = source_door_aim(&reference, &run);
    let door = aim.use_target.unwrap().item;
    step_input(
        &mut run,
        FrameInput {
            held: keys::USE,
            released: keys::GRAB,
            ..Default::default()
        },
        aim,
    );
    assert!(run.area.interaction.push_gripping());
    assert_eq!(run.area.interaction.dragging_mechanism(), Some(door));
    let mask = run
        .area
        .interaction
        .available_actions(
            &aim,
            &run.area.player,
            &run.area.items,
            &run.physics,
            run.survival.output(),
        )
        .actions
        .0;
    let before = run
        .area
        .interaction
        .mechanisms
        .rotation_degrees(door)
        .unwrap();
    let out = step_input(
        &mut run,
        FrameInput {
            held: keys::USE | keys::GRAB,
            pressed: keys::GRAB,
            mouse: [0.3, 0.2],
            ..Default::default()
        },
        aim,
    );
    assert_push_released(&run, baseline_joints);
    assert_ne!(mask & ActionMask::RELEASE_GRIP, 0);
    assert_eq!(
        mask & (ActionMask::MOUNT
            | ActionMask::DISMOUNT
            | ActionMask::PICKUP
            | ActionMask::PUSH
            | ActionMask::GRIP),
        0
    );
    assert_eq!(out.action, Some(InteractionAction::Mechanism(door)));
    assert_eq!(out.look_mouse, [0.; 2]);
    assert_eq!(run.area.interaction.dragging_mechanism(), Some(door));
    let during = run
        .area
        .interaction
        .mechanisms
        .rotation_degrees(door)
        .unwrap();
    let out = step_input(
        &mut run,
        FrameInput {
            held: keys::USE,
            released: keys::GRAB,
            mouse: [-0.3, -0.2],
            ..Default::default()
        },
        aim,
    );
    assert_eq!(out.action, Some(InteractionAction::Mechanism(door)));
    assert_eq!(out.look_mouse, [0.; 2]);
    let after = run
        .area
        .interaction
        .mechanisms
        .rotation_degrees(door)
        .unwrap();
    assert!(
        before != during || during != after,
        "drag must still change the source door"
    );
    assert_push_released(&run, baseline_joints);
}

#[test]
#[ignore = "requires build 11140890 export"]
fn frame_recenter_keeps_source_pose_and_later_handling() {
    use crate::{reference::ItemId, save::IdentityMap};
    let (reference, mut run) = source_scene();
    handling_frame(&mut run, &reference, FrameInput::default(), FIXED_DT);
    let oil = ItemId(120);
    run.area.player.rebase([0., -50., 0.]);
    acquire(&mut run, oil);
    let ids = IdentityMap::new(&reference).unwrap();
    let mut pending = FrameInput {
        held: keys::TRANSLATE,
        carry: crate::simulation::CarryDeltas {
            owner: Some(oil),
            mouse: [10., 11.],
            wheel: 12.,
        },
        ..Default::default()
    };
    let view = run.area.player.view_pose(&run.physics);
    let frame = run.advance_inventory_frame(
        &reference,
        FrameInput {
            held: keys::RECENTER | keys::ROTATE | keys::TRANSLATE,
            pressed: keys::RECENTER,
            mouse: [2., 3.],
            wheel: 4.,
            ..Default::default()
        },
        FIXED_DT,
        view,
    );
    assert_eq!(frame.recentered, Some(oil));
    frame.merge_fixed_input(&mut pending);
    assert_eq!(pending.carry, crate::simulation::CarryDeltas::default());
    assert_eq!(pending.mouse, [0.; 2]);
    let recentered = run
        .area
        .interaction
        .capture_carry_save(&ids)
        .unwrap()
        .unwrap();
    assert_eq!(recentered.offset, [0.; 3]);
    step_fixed_input(&mut run, pending.take_step(), ResolvedAim::default());
    let saved = run
        .area
        .interaction
        .capture_carry_save(&ids)
        .unwrap()
        .unwrap();
    assert_eq!(saved.offset, recentered.offset);
    near_rotation(
        run.physics.rapier().bodies[anchor_handle(&run, oil)]
            .next_position()
            .rotation,
        Rotation::from_array(view.rotation)
            * Rotation::from_array(saved.local_grip.rotation).inverse(),
    );
    let frame = run.advance_inventory_frame(
        &reference,
        FrameInput {
            held: keys::RECENTER | keys::TRANSLATE,
            mouse: [7., 8.],
            wheel: 9.,
            ..Default::default()
        },
        FIXED_DT,
        view,
    );
    frame.merge_fixed_input(&mut pending);
    // No fixed step runs between recenter and this separate handling gesture.
    let later = run.advance_inventory_frame(
        &reference,
        FrameInput {
            held: keys::TRANSLATE,
            mouse: [0.2, 0.3],
            wheel: 0.4,
            ..Default::default()
        },
        FIXED_DT,
        view,
    );
    later.merge_fixed_input(&mut pending);
    assert_eq!(pending.carry.mouse, [0.2, 0.3]);
    assert_eq!(pending.carry.wheel, 0.4);
    step_fixed_input(&mut run, pending.take_step(), ResolvedAim::default());
    assert_ne!(
        run.area
            .interaction
            .capture_carry_save(&ids)
            .unwrap()
            .unwrap()
            .offset,
        recentered.offset
    );
}

#[test]
#[ignore = "requires build 11140890 export"]
fn frame_recenter_preserves_queued_and_current_drag_input() {
    let (reference, mut run) = source_scene();
    handling_frame(&mut run, &reference, FrameInput::default(), FIXED_DT);
    let oil = reference
        .items
        .iter()
        .find(|i| i.name == "OilCan")
        .unwrap()
        .id;
    acquire(&mut run, oil);
    let aim = source_door_aim(&reference, &run);
    let door = aim.use_target.unwrap().item;
    step_input(
        &mut run,
        FrameInput {
            held: keys::USE,
            ..Default::default()
        },
        aim,
    );
    assert_eq!(run.area.interaction.dragging_mechanism(), Some(door));
    run.aim = aim;
    let view = run.area.player.view_pose(&run.physics);
    let prior = run.advance_inventory_frame(
        &reference,
        FrameInput {
            held: keys::USE | keys::ROTATE,
            mouse: [0.2, 0.3],
            ..Default::default()
        },
        FIXED_DT,
        view,
    );
    assert_eq!(prior.fixed_input.mouse, [0.2, 0.3]);
    assert_eq!(prior.fixed_input.carry.mouse, [0.; 2]);
    let mut pending = FrameInput::default();
    prior.merge_fixed_input(&mut pending);
    let current = run.advance_inventory_frame(
        &reference,
        FrameInput {
            held: keys::USE | keys::ROTATE | keys::RECENTER,
            pressed: keys::RECENTER,
            mouse: [0.4, 0.5],
            ..Default::default()
        },
        FIXED_DT,
        view,
    );
    assert_eq!(current.recentered, Some(oil));
    assert_eq!(current.fixed_input.mouse, [0.4, 0.5]);
    current.merge_fixed_input(&mut pending);
    assert_eq!(pending.mouse, [0.6, 0.8]);
    assert_eq!(pending.carry, crate::simulation::CarryDeltas::default());
    let before = run
        .area
        .interaction
        .mechanisms
        .rotation_degrees(door)
        .unwrap();
    let out = step_fixed_input(&mut run, pending.take_step(), aim);
    assert_eq!(out.action, Some(InteractionAction::Mechanism(door)));
    assert_eq!(out.look_mouse, [0.; 2]);
    let during = run
        .area
        .interaction
        .mechanisms
        .rotation_degrees(door)
        .unwrap();
    // Source limits can reject one drag direction at the closed angle.
    let opposite = run.advance_inventory_frame(
        &reference,
        FrameInput {
            held: keys::USE | keys::ROTATE,
            mouse: [-0.6, -0.8],
            ..Default::default()
        },
        FIXED_DT,
        view,
    );
    assert_eq!(opposite.fixed_input.mouse, [-0.6, -0.8]);
    opposite.merge_fixed_input(&mut pending);
    let out = step_fixed_input(&mut run, pending.take_step(), aim);
    assert_eq!(out.action, Some(InteractionAction::Mechanism(door)));
    let after = run
        .area
        .interaction
        .mechanisms
        .rotation_degrees(door)
        .unwrap();
    assert!(
        before != during || during != after,
        "drag must change the source door within its limits"
    );
}
