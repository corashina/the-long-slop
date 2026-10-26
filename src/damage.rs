//! Source breakablescript damage rules and bound break targets.
//! The rules are pure functions. `Breakables` holds the targets of active content.
use crate::reference::{BreakableReference, ItemId, LightControlId, SourceId, TriggerRole};
use crate::vehicle::{ColliderHandle, PhysicsWorld};
use crate::world::{GeneratedId, TriggerRoles, UnityRandom};
use rapier3d::prelude::{QueryFilter, RigidBodyHandle};
use std::collections::VecDeque;

pub mod gibs;

/// Source breakablescript.velocityToMass.
const VELOCITY_TO_MASS: f32 = 0.25;

/// Current breakablescript health values. They start at the record values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BreakHealth {
    pub health: f32,
    pub shoot_health: f32,
    pub explosion_health: f32,
}
impl BreakHealth {
    pub fn new(rules: &BreakableReference) -> Self {
        Self {
            health: rules.health,
            shoot_health: rules.shoot_health,
            explosion_health: rules.explosion_health,
        }
    }
}

/// Source breakablescript.OnCollisionEnter: the force passed to TryBreak.
pub fn collision_force(rules: &BreakableReference, relative_speed: f32) -> Option<f32> {
    if rules.no_collider
        || rules.only_decrease_above && relative_speed < rules.decrease_above_health
    {
        return None;
    }
    Some(relative_speed)
}
/// Source breakablescript.TryBreak(Rigidbody): the force of a body.
pub fn body_force(rules: &BreakableReference, speed: f32, mass: f32) -> f32 {
    if rules.only_velocity {
        speed * VELOCITY_TO_MASS
    } else {
        speed * VELOCITY_TO_MASS * mass * rules.mass_modifier
    }
}
/// Source breakablescript.TryBreak(float). True breaks.
pub fn try_break(rules: &BreakableReference, health: &mut f32, force: f32) -> bool {
    if force >= *health {
        return true;
    }
    if rules.decrease_after_attack {
        *health -= force;
    }
    false
}
/// Source breakablescript.TryBreakShoot. True breaks.
pub fn try_break_shoot(rules: &BreakableReference, state: &mut BreakHealth, force: f32) -> bool {
    if rules.no_shoot {
        return false;
    }
    if rules.shoot_different {
        let broken = force >= state.shoot_health;
        state.shoot_health -= force;
        return broken;
    }
    try_break(rules, &mut state.health, force)
}
/// Source breakablescript.TryBreakExplosion. True breaks.
pub fn try_break_explosion(
    rules: &BreakableReference,
    state: &mut BreakHealth,
    force: f32,
) -> bool {
    if rules.explosion_different {
        let broken = force >= state.explosion_health;
        state.explosion_health -= force;
        return broken;
    }
    try_break(rules, &mut state.health, force)
}

/// The object that a breakablescript breaks.
#[derive(Clone, Debug, PartialEq)]
pub enum BreakKind {
    /// A world content item. Its collider user data holds the item.
    Item(ItemId),
    /// A lamp trigger of a structure. The record lamp lists the lamps of the control.
    Lamp {
        parent: GeneratedId,
        control: LightControlId,
    },
}

/// One bound breakablescript. `record` is an index in `ReferenceManifest::breakables`.
#[derive(Clone, Debug)]
pub struct BreakTarget {
    pub kind: BreakKind,
    pub record: usize,
    pub health: BreakHealth,
    /// Source breakablescript.destroyed: a target breaks once.
    pub destroyed: bool,
    /// Source Break destroyed the lamp trigger: its colliders stay disabled.
    pub removed: bool,
}

/// Break targets of active content and the breaks of this step.
pub struct Breakables {
    targets: Vec<BreakTarget>,
    /// Lamp trigger colliders and their target.
    colliders: Vec<(ColliderHandle, usize)>,
    queue: VecDeque<usize>,
    scan: Option<[u64; 4]>,
    /// UnityEngine.Random draws of breakablescript.Break.
    random: UnityRandom,
}

