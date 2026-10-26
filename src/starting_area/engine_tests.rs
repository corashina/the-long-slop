use super::*;
use crate::save::{CameraSave, IdentityMap, SourceHash, SourceStamp};

fn installed_source() -> (ReferenceManifest, SourceStamp) {
    let path = "web/assets/reference/manifest.json";
    let bytes = std::fs::read(path).unwrap();
    let reference = ReferenceManifest::from_runtime_json(&bytes).unwrap();
    assert_eq!(reference.build_id, 11140890);
    assert_eq!(reference.vehicles.len(), 1);
    assert_eq!(reference.vehicles[0].engine.source.file, "level1");
    assert_eq!(reference.vehicles[0].engine.source.path_id, 68282);
    let digest = std::process::Command::new("sha256sum")
        .arg(path)
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
            .map(|hash| SourceHash {
                file: hash.file.clone(),
                sha256: hash.sha256.clone(),
            })
            .collect(),
    };
    (reference, stamp)
}

fn step(run: &mut GarageRuntime, reference: &ReferenceManifest, input: FrameInput) {
    run.step_garage_car(reference, 0, input, true, Default::default());
}

fn fluid_entries(quantities: [f32; 6], order: &[u8]) -> Vec<crate::fluids::FluidEntry> {
    order
        .iter()
        .map(|&fluid_type| crate::fluids::FluidEntry {
            fluid_type,
            amount: quantities[fluid_type as usize],
        })
        .collect()
}

#[test]
#[ignore = "requires the installed build 11140890 export"]
fn g04_instrument_sample_keeps_nonfinite_fluid_bits_and_recovers() {
    use crate::fluids::FluidEntry;
    use crate::simulation::instruments::{InstrumentState, needle_angle};

    let (reference, _) = installed_source();
    let mut run = GarageRuntime::new_default(&reference).unwrap();
    run.garage_car_mut(0).ignition.ignition = true;
    let vehicle = &reference.vehicles[0];
    let instruments = &vehicle.instruments;
    let mut state = InstrumentState::new(instruments);
    let source_entries = [
        FluidEntry {
            fluid_type: 3,
            amount: -0.25,
        },
        FluidEntry {
            fluid_type: 0,
            amount: f32::from_bits(0x7fc1_2345),
        },
    ];
    for id in [vehicle.engine.fuel_tank, vehicle.engine.oil_tank] {
        run.area
            .items
            .get_mut(id)
            .unwrap()
            .fluid
            .as_mut()
            .unwrap()
            .restore_entries(&source_entries);
    }
    let sample = run.instrument_sample(&reference, 0);
    assert!(sample.local_ignition);
    assert_eq!(sample.fuel_liters, 0.);
    assert_eq!(sample.oil_liters, 0.);
    state.initialize_from_sample(instruments, sample);
    state.update(instruments, sample, FIXED_DT);
    assert!(state.needle_angles.iter().all(|angle| angle.is_finite()));
    for id in [vehicle.engine.fuel_tank, vehicle.engine.oil_tank] {
        let entries: Vec<_> = run
            .area
            .items
            .get(id)
            .unwrap()
            .fluid
            .as_ref()
            .unwrap()
            .entries()
            .map(|entry| (entry.fluid_type, entry.amount.to_bits()))
            .collect();
        assert_eq!(
            entries,
            source_entries
                .iter()
                .map(|entry| (entry.fluid_type, entry.amount.to_bits()))
                .collect::<Vec<_>>()
        );
        run.area
            .items
            .get_mut(id)
            .unwrap()
            .fluid
            .as_mut()
            .unwrap()
            .restore_entries(&[FluidEntry {
                fluid_type: 0,
                amount: 10.,
            }]);
    }
    let sample = run.instrument_sample(&reference, 0);
    assert_eq!(sample.fuel_liters, 10.);
    assert_eq!(sample.oil_liters, 10.);
    let fuel_needle = instruments
        .needles
        .iter()
        .position(|needle| needle.signal == "fuel")
        .unwrap();
    let empty_angle = state.needle_angles[fuel_needle];
    for _ in 0..600 {
        state.update(instruments, sample, FIXED_DT);
    }
    assert!(state.needle_angles.iter().all(|angle| angle.is_finite()));
    assert_ne!(state.needle_angles[fuel_needle], empty_angle);
    assert!(
        (state.needle_angles[fuel_needle]
            - needle_angle(&instruments.needles[fuel_needle], sample.fuel_liters))
        .abs()
            < 0.01
    );
}

