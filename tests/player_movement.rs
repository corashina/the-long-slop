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
    run.area.player.step(
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
        player.step(
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
            player.step(FrameInput::default(), context, &mut run.physics, FIXED_DT);
            run.physics.step_physics();
        }
        assert!(player.grounded());
        assert!((run.physics.rapier().bodies[player.body_handle()].mass() - 72.).abs() < 1e-5);
        let before = Vector::from_array(player.body_pose(&run.physics).position);
        player.step(
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
        player.step(
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
            player.step(
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
            player.step(
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
            player.step(
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
    run.area.player.step(
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
