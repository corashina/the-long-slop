//! Critical source posture, complete geometry, and persistence checks.
use my_summer_slop::{
    player::{PlayerState, PlayerStepContext, collision::PlayerSweepShape},
    reference::ReferenceManifest,
    save::{CameraSave, SourceHash, SourceStamp},
    simulation::{FIXED_DT, FrameInput, keys},
    starting_area::GarageRuntime,
    vehicle::VehiclePose,
};
use rapier3d::{
    math::{Pose, Vector},
    parry::{
        query::intersection_test,
        shape::{Ball, Shape},
    },
};

fn garage() -> (ReferenceManifest, GarageRuntime, SourceStamp) {
    let path = "web/assets/reference/manifest.json";
    let bytes = std::fs::read(path).expect("Task 3 requires the installed branch runtime export");
    let r = ReferenceManifest::from_json(&bytes)
        .expect("Installed export must include recovered G09 player records");
    let digest = std::process::Command::new("sha256sum")
        .arg(path)
        .output()
        .unwrap();
    assert!(digest.status.success());
    let stamp = SourceStamp {
        app_id: r.app_id,
        build_id: r.build_id,
        manifest_sha256: String::from_utf8(digest.stdout)
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
            .into(),
        source_hashes: r
            .source_hashes
            .iter()
            .map(|h| SourceHash {
                file: h.file.clone(),
                sha256: h.sha256.clone(),
            })
            .collect(),
    };
    let game = GarageRuntime::new(&r).unwrap();
    (r, game, stamp)
}

