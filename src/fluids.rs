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
    order: [u8; 6],
    order_len: u8,
}

impl FluidContainer {
    pub fn new(capacity_liters: f32, mixture: FluidMixture) -> Self {
        let mut container = Self {
            capacity_liters,
            mixture,
            order: [0; 6],
            order_len: 0,
        };
        container.track_positive_types();
        container
    }

    fn remember(&mut self, index: usize) {
        if self.order[..self.order_len as usize].contains(&(index as u8)) {
            return;
        }
        self.order[self.order_len as usize] = index as u8;
        self.order_len += 1;
    }

    fn track_positive_types(&mut self) {
        for index in 0..6 {
            if self.mixture.quantities_liters[index] > 0. {
                self.remember(index);
            }
        }
    }

    pub(crate) fn record_type(&mut self, index: usize) {
        if self.mixture.quantities_liters[index] > 0. {
            self.remember(index);
        }
    }

    fn change_one(&mut self, amount: f32, index: usize) -> f32 {
        let free = self.capacity_liters - self.mixture.total();
        let changed = if amount < f32::MIN {
            f32::MIN
        } else if amount > free {
            free
        } else {
            amount
        };
        if self.order[..self.order_len as usize].contains(&(index as u8)) {
            self.mixture.quantities_liters[index] += changed;
            return changed;
        }
        if changed <= 0. {
            return 0.;
        }
        self.remember(index);
        self.mixture.quantities_liters[index] = changed;
        changed
    }

    fn cleanup_one(&mut self) {
        if let Some(position) = self.order[..self.order_len as usize]
            .iter()
            .position(|&index| self.mixture.quantities_liters[index as usize] <= 0.)
        {
            let index = self.order[position] as usize;
            self.mixture.quantities_liters[index] = 0.;
            self.order
                .copy_within(position + 1..self.order_len as usize, position);
            self.order_len -= 1;
        }
    }
}

impl From<&TankReference> for FluidContainer {
    fn from(reference: &TankReference) -> Self {
        Self::new(
            reference.capacity_liters,
            FluidMixture {
                quantities_liters: reference.quantities_liters,
            },
        )
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
    source.track_positive_types();
    for index in 0..6 {
        let amount = source.mixture.quantities_liters[index] * ratio;
        source.mixture.quantities_liters[index] -= amount;
        destination.mixture.quantities_liters[index] += amount;
    }
    for position in 0..source.order_len as usize {
        let index = source.order[position] as usize;
        destination.record_type(index);
    }
    source.cleanup_one();
    destination.cleanup_one();
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
    container.cleanup_one();
    removed
}

/// Source drink pour uses the remaining source fraction for each fluid.
pub fn drink_transfer(
    source: &mut FluidContainer,
    destination: &mut FluidContainer,
    liters: f32,
    filter_water: bool,
    filter_blood: bool,
) -> f32 {
    if !liters.is_finite() || liters <= 0. {
        return 0.;
    }
    source.track_positive_types();
    destination.track_positive_types();
    let mut debited = 0.;
    for position in 0..source.order_len as usize {
        let index = source.order[position] as usize;
        let total = source.mixture.total();
        let fraction = source.mixture.quantities_liters[index] / total;
        let source_before = source.mixture.quantities_liters[index].max(0.);
        let accepted = destination.change_one(liters * fraction, index);
        source.change_one(-accepted, index);
        debited += accepted.max(0.).min(source_before);
        if (index == 3 && filter_water) || (index == 4 && filter_blood) {
            destination.change_one(-liters * fraction, index);
        }
    }
    destination.cleanup_one();
    source.cleanup_one();
    debited
}

/// Particle contact adds emitted fluid to the bladder without another source debit.
pub fn contact_drink_transfer(
    destination: &mut FluidContainer,
    source: &FluidContainer,
    fractions: [f32; 6],
    liters: f32,
    filter_water: bool,
    filter_blood: bool,
) {
    if !liters.is_finite() || liters <= 0. {
        return;
    }
    destination.track_positive_types();
    for position in 0..source.order_len as usize {
        let index = source.order[position] as usize;
        let fraction = fractions[index];
        let amount = liters * fraction;
        destination.change_one(amount, index);
        if (index == 3 && filter_water) || (index == 4 && filter_blood) {
            destination.change_one(-amount, index);
        }
    }
    destination.cleanup_one();
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
