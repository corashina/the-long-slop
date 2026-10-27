//! Source `explosionscript.Explode`, `tankscript.TryExplode`, `explosivescript.TryExplode`,
//! `fluidcontainer.isExplosive` and the blast particle and sound.
use crate::fluids::FluidContainer;
use crate::particles::{ParticleOutput, ParticleState};
use crate::reference::{
    EXPLOSION_TYPES, ExplosionsReference, ItemId, ReferenceError, ReferenceManifest,
};
use crate::vehicle::PhysicsWorld;
use crate::world::{Explosive, TankExplosion};
use rapier3d::math::{Pose, Rotation, Vector};
use rapier3d::prelude::{Ball, ColliderHandle, QueryFilter, RigidBodyHandle};

/// `Rigidbody.AddExplosionForce` upwards modifier of `explosionscript.Explode`.
const UPWARDS: f32 = 0.3;

/// One blast: the prefab type, `explosionForce` after `pF`, `explosionDist`, the point in
/// physics space, and the particle start color.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Blast {
    pub kind: usize,
    pub force: f32,
    pub radius: f32,
    pub position: Vector,
    pub tint: Option<[f32; 4]>,
}

/// `explosionscript.CalcPower`.
pub fn power(distance: f32, radius: f32, force: f32) -> f32 {
    let t = if radius > 0. {
        (distance / radius).clamp(0., 1.)
    } else {
        1.
    };
    let angle = 90. - 180. * t;
    (f32::to_radians(angle).sin() + 1.) * 0.5 * force
}

/// `fluidcontainer.isExplosive`: explosive fluids are over 40 % of more than 0.2 L.
pub fn is_explosive(fluid: &FluidContainer, explosive: &[u8]) -> bool {
    let total = fluid.total();
    let f: f32 = fluid
        .entries()
        .filter(|e| explosive.contains(&e.fluid_type))
        .map(|e| e.amount)
        .sum();
    f / total > 0.4 && total > 0.2
}

/// `tankscript.TryExplode` without `alreadyExploded`: the `pF` of the blast, or `None`.
pub fn tank_factor(
    e: &TankExplosion,
    fluid: &FluidContainer,
    explosive: &[u8],
    force: f32,
) -> Option<f32> {
    (!e.never_explode
        && (e.always_explode || is_explosive(fluid, explosive))
        && force >= e.force_needed)
        .then(|| {
            if e.always_explode {
                e.always_explode_force
            } else {
                fluid.total()
            }
        })
}

/// `explosivescript.TryExplode`: the `pF` of the blast, or `None`. `rounds` is the item's
/// ammo count when the prop has an ammo link.
pub fn prop_factor(e: &Explosive, rounds: Option<f32>, force: f32) -> Option<f32> {
    if force <= e.force_needed {
        return None;
    }
    match rounds {
        None => Some(e.explode_force),
        Some(r) => (r >= e.min_ammo).then_some(r * e.explode_force),
    }
}

/// `Rigidbody.AddExplosionForce(force, at, radius, UPWARDS, Impulse)` on a body with its
/// center of mass at `com`: the impulse falls linearly to zero at `radius`.
pub fn explosion_impulse(force: f32, at: Vector, radius: f32, com: Vector) -> Vector {
    let distance = com.distance(at);
    if radius > 0. && distance > radius {
        return Vector::ZERO;
    }
    let falloff = if radius > 0. {
        1. - distance / radius
    } else {
        1.
    };
    let centre = at - Vector::Y * UPWARDS;
    (com - centre).normalize_or(Vector::Y) * force * falloff
}

struct ExplosionType {
    force: f32,
    sound: (u32, f32),
    /// Particles that one Instantiate starts: the burst and one duration of the rate.
    count: usize,
}

/// The blasts of one step and their particles.
pub struct ExplosionState {
    types: Vec<ExplosionType>,
    particles: Vec<ParticleState>,
    explosive_fluids: Vec<u8>,
    queue: Vec<Blast>,
    next: usize,
    /// `alreadyExploded` and `exploded` of this step.
    exploded: Vec<ItemId>,
    hits: Vec<(ColliderHandle, f32)>,
    bodies: Vec<RigidBodyHandle>,
    /// The strongest blast power at the player this step. Hearing and death read it.
    pub player: Option<f32>,
}

