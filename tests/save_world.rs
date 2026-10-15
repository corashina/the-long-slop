mod common;
use my_summer_slop::starting_area::GarageRuntime;
use my_summer_slop::world::{nearest_station, sync_station_supplies};

#[test]
fn station_identity_is_visit_order_independent() {
    let reference = common::reference();
    let visit = |order: &[i64]| {
        let mut game = GarageRuntime::new(&reference).unwrap();
        for station in order {
            let point = nearest_station(
                game.stream.seed,
                *station as f64 * 640.,
                &reference.stations,
            )
            .absolute_point;
            game.stream.sync_at(point, &mut game.physics);
            sync_station_supplies(
                &mut game.stream,
                &mut game.area.items,
                &mut game.area.interaction,
                &mut game.physics,
            );
        }
        let key = my_summer_slop::save::InstanceId::Station {
            seed: game.stream.seed.to_string(),
            index: order.last().unwrap().to_string(),
            role: "pump".into(),
        };
        let binding = game.stream.station_item(&key).unwrap().unwrap();
        assert!(game.area.items.get(binding).unwrap().fluid.is_some());
        game.stream.capture_save(&game.area.interaction)
    };
    let forward = visit(&[10, 20, 30]);
    let reverse = visit(&[30, 20, 10]);
    assert_eq!(forward, reverse);
}

