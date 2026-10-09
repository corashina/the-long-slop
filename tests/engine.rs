mod common;

use desert_drive::fluids::{FluidContainer, VehicleFluids};
use desert_drive::reference::EngineReference;
use desert_drive::simulation::{
    FIXED_DT,
    engine::{EngineInput, EngineOutput, EngineState},
};

fn fluids() -> [FluidContainer; 3] {
    let r = common::reference();
    [r.engine.fuel_tank, r.engine.oil_tank, r.engine.coolant_tank]
        .map(|id| FluidContainer::from(r.tanks.iter().find(|t| t.item == id).unwrap()))
}

fn step(
    state: &mut EngineState,
    input: EngineInput,
    tanks: &mut [FluidContainer; 3],
    r: &EngineReference,
) -> EngineOutput {
    let [fuel, sump, coolant] = tanks;
    state.step(
        input,
        VehicleFluids {
            fuel,
            sump,
            coolant,
        },
        r,
        0.0,
        FIXED_DT,
    )
}

#[test]
fn ignition_edges_and_start() {
    let r = common::reference().engine;
    let mut tanks = fluids();
    let mut state = EngineState::new(&r);
    let tap = EngineInput {
        ignition_pressed: true,
        ignition_released: true,
        ..Default::default()
    };
    assert!(step(&mut state, tap, &mut tanks, &r).ignition);
    for _ in 0..3 {
        assert!(step(&mut state, EngineInput::default(), &mut tanks, &r).ignition);
    }
    assert!(!step(&mut state, tap, &mut tanks, &r).ignition);
    let held = EngineInput {
        ignition_held: true,
        ..Default::default()
    };
    assert!(!step(&mut state, held, &mut tanks, &r).starter);
    let mut output = EngineOutput::default();
    for _ in 0..120 {
        output = step(&mut state, held, &mut tanks, &r);
    }
    assert!(output.starter && output.ignition && output.running);
    assert!(output.rpm > r.shutdown_rpm);
    output = step(
        &mut state,
        EngineInput {
            ignition_released: true,
            ..Default::default()
        },
        &mut tanks,
        &r,
    );
    assert!(output.ignition);
    step(&mut state, EngineInput::default(), &mut tanks, &r);
    output = step(&mut state, tap, &mut tanks, &r);
    assert!(!output.ignition);
    for _ in 0..15 {
        output = step(&mut state, EngineInput::default(), &mut tanks, &r);
    }
    assert!(!output.running);
}

#[test]
fn bad_fuel_oil_and_heat_prevent_running() {
    let r = common::reference().engine;
    for fault in 0..5 {
        let mut tanks = fluids();
        let mut state = EngineState::new(&r);
        state.ignition = fault != 0;
        state.rpm = r.idle_rpm;
        match fault {
            1 => tanks[0].mixture.quantities_liters = [0.0, 0.0, 0.0, 5.0, 0.0, 0.0],
            2 => tanks[1].mixture.quantities_liters = [0.0; 6],
            3 => state.temperature = r.optimal_temperature[3],
            4 => tanks[0].mixture.quantities_liters = [1.5, 0.0, 0.0, 3.5, 0.0, 0.0],
            _ => {}
        }
        let fuel_before = tanks[0].mixture.total();
        let output = step(&mut state, EngineInput::default(), &mut tanks, &r);
        assert!(!output.running, "fault {fault}");
        assert_eq!(tanks[0].mixture.total(), fuel_before);
    }
}

