use desert_drive::fluids::{FluidContainer, FluidMixture, spill, suitability, transfer};

fn container(capacity: f32, quantities: [f32; 6]) -> FluidContainer {
    FluidContainer {
        capacity_liters: capacity,
        mixture: FluidMixture {
            quantities_liters: quantities,
        },
    }
}

#[test]
fn transfer_conserves_each_type() {
    let mut source = container(20.0, [3.0, 2.0, 1.0, 4.0, 2.0, 1.0]);
    let mut destination = container(8.0, [1.0, 0.0, 1.0, 1.0, 0.0, 0.0]);
    let before: [f32; 6] =
        std::array::from_fn(|i| source.mixture.amount(i) + destination.mixture.amount(i));
    let accepted = transfer(&mut source, &mut destination, 10.0);
    assert!((accepted - 5.0).abs() < 0.00001);
    for (index, amount) in before.iter().enumerate() {
        assert!(
            (source.mixture.amount(index) + destination.mixture.amount(index) - amount).abs()
                < 0.00001
        );
    }
    assert!((destination.mixture.total() - 8.0).abs() < 0.00001);
    assert_eq!(transfer(&mut source, &mut destination, 1.0), 0.0);
    assert_eq!(transfer(&mut source, &mut destination, f32::NAN), 0.0);
    assert_eq!(transfer(&mut source, &mut destination, -1.0), 0.0);
    let mut empty = container(20.0, [0.0; 6]);
    let remaining = source.mixture.total();
    assert_eq!(transfer(&mut source, &mut empty, f32::MAX), remaining);
    assert_eq!(source.mixture.total(), 0.0);
}

#[test]
fn spill_clamps_and_preserves_ratios() {
    let mut tank = container(5.0, [1.0, 1.0, 0.0, 2.0, 0.0, 0.0]);
    assert_eq!(spill(&mut tank, f32::NAN), 0.0);
    assert_eq!(spill(&mut tank, -1.0), 0.0);
    assert_eq!(spill(&mut tank, 2.0), 2.0);
    assert_eq!(
        tank.mixture.quantities_liters,
        [0.5, 0.5, 0.0, 1.0, 0.0, 0.0]
    );
    assert_eq!(spill(&mut tank, f32::MAX), 2.0);
    assert_eq!(tank.mixture.total(), 0.0);
    assert_eq!(spill(&mut tank, 1.0), 0.0);
}

#[test]
fn suitability_uses_the_limiting_required_ratio() {
    let required = FluidMixture {
        quantities_liters: [9.0, 1.0, 0.0, 0.0, 0.0, 0.0],
    };
    assert!(
        (suitability(&container(10.0, [9.5, 0.5, 0.0, 0.0, 0.0, 0.0]), &required) - 0.5).abs()
            < 0.00001
    );
    assert_eq!(
        suitability(&container(10.0, [9.0, 1.0, 0.0, 0.0, 0.0, 0.0]), &required),
        1.0
    );
    assert_eq!(
        suitability(&container(10.0, [9.0, 0.0, 0.0, 0.0, 0.0, 0.0]), &required),
        0.0
    );
    assert_eq!(suitability(&container(10.0, [0.0; 6]), &required), 0.0);
}