#[test]
fn walking_focus_keeps_parked_vehicle_support() {
    use rapier3d::math::Vector;
    use rapier3d::prelude::{QueryFilter, Ray};
    let reference = common::reference();
    let mut game = GarageRuntime::new(&reference).unwrap();
    let car = game
        .stream
        .origin
        .absolute(game.physics.snapshot(0).pose.position);
    let focus = [car[0] + 800., 0., car[2] - 1600.];
    game.stream.sync_at(focus, &mut game.physics);
    game.physics.refresh_queries();
    for point in [car, focus] {
        let mut local = game.stream.origin.local(point);
        local[1] = 30.;
        let ray = Ray::new(Vector::from_array(local), -Vector::Y);
        assert!(
            game.physics
                .queries(QueryFilter::default().exclude_rigid_body(game.physics.chassis_handle(0)))
                .cast_ray(&ray, 50., true)
                .is_some(),
            "missing support at {point:?}"
        );
    }
}
#[test]
fn saved_coordinates_reject_extreme_finite_positions() {
    use my_summer_slop::save::*;
    let reference = common::reference();
    let source = SourceStamp {
        app_id: reference.app_id,
        build_id: reference.build_id,
        manifest_sha256: "a".repeat(64),
        source_hashes: reference
            .source_hashes
            .iter()
            .map(|hash| SourceHash {
                file: hash.file.clone(),
                sha256: hash.sha256.clone(),
            })
            .collect(),
    };
    let game = GarageRuntime::new(&reference).unwrap();
    let saved = game
        .capture_save(
            &reference,
            &source,
            CameraSave::FirstPerson,
            "2026-10-12T12:00:00Z",
        )
        .unwrap();
    let bound = (1_u64 << 45) as f64;
    for axis in 0..3 {
        for value in [-bound, bound] {
            let mut sample = saved.clone();
            sample.player.pose.position[axis] = value;
            sample.validate(&reference, &source).unwrap();
        }
        for value in [-f64::MAX, f64::MAX, -bound - 1., bound + 1.] {
            let mut sample = saved.clone();
            sample.player.pose.position[axis] = value;
            assert!(
                sample.validate(&reference, &source).is_err(),
                "accepted {value} on {axis}"
            );
        }
    }
    let mut sample = saved.clone();
    sample.world.records.push(WorldRecord::Pump {
        instance: InstanceId::Station {
            seed: "42".into(),
            index: i64::MAX.to_string(),
            role: "pump".into(),
        },
        valve: 1.,
    });
    assert!(sample.validate(&reference, &source).is_err());
}
#[test]
fn distant_dropped_assembly_survives_stream_out_and_restore() {
    use my_summer_slop::save::{CameraSave, SourceHash, SourceStamp};
    use rapier3d::math::{Pose, Vector};
    let mut reference = common::reference();
    let rim = reference.wheel_slots[0].rim;
    let mut condition = reference.conditions.surfaces[0].clone();
    condition.id.0 = 900;
    condition.source.path_id = 900;
    condition.item = Some(rim);
    condition.node = reference.items.iter().find(|i| i.id == rim).unwrap().node;
    condition.renderers.clear();
    reference.conditions.surfaces.push(condition);
    let mut game = GarageRuntime::new(&reference).unwrap();
    let slot = reference.vehicles[0].wheel_slots[0];
    let rim = game
        .wheels
        .remove(slot, &mut game.area.items, &mut game.physics)
        .unwrap();
    common::set_condition(&mut game.area, &reference, rim, 2);
    let handle = game.area.items.get(rim).unwrap().body.unwrap();
    game.physics.rapier_mut().bodies[handle]
        .set_position(Pose::from_translation(Vector::new(40., 5., -20.)), true);
    game.physics.rapier_mut().bodies[handle].set_linvel(Vector::new(1., 2., 3.), true);
    game.area.items.sync_poses(&game.physics);
    let source = SourceStamp {
        app_id: reference.app_id,
        build_id: reference.build_id,
        manifest_sha256: "a".repeat(64),
        source_hashes: reference
            .source_hashes
            .iter()
            .map(|h| SourceHash {
                file: h.file.clone(),
                sha256: h.sha256.clone(),
            })
            .collect(),
    };
    let home = game.area.bodies[&my_summer_slop::reference::BodyId(1)];
    game.physics.rapier_mut().bodies[home].set_translation(Vector::new(12.25, 0., 8.5), false);
    game.area
        .items
        .get_mut(my_summer_slop::reference::ItemId(18))
        .unwrap()
        .fluid
        .as_mut()
        .unwrap()
        .mixture
        .quantities_liters = [3., 1., 0., 0., 0., 0.];
    game.area.items.sync_poses(&game.physics);
    let before = game
        .capture_save(
            &reference,
            &source,
            CameraSave::FirstPerson,
            "2026-10-12T12:00:00Z",
        )
        .unwrap();
    let bodies = game.physics.rapier().bodies.len();
    let colliders = game.physics.collider_count();
    let focus = [0., 0., -10000.];
    game.stream.sync_at(focus, &mut game.physics);
    game.persistent_world
        .sync(
            &reference,
            &mut game.area,
            &mut game.wheels,
            &game.stream,
            &mut game.physics,
        )
        .unwrap();
    assert!(!game.physics.rapier().bodies[handle].is_enabled());
    assert_eq!(
        game.physics.rapier().bodies[handle].translation(),
        Vector::ZERO
    );
    let away = game
        .capture_save(
            &reference,
            &source,
            CameraSave::FirstPerson,
            "2026-10-12T12:00:00Z",
        )
        .unwrap();
    assert_eq!(before.bodies, away.bodies);
    assert_eq!(before.items, away.items);
    assert_eq!(before.conditions, away.conditions);
    assert_eq!(before.attachments, away.attachments);
    let ids = my_summer_slop::save::IdentityMap::new(&reference).unwrap();
    let mut restored = GarageRuntime::new(&reference).unwrap();
    restored
        .wheels
        .remove(slot, &mut restored.area.items, &mut restored.physics)
        .unwrap();
    for saved in &away.items {
        let item = restored
            .area
            .items
            .get_mut(ids.item(&saved.instance).unwrap())
            .unwrap();
        item.temperature = saved.temperature;
        item.local_pose = my_summer_slop::vehicle::VehiclePose {
            position: saved.local_pose.position,
            rotation: saved.local_pose.rotation,
        };
        if let (Some(fluid), Some(quantities)) = (&mut item.fluid, saved.fluids) {
            fluid.mixture.quantities_liters = quantities;
        }
    }
    restored
        .area
        .conditions
        .restore_save(&reference, &away.conditions)
        .unwrap();
    restored.persistent_world =
        my_summer_slop::save::PersistentWorld::restore(&away, &ids).unwrap();
    restored.stream.sync_at(focus, &mut restored.physics);
    restored
        .persistent_world
        .sync(
            &reference,
            &mut restored.area,
            &mut restored.wheels,
            &restored.stream,
            &mut restored.physics,
        )
        .unwrap();
    let restored_save = restored
        .capture_save(
            &reference,
            &source,
            CameraSave::FirstPerson,
            "2026-10-12T12:00:00Z",
        )
        .unwrap();
    assert_eq!(away.bodies, restored_save.bodies);
    assert_eq!(away.items, restored_save.items);
    game = restored;
    let handle = game.area.items.get(rim).unwrap().body.unwrap();
    for _ in 0..3 {
        game.stream.sync_at([0., 0., 0.], &mut game.physics);
        game.persistent_world
            .sync(
                &reference,
                &mut game.area,
                &mut game.wheels,
                &game.stream,
                &mut game.physics,
            )
            .unwrap();
        assert!(game.physics.rapier().bodies[handle].is_enabled());
        assert_eq!(
            game.physics.rapier().bodies[handle].translation(),
            Vector::new(40., 5., -20.)
        );
        assert_eq!(
            game.physics.rapier().bodies[handle].linvel(),
            Vector::new(1., 2., 3.)
        );
        assert_eq!(game.physics.rapier().bodies.len(), bodies);
        assert_eq!(game.physics.collider_count(), colliders);
        game.stream.sync_at(focus, &mut game.physics);
        game.persistent_world
            .sync(
                &reference,
                &mut game.area,
                &mut game.wheels,
                &game.stream,
                &mut game.physics,
            )
            .unwrap();
    }
}

