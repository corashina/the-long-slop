//! ROADMAP test pass: Car parts and repair (CP rows). Each test cites its source method.
use crate::reference::ReferenceManifest;
use crate::starting_area::GarageRuntime;
use crate::world::stream::tests::kit::*;
use crate::world::{GameMode, StartCar, StartCarChoice};

/// A new game with the start car `field`, or the default car.
fn new_game(reference: &ReferenceManifest, field: Option<&str>) -> GarageRuntime {
    let mode = GameMode {
        start_car: field.map_or(StartCarChoice::default(), |f| {
            StartCarChoice::Field(StartCar::from_field(f).unwrap())
        }),
        ..GameMode::default()
    };
    let mut run =
        GarageRuntime::new(reference, mode, reference.world.draw_distance.default).unwrap();
    prepare_ready(&mut run, reference);
    start_vehicle(&mut run);
    run
}

/// (prototype node, state) of each start car part condition, by node.
fn start_states(run: &GarageRuntime) -> Vec<(usize, u8)> {
    let c = &run.area.interaction.conditions;
    let mut states: Vec<_> = start_car_conditions(run)
        .iter()
        .map(|&(id, _, node)| (node, c.get(id).state))
        .collect();
    states.sort();
    states
}

/// CP18, CP19: source `partconditionscript.Start` draws the start state with the fixed
/// startup random (`mainscript.startupRandomState`), so each new game draws the same wear.
#[test]
#[ignore = "requires the installed source export"]
fn tp_cp18_start_car_starts_worn_the_same_each_game() {
    let reference = reference();
    for field in [None, Some("StartCar01"), Some("StartCar05")] {
        let first = start_states(&new_game(&reference, field));
        let second = start_states(&new_game(&reference, field));
        assert_eq!(first, second, "{field:?}: the same wear each new game");
        // The body is the root node condition.
        let body = first[0].1;
        assert!(
            (1..=3).contains(&body),
            "{field:?}: body state {body} is Used to Old"
        );
        assert!(
            first.iter().any(|s| s.1 == 4),
            "{field:?}: a part shows rust"
        );
    }
}

/// The aim of a use ray that hits `item` on its use collider.
fn aim_on(run: &GarageRuntime, item: crate::reference::ItemId) -> crate::interaction::ResolvedAim {
    let (collider, c) = run
        .physics
        .rapier()
        .colliders
        .iter()
        .find(|(h, _)| run.area.interaction.use_owner(*h) == Some(item))
        .expect("item use collider");
    crate::interaction::ResolvedAim {
        use_target: Some(crate::interaction::AimTarget {
            collider,
            item,
            distance: 0.5,
            point: c.compute_aabb().center().to_array(),
            slot: None,
        }),
        ..Default::default()
    }
}

/// One interaction step with `aim`, as `GarageRuntime::step` runs it.
fn interact(
    run: &mut GarageRuntime,
    aim: crate::interaction::ResolvedAim,
    input: crate::simulation::FrameInput,
) {
    let needs = run.survival.output();
    run.area.interaction.step(
        input,
        aim,
        &mut run.area.player,
        &mut run.area.body,
        &mut run.area.items,
        &mut run.physics,
        crate::simulation::FIXED_DT,
        needs,
        1.,
    );
}