#[test]
fn source_posture_collision_and_locomotion() {
    let (r, mut run, _) = garage();
    let context = PlayerStepContext {
        wheel_posture_allowed: true,
        ..Default::default()
    };
    run.area.player.step_view(
        FrameInput {
            wheel: -16.6 / r.player.scroll_sensitivity,
            ..Default::default()
        },
        context,
        FIXED_DT,
    );
    run.area.player.step_movement(
        FrameInput {
            wheel: -16.6 / r.player.scroll_sensitivity,
            ..Default::default()
        },
        context,
        &mut run.physics,
        FIXED_DT,
    );
    assert!((run.area.player.posture() - 0.17).abs() < 1.0e-6);
    for value in [0.0, 0.17, 0.3, 1.0] {
        let mut shape = PlayerSweepShape::new(&r.player);
        shape.set_posture(&r.player, value);
        assert!(shape.compute_local_aabb().half_extents().min_element() > 0.0);
        assert_eq!(shape.feet_enabled(), value > 0.3);
    }
    // Use an isolated floor so source world objects cannot change measured caps.
    let floor = run.physics.insert_collider(
        rapier3d::prelude::ColliderBuilder::cuboid(20., 0.1, 20.)
            .translation(Vector::new(2000., -0.1, 2000.)),
    );
    run.physics.refresh_queries();
    for (posture, run_key, expected) in [
        (1., 0, 1.75),
        (1., keys::RUN, 4.375),
        (0.17, 0, 0.8225),
        (0.3, 0, 1.05),
        (0., 0, 0.525),
    ] {
        let mut player = PlayerState::new(
            &r.player,
            VehiclePose {
                position: [2000., 0.9, 2000.],
                rotation: [0., 0., 0., 1.],
            },
            &mut run.physics,
        );
        player.step_view(
            FrameInput {
                wheel: (posture - 1.)
                    / (r.player.posture.scroll_step * r.player.scroll_sensitivity),
                ..Default::default()
            },
            context,
            FIXED_DT,
        );
        player.step_movement(
            FrameInput {
                wheel: (posture - 1.)
                    / (r.player.posture.scroll_step * r.player.scroll_sensitivity),
                ..Default::default()
            },
            context,
            &mut run.physics,
            FIXED_DT,
        );
        for _ in 0..100 {
            player.step_view(FrameInput::default(), context, FIXED_DT);
            player.step_movement(FrameInput::default(), context, &mut run.physics, FIXED_DT);
            run.physics.step_physics();
        }
        assert!(player.grounded());
        assert!((run.physics.rapier().bodies[player.body_handle()].mass() - 72.).abs() < 1e-5);
        let before = Vector::from_array(player.body_pose(&run.physics).position);
        player.step_view(
            FrameInput {
                held: keys::FORWARD | run_key,
                ..Default::default()
            },
            context,
            FIXED_DT,
        );
        player.step_movement(
            FrameInput {
                held: keys::FORWARD | run_key,
                ..Default::default()
            },
            context,
            &mut run.physics,
            FIXED_DT,
        );
        let after = Vector::from_array(player.body_pose(&run.physics).position);
        assert!(((after - before).z.abs() / FIXED_DT - expected).abs() < 0.01);
        let jump_before = player
            .capture_save(
                &my_summer_slop::save::IdentityMap::new(&r).unwrap(),
                &run.stream.origin,
                &run.physics,
                None,
            )
            .unwrap()
            .vertical_velocity;
        player.step_view(
            FrameInput {
                held: keys::SPACE,
                pressed: keys::SPACE,
                ..Default::default()
            },
            context,
            FIXED_DT,
        );
        player.step_movement(
            FrameInput {
                held: keys::SPACE,
                pressed: keys::SPACE,
                ..Default::default()
            },
            context,
            &mut run.physics,
            FIXED_DT,
        );
        let jump_after = player
            .capture_save(
                &my_summer_slop::save::IdentityMap::new(&r).unwrap(),
                &run.stream.origin,
                &run.physics,
                None,
            )
            .unwrap()
            .vertical_velocity;
        let expected_jump = r.player.jump_force / r.player.mass_kg * r.player.source_force_interval
            + run.physics.rapier().gravity.y * FIXED_DT;
        assert!((jump_after - jump_before - expected_jump).abs() < 1e-5);
        if posture == 1. {
            let snapshot =
                |player: &PlayerState, physics: &my_summer_slop::vehicle::PhysicsWorld| {
                    player
                        .capture_save(
                            &my_summer_slop::save::IdentityMap::new(&r).unwrap(),
                            &run.stream.origin,
                            physics,
                            None,
                        )
                        .unwrap()
                        .vertical_velocity
                };
            let before = snapshot(&player, &run.physics);
            player.step_view(
                FrameInput {
                    held: keys::SPACE,
                    pressed: keys::SPACE,
                    ..Default::default()
                },
                context,
                FIXED_DT,
            );
            player.step_movement(
                FrameInput {
                    held: keys::SPACE,
                    pressed: keys::SPACE,
                    ..Default::default()
                },
                context,
                &mut run.physics,
                FIXED_DT,
            );
            let after = snapshot(&player, &run.physics);
            let impulse = r.player.jump_force / r.player.mass_kg * r.player.source_force_interval;
            let gravity = run.physics.rapier().gravity.y * FIXED_DT;
            assert!(
                (after - before - impulse * r.player.duck_jump_multiplier - gravity).abs() < 1e-5
            );
            player.step_view(
                FrameInput {
                    held: keys::SPACE,
                    pressed: keys::SPACE,
                    ..Default::default()
                },
                context,
                FIXED_DT,
            );
            player.step_movement(
                FrameInput {
                    held: keys::SPACE,
                    pressed: keys::SPACE,
                    ..Default::default()
                },
                context,
                &mut run.physics,
                FIXED_DT,
            );
            let once = snapshot(&player, &run.physics);
            assert!((once - after - gravity).abs() < 1e-5);
            player.step_view(
                FrameInput {
                    released: keys::SPACE,
                    ..Default::default()
                },
                context,
                FIXED_DT,
            );
            player.step_movement(
                FrameInput {
                    released: keys::SPACE,
                    ..Default::default()
                },
                context,
                &mut run.physics,
                FIXED_DT,
            );
            let release = snapshot(&player, &run.physics);
            assert!(
                (release - once + impulse * r.player.inverse_duck_multiplier - gravity).abs()
                    < 1e-5
            );
        }
        let body = &run.physics.rapier().bodies[player.body_handle()];
        for handle in body.colliders() {
            assert_eq!(
                std::sync::Arc::strong_count(
                    &run.physics.rapier().colliders[*handle].shared_shape().0
                ),
                1
            );
        }

        run.physics.remove_body(player.body_handle());
    }
    run.physics.remove_collider(floor);
    let mut shape = PlayerSweepShape::new(&r.player);
    shape.set_posture(&r.player, 0.31);
    let foot = Vector::from_array(r.player.posture.feet.center);
    let obstacle = Pose::from_translation(foot - Vector::Y * (shape.radius() - 0.005));
    assert!(
        intersection_test(&Pose::IDENTITY, &shape, &obstacle, &Ball::new(0.002))
            .unwrap()
            .intersecting
    );
    assert!(
        !intersection_test(
            &shape.capsule_pose(),
            shape.capsule(),
            &obstacle,
            &Ball::new(0.002)
        )
        .unwrap()
        .intersecting
    );
}