#[test]
fn remote_car_freezes_and_resumes_without_ghosts() {
    use my_summer_slop::player::PlayerState;
    use my_summer_slop::simulation::FrameInput;
    use my_summer_slop::vehicle::VehiclePose;
    use rapier3d::math::Vector;
    use rapier3d::prelude::{QueryFilter, Ray};
    let reference = common::reference();
    let mut game = GarageRuntime::new(&reference).unwrap();
    game.started = true;
    let old = game.area.player.body_handle();
    game.physics.rapier_mut().remove_body(old);
    game.area.player = PlayerState::new(
        &reference.player,
        VehiclePose {
            position: [0., 10., -10000.],
            rotation: [0., 0., 0., 1.],
        },
        &mut game.physics,
    );
    game.step(&reference, FrameInput::default());
    let chassis = game.physics.chassis_handle(0);
    assert!(!game.physics.body_active(chassis));
    let engine = game.vehicles[0].engine.capture_save();
    let absolute =
        game.persistent_world
            .absolute_vehicle_position(&game.physics, &game.stream.origin, 0);
    let ids = my_summer_slop::save::IdentityMap::new(&reference).unwrap();
    let wheels = game.wheels.capture_save(&ids, &game.physics).unwrap();
    for _ in 0..10 {
        game.step(&reference, FrameInput::default());
    }
    assert_eq!(game.vehicles[0].engine.capture_save(), engine);
    assert_eq!(
        game.wheels.capture_save(&ids, &game.physics).unwrap(),
        wheels
    );
    assert_eq!(
        game.persistent_world
            .absolute_vehicle_position(&game.physics, &game.stream.origin, 0),
        absolute
    );
    assert_eq!(
        game.physics.rapier().bodies[chassis].translation(),
        Vector::ZERO
    );
    let only_car = |_: my_summer_slop::vehicle::ColliderHandle, c: &rapier3d::prelude::Collider| {
        c.user_data & my_summer_slop::vehicle::VEHICLE_COLLIDER_FLAG != 0
    };
    assert!(
        game.physics
            .queries(QueryFilter::default().predicate(&only_car))
            .cast_ray(&Ray::new(Vector::new(0., 10., 0.), -Vector::Y), 20., true)
            .is_none()
    );
    assert!(
        game.physics
            .rapier()
            .colliders
            .iter()
            .filter(|(_, c)| c.user_data & my_summer_slop::vehicle::SLOT_SENSOR_FLAG != 0)
            .all(|(_, c)| !c.is_enabled())
    );
    game.stream.origin.offset = [0.; 3];
    game.area.origin = [0.; 3];
    game.stream.sync_at(absolute, &mut game.physics);
    game.persistent_world
        .sync(
            &reference,
            &mut game.area,
            &mut game.wheels,
            &game.stream,
            &mut game.physics,
        )
        .unwrap();
    assert!(game.physics.body_active(chassis));
    assert_eq!(
        game.stream
            .origin
            .absolute(game.physics.snapshot(0).pose.position),
        absolute
    );
}

