//! Source boxes: spawnInBoxScript content and breakablescript breaks of generated boxes.
//! Content rolls at generation. It publishes only after its box breaks, at the box pose.
use super::WorldStream;
use crate::interaction::items::{ItemLocation, ItemStore};
use crate::interaction::storage::StorageLocks;
use crate::reference::{
    BoxBreakRules, ItemId, ReferenceError, ReferenceManifest, SourceId, TriggerRole,
};
use crate::vehicle::PhysicsWorld;
use crate::world::{AbsolutePose, GeneratedId, ObjectKey, Placement};
use rapier3d::math::{Rotation, Vector};
use rapier3d::prelude::QueryFilter;
use serde::{Deserialize, Serialize};

/// One broken box. `origin` is its generated pose and `pose` its pose at the break.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoxRelease {
    pub id: GeneratedId,
    pub origin: AbsolutePose,
    pub pose: AbsolutePose,
}

/// Broken boxes. Box content keys are `box id + (spawnInBoxScript, copy)`.
#[derive(Default)]
pub(crate) struct BoxReleases {
    spawner: Option<SourceId>,
    released: Vec<BoxRelease>,
}
impl BoxReleases {
    pub fn new(spawner: SourceId) -> Self {
        Self {
            spawner: Some(spawner),
            released: Vec::new(),
        }
    }
    /// True when `child` is a content copy of the box `parent`.
    fn content_of(&self, parent: &GeneratedId, child: &GeneratedId) -> bool {
        child.layer == parent.layer
            && child.generator == parent.generator
            && child.unit == parent.unit
            && child.ordinal == parent.ordinal
            && child.children.len() == parent.children.len() + 1
            && child.children.starts_with(&parent.children)
            && child.children.last().map(|(s, _)| s) == self.spawner.as_ref()
    }
    fn is_content(&self, id: &GeneratedId) -> bool {
        id.children.last().map(|(s, _)| s) == self.spawner.as_ref()
    }
    fn release_of(&self, id: &GeneratedId) -> Option<&BoxRelease> {
        self.released.iter().find(|r| self.content_of(&r.id, id))
    }
    /// Source spawned flag: content of an intact box does not publish.
    pub fn withheld(&self, id: &GeneratedId) -> bool {
        self.is_content(id) && self.release_of(id).is_none()
    }
    /// Returns false when the box is already broken, so content spawns once.
    pub fn release(&mut self, release: BoxRelease) -> bool {
        if self.released.iter().any(|r| r.id == release.id) {
            return false;
        }
        self.released.push(release);
        true
    }
    /// Moves released content from the generated box pose to the break pose.
    pub fn place(&self, placement: &mut Placement) {
        if let Some(release) = self.release_of(&placement.id) {
            relocate(placement, &release.origin, &release.pose);
        }
    }
    pub fn released(&self) -> &[BoxRelease] {
        &self.released
    }
    pub fn retain(&mut self, keep: impl FnMut(&BoxRelease) -> bool) {
        self.released.retain(keep);
    }
    pub fn restore(&mut self, released: &[BoxRelease]) {
        self.released.clear();
        for release in released {
            self.release(release.clone());
        }
    }
}

fn rotation(pose: &AbsolutePose) -> Rotation {
    Rotation::from_array(pose.rotation)
}
/// Applies `pose * inverse(origin)` to a placement and its bounds.
pub(crate) fn relocate(placement: &mut Placement, origin: &AbsolutePose, pose: &AbsolutePose) {
    let turn = rotation(pose) * rotation(origin).inverse();
    let map = |p: [f64; 3]| -> [f64; 3] {
        let local = Vector::new(
            (p[0] - origin.position[0]) as f32,
            (p[1] - origin.position[1]) as f32,
            (p[2] - origin.position[2]) as f32,
        );
        let moved = turn * local;
        [
            pose.position[0] + moved.x as f64,
            pose.position[1] + moved.y as f64,
            pose.position[2] + moved.z as f64,
        ]
    };
    let [min, max] = [placement.bounds.min, placement.bounds.max];
    let mut bounds = crate::world::WorldBounds {
        min: [f64::INFINITY; 3],
        max: [f64::NEG_INFINITY; 3],
    };
    for corner in 0..8 {
        let p = map(std::array::from_fn(|a| {
            if corner & (1 << a) == 0 {
                min[a]
            } else {
                max[a]
            }
        }));
        bounds.min = std::array::from_fn(|a| bounds.min[a].min(p[a]));
        bounds.max = std::array::from_fn(|a| bounds.max[a].max(p[a]));
    }
    placement.pose.position = map(placement.pose.position);
    placement.pose.rotation = (turn * Rotation::from_array(placement.pose.rotation))
        .normalize()
        .to_array();
    placement.bounds = bounds;
}

