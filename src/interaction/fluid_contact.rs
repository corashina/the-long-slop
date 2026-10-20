//! World-space source fluid particles and collision-driven drinking.

use crate::reference::FluidParticleReference;
use crate::vehicle::PhysicsWorld;
use rapier3d::math::{Pose, Rotation, Vector};
use rapier3d::parry::query::ShapeCastOptions;
use rapier3d::prelude::{Ball, ColliderHandle, QueryFilter, RigidBodyHandle};

pub struct FluidParticle {
    pub position: Vector,
    pub size: f32,
    pub color: [f32; 4],
    pub rotation: f32,
    pub age: f32,
    start_color: [f32; 4],
    velocity: Vector,
    remaining: f32,
}

#[derive(Clone, Copy)]
pub(crate) struct FluidCollision {
    pub collider: ColliderHandle,
    pub position: Vector,
}

pub struct FluidContact {
    collisions: Vec<FluidCollision>,
    player_hit: bool,
    particles: Vec<FluidParticle>,
    emission_remainder: f32,
    last_emitter: Option<Vector>,
    random: u32,
    pub(crate) last_poured: f32,
    visual: crate::reference::ParticleReference,
    pub(crate) splash: crate::particles::ParticleState,
    colors: [[f32; 4]; 6],
    start_color: [f32; 4],
    origin: [f64; 3],
}

impl FluidContact {
    pub(crate) fn new(
        reference: &FluidParticleReference,
        visual: &crate::reference::ParticleReference,
        splash: &crate::reference::ParticleReference,
        seed: u32,
    ) -> Self {
        Self {
            particles: Vec::with_capacity(reference.capacity as usize),
            collisions: Vec::with_capacity(reference.capacity as usize),
            player_hit: false,
            emission_remainder: 0.,
            last_emitter: None,
            random: seed.max(1),
            last_poured: 0.,
            visual: visual.clone(),
            splash: crate::particles::ParticleState::new(splash, seed as u64),
            colors: std::array::from_fn(|i| {
                let c = visual.vectors[&format!("fluid.{i}")];
                [
                    c[0],
                    c[1],
                    c[2],
                    visual.scalars[&format!("fluid.{i}.alpha")],
                ]
            }),
            start_color: [1.; 4],
            origin: [0.; 3],
        }
    }

    pub(crate) fn clear_outputs(&mut self) {
        self.collisions.clear();
        self.player_hit = false;
        self.last_poured = 0.;
    }
    pub(crate) fn collisions(&self) -> &[FluidCollision] {
        &self.collisions
    }
    pub(crate) fn player_hit(&self) -> bool {
        self.player_hit
    }
    pub fn particles(&self) -> &[FluidParticle] {
        &self.particles
    }
    pub fn visual(&self) -> &crate::reference::ParticleReference {
        &self.visual
    }
    pub fn splash(&self) -> &[crate::particles::ParticleOutput] {
        self.splash.particles()
    }
    pub(crate) fn set_color(&mut self, fractions: [f32; 6]) {
        self.start_color =
            std::array::from_fn(|i| (0..6).map(|j| fractions[j] * self.colors[j][i]).sum());
        if self.visual.flags["outlet.forceColor"] {
            let c = self.visual.vectors["outlet.forceColor"];
            let force = [c[0], c[1], c[2], self.visual.scalars["outlet.forceAlpha"]];
            for i in 0..4 {
                self.start_color[i] +=
                    (force[i] - self.start_color[i]) * self.visual.scalars["outlet.forceColorM"];
            }
        }
        for component in &mut self.start_color {
            if !component.is_finite() {
                *component = 0.;
            }
        }
    }
    pub(crate) fn rebase(&mut self, shift: Vector) {
        for i in 0..3 {
            self.origin[i] += shift[i] as f64;
        }
        for particle in &mut self.particles {
            particle.position -= shift;
        }
        if let Some(emitter) = self.last_emitter.as_mut() {
            *emitter -= shift;
        }
    }

