use my_summer_slop::fluids::{FluidContainer, FluidEntry, transfer};

fn container(capacity: f32, quantities: [f32; 6]) -> FluidContainer {
    let entries: Vec<_> = quantities
        .into_iter()
        .enumerate()
        .filter_map(|(fluid_type, amount)| {
            (amount > 0.).then_some(FluidEntry {
                fluid_type: fluid_type as u8,
                amount,
            })
        })
        .collect();
    FluidContainer::new(capacity, &entries)
}

#[test]
fn transfer_conserves_each_type() {
    let mut source = container(20.0, [3.0, 2.0, 1.0, 4.0, 2.0, 1.0]);
    let mut destination = container(8.0, [1.0, 0.0, 1.0, 1.0, 0.0, 0.0]);
    let before: [f32; 6] =
        std::array::from_fn(|i| source.mixture().amount(i) + destination.mixture().amount(i));
    let accepted = transfer(&mut source, &mut destination, 10.0);
    assert!((accepted - 5.0).abs() < 0.00001);
    for (index, amount) in before.iter().enumerate() {
        assert!(
            (source.mixture().amount(index) + destination.mixture().amount(index) - amount).abs()
                < 0.00001
        );
    }
    assert!((destination.mixture().total() - 8.0).abs() < 0.00001);
    assert_eq!(transfer(&mut source, &mut destination, 1.0), 0.0);
    assert_eq!(transfer(&mut source, &mut destination, f32::NAN), 0.0);
    assert_eq!(transfer(&mut source, &mut destination, -1.0), 0.0);
    let mut empty = container(20.0, [0.0; 6]);
    let remaining = source.mixture().total();
    assert_eq!(transfer(&mut source, &mut empty, f32::MAX), remaining);
    assert_eq!(source.mixture().total(), 0.0);
}