/// Source breakablescript.OnCollisionEnter: the force passed to TryBreak.
pub(crate) fn collision_force(rules: &BoxBreakRules, relative_speed: f32) -> Option<f32> {
    if rules.no_collider
        || rules.only_decrease_above && relative_speed < rules.decrease_above_health
    {
        return None;
    }
    Some(relative_speed)
}
/// Source breakablescript.TryBreak(float). True breaks the box.
pub(crate) fn try_break(rules: &BoxBreakRules, health: &mut f32, force: f32) -> bool {
    if force >= *health {
        return true;
    }
    if rules.decrease_after_attack {
        *health -= force;
    }
    false
}

/// Bound generated box and its current breakablescript health.
pub(crate) struct BoxBinding {
    item: ItemId,
    health: f32,
}
#[derive(Default)]
pub(crate) struct BoxRuntime {
    bindings: Vec<BoxBinding>,
    scan: Option<(u64, u64)>,
    changes: Option<u64>,
    broken: Vec<ItemId>,
}

impl WorldStream {
    fn box_rules<'a>(&self, reference: &'a ReferenceManifest) -> &'a BoxBreakRules {
        &reference.acquisition.r#box.break_rules
    }
    /// Box content that waits for its box to break.
    pub(super) fn box_content_withheld(&self, id: &GeneratedId) -> bool {
        self.contents.boxes.withheld(id)
    }
    fn refresh_boxes(&mut self, reference: &ReferenceManifest, items: &ItemStore) {
        if self.boxes.changes != Some(self.changes.revision()) {
            // A forgotten box removal respawns the box and withholds its content again.
            let changes = &self.changes;
            self.contents.boxes.retain(|r| {
                changes
                    .get(ObjectKey::Generated(r.id.clone()))
                    .is_some_and(|c| c.snapshot.is_none())
            });
            self.boxes.changes = Some(self.changes.revision());
        }
        let scan = (self.contents.active_revision(), items.membership_revision());
        if self.boxes.scan == Some(scan) {
            return;
        }
        self.boxes.scan = Some(scan);
        let prototype = &reference.acquisition.r#box.prototype.0;
        let health = self.box_rules(reference).health;
        let active = &self.contents.active;
        let bound = |item: ItemId| {
            items.get(item).is_some_and(|s| !s.removed)
                && active
                    .iter()
                    .any(|c| c.item == Some(item) && &c.content.placement.prototype == prototype)
        };
        self.boxes.bindings.retain(|b| bound(b.item));
        for content in active {
            let Some(item) = content.item else { continue };
            if bound(item) && !self.boxes.bindings.iter().any(|b| b.item == item) {
                self.boxes.bindings.push(BoxBinding { item, health });
            }
        }
    }
    /// Run after the physics step. Queues boxes that break this step.
    pub fn step_boxes(
        &mut self,
        reference: &ReferenceManifest,
        items: &ItemStore,
        physics: &PhysicsWorld,
        dt: f32,
    ) {
        self.refresh_boxes(reference, items);
        if self.boxes.bindings.is_empty() {
            return;
        }
        let rules = self.box_rules(reference);
        let item_of = |handle| {
            physics
                .rapier()
                .colliders
                .get(handle)
                .map(|c| (c.user_data & u32::MAX as u128) as u32)
                .filter(|id| *id != 0)
                .map(|id| ItemId(id - 1))
        };
        for hit in physics.hits() {
            for handle in hit.colliders {
                let Some(item) = item_of(handle) else {
                    continue;
                };
                let Some(binding) = self.boxes.bindings.iter_mut().find(|b| b.item == item) else {
                    continue;
                };
                if let Some(force) = collision_force(rules, hit.speed)
                    && try_break(rules, &mut binding.health, force)
                    && !self.boxes.broken.contains(&item)
                {
                    self.boxes.broken.push(item);
                }
            }
        }
        // Source playerdeathtriggerscript.OnTriggerStay. Unity skips sleeping bodies.
        if self.trigger_roles.is_empty() {
            return;
        }
        let bodies = &physics.rapier().bodies;
        let filter = QueryFilter::default().exclude_solids();
        for binding in &mut self.boxes.bindings {
            let Some(handle) = items.get(binding.item).and_then(|s| s.body) else {
                continue;
            };
            let Some(body) = bodies.get(handle) else {
                continue;
            };
            if !body.is_enabled() || body.is_sleeping() {
                continue;
            }
            for &collider in body.colliders() {
                let collider = &physics.rapier().colliders[collider];
                if !collider.is_enabled() || collider.is_sensor() {
                    continue;
                }
                for (trigger, other) in physics
                    .queries(filter)
                    .intersect_shape(*collider.position(), collider.shape())
                {
                    if let Some(TriggerRole::Hazard {
                        damage_breakable, ..
                    }) = self.trigger_roles.get(trigger, other)
                        && try_break(rules, &mut binding.health, damage_breakable * dt)
                        && !self.boxes.broken.contains(&binding.item)
                    {
                        self.boxes.broken.push(binding.item);
                    }
                }
            }
        }
    }
    /// Next box queued by `step_boxes`. Drop it from the hand before `break_box`.
    pub fn take_broken_box(&mut self) -> Option<ItemId> {
        self.boxes.broken.pop()
    }
    /// Source breakablescript.Break and mountStuff.Unstore: remove the box, release its content.
    /// Items locked in the box unlock at their current pose first.
    pub fn break_box(
        &mut self,
        item: ItemId,
        items: &mut ItemStore,
        storage: &mut StorageLocks,
        physics: &mut PhysicsWorld,
    ) -> Result<Option<BoxRelease>, ReferenceError> {
        let Some(state) = items.get(item) else {
            return Ok(None);
        };
        if state.removed || state.location != ItemLocation::World {
            return Ok(None);
        }
        let Some(body) = state.body else {
            return Ok(None);
        };
        let Some(content) = self.contents.active.iter().find(|c| c.item == Some(item)) else {
            return Ok(None);
        };
        let release = BoxRelease {
            id: content.content.placement.id.clone(),
            origin: content.content.placement.pose,
            pose: AbsolutePose {
                position: self.origin.absolute(state.pose.position),
                rotation: state.pose.rotation,
            },
        };
        if !self.contents.boxes.release(release.clone()) {
            return Ok(None);
        }
        storage.unbind_owner(item, |_, _| false, items, physics)?;
        self.changes.remove(items.object_key(item));
        let contents = &mut self.contents;
        for content in &mut contents.active {
            if contents
                .boxes
                .content_of(&release.id, &content.content.placement.id)
            {
                relocate(
                    &mut content.content.placement,
                    &release.origin,
                    &release.pose,
                );
            }
        }
        physics.edit_body(body, |body| body.set_enabled(false));
        physics.refresh_queries();
        let state = items.get_mut(item).unwrap();
        state.removed = true;
        items.mark_dirty(item);
        self.boxes.bindings.retain(|b| b.item != item);
        self.boxes.scan = None;
        self.boxes.changes = Some(self.changes.revision());
        Ok(Some(release))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::WorldLayer;
    use crate::world::spawn_selection::{box_count, pick_weighted};
    use crate::world::{UnityRandom, WorldBounds};

    /// acquisition.box.break_rules of item 62 (box01).
    fn rules() -> BoxBreakRules {
        BoxBreakRules {
            health: 7.,
            decrease_after_attack: false,
            only_decrease_above: false,
            decrease_above_health: 0.,
            mass_modifier: 0.,
            only_velocity: false,
            no_trigger: true,
            no_collider: false,
            no_destroy: false,
            no_gib: false,
            destroyed: false,
        }
    }
    fn source(path_id: i64) -> SourceId {
        SourceId {
            file: "sharedassets1.assets".into(),
            path_id,
        }
    }
    fn box_id() -> GeneratedId {
        GeneratedId {
            layer: WorldLayer::Building,
            generator: source(1),
            unit: [3, 4],
            ordinal: 5,
            children: vec![(source(2), 0)],
        }
    }
    fn content(copy: u32) -> GeneratedId {
        let mut id = box_id();
        id.children.push((source(158204), copy));
        id
    }
    fn pose(position: [f64; 3], yaw: f32) -> AbsolutePose {
        AbsolutePose {
            position,
            rotation: Rotation::from_rotation_y(yaw).to_array(),
        }
    }

    #[test]
    fn break_threshold_matches_source_health() {
        let rules = rules();
        let mut health = rules.health;
        assert_eq!(collision_force(&rules, 6.99), Some(6.99));
        assert!(!try_break(&rules, &mut health, 6.99));
        assert_eq!(health, 7., "decreaseAfterAttack is off");
        assert!(try_break(&rules, &mut health, 7.));
        // A hazard with damageBreakable d breaks the box when d * dt reaches health.
        assert!(!try_break(&rules, &mut health, 349. * 0.02));
        assert!(try_break(&rules, &mut health, 350. * 0.02));
        let mut decreasing = BoxBreakRules {
            decrease_after_attack: true,
            ..rules.clone()
        };
        assert!(!try_break(&decreasing, &mut health, 4.));
        assert!(try_break(&decreasing, &mut health, 3.));
        decreasing.only_decrease_above = true;
        decreasing.decrease_above_health = 5.;
        assert_eq!(collision_force(&decreasing, 4.), None);
        decreasing.no_collider = true;
        assert_eq!(collision_force(&decreasing, 40.), None);
    }

    #[test]
    fn content_spawns_once_and_survives_save() {
        let mut boxes = BoxReleases::new(source(158204));
        assert!(boxes.withheld(&content(0)) && boxes.withheld(&content(1)));
        assert!(!boxes.withheld(&box_id()), "the box itself publishes");
        let release = BoxRelease {
            id: box_id(),
            origin: pose([10., 0., 10.], 0.),
            pose: pose([20., 1., 10.], std::f32::consts::FRAC_PI_2),
        };
        assert!(boxes.release(release.clone()));
        assert!(!boxes.release(release.clone()), "a box breaks once");
        assert_eq!(boxes.released().len(), 1);
        assert!(!boxes.withheld(&content(0)) && !boxes.withheld(&content(1)));
        let mut other = box_id();
        other.ordinal = 6;
        other.children.push((source(158204), 0));
        assert!(boxes.withheld(&other));

        let json = serde_json::to_string(boxes.released()).unwrap();
        let mut loaded = BoxReleases::new(source(158204));
        loaded.restore(&serde_json::from_str::<Vec<BoxRelease>>(&json).unwrap());
        assert_eq!(loaded.released(), boxes.released());
        assert!(!loaded.withheld(&content(0)));
        assert!(
            !loaded.release(release),
            "a loaded broken box does not spawn again"
        );
    }

    #[test]
    fn released_content_follows_the_box() {
        let mut boxes = BoxReleases::new(source(158204));
        boxes.release(BoxRelease {
            id: box_id(),
            origin: pose([10., 0., 10.], 0.),
            pose: pose([20., 1., 10.], std::f32::consts::FRAC_PI_2),
        });
        let mut placement = Placement {
            id: content(1),
            prototype: source(9),
            pose: pose([10.15, 0., 10.], 0.),
            scale: [1.; 3],
            bounds: WorldBounds {
                min: [10.05, -0.1, 9.9],
                max: [10.25, 0.1, 10.1],
            },
            object_seed: 0,
        };
        boxes.place(&mut placement);
        let p = placement.pose.position;
        assert!((p[0] - 20.).abs() < 1e-5 && (p[1] - 1.).abs() < 1e-5);
        assert!((p[2] - 9.85).abs() < 1e-5, "{p:?}");
        assert!(placement.bounds.min[2] < p[2] && p[2] < placement.bounds.max[2]);
        assert!(placement.bounds.min[0] < p[0] && p[0] < placement.bounds.max[0]);
    }

    #[test]
    fn content_count_stays_in_source_range() {
        let mut random = UnityRandom::new(1234);
        assert_eq!(box_count(0, &mut random), 1);
        let before = random.snapshot();
        assert_eq!(box_count(0, &mut random), 1);
        assert_eq!(random.snapshot(), before, "no draw without a maximum");
        // Item 75443 has maxCountIfSpawnedInBox 10.
        let mut seen = [false; 11];
        for _ in 0..500 {
            let n = box_count(10, &mut random);
            assert!((1..=10).contains(&n));
            seen[n as usize] = true;
        }
        assert!(seen[1..].iter().all(|v| *v));
    }

    #[test]
    fn weighted_pick_follows_box_chances() {
        // acquisition.box.table weights of item 62.
        let chances = [
            0.5, 0.5, 0.25, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 1., 1., 1., 2., 1., 1.,
        ];
        let total: f32 = chances.iter().sum();
        assert_eq!(pick_weighted(chances.iter().copied(), 0.), Some(0));
        assert_eq!(pick_weighted(chances.iter().copied(), 0.5), Some(1));
        assert_eq!(pick_weighted(chances.iter().copied(), 1.2), Some(2));
        assert_eq!(pick_weighted(chances.iter().copied(), 9.), Some(14));
        assert_eq!(pick_weighted(chances.iter().copied(), total), None);
        let mut random = UnityRandom::new(42);
        let mut counts = [0u32; 17];
        let draws = 20_000;
        for _ in 0..draws {
            let draw = random.range(0., total);
            counts[pick_weighted(chances.iter().copied(), draw).unwrap()] += 1;
        }
        for (count, chance) in counts.iter().zip(chances) {
            let expected = draws as f32 * chance / total;
            assert!((*count as f32 - expected).abs() < expected * 0.15 + 20.);
        }
        let mut again = UnityRandom::new(42);
        let first = pick_weighted(chances.iter().copied(), again.range(0., total));
        let mut random = UnityRandom::new(42);
        assert_eq!(
            first,
            pick_weighted(chances.iter().copied(), random.range(0., total))
        );
    }
}
