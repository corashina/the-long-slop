//! World car systems on a fixture car copy: headlights and gauges.
use super::*;
use crate::interaction::items::ItemStore;
use crate::reference::WorldPrototypeReference;
use crate::simulation::car_lights::CarLightLinks;
use crate::simulation::instruments::{InstrumentState, needle_angle};
use crate::simulation::powertrain::EnginePart;
use crate::simulation::{DriveInput, FIXED_DT};
use crate::vehicle::DriveTorques;
use crate::world::changes::ChangeRecords;
use crate::world::content_vehicles::{CarCopy, spawn};
use rapier3d::prelude::ColliderBuilder;
use serde_json::json;

/// The fixture car with one headlight controller (off, low, high) and a speed gauge with a
/// battery lamp.
fn prototype() -> WorldPrototypeReference {
    let mut prototype = crate::vehicle::car_prototype_fixture();
    let src = |id: u32| json!({"file": "a", "path_id": id});
    let material =
        |id: u32, index: u32| json!({"source": src(id), "asset": 5, "gltf_material": index});
    let state = |signal: &str, node: u32, intensity: f32| {
        json!({"signal": signal, "lights": [node],
        "intensity": intensity, "renderers": [4], "on_material": material(41, 1), "off_material": material(42, 2),
        "broken": false, "working_nodes": [], "broken_nodes": [], "dont_overwrite": false})
    };
    let vehicle = prototype.vehicle.as_mut().unwrap();
    vehicle.lights = vec![serde_json::from_value(json!({"source": src(40), "kind": "headlight", "node": 0,
        "electronics": null, "startup_on": false, "current_state": 0, "requested_state": 0, "energy": 1.0,
        "energy_move": 10.0, "enabled": true, "reverse_switch": false, "cabin": null,
        "states": [state("off", 1, 0.0), state("low", 2, 2.0), state("high", 3, 3.0)]}))
    .unwrap()];
    vehicle.instruments = Some(
        serde_json::from_value(json!({"backlight": {"kind": "absent"}, "electronics_output": 1.0,
            "electronics_off_threshold": 0.2,
            "needles": [{"source": src(50), "node": 0, "backlight": {"kind": "absent"}, "signal": "speed",
                "min_value": 0.0, "max_value": 200.0, "min_angle": 0.0, "max_angle": 270.0, "axis": 2,
                "rest": [0.0, 0.0, 0.0], "offset": 0.0, "inv": false, "value_smoothing": 8.0,
                "signal_smoothing": 0.0, "no_value_lerp": false, "lerp_angle": false}],
            "lamps": [{"source": src(51), "node": 0, "signal": "battery", "on": material(52, 1),
                "off": material(53, 2), "min_value": 1.0, "max_value": 1.0, "stay_on": false}],
            "odometer": {"source": src(54), "initial_value": 0.0, "base": 10, "decimals": 0.0,
                "use_multiplier": false, "multiplier": 1.0, "digits": []},
            "low_fuel": 5.0, "low_oil": 1.0, "oil_test_rpm": 500.0, "battery_test_rpm": 500.0}))
        .unwrap(),
    );
    prototype
}

/// The item of the fixture car's light switch usable.
const SWITCH: ItemId = ItemId(30);

