//! Filling a tank from a held container and spilling from open, tilted tanks.

use super::*;

impl InteractionState {
    pub(super) fn fill(
        &mut self,
        target: Option<AimTarget>,
        items: &mut ItemStore,
        dt: f32,
    ) -> Option<InteractionAction> {
        let held = self.held.as_ref()?;
        let target = target?;
        let opening = self
            .openings
            .iter()
            .find(|opening| opening.reference.cap == target.item && opening.valve > 0.0)?;
        let supply = self.supplies.contains(&opening.reference.tank);
        let (source_id, destination, flow) = if supply {
            if !self.openings.iter().any(|destination| {
                destination.reference.tank == held.item && destination.valve > 0.0
            }) {
                (held.item, held.item, 0.0)
            } else {
                (
                    opening.reference.tank,
                    held.item,
                    opening.reference.flow_liters_per_second * opening.valve,
                )
            }
        } else {
            (
                held.item,
                opening.reference.tank,
                opening.reference.flow_liters_per_second * opening.valve,
            )
        };
        let source_allowed = self
            .tank_rules
            .get(&source_id)
            .is_some_and(|rules| !rules.no_fill_from);
        let destination_rules = self.tank_rules.get(&destination);
        if source_id != destination
            && source_allowed
            && destination_rules.is_some_and(|rules| !rules.no_fill_this)
            && let Some((source, destination_state)) = items.get_pair_mut(source_id, destination)
            && let (Some(source), Some(destination_state)) =
                (source.fluid.as_mut(), destination_state.fluid.as_mut())
        {
            let mut amount = flow * dt;
            let source_total = source.mixture.total();
            let destination_total = destination_state.mixture.total();
            if destination_rules.is_some_and(|rules| rules.slow_oil)
                && source_total > 0.0
                && source.mixture.amount(1) / source_total > 0.25
                && destination_total > 0.05
                && destination_state.mixture.amount(1) / destination_total < 1.0
            {
                amount = (amount * destination_total * self.slow_oil_modifier)
                    .max(0.0001)
                    .min(amount);
            }
            let liters = transfer(source, destination_state, amount);
            if liters > 0.0 {
                // The source cap pours. A held source pours from its open cap.
                let pouring = if supply {
                    Some(opening.reference.id)
                } else {
                    self.openings
                        .iter()
                        .find(|o| o.reference.tank == held.item && o.valve > 0.0)
                        .map(|o| o.reference.id)
                };
                if let Some(opening) = self
                    .openings
                    .iter_mut()
                    .find(|opening| Some(opening.reference.id) == pouring)
                {
                    opening.pour = flow;
                }
                return Some(InteractionAction::Fill {
                    source: source_id,
                    destination,
                    liters,
                });
            }
        }
        None
    }

    pub(super) fn spill(&mut self, items: &mut ItemStore, filling: Option<ItemId>, dt: f32) {
        for opening in &mut self.openings {
            if opening.valve <= 0.0
                || opening.reference.spill_disabled
                || opening.reference.tube_connected
                || filling == Some(opening.reference.tank)
            {
                continue;
            }
            let Some(item) = items.get_mut(opening.reference.tank) else {
                continue;
            };
            let forward = (to_rapier(item.pose) * opening.local_pose).rotation * opening.tilt_axis;
            let angle = forward.dot(Vector::Y).clamp(-1.0, 1.0).acos().to_degrees();
            if let Some(fluid) = item.fluid.as_mut() {
                let fill = (fluid.mixture.total() / fluid.capacity_liters).clamp(0.0, 1.0);
                let threshold = 100.0 + (opening.reference.tilt_degrees - 100.0) * fill;
                let factor = if opening.reference.angle_disabled {
                    1.0
                } else {
                    inverse_lerp(threshold, opening.reference.tilt_degrees + 90.0, angle)
                        .max(inverse_lerp(90.0, 180.0, angle))
                };
                if !opening.reference.angle_disabled && angle <= threshold {
                    continue;
                }
                let rate = factor * opening.valve * opening.reference.flow_liters_per_second;
                if spill(fluid, rate * dt) > 0.0 {
                    opening.pour = rate;
                }
            }
        }
    }
}