impl Default for Breakables {
    fn default() -> Self {
        Self {
            targets: Vec::new(),
            colliders: Vec::new(),
            queue: VecDeque::new(),
            scan: None,
            random: UnityRandom::new(0x6272_6b6c),
        }
    }
}

impl Breakables {
    pub fn targets(&self) -> &[BreakTarget] {
        &self.targets
    }
    /// The target of a collider: an item collider or a lamp trigger collider.
    pub fn target_of(&self, physics: &PhysicsWorld, collider: ColliderHandle) -> Option<usize> {
        if let Some((_, target)) = self.colliders.iter().find(|(c, _)| *c == collider) {
            return Some(*target);
        }
        // Collider user data low 32 bits hold the item id plus one. Zero is no item.
        let data = physics.rapier().colliders.get(collider)?.user_data;
        let item = ((data & u32::MAX as u128) as u32)
            .checked_sub(1)
            .map(ItemId)?;
        self.targets
            .iter()
            .position(|t| t.kind == BreakKind::Item(item))
    }
    /// Source breakablescript.TryBreak(float).
    pub fn try_break(&mut self, records: &[BreakableReference], target: usize, force: f32) {
        let t = &mut self.targets[target];
        let broken = try_break(&records[t.record], &mut t.health.health, force);
        self.queue_break(target, broken);
    }
    /// Source meleeweaponscript hit: `TryBreak(decreaseAfterAttack ? damage * dt : damage)`.
    pub fn try_break_melee(
        &mut self,
        records: &[BreakableReference],
        target: usize,
        damage: f32,
        dt: f32,
    ) {
        let rules = &records[self.targets[target].record];
        let force = if rules.decrease_after_attack {
            damage * dt
        } else {
            damage
        };
        self.try_break(records, target, force);
    }
    /// Source breakablescript.TryBreak(Rigidbody).
    pub fn try_break_body(
        &mut self,
        records: &[BreakableReference],
        target: usize,
        speed: f32,
        mass: f32,
    ) {
        let force = body_force(&records[self.targets[target].record], speed, mass);
        self.try_break(records, target, force);
    }
    /// Source breakablescript.TryBreakShoot.
    pub fn try_break_shoot(&mut self, records: &[BreakableReference], target: usize, force: f32) {
        let t = &mut self.targets[target];
        let broken = try_break_shoot(&records[t.record], &mut t.health, force);
        self.queue_break(target, broken);
    }
    /// Source breakablescript.TryBreakExplosion.
    pub fn try_break_explosion(
        &mut self,
        records: &[BreakableReference],
        target: usize,
        force: f32,
    ) {
        let t = &mut self.targets[target];
        let broken = try_break_explosion(&records[t.record], &mut t.health, force);
        self.queue_break(target, broken);
    }
    /// Source breakablescript.Break runs once per target.
    fn queue_break(&mut self, target: usize, broken: bool) {
        let t = &mut self.targets[target];
        if broken && !t.destroyed {
            t.destroyed = true;
            self.queue.push_back(target);
        }
    }
    /// Run after the physics step. Source breakablescript.OnCollisionEnter for started contacts
    /// and playerdeathtriggerscript.OnTriggerStay for item bodies in hazard triggers.
    /// Source OnTriggerEnter is not bound: only Cacti has `no_trigger` false, and it binds no target.
    pub fn step_hits(
        &mut self,
        records: &[BreakableReference],
        physics: &PhysicsWorld,
        roles: &TriggerRoles,
        body_of: impl Fn(ItemId) -> Option<RigidBodyHandle>,
        dt: f32,
    ) {
        if self.targets.is_empty() {
            return;
        }
        for hit in physics.hits() {
            for handle in hit.colliders {
                let Some(target) = self.target_of(physics, handle) else {
                    continue;
                };
                if let Some(force) =
                    collision_force(&records[self.targets[target].record], hit.speed)
                {
                    self.try_break(records, target, force);
                }
            }
        }
        // Unity skips sleeping bodies in OnTriggerStay.
        if roles.is_empty() {
            return;
        }
        let rapier = physics.rapier();
        let filter = QueryFilter::default().exclude_solids();
        for target in 0..self.targets.len() {
            let t = &self.targets[target];
            let BreakKind::Item(item) = t.kind else {
                continue;
            };
            if t.destroyed {
                continue;
            }
            let Some(body) = body_of(item).and_then(|h| rapier.bodies.get(h)) else {
                continue;
            };
            if !body.is_enabled() || body.is_sleeping() {
                continue;
            }
            for &collider in body.colliders() {
                let collider = &rapier.colliders[collider];
                if !collider.is_enabled() || collider.is_sensor() {
                    continue;
                }
                for (trigger, other) in physics
                    .queries(filter)
                    .intersect_shape(*collider.position(), collider.shape())
                {
                    if let Some(TriggerRole::Hazard {
                        damage_breakable, ..
                    }) = roles.get(trigger, other)
                    {
                        self.try_break(records, target, damage_breakable * dt);
                    }
                }
            }
        }
    }
    /// Next broken target, in break order.
    pub fn take_break(&mut self) -> Option<usize> {
        self.queue.pop_front()
    }
    /// Source Break: `clips[Random.Range(0, clips.Length)]`. No draw without clips.
    pub fn pick_clip<'a>(&mut self, record: &'a BreakableReference) -> Option<&'a SourceId> {
        if record.clips.is_empty() {
            return None;
        }
        let index = self.random.range_int(0, record.clips.len() as i32);
        record.clips.get(index as usize)
    }
    /// UnityEngine.Random of breaks and gib starts.
    pub fn random(&mut self) -> &mut UnityRandom {
        &mut self.random
    }
    /// Stores the binding scan. False when it did not change.
    pub(crate) fn rescan(&mut self, scan: [u64; 4]) -> bool {
        if self.scan == Some(scan) {
            return false;
        }
        self.scan = Some(scan);
        true
    }
    /// Keeps targets in order and moves queued breaks with them. Clears lamp colliders.
    pub(crate) fn retain(&mut self, mut keep: impl FnMut(&BreakTarget) -> bool) {
        let mut next = 0;
        for index in 0..self.targets.len() {
            if !keep(&self.targets[index]) {
                self.queue.retain(|q| *q != index);
                continue;
            }
            self.targets.swap(next, index);
            for q in &mut self.queue {
                if *q == index {
                    *q = next;
                }
            }
            next += 1;
        }
        self.targets.truncate(next);
        self.colliders.clear();
    }
    /// Adds a target unless the same kind and record is bound.
    pub(crate) fn bind(&mut self, kind: BreakKind, record: usize, rules: &BreakableReference) {
        if !self
            .targets
            .iter()
            .any(|t| t.record == record && t.kind == kind)
        {
            self.targets.push(BreakTarget {
                kind,
                record,
                health: BreakHealth::new(rules),
                destroyed: false,
                removed: false,
            });
        }
    }
    pub(crate) fn bind_collider(&mut self, collider: ColliderHandle, target: usize) {
        self.colliders.push((collider, target));
    }
    /// The lamp trigger colliders of a target.
    pub fn colliders_of(&self, target: usize) -> impl Iterator<Item = ColliderHandle> + '_ {
        self.colliders
            .iter()
            .filter(move |(_, t)| *t == target)
            .map(|(c, _)| *c)
    }
    /// Source Break destroys the lamp trigger of a target.
    pub(crate) fn remove(&mut self, target: usize) {
        self.targets[target].removed = true;
    }
    /// True for a collider of a destroyed lamp trigger.
    pub fn removed_collider(&self, collider: ColliderHandle) -> bool {
        self.colliders
            .iter()
            .any(|(c, t)| *c == collider && self.targets[*t].removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules(health: f32) -> BreakableReference {
        BreakableReference {
            source: SourceId {
                file: "sharedassets1.assets".into(),
                path_id: 1,
            },
            owners: Vec::new(),
            health,
            shoot_health: 0.,
            shoot_different: false,
            explosion_health: 0.,
            explosion_different: false,
            no_shoot: false,
            decrease_after_attack: false,
            only_decrease_above: false,
            decrease_above_health: 0.,
            only_velocity: false,
            mass_modifier: 1.,
            child_modifier: 0.,
            children: Vec::new(),
            no_trigger: true,
            no_collider: false,
            no_gib: false,
            no_destroy: false,
            gib: None,
            reduced_gib: None,
            clips: Vec::new(),
            lamp: None,
        }
    }

    #[test]
    fn health_threshold_and_decrease() {
        // Box: health 7, no decrease. A hazard breaks it when damage * dt reaches 7.
        let r#box = rules(7.);
        let mut health = r#box.health;
        assert!(!try_break(&r#box, &mut health, 6.99));
        assert!(try_break(&r#box, &mut health, 350. * 0.02));
        // Bottle: health 6, no decrease. Hits below 6 never add up.
        let bottle = rules(6.);
        let mut health = bottle.health;
        for _ in 0..10 {
            assert!(!try_break(&bottle, &mut health, 5.9));
        }
        assert_eq!(health, 6.);
        assert!(try_break(&bottle, &mut health, 6.));
        // Lamp: health 5, decreaseAfterAttack. Hits add up.
        let lamp = BreakableReference {
            decrease_after_attack: true,
            ..rules(5.)
        };
        let mut health = lamp.health;
        assert!(!try_break(&lamp, &mut health, 3.));
        assert_eq!(health, 2.);
        assert!(try_break(&lamp, &mut health, 2.));
    }

    #[test]
    fn collision_and_body_force() {
        let mut bottle = rules(6.);
        assert_eq!(body_force(&bottle, 8., 2.), 4.);
        bottle.mass_modifier = 0.5;
        assert_eq!(body_force(&bottle, 8., 2.), 2.);
        bottle.only_velocity = true;
        assert_eq!(body_force(&bottle, 8., 2.), 2.);
        assert_eq!(collision_force(&bottle, 6.99), Some(6.99));
        bottle.only_decrease_above = true;
        bottle.decrease_above_health = 5.;
        assert_eq!(collision_force(&bottle, 4.), None);
        assert_eq!(collision_force(&bottle, 5.), Some(5.));
        bottle.no_collider = true;
        assert_eq!(collision_force(&bottle, 40.), None);
    }

    #[test]
    fn a_target_breaks_once_and_queued_breaks_follow_rebinding() {
        let records = [rules(6.), rules(7.)];
        let mut breakables = Breakables::default();
        for (item, record) in [(1, 0), (2, 1), (3, 0)] {
            breakables.bind(BreakKind::Item(ItemId(item)), record, &records[record]);
        }
        breakables.bind(BreakKind::Item(ItemId(1)), 0, &records[0]);
        assert_eq!(breakables.targets().len(), 3, "a bound target binds once");
        breakables.try_break(&records, 2, 5.);
        assert!(breakables.take_break().is_none());
        breakables.try_break(&records, 2, 6.);
        breakables.try_break_body(&records, 2, 400., 1.);
        breakables.try_break(&records, 1, 7.);
        assert!(breakables.targets()[2].destroyed);
        // Item 1 goes before the breaks are taken. Queued targets move with their index.
        breakables.retain(|t| t.kind != BreakKind::Item(ItemId(1)));
        let item = |b: &Breakables, i: usize| b.targets()[i].kind.clone();
        let first = breakables.take_break().unwrap();
        assert_eq!(item(&breakables, first), BreakKind::Item(ItemId(3)));
        let second = breakables.take_break().unwrap();
        assert_eq!(item(&breakables, second), BreakKind::Item(ItemId(2)));
        assert!(breakables.take_break().is_none(), "a target breaks once");
    }

    #[test]
    fn melee_damage_scales_with_dt_only_for_decrease_after_attack() {
        // Shovel damage 40. Lamp: health 5, decreaseAfterAttack. Bottle: health 6.
        let records = [
            BreakableReference {
                decrease_after_attack: true,
                ..rules(5.)
            },
            rules(6.),
        ];
        let mut breakables = Breakables::default();
        breakables.bind(BreakKind::Item(ItemId(1)), 0, &records[0]);
        breakables.bind(BreakKind::Item(ItemId(2)), 1, &records[1]);
        // 40 * 0.05 = 2 per frame: health 5 goes to 1 after two frames and breaks at the third.
        for _ in 0..2 {
            breakables.try_break_melee(&records, 0, 40., 0.05);
        }
        assert_eq!(breakables.targets()[0].health.health, 1.);
        assert!(breakables.take_break().is_none());
        breakables.try_break_melee(&records, 0, 40., 0.05);
        assert_eq!(breakables.take_break(), Some(0));
        // Full damage breaks the bottle at the first frame.
        breakables.try_break_melee(&records, 1, 40., 0.05);
        assert_eq!(breakables.take_break(), Some(1));
    }

    #[test]
    fn hazard_damage_adds_up_and_breaks_an_item_once() {
        use rapier3d::prelude::{ColliderBuilder, RigidBodyBuilder};
        let records = [BreakableReference {
            decrease_after_attack: true,
            ..rules(7.)
        }];
        let mut physics = PhysicsWorld::empty_for_tests();
        let hazard = physics.insert_collider(ColliderBuilder::cuboid(2., 2., 2.).sensor(true));
        let (body, _) = physics.insert(
            RigidBodyBuilder::dynamic(),
            ColliderBuilder::cuboid(0.2, 0.2, 0.2),
        );
        physics.refresh_queries();
        let mut roles = TriggerRoles::default();
        roles.insert(
            hazard,
            TriggerRole::Hazard {
                damage_player: 0.,
                damage_breakable: 100.,
            },
        );
        let mut breakables = Breakables::default();
        breakables.bind(BreakKind::Item(ItemId(4)), 0, &records[0]);
        let step = |b: &mut Breakables| {
            b.step_hits(&records, &physics, &roles, |_| Some(body), 0.02);
        };
        // 2 per step: health 7 goes to 1 after three steps and breaks at the fourth.
        for _ in 0..3 {
            step(&mut breakables);
        }
        assert!(breakables.take_break().is_none());
        for _ in 0..3 {
            step(&mut breakables);
        }
        assert_eq!(breakables.take_break(), Some(0));
        assert!(breakables.take_break().is_none(), "a target breaks once");
    }

    #[test]
    fn shoot_and_explosion_health() {
        // Cacti: health 64, decreaseAfterAttack, explosionDifferent with health 2.
        let cacti = BreakableReference {
            decrease_after_attack: true,
            explosion_different: true,
            explosion_health: 2.,
            ..rules(64.)
        };
        let mut state = BreakHealth::new(&cacti);
        assert!(!try_break_explosion(&cacti, &mut state, 1.5));
        assert!(try_break_explosion(&cacti, &mut state, 0.5));
        assert_eq!(state.health, 64., "explosions skip the main health");
        // Shots use the main health unless shootDifferent.
        assert!(!try_break_shoot(&cacti, &mut state, 60.));
        assert_eq!(state.health, 4.);
        let shot = BreakableReference {
            shoot_different: true,
            shoot_health: 3.,
            ..cacti.clone()
        };
        let mut state = BreakHealth::new(&shot);
        assert!(!try_break_shoot(&shot, &mut state, 2.));
        assert!(try_break_shoot(&shot, &mut state, 1.));
        assert_eq!(state.health, 64.);
        let no_shoot = BreakableReference {
            no_shoot: true,
            ..shot
        };
        assert!(!try_break_shoot(&no_shoot, &mut state, 100.));
    }

    #[test]
    fn break_clip_draws_once_only_with_clips() {
        let mut breakables = Breakables::default();
        let mut expected = UnityRandom::new(0x6272_6b6c);
        assert!(breakables.pick_clip(&rules(6.)).is_none());
        // Vase (kocsog): three clips, one Random.Range(0, 3) per break.
        let clip = |path_id| SourceId {
            file: "sharedassets1.assets".into(),
            path_id,
        };
        let vase = BreakableReference {
            clips: vec![clip(1), clip(2), clip(3)],
            ..rules(5.)
        };
        for _ in 0..8 {
            let index = expected.range_int(0, 3) as usize;
            assert_eq!(breakables.pick_clip(&vase), Some(&vase.clips[index]));
        }
    }
}
