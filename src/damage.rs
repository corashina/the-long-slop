//! Source breakablescript damage rules. Pure functions with no world access.
use crate::reference::BreakableReference;

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