#[test]
fn source_posture_save_and_recovery() {
    let (r, mut run, stamp) = garage();
    run.area.player.step_view(
        FrameInput {
            wheel: -16.6 / r.player.scroll_sensitivity,
            ..Default::default()
        },
        PlayerStepContext {
            wheel_posture_allowed: true,
            ..Default::default()
        },
        FIXED_DT,
    );
    run.area.player.step_movement(
        FrameInput {
            wheel: -16.6 / r.player.scroll_sensitivity,
            ..Default::default()
        },
        PlayerStepContext {
            wheel_posture_allowed: true,
            ..Default::default()
        },
        &mut run.physics,
        FIXED_DT,
    );
    assert!((run.area.player.posture() - 0.17).abs() < 1.0e-6);
    let saved = run
        .capture_save(&r, &stamp, CameraSave::FirstPerson, "2026-10-20T10:00:00Z")
        .unwrap();
    assert_eq!(saved.player.posture, run.area.player.posture());
    let restored = GarageRuntime::from_save(&r, &stamp, &saved).unwrap();
    assert_eq!(restored.area.player.posture(), saved.player.posture);
    let oil = r
        .items
        .iter()
        .find(|item| item.name == "OilCan")
        .unwrap()
        .id;
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
    let held = run.area.items.get(oil).unwrap().body.unwrap();
    let held_before = *run.physics.rapier().bodies[held].position();
    let pose_before = run.area.player.body_pose(&run.physics);
    let posture_before = run.area.player.posture();
    let vehicle_pose = run.physics.snapshot(0).pose;
    run.physics.insert_collider(
        rapier3d::prelude::ColliderBuilder::cuboid(100., 100., 100.)
            .translation(Vector::from_array(vehicle_pose.position)),
    );
    run.physics.refresh_queries();
    run.collect_recovery_ownership(&r);
    assert!(run.recovery_clearance(vehicle_pose).is_none());
    assert_eq!(run.area.player.body_pose(&run.physics), pose_before);
    assert_eq!(run.area.player.posture(), posture_before);
    assert_eq!(*run.physics.rapier().bodies[held].position(), held_before);
    for value in [-0.01, 1.01, f32::NAN, f32::INFINITY] {
        let mut invalid = saved.clone();
        invalid.player.posture = value;
        assert!(invalid.validate(&r, &stamp).is_err());
    }
    let mut invalid = saved.clone();
    invalid.schema_version = 6;
    assert!(invalid.validate(&r, &stamp).is_err());
    assert!(
        my_summer_slop::save::SaveSnapshot::from_json(&serde_json::to_vec(&invalid).unwrap())
            .is_err()
    );
}