#[test]
fn torque_fuel_and_penalties_follow_source_formulas() {
    let r = common::reference().engine;
    let mut tanks = fluids();
    let mut state = EngineState::new(&r);
    state.ignition = true;
    state.rpm = 3500.0;
    let input = EngineInput {
        throttle: 1.0,
        clutch: 1.0,
        wheel_rpm: 3500.0,
        ..Default::default()
    };
    let output = step(&mut state, input, &mut tanks, &r);
    assert!((output.torque_nm - 87.0).abs() < 0.00001);
    assert_eq!(output.target_rpm, r.max_rpm);
    assert!(tanks[1].mixture.amount(1) < 2.5);
    let expected_consumption =
        FIXED_DT * r.globals.fuel_consumption * (2.0 + 8.0 * 3500.0 / r.max_rpm);
    assert!((output.consumed_fuel_liters - expected_consumption).abs() < 0.00000001);

    let mut oily = fluids();
    oily[1].mixture.quantities_liters = [0.0, 0.75, 0.0, 0.0, 0.0, 0.0];
    let mut oil_state = EngineState::new(&r);
    oil_state.ignition = true;
    oil_state.rpm = 3500.0;
    let oil_output = step(&mut oil_state, input, &mut oily, &r);
    assert_eq!(oil_output.oil_penalty, 0.125);
    assert!((oil_output.torque_nm - output.torque_nm * 0.875).abs() < 0.00001);

    let mut hot = fluids();
    let mut hot_state = EngineState::new(&r);
    hot_state.ignition = true;
    hot_state.rpm = 3500.0;
    hot_state.temperature = 120.0;
    let hot_output = step(&mut hot_state, input, &mut hot, &r);
    assert_eq!(hot_output.heat_penalty, 0.125);
    assert!((hot_output.torque_nm - output.torque_nm * 0.875).abs() < 0.00001);

    let mut mixed = fluids();
    mixed[0].mixture.quantities_liters = [2.5, 0.0, 0.0, 2.5, 0.0, 0.0];
    let mut mixed_state = EngineState::new(&r);
    mixed_state.ignition = true;
    mixed_state.rpm = 1200.0;
    let mixed_output = step(
        &mut mixed_state,
        EngineInput {
            wheel_rpm: 1200.0,
            ..input
        },
        &mut mixed,
        &r,
    );
    assert_eq!(mixed_output.fuel_suitability, 0.5);
    assert!((mixed_output.target_rpm - 1850.0).abs() < 0.0001);
    assert_eq!(mixed[0].mixture.amount(0), mixed[0].mixture.amount(3));

    let mut middle = fluids();
    let mut middle_state = EngineState::new(&r);
    middle_state.ignition = true;
    middle_state.rpm = 1250.0;
    let middle_output = step(
        &mut middle_state,
        EngineInput {
            wheel_rpm: 1250.0,
            ..input
        },
        &mut middle,
        &r,
    );
    assert!((middle_output.torque_nm - 36.176_11).abs() < 0.00001);

    let mut braking = fluids();
    let mut braking_state = EngineState::new(&r);
    braking_state.ignition = true;
    braking_state.rpm = 3500.0;
    let braking_output = step(
        &mut braking_state,
        EngineInput {
            throttle: 0.0,
            ..input
        },
        &mut braking,
        &r,
    );
    assert_eq!(braking_output.torque_nm, 0.0);
    assert!((braking_output.brake_torque_nm - 78.3).abs() < 0.00001);
}

#[test]
fn shutdown_uses_grace_then_stops_fuel_consumption() {
    let r = common::reference().engine;
    let mut tanks = fluids();
    let mut state = EngineState::new(&r);
    state.rpm = r.idle_rpm;
    state.ignition = true;
    assert!(step(&mut state, EngineInput::default(), &mut tanks, &r).running);
    let tap = EngineInput {
        ignition_pressed: true,
        ignition_released: true,
        ..Default::default()
    };
    let output = step(&mut state, tap, &mut tanks, &r);
    assert!(!output.ignition && output.running);
    assert!(output.consumed_fuel_liters > 0.0);
    for _ in 0..15 {
        step(&mut state, EngineInput::default(), &mut tanks, &r);
    }
    let fuel_before = tanks[0].mixture.total();
    let output = step(&mut state, EngineInput::default(), &mut tanks, &r);
    assert!(!output.running);
    assert_eq!(output.consumed_fuel_liters, 0.0);
    assert_eq!(tanks[0].mixture.total(), fuel_before);
}

#[test]
fn start_chance_uses_the_recovered_interval() {
    let mut r = common::reference().engine;
    r.start_chance = 0.5;
    let mut tanks = fluids();
    let mut state = EngineState::new(&r);
    let held = EngineInput {
        ignition_held: true,
        ..Default::default()
    };
    for _ in 0..120 {
        let [fuel, sump, coolant] = &mut tanks;
        let output = state.step(
            held,
            VehicleFluids {
                fuel,
                sump,
                coolant,
            },
            &r,
            0.75,
            FIXED_DT,
        );
        assert!(!output.running);
    }
    let mut output = EngineOutput::default();
    for _ in 0..70 {
        output = step(&mut state, held, &mut tanks, &r);
    }
    assert!(output.running);
}

#[test]
fn missing_coolant_changes_cooling_without_a_start_lock() {
    let r = common::reference().engine;
    let mut wet = fluids();
    let mut dry = wet.clone();
    dry[2].mixture.quantities_liters = [0.0; 6];
    let mut wet_state = EngineState::new(&r);
    wet_state.temperature = 100.0;
    wet_state.rpm = r.idle_rpm;
    wet_state.ignition = true;
    let mut dry_state = wet_state.clone();
    let wet_output = step(&mut wet_state, EngineInput::default(), &mut wet, &r);
    let dry_output = step(&mut dry_state, EngineInput::default(), &mut dry, &r);
    assert!(wet_output.running && dry_output.running);
    assert!(dry_output.temperature > wet_output.temperature);
    assert!(wet_output.consumed_fuel_liters > 0.0);
}