#[test]
#[ignore = "requires the installed build 11140890 export"]
fn ignition_and_attachment() {
    let (reference, stamp) = installed_source();
    let source = &reference.vehicles[0].engine;
    let mut run = GarageRuntime::new_default(&reference).unwrap();
    let held = FrameInput {
        held: keys::IGNITION,
        ..Default::default()
    };
    let tap = FrameInput {
        pressed: keys::IGNITION,
        released: keys::IGNITION,
        ..Default::default()
    };
    step(&mut run, &reference, tap);
    assert!(run.garage_car(0).ignition.ignition);
    step(&mut run, &reference, tap);
    assert!(!run.garage_car(0).ignition.ignition);
    for _ in 0..(source.starter_delay_seconds / FIXED_DT).ceil() as usize + 2 {
        step(&mut run, &reference, held);
    }
    assert!(run.garage_car(0).ignition.starter);
    step(
        &mut run,
        &reference,
        FrameInput {
            released: keys::IGNITION,
            ..Default::default()
        },
    );
    assert!(run.garage_car(0).ignition.starter);
    step(&mut run, &reference, FrameInput::default());
    assert!(!run.garage_car(0).ignition.starter);
    run.clear_input();
    assert!(run.garage_car(0).ignition.ignition);
    assert!(
        !run.engine_part(reference.vehicles[0].engine.item)
            .state
            .output()
            .starter
    );
    // Seed a running state. Sampling and run-decision order belong to Task 3.
    let mut mechanical = run
        .engine_part(reference.vehicles[0].engine.item)
        .state
        .capture_save();
    run.garage_car_mut(0).ignition.ignition = true;
    mechanical.rpm = 2000.;
    mechanical.elapsed = 1.;
    mechanical.last_run_time = 1.;
    mechanical.supplied_fuel = 0.4;
    mechanical.random_start = 0.;
    run.engine_part_mut(reference.vehicles[0].engine.item)
        .state
        .restore_save(&mechanical)
        .unwrap();
    step(&mut run, &reference, FrameInput::default());
    assert!(
        run.engine_part(reference.vehicles[0].engine.item)
            .state
            .output()
            .running
    );
    let engine_slot = reference
        .part_slots
        .iter()
        .find(|slot| slot.part == Some(source.item))
        .unwrap()
        .id;
    let engine_before = run
        .engine_part(reference.vehicles[0].engine.item)
        .state
        .capture_save();
    let item_before = run.area.items.get(source.item).unwrap().clone();
    let fuel_before = run.area.items.get(source.fuel_tank).unwrap().fluid.clone();
    let oil_before = run.area.items.get(source.oil_tank).unwrap().fluid.clone();
    let coolant_before = run
        .area
        .items
        .get(source.coolant_tank)
        .unwrap()
        .fluid
        .clone();
    run.area
        .interaction
        .parts
        .can_remove(source.item, &run.area.player, &run.area.items)
        .unwrap();
    run.area
        .interaction
        .parts
        .remove(source.item, &mut run.area.items, &mut run.physics)
        .unwrap();
    run.area.interaction.parts.detach_mechanism(
        source.item,
        &mut run.area.interaction.mechanisms,
        &mut run.physics,
    );
    run.area
        .interaction
        .sync_parts(&mut run.area.items, &mut run.physics)
        .unwrap();
    let item = run.area.items.get(source.item).unwrap();
    assert_eq!(item.id, item_before.id);
    assert_eq!(item.temperature, item_before.temperature);
    assert_eq!(item.fluid, item_before.fluid);
    assert_eq!(
        run.engine_part(reference.vehicles[0].engine.item)
            .state
            .capture_save(),
        engine_before
    );
    assert_eq!(
        run.area.items.get(source.fuel_tank).unwrap().fluid,
        fuel_before
    );
    assert_eq!(
        run.area.items.get(source.oil_tank).unwrap().fluid,
        oil_before
    );
    assert_eq!(
        run.area.items.get(source.coolant_tank).unwrap().fluid,
        coolant_before
    );
    step(&mut run, &reference, FrameInput::default());
    let output = run
        .engine_part(reference.vehicles[0].engine.item)
        .state
        .output();
    assert!(output.running, "detached engine retains run grace");
    assert!(output.rpm > 0., "detached engine retains residual RPM");
    assert_eq!(output.torque_nm, 0.);
    assert_eq!(output.brake_torque_nm, 0.);
    assert_eq!(output.oil_penalty, engine_before.oil_penalty);
    assert_eq!(
        run.area.items.get(source.oil_tank).unwrap().fluid,
        oil_before
    );
    assert_eq!(
        run.area.items.get(source.coolant_tank).unwrap().fluid,
        coolant_before
    );
    assert!(
        run.area
            .items
            .get(source.fuel_tank)
            .unwrap()
            .fluid
            .as_ref()
            .unwrap()
            .total()
            < fuel_before.as_ref().unwrap().total()
    );
    run.area
        .items
        .get_mut(source.oil_tank)
        .unwrap()
        .fluid
        .as_mut()
        .unwrap()
        .restore_entries(&[]);
    step(&mut run, &reference, tap);
    assert_eq!(
        run.engine_part(reference.vehicles[0].engine.item)
            .state
            .output()
            .oil_penalty,
        engine_before.oil_penalty
    );
    assert!(!run.garage_car(0).ignition.ignition);
    step(&mut run, &reference, tap);
    assert!(run.garage_car(0).ignition.ignition);
    let sample = run.instrument_sample(&reference, 0);
    assert!(sample.local_ignition);
    assert_eq!(sample.rpm, 0.);
    assert_eq!(sample.temperature, 0.);
    assert_eq!(
        run.engine_part(reference.vehicles[0].engine.item)
            .state
            .output()
            .torque_nm,
        0.
    );
    let saved = run
        .capture_save(
            &reference,
            &stamp,
            CameraSave::FirstPerson,
            "2026-10-20T12:00:00Z",
        )
        .unwrap();
    assert!(saved.garage_vehicle(0).ignition);
    let restored = GarageRuntime::from_save(
        &reference,
        &stamp,
        &saved,
        crate::world::GameMode::default(),
        reference.world.draw_distance.default,
    )
    .unwrap();
    assert!(restored.paused);
    assert!(restored.garage_car(0).ignition.ignition);
    assert!(!restored.garage_car(0).ignition.starter);
    assert!(
        !restored
            .engine_part(reference.vehicles[0].engine.item)
            .state
            .output()
            .starter
    );
    assert_eq!(
        restored
            .engine_part(reference.vehicles[0].engine.item)
            .state
            .capture_save(),
        *saved.garage_engine(&reference, 0)
    );
    let ids = IdentityMap::new(&reference).unwrap();
    let restored_items = restored
        .area
        .items
        .capture_save(
            &ids,
            &restored.stream.origin,
            &restored.physics,
            &restored.area.interaction,
            &reference,
        )
        .unwrap();
    for item_id in [
        source.item,
        source.fuel_tank,
        source.oil_tank,
        source.coolant_tank,
    ] {
        let expected = saved
            .items
            .iter()
            .find(|item| ids.item(&item.instance).unwrap() == item_id)
            .unwrap();
        let actual = restored_items
            .iter()
            .find(|item| item.instance == expected.instance)
            .unwrap();
        assert_eq!(actual.fluids, expected.fluids);
        assert_eq!(actual.temperature, expected.temperature);
        assert_eq!(actual.location, expected.location);
    }
    // A loose engine can stay active after its chassis leaves the active world.
    let chassis = run
        .physics
        .chassis_handle(&crate::vehicle::garage_vehicle_key(0));
    let loose = run.area.items.get(source.item).unwrap().body.unwrap();
    assert_ne!(loose, chassis);
    let before = run
        .engine_part(reference.vehicles[0].engine.item)
        .state
        .capture_save();
    let chassis_velocity = run.physics.rapier().bodies[chassis].linvel();
    run.physics.inactive_bodies.insert(chassis);
    step(&mut run, &reference, FrameInput::default());
    let after = run
        .engine_part(reference.vehicles[0].engine.item)
        .state
        .capture_save();
    assert!(after.elapsed > before.elapsed);
    assert!(after.rpm > 0.);
    assert_eq!(
        run.physics.rapier().bodies[chassis].linvel(),
        chassis_velocity
    );
    run.physics.inactive_bodies.insert(loose);
    step(&mut run, &reference, FrameInput::default());
    assert_eq!(
        run.engine_part(reference.vehicles[0].engine.item)
            .state
            .capture_save(),
        after
    );
    run.physics.inactive_bodies.remove(&loose);
    for _ in 0..20 {
        step(&mut run, &reference, FrameInput::default());
    }
    assert!(
        !run.engine_part(reference.vehicles[0].engine.item)
            .state
            .output()
            .running
    );
    assert!(
        run.engine_part(reference.vehicles[0].engine.item)
            .state
            .output()
            .rpm
            < output.rpm
    );
    assert_eq!(
        run.engine_part(reference.vehicles[0].engine.item)
            .state
            .capture_save()
            .supplied_fuel,
        0.
    );
    // Chassis commands do not start a detached engine.
    run.physics.inactive_bodies.remove(&chassis);
    for _ in 0..(source.starter_delay_seconds / FIXED_DT).ceil() as usize + 2 {
        step(&mut run, &reference, held);
    }
    assert!(run.garage_car(0).ignition.starter);
    assert!(
        !run.engine_part(reference.vehicles[0].engine.item)
            .state
            .output()
            .starter
    );
    assert_eq!(
        run.engine_part(reference.vehicles[0].engine.item)
            .state
            .output()
            .torque_nm,
        0.
    );
    run.clear_input();
    let engine_before_install = run
        .engine_part(reference.vehicles[0].engine.item)
        .state
        .capture_save();
    let item_before_install = run.area.items.get(source.item).unwrap().clone();
    run.area
        .interaction
        .parts
        .install(
            source.item,
            engine_slot,
            &mut run.area.items,
            &mut run.physics,
        )
        .unwrap();
    run.area
        .interaction
        .parts
        .mount_mechanism(source.item, &mut run.area.interaction.mechanisms);
    run.area
        .interaction
        .sync_parts(&mut run.area.items, &mut run.physics)
        .unwrap();
    assert_eq!(
        run.engine_part(reference.vehicles[0].engine.item)
            .state
            .capture_save(),
        engine_before_install
    );
    let item = run.area.items.get(source.item).unwrap();
    assert_eq!(item.id, item_before_install.id);
    assert_eq!(item.temperature, item_before_install.temperature);
    assert_eq!(item.fluid, item_before_install.fluid);
    assert!(run.area.items.mounted(source.item));
}