#[test]
fn free_look_preserves_body_and_head_anchors() {
    let (_, mut run, _) = garage();
    let body_before = run.area.player.body_pose(&run.physics);
    let head_before = run.area.player.view(&run.physics).head_pose;
    run.area.player.step_view(
        FrameInput {
            player_held: my_summer_slop::simulation::player_keys::LEAN,
            player_pressed: my_summer_slop::simulation::player_keys::LEAN,
            mouse: [60., 20.],
            ..Default::default()
        },
        PlayerStepContext::default(),
        FIXED_DT,
    );
    run.area.player.step_movement(
        FrameInput {
            player_held: my_summer_slop::simulation::player_keys::LEAN,
            player_pressed: my_summer_slop::simulation::player_keys::LEAN,
            mouse: [60., 20.],
            ..Default::default()
        },
        PlayerStepContext::default(),
        &mut run.physics,
        FIXED_DT,
    );
    let view = run.area.player.view(&run.physics);
    assert_ne!(view.camera_pose.rotation, view.head_pose.rotation);
    assert!(!view.ray_actions_blocked);
    assert_eq!(view.head_pose.rotation, head_before.rotation);
    assert_eq!(
        run.area.player.body_pose(&run.physics).rotation,
        body_before.rotation
    );
}

fn bring_item_to_head(
    r: &ReferenceManifest,
    run: &mut GarageRuntime,
    id: my_summer_slop::reference::ItemId,
) {
    let item = run.area.items.get(id).unwrap();
    let body = item.body.unwrap();
    let head = my_summer_slop::vehicle::to_rapier(run.area.player.view(&run.physics).head_pose);
    let target = Pose::from_parts(
        head.translation + head.rotation * Vector::from_array(r.player.carry_anchor_offset),
        head.rotation,
    ) * my_summer_slop::vehicle::to_rapier(item.local_pose).inverse();
    run.physics
        .edit_body(body, |body| body.set_position(target, true));
    run.area.items.sync_poses(&run.physics);
    run.physics.refresh_queries();
}