/// CP20: source `fpscontroller.RayStuff` with `tankcapscript` (`GetCapString`, `canToggle`,
/// `USE`): the oil and coolant caps show their name and level, and E opens and closes them.
#[test]
#[ignore = "requires the installed source export"]
fn tp_cp20_oil_and_coolant_caps_show_level_and_toggle() {
    use crate::interaction::feedback::{ActionMask, FeedbackContext};
    use crate::simulation::{FrameInput, keys};
    let (reference, mut run) = load_garage();
    seconds(&mut run, &reference, 1.);
    let car = run
        .physics
        .snapshot(run.vehicle.as_ref().unwrap())
        .pose
        .position;
    let caps: Vec<_> = run
        .area
        .items
        .iter()
        .filter(|i| !i.removed && run.area.interaction.generated_opening(i.id).is_some())
        .filter(|i| {
            let p = i.pose.position;
            (p[0] - car[0]).hypot(p[2] - car[2]) < 4.
        })
        .map(|i| i.id)
        .collect();
    let mut fluids = Vec::new();
    for cap in caps {
        let aim = aim_on(&run, cap);
        let available = run.area.interaction.available_actions(
            &aim,
            &run.area.player,
            &run.area.items,
            &run.physics,
            run.survival.output(),
        );
        let info = run
            .area
            .interaction
            .feedback(&available, &run.area.items, &reference);
        assert_eq!(available.context, FeedbackContext::Cap, "{cap:?}");
        let f = info.fluid.as_ref().expect("cap level");
        let main = (0..6)
            .max_by(|&a, &b| f.quantities[a].total_cmp(&f.quantities[b]))
            .unwrap();
        assert!(
            available.actions.0 & ActionMask::USE != 0,
            "{cap:?}: E toggles"
        );
        assert!(f.presentation.has_name && !f.presentation.no_text && f.capacity > 0.);
        fluids.push(main);
        {
            let press = FrameInput {
                pressed: keys::USE,
                held: keys::USE,
                ..FrameInput::default()
            };
            let before = run.area.interaction.generated_valve(cap).unwrap();
            interact(&mut run, aim, press);
            interact(
                &mut run,
                aim,
                FrameInput {
                    released: keys::USE,
                    ..FrameInput::default()
                },
            );
            let open = run.area.interaction.generated_valve(cap).unwrap();
            interact(&mut run, aim, press);
            interact(
                &mut run,
                aim,
                FrameInput {
                    released: keys::USE,
                    ..FrameInput::default()
                },
            );
            let closed = run.area.interaction.generated_valve(cap).unwrap();
            assert_eq!(
                (before, open, closed),
                (0., 1., 0.),
                "{cap:?}: E opens and closes"
            );
        }
    }
    // Source fluid types: 1 oil, 3 water.
    assert!(
        fluids.contains(&1) && fluids.contains(&3),
        "oil and coolant caps: {fluids:?}"
    );
}

