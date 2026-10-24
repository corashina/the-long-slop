//! Source hit sounds: `pickupable.OnCollisionEnter` and `carscript.OnCollisionEnter`.
use super::{AudioAction, AudioKind, AudioState, SurvivalAudioEvent};
use crate::interaction::items::ItemLocation;
use crate::reference::{ColliderId, CollisionAudioReference, ItemId, ReferenceManifest};
use crate::starting_area::StartingArea;
use crate::vehicle::{PhysicsWorld, VehiclePose, to_rapier};
use rapier3d::prelude::{Ball, QueryFilter};
use std::collections::HashMap;

/// Played item hit gain. `None` below the source minimum or for a loose item in an interior.
pub fn item_hit_gain(
    speed: f32,
    gain: f32,
    held: bool,
    interior: bool,
    rules: &CollisionAudioReference,
) -> Option<f32> {
    if interior && !held {
        return None;
    }
    let g = speed * gain * if held { 1. } else { rules.loose_multiplier };
    (g > rules.item_minimum).then_some(g - rules.item_minimum)
}

/// Clip and gain of a car hit from the highest tier below the impact. `sample` is in 0..1.
pub fn car_hit(
    speed: f32,
    gain: f32,
    rules: &CollisionAudioReference,
    sample: f32,
) -> Option<(u32, f32)> {
    let m = speed * gain * rules.car_multiplier;
    let tier = rules.car_tiers.iter().rev().find(|t| m > t.threshold)?;
    let index = ((sample * tier.clips.len() as f32) as usize).min(tier.clips.len() - 1);
    Some((tier.clips[index], tier.fixed_gain.unwrap_or(m)))
}

struct ItemSound {
    clip: Option<u32>,
    gain: f32,
    interior: bool,
}

/// Hit sound data per pickup item and car, with source interior flags.
#[derive(Default)]
pub struct ImpactState {
    items: HashMap<ItemId, ItemSound>,
    vehicle_gains: Vec<f32>,
    wait: f32,
}

#[derive(Clone, Copy)]
enum Side {
    Item(ItemId),
    Car(usize),
    Other,
}

impl ImpactState {
    pub fn bind(&mut self, game: &ReferenceManifest) {
        self.items = game
            .items
            .iter()
            .filter(|i| i.pickup.is_some())
            .map(|i| {
                let sound = ItemSound {
                    clip: i.collision_clip,
                    gain: i.collision_gain,
                    interior: false,
                };
                (i.id, sound)
            })
            .collect();
        self.vehicle_gains = game.vehicles.iter().map(|v| v.collision_gain).collect();
        self.reset();
    }
    pub fn reset(&mut self) {
        self.wait = 0.;
        for sound in self.items.values_mut() {
            sound.interior = false;
        }
    }
    /// Every `period` seconds, test each loose item. Held and stored items keep their flag.
    fn refresh(
        &mut self,
        dt: f32,
        period: f32,
        mut loose: impl FnMut(ItemId) -> Option<[f32; 3]>,
        mut inside: impl FnMut([f32; 3]) -> bool,
    ) {
        self.wait -= dt;
        if self.wait > 0. {
            return;
        }
        self.wait += period;
        for (&id, sound) in &mut self.items {
            if let Some(position) = loose(id) {
                sound.interior = inside(position);
            }
        }
    }
    fn interior(&self, id: ItemId, held: bool) -> bool {
        !held && self.items.get(&id).is_some_and(|s| s.interior)
    }
}

fn side(physics: &PhysicsWorld, handle: crate::vehicle::ColliderHandle) -> Side {
    let Some(collider) = physics.rapier().colliders.get(handle) else {
        return Side::Other;
    };
    if let Some(vehicle) = collider.parent().and_then(|b| physics.vehicle_of(b)) {
        return Side::Car(vehicle);
    }
    match (collider.user_data & u32::MAX as u128) as u32 {
        0 => Side::Other,
        id => Side::Item(ItemId(id - 1)),
    }
}

