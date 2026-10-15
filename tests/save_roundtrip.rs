mod common;
use my_summer_slop::{reference::*, save::*, simulation::FrameInput, starting_area::GarageRuntime};
use rapier3d::math::Vector;
fn fixture() -> ReferenceManifest {
    let mut r = common::reference();
    r.openings[0].valve = 0.37;
    r.vehicles[0].engine.start_chance = 0.5;
    r
}

fn stamp(r: &ReferenceManifest) -> SourceStamp {
    SourceStamp {
        app_id: r.app_id,
        build_id: r.build_id,
        manifest_sha256: "a".repeat(64),
        source_hashes: r
            .source_hashes
            .iter()
            .map(|s| my_summer_slop::save::SourceHash {
                file: s.file.clone(),
                sha256: s.sha256.clone(),
            })
            .collect(),
    }
}
fn capture(g: &GarageRuntime, r: &ReferenceManifest) -> SaveSnapshot {
    g.capture_save(
        r,
        &stamp(r),
        CameraSave::FirstPerson,
        "2026-10-12T12:00:00Z",
    )
    .unwrap()
}
fn assert_logical_equal(a: &SaveSnapshot, b: &SaveSnapshot) {
    fn compare(a: &serde_json::Value, b: &serde_json::Value, path: &str) {
        match (a, b) {
            (serde_json::Value::Number(x), serde_json::Value::Number(y))
                if x.is_f64() || y.is_f64() =>
            {
                let tolerance = if path.contains("position") {
                    1e-3
                } else {
                    1e-5
                };
                assert!(
                    (x.as_f64().unwrap() - y.as_f64().unwrap()).abs() <= tolerance,
                    "{path}: {x} != {y}"
                );
            }
            (serde_json::Value::Object(x), serde_json::Value::Object(y)) => {
                assert_eq!(x.len(), y.len());
                for (k, v) in x {
                    if k != "saved_at" {
                        compare(v, &y[k], &format!("{path}.{k}"));
                    }
                }
            }
            (serde_json::Value::Array(x), serde_json::Value::Array(y)) => {
                assert_eq!(x.len(), y.len(), "{path}: {x:?} != {y:?}");
                for (i, (v, w)) in x.iter().zip(y).enumerate() {
                    compare(v, w, &format!("{path}[{i}]"));
                }
            }
            _ => assert_eq!(a, b, "{path}"),
        }
    }
    compare(
        &serde_json::to_value(a).unwrap(),
        &serde_json::to_value(b).unwrap(),
        "",
    );
}
#[test]
fn pause_save_continue_clears_short_ignition_hold() {
    starter_boundary(1, false);
}
#[test]
fn pause_save_continue_clears_engaged_starter() {
    starter_boundary(120, true);
}
fn starter_boundary(steps: usize, engaged: bool) {
    use my_summer_slop::simulation::keys;
    let r = fixture();
    let mut original = GarageRuntime::new(&r).unwrap();
    original.started = true;
    let seat = r.seats.iter().find(|s| s.driver).unwrap();
    assert!(original.area.player.enter(
        seat.id,
        my_summer_slop::vehicle::VehiclePose {
            position: [0., 1., 0.],
            rotation: [0., 0., 0., 1.]
        },
        &mut original.physics
    ));
    for step in 0..steps {
        original.step(
            &r,
            FrameInput {
                held: keys::IGNITION,
                pressed: if step == 0 { keys::IGNITION } else { 0 },
                ..Default::default()
            },
        );
    }
    assert_eq!(original.vehicles[0].engine_output.starter, engaged);
    original.paused = true;
    let before = capture(&original, &r);
    original.clear_input();
    let saved = capture(&original, &r);
    assert_eq!(saved.vehicles[0].engine, before.vehicles[0].engine);
    assert_eq!(saved.progress, before.progress);
    assert_eq!(saved.player.recoil_velocity, before.player.recoil_velocity);
    assert_eq!(saved.items, before.items);
    assert!(
        !original.vehicles[0].engine_output.starter,
        "cached starter remains engaged"
    );
    let mut loaded = GarageRuntime::from_save(&r, &stamp(&r), &saved).unwrap();
    assert_eq!(
        original.vehicles[0].engine.capture_save(),
        loaded.vehicles[0].engine.capture_save()
    );
    original.clear_input();
    loaded.clear_input();
    original.paused = false;
    loaded.paused = false;
    original.step(&r, FrameInput::default());
    loaded.step(&r, FrameInput::default());
    assert_eq!(
        original.vehicles[0].engine.ignition,
        saved
            .vehicles
            .iter()
            .find(|v| v.instance == InstanceId::initial(&r.vehicles[0].source))
            .unwrap()
            .engine
            .ignition,
        "neutral input released a canceled short hold"
    );
    assert_eq!(
        original.vehicles[0].engine.ignition,
        loaded.vehicles[0].engine.ignition
    );
    assert!(!original.vehicles[0].engine_output.starter);
    assert!(!loaded.vehicles[0].engine_output.starter);
    assert_eq!(original.attempts, loaded.attempts);
}
#[test]
fn driven_rebased_roundtrip_preserves_logic_and_resumes() {
    let mut r = fixture();
    r.vehicles[0].engine.start_chance = 1.;
    let mut g = GarageRuntime::new(&r).unwrap();
    g.started = true;
    use my_summer_slop::simulation::keys;
    let seat = r.seats.iter().find(|s| s.driver).unwrap();
    assert!(g.area.player.enter(
        seat.id,
        my_summer_slop::vehicle::VehiclePose {
            position: [0., 1., 0.],
            rotation: [0., 0., 0., 1.]
        },
        &mut g.physics
    ));
    for step in 0..240 {
        g.step(
            &r,
            FrameInput {
                held: keys::IGNITION,
                pressed: if step == 0 { keys::IGNITION } else { 0 },
                ..Default::default()
            },
        );
    }
    assert!(
        g.vehicles[0].engine_output.running,
        "{:?}, mounted {}",
        g.vehicles[0].engine_output,
        g.area.items.mounted(r.vehicles[0].engine.item)
    );
    g.vehicles[0].handbrake = false;
    for _ in 0..60 {
        g.step(
            &r,
            FrameInput {
                held: keys::FORWARD,
                ..Default::default()
            },
        );
    }
    assert!(g.vehicles[0].trip_m > 0.1);
    assert!(
        g.wheels
            .capture_save(&IdentityMap::new(&r).unwrap(), &g.physics)
            .unwrap()
            .0
            .iter()
            .any(|w| w.rpm != 0.)
    );
    let h = g.physics.chassis_handle(0);
    g.physics.rapier_mut().bodies[h].set_linvel(Vector::new(1., 0., -3.), true);
    g.vehicles[0].trip_m = 1234.5;
    g.attempts = 9007199254740993;
    let shift = [2048., 0., -4096.];
    g.physics.rebase(shift);
    g.stream.origin.shift(shift);
    g.area.sync_origin(g.stream.origin.offset);
    let saved = capture(&g, &r);
    let mut loaded = GarageRuntime::from_save(&r, &stamp(&r), &saved).unwrap();
    assert!(loaded.paused && loaded.started);
    assert_logical_equal(&saved, &capture(&loaded, &r));
    loaded.paused = false;
    loaded.step(&r, FrameInput::default());
    assert!(
        (loaded.vehicles[0].trip_m
            - saved
                .vehicles
                .iter()
                .find(|v| v.instance == InstanceId::initial(&r.vehicles[0].source))
                .unwrap()
                .trip_m)
            .abs()
            < 1.
    );
}
#[test]
fn removed_part_split_wheel_and_held_item_rebuild() {
    let mut r = fixture();
    let engine = r.vehicles[0].engine.item;
    let engine_condition = common::add_condition_owner(&mut r, engine);
    let portable = r
        .tanks
        .iter()
        .find(|t| t.role == TankRole::Portable && t.capacity_liters >= 69.)
        .unwrap()
        .item;
    let cap = r.openings.iter().find(|o| o.tank == portable).unwrap().cap;
    r.tanks
        .iter_mut()
        .find(|t| t.item == portable)
        .unwrap()
        .quantities_liters = [0., 0., 0., 69., 0., 0.];
    let node = r.items.iter().find(|i| i.id == portable).unwrap().node;
    let mut body = r.bodies[1].clone();
    body.id = BodyId(91);
    body.source.path_id = 90093;
    body.node = node;
    body.parent_body = None;
    body.mass_kg = 75.0; // Six kilograms dry plus 69 liters from the source tank.
    body.pose.translation = [10., 5., 0.];
    r.bodies.push(body);
    let node = r.nodes.iter_mut().find(|n| n.id == node).unwrap();
    node.parent = None;
    node.transform.translation = [10., 5., 0.];
    for id in [portable, cap] {
        r.items.iter_mut().find(|i| i.id == id).unwrap().body = BodyId(91);
    }
    r.tanks
        .iter_mut()
        .find(|t| t.item == portable)
        .unwrap()
        .role = TankRole::Portable;
    let mut g = GarageRuntime::new(&r).unwrap();
    g.started = true;
    for _ in 0..120 {
        g.step(&r, FrameInput::default());
    }
    g.area
        .interaction
        .parts
        .remove(engine, &mut g.area.items, &mut g.physics)
        .unwrap();
    g.area.interaction.parts.detach_mechanism(
        engine,
        &mut g.area.interaction.mechanisms,
        &mut g.physics,
    );
    common::set_condition(&mut g.area, &r, engine, 3);
    assert_eq!(g.area.conditions.item_quality(engine), Some(0.25));
    g.area
        .items
        .get_mut(portable)
        .unwrap()
        .fluid
        .as_mut()
        .unwrap()
        .mixture
        .quantities_liters = [0., 0., 0., 10., 0., 0.];
    // Normal simulation commits the changed masses before the save.
    g.step(&r, FrameInput::default());
    let portable_body = g.area.items.get(portable).unwrap().body.unwrap();
    let portable_mass = g.physics.rapier().bodies[portable_body].mass();
    let portable_volume = g
        .area
        .items
        .get(portable)
        .unwrap()
        .fluid
        .as_ref()
        .unwrap()
        .mixture
        .total();
    assert!(portable_mass < 75. && (portable_mass - 6. - portable_volume).abs() < 1e-5);
    let slot = r.wheel_slots[0].id;
    if let Some(cover) = g.area.interaction.parts.covering_item(r.wheel_slots[0].rim) {
        common::detach(&mut g, cover);
    }
    let rim = g
        .wheels
        .remove(slot, &mut g.area.items, &mut g.physics)
        .unwrap();
    let (_, tire) = g
        .wheels
        .split(rim, &mut g.area.items, &mut g.physics)
        .unwrap();
    let point = g.area.items.get(tire).unwrap().pose.position;
    g.area
        .interaction
        .pickup_item(
            tire,
            point,
            &g.area.player,
            &mut g.area.items,
            &mut g.physics,
        )
        .unwrap();
    g.physics.reset_preserving_wheels(
        0,
        my_summer_slop::vehicle::VehiclePose {
            position: [0., 0.45, -30.],
            rotation: [0., 0., 0., 1.],
        },
    );
    g.area.sync_attachments(&mut g.physics, false);
    g.area
        .interaction
        .sync_parts(&mut g.area.items, &mut g.physics)
        .unwrap();
    g.wheels.sync(&mut g.area.items, &mut g.physics).unwrap();
    g.area.items.sync_poses(&g.physics);
    let saved = capture(&g, &r);
    let mut loaded = GarageRuntime::from_save(&r, &stamp(&r), &saved).unwrap();
    assert_logical_equal(&saved, &capture(&loaded, &r));
    assert_eq!(loaded.area.conditions.item_quality(engine), Some(0.25));
    assert_eq!(loaded.area.conditions.get(engine_condition).state, 3);
    assert_eq!(loaded.physics.wheel_support_count(0), 3);
    assert_eq!(loaded.area.interaction.held(), Some(tire));
    assert_eq!(
        loaded.physics.rapier().impulse_joints.len(),
        g.physics.rapier().impulse_joints.len()
    );
    let old = g.area.items.get(rim).unwrap().body.unwrap();
    assert!(
        loaded
            .area
            .interaction
            .parts
            .slot_item(
                r.part_slots
                    .iter()
                    .find(|s| s.part == Some(engine))
                    .unwrap()
                    .id
            )
            .is_none()
    );
    let new = loaded.area.items.get(rim).unwrap().body.unwrap();
    assert!(
        (g.physics.rapier().bodies[old].mass() - loaded.physics.rapier().bodies[new].mass()).abs()
            < 1e-5
    );
    let masses: Vec<_> = [engine, portable]
        .into_iter()
        .map(|id| {
            let old = g.area.items.get(id).unwrap().body.unwrap();
            let new = loaded.area.items.get(id).unwrap().body.unwrap();
            (
                id,
                g.physics.rapier().bodies[old].mass(),
                loaded.physics.rapier().bodies[new].mass(),
            )
        })
        .collect();
    assert!(
        masses
            .iter()
            .all(|(_, old, new)| *old > 0. && (old - new).abs() < 1e-5),
        "paused candidate mass mismatch: {masses:?}"
    );
    g.started = true;
    loaded.paused = false;
    let pose = loaded.physics.snapshot(0).pose.position;
    g.step(&r, FrameInput::default());
    loaded.step(&r, FrameInput::default());
    assert_eq!(
        g.physics.snapshot(0).wheels_contact,
        loaded.physics.snapshot(0).wheels_contact
    );
    assert!(
        loaded.physics.snapshot(0).wheels_contact.iter().any(|c| *c),
        "original {:?} loaded {:?}",
        g.physics.snapshot(0),
        loaded.physics.snapshot(0)
    );
    let next = loaded.physics.snapshot(0).pose.position;
    assert!(pose.iter().zip(next).all(|(a, b)| (*a - b).abs() < 0.1));
    assert!(!loaded.physics.rapier().bodies[new].colliders().is_empty());
    for handle in loaded.physics.rapier().bodies[new].colliders() {
        assert_eq!(
            loaded.physics.rapier().colliders[*handle].parent(),
            Some(new)
        );
    }
    loaded.physics.rapier_mut().bodies[new].set_translation(Vector::new(5., 4., 5.), true);
    loaded.physics.rapier_mut().bodies[new].set_linvel(Vector::new(0., -2., 0.), true);
    loaded.paused = false;
    for _ in 0..180 {
        loaded.step(&r, FrameInput::default());
    }
    let y = loaded.physics.rapier().bodies[new].translation().y;
    assert!(
        (-1.0..2.0).contains(&y),
        "loose rim did not collide with terrain: {y}"
    );
}
#[test]
fn fluid_mechanism_lighting_roundtrip_resumes() {
    let mut r = common::reference();
    let mut body = r.bodies[1].clone();
    body.id = BodyId(90);
    body.source.path_id = 90090;
    body.parent_body = Some(r.vehicles[0].body);
    r.bodies.push(body);
    let mechanism_id = MechanismId(r.mechanisms.iter().map(|m| m.id.0).max().unwrap() + 1);
    common::add_rotation(&mut r, 1, 90, [-30., 40.]);
    r.mechanisms.last_mut().unwrap().id = mechanism_id;
    for surface in &mut r.conditions.surfaces {
        if let Some(item) = surface.item {
            surface.node = r.items.iter().find(|i| i.id == item).unwrap().node;
        }
    }
    let supply_id = r
        .tanks
        .iter()
        .find(|t| t.role == TankRole::Portable && t.quantities_liters[0] >= 0.25)
        .unwrap()
        .item;
    // The helper shares source keys; this save fixture needs unique stable identities.
    for (i, n) in r.nodes.iter_mut().enumerate() {
        n.source.path_id = 100000 + i as i64;
    }
    let mut g = GarageRuntime::new(&r).unwrap();
    let mut supply = g
        .area
        .items
        .get(supply_id)
        .unwrap()
        .fluid
        .as_ref()
        .unwrap()
        .clone();
    let destination = g
        .area
        .items
        .get_mut(r.vehicles[0].engine.fuel_tank)
        .unwrap()
        .fluid
        .as_mut()
        .unwrap();
    assert_eq!(
        my_summer_slop::fluids::transfer(&mut supply, destination, 0.25),
        0.25
    );
    g.area.items.get_mut(supply_id).unwrap().fluid = Some(supply);
    let control = r.mechanisms.last().unwrap().item;
    let view = g.area.player.view_pose(&g.physics);
    g.area.interaction.mechanisms.step(
        control,
        FrameInput {
            held: my_summer_slop::simulation::keys::USE,
            mouse: [-10., 0.],
            ..Default::default()
        },
        view,
        0.02,
    );
    g.area.interaction.sync_mechanisms(&mut g.physics);
    g.area.items.sync_poses(&g.physics);
    assert_ne!(
        g.area.interaction.mechanisms.rotation_degrees(control),
        Some([0.; 2])
    );
    g.lighting.cycle.advance(
        &r.lighting.cycle,
        my_summer_slop::lighting::CycleAdvance {
            simulation_delta: 18.25,
            cycle_extra: 0.,
            cloud_scale: 1.,
            sleep_progress: None,
        },
    );
    g.lighting.cycle_headlights(&r.lighting, 0);
    for c in &r.lighting.controllers {
        if (c.kind == LightControllerKind::Switch && c.usable_source.is_some())
            || c.kind == LightControllerKind::Cabin
        {
            g.lighting.use_control(&r.lighting, c.id);
        }
    }
    g.lighting.advance_fades(&r.lighting, 0.001);
    let mut saved = capture(&g, &r);
    saved.mechanisms.openings[0].valve = 0.37;
    assert!(
        saved
            .lighting
            .controllers
            .iter()
            .any(|c| c.energy > 0. && c.energy < 1.
                || matches!(c.switch_motion, Some(SwitchMotionSave::Moving { .. })))
    );
    let mut loaded = GarageRuntime::from_save(&r, &stamp(&r), &saved).unwrap();
    assert_logical_equal(&saved, &capture(&loaded, &r));
    let ids = IdentityMap::new(&r).unwrap();
    g.lighting.advance_fades(&r.lighting, 0.02);
    loaded.lighting.advance_fades(&r.lighting, 0.02);
    assert_eq!(
        g.lighting.capture_save(&r.lighting, &ids).unwrap(),
        loaded.lighting.capture_save(&r.lighting, &ids).unwrap()
    );
    for game in [&mut g, &mut loaded] {
        game.area.interaction.mechanisms.step(
            control,
            FrameInput {
                held: my_summer_slop::simulation::keys::USE,
                mouse: [-2., 0.],
                ..Default::default()
            },
            view,
            0.02,
        );
        game.area.interaction.sync_mechanisms(&mut game.physics);
    }
    assert_eq!(
        g.area.interaction.mechanisms.rotation_degrees(control),
        loaded.area.interaction.mechanisms.rotation_degrees(control)
    );
    loaded.paused = false;
    loaded.step(&r, FrameInput::default());
}
#[test]
fn failed_candidate_keeps_original_runtime() {
    let r = common::reference();
    let g = GarageRuntime::new(&r).unwrap();
    let saved = capture(&g, &r);
    let mut bad = saved.clone();
    for kind in 0..9 {
        let mut unsafe_save = saved.clone();
        match kind {
            0 => unsafe_save.vehicles[0].body.linear_velocity[0] = f32::MAX,
            1 => unsafe_save.vehicles[0].body.angular_velocity[1] = f32::MAX,
            2 => unsafe_save.items[0].local_pose.position[0] = f32::MAX,
            3 => unsafe_save.player.vertical_velocity = f32::MAX,
            4 => unsafe_save.attachments.wheels[0].rotation = f32::MAX,
            5 => unsafe_save.attachments.wheels[0].previous_rotation = -f32::MAX,
            6 => unsafe_save.player.recoil_velocity[2] = f32::MAX,
            7 => unsafe_save.vehicles[0].trip_m = f64::MAX,
            _ => unsafe_save.lighting.cycle.elapsed_seconds = f64::MAX,
        }
        assert!(
            GarageRuntime::from_save(&r, &stamp(&r), &unsafe_save).is_err(),
            "unsafe field {kind}"
        );
        assert_logical_equal(&saved, &capture(&g, &r));
    }
    bad.attachments.wheels[0].rim = None;
    assert!(GarageRuntime::from_save(&r, &stamp(&r), &bad).is_err());
    assert_logical_equal(&saved, &capture(&g, &r));
}
#[test]
#[ignore = "requires installed source export"]
fn installed_source_roundtrip() {
    let r =
        ReferenceManifest::from_json(&std::fs::read("web/assets/reference/manifest.json").unwrap())
            .unwrap();
    let mut g = GarageRuntime::new(&r).unwrap();
    let saved = capture(&g, &r);
    let loaded = GarageRuntime::from_save(&r, &stamp(&r), &saved).unwrap();
    assert_logical_equal(&saved, &capture(&loaded, &r));
    let removable = r
        .parts
        .iter()
        .find(|p| {
            g.area
                .interaction
                .parts
                .can_remove(p.item, &g.area.player, &g.area.items)
                .is_ok()
        })
        .unwrap()
        .item;
    g.area
        .interaction
        .parts
        .remove(removable, &mut g.area.items, &mut g.physics)
        .unwrap();
    g.area.interaction.parts.detach_mechanism(
        removable,
        &mut g.area.interaction.mechanisms,
        &mut g.physics,
    );
    g.area
        .interaction
        .sync_parts(&mut g.area.items, &mut g.physics)
        .unwrap();
    let slot = &r.wheel_slots[0];
    if let Some(cover) = g.area.interaction.parts.covering_item(slot.rim) {
        g.area
            .interaction
            .parts
            .remove(cover, &mut g.area.items, &mut g.physics)
            .unwrap();
        g.area.interaction.parts.detach_mechanism(
            cover,
            &mut g.area.interaction.mechanisms,
            &mut g.physics,
        );
    }
    let spare = r
        .wheel_parts
        .iter()
        .find(|p| {
            r.items
                .iter()
                .any(|i| i.id == p.item && matches!(i.kind, ItemKind::Rim))
                && !r.wheel_slots.iter().any(|s| s.rim == p.item)
        })
        .unwrap()
        .item;
    g.wheels
        .remove(slot.id, &mut g.area.items, &mut g.physics)
        .unwrap();
    g.wheels
        .install(spare, slot.id, &mut g.area.items, &mut g.physics)
        .unwrap();
    let saved = capture(&g, &r);
    let mut loaded = GarageRuntime::from_save(&r, &stamp(&r), &saved).unwrap();
    assert_logical_equal(&saved, &capture(&loaded, &r));
    let spare_source = r.items.iter().find(|i| i.id == spare).unwrap().body;
    assert!(
        !loaded.physics.rapier().bodies[loaded.area.bodies[&spare_source]].is_enabled(),
        "mounted spare left its old body enabled"
    );
    loaded.paused = false;
    loaded.step(&r, FrameInput::default());
}
#[test]
fn instruments_initialize_without_time_step() {
    use my_summer_slop::simulation::instruments::*;
    let r = common::reference();
    let mut state = InstrumentState::new(&r.vehicles[0].instruments);
    let sample = InstrumentSample {
        speed_kmh: 80.,
        fuel_liters: 39.,
        temperature: 130.,
        oil_liters: 5.,
        rpm: 1000.,
        local_ignition: true,
        headlights: 2,
        distance_m: 12345.,
    };
    state.initialize_from_sample(&r.vehicles[0].instruments, sample);
    assert!((state.needle_angles[0]).abs() < 1e-5);
    assert_eq!(state.needle_angles[1], 30.);
    assert_eq!(state.needle_angles[2], 26.);
    let angles = state.needle_angles;
    let digits = state.digit_angles;
    state.update(&r.vehicles[0].instruments, sample, 0.02);
    assert_eq!(angles, state.needle_angles);
    assert_eq!(digits, state.digit_angles);
    assert!(state.digits_lit);
}
#[test]
fn distant_world_and_dormant_car_roundtrip() {
    let r = fixture();
    let mut g = GarageRuntime::new(&r).unwrap();
    if let Some(cover) = g.area.interaction.parts.covering_item(r.wheel_slots[0].rim) {
        common::detach(&mut g, cover);
    }
    let rim = g
        .wheels
        .remove(r.wheel_slots[0].id, &mut g.area.items, &mut g.physics)
        .unwrap();
    let h = g.area.items.get(rim).unwrap().body.unwrap();
    g.physics.rapier_mut().bodies[h].set_linvel(Vector::new(1., 2., 3.), true);
    let mut saved = capture(&g, &r);
    saved.player.pose.position = [10000000000., 1., -10000000000.];
    let loaded = GarageRuntime::from_save(&r, &stamp(&r), &saved).unwrap();
    assert!(!loaded.physics.body_active(loaded.physics.chassis_handle(0)));
    assert_logical_equal(&saved, &capture(&loaded, &r));
}
#[test]
fn seated_relation_survives_large_absolute_coordinates() {
    let r = common::reference();
    let mut g = GarageRuntime::new(&r).unwrap();
    let seat = r.seats.iter().find(|s| s.driver).unwrap();
    let entry = my_summer_slop::vehicle::VehiclePose {
        position: [0.03, 1.17, -0.123],
        rotation: rapier3d::math::Rotation::from_rotation_y(0.2).to_array(),
    };
    assert!(g.area.player.enter(seat.id, entry, &mut g.physics));
    let expected = g.area.player.body_pose(&g.physics);
    let mut saved = capture(&g, &r);
    let mut encoded = serde_json::to_value(&saved).unwrap();
    encoded["player"]["seat"]
        .as_object_mut()
        .unwrap()
        .remove("local_pose");
    assert!(SaveSnapshot::from_json(&serde_json::to_vec(&encoded).unwrap()).is_err());
    let mut invalid = saved.clone();
    invalid.player.seat.as_mut().unwrap().local_pose.position[0] = f32::MAX;
    assert!(invalid.validate(&r, &stamp(&r)).is_err());
    let offset = my_summer_slop::world::MAX_ABSOLUTE_COORDINATE - 100.;
    let vehicle = saved
        .vehicles
        .iter_mut()
        .find(|v| v.instance == saved.player.seat.as_ref().unwrap().vehicle)
        .unwrap();
    vehicle.body.pose.position[0] += offset;
    vehicle.body.pose.position[2] -= offset;
    saved.player.pose.position[0] += offset;
    saved.player.pose.position[2] -= offset;
    let mut loaded = GarageRuntime::from_save(&r, &stamp(&r), &saved).unwrap();
    loaded.physics.reset(0, g.physics.snapshot(0).pose);
    let actual = loaded.area.player.body_pose(&loaded.physics);
    for (a, b) in actual.position.iter().zip(expected.position) {
        assert!(
            (*a - b).abs() < 1e-5,
            "seat relation {actual:?} != {expected:?}"
        );
    }
    for (a, b) in actual.rotation.iter().zip(expected.rotation) {
        assert!((*a - b).abs() < 1e-5);
    }
}
#[test]
fn crouched_player_rebuilds_shape_and_support() {
    let mut r = common::reference();
    r.area.player_spawn.translation = [5., 2., 5.];
    let mut g = GarageRuntime::new(&r).unwrap();
    g.started = true;
    for _ in 0..120 {
        g.step(
            &r,
            FrameInput {
                held: my_summer_slop::simulation::keys::CROUCH,
                ..Default::default()
            },
        );
    }
    assert!(g.area.player.crouched() && g.area.player.grounded());
    let saved = capture(&g, &r);
    let loaded = GarageRuntime::from_save(&r, &stamp(&r), &saved).unwrap();
    assert_logical_equal(&saved, &capture(&loaded, &r));
    assert!(loaded.area.player.grounded());
    let height = |game: &GarageRuntime| {
        let h = game.area.player.body_handle();
        let collider = game.physics.rapier().bodies[h].colliders()[0];
        game.physics.rapier().colliders[collider]
            .shape()
            .compute_local_aabb()
            .extents()
            .y
    };
    assert!((height(&g) - height(&loaded)).abs() < 1e-5);
}