/// Seats the player in the start car, fills petrol and holds the key until the engine runs.
fn start_engine(run: &mut GarageRuntime, reference: &ReferenceManifest) {
    use crate::simulation::{FrameInput, keys};
    act(run, reference, r#"{"action":"seat","seat":"driver"}"#);
    act(run, reference, r#"{"action":"fill","fluid":0,"liters":20}"#);
    let key = run.vehicle.clone().unwrap();
    for step in 0..1000 {
        let pressed = if step == 0 { keys::IGNITION } else { 0 };
        frame_with(
            run,
            reference,
            FrameInput {
                held: keys::IGNITION,
                pressed,
                ..FrameInput::default()
            },
        );
        if run.engine_output(&key).running {
            break;
        }
    }
    frame_with(
        run,
        reference,
        FrameInput {
            released: keys::IGNITION,
            ..FrameInput::default()
        },
    );
    seconds(run, reference, 0.5);
    assert!(run.engine_output(&key).running, "the start car engine runs");
}

/// CP2: source `coolantTankscript.Cool` turns the fan by `selectedFanSpeed` (`maxFanSpeed`
/// times 1, 0.8, 0.6, 0.4, 0.2 per condition) while the engine runs and the fan is on.
/// CP3: source `carscript` smoke `rateOverTime = InverseLerp(0, maxRpm, rpm) * defSmokeRate`,
/// start color `GetBurnColor` every 2 s, and a 5 x 5 sheet.
#[test]
#[ignore = "requires the installed source export"]
fn tp_cp2_cp3_running_engine_spins_the_fan_and_smokes() {
    use crate::simulation::{FrameInput, keys};
    let (reference, mut run) = load_garage();
    seconds(&mut run, &reference, 1.);
    let key = run.vehicle.clone().unwrap();
    let vehicle = crate::starting_area::world_vehicle(&reference, run.vehicles[&key].prototype);
    for system in &vehicle.exhaust {
        assert_eq!(
            (
                system.scalars["UVModule.tilesX"],
                system.scalars["UVModule.tilesY"]
            ),
            (5., 5.),
            "smoke sheet"
        );
    }
    start_engine(&mut run, &reference);
    let mounted = run.powertrain.mounted_on(&run.area.items, &key);
    let radiator = mounted.radiator.expect("start car radiator");
    let engine = mounted.engine.unwrap();
    // A hot engine turns the fan on.
    let hot = run
        .powertrain
        .radiator(radiator)
        .unwrap()
        .record
        .fan_temperature_on
        + 5.;
    run.powertrain.engine_mut(engine).unwrap().state.temperature = hot;
    let angle = |run: &GarageRuntime| run.powertrain.radiator(radiator).unwrap().fan_angle;
    let before = angle(&run);
    seconds(&mut run, &reference, 1.);
    assert_ne!(angle(&run), before, "the fan spins while the engine runs");
    // Idle smoke, then more smoke at high rpm.
    let smoke = |run: &GarageRuntime| run.vehicles[&key].exhaust.particles(0).len();
    let idle = smoke(&run);
    assert!(idle > 0, "idle smoke");
    for _ in 0..150 {
        frame_with(
            &mut run,
            &reference,
            FrameInput {
                player_held: crate::simulation::player_keys::GAS,
                ..FrameInput::default()
            },
        );
    }
    assert!(
        smoke(&run) > idle,
        "smoke rate follows the rpm: {} > {idle}",
        smoke(&run)
    );
    // A fuel mix change changes the smoke color within 2 s.
    let color = |run: &GarageRuntime| {
        run.vehicles[&key]
            .exhaust
            .particles(0)
            .last()
            .unwrap()
            .color
    };
    let petrol = color(&run);
    let tank = run.vehicles[&key].fuel_tank.unwrap();
    let fluid = run
        .area
        .items
        .get_mut(tank)
        .unwrap()
        .fluid
        .as_mut()
        .unwrap();
    let total = fluid.total();
    fluid.restore_entries(&[
        crate::fluids::FluidEntry {
            fluid_type: 0,
            amount: total * 0.7,
        },
        crate::fluids::FluidEntry {
            fluid_type: 1,
            amount: total * 0.3,
        },
    ]);
    seconds(&mut run, &reference, 3.);
    assert_ne!(
        color(&run),
        petrol,
        "oil in the fuel changes the smoke color"
    );
    // The engine stops: the fan and the smoke stop.
    for _ in 0..200 {
        frame_with(
            &mut run,
            &reference,
            FrameInput {
                held: keys::IGNITION,
                pressed: keys::IGNITION,
                ..FrameInput::default()
            },
        );
        frame_with(
            &mut run,
            &reference,
            FrameInput {
                released: keys::IGNITION,
                ..FrameInput::default()
            },
        );
        if !run.engine_output(&key).running {
            break;
        }
    }
    assert!(!run.engine_output(&key).running, "the key stops the engine");
    let stopped = angle(&run);
    seconds(&mut run, &reference, 1.);
    assert_eq!(angle(&run), stopped, "the fan stops with the engine");
}

/// CP7: source `carscript.steer` saves with the car (`savedata.steer`) and the wheels keep
/// their angle when nobody drives.
#[test]
#[ignore = "requires the installed source export"]
fn tp_cp7_turned_wheels_stay_after_save_and_load() {
    use crate::simulation::{FrameInput, keys};
    let (reference, mut run) = load_garage();
    seconds(&mut run, &reference, 1.);
    act(&mut run, &reference, r#"{"action":"seat","seat":"driver"}"#);
    let key = run.vehicle.clone().unwrap();
    // MC2: the parked garage chassis sleeps and a sleeping chassis does not steer. Wake it, as
    // a car out of the garage would be.
    let chassis = run.physics.chassis_handle(&key);
    for _ in 0..100 {
        run.physics.edit_body(chassis, |b| b.wake_up(true));
        frame_with(
            &mut run,
            &reference,
            FrameInput {
                held: keys::LEFT,
                player_held: crate::simulation::player_keys::STEER_LEFT,
                ..FrameInput::default()
            },
        );
    }
    seconds(&mut run, &reference, 1.);
    let steer = run.vehicles[&key].steer;
    assert!(steer.abs() > 0.5, "wheels turned: {steer}");
    let mut loaded = save_load(&run, &reference);
    seconds(&mut loaded, &reference, 1.);
    let key = loaded
        .vehicles
        .keys()
        .find(|k| **k == key)
        .cloned()
        .expect("start car after load");
    assert!(
        (loaded.vehicles[&key].steer - steer).abs() < 1e-3,
        "{} != {steer}",
        loaded.vehicles[&key].steer
    );
}

/// CP1: source `partslotscript.UnCraft` and `Craft`: the hood and the trunk lid come off and
/// go back on. `partconditionscript` repair: the wire brush takes off rust, the sponge dirt.
#[test]
#[ignore = "requires the installed source export"]
fn tp_cp1_hood_and_trunk_lid_refit_and_repair() {
    use crate::interaction::items::ItemLocation;
    let (reference, mut run) = load_garage();
    seconds(&mut run, &reference, 1.);
    let key = run.vehicle.clone().unwrap();
    let car = run.physics.snapshot(&key).pose;
    let root = run.area.items.resolve_key(&key).unwrap();
    let rotation = rapier3d::math::Rotation::from_array(car.rotation);
    // Hinged parts of the car: doors, the hood and the trunk lid. The hood and the lid sit on
    // the car axis, at its two ends.
    let mut hinged: Vec<_> = run
        .area
        .interaction
        .parts
        .mounted()
        .map(|(item, _)| item)
        .filter(|&item| run.area.items.descends(item, root))
        .map(|item| {
            let p =
                rapier3d::math::Vector::from_array(run.area.items.get(item).unwrap().pose.position)
                    - rapier3d::math::Vector::from_array(car.position);
            (rotation.inverse() * p, item)
        })
        .collect();
    let all = hinged.clone();
    hinged.retain(|(local, _)| local.x.abs() < 0.3 && local.y > 0.3);
    hinged.sort_by(|a, b| a.0.z.total_cmp(&b.0.z));
    assert!(hinged.len() >= 2, "hood and trunk lid: {all:?}");
    for (_, part) in [hinged[0], hinged[hinged.len() - 1]] {
        let ItemLocation::PartSlot(slot) = run.area.items.get(part).unwrap().location else {
            panic!("{part:?} is mounted");
        };
        let i = &mut run.area.interaction;
        i.parts
            .remove(part, &mut run.area.items, &mut run.physics)
            .expect("part comes off");
        i.parts
            .detach_mechanism(part, &mut i.mechanisms, &mut run.physics);
        i.sync_parts(&mut run.area.items, &mut run.physics).unwrap();
        assert!(!i.parts.installed(part));
        seconds(&mut run, &reference, 0.5);
        let i = &mut run.area.interaction;
        i.parts
            .install(part, slot, &mut run.area.items, &mut run.physics)
            .expect("part goes back on");
        i.parts.mount_mechanism(part, &mut i.mechanisms);
        i.sync_parts(&mut run.area.items, &mut run.physics).unwrap();
        assert!(i.parts.installed(part));
        seconds(&mut run, &reference, 0.5);
        assert_eq!(
            run.area.items.get(part).unwrap().location,
            ItemLocation::PartSlot(slot)
        );
    }
    // Repair: each tool that the source permission table allows improves the part state.
    let mut improved = [false; 3];
    for (id, ..) in start_car_conditions(&run) {
        for tool in [1u8, 2] {
            let c = &mut run.area.interaction.conditions;
            let before = c.get(id).state;
            if before > 0 && before < 4 && c.can_repair(id, tool) && !c.can_repair(id, 3) {
                assert_eq!(
                    c.repair(id, tool, 1.),
                    crate::conditions::RepairResult::Improved
                );
                assert_eq!(c.get(id).state, before - 1);
                improved[tool as usize] = true;
            }
        }
    }
    assert!(
        improved[1] && improved[2],
        "wire brush and sponge repair: {improved:?}"
    );
}