impl AudioState {
    /// Emits hit sounds for the contacts of the last physics step.
    pub fn hits(&mut self, area: &StartingArea, physics: &PhysicsWorld, dt: f32) {
        let rules = &self.reference.collisions;
        let ball = Ball::new(rules.interior_radius);
        let colliders = &self.colliders;
        self.impacts.refresh(
            dt,
            rules.interior_check_seconds,
            |id| {
                area.items
                    .get(id)
                    .filter(|s| s.location == ItemLocation::World && !s.removed)
                    .map(|s| s.pose.position)
            },
            |position| {
                let pose = to_rapier(VehiclePose {
                    position,
                    rotation: [0., 0., 0., 1.],
                });
                physics
                    .queries(QueryFilter::default())
                    .intersect_shape(pose, &ball)
                    .any(|(_, c)| colliders.contains_key(&ColliderId((c.user_data >> 64) as u32)))
            },
        );
        let held = |id: ItemId| {
            area.items
                .get(id)
                .is_some_and(|s| s.location == ItemLocation::Held)
        };
        for hit in physics.hits() {
            let sides = hit.colliders.map(|h| side(physics, h));
            let position = std::array::from_fn(|i| area.origin[i] + f64::from(hit.point[i]));
            for (this, other) in [(sides[0], sides[1]), (sides[1], sides[0])] {
                let (kind, emitter, clip, gain, pitch) = match this {
                    Side::Item(id) => {
                        let Some(sound) = self.impacts.items.get(&id) else {
                            continue;
                        };
                        let Some(clip) = sound.clip else {
                            continue;
                        };
                        let held = held(id);
                        let interior = self.impacts.interior(id, held);
                        let rules = &self.reference.collisions;
                        let Some(gain) = item_hit_gain(hit.speed, sound.gain, held, interior, rules)
                        else {
                            continue;
                        };
                        (AudioKind::ItemHit, 0, clip, gain, 1.)
                    }
                    Side::Car(vehicle) => {
                        if let Side::Item(id) = other
                            && self.impacts.interior(id, held(id))
                        {
                            continue;
                        }
                        let sample = self.sample();
                        let [low, high] = self.reference.shared.random_pitch;
                        let pitch = low + (high - low) * self.sample();
                        let gain = self.impacts.vehicle_gains[vehicle];
                        let Some((clip, gain)) =
                            car_hit(hit.speed, gain, &self.reference.collisions, sample)
                        else {
                            continue;
                        };
                        (AudioKind::CarHit, vehicle as u32, clip, gain, pitch)
                    }
                    Side::Other => continue,
                };
                self.survival_event(SurvivalAudioEvent {
                    session: self.session,
                    emitter,
                    kind,
                    action: AudioAction::Start,
                    clip,
                    gain,
                    pitch,
                    position,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reference::CarTierReference;

    fn rules() -> CollisionAudioReference {
        let tier = |threshold, clip, fixed_gain| CarTierReference {
            threshold,
            clips: vec![clip, clip + 10],
            fixed_gain,
        };
        CollisionAudioReference {
            car_tiers: [
                tier(0.05, 1, Some(1.)),
                tier(0.2, 2, None),
                tier(0.5, 3, None),
                tier(0.7, 4, None),
            ],
            interior_check_seconds: 10.,
            interior_radius: 0.1,
            loose_multiplier: 0.3,
            item_minimum: 0.075,
            car_multiplier: 0.3,
            method: Vec::new(),
        }
    }

    #[test]
    fn item_gain_follows_source_rule() {
        let r = rules();
        assert_eq!(item_hit_gain(0.5, 0.5, true, false, &r), Some(0.25 - 0.075));
        assert!((item_hit_gain(2., 0.5, false, false, &r).unwrap() - 0.225).abs() < 1e-6);
        // Loose: 0.5 * 0.5 * 0.3 = 0.075 is not above the minimum.
        assert_eq!(item_hit_gain(0.5, 0.5, false, false, &r), None);
        assert_eq!(item_hit_gain(2., 0.5, false, true, &r), None);
        assert!(item_hit_gain(2., 0.5, true, true, &r).is_some());
    }

    #[test]
    fn car_hit_picks_highest_tier_above_threshold() {
        let r = rules();
        let at = |m: f32, sample| car_hit(m / 0.3, 1., &r, sample);
        assert_eq!(at(0.05, 0.), None);
        assert_eq!(at(0.06, 0.), Some((1, 1.)));
        assert_eq!(at(0.06, 0.99), Some((11, 1.)));
        for (threshold, clip) in [(0.2, 1), (0.5, 2), (0.7, 3)] {
            assert_eq!(at(threshold, 0.).unwrap().0, clip, "at {threshold}");
        }
        for (threshold, clip) in [(0.2, 2), (0.5, 3), (0.7, 4)] {
            let (c, gain) = at(threshold + 0.01, 0.).unwrap();
            assert_eq!(c, clip);
            assert!((gain - (threshold + 0.01)).abs() < 1e-5);
        }
    }

    #[test]
    fn held_items_skip_interior_test() {
        let mut state = ImpactState::default();
        for id in [1, 2] {
            let sound = ItemSound {
                clip: Some(1),
                gain: 1.,
                interior: false,
            };
            state.items.insert(ItemId(id), sound);
        }
        // Item 2 is held: no loose position, so its flag stays.
        let loose = |id: ItemId| (id.0 == 1).then_some([0.; 3]);
        state.refresh(0.02, 10., loose, |_| true);
        assert!(state.interior(ItemId(1), false));
        assert!(!state.interior(ItemId(2), false));
        assert!(!state.interior(ItemId(1), true));
        state.refresh(0.02, 10., loose, |_| false);
        assert!(state.interior(ItemId(1), false), "test waits for the period");
    }
}