#[test]
#[ignore = "requires installed source export"]
fn installed_fleet_roundtrip_preserves_owners_and_selection() {
    use my_summer_slop::save::{InstanceId, SourceKey};
    let r =
        ReferenceManifest::from_json(&std::fs::read("web/assets/reference/manifest.json").unwrap())
            .unwrap();
    assert_eq!(r.vehicles.len(), 2);
    let mut game = GarageRuntime::new(&r).unwrap();
    game.vehicle = 1;
    for index in 0..2 {
        let vehicle = &mut game.vehicles[index];
        let mut engine = vehicle.engine.capture_save();
        engine.ignition = true;
        engine.rpm = 800. + index as f32 * 300.;
        engine.temperature = 40. + index as f32 * 20.;
        engine.elapsed = 10.;
        engine.last_run_time = 9.9;
        engine.random_start = 0.25 + index as f32 * 0.5;
        vehicle.engine_output = vehicle.engine.restore_save(&engine);
        vehicle.trip_m = 1234.5 + index as f64 * 400.;
        vehicle.handbrake = index == 0;
        game.area
            .items
            .get_mut(r.vehicles[index].engine.item)
            .unwrap()
            .temperature = Some(engine.temperature);
        for _ in 0..=index {
            game.lighting.cycle_headlights(&r.lighting, index);
        }
        let chassis = game.physics.chassis_handle(index);
        game.physics.rapier_mut().bodies[chassis]
            .set_linvel(Vector::new(index as f32 + 1., 0., -2.), true);
    }
    let mut saved = capture(&game, &r);
    assert!(saved.player.seat.is_none());
    assert_eq!(
        saved.progress.selected_vehicle,
        InstanceId::initial(&r.vehicles[1].source)
    );
    for (index, wheel) in saved.attachments.wheels.iter_mut().enumerate() {
        wheel.rotation = index as f32 + 0.25;
        wheel.previous_rotation = wheel.rotation - 0.01;
        wheel.rpm = 30. + index as f32;
    }
    for vehicle in &mut saved.vehicles {
        vehicle.gear = 1;
    }
    let expected = saved.clone();
    saved.vehicles.reverse();
    let mut loaded = GarageRuntime::from_save(&r, &stamp(&r), &saved).unwrap();
    assert_eq!(loaded.vehicle, 1);
    assert_logical_equal(&expected, &capture(&loaded, &r));
    for index in 0..2 {
        assert_eq!(
            loaded.vehicles[index].engine.capture_save(),
            game.vehicles[index].engine.capture_save()
        );
        assert_eq!(loaded.lighting.headlights(index), (index + 1) as u8);
        assert_eq!(loaded.physics.wheel_support_count(index), 4);
    }
    let before = loaded
        .vehicles
        .iter()
        .map(|v| v.engine.capture_save().elapsed)
        .collect::<Vec<_>>();
    loaded.paused = false;
    loaded.step(&r, FrameInput::default());
    for index in 0..2 {
        assert!(loaded.vehicles[index].engine.capture_save().elapsed > before[index]);
    }

    let mut missing = expected.clone();
    missing.vehicles.pop();
    assert!(missing.validate(&r, &stamp(&r)).is_err());
    let mut duplicate = expected.clone();
    duplicate.vehicles[1] = duplicate.vehicles[0].clone();
    assert!(duplicate.validate(&r, &stamp(&r)).is_err());
    let mut unknown = expected.clone();
    unknown.progress.selected_vehicle = InstanceId::Initial {
        source: SourceKey {
            file: "missing".into(),
            path_id: "1".into(),
        },
    };
    assert!(unknown.validate(&r, &stamp(&r)).is_err());

    let mut seated = GarageRuntime::new(&r).unwrap();
    let seat = r.seats.iter().find(|s| s.vehicle == 1 && s.driver).unwrap();
    let anchor = r.anchors.iter().find(|a| a.id == seat.anchor).unwrap();
    assert!(seated.area.player.enter(
        seat.id,
        my_summer_slop::vehicle::VehiclePose {
            position: anchor.transform.translation,
            rotation: anchor.transform.rotation
        },
        &mut seated.physics
    ));
    seated.vehicle = 1;
    let saved_seat = capture(&seated, &r);
    let seated_loaded = GarageRuntime::from_save(&r, &stamp(&r), &saved_seat).unwrap();
    assert_eq!(seated_loaded.area.player.seat_vehicle(), Some(1));
    assert_logical_equal(&saved_seat, &capture(&seated_loaded, &r));
    let mut wrong_seat = saved_seat;
    wrong_seat.player.seat.as_mut().unwrap().vehicle = InstanceId::initial(&r.vehicles[0].source);
    wrong_seat.progress.selected_vehicle = InstanceId::initial(&r.vehicles[0].source);
    assert!(wrong_seat.validate(&r, &stamp(&r)).is_err());
}