fn source_target(
    r: &ReferenceManifest,
    run: &GarageRuntime,
    item: my_summer_slop::reference::ItemId,
    use_target: bool,
) -> my_summer_slop::interaction::AimTarget {
    let binding = r
        .action_bindings
        .iter()
        .find(|binding| {
            if use_target {
                binding.use_owner == Some(item)
            } else {
                binding.pickup_owner == Some(item)
            }
        })
        .unwrap();
    let (collider, _) = run
        .physics
        .rapier()
        .colliders
        .iter()
        .find(|(_, collider)| (collider.user_data >> 64) as u32 == binding.collider.0)
        .unwrap();
    my_summer_slop::interaction::AimTarget {
        collider,
        item,
        distance: 0.1,
        point: run.area.items.get(item).unwrap().pose.position,
        slot: None,
    }
}
fn interact(
    run: &mut GarageRuntime,
    input: FrameInput,
    aim: my_summer_slop::interaction::ResolvedAim,
) -> Option<my_summer_slop::interaction::InteractionAction> {
    let needs = run.needs_output();
    run.area.interaction.step(
        input,
        aim,
        &mut run.area.player,
        &mut run.area.items,
        &mut run.physics,
        FIXED_DT,
        needs,
        1.,
    )
}
#[test]
fn q_tap_boundary_and_cancel() {
    use my_summer_slop::simulation::player_keys;
    for (duration, tapped) in [(0.19, true), (0.2, false)] {
        let (_, mut run, _) = garage();
        run.area.player.step_view(
            FrameInput {
                player_pressed: player_keys::LEAN,
                player_held: player_keys::LEAN,
                ..Default::default()
            },
            Default::default(),
            duration,
        );
        run.area.player.step_view(
            FrameInput {
                player_released: player_keys::LEAN,
                ..Default::default()
            },
            Default::default(),
            FIXED_DT,
        );
        let yaw = run.area.player.yaw_degrees();
        run.area.player.step_view(
            FrameInput {
                mouse: [10., 0.],
                ..Default::default()
            },
            Default::default(),
            FIXED_DT,
        );
        assert_eq!(run.area.player.yaw_degrees() == yaw, tapped);
    }
    let (r, mut run, _) = garage();
    let wheel = FrameInput {
        wheel: -16.6 / r.player.scroll_sensitivity,
        ..Default::default()
    };
    run.area.player.step_movement(
        wheel,
        PlayerStepContext {
            wheel_posture_allowed: true,
            ..Default::default()
        },
        &mut run.physics,
        FIXED_DT,
    );
    let posture = run.area.player.posture();
    run.area.player.step_view(
        FrameInput {
            player_pressed: player_keys::LEAN,
            player_held: player_keys::LEAN,
            mouse: [20., 10.],
            ..Default::default()
        },
        Default::default(),
        0.1,
    );
    run.clear_input();
    let neutral = run
        .area
        .player
        .neutral_view_at(run.area.player.body_pose(&run.physics));
    assert_eq!(
        run.area.player.view(&run.physics).camera_pose,
        neutral.camera_pose
    );
    run.area.player.step_view(
        FrameInput {
            player_released: player_keys::LEAN,
            mouse: [10., 0.],
            ..Default::default()
        },
        Default::default(),
        FIXED_DT,
    );
    assert_eq!(run.area.player.yaw_degrees(), 10.);
    assert_eq!(run.area.player.posture(), posture);
}
#[test]
fn seated_lean_blocks_ray_actions() {
    use my_summer_slop::{
        interaction::{InteractionAction, ResolvedAim},
        simulation::player_keys,
    };
    let (r, mut run, _) = garage();
    let seat = &r.seats[0];
    let anchor = r.anchors.iter().find(|a| a.id == seat.anchor).unwrap();
    let head_before = run.area.player.view(&run.physics).head_pose;
    assert!(run.area.player.enter(
        seat.id,
        VehiclePose {
            position: anchor.transform.translation,
            rotation: anchor.transform.rotation
        },
        &mut run.physics
    ));
    let entered = run.area.player.view(&run.physics).head_pose;
    assert!(
        (Vector::from_array(entered.position) - Vector::from_array(head_before.position)).length()
            < 1e-5
    );
    assert!(
        rapier3d::math::Rotation::from_array(entered.rotation)
            .dot(rapier3d::math::Rotation::from_array(head_before.rotation))
            .abs()
            > 0.99999
    );
    let oil = r.items.iter().find(|i| i.name == "OilCan").unwrap().id;
    bring_item_to_head(&r, &mut run, oil);
    let target = source_target(&r, &run, oil, false);
    let aim = ResolvedAim {
        grab_target: Some(target),
        ..Default::default()
    };
    run.area.player.step_view(
        FrameInput {
            player_held: player_keys::LEAN,
            player_pressed: player_keys::LEAN,
            mouse: [10000., 0.],
            ..Default::default()
        },
        Default::default(),
        FIXED_DT,
    );
    assert!(run.area.player.view(&run.physics).ray_actions_blocked);
    let input = FrameInput {
        held: keys::GRAB,
        pressed: keys::GRAB,
        ..Default::default()
    };
    assert!(interact(&mut run, input, aim).is_none());
    assert!(run.area.interaction.held().is_none());
    run.area.player.step_view(
        FrameInput {
            player_released: player_keys::LEAN,
            ..Default::default()
        },
        Default::default(),
        FIXED_DT,
    );
    assert!(!run.area.player.view(&run.physics).ray_actions_blocked);
    assert_eq!(
        interact(&mut run, input, aim),
        Some(InteractionAction::Pickup(oil))
    );
}
#[test]
fn seat_target_press_release_matches() {
    use my_summer_slop::interaction::{InteractionAction, ResolvedAim};
    let (r, mut run, _) = garage();
    let oil = r
        .items
        .iter()
        .find(|item| item.name == "OilCan")
        .unwrap()
        .id;
    bring_item_to_head(&r, &mut run, oil);
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
    let first = source_target(&r, &run, r.seats[0].item, true);
    let second = source_target(&r, &run, r.seats[1].item, true);
    interact(
        &mut run,
        FrameInput {
            held: keys::USE,
            pressed: keys::USE,
            ..Default::default()
        },
        ResolvedAim {
            use_target: Some(first),
            ..Default::default()
        },
    );
    // These source seats have no usable component. Their null identity and hold exemption apply.
    for _ in 0..20 {
        interact(
            &mut run,
            FrameInput {
                held: keys::USE,
                ..Default::default()
            },
            ResolvedAim {
                use_target: Some(first),
                ..Default::default()
            },
        );
    }
    assert_eq!(
        interact(
            &mut run,
            FrameInput {
                released: keys::USE,
                ..Default::default()
            },
            ResolvedAim {
                use_target: Some(second),
                ..Default::default()
            }
        ),
        Some(InteractionAction::Seat(r.seats[1].id))
    );
    assert_eq!(run.area.player.seat(), Some(r.seats[1].id));
    assert_eq!(run.area.interaction.held(), Some(oil));
}
#[test]
fn exit_boundary_inherits_every_parent_velocity_axis() {
    use my_summer_slop::player::ExitReference;
    let (r, mut run, _) = garage();
    let seat = &r.seats[0];
    let anchor = r.anchors.iter().find(|a| a.id == seat.anchor).unwrap();
    assert!(run.area.player.enter(
        seat.id,
        VehiclePose {
            position: anchor.transform.translation,
            rotation: anchor.transform.rotation
        },
        &mut run.physics
    ));
    let colliders: Vec<_> = run
        .physics
        .rapier()
        .colliders
        .iter()
        .map(|(handle, _)| handle)
        .collect();
    for handle in colliders {
        run.physics
            .edit_collider(handle, |collider| collider.set_enabled(false));
    }
    run.physics.refresh_queries();
    let limits = ExitReference::from_reference(&r.player, seat);
    let before = run.area.player.body_pose(&run.physics);
    let yaw = run.area.player.yaw_degrees();
    run.area.player.step_view(
        FrameInput {
            mouse: [64.99 - yaw, 0.],
            ..Default::default()
        },
        Default::default(),
        FIXED_DT,
    );
    assert!(!run.area.player.try_exit(&mut run.physics, &limits));
    assert_eq!(run.area.player.seat(), Some(seat.id));
    assert_eq!(run.area.player.body_pose(&run.physics), before);
    run.area.player.step_view(
        FrameInput {
            mouse: [0.01, 0.],
            ..Default::default()
        },
        Default::default(),
        FIXED_DT,
    );
    let chassis = run.physics.chassis_handle(seat.vehicle);
    run.physics.edit_body(chassis, |body| {
        body.set_linvel(Vector::new(2., 1., -3.), true)
    });
    assert!(run.area.player.try_exit(&mut run.physics, &limits));
    let saved = run
        .area
        .player
        .capture_save(
            &my_summer_slop::save::IdentityMap::new(&r).unwrap(),
            &run.stream.origin,
            &run.physics,
            None,
        )
        .unwrap();
    for (actual, expected) in [
        saved.recoil_velocity[0],
        saved.vertical_velocity,
        saved.recoil_velocity[2],
    ]
    .into_iter()
    .zip([0.6, 0.3, -0.9])
    {
        assert!((actual - expected).abs() < 1e-6);
    }
}