#[test]
#[ignore = "requires the installed build 11140890 export"]
fn active_chassis_preserves_inactive_detached_engine() {
    let (reference, _) = installed_source();
    let source = &reference.vehicles[0].engine;
    let mut run = GarageRuntime::new_default(&reference).unwrap();
    let mut mechanical = run
        .engine_part(reference.vehicles[0].engine.item)
        .state
        .capture_save();
    mechanical.rpm = 2000.;
    mechanical.temperature = 80.;
    mechanical.elapsed = 1.;
    mechanical.last_run_time = 1.;
    mechanical.supplied_fuel = 0.4;
    mechanical.random_start = 0.;
    run.garage_car_mut(0).ignition.ignition = true;
    run.engine_part_mut(reference.vehicles[0].engine.item)
        .state
        .restore_save(&mechanical)
        .unwrap();
    step(&mut run, &reference, FrameInput::default());
    run.area
        .interaction
        .parts
        .remove(source.item, &mut run.area.items, &mut run.physics)
        .unwrap();
    run.area.interaction.parts.detach_mechanism(
        source.item,
        &mut run.area.interaction.mechanisms,
        &mut run.physics,
    );
    run.area
        .interaction
        .sync_parts(&mut run.area.items, &mut run.physics)
        .unwrap();
    step(&mut run, &reference, FrameInput::default());
    assert!(
        run.engine_part(reference.vehicles[0].engine.item)
            .state
            .output()
            .running
    );
    let loose = run.area.items.get(source.item).unwrap().body.unwrap();
    let chassis = run
        .physics
        .chassis_handle(&crate::vehicle::garage_vehicle_key(0));
    assert_ne!(loose, chassis);
    run.physics.edit_body(loose, |body| body.set_enabled(false));
    run.physics.inactive_bodies.insert(loose);
    assert!(run.physics.body_active(chassis));
    assert!(!run.physics.body_active(loose));
    let before = run
        .engine_part(reference.vehicles[0].engine.item)
        .state
        .capture_save();
    let contents = [
        source.item,
        source.fuel_tank,
        source.oil_tank,
        source.coolant_tank,
    ]
    .map(|id| {
        let item = run.area.items.get(id).unwrap();
        (id, item.temperature, item.fluid.clone())
    });
    let tap = FrameInput {
        pressed: keys::IGNITION,
        released: keys::IGNITION,
        ..Default::default()
    };
    step(&mut run, &reference, tap);
    assert!(!run.garage_car(0).ignition.ignition);
    assert!(!run.instrument_sample(&reference, 0).local_ignition);
    run.physics.edit_body(chassis, |body| body.sleep());
    run.step_garage_car(
        &reference,
        0,
        tap,
        true,
        crate::simulation::input::DrivingControls {
            brake: 1.,
            ..Default::default()
        },
    );
    assert!(run.garage_car(0).ignition.ignition);
    assert!(run.instrument_sample(&reference, 0).local_ignition);
    assert!(!run.physics.rapier().bodies[chassis].is_sleeping());
    let brake = reference
        .lighting
        .controllers
        .iter()
        .find(|control| {
            control.kind == crate::reference::LightControllerKind::Rear
                && control.vehicle == Some(0)
        })
        .unwrap()
        .states
        .iter()
        .find(|lamp| lamp.signal == crate::reference::LampSignal::Brake)
        .unwrap();
    assert!(!brake.lights.is_empty());
    for id in &brake.lights {
        assert!(
            run.lighting
                .light_outputs()
                .iter()
                .find(|light| light.id == *id)
                .unwrap()
                .enabled
        );
    }
    for _ in 0..(source.starter_delay_seconds / FIXED_DT).ceil() as usize + 2 {
        step(
            &mut run,
            &reference,
            FrameInput {
                held: keys::IGNITION,
                ..Default::default()
            },
        );
    }
    assert!(run.garage_car(0).ignition.starter);
    assert!(
        !run.engine_part(reference.vehicles[0].engine.item)
            .state
            .output()
            .starter
    );
    assert_eq!(
        run.engine_part(reference.vehicles[0].engine.item)
            .state
            .output()
            .torque_nm,
        0.
    );
    assert_eq!(
        run.engine_part(reference.vehicles[0].engine.item)
            .state
            .output()
            .brake_torque_nm,
        0.
    );
    assert_eq!(
        run.engine_part(reference.vehicles[0].engine.item)
            .state
            .capture_save(),
        before
    );
    for (id, temperature, fluid) in contents {
        let item = run.area.items.get(id).unwrap();
        assert_eq!(item.temperature, temperature);
        assert_eq!(item.fluid, fluid);
    }
}