#[test]
#[ignore = "requires installed source export"]
fn installed_fleet_dormancy_and_recovery_preserve_other_vehicle() {
    let r =
        ReferenceManifest::from_json(&std::fs::read("web/assets/reference/manifest.json").unwrap())
            .unwrap();
    assert_eq!(r.vehicles.len(), 2);
    let mut game = GarageRuntime::new(&r).unwrap();
    game.vehicle = 1;
    game.vehicles[0].trip_m = 300.;
    game.vehicles[1].trip_m = 700.;
    let mut saved = capture(&game, &r);
    saved.player.pose.position = [10000000000., 1., -10000000000.];
    let mut loaded = GarageRuntime::from_save(&r, &stamp(&r), &saved).unwrap();
    for index in 0..2 {
        assert!(
            !loaded
                .physics
                .body_active(loaded.physics.chassis_handle(index))
        );
    }
    assert_logical_equal(&saved, &capture(&loaded, &r));
    let first = saved
        .vehicles
        .iter()
        .find(|v| v.instance == my_summer_slop::save::InstanceId::initial(&r.vehicles[0].source))
        .unwrap()
        .clone();
    loaded.recover(&r);
    assert_eq!(loaded.vehicle, 1);
    assert!(loaded.physics.body_active(loaded.physics.chassis_handle(1)));
    assert!(!loaded.physics.body_active(loaded.physics.chassis_handle(0)));
    let recovered = capture(&loaded, &r);
    assert_eq!(
        &first,
        recovered
            .vehicles
            .iter()
            .find(|v| v.instance == first.instance)
            .unwrap()
    );
    let again = GarageRuntime::from_save(&r, &stamp(&r), &recovered).unwrap();
    assert_eq!(again.vehicle, 1);
    assert_logical_equal(&recovered, &capture(&again, &r));
}