#[test]
fn world_car_headlights_switch_and_gauges_read_rpm_and_speed() {
    let prototype = prototype();
    let record = prototype.vehicle.as_ref().unwrap();
    let key = ObjectKey::Imported(ItemId(1));
    let mut physics = PhysicsWorld::empty(1, 2);
    physics.insert_box([0., -0.5, 0.], [500., 0.5, 500.]);
    let (body, collider) = physics.insert(
        RigidBodyBuilder::dynamic()
            .translation(rapier3d::math::Vector::new(0., 0.5, 0.))
            .additional_mass(900.),
        ColliderBuilder::cuboid(0.8, 0.25, 2.0).density(0.),
    );
    let mut vehicles = BTreeMap::new();
    spawn(
        CarCopy {
            key: key.clone(),
            prototype: 0,
            chassis: body,
            colliders: &[collider],
            bodies: Vec::new(),
            fuel_tank: None,
            electronics: None,
            lights: vec![CarLightLinks {
                electronics: None,
                owner: None,
                doors: Vec::new(),
            }],
            light_switch: Some(SWITCH),
            handbrake_lever: None,
            owner: ItemId(1),
            wheel_slots: &[],
            wheel_part_mass: 0.,
            tanks: Vec::new(),
        },
        &prototype,
        &crate::wheel_effects::tests::skid(),
        &mut physics,
        &mut vehicles,
        &mut crate::interaction::wheels::WheelAssemblies::default(),
        &mut crate::interaction::items::ItemStore::from_states(Vec::new()),
        &mut ChangeRecords::default(),
    )
    .unwrap();
    physics.refresh_queries();
    let vehicle = vehicles.get_mut(&key).unwrap();

    // Headlights: off, low, high. Each request fades the old state out and the new one in.
    let lit = |vehicle: &VehicleRuntime, node: u32| {
        let light = vehicle
            
            .lights
            .lights()
            .iter()
            .find(|l| l.node == node);
        light.filter(|l| l.enabled).map_or(0., |l| l.intensity)
    };
    let step_lights = |vehicle: &mut VehicleRuntime, seconds: f32| {
        for _ in 0..(seconds / FIXED_DT) as usize {
            let input = crate::simulation::car_lights::CarLightInput {
                requested: vehicle.requested_headlights,
                ..Default::default()
            };
            vehicle
                
                .lights
                .advance(record, input, |_| true, |_| 0., FIXED_DT);
        }
    };
    step_lights(vehicle, 0.5);
    assert_eq!((lit(vehicle, 2), lit(vehicle, 3)), (0., 0.));
    vehicle.requested_headlights = 1;
    step_lights(vehicle, 0.5);
    assert_eq!((lit(vehicle, 2), lit(vehicle, 3)), (2., 0.));
    let renderer = vehicle.lights.materials()[0];
    assert_eq!(
        renderer.choice.unwrap().link(record).unwrap().gltf_material,
        1
    );
    vehicle.requested_headlights = 2;
    step_lights(vehicle, 0.5);
    assert_eq!((lit(vehicle, 2), lit(vehicle, 3)), (0., 3.));

    // Gauges: an engine part at 2400 rpm and a car moving forward.
    let engine_item = ItemId(9);
    let mut powertrain = PowertrainParts::default();
    let mut engine = EnginePart::new(
        engine_item,
        serde_json::from_value(crate::reference::engine_part_fixture()).unwrap(),
        None,
    );
    let mut saved = engine.state.capture_save();
    saved.rpm = 2400.;
    engine.state.restore_save(&saved).unwrap();
    powertrain.register_engine(engine);
    vehicle.powertrain.engine = Some(engine_item);
    vehicle.ignition.ignition = true;
    for _ in 0..(1. / FIXED_DT) as usize {
        let throttle = DriveInput {
            throttle: 1.,
            ..Default::default()
        };
        let torque = DriveTorques {
            motor_nm: 1600.,
            engine_brake_nm: 0.,
        };
        physics.apply_vehicle_forces(&key, throttle, torque);
        physics.step_physics();
    }
    let items = ItemStore::from_states(Vec::new());
    let r = record.instruments.as_ref().unwrap();
    let sample = instrument_sample(&items, &physics, &powertrain, &key, vehicle, r, 2);
    let speed = physics.snapshot(&key).speed_mps * 3.6;
    assert_eq!(sample.rpm, 2400.);
    assert!(speed > 3. && sample.speed_kmh == speed, "{speed}");
    assert_eq!(sample.headlights, 2);
    let state = vehicle.instruments.as_mut().unwrap();
    state.initialize_from_sample(r, sample);
    assert_eq!(state.needle_angles[0], needle_angle(&r.needles[0], speed));
    assert_eq!(state.lamp_bits, 0, "battery lamp off above the test rpm");
    let mut stalled = InstrumentState::new(r);
    stalled.initialize_from_sample(
        r,
        crate::simulation::instruments::InstrumentSample { rpm: 0., ..sample },
    );
    assert_eq!(stalled.lamp_bits, 1, "battery lamp on below the test rpm");
}

/// Using the switch usable cycles the headlight request of its car: off, low, high, off.
#[test]
fn light_switch_click_toggles_world_car_headlights() {
    let prototype = prototype();
    let key = ObjectKey::Imported(ItemId(1));
    let mut physics = PhysicsWorld::empty(1, 2);
    let (body, collider) = physics.insert(
        RigidBodyBuilder::dynamic().additional_mass(900.),
        ColliderBuilder::cuboid(0.8, 0.25, 2.0).density(0.),
    );
    let mut vehicles = BTreeMap::new();
    spawn(
        CarCopy {
            key: key.clone(),
            prototype: 0,
            chassis: body,
            colliders: &[collider],
            bodies: Vec::new(),
            fuel_tank: None,
            electronics: None,
            lights: vec![CarLightLinks {
                electronics: None,
                owner: None,
                doors: Vec::new(),
            }],
            light_switch: Some(SWITCH),
            handbrake_lever: None,
            owner: ItemId(1),
            wheel_slots: &[],
            wheel_part_mass: 0.,
            tanks: Vec::new(),
        },
        &prototype,
        &crate::wheel_effects::tests::skid(),
        &mut physics,
        &mut vehicles,
        &mut crate::interaction::wheels::WheelAssemblies::default(),
        &mut crate::interaction::items::ItemStore::from_states(Vec::new()),
        &mut ChangeRecords::default(),
    )
    .unwrap();
    assert!(
        use_light_switch(&mut vehicles, ItemId(31)).is_none(),
        "another usable"
    );
    let mut states = Vec::new();
    for _ in 0..3 {
        let (car, record) = use_light_switch(&mut vehicles, SWITCH).unwrap();
        assert_eq!((car, record), (key.clone(), 0));
        states.push(vehicles[&key].requested_headlights);
    }
    assert_eq!(states, [1, 2, 0]);
    // The low beam lights after a switch to low.
    use_light_switch(&mut vehicles, SWITCH);
    let vehicle = vehicles.get_mut(&key).unwrap();
    let record = prototype.vehicle.as_ref().unwrap();
    for _ in 0..(0.5 / FIXED_DT) as usize {
        let input = crate::simulation::car_lights::CarLightInput {
            requested: vehicle.requested_headlights,
            ..Default::default()
        };
        vehicle
            
            .lights
            .advance(record, input, |_| true, |_| 0., FIXED_DT);
    }
    let low = vehicle
        
        .lights
        .lights()
        .iter()
        .find(|l| l.node == 2)
        .unwrap();
    assert!(low.enabled && low.intensity == 2.);
}
