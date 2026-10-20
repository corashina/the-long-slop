//! Engine save boundaries checked against the installed build 11140890 export.
use my_summer_slop::{
    fluids::{FluidContainer, FluidMixture, transfer},
    reference::ReferenceManifest,
    save::{
        CameraSave, FluidSnapshot, IdentityMap, SaveError, SaveSnapshot, SourceHash, SourceStamp,
    },
    starting_area::GarageRuntime,
};

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

#[test]
#[ignore = "requires the installed build 11140890 export"]
fn fluid_save_order_and_validation() {
    let (reference, stamp) = installed_source();
    let mut game = GarageRuntime::new(&reference).unwrap();
    let fuel_id = reference.vehicles[0].engine.fuel_tank;
    let container = game
        .area
        .items
        .get_mut(fuel_id)
        .unwrap()
        .fluid
        .as_mut()
        .unwrap();
    *container = FluidContainer::new(container.capacity_liters, FluidMixture::default());
    for (index, liters) in [(3, 3.), (0, 2.)] {
        let mut quantities_liters = [0.; 6];
        quantities_liters[index] = liters;
        let mut source = FluidContainer::new(liters, FluidMixture { quantities_liters });
        assert_eq!(transfer(&mut source, container, liters), liters);
    }
    let saved = game
        .capture_save(
            &reference,
            &stamp,
            CameraSave::FirstPerson,
            "2026-10-20T12:00:00Z",
        )
        .unwrap();
    let ids = IdentityMap::new(&reference).unwrap();
    let fuel = saved
        .items
        .iter()
        .position(|item| ids.item(&item.instance).unwrap() == fuel_id)
        .unwrap();
    let fluids = saved.items[fuel].fluids.as_ref().unwrap();
    assert_eq!(fluids.order, [3, 0]);
    assert_eq!(fluids.quantities_liters, [2., 0., 0., 3., 0., 0.]);
    let decoded = SaveSnapshot::from_json(&serde_json::to_vec(&saved).unwrap()).unwrap();
    let restored = GarageRuntime::from_save(&reference, &stamp, &decoded).unwrap();
    // Item contents restore before the world streaming gate opens.
    let recaptured = restored
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
    assert_eq!(recaptured[fuel].fluids, saved.items[fuel].fluids);
    for order in [
        vec![3, 0, 0],
        vec![3, 0, 6],
        vec![0, 1, 2, 3, 4, 5, 0],
        vec![3],
    ] {
        let mut invalid = saved.clone();
        invalid.items[fuel].fluids.as_mut().unwrap().order = order;
        assert!(invalid.validate(&reference, &stamp).is_err());
    }
    for amount in [
        -1.,
        f32::NAN,
        reference
            .tanks
            .iter()
            .find(|tank| tank.item == fuel_id)
            .unwrap()
            .capacity_liters
            + 1.,
    ] {
        let mut invalid = saved.clone();
        invalid.items[fuel]
            .fluids
            .as_mut()
            .unwrap()
            .quantities_liters[0] = amount;
        assert!(invalid.validate(&reference, &stamp).is_err());
    }
    game.area
        .items
        .get_mut(fuel_id)
        .unwrap()
        .fluid
        .as_mut()
        .unwrap()
        .mixture
        .quantities_liters[1] = 1.;
    let quantity_only = game
        .capture_save(
            &reference,
            &stamp,
            CameraSave::FirstPerson,
            "2026-10-20T12:00:00Z",
        )
        .unwrap();
    assert_eq!(
        quantity_only.items[fuel].fluids.as_ref().unwrap().order,
        [3, 0, 1]
    );
    let restored = GarageRuntime::from_save(&reference, &stamp, &quantity_only).unwrap();
    // Item contents restore before the world streaming gate opens.
    let recaptured = restored
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
    assert_eq!(recaptured[fuel].fluids, quantity_only.items[fuel].fluids);
    let mut retained_zero = saved.clone();
    retained_zero.items[fuel]
        .fluids
        .as_mut()
        .unwrap()
        .order
        .push(5);
    retained_zero.validate(&reference, &stamp).unwrap();
    let restored = GarageRuntime::from_save(&reference, &stamp, &retained_zero).unwrap();
    // Item contents restore before the world streaming gate opens.
    let recaptured = restored
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
    assert_eq!(recaptured[fuel].fluids.as_ref().unwrap().order, [3, 0, 5]);
    let mut invalid = saved.clone();
    invalid
        .items
        .iter_mut()
        .find(|item| item.fluids.is_none())
        .unwrap()
        .fluids = Some(FluidSnapshot {
        quantities_liters: [0.; 6],
        order: vec![],
    });
    assert!(invalid.validate(&reference, &stamp).is_err());
    let mut invalid = saved.clone();
    invalid.schema_version = 6;
    assert!(invalid.validate(&reference, &stamp).is_err());
    assert!(GarageRuntime::from_save(&reference, &stamp, &invalid).is_err());
    assert!(matches!(
        SaveSnapshot::from_json(br#"{"schema_version":6}"#),
        Err(SaveError::IncompatibleSources(_))
    ));
    assert!(
        serde_json::from_str::<FluidSnapshot>(
            r#"{"quantities_liters":[0,0,0,0,0,0],"order":[],"extra":true}"#
        )
        .is_err()
    );
}

#[test]
#[ignore = "requires the installed build 11140890 export"]
fn ignition_save_ownership_and_cancellation() {
    use my_summer_slop::simulation::ignition::IgnitionInput;
    let (reference, stamp) = installed_source();
    let mut game = GarageRuntime::new(&reference).unwrap();
    let delay = reference.vehicles[0].engine.starter_delay_seconds;
    let held = IgnitionInput {
        held: true,
        ..Default::default()
    };
    game.vehicles[0].ignition.step(held, delay, true, delay);
    game.vehicles[0].ignition.step(held, delay, true, 0.02);
    game.vehicles[0].engine_output.starter = true;
    assert!(game.vehicles[0].ignition.starter);
    let mechanics = game.vehicles[0].engine.capture_save();
    let saved = game
        .capture_save(
            &reference,
            &stamp,
            CameraSave::FirstPerson,
            "2026-10-20T12:00:00Z",
        )
        .unwrap();
    assert!(saved.vehicles[0].ignition);
    let json = serde_json::to_value(&saved).unwrap();
    assert!(json["vehicles"][0]["engine"].get("ignition").is_none());
    assert!(json["vehicles"][0].get("starter").is_none());
    let loaded = GarageRuntime::from_save(&reference, &stamp, &saved).unwrap();
    assert!(loaded.paused);
    assert!(loaded.vehicles[0].ignition.ignition);
    assert!(!loaded.vehicles[0].ignition.starter);
    assert!(!loaded.vehicles[0].engine_output.starter);
    assert_eq!(loaded.vehicles[0].engine.capture_save(), mechanics);
    game.clear_input();
    assert!(game.vehicles[0].ignition.ignition);
    assert!(!game.vehicles[0].ignition.starter);
    assert!(!game.vehicles[0].engine_output.starter);
    assert_eq!(game.vehicles[0].engine.capture_save(), mechanics);
    let mut obsolete = json.clone();
    obsolete["vehicles"][0]["engine"]["ignition"] = true.into();
    assert!(SaveSnapshot::from_json(&serde_json::to_vec(&obsolete).unwrap()).is_err());
    let mut missing = json;
    missing["vehicles"][0]
        .as_object_mut()
        .unwrap()
        .remove("ignition");
    assert!(SaveSnapshot::from_json(&serde_json::to_vec(&missing).unwrap()).is_err());
    for penalty in [-0.01, 1.01, f32::NAN] {
        let mut invalid = saved.clone();
        invalid.vehicles[0].engine.oil_penalty = penalty;
        assert!(invalid.validate(&reference, &stamp).is_err());
    }
}

use my_summer_slop::{
    fluids::VehicleFluids,
    reference::EngineReference,
    simulation::engine::{EngineInput, EngineOutput, EngineState, InstalledEngineInput},
};

#[derive(Clone)]
struct EngineRig {
    engine: EngineState,
    fuel: FluidContainer,
    sump: FluidContainer,
    coolant: FluidContainer,
}
impl EngineRig {
    fn new(reference: &ReferenceManifest) -> Self {
        let source = &reference.vehicles[0].engine;
        let container =
            |id| FluidContainer::from(reference.tanks.iter().find(|tank| tank.item == id).unwrap());
        Self {
            engine: EngineState::new(source),
            fuel: container(source.fuel_tank),
            sump: container(source.oil_tank),
            coolant: container(source.coolant_tank),
        }
    }
    fn step(
        &mut self,
        source: &EngineReference,
        input: EngineInput,
        sample: impl FnMut(u64) -> f32,
        dt: f32,
    ) -> EngineOutput {
        self.engine.step(
            input,
            VehicleFluids {
                fuel: &mut self.fuel,
                sump: &mut self.sump,
                coolant: &mut self.coolant,
            },
            source,
            sample,
            dt,
        )
    }
}
fn installed() -> EngineInput {
    EngineInput {
        installed: Some(InstalledEngineInput {
            ignition: true,
            throttle: 1.,
            clutch: 1.,
            wheel_rpm: 2000.,
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn primed(rig: &EngineRig) -> my_summer_slop::save::EngineSnapshot {
    let mut state = rig.engine.capture_save();
    state.elapsed = 1.;
    state.last_run_time = 1.;
    state.rpm = 2000.;
    state.random_start = 0.;
    state.fuel_suitability = 1.;
    state.supplied_fuel = 0.4;
    state
}
fn close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.000001,
        "{actual} != {expected}"
    );
}

#[test]
#[ignore = "requires the installed build 11140890 export"]
fn engine_draws_and_restore() {
    let (reference, _) = installed_source();
    // Rule test: make failed samples possible. The installed Lada chance is one.
    let mut rule = reference.vehicles[0].engine.clone();
    rule.start_chance = 0.5;
    let source = &rule;
    let mut rig = EngineRig::new(&reference);
    let saved = rig.engine.capture_save();
    for dt in [0., -1., f32::NAN, f32::INFINITY] {
        rig.step(
            source,
            installed(),
            |_| panic!("no draw for invalid dt"),
            dt,
        );
        assert_eq!(rig.engine.capture_save(), saved);
    }
    assert_eq!(saved.random_start, 1.);
    assert_eq!(saved.start_draw, "0");
    let interval = source.start_chance_interval_seconds * source.engine_start_time_modifier;
    let mut input = installed();
    input.installed.as_mut().unwrap().starter = true;
    let mut draw_indices = Vec::new();
    rig.step(
        source,
        input,
        |_| panic!("no draw before the interval"),
        interval * 0.5,
    );
    rig.step(
        source,
        input,
        |_| panic!("no draw at the interval boundary"),
        interval * 0.5,
    );
    rig.step(
        source,
        input,
        |draw| {
            draw_indices.push(draw);
            1.
        },
        0.001,
    );
    assert!(!rig.engine.running());
    rig.step(
        source,
        input,
        |draw| {
            draw_indices.push(draw);
            0.
        },
        interval + 0.001,
    );
    assert_eq!(draw_indices, vec![0, 1]);
    assert_eq!(rig.engine.capture_save().random_start, 0.);
    rig.step(source, input, |_| panic!("no draw while running"), 0.02);
    assert!(rig.engine.running());
    rig.step(
        source,
        EngineInput::default(),
        |_| panic!("no detached starter"),
        0.21,
    );
    let mut saved = rig.engine.capture_save();
    saved.random_start = 1.;
    rig.engine.restore_save(&saved).unwrap();
    rig.engine.clear_input();
    assert_eq!(rig.engine.capture_save(), saved);
    let mut restored = rig.clone();
    restored.engine = EngineState::new(source);
    restored.engine.restore_save(&saved).unwrap();
    let mut next = None;
    let mut loaded_next = None;
    let sample = |draw| (draw as f32 + 1.) / 10.;
    rig.step(
        source,
        input,
        |draw| {
            next = Some((draw, sample(draw)));
            sample(draw)
        },
        interval + 0.01,
    );
    restored.step(
        source,
        input,
        |draw| {
            loaded_next = Some((draw, sample(draw)));
            sample(draw)
        },
        interval + 0.01,
    );
    assert_eq!(next, Some((2, 0.3)));
    assert_eq!(loaded_next, next);
    assert_eq!(restored.engine.capture_save(), rig.engine.capture_save());
    for counter in ["", "-1", "+1", "01", " 1", "1.0", "18446744073709551616"] {
        let mut invalid = saved.clone();
        invalid.start_draw = counter.into();
        let before = restored.engine.capture_save();
        assert!(restored.engine.restore_save(&invalid).is_err());
        assert_eq!(restored.engine.capture_save(), before);
    }
    let mut maximum = saved.clone();
    maximum.start_draw = u64::MAX.to_string();
    restored.engine.restore_save(&maximum).unwrap();
    restored.step(
        source,
        input,
        |draw| {
            assert_eq!(draw, u64::MAX);
            1.
        },
        interval + 0.01,
    );
    assert_eq!(restored.engine.capture_save().start_draw, "0");

    let (_, stamp) = installed_source();
    let game = GarageRuntime::new(&reference).unwrap();
    let mut full = game
        .capture_save(
            &reference,
            &stamp,
            CameraSave::FirstPerson,
            "2026-10-20T12:00:00Z",
        )
        .unwrap();
    for counter in ["-1", "01", "18446744073709551616"] {
        full.vehicles[0].engine.start_draw = counter.into();
        assert!(full.validate(&reference, &stamp).is_err());
    }
    full.vehicles[0].engine.start_draw = u64::MAX.to_string();
    full.validate(&reference, &stamp).unwrap();
    for field in ["fuel_suitability", "heat_penalty", "start_draw"] {
        let mut missing = serde_json::to_value(&full).unwrap();
        missing["vehicles"][0]["engine"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(SaveSnapshot::from_json(&serde_json::to_vec(&missing).unwrap()).is_err());
    }

    for penalty in [-0.01, 1.01, f32::NAN] {
        for heat in [false, true] {
            let mut invalid = full.clone();
            if heat {
                invalid.vehicles[0].engine.heat_penalty = penalty;
            } else {
                invalid.vehicles[0].engine.fuel_suitability = penalty;
            }
            assert!(invalid.validate(&reference, &stamp).is_err());
        }
    }
}

#[test]
#[ignore = "requires the installed build 11140890 export"]
fn engine_thresholds_and_grace() {
    let (reference, _) = installed_source();
    let source = &reference.vehicles[0].engine;
    let mut rig = EngineRig::new(&reference);
    let mut saved = rig.engine.capture_save();
    saved.elapsed = 0.2;
    saved.last_run_time = 0.;
    rig.engine.restore_save(&saved).unwrap();
    assert!(!rig.engine.running());
    saved.elapsed = 0.2_f32.next_down();
    rig.engine.restore_save(&saved).unwrap();
    assert!(rig.engine.running());
    let mut fresh = rig.engine.capture_save();
    fresh.elapsed = 1.;
    fresh.last_run_time = -1.;
    fresh.rpm = 2000.;
    fresh.random_start = 0.;
    rig.engine.restore_save(&fresh).unwrap();
    let output = rig.step(source, installed(), |_| 0., 0.02);
    assert!(!output.running, "initial cached fuel suitability is zero");
    for (fuel, heat, expected) in [
        (0.3_f32.next_down(), 0., false),
        (0.3, 0., false),
        (0.3_f32.next_up(), 0., true),
        (1., 0.99_f32.next_down(), true),
        (1., 0.99, false),
        (1., 0.99_f32.next_up(), false),
    ] {
        let mut rig = EngineRig::new(&reference);
        let mut state = primed(&rig);
        state.last_run_time = -1.;
        state.fuel_suitability = fuel;
        state.heat_penalty = heat;
        rig.engine.restore_save(&state).unwrap();
        assert_eq!(
            rig.step(source, installed(), |_| panic!("no starter"), 0.02)
                .running,
            expected
        );
    }
    // Mounted oil service precedes the engine decision in this fixed-step order.
    for (penalty, expected) in [(0.98999_f32, true), (0.99001, false)] {
        let mut rig = EngineRig::new(&reference);
        let mut state = primed(&rig);
        state.last_run_time = -1.;
        rig.engine.restore_save(&state).unwrap();
        rig.sump.mixture.quantities_liters = [0.; 6];
        rig.sump.mixture.quantities_liters[source.oil_fluid as usize] =
            rig.sump.capacity_liters * 0.3 * (1. - penalty.cbrt());
        let output = rig.step(source, installed(), |_| 0., 0.02);
        assert_eq!(
            output.running, expected,
            "oil penalty {}",
            output.oil_penalty
        );
        assert_eq!(output.oil_penalty < 0.99, expected);
    }
    let mut healthy = EngineRig::new(&reference);
    healthy.engine.restore_save(&primed(&healthy)).unwrap();
    let normal = healthy.step(source, installed(), |_| 0., 0.02);
    assert!(normal.torque_nm > 0.);
    let mut low_oil = EngineRig::new(&reference);
    low_oil.engine.restore_save(&primed(&low_oil)).unwrap();
    low_oil.sump.mixture.quantities_liters[source.oil_fluid as usize] =
        low_oil.sump.capacity_liters * 0.03;
    let weak = low_oil.step(source, installed(), |_| 0., 0.02);
    assert!(weak.torque_nm > 0. && weak.torque_nm < normal.torque_nm);
    low_oil.sump.mixture.quantities_liters = [0.; 6];
    assert!(!low_oil.step(source, installed(), |_| 0., 0.21).running);

    let mut hot = EngineRig::new(&reference);
    let mut state = primed(&hot);
    state.temperature = source.optimal_temperature[3];
    hot.engine.restore_save(&state).unwrap();
    let first = hot.step(source, installed(), |_| 0., 0.02);
    assert!(first.running, "decision reads the previous heat penalty");
    assert_eq!(first.heat_penalty, 1.);
    assert_eq!(first.torque_nm, 0., "torque reads current heat penalty");
    assert!(!hot.step(source, installed(), |_| 0., 0.21).running);
    let mut warm = EngineRig::new(&reference);
    let mut state = primed(&warm);
    state.temperature = (source.optimal_temperature[2] + source.optimal_temperature[3]) * 0.5;
    warm.engine.restore_save(&state).unwrap();
    let output = warm.step(source, installed(), |_| 0., 0.02);
    assert!(output.torque_nm > 0. && output.torque_nm < normal.torque_nm);

    let mut fuel = EngineRig::new(&reference);
    fuel.engine.restore_save(&primed(&fuel)).unwrap();
    fuel.fuel.mixture.quantities_liters = [0.; 6];
    let first = fuel.step(source, installed(), |_| 0., 0.02);
    assert!(first.running, "decision reads previous fuel suitability");
    assert_eq!(first.fuel_suitability, 0.);
    assert_eq!(first.target_rpm, 0.);
    assert!(!fuel.step(source, installed(), |_| 0., 0.21).running);
    fuel.fuel.mixture.quantities_liters[0] = 1.;
    assert!(!fuel.step(source, installed(), |_| 0., 0.02).running);
    assert!(fuel.step(source, installed(), |_| 0., 0.02).running);
}

#[test]
#[ignore = "requires the installed build 11140890 export"]
fn engine_fluids_and_cooling() {
    let (reference, stamp) = installed_source();
    let source = &reference.vehicles[0].engine;
    let mut rig = EngineRig::new(&reference);
    // Rule test: larger rates expose sequential arithmetic above f32 rounding.
    let mut rule = source.clone();
    rule.globals.fuel_consumption = 10.;
    rule.globals.oil_consumption = 1.;
    rule.globals.coolant_evaporate_rate = 20.;
    rule.radiator.max_fluid_loss = 30.;
    rule.radiator.max_value = 0.;
    rule.heat_loss = 0.;
    rule.heat_gain_min = 0.;
    rule.heat_gain_max = 0.;
    rule.rpm_change_modifier = 0.;
    rig.fuel = FluidContainer::new(
        39.,
        FluidMixture {
            quantities_liters: [2., 0., 0., 3., 0., 0.],
        },
    );
    rig.sump = FluidContainer::new(
        5.,
        FluidMixture {
            quantities_liters: [0., 2., 0., 3., 0., 0.],
        },
    );
    rig.coolant = FluidContainer::new(
        5.,
        FluidMixture {
            quantities_liters: [0., 0., 0., 2., 3., 0.],
        },
    );
    let mut saved = primed(&rig);
    saved.temperature = 120.;
    rig.engine.restore_save(&saved).unwrap();
    let mut input = installed();
    input.sleep_modifier = 2.;
    input.installed.as_mut().unwrap().ignition = false;
    input.installed.as_mut().unwrap().radiator_condition = Some(4);
    let dt = 0.02;
    let output = rig.step(&rule, input, |_| panic!("no starter"), dt);
    let fuel_amount = 0.4
        * dt
        * input.sleep_modifier
        * rule.globals.fuel_consumption
        * (rule.fuel_consumption.iter().sum::<f32>() * (0.2 + 0.8 * 2000. / rule.max_rpm));
    let fuel_first = 2. - fuel_amount * 2. / 5.;
    let fuel_second = 3. - fuel_amount * 3. / (fuel_first + 3.);
    close(rig.fuel.mixture.amount(0), fuel_first);
    close(rig.fuel.mixture.amount(3), fuel_second);
    close(output.consumed_fuel_liters, 5. - (fuel_first + fuel_second));
    let oil_amount = rule.globals.oil_consumption
        * rule.oil_consumption_modifier
        * (2000. / rule.max_rpm)
        * dt
        * input.sleep_modifier;
    let oil_first = 2. - oil_amount * 2. / 5.;
    let oil_second = 3. - oil_amount * 3. / (oil_first + 3.);
    close(rig.sump.mixture.amount(1), oil_first);
    close(rig.sump.mixture.amount(3), oil_second);
    // Existing order: mechanical heat, sump drain, evaporation, radiator cooling/leak.
    let evaporation = rule.globals.coolant_evaporate_rate / 60.
        * dt
        * input.sleep_modifier
        * ((120. - rule.globals.min_steam_temperature)
            / (rule.globals.car_temperature_bounds[1] - rule.globals.min_steam_temperature));
    let water = 2. - evaporation * 2. / 5.;
    let blood = 3. - evaporation * 3. / (water + 3.);
    let leak = rule.radiator.max_fluid_loss / 60. * dt * input.sleep_modifier;
    let water_after_leak = water - leak * water / (water + blood);
    let blood_after_leak = blood - leak * blood / (water_after_leak + blood);
    close(rig.coolant.mixture.amount(3), water_after_leak);
    close(rig.coolant.mixture.amount(4), blood_after_leak);

    // Source sequential overdraw can create fluid after a negative total.
    // The engine publication boundary bounds each amount by its previous amount.
    let mut overdraw = EngineRig::new(&reference);
    overdraw.fuel = FluidContainer::new(
        4.,
        FluidMixture {
            quantities_liters: [1., 1., 1., 1., 0., 0.],
        },
    );
    overdraw.engine.restore_save(&primed(&overdraw)).unwrap();
    let mut drain_rule = source.clone();
    drain_rule.globals.fuel_consumption =
        4. / (0.4 * dt * (10. * (0.2 + 0.8 * 2000. / source.max_rpm)));
    let output = overdraw.step(&drain_rule, EngineInput::default(), |_| 0., dt);
    for (actual, expected) in overdraw
        .fuel
        .mixture
        .quantities_liters
        .into_iter()
        .zip([0., 0., 0., 1., 0., 0.])
    {
        close(actual, expected);
    }
    close(output.consumed_fuel_liters, 3.);

    // Installed rates and supported sleep scaling also deplete mixed containers.
    let mut near_empty = EngineRig::new(&reference);
    near_empty.fuel = FluidContainer::new(
        39.,
        FluidMixture {
            quantities_liters: [0.000001; 6],
        },
    );
    near_empty.sump = FluidContainer::new(
        5.,
        FluidMixture {
            quantities_liters: [0.0000001; 6],
        },
    );
    near_empty.coolant = FluidContainer::new(
        5.,
        FluidMixture {
            quantities_liters: [0.0000001; 6],
        },
    );
    let mut state = primed(&near_empty);
    state.temperature = 120.;
    near_empty.engine.restore_save(&state).unwrap();
    let mut input = installed();
    input.sleep_modifier = 20.;
    input.installed.as_mut().unwrap().radiator_condition = Some(4);
    for _ in 0..100 {
        let output = near_empty.step(source, input, |_| 0., dt);
        assert!(output.temperature.is_finite());
        assert!(output.consumed_fuel_liters.is_finite() && output.consumed_fuel_liters >= 0.);
        for container in [&near_empty.fuel, &near_empty.sump, &near_empty.coolant] {
            assert!(
                container
                    .mixture
                    .quantities_liters
                    .iter()
                    .all(|amount| amount.is_finite() && *amount >= 0.)
            );
            assert!(container.mixture.total() <= container.capacity_liters);
        }
    }
    assert!(!near_empty.engine.running());
    for wrong in [false, true] {
        let mut rig = EngineRig::new(&reference);
        rig.fuel = FluidContainer::new(
            39.,
            FluidMixture {
                quantities_liters: if wrong {
                    [0., 0., 5., 0., 0., 0.]
                } else {
                    [0.; 6]
                },
            },
        );
        for _ in 0..20 {
            let output = rig.step(source, installed(), |_| 0., dt);
            assert!(!output.running);
            assert_eq!(output.fuel_suitability, 0.);
            assert!(output.rpm.is_finite() && output.temperature.is_finite());
        }
    }

    // Cooling rule tests freeze mechanical heat to isolate radiator boundaries.
    let mut cooling_rule = source.clone();
    cooling_rule.heat_gain_min = 0.;
    cooling_rule.heat_gain_max = 0.;
    cooling_rule.heat_loss = 0.;
    cooling_rule.globals.coolant_evaporate_rate = 0.;
    cooling_rule.radiator.fan_on_when_engine_runs = false;
    let cooled = |temperature, condition, empty, fan_on, speed| {
        let mut rig = EngineRig::new(&reference);
        let mut state = primed(&rig);
        state.temperature = temperature;
        state.fan_on = fan_on;
        rig.engine.restore_save(&state).unwrap();
        if empty {
            rig.coolant.mixture.quantities_liters = [0.; 6];
        }
        let mut input = installed();
        let installed = input.installed.as_mut().unwrap();
        installed.radiator_condition = condition;
        installed.speed_kmh = speed;
        let output = rig.step(&cooling_rule, input, |_| 0., dt);
        (output, rig.engine.capture_save(), rig.coolant)
    };
    let radiator = &cooling_rule.radiator;
    assert_eq!(cooled(100., None, false, false, 0.).0.temperature, 100.);
    assert_eq!(cooled(100., Some(0), true, false, 0.).0.temperature, 100.);
    let good = cooled(100., Some(0), false, false, 0.);
    let worn = cooled(100., Some(4), false, false, 0.);
    assert!(good.0.temperature < worn.0.temperature && worn.0.temperature < 100.);
    assert!(worn.2.mixture.total() < good.2.mixture.total());
    let off = radiator.thermostat_range[0];
    assert_eq!(
        cooled(off.next_down(), Some(0), false, false, 0.)
            .0
            .temperature,
        off.next_down()
    );
    let midpoint = (radiator.thermostat_range[0] + radiator.thermostat_range[1]) * 0.5;
    let midpoint_expected = midpoint
        - dt * (radiator.max_value * 0.5 / 60.)
            * (midpoint / source.globals.temperature_difference_modifier)
                .abs()
                .clamp(1., 1000.);
    close(
        cooled(midpoint, Some(0), false, false, 0.).0.temperature,
        midpoint_expected,
    );

    assert_eq!(cooled(off, Some(0), false, false, 0.).0.temperature, off);
    assert!(
        cooled(off.next_up(), Some(0), false, false, 0.)
            .0
            .temperature
            <= off.next_up()
    );
    // At the fully open thermostat, cooling follows the source temperature factor.
    let open = radiator.thermostat_range[1];
    let expected = open
        - dt * (radiator.max_value / 60.)
            * (open / source.globals.temperature_difference_modifier)
                .abs()
                .clamp(1., 1000.);
    close(
        cooled(open, Some(0), false, false, 0.).0.temperature,
        expected,
    );
    assert!(
        !cooled(radiator.fan_temperature_on, Some(0), false, false, 0.)
            .1
            .fan_on
    );
    assert!(
        cooled(
            radiator.fan_temperature_on.next_up(),
            Some(0),
            false,
            false,
            0.
        )
        .1
        .fan_on
    );
    assert!(
        cooled(radiator.fan_temperature_off, Some(0), false, true, 0.)
            .1
            .fan_on
    );
    assert!(
        !cooled(
            radiator.fan_temperature_off.next_down(),
            Some(0),
            false,
            true,
            0.
        )
        .1
        .fan_on
    );
    // Source quirk: a latched fan adds no speed between its on/off thresholds.
    let latched = cooled(95., Some(0), false, true, 0.);
    let unlatched = cooled(95., Some(0), false, false, 0.);
    assert_eq!(latched.0.temperature, unlatched.0.temperature);
    let forced = cooled(103., Some(0), false, false, 0.);
    assert!(forced.1.fan_on);

    let mut stopped = EngineRig::new(&reference);
    let mut state = stopped.engine.capture_save();
    state.temperature = 120.;
    stopped.engine.restore_save(&state).unwrap();
    let before = stopped.coolant.mixture.total();
    let mut stopped_input = installed();
    stopped_input.installed.as_mut().unwrap().ignition = false;
    stopped_input.installed.as_mut().unwrap().radiator_condition = Some(4);
    let out = stopped.step(source, stopped_input, |_| 0., dt);
    assert!(!out.running);
    assert!(
        stopped.coolant.mixture.total() < before,
        "hot stopped evaporation"
    );
    let mut missing = stopped.clone();
    let before_missing = missing.coolant.clone();
    stopped_input.installed.as_mut().unwrap().radiator_condition = None;
    missing.step(source, stopped_input, |_| 0., dt);
    assert_eq!(
        missing.coolant, before_missing,
        "missing radiator has no evaporation or leak"
    );
    let mut detached = stopped.clone();
    let coolant = detached.coolant.clone();
    let oil = detached.sump.clone();
    let before = detached.engine.temperature;
    detached.step(source, EngineInput::default(), |_| 0., dt);
    assert!(detached.engine.temperature < before);
    assert_eq!(detached.coolant, coolant);
    assert_eq!(detached.sump, oil);

    // Restore actual item quantities and order, without opening the world gate.
    let mut game = GarageRuntime::new(&reference).unwrap();
    game.vehicles[0].engine = rig.engine.clone();
    for (id, fluid) in [
        (source.fuel_tank, &rig.fuel),
        (source.oil_tank, &rig.sump),
        (source.coolant_tank, &rig.coolant),
    ] {
        game.area.items.get_mut(id).unwrap().fluid = Some(fluid.clone());
    }
    let saved = game
        .capture_save(
            &reference,
            &stamp,
            CameraSave::FirstPerson,
            "2026-10-20T12:00:00Z",
        )
        .unwrap();
    let loaded = GarageRuntime::from_save(&reference, &stamp, &saved).unwrap();
    let mut continued = EngineRig {
        engine: loaded.vehicles[0].engine.clone(),
        fuel: loaded
            .area
            .items
            .get(source.fuel_tank)
            .unwrap()
            .fluid
            .clone()
            .unwrap(),
        sump: loaded
            .area
            .items
            .get(source.oil_tank)
            .unwrap()
            .fluid
            .clone()
            .unwrap(),
        coolant: loaded
            .area
            .items
            .get(source.coolant_tank)
            .unwrap()
            .fluid
            .clone()
            .unwrap(),
    };
    for _ in 0..20 {
        rig.step(source, input, |_| 0., dt);
        continued.step(source, input, |_| 0., dt);
        assert_eq!(rig.engine.capture_save(), continued.engine.capture_save());
        assert_eq!(rig.fuel, continued.fuel);
        assert_eq!(rig.sump, continued.sump);
        assert_eq!(rig.coolant, continued.coolant);
    }
}