#[test]
#[ignore = "requires installed source export"]
fn installed_cross_vehicle_wheels_restore_current_owner() {
    let r =
        ReferenceManifest::from_json(&std::fs::read("web/assets/reference/manifest.json").unwrap())
            .unwrap();
    let mut game = GarageRuntime::new(&r).unwrap();
    let slots = [r.vehicles[0].wheel_slots[0], r.vehicles[1].wheel_slots[0]];
    let mut rims = Vec::new();
    for slot in slots {
        let rim = r.wheel_slots.iter().find(|s| s.id == slot).unwrap().rim;
        if let Some(cover) = game.area.interaction.parts.covering_item(rim) {
            game.area
                .interaction
                .parts
                .remove(cover, &mut game.area.items, &mut game.physics)
                .unwrap();
            game.area.interaction.parts.detach_mechanism(
                cover,
                &mut game.area.interaction.mechanisms,
                &mut game.physics,
            );
        }
        rims.push(
            game.wheels
                .remove(slot, &mut game.area.items, &mut game.physics)
                .unwrap(),
        );
    }
    for index in 0..2 {
        game.wheels
            .install(
                rims[1 - index],
                slots[index],
                &mut game.area.items,
                &mut game.physics,
            )
            .unwrap();
    }
    let saved = capture(&game, &r);
    let mut loaded = GarageRuntime::from_save(&r, &stamp(&r), &saved).unwrap();
    assert_logical_equal(&saved, &capture(&loaded, &r));
    for index in 0..2 {
        assert_eq!(
            loaded.area.items.get(rims[1 - index]).unwrap().body,
            Some(loaded.physics.chassis_handle(index))
        );
        assert_eq!(loaded.physics.wheel_support_count(index), 4);
    }
    loaded.paused = false;
    loaded.step(&r, FrameInput::default());
    for index in 0..2 {
        assert_eq!(
            loaded.area.items.get(rims[1 - index]).unwrap().body,
            Some(loaded.physics.chassis_handle(index))
        );
    }
}

