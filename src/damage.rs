//! Source breakablescript damage rules and bound break targets.
//! The rules are pure functions. `Breakables` holds the targets of active content.
use crate::reference::{BreakableReference, ItemId, LightControlId};
use crate::vehicle::{ColliderHandle, PhysicsWorld};
use crate::world::GeneratedId;
use std::collections::VecDeque;

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
}

/// Break targets of active content and the breaks of this step.
#[derive(Default)]
pub struct Breakables {
    targets: Vec<BreakTarget>,
    /// Lamp trigger colliders and their target.
    colliders: Vec<(ColliderHandle, usize)>,
    queue: VecDeque<usize>,
    scan: Option<[u64; 4]>,
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
    /// Next broken target, in break order.
    pub fn take_break(&mut self) -> Option<usize> {
        self.queue.pop_front()
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
            });
        }
    }
    pub(crate) fn bind_collider(&mut self, collider: ColliderHandle, target: usize) {
        self.colliders.push((collider, target));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reference::SourceId;

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
}