#[test]
#[ignore = "requires the installed build 11140890 export"]
fn interval_draws_preserve_attempts_and_cancellation() {
    let (mut reference, _) = installed_source();
    // Rule test: keep combustion stopped so one held press consumes many draws.
    reference.vehicles[0].engine.start_chance = 0.;
    let mut run = GarageRuntime::new_default(&reference).unwrap();
    let held = FrameInput {
        held: keys::IGNITION,
        ..Default::default()
    };
    step(&mut run, &reference, held);
    let attempts = run.attempts;
    for _ in 0..180 {
        step(&mut run, &reference, held);
    }
    let state = run
        .engine_part(reference.vehicles[0].engine.item)
        .state
        .capture_save();
    assert!(state.start_draw.parse::<u64>().unwrap() >= 2);
    assert_eq!(run.attempts, attempts);
    assert!(
        run.engine_part(reference.vehicles[0].engine.item)
            .state
            .output()
            .starter
    );
    run.clear_input();
    assert!(
        !run.engine_part(reference.vehicles[0].engine.item)
            .state
            .output()
            .starter
    );
    assert_eq!(
        run.engine_part(reference.vehicles[0].engine.item)
            .state
            .capture_save(),
        state
    );
    assert_eq!(run.attempts, attempts);
}

#[test]
#[ignore = "requires the installed build 11140890 export"]
fn garage_engine_remove_restore_continue() {
    let (reference, stamp) = installed_source();
    let source = &reference.vehicles[0].engine;
    let ids = IdentityMap::new(&reference).unwrap();
    let engine_slot = reference
        .part_slots
        .iter()
        .find(|slot| slot.part == Some(source.item))
        .unwrap()
        .id;
    let held = FrameInput {
        held: keys::IGNITION,
        ..Default::default()
    };
    let tap = FrameInput {
        pressed: keys::IGNITION,
        released: keys::IGNITION,
        ..Default::default()
    };
    let contents = |run: &GarageRuntime| {
        run.area
            .items
            .capture_save(
                &ids,
                &run.stream.origin,
                &run.physics,
                &run.area.interaction,
                &reference,
            )
            .unwrap()
            .into_iter()
            .filter(|item| {
                [
                    source.item,
                    source.fuel_tank,
                    source.oil_tank,
                    source.coolant_tank,
                ]
                .contains(&ids.item(&item.instance).unwrap())
            })
            .collect::<Vec<_>>()
    };
    let compare = |run: &GarageRuntime, loaded: &GarageRuntime| {
        assert_eq!(
            run.engine_part(reference.vehicles[0].engine.item)
                .state
                .capture_save(),
            loaded
                .engine_part(reference.vehicles[0].engine.item)
                .state
                .capture_save()
        );
        assert_eq!(
            run.garage_car(0).ignition.ignition,
            loaded.garage_car(0).ignition.ignition
        );
        assert_eq!(
            run.garage_car(0).ignition.starter,
            loaded.garage_car(0).ignition.starter
        );
        assert_eq!(
            run.garage_car(0).drivetrain.gear(),
            loaded.garage_car(0).drivetrain.gear()
        );
        assert_eq!(run.garage_car(0).handbrake, loaded.garage_car(0).handbrake);
        assert_eq!(run.garage_car(0).trip_m, loaded.garage_car(0).trip_m);
        let expected = contents(run);
        let actual = contents(loaded);
        for item in expected {
            let other = actual
                .iter()
                .find(|other| other.instance == item.instance)
                .unwrap();
            assert_eq!(other.fluids, item.fluids);
            assert_eq!(other.temperature, item.temperature);
            assert_eq!(other.location, item.location);
        }
    };
    let install = |run: &mut GarageRuntime| {
        let before = run
            .engine_part(reference.vehicles[0].engine.item)
            .state
            .capture_save();
        let items = contents(run);
        run.area
            .interaction
            .parts
            .install(
                source.item,
                engine_slot,
                &mut run.area.items,
                &mut run.physics,
            )
            .unwrap();
        run.area
            .interaction
            .parts
            .mount_mechanism(source.item, &mut run.area.interaction.mechanisms);
        run.area
            .interaction
            .sync_parts(&mut run.area.items, &mut run.physics)
            .unwrap();
        assert!(run.area.items.mounted(source.item));
        assert_eq!(
            run.engine_part(reference.vehicles[0].engine.item)
                .state
                .capture_save(),
            before
        );
        for item in items {
            let after = contents(run)
                .into_iter()
                .find(|other| other.instance == item.instance)
                .unwrap();
            assert_eq!(after.fluids, item.fluids);
            assert_eq!(after.temperature, item.temperature);
        }
    };
    let mut run = GarageRuntime::new_default(&reference).unwrap();
    // Empty-fuel cranking consumes real interval draws before mixed fuel is added.
    run.area
        .items
        .get_mut(source.fuel_tank)
        .unwrap()
        .fluid
        .as_mut()
        .unwrap()
        .restore_entries(&[]);
    for _ in 0..600 {
        step(&mut run, &reference, held);
        if run
            .engine_part(reference.vehicles[0].engine.item)
            .state
            .capture_save()
            .start_draw
            .parse::<u64>()
            .unwrap()
            >= 2
        {
            break;
        }
    }
    assert!(
        !run.engine_part(reference.vehicles[0].engine.item)
            .state
            .output()
            .running
    );
    assert!(
        run.engine_part(reference.vehicles[0].engine.item)
            .state
            .capture_save()
            .start_draw
            .parse::<u64>()
            .unwrap()
            >= 2
    );
    // Mixed contents use deliberately nonnumeric order and retain a zero entry.
    for (item, quantities, order) in [
        (source.fuel_tank, [5., 0., 0., 0.02, 0., 0.], [3, 0, 5]),
        (source.oil_tank, [0., 3., 0., 0.02, 0., 0.], [3, 1, 5]),
        (source.coolant_tank, [0., 0., 0., 3., 0.02, 0.], [4, 3, 5]),
    ] {
        run.area
            .items
            .get_mut(item)
            .unwrap()
            .fluid
            .as_mut()
            .unwrap()
            .restore_entries(&fluid_entries(quantities, &order));
    }
    // Start through the production fixed vehicle step, with installed source rates.
    for _ in 0..600 {
        step(&mut run, &reference, held);
        if run
            .engine_part(reference.vehicles[0].engine.item)
            .state
            .output()
            .running
        {
            break;
        }
    }
    assert!(
        run.engine_part(reference.vehicles[0].engine.item)
            .state
            .output()
            .running
    );
    assert!(run.garage_car(0).ignition.starter);
    let before_remove = run
        .engine_part(reference.vehicles[0].engine.item)
        .state
        .capture_save();
    assert!(before_remove.elapsed > 0. && before_remove.start_draw.parse::<u64>().unwrap() > 0);
    let before_items = contents(&run);
    run.area
        .interaction
        .parts
        .can_remove(source.item, &run.area.player, &run.area.items)
        .unwrap();
    run.area
        .interaction
        .parts
        .remove(source.item, &mut run.area.items, &mut run.physics)
        .unwrap();
    run.area.interaction.parts.detach_mechanism(
        source.item,
        &mut run.area.interaction.mechanisms,
        &mut run.physics,
    );
    run.area
        .interaction
        .sync_parts(&mut run.area.items, &mut run.physics)
        .unwrap();
    assert!(!run.area.items.mounted(source.item));
    assert_eq!(
        run.engine_part(reference.vehicles[0].engine.item)
            .state
            .capture_save(),
        before_remove
    );
    for item in before_items {
        let detached = contents(&run)
            .into_iter()
            .find(|other| other.instance == item.instance)
            .unwrap();
        assert_eq!(detached.fluids, item.fluids);
        assert_eq!(detached.temperature, item.temperature);
    }
    step(&mut run, &reference, held);
    assert!(
        run.engine_part(reference.vehicles[0].engine.item)
            .state
            .output()
            .running
    );
    assert_eq!(
        run.engine_part(reference.vehicles[0].engine.item)
            .state
            .output()
            .torque_nm,
        0.
    );
    assert_eq!(
        run.engine_part(reference.vehicles[0].engine.item)
            .state
            .output()
            .brake_torque_nm,
        0.
    );
    assert!(
        !run.engine_part(reference.vehicles[0].engine.item)
            .state
            .output()
            .starter
    );
    assert!(run.garage_car(0).ignition.starter);
    assert!(
        run.engine_part(reference.vehicles[0].engine.item)
            .state
            .capture_save()
            .elapsed
            > before_remove.elapsed
    );
    let saved = run
        .capture_save(
            &reference,
            &stamp,
            CameraSave::FirstPerson,
            "2026-10-20T12:00:00Z",
        )
        .unwrap();
    let mut loaded = GarageRuntime::from_save(
        &reference,
        &stamp,
        &saved,
        crate::world::GameMode::default(),
        reference.world.draw_distance.default,
    )
    .unwrap();
    assert!(loaded.paused);
    assert!(loaded.restore_pending());
    assert!(
        loaded
            .capture_save(
                &reference,
                &stamp,
                CameraSave::FirstPerson,
                "2026-10-20T12:00:00Z"
            )
            .is_err()
    );
    // Full capture is blocked until world restore finishes. Read restored owners directly.
    assert_eq!(
        loaded.garage_car(0).ignition.ignition,
        saved.garage_vehicle(0).ignition
    );
    let restored_engine = loaded
        .engine_part(reference.vehicles[0].engine.item)
        .state
        .capture_save();
    assert_eq!(
        restored_engine.start_draw,
        saved.garage_engine(&reference, 0).start_draw
    );
    assert_eq!(
        restored_engine.temperature,
        saved.garage_engine(&reference, 0).temperature
    );
    assert_eq!(restored_engine, *saved.garage_engine(&reference, 0));
    assert!(!loaded.garage_car(0).ignition.starter);
    assert!(
        !loaded
            .engine_part(reference.vehicles[0].engine.item)
            .state
            .output()
            .starter
    );
    let restored_items = contents(&loaded);
    for item in &restored_items {
        let original = saved
            .items
            .iter()
            .find(|original| original.instance == item.instance)
            .unwrap();
        let mut expected_fluids = original.fluids.clone();
        if let Some(entries) = &mut expected_fluids {
            if let Some(index) = entries.iter().position(|entry| entry.amount <= 0.) {
                entries.remove(index);
            }
        }
        assert_eq!(item.fluids, expected_fluids);
        assert_eq!(item.temperature, original.temperature);
        assert_eq!(item.location, original.location);
    }
    let saved_fuel = saved
        .items
        .iter()
        .find(|item| ids.item(&item.instance).unwrap() == source.fuel_tank)
        .unwrap();
    let restored_fuel = restored_items
        .iter()
        .find(|item| item.instance == saved_fuel.instance)
        .unwrap();
    assert_eq!(restored_fuel.fluids, saved_fuel.fluids);
    assert_eq!(
        saved_fuel
            .fluids
            .as_ref()
            .unwrap()
            .iter()
            .map(|entry| entry.fluid_type)
            .collect::<Vec<_>>(),
        [3, 0]
    );
    let loaded_chassis = loaded
        .physics
        .chassis_handle(&crate::vehicle::garage_vehicle_key(0));
    let loaded_engine = loaded.area.items.get(source.item).unwrap().body.unwrap();
    assert!(!loaded.physics.body_active(loaded_chassis));
    assert!(!loaded.physics.body_active(loaded_engine));
    // Reuse native renderer receipts, then complete the normal restore lifecycle.
    // These receipts do not execute browser rendering or asset uploads.
    crate::world::stream::tests::prepare_ready(&mut loaded, &reference);
    assert!(loaded.advance_saved_restore(&reference).unwrap());
    loaded.sync_world_activity(&reference).unwrap();
    assert!(!loaded.restore_pending());
    assert!(loaded.physics.body_active(loaded_chassis));
    assert!(loaded.physics.body_active(loaded_engine));
    let restored = loaded
        .capture_save(
            &reference,
            &stamp,
            CameraSave::FirstPerson,
            "2026-10-20T12:00:00Z",
        )
        .unwrap();
    assert_eq!(
        restored.garage_vehicle(0).ignition,
        saved.garage_vehicle(0).ignition
    );
    assert_eq!(
        restored.garage_engine(&reference, 0),
        saved.garage_engine(&reference, 0)
    );
    let restored_fuel = restored
        .items
        .iter()
        .find(|item| item.instance == saved_fuel.instance)
        .unwrap();
    assert_eq!(restored_fuel.fluids, saved_fuel.fluids);
    // Apply the same logical load cleanup once before comparing continued runs.
    run.area.items.cleanup_loaded_fluids();
    run.clear_input();
    compare(&run, &loaded);
    for expected in [false, true] {
        step(&mut run, &reference, tap);
        step(&mut loaded, &reference, tap);
        assert_eq!(run.garage_car(0).ignition.ignition, expected);
        assert_eq!(loaded.instrument_sample(&reference, 0).rpm, 0.);
        assert_eq!(
            loaded.instrument_sample(&reference, 0).local_ignition,
            expected
        );
        compare(&run, &loaded);
    }
    // Continue both runs through the same fixed vehicle boundary after restoration.
    for _ in 0..600 {
        step(&mut run, &reference, FrameInput::default());
        step(&mut loaded, &reference, FrameInput::default());
        compare(&run, &loaded);
        for state in [&run, &loaded] {
            assert_eq!(
                state
                    .engine_part(reference.vehicles[0].engine.item)
                    .state
                    .output()
                    .torque_nm,
                0.
            );
            assert_eq!(
                state
                    .engine_part(reference.vehicles[0].engine.item)
                    .state
                    .output()
                    .brake_torque_nm,
                0.
            );
        }
        let state = run
            .engine_part(reference.vehicles[0].engine.item)
            .state
            .capture_save();
        if !run
            .engine_part(reference.vehicles[0].engine.item)
            .state
            .output()
            .running
            && run
                .engine_part(reference.vehicles[0].engine.item)
                .state
                .output()
                .rpm
                <= source.shutdown_rpm
            && state.elapsed - state.last_start_chance_time
                > source.start_chance_interval_seconds * source.engine_start_time_modifier
        {
            break;
        }
    }
    assert!(
        !run.engine_part(reference.vehicles[0].engine.item)
            .state
            .output()
            .running
    );
    assert!(
        run.engine_part(reference.vehicles[0].engine.item)
            .state
            .output()
            .rpm
            <= source.shutdown_rpm
    );
    assert_eq!(
        run.engine_part(reference.vehicles[0].engine.item)
            .state
            .capture_save()
            .supplied_fuel,
        0.
    );
    let fuel_after = run
        .area
        .items
        .get(source.fuel_tank)
        .unwrap()
        .fluid
        .as_ref()
        .unwrap();
    assert!(
        fuel_after.total()
            < saved_fuel
                .fluids
                .as_ref()
                .unwrap()
                .iter()
                .map(|entry| entry.amount)
                .sum::<f32>()
    );
    for item in [source.oil_tank, source.coolant_tank] {
        let original = restored_items
            .iter()
            .find(|entry| ids.item(&entry.instance).unwrap() == item)
            .unwrap();
        let current = contents(&run)
            .into_iter()
            .find(|entry| entry.instance == original.instance)
            .unwrap();
        assert_eq!(current.fluids, original.fluids);
    }
    install(&mut run);
    install(&mut loaded);
    compare(&run, &loaded);
    for _ in 0..600 {
        step(&mut run, &reference, held);
        step(&mut loaded, &reference, held);
        compare(&run, &loaded);
        if run
            .engine_part(reference.vehicles[0].engine.item)
            .state
            .output()
            .running
        {
            break;
        }
    }
    assert!(
        run.engine_part(reference.vehicles[0].engine.item)
            .state
            .output()
            .running
    );
    assert!(
        loaded
            .engine_part(reference.vehicles[0].engine.item)
            .state
            .output()
            .running
    );
    let final_engine = loaded
        .engine_part(reference.vehicles[0].engine.item)
        .state
        .capture_save();
    assert!(
        final_engine.start_draw.parse::<u64>().unwrap()
            > saved
                .garage_engine(&reference, 0)
                .start_draw
                .parse::<u64>()
                .unwrap()
    );
    assert!(!loaded.restore_pending());
    assert!(loaded.paused);
}