#[test]
fn source_conditions_round_trip_and_reject_invalid_candidates() {
    let r = common::reference();
    let g = GarageRuntime::new(&r).unwrap();
    let mut saved = capture(&g, &r);
    assert_eq!(saved.schema_version, 4);
    saved.conditions.surfaces[0].color = [0.2, 0.4, 0.7, 1.];
    saved.conditions.surfaces[0].repair_progress = -0.5;
    let brush_owner = r
        .conditions
        .surfaces
        .iter()
        .find(|s| s.id == r.conditions.brushes[0].own_condition)
        .unwrap();
    let brush_state = saved
        .conditions
        .surfaces
        .iter_mut()
        .find(|s| s.source == my_summer_slop::save::SourceKey::from(&brush_owner.source))
        .unwrap();
    brush_state.color = [0.7, 0.3, 0.1, 1.];
    brush_state.state = 5;
    saved.conditions.tools[0].disabled = true;
    saved.conditions.tools[1].quantity = Some(37.5);
    saved.conditions.washes[0].active = false;
    saved.conditions.washes[0].on = false;
    saved.conditions.washes[0].destroyed = true;
    let loaded = GarageRuntime::from_save(&r, &saved.source, &saved).unwrap();
    assert_eq!(capture(&loaded, &r).conditions, saved.conditions);
    let reject = |mut value: SaveSnapshot, edit: fn(&mut ConditionSnapshot)| {
        edit(&mut value.conditions);
        assert!(GarageRuntime::from_save(&r, &saved.source, &value).is_err());
    };
    reject(saved.clone(), |c| {
        c.surfaces[0].color[0] = f32::NAN;
    });
    reject(saved.clone(), |c| {
        c.surfaces[0].state2 = u32::MAX;
    });
    reject(saved.clone(), |c| {
        c.surfaces[0].state3 = u32::MAX;
    });
    reject(saved.clone(), |c| {
        c.surfaces[0].state = 5;
    });
    reject(saved.clone(), |c| {
        c.surfaces[0].repair_progress = f32::NAN;
    });
    reject(saved.clone(), |c| {
        c.surfaces[0].repair_progress = -1.;
    });
    reject(saved.clone(), |c| {
        c.surfaces[0].repair_progress = 1.;
    });
    let mut wash_boundary = saved.clone();
    wash_boundary.conditions.washes[0].repair_progress = 1.;
    assert!(GarageRuntime::from_save(&r, &saved.source, &wash_boundary).is_ok());
    reject(saved.clone(), |c| {
        c.tools[1].quantity = Some(f32::INFINITY);
    });
    reject(saved.clone(), |c| {
        c.tools[1].quantity = Some(-1.);
    });
    reject(saved.clone(), |c| {
        c.tools[1].quantity = Some(1e10);
    });
    reject(saved.clone(), |c| {
        c.tools[0].disabled = false;
    });
    reject(saved.clone(), |c| {
        c.surfaces[1] = c.surfaces[0].clone();
    });
    reject(saved.clone(), |c| {
        c.surfaces.pop();
    });
    reject(saved.clone(), |c| {
        c.tools.pop();
    });
    reject(saved.clone(), |c| {
        c.washes.pop();
    });
    assert_eq!(capture(&g, &r).conditions.surfaces[0].color, [1.; 4]);
}

