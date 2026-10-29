//! Car crash step: source `carscript.Update` calls `DamageStuff(|speed - lspeed|)`.
use super::GarageRuntime;
use crate::conditions::ConditionStore;
use crate::crash::{Candidate, Flags};
use crate::interaction::items::{ItemLocation, ItemStore};
use crate::interaction::wheels::WheelAssemblies;
use crate::reference::{ItemId, ReferenceManifest};
use crate::vehicle::PhysicsWorld;
use crate::world::ObjectKey;

/// Source `carscript.Stuff` speed: forward speed in km/h. The port has no `whlocked`.
pub(super) fn forward_kmh(physics: &PhysicsWorld, key: &ObjectKey) -> f32 {
    physics.snapshot(key).speed_mps.abs() * 3.6
}

/// Source `partconditionscript.state`: the manifest state, else the generated spawn state.
fn condition_state(conditions: &ConditionStore, items: &ItemStore, item: ItemId) -> Option<u8> {
    conditions.item_state(item).or_else(|| {
        let quality = items.get(item)?.generated_condition?;
        Some(((1. - quality) * 4.).round().clamp(0., 4.) as u8)
    })
}

/// True if `root` holds `item` through slots, wheel slots, attachments or storage locks.
fn under(items: &ItemStore, wheels: &WheelAssemblies, mut item: ItemId, root: ItemId) -> bool {
    for _ in 0..items.len() {
        let parent = match items.get(item).map(|i| i.location) {
            Some(ItemLocation::WheelSlot(slot)) => wheels.slot_owner(slot),
            _ => items.parent(item),
        };
        match parent {
            Some(parent) if parent == root => return true,
            Some(parent) => item = parent,
            None => return false,
        }
    }
    false
}

impl GarageRuntime {
    /// Runs the crash rule for every loaded car after the physics step.
    pub(super) fn step_crashes(&mut self, reference: &ReferenceManifest) {
        let driving = self.area.player.driving();
        // Taking the map lets the fall-off borrow the runtime without allocation.
        let vehicles = std::mem::take(&mut self.vehicles);
        for (key, vehicle) in &vehicles {
            let record = vehicle.record(reference);
            let v = (forward_kmh(&self.physics, key) - vehicle.last_speed).abs();
            let driver = driving && self.area.player.seat_vehicle() == Some(key);
            let car = self.area.items.resolve_key(key);
            let condition = record
                .condition
                .and(car)
                .and_then(|car| condition_state(&self.area.conditions, &self.area.items, car));
            let flags = Flags {
                driving: driver,
                player_driving: driver,
                unpaused: !self.paused,
                has_blood: record.blood.is_some(),
                // PR 3 reads the blood wash state.
                blood_on: false,
            };
            let outcome = crate::crash::damage(
                &reference.crash,
                &record.crash,
                v,
                flags,
                condition,
                &mut self.crash.random,
            );
            // PR 3 runs the horn, blood and kill outcomes.
            if let (Some((lerp, count)), Some(car)) = (outcome.fall_off, car) {
                self.candidates(reference, car, lerp);
                self.fall_off(count);
            }
        }
        self.vehicles = vehicles;
    }

    /// Source `currentParts`: parts under the car that pass `WouldFallOff(crashLerp)`.
    fn candidates(&mut self, reference: &ReferenceManifest, car: ItemId, lerp: f32) {
        let (items, conditions, wheels) = (&self.area.items, &self.area.conditions, &self.wheels);
        let passes = |item, category| {
            crate::crash::would_fall_off(
                &reference.crash,
                category,
                condition_state(conditions, items, item),
                lerp,
            )
        };
        let candidates = &mut self.crash.candidates;
        candidates.clear();
        for (item, category) in self.area.interaction.parts.mounted() {
            if under(items, wheels, item, car) && passes(item, category) {
                candidates.push(Candidate::Part(item));
            }
        }
        // Tires are category `gumi`, which never falls off, so only rims are candidates.
        for (slot, rim, category) in wheels.mounted_rims(car) {
            if passes(rim, category) {
                candidates.push(Candidate::Wheel(slot));
            }
        }
    }

    /// Source `FallOFf` on `count` random candidates.
    fn fall_off(&mut self, count: usize) {
        let interaction = &mut self.area.interaction;
        let (items, physics) = (&mut self.area.items, &mut self.physics);
        for _ in 0..count {
            if self.crash.candidates.is_empty() {
                break;
            }
            let len = self.crash.candidates.len() as i32;
            let index = self.crash.random.range_int(0, len) as usize;
            let item = match self.crash.candidates.remove(index) {
                Candidate::Part(item) => {
                    interaction
                        .parts
                        .fall_off(item, items, physics)
                        .expect("mounted part falls off");
                    interaction
                        .parts
                        .detach_mechanism(item, &mut interaction.mechanisms, physics);
                    interaction.sync_parts(items, physics).expect("bound parts");
                    item
                }
                Candidate::Wheel(slot) => self
                    .wheels
                    .fall_off(slot, items, physics)
                    .expect("mounted wheel falls off"),
            };
            interaction.storage.hold_till_pickup(item);
        }
    }
}