#[test]
fn seated_recenter_follows_carried_object_priority() {
    let (r, mut run, _) = garage();
    let seat = &r.seats[0];
    let anchor = r.anchors.iter().find(|a| a.id == seat.anchor).unwrap();
    assert!(run.area.player.enter(
        seat.id,
        VehiclePose {
            position: anchor.transform.translation,
            rotation: anchor.transform.rotation
        },
        &mut run.physics
    ));
    run.area.player.step_view(
        FrameInput {
            mouse: [30., 20.],
            ..Default::default()
        },
        Default::default(),
        FIXED_DT,
    );
    let yaw = run.area.player.yaw_degrees();
    let pitch = run.area.player.pitch_degrees();
    run.area.player.step_view(
        FrameInput {
            held: keys::RECENTER,
            ..Default::default()
        },
        PlayerStepContext {
            recenter_consumed: true,
            ..Default::default()
        },
        FIXED_DT,
    );
    assert_eq!(run.area.player.yaw_degrees(), yaw);
    assert_eq!(run.area.player.pitch_degrees(), pitch);
    run.area.player.step_view(
        FrameInput {
            held: keys::RECENTER,
            mouse: [2., 1.],
            ..Default::default()
        },
        Default::default(),
        FIXED_DT,
    );
    let amount = FIXED_DT * r.player.posture.rise_rate;
    assert!((run.area.player.yaw_degrees() - (yaw * (1. - amount) + 2.)).abs() < 1e-5);
    assert!(
        (run.area.player.pitch_degrees()
            - (pitch * (1. - amount) + seat.default_pitch_degrees * amount + 1.))
            .abs()
            < 1e-5
    );
}