    fn random(&mut self) -> f32 {
        self.random ^= self.random << 13;
        self.random ^= self.random >> 17;
        self.random ^= self.random << 5;
        (self.random as f32) * (1. / u32::MAX as f32)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn advance(
        &mut self,
        reference: &FluidParticleReference,
        cap_pose: Pose,
        local_axis: Vector,
        player_body: Option<RigidBodyHandle>,
        cap_center: Vector,
        cap_radius: f32,
        contact_radius: f32,
        physics: &PhysicsWorld,
        dt: f32,
        rate: f32,
        speed: f32,
        poured: Option<f32>,
    ) {
        self.clear_outputs();
        if !dt.is_finite() || dt <= 0. {
            return;
        }
        let rate = if rate.is_finite() { rate } else { 0. };
        let speed = if speed.is_finite() { speed } else { 0. };
        self.last_poured = poured.unwrap_or(0.);
        self.splash.rotation = cap_pose.rotation;
        self.splash.set_origin(self.origin);
        self.splash.advance(
            dt,
            std::array::from_fn(|i| cap_pose.translation[i] as f64 + self.origin[i]),
            0.,
            physics,
        );
        let emitter = cap_pose
            * Pose::from_parts(
                Vector::from_array(reference.local_pose.translation),
                Rotation::from_xyzw(
                    reference.local_pose.rotation[0],
                    reference.local_pose.rotation[1],
                    reference.local_pose.rotation[2],
                    reference.local_pose.rotation[3],
                ),
            );
        let axis = (emitter.rotation * local_axis).normalize_or_zero();
        let emitter_distance = self
            .last_emitter
            .map_or(0., |previous| (emitter.translation - previous).length());
        self.last_emitter = Some(emitter.translation);
        if rate > 0. && speed > 0. {
            self.emission_remainder += rate * dt + rate * 2. * emitter_distance;
            let count = self.emission_remainder.floor() as usize;
            self.emission_remainder -= count as f32;
            let tangent = if axis.y.abs() < 0.9 {
                axis.cross(Vector::Y).normalize()
            } else {
                axis.cross(Vector::X).normalize()
            };
            let bitangent = axis.cross(tangent);
            let min_cos = reference.cone_degrees.to_radians().cos();
            for _ in 0..count.min(reference.capacity as usize - self.particles.len()) {
                let cosine = 1. - self.random() * (1. - min_cos);
                let azimuth = self.random() * std::f32::consts::TAU;
                let radial = (1. - cosine * cosine).sqrt();
                let direction =
                    axis * cosine + (tangent * azimuth.cos() + bitangent * azimuth.sin()) * radial;
                let offset_radius = self.random().sqrt() * reference.cone_radius;
                let offset_angle = self.random() * std::f32::consts::TAU;
                let draw = self.random();
                let rotation = crate::particles::particle_curve(
                    &self.visual.curves["InitialModule.startRotation"],
                    0.,
                    draw,
                );
                self.particles.push(FluidParticle {
                    size: self.visual.curves["InitialModule.startSize"].scalar,
                    color: self.start_color,
                    start_color: self.start_color,
                    rotation,
                    age: 0.,
                    position: emitter.translation
                        + (tangent * offset_angle.cos() + bitangent * offset_angle.sin())
                            * offset_radius,
                    velocity: direction * speed,
                    remaining: reference.lifetime_seconds,
                });
            }
        }
        let mut index = 0;
        while index < self.particles.len() {
            let particle = &mut self.particles[index];
            particle.remaining -= dt;
            particle.age += dt;
            let t = (particle.age / reference.lifetime_seconds).clamp(0., 1.);
            particle.size = self.visual.curves["InitialModule.startSize"].scalar
                * crate::particles::particle_curve(&self.visual.curves["SizeModule.curve"], t, 0.);
            let color = crate::particles::particle_color(
                &self.visual.colors["ColorModule.gradient"],
                t,
                0.,
            );
            particle.color = std::array::from_fn(|i| particle.start_color[i] * color[i]);
            if particle.remaining <= 0. {
                self.particles.swap_remove(index);
                continue;
            }
            let velocity =
                particle.velocity + Vector::new(0., -9.81 * reference.gravity_multiplier, 0.) * dt;
            let travel = velocity * dt;
            particle.velocity = velocity;
            let radius = particle.size * 0.5 * self.visual.scalars["CollisionModule.radiusScale"];
            let cap_hit = if contact_radius > 0. {
                sphere_hit(particle.position, travel, cap_center, cap_radius + radius)
            } else {
                None
            };
            let surfaces = |_: ColliderHandle, collider: &rapier3d::prelude::Collider| {
                collider.is_enabled()
                    && !collider.is_sensor()
                    && collider.collision_groups().memberships.bits() & reference.collision_mask
                        != 0
                    && player_body.is_none_or(|body| collider.parent() != Some(body))
            };
            let world_hit = physics
                .queries(QueryFilter::default().predicate(&surfaces))
                .cast_shape(
                    &Pose::from_parts(particle.position, Rotation::IDENTITY),
                    travel,
                    &Ball::new(radius),
                    ShapeCastOptions {
                        max_time_of_impact: 1.,
                        stop_at_penetration: true,
                        ..Default::default()
                    },
                );
            if let Some(time) = cap_hit
                && (particle.position + travel * time - cap_center).length() < contact_radius
                && world_hit
                    .as_ref()
                    .is_none_or(|(_, obstruction)| time < obstruction.time_of_impact)
            {
                self.player_hit = true;
                self.particles.swap_remove(index);
            } else if let Some((collider, hit)) = world_hit {
                self.collisions.push(FluidCollision {
                    collider,
                    position: hit.witness1,
                });
                if hit.time_of_impact <= 0. {
                    self.particles.swap_remove(index);
                    continue;
                }
                particle.remaining -= reference.lifetime_seconds
                    * self.visual.curves["CollisionModule.m_EnergyLossOnCollision"].scalar;
                particle.age = reference.lifetime_seconds - particle.remaining;
                if particle.remaining <= 0. {
                    self.particles.swap_remove(index);
                    continue;
                }
                let normal = hit.normal1.normalize_or_zero();
                let inward = particle.velocity.dot(normal).min(0.);
                particle.position += travel * hit.time_of_impact + normal * 0.0001;
                particle.velocity = (particle.velocity - normal * inward) * (1. - reference.dampen)
                    - normal * inward * reference.bounce;
                index += 1;
            } else {
                particle.position += travel;
                index += 1;
            }
        }
    }
}

fn sphere_hit(start: Vector, travel: Vector, center: Vector, radius: f32) -> Option<f32> {
    let delta = start - center;
    let c = delta.length_squared() - radius * radius;
    if c <= 0. {
        return Some(0.);
    }
    let a = travel.length_squared();
    if a <= 0. {
        return None;
    }
    let b = 2. * delta.dot(travel);
    let discriminant = b * b - 4. * a * c;
    if discriminant < 0. {
        return None;
    }
    let time = (-b - discriminant.sqrt()) / (2. * a);
    (0. ..=1.).contains(&time).then_some(time)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reference::ReferenceManifest;
    use rapier3d::prelude::{ColliderBuilder, RigidBodyBuilder};

    #[test]
    fn g04_receipts_own_colliders_record_overlap_bounce_and_clear_outputs() {
        let mut r = ReferenceManifest::from_runtime_json(
            &std::fs::read("web/assets/reference/manifest.json").unwrap(),
        )
        .unwrap();
        let opening = r
            .openings
            .iter()
            .find(|o| o.tank == crate::reference::ItemId(116))
            .unwrap();
        let mut reference = opening.particles.clone();
        let visual = r
            .particles
            .iter()
            .find(|p| p.role == format!("outlet_{}", opening.id.0))
            .unwrap();
        let splash = r
            .particles
            .iter()
            .find(|p| p.role == format!("splash_{}", opening.id.0))
            .unwrap();
        let mut contact = FluidContact::new(&reference, visual, splash, 1);
        reference.gravity_multiplier = 0.;
        reference.bounce = 1.;
        reference.dampen = 0.;
        reference.collision_mask = u32::MAX;
        contact
            .visual
            .curves
            .get_mut("CollisionModule.m_EnergyLossOnCollision")
            .unwrap()
            .scalar = 0.;
        r.vehicles.clear();
        let mut physics = PhysicsWorld::new(&r, &[]).unwrap();
        let source = physics.insert_body(RigidBodyBuilder::fixed());
        let collider =
            physics.insert_collider_with_parent(ColliderBuilder::cuboid(2., 0.1, 2.), source);
        physics.refresh_queries();
        let add = |contact: &mut FluidContact, position: Vector| {
            contact.particles.push(FluidParticle {
                position,
                size: 0.01,
                color: [1.; 4],
                start_color: [1.; 4],
                rotation: 0.,
                age: 0.,
                velocity: Vector::new(0., -10., 0.),
                remaining: reference.lifetime_seconds,
            })
        };
        let advance = |contact: &mut FluidContact, dt, player| {
            contact.advance(
                &reference,
                Pose::IDENTITY,
                Vector::Y,
                player,
                Vector::ZERO,
                0.,
                0.,
                &physics,
                dt,
                0.,
                0.,
                Some(1.),
            )
        };
        let particle_capacity = contact.particles.capacity();
        let collision_capacity = contact.collisions.capacity();
        add(&mut contact, Vector::new(0., 0.5, 0.));
        advance(&mut contact, 0.1, None);
        assert_eq!(contact.collisions().len(), 1);
        assert_eq!(contact.collisions()[0].collider, collider);
        assert!(
            (contact.collisions()[0].position.y - 0.1).abs() < 1e-5,
            "receipt stores the world witness, not the particle center"
        );
        assert!(contact.particles()[0].velocity.y > 0.);
        advance(&mut contact, 0., None);
        assert!(contact.collisions().is_empty());
        assert!(!contact.player_hit());
        assert_eq!(contact.last_poured, 0.);
        for _ in 0..256 {
            contact.particles.clear();
            add(&mut contact, Vector::ZERO);
            advance(&mut contact, 0.1, None);
            assert_eq!(
                contact.collisions().len(),
                1,
                "initial overlap with the emitter body must record a hit"
            );
            assert!(contact.particles().is_empty());
        }
        contact
            .visual
            .curves
            .get_mut("CollisionModule.m_EnergyLossOnCollision")
            .unwrap()
            .scalar = 1.;
        add(&mut contact, Vector::new(0., 0.5, 0.));
        advance(&mut contact, 0.1, None);
        assert_eq!(
            contact.collisions().len(),
            1,
            "record the hit before collision lifetime loss removes the particle"
        );
        assert!(contact.particles().is_empty());
        assert_eq!(contact.particles.capacity(), particle_capacity);
        assert_eq!(contact.collisions.capacity(), collision_capacity);
        add(&mut contact, Vector::ZERO);
        advance(&mut contact, 0.1, Some(source));
        assert!(
            contact.collisions().is_empty(),
            "the synthetic drink collider still excludes its player body"
        );
        contact.particles.clear();
        add(&mut contact, Vector::new(10., 0., 0.));
        contact.advance(
            &reference,
            Pose::IDENTITY,
            Vector::Y,
            None,
            Vector::new(10., 0., 0.),
            1.,
            1.,
            &physics,
            0.1,
            0.,
            0.,
            Some(1.),
        );
        assert!(contact.player_hit());
        advance(&mut contact, 0., None);
        assert!(!contact.player_hit());
        assert!(contact.collisions().is_empty());
    }
}