#[test]
fn dormant_car_recovery_keeps_cargo_and_logical_state() {
    use my_summer_slop::player::PlayerState;
    use my_summer_slop::simulation::FrameInput;
    use my_summer_slop::vehicle::VehiclePose;
    use rapier3d::math::Vector;
    let reference = common::reference();
    let mut game = GarageRuntime::new(&reference).unwrap();
    game.started = true;
    let rim = game
        .wheels
        .remove(
            reference.vehicles[0].wheel_slots[0],
            &mut game.area.items,
            &mut game.physics,
        )
        .unwrap();
    let cargo = game.area.items.get(rim).unwrap().body.unwrap();
    let chassis = game.physics.chassis_handle(0);
    let car = *game.physics.rapier().bodies[chassis].position();
    game.physics.rapier_mut().bodies[cargo]
        .set_translation(car.translation + Vector::new(0., 0.1, 0.), true);
    let old = game.area.player.body_handle();
    game.physics.rapier_mut().remove_body(old);
    game.area.player = PlayerState::new(
        &reference.player,
        VehiclePose {
            position: [0., 10., -10000.],
            rotation: [0., 0., 0., 1.],
        },
        &mut game.physics,
    );
    game.step(&reference, FrameInput::default());
    assert!(!game.physics.body_active(chassis));
    assert!(!game.physics.body_active(cargo));
    let ids = my_summer_slop::save::IdentityMap::new(&reference).unwrap();
    let engine = game.vehicles[0].engine.capture_save();
    let wheels = game.wheels.capture_save(&ids, &game.physics).unwrap();
    let fluids = game
        .area
        .items
        .iter()
        .filter_map(|item| {
            item.fluid
                .as_ref()
                .map(|f| (item.id, f.mixture.quantities_liters))
        })
        .collect::<Vec<_>>();
    let trip = game.vehicles[0].trip_m;
    game.recover(&reference);
    assert!(game.physics.body_active(chassis));
    assert!(game.physics.body_active(cargo));
    assert!(
        (game
            .stream
            .origin
            .absolute(game.physics.snapshot(0).pose.position)[2]
            + 10000.)
            .abs()
            < 1.
    );
    assert_eq!(game.vehicles[0].engine.capture_save(), engine);
    assert_eq!(
        game.wheels.capture_save(&ids, &game.physics).unwrap(),
        wheels
    );
    assert_eq!(game.vehicles[0].trip_m, trip);
    for (id, amounts) in fluids {
        assert_eq!(
            game.area
                .items
                .get(id)
                .unwrap()
                .fluid
                .as_ref()
                .unwrap()
                .mixture
                .quantities_liters,
            amounts
        );
    }
    let relative = game.physics.rapier().bodies[chassis].position().inverse()
        * game.physics.rapier().bodies[cargo].position().translation;
    assert!(relative.length() < 1.0, "cargo left behind: {relative:?}");
}

