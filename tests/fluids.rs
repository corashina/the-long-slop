use the_long_slop::fluids::{FluidContainer, FluidEntry, spill, transfer};

fn tank(capacity: f32, entries: &[(u8, f32)]) -> FluidContainer {
    let entries: Vec<_> = entries
        .iter()
        .map(|&(fluid_type, amount)| FluidEntry { fluid_type, amount })
        .collect();
    FluidContainer::new(capacity, &entries)
}

fn assert_entries(tank: &FluidContainer, expected: &[(u8, f32)]) {
    let entries: Vec<_> = tank.entries().collect();
    assert_eq!(entries.len(), expected.len());
    for (entry, &(fluid_type, amount)) in entries.iter().zip(expected) {
        assert_eq!(entry.fluid_type, fluid_type);
        assert!(entry.amount.is_finite());
        assert!(
            (entry.amount - amount).abs() <= 1e-6,
            "{entry:?} != {amount}"
        );
    }
}

#[test]
fn g04_ordered_transfer() {
    let mut source = tank(10., &[(0, 2.), (3, 2.)]);
    let mut receiver = tank(10., &[]);
    let moved = transfer(&mut source, &mut receiver, 2.);
    assert!((moved - 7. / 3.).abs() <= 1e-6);
    assert_entries(&source, &[(0, 1.), (3, 2. / 3.)]);
    assert_entries(&receiver, &[(0, 1.), (3, 4. / 3.)]);
}

#[test]
fn g04_receiver_capacity() {
    let mut source = tank(10., &[(0, 2.), (3, 2.)]);
    let mut receiver = tank(1.5, &[]);
    assert_eq!(transfer(&mut source, &mut receiver, 2.), 1.5);
    assert_entries(&source, &[(0, 1.), (3, 1.5)]);
    assert_entries(&receiver, &[(0, 1.), (3, 0.5)]);
    assert_eq!(transfer(&mut source, &mut receiver, 1.), 0.);
    assert_entries(&source, &[(0, 1.), (3, 1.5)]);
    assert_entries(&receiver, &[(0, 1.), (3, 0.5)]);
}

#[test]
fn g04_spill_cleanup() {
    let mut source = tank(10., &[(0, 1.), (1, 0.), (3, 1.)]);
    assert_eq!(spill(&mut source, 2.), 3.);
    assert_entries(&source, &[(1, 0.), (3, -1.)]);
}

#[test]
fn g04_nonfinite_spill() {
    let mut source = tank(10., &[(0, 1.), (1, 0.)]);
    assert!(spill(&mut source, 1.).is_nan());
    let entries: Vec<_> = source.entries().collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].fluid_type, 1);
    assert!(source.mixture().amount(1).is_nan());
}

#[test]
fn invalid_requests_preserve_entries() {
    for amount in [f32::NAN, f32::INFINITY, -1., 0.] {
        let mut source = tank(10., &[(0, 2.), (3, 2.)]);
        let mut receiver = tank(10., &[]);
        assert_eq!(transfer(&mut source, &mut receiver, amount), 0.);
        assert_eq!(spill(&mut source, amount), 0.);
        assert_entries(&source, &[(0, 2.), (3, 2.)]);
        assert_entries(&receiver, &[]);
    }
}
