use super::*;
use crate::reference::InstrumentReference;
use crate::simulation::instruments::{InstrumentSample, InstrumentState};
use std::collections::HashSet;

#[test]
fn g10_instrument_backlight_contract() {
    let (reference, _) = load_garage();
    let instruments = &reference.vehicles[0].instruments;
    let value = serde_json::to_value(instruments).unwrap();
    let mut missing = value.clone();
    missing.as_object_mut().unwrap().remove("backlight");
    assert!(serde_json::from_value::<InstrumentReference>(missing).is_err());
    let mut missing = value.clone();
    missing["needles"][0]
        .as_object_mut()
        .unwrap()
        .remove("backlight");
    assert!(serde_json::from_value::<InstrumentReference>(missing).is_err());
    let nodes: HashSet<_> = reference.nodes.iter().map(|node| node.id).collect();
    instruments.validate(&nodes, &reference.assets).unwrap();
    let renderer = instruments
        .needles
        .iter()
        .enumerate()
        .find(|(_, needle)| {
            serde_json::to_value(needle).unwrap()["backlight"]["kind"] == "renderer"
        })
        .expect("source needle renderer")
        .0;
    for dashboard in [false, true] {
        for pointer in ["/node", "/on/asset", "/off/gltf_material"] {
            let mut broken = value.clone();
            let mut backlight = value["needles"][renderer]["backlight"].clone();
            *backlight.pointer_mut(pointer).unwrap() = serde_json::json!(u32::MAX);
            if dashboard {
                broken["backlight"] = backlight;
            } else {
                broken["needles"][renderer]["backlight"] = backlight;
            }
            let broken: InstrumentReference = serde_json::from_value(broken).unwrap();
            assert!(
                broken.validate(&nodes, &reference.assets).is_err(),
                "{pointer}"
            );
        }
    }
}

#[test]
fn g10_instrument_lighting_matrix() {
    let (reference, _) = load_garage();
    let source = &reference.vehicles[0].instruments;
    for electronics in [
        source.electronics_off_threshold - 0.001,
        source.electronics_off_threshold,
        source.electronics_off_threshold + 0.001,
    ] {
        let mut r = source.clone();
        r.electronics_output = electronics;
        for ignition in [false, true] {
            let local_ignition = ignition && electronics > r.electronics_off_threshold;
            for headlights in 0..=2 {
                for (fuel_liters, oil_liters, rpm) in [
                    (r.low_fuel - 0.001, r.low_oil - 0.001, 0.),
                    (r.low_fuel, r.low_oil, r.oil_test_rpm),
                    (
                        r.low_fuel + 0.001,
                        r.low_oil + 0.001,
                        r.oil_test_rpm - 0.001,
                    ),
                    (
                        r.low_fuel + 0.001,
                        r.low_oil + 0.001,
                        r.oil_test_rpm + 0.001,
                    ),
                    (
                        r.low_fuel + 0.001,
                        r.low_oil + 0.001,
                        r.battery_test_rpm - 0.001,
                    ),
                    (r.low_fuel + 0.001, r.low_oil + 0.001, r.battery_test_rpm),
                    (
                        r.low_fuel + 0.001,
                        r.low_oil + 0.001,
                        r.battery_test_rpm + 0.001,
                    ),
                ] {
                    let sample = InstrumentSample {
                        speed_kmh: 43.,
                        fuel_liters,
                        oil_liters,
                        rpm,
                        temperature: 76.,
                        local_ignition,
                        headlights,
                        distance_m: 12345.,
                    };
                    let mut state = InstrumentState::new(&r);
                    state.initialize_from_sample(&r, sample);
                    assert_eq!(
                        state.digits_lit,
                        local_ignition && matches!(headlights, 1 | 2)
                    );
                    for (i, lamp) in r.lamps.iter().enumerate() {
                        let expected = match lamp.signal.as_str() {
                            "fuel" => local_ignition && fuel_liters < r.low_fuel,
                            "oil" => {
                                local_ignition && (oil_liters < r.low_oil || rpm < r.oil_test_rpm)
                            }
                            "battery" => local_ignition && rpm < r.battery_test_rpm,
                            "headlight2" => {
                                headlights == 2 && electronics > r.electronics_off_threshold
                            }
                            _ => panic!("source warning signal"),
                        };
                        assert_eq!(state.lamp_bits & (1 << i) != 0, expected, "{}", lamp.signal);
                    }
                    let baseline = (
                        state.needle_angles.clone(),
                        state.digit_angles.clone(),
                        state.lamp_bits,
                        state.digits_lit,
                    );
                    state.update(&r, InstrumentSample::default(), 1.);
                    state.initialize_from_sample(&r, sample);
                    assert_eq!(
                        (
                            state.needle_angles,
                            state.digit_angles,
                            state.lamp_bits,
                            state.digits_lit
                        ),
                        baseline
                    );
                }
            }
        }
    }
}