#[test]
#[ignore = "requires installed source export"]
fn installed_world_hibernation_preserves_source_inventory() {
    use my_summer_slop::reference::ReferenceManifest;
    use my_summer_slop::save::*;
    let reference =
        ReferenceManifest::from_json(&std::fs::read("web/assets/reference/manifest.json").unwrap())
            .unwrap();
    let source = SourceStamp {
        app_id: reference.app_id,
        build_id: reference.build_id,
        manifest_sha256: "a".repeat(64),
        source_hashes: reference
            .source_hashes
            .iter()
            .map(|h| SourceHash {
                file: h.file.clone(),
                sha256: h.sha256.clone(),
            })
            .collect(),
    };
    let mut game = GarageRuntime::new(&reference).unwrap();
    let before = game
        .capture_save(
            &reference,
            &source,
            CameraSave::FirstPerson,
            "2026-10-12T12:00:00Z",
        )
        .unwrap();
    game.stream.sync_at([0., 0., -10000.], &mut game.physics);
    game.persistent_world
        .sync(
            &reference,
            &mut game.area,
            &mut game.wheels,
            &game.stream,
            &mut game.physics,
        )
        .unwrap();
    let away = game
        .capture_save(
            &reference,
            &source,
            CameraSave::FirstPerson,
            "2026-10-12T12:00:00Z",
        )
        .unwrap();
    assert_eq!(before.items, away.items);
    assert_eq!(before.conditions, away.conditions);
    assert_eq!(before.bodies, away.bodies);
    assert_eq!(before.mechanisms, away.mechanisms);
    game.stream.sync_at(
        reference.area.player_spawn.translation.map(f64::from),
        &mut game.physics,
    );
    game.persistent_world
        .sync(
            &reference,
            &mut game.area,
            &mut game.wheels,
            &game.stream,
            &mut game.physics,
        )
        .unwrap();
    let returned = game
        .capture_save(
            &reference,
            &source,
            CameraSave::FirstPerson,
            "2026-10-12T12:00:00Z",
        )
        .unwrap();
    assert_eq!(before.items, returned.items);
    assert_eq!(before.bodies, returned.bodies);
}

#[test]
fn rotated_body_stays_inactive_until_its_geometry_fits() {
    use my_summer_slop::reference::BodyId;
    use rapier3d::math::{Pose, Rotation, Vector};
    use rapier3d::prelude::SharedShape;
    let reference = common::reference();
    let mut game = GarageRuntime::new(&reference).unwrap();
    let handle = game.area.bodies[&BodyId(1)];
    let colliders = game.physics.rapier().bodies[handle].colliders().to_vec();
    assert!(!colliders.is_empty());
    for collider in colliders {
        let collider = &mut game.physics.rapier_mut().colliders[collider];
        collider.set_shape(SharedShape::cuboid(1., 1., 1.));
        collider.set_position_wrt_parent(Pose::IDENTITY);
    }
    let position = [2046.8, 5., 0.];
    game.physics.rapier_mut().bodies[handle]
        .set_position(Pose::from_translation(Vector::from_array(position)), false);
    game.stream
        .sync_at(position.map(f64::from), &mut game.physics);
    game.persistent_world
        .sync(
            &reference,
            &mut game.area,
            &mut game.wheels,
            &game.stream,
            &mut game.physics,
        )
        .unwrap();
    assert!(game.physics.body_active(handle), "axis-aligned cube fits");
    game.physics.rapier_mut().bodies[handle].set_rotation(
        Rotation::from_rotation_y(std::f32::consts::FRAC_PI_4),
        false,
    );
    game.persistent_world
        .sync(
            &reference,
            &mut game.area,
            &mut game.wheels,
            &game.stream,
            &mut game.physics,
        )
        .unwrap();
    assert!(
        !game.physics.body_active(handle),
        "rotated cube crosses origin bound"
    );
    // Dormant bounds must use the saved rotation, not the parked body's rotation.
    game.physics.rapier_mut().bodies[handle].set_rotation(Rotation::IDENTITY, false);
    game.persistent_world
        .sync(
            &reference,
            &mut game.area,
            &mut game.wheels,
            &game.stream,
            &mut game.physics,
        )
        .unwrap();
    assert!(
        !game.physics.body_active(handle),
        "saved rotation still crosses bound"
    );
    game.stream.origin.shift([2., 0., 0.]);
    game.stream
        .sync_at(position.map(f64::from), &mut game.physics);
    game.persistent_world
        .sync(
            &reference,
            &mut game.area,
            &mut game.wheels,
            &game.stream,
            &mut game.physics,
        )
        .unwrap();
    assert!(
        game.physics.body_active(handle),
        "rotated cube fits after origin shift"
    );
}