#[test]
fn carried_q_wheel_routes_to_posture_without_distance_change() {
    use my_summer_slop::simulation::player_keys;
    let (r, mut run, stamp) = garage();
    let oil = r
        .items
        .iter()
        .find(|item| item.name == "OilCan")
        .unwrap()
        .id;
    bring_item_to_head(&r, &mut run, oil);
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
    let input = FrameInput {
        player_held: player_keys::LEAN,
        player_pressed: player_keys::LEAN,
        wheel: -1. / r.player.scroll_sensitivity,
        ..Default::default()
    };
    run.area.player.step_view(
        input,
        PlayerStepContext {
            duck_allowed: true,
            wheel_posture_allowed: true,
            picked_up: true,
            ..Default::default()
        },
        FIXED_DT,
    );
    let before = run
        .capture_save(&r, &stamp, CameraSave::FirstPerson, "2026-10-20T10:00:00Z")
        .unwrap()
        .player
        .carry
        .unwrap()
        .offset;
    interact(&mut run, input, Default::default());
    run.area.player.step_movement(
        input,
        PlayerStepContext {
            duck_allowed: true,
            wheel_posture_allowed: true,
            picked_up: true,
            ..Default::default()
        },
        &mut run.physics,
        FIXED_DT,
    );
    let after = run
        .capture_save(&r, &stamp, CameraSave::FirstPerson, "2026-10-20T10:00:00Z")
        .unwrap()
        .player
        .carry
        .unwrap()
        .offset;
    assert_eq!(before, after);
    assert!((run.area.player.posture() - 0.95).abs() < 1e-6);
}

#[test]
fn installed_zoom_uses_candidate_base_and_neutral_load() {
    use my_summer_slop::simulation::player_keys;
    let (reference, mut live, stamp) = garage();
    let saved = live
        .capture_save(
            &reference,
            &stamp,
            CameraSave::FirstPerson,
            "2026-10-20T10:00:00Z",
        )
        .unwrap();
    live.area.player.step_view(
        FrameInput {
            player_held: player_keys::ZOOM,
            mouse: [10., 20.],
            ..Default::default()
        },
        Default::default(),
        1.,
    );
    let view = live.area.player.view(&live.physics);
    assert!((view.fov_degrees - 26.).abs() < 1e-5);
    let yaw_delta =
        (live.area.player.yaw_degrees() - saved.player.yaw + 180.).rem_euclid(360.) - 180.;
    assert!((yaw_delta - 4.).abs() < 1e-5);
    let candidate = GarageRuntime::from_save(&reference, &stamp, &saved).unwrap();
    let restored = candidate.area.player.view(&candidate.physics);
    assert_eq!(restored.fov_degrees, reference.player.view.foot_fov_degrees);
    assert_eq!(candidate.area.player.posture(), live.area.player.posture());
}
