//! Six-fluid containers owned by the item store.

use crate::reference::TankReference;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FluidMixture {
    pub quantities_liters: [f32; 6],
}

impl FluidMixture {
    pub fn total(&self) -> f32 {
        self.quantities_liters.iter().sum()
    }

    pub fn amount(&self, index: usize) -> f32 {
        self.quantities_liters[index]
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FluidContainer {
    pub capacity_liters: f32,
    pub mixture: FluidMixture,
}

impl From<&TankReference> for FluidContainer {
    fn from(reference: &TankReference) -> Self {
        Self {
            capacity_liters: reference.capacity_liters,
            mixture: FluidMixture {
                quantities_liters: reference.quantities_liters,
            },
        }
    }
}

pub struct VehicleFluids<'a> {
    pub fuel: &'a mut FluidContainer,
    pub sump: &'a mut FluidContainer,
    pub coolant: &'a mut FluidContainer,
}

pub fn transfer(source: &mut FluidContainer, destination: &mut FluidContainer, liters: f32) -> f32 {
    let total = source.mixture.total();
    if !liters.is_finite() || liters <= 0.0 || total <= 0.0 {
        return 0.0;
    }
    let capacity = (destination.capacity_liters - destination.mixture.total()).max(0.0);
    let accepted = liters.min(total).min(capacity);
    let ratio = accepted / total;
    for index in 0..6 {
        let amount = source.mixture.quantities_liters[index] * ratio;
        source.mixture.quantities_liters[index] -= amount;
        destination.mixture.quantities_liters[index] += amount;
    }
    accepted
}

pub fn spill(container: &mut FluidContainer, liters: f32) -> f32 {
    let total = container.mixture.total();
    if !liters.is_finite() || liters <= 0.0 || total <= 0.0 {
        return 0.0;
    }
    let removed = liters.min(total);
    let ratio = removed / total;
    for amount in &mut container.mixture.quantities_liters {
        *amount -= *amount * ratio;
    }
    removed
}

/// The source `canRun` takes the smallest supplied-to-required fluid fraction.
pub fn suitability(container: &FluidContainer, required: &FluidMixture) -> f32 {
    let total = container.mixture.total();
    let required_total = required.total();
    if total <= 0.0 || required_total <= 0.0 {
        return 0.0;
    }
    let mut result: f32 = 1.0;
    for index in 0..6 {
        let amount = required.amount(index);
        if amount > 0.0 {
            result =
                result.min((container.mixture.amount(index) / total) / (amount / required_total));
        }
    }
    result.clamp(0.0, 1.0)
}
