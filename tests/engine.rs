mod common;

use my_summer_slop::fluids::{FluidContainer, VehicleFluids};
use my_summer_slop::reference::EngineReference;
use my_summer_slop::simulation::{
    FIXED_DT,
    engine::{EngineInput, EngineOutput, EngineState},
};

fn fluids() -> [FluidContainer; 3] {
    let r = common::reference();
    [
        r.vehicles[0].engine.fuel_tank,
        r.vehicles[0].engine.oil_tank,
        r.vehicles[0].engine.coolant_tank,
    ]
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
fn bad_fuel_oil_and_heat_prevent_running() {
    let r = common::reference().vehicles.remove(0).engine;
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
fn removed_engine_stops_and_removed_radiator_cannot_cool_or_consume_contents() {
    let r = common::reference().vehicles.remove(0).engine;
    let mut tanks = fluids();
    let mut state = EngineState::new(&r);
    state.rpm = 3000.;
    state.ignition = true;
    state.temperature = 110.;
    let before = tanks.clone();
    let output = step(
        &mut state,
        EngineInput {
            engine_present: false,
            ignition_held: true,
            throttle: 1.,
            ..Default::default()
        },
        &mut tanks,
        &r,
    );
    assert!(!output.running);
    assert_eq!(output.rpm, 0.);
    assert_eq!(output.torque_nm, 0.);
    assert_eq!(tanks, before);
    let mut connected = EngineState::new(&r);
    connected.rpm = 3000.;
    connected.ignition = true;
    connected.temperature = 110.;
    let mut disconnected = connected.clone();
    let mut other = tanks.clone();
    let input = EngineInput {
        throttle: 1.,
        ambient_temperature: 20.,
        ..Default::default()
    };
    for _ in 0..30 {
        step(&mut connected, input, &mut tanks, &r);
        step(
            &mut disconnected,
            EngineInput {
                radiator_present: false,
                ..input
            },
            &mut other,
            &r,
        );
    }
    assert!(disconnected.temperature > connected.temperature);
    assert_eq!(other[2], before[2]);
}
