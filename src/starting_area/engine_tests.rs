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
    run.step_vehicle(reference, 0, input, true, Default::default());
}

#[test]
#[ignore = "requires the installed build 11140890 export"]
fn ignition_and_attachment() {
    let (reference, stamp) = installed_source();
    let source = &reference.vehicles[0].engine;
    let mut run = GarageRuntime::new(&reference).unwrap();
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
    assert!(run.vehicles[0].ignition.ignition);
    step(&mut run, &reference, tap);
    assert!(!run.vehicles[0].ignition.ignition);
    for _ in 0..(source.starter_delay_seconds / FIXED_DT).ceil() as usize + 2 {
        step(&mut run, &reference, held);
    }
    assert!(run.vehicles[0].ignition.starter);
    step(
        &mut run,
        &reference,
        FrameInput {
            released: keys::IGNITION,
            ..Default::default()
        },
    );
    assert!(run.vehicles[0].ignition.starter);
    step(&mut run, &reference, FrameInput::default());
    assert!(!run.vehicles[0].ignition.starter);
    run.clear_input();
    assert!(run.vehicles[0].ignition.ignition);
    assert!(!run.vehicles[0].engine_output.starter);
    // Seed a running state. Sampling and run-decision order belong to Task 3.
    let mut mechanical = run.vehicles[0].engine.capture_save();
    run.vehicles[0].ignition.ignition = true;
    mechanical.rpm = 2000.;
    mechanical.elapsed = 1.;
    mechanical.last_run_time = 1.;
    mechanical.supplied_fuel = 0.4;
    mechanical.random_start = 0.;
    run.vehicles[0].engine_output = run.vehicles[0].engine.restore_save(&mechanical);
    step(&mut run, &reference, FrameInput::default());
    assert!(run.vehicles[0].engine_output.running);
    let engine_slot = reference
        .part_slots
        .iter()
        .find(|slot| slot.part == Some(source.item))
        .unwrap()
        .id;
    let engine_before = run.vehicles[0].engine.capture_save();
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
    assert_eq!(run.vehicles[0].engine.capture_save(), engine_before);
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
    let output = run.vehicles[0].engine_output;
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
            .mixture
            .total()
            < fuel_before.as_ref().unwrap().mixture.total()
    );
    run.area
        .items
        .get_mut(source.oil_tank)
        .unwrap()
        .fluid
        .as_mut()
        .unwrap()
        .mixture
        .quantities_liters = [0.; 6];
    step(&mut run, &reference, tap);
    assert_eq!(
        run.vehicles[0].engine_output.oil_penalty,
        engine_before.oil_penalty
    );
    assert!(!run.vehicles[0].ignition.ignition);
    step(&mut run, &reference, tap);
    assert!(run.vehicles[0].ignition.ignition);
    let sample = run.instrument_sample(&reference, 0);
    assert!(sample.local_ignition);
    assert_eq!(sample.rpm, 0.);
    assert_eq!(sample.temperature, 0.);
    assert_eq!(run.vehicles[0].engine_output.torque_nm, 0.);
    let saved = run
        .capture_save(
            &reference,
            &stamp,
            CameraSave::FirstPerson,
            "2026-10-20T12:00:00Z",
        )
        .unwrap();
    assert!(saved.vehicles[0].ignition);
    let restored = GarageRuntime::from_save(&reference, &stamp, &saved).unwrap();
    assert!(restored.paused);
    assert!(restored.vehicles[0].ignition.ignition);
    assert!(!restored.vehicles[0].ignition.starter);
    assert!(!restored.vehicles[0].engine_output.starter);
    assert_eq!(
        restored.vehicles[0].engine.capture_save(),
        saved.vehicles[0].engine
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
    let chassis = run.physics.chassis_handle(0);
    let loose = run.area.items.get(source.item).unwrap().body.unwrap();
    assert_ne!(loose, chassis);
    let before = run.vehicles[0].engine.capture_save();
    let chassis_velocity = run.physics.rapier().bodies[chassis].linvel();
    run.physics.inactive_bodies.insert(chassis);
    step(&mut run, &reference, FrameInput::default());
    let after = run.vehicles[0].engine.capture_save();
    assert!(after.elapsed > before.elapsed);
    assert!(after.rpm > 0.);
    assert_eq!(
        run.physics.rapier().bodies[chassis].linvel(),
        chassis_velocity
    );
    run.physics.inactive_bodies.insert(loose);
    step(&mut run, &reference, FrameInput::default());
    assert_eq!(run.vehicles[0].engine.capture_save(), after);
    run.physics.inactive_bodies.remove(&loose);
    for _ in 0..20 {
        step(&mut run, &reference, FrameInput::default());
    }
    assert!(!run.vehicles[0].engine_output.running);
    assert!(run.vehicles[0].engine_output.rpm < output.rpm);
    assert_eq!(run.vehicles[0].engine.capture_save().supplied_fuel, 0.);
    // Chassis commands do not start a detached engine.
    run.physics.inactive_bodies.remove(&chassis);
    for _ in 0..(source.starter_delay_seconds / FIXED_DT).ceil() as usize + 2 {
        step(&mut run, &reference, held);
    }
    assert!(run.vehicles[0].ignition.starter);
    assert!(!run.vehicles[0].engine_output.starter);
    assert_eq!(run.vehicles[0].engine_output.torque_nm, 0.);
    run.clear_input();
    let engine_before_install = run.vehicles[0].engine.capture_save();
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
    assert_eq!(run.vehicles[0].engine.capture_save(), engine_before_install);
    let item = run.area.items.get(source.item).unwrap();
    assert_eq!(item.id, item_before_install.id);
    assert_eq!(item.temperature, item_before_install.temperature);
    assert_eq!(item.fluid, item_before_install.fluid);
    assert!(run.area.items.mounted(source.item));
}