impl ExplosionState {
    pub fn new(r: &ReferenceManifest) -> Result<Self, ReferenceError> {
        let clips = super::melee::clip_table(r);
        let e: &ExplosionsReference = &r.explosions;
        let mut types = Vec::with_capacity(EXPLOSION_TYPES);
        for t in &e.types {
            let clip = clips
                .iter()
                .find(|(s, _, _)| *s == t.sound.clip)
                .ok_or_else(|| ReferenceError("Missing explosion clip".into()))?
                .1;
            let p = &t.particle;
            let burst = if p.scalars.get("EmissionModule.m_BurstCount") == Some(&1.) {
                p.curves["EmissionModule.m_Bursts.0.countCurve"].scalar
            } else {
                0.
            };
            types.push(ExplosionType {
                force: t.explosion_force,
                sound: (clip, t.sound.volume),
                count: (burst + p.rate_over_time * p.duration).round() as usize,
            });
        }
        Ok(Self {
            types,
            particles: e
                .types
                .iter()
                .enumerate()
                .map(|(i, t)| ParticleState::emit_only(&t.particle, 0x626c_7374 + i as u64))
                .collect(),
            explosive_fluids: e.explosive_fluids.clone(),
            queue: Vec::with_capacity(16),
            next: 0,
            exploded: Vec::with_capacity(16),
            hits: Vec::with_capacity(256),
            bodies: Vec::with_capacity(64),
            player: None,
        })
    }
    pub fn explosive_fluids(&self) -> &[u8] {
        &self.explosive_fluids
    }
    /// `GetExplosionType(kind)` and `explosionscript.Explode(pF, distAdd)` at `position`.
    pub fn blast(
        &self,
        kind: i32,
        factor: f32,
        dist_add: f32,
        position: Vector,
        tint: Option<[f32; 4]>,
    ) -> Blast {
        let kind = usize::try_from(kind)
            .ok()
            .filter(|k| *k < EXPLOSION_TYPES)
            .unwrap_or(0);
        let force = self.types[kind].force * factor;
        Blast {
            kind,
            force,
            radius: force * 0.5 + dist_add,
            position,
            tint,
        }
    }
    /// Queues the blast of `item`. False when `item` already exploded this step.
    pub fn trigger(&mut self, item: ItemId, blast: Blast) -> bool {
        if self.exploded.contains(&item) {
            return false;
        }
        self.exploded.push(item);
        self.queue.push(blast);
        true
    }
    /// The next queued blast. Blasts that it triggers run after it.
    pub fn next(&mut self) -> Option<Blast> {
        let blast = self.queue.get(self.next).copied()?;
        self.next += 1;
        Some(blast)
    }
    /// Ends the step: the queue is empty.
    pub fn finish(&mut self) {
        self.queue.clear();
        self.exploded.clear();
        self.next = 0;
    }
    /// `Physics.OverlapSphere` with all layers and triggers: each collider and its power, and
    /// each body once.
    pub fn query(&mut self, physics: &PhysicsWorld, blast: &Blast) {
        self.hits.clear();
        self.bodies.clear();
        let predicate = |_, c: &rapier3d::prelude::Collider| c.is_enabled();
        let filter = QueryFilter::default().predicate(&predicate);
        let ball = Ball::new(blast.radius.max(f32::EPSILON));
        let at = Pose::from_translation(blast.position);
        for (handle, collider) in physics.queries(filter).intersect_shape(at, &ball) {
            let d = collider.translation().distance(blast.position);
            self.hits
                .push((handle, power(d, blast.radius, blast.force)));
            if let Some(body) = collider.parent()
                && !self.bodies.contains(&body)
            {
                self.bodies.push(body);
            }
        }
    }
    pub fn hits(&self) -> &[(ColliderHandle, f32)] {
        &self.hits
    }
    /// `AddExplosionForce(2 * force)` on each dynamic body of the last query but the player.
    pub fn push(&self, physics: &mut PhysicsWorld, blast: &Blast, player: Option<RigidBodyHandle>) {
        for &body in &self.bodies {
            let rb = &physics.rapier().bodies[body];
            if Some(body) == player || !rb.is_dynamic() {
                continue;
            }
            let impulse = explosion_impulse(
                2. * blast.force,
                blast.position,
                blast.radius,
                rb.center_of_mass(),
            );
            physics.edit_body(body, |b| b.apply_impulse(impulse, true));
        }
    }
    /// The prefab particle at the point with its start color, and the blast clip and gain.
    pub fn effect(&mut self, blast: &Blast, absolute: [f64; 3]) -> (u32, f32) {
        let t = &self.types[blast.kind];
        let p = &mut self.particles[blast.kind];
        p.start_color = blast.tint;
        p.emit(t.count, absolute, Rotation::IDENTITY);
        t.sound
    }
    pub fn advance(&mut self, dt: f32, origin: [f64; 3], physics: &PhysicsWorld) {
        for p in &mut self.particles {
            p.set_origin(origin);
            p.advance(dt, origin, 0., physics);
        }
    }
    pub fn particles(&self, kind: usize) -> &[ParticleOutput] {
        self.particles[kind].particles()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fluids::FluidEntry;

    fn tank() -> TankExplosion {
        TankExplosion {
            never_explode: false,
            always_explode: false,
            always_explode_force: 3.,
            force_needed: 10.,
            explosion_type: 0,
            no_shoot_hole: false,
            no_shoot_explode: false,
            parent_dest: None,
        }
    }

    fn fluid(entries: &[(u8, f32)]) -> FluidContainer {
        let entries: Vec<_> = entries
            .iter()
            .map(|&(fluid_type, amount)| FluidEntry { fluid_type, amount })
            .collect();
        FluidContainer::new(20., &entries)
    }

    #[test]
    fn power_falls_from_full_at_the_point_to_zero_at_the_radius() {
        assert_eq!(power(0., 10., 8.), 8.);
        assert!((power(5., 10., 8.) - 4.).abs() < 1e-5);
        assert!(power(10., 10., 8.).abs() < 1e-5);
        assert!(power(20., 10., 8.).abs() < 1e-5);
    }

    #[test]
    fn tank_needs_explosive_fluid_force_and_no_never_explode() {
        let gas = [0];
        let full = fluid(&[(0, 10.), (1, 5.)]);
        assert_eq!(tank_factor(&tank(), &full, &gas, 10.), Some(15.));
        assert_eq!(tank_factor(&tank(), &full, &gas, 9.), None);
        // 40 % or less, or 0.2 L or less, does not explode.
        assert_eq!(
            tank_factor(&tank(), &fluid(&[(0, 4.), (1, 6.)]), &gas, 99.),
            None
        );
        assert_eq!(tank_factor(&tank(), &fluid(&[(0, 0.2)]), &gas, 99.), None);
        assert_eq!(tank_factor(&tank(), &fluid(&[]), &gas, 99.), None);
        let always = TankExplosion {
            always_explode: true,
            ..tank()
        };
        assert_eq!(tank_factor(&always, &fluid(&[]), &gas, 10.), Some(3.));
        let never = TankExplosion {
            never_explode: true,
            ..always
        };
        assert_eq!(tank_factor(&never, &full, &gas, 99.), None);
    }

    #[test]
    fn prop_ammo_scales_the_force_from_min_ammo() {
        let e = Explosive {
            force_needed: 5.,
            explode_force: 2.,
            explosion_type: 0,
            min_ammo: 3.,
            ammo: None,
            color: [1.; 4],
            ccolor: None,
        };
        assert_eq!(prop_factor(&e, None, 5.), None);
        assert_eq!(prop_factor(&e, None, 6.), Some(2.));
        assert_eq!(prop_factor(&e, Some(2.), 6.), None);
        assert_eq!(prop_factor(&e, Some(4.), 6.), Some(8.));
    }

    #[test]
    fn a_chain_runs_each_item_once() {
        let mut state = ExplosionState {
            types: Vec::new(),
            particles: Vec::new(),
            explosive_fluids: Vec::new(),
            queue: Vec::new(),
            next: 0,
            exploded: Vec::new(),
            hits: Vec::new(),
            bodies: Vec::new(),
            player: None,
        };
        let blast = |force| Blast {
            kind: 0,
            force,
            radius: 1.,
            position: Vector::ZERO,
            tint: None,
        };
        let (a, b) = (ItemId(1), ItemId(2));
        assert!(state.trigger(a, blast(1.)));
        assert_eq!(state.next(), Some(blast(1.)));
        // Blast A sets off B and reaches A again.
        assert!(state.trigger(b, blast(2.)));
        assert!(!state.trigger(a, blast(1.)));
        assert_eq!(state.next(), Some(blast(2.)));
        // Blast B reaches A and B.
        assert!(!state.trigger(a, blast(1.)));
        assert!(!state.trigger(b, blast(2.)));
        assert_eq!(state.next(), None);
        state.finish();
        assert_eq!(state.next(), None);
    }
}
