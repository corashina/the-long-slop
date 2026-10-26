//! Source hit sounds: `pickupable.OnCollisionEnter` and `carscript.OnCollisionEnter`.
use super::{AudioAction, AudioKind, AudioState, SurvivalAudioEvent};
use crate::interaction::items::{ItemLocation, ItemState};
use crate::reference::{ColliderId, CollisionAudioReference, ItemId, ReferenceManifest};
use crate::starting_area::StartingArea;
use crate::vehicle::{PhysicsWorld, VehiclePose, to_rapier};
use rapier3d::prelude::{Ball, QueryFilter};
use std::collections::HashSet;

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

/// Clip and gain of an item hit. Items keep their source hit sound in the item store.
fn item_hit(
    state: &ItemState,
    speed: f32,
    interior: bool,
    rules: &CollisionAudioReference,
) -> Option<(u32, f32)> {
    let sound = state.hit_sound?;
    let held = state.location == ItemLocation::Held;
    let gain = item_hit_gain(speed, sound.gain, held, interior, rules)?;
    Some((sound.clip, gain))
}

/// Car hit gains and the source interior flags of pickup items.
#[derive(Default)]
pub struct ImpactState {
    /// Items that the last interior test found in an interior.
    interior: HashSet<ItemId>,
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
        self.interior = HashSet::with_capacity(256);
        self.vehicle_gains = game.vehicles.iter().map(|v| v.collision_gain).collect();
        self.reset();
    }
    pub fn reset(&mut self) {
        self.wait = 0.;
        self.interior.clear();
    }
    /// Every `period` seconds, test each loose pickup. Held and stored items keep their flag.
    fn refresh(
        &mut self,
        dt: f32,
        period: f32,
        loose: impl Iterator<Item = (ItemId, [f32; 3])>,
        exists: impl Fn(ItemId) -> bool,
        mut inside: impl FnMut([f32; 3]) -> bool,
    ) {
        self.wait -= dt;
        if self.wait > 0. {
            return;
        }
        self.wait += period;
        self.interior.retain(|&id| exists(id));
        for (id, position) in loose {
            if inside(position) {
                self.interior.insert(id);
            } else {
                self.interior.remove(&id);
            }
        }
    }
    fn interior(&self, id: ItemId, held: bool) -> bool {
        !held && self.interior.contains(&id)
    }
}

fn side(physics: &PhysicsWorld, handle: crate::vehicle::ColliderHandle) -> Side {
    let Some(collider) = physics.rapier().colliders.get(handle) else {
        return Side::Other;
    };
    if let Some(vehicle) = collider
        .parent()
        .and_then(|b| physics.vehicle_of(b))
        .and_then(crate::vehicle::garage_vehicle_index)
    {
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
            area.items
                .iter()
                .filter(|s| {
                    s.location == ItemLocation::World
                        && !s.removed
                        && area.interaction.pickup_rule(s.id).is_some()
                })
                .map(|s| (s.id, s.pose.position)),
            |id| area.items.get(id).is_some(),
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
                        let Some(state) = area.items.get(id) else {
                            continue;
                        };
                        let interior = self.impacts.interior.contains(&id);
                        let rules = &self.reference.collisions;
                        let Some((clip, gain)) = item_hit(state, hit.speed, interior, rules) else {
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
        // Item 2 is held: no loose position, so its flag stays.
        let loose = || [(ItemId(1), [0.; 3])].into_iter();
        state.interior.insert(ItemId(3));
        state.refresh(0.02, 10., loose(), |id| id.0 != 3, |_| true);
        assert!(state.interior(ItemId(1), false));
        assert!(!state.interior(ItemId(2), false));
        assert!(!state.interior(ItemId(1), true));
        assert!(
            !state.interior(ItemId(3), false),
            "removed items lose their flag"
        );
        state.refresh(0.02, 10., loose(), |_| true, |_| false);
        assert!(
            state.interior(ItemId(1), false),
            "test waits for the period"
        );
    }

    #[test]
    fn generated_item_hit_uses_its_sound() {
        use crate::interaction::items::HitSound;
        let mut item = ItemState {
            id: ItemId(900),
            body: None,
            inactive_pose: None,
            location: ItemLocation::World,
            pose: VehiclePose {
                position: [0.; 3],
                rotation: [0., 0., 0., 1.],
            },
            local_pose: VehiclePose {
                position: [0.; 3],
                rotation: [0., 0., 0., 1.],
            },
            generated_condition: Some(1.),
            temperature: None,
            fluid: None,
            food: None,
            hit_sound: Some(HitSound { clip: 7, gain: 0.5 }),
            removed: false,
            pending_attachment: false,
            buried: false,
        };
        let r = rules();
        let (clip, gain) = item_hit(&item, 2., false, &r).unwrap();
        assert_eq!(clip, 7);
        assert!((gain - 0.225).abs() < 1e-6);
        assert_eq!(item_hit(&item, 2., true, &r), None);
        item.location = ItemLocation::Held;
        assert!(item_hit(&item, 2., true, &r).is_some());
        item.hit_sound = None;
        assert_eq!(item_hit(&item, 2., false, &r), None);
    }
}