#[test]
fn active_wash_without_on_stays_visible_through_save() {
    let mut r = fixture();
    r.conditions.washes[0].active = true;
    r.conditions.washes[0].on = false;
    let id = r.conditions.washes[0].id;
    let game = GarageRuntime::new(&r).unwrap();
    let wash = game.area.conditions.wash(id);
    assert!(wash.visible());
    assert!(!wash.on);
    assert!(!wash.destroyed);
    let saved = capture(&game, &r);
    let loaded = GarageRuntime::from_save(&r, &stamp(&r), &saved).unwrap();
    let restored = loaded.area.conditions.wash(id);
    assert!(restored.visible());
    assert!(!restored.on);
    assert!(!restored.destroyed);
    assert_eq!(
        capture(&loaded, &r).conditions.washes,
        saved.conditions.washes
    );
}

#[test]
fn driving_save_restores_both_gears_and_assists_without_shift_edges() {
    let r = common::reference();
    let mut game = GarageRuntime::new(&r).unwrap();
    game.paused = true;
    let assists = DrivingAssists {
        automatic_clutch: false,
        automatic_gears: false,
    };
    game.set_driving_assists(assists).unwrap();
    let mut saved = capture(&game, &r);
    for (index, v) in r.vehicles.iter().enumerate() {
        saved
            .vehicles
            .iter_mut()
            .find(|record| record.instance == InstanceId::initial(&v.source))
            .unwrap()
            .gear = if index == 0 { 0 } else { 3 };
    }
    let mut loaded = GarageRuntime::from_save(&r, &stamp(&r), &saved).unwrap();
    assert_eq!(loaded.driving_assists, assists);
    assert_eq!(loaded.vehicles[0].drivetrain.gear(), 0);
    assert_eq!(loaded.vehicles[1].drivetrain.gear(), 3);
    loaded.clear_input();
    loaded.paused = false;
    loaded.started = true;
    loaded.step(&r, FrameInput::default());
    assert_eq!(loaded.vehicles[0].drivetrain.gear(), 0);
    assert_eq!(loaded.vehicles[1].drivetrain.gear(), 3);
    assert_eq!(
        GarageRuntime::new(&r).unwrap().driving_assists,
        r.driving.default_assists
    );
}
#[test]
fn invalid_driving_save_keeps_active_scene() {
    let r = common::reference();
    let game = GarageRuntime::new(&r).unwrap();
    let before = capture(&game, &r);
    for case in 0..3 {
        let mut bad = before.clone();
        match case {
            0 => bad.schema_version = 1,
            1 => bad.vehicles[1].gear = r.vehicles[1].gear_ratios.len(),
            _ => {
                bad.driving_assists = DrivingAssists {
                    automatic_clutch: false,
                    automatic_gears: true,
                }
            }
        }
        assert!(GarageRuntime::from_save(&r, &stamp(&r), &bad).is_err());
        assert_eq!(capture(&game, &r), before);
    }
}
