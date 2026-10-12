//! World-space source fluid particles and collision-driven drinking.

use crate::reference::FluidParticleReference;
use crate::vehicle::PhysicsWorld;
use rapier3d::math::{Pose, Rotation, Vector};
use rapier3d::parry::query::ShapeCastOptions;
use rapier3d::prelude::{Ball, ColliderHandle, QueryFilter, RigidBodyHandle};

struct Particle {
    position: Vector,
    velocity: Vector,
    remaining: f32,
}

pub(super) struct FluidContact {
    particles: Vec<Particle>,
    emission_remainder: f32,
    last_emitter: Option<Vector>,
    random: u32,
    pub(super) last_poured: f32,
}

impl FluidContact {
    pub(super) fn new(reference: &FluidParticleReference, seed: u32) -> Self {
        Self {
            particles: Vec::with_capacity(reference.capacity as usize),
            emission_remainder: 0.,
            last_emitter: None,
            random: seed.max(1),
            last_poured: 0.,
        }
    }

    pub(super) fn rebase(&mut self, shift: Vector) {
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
    pub(super) fn advance(
        &mut self,
        reference: &FluidParticleReference,
        cap_pose: Pose,
        local_axis: Vector,
        source_body: Option<RigidBodyHandle>,
        player_body: Option<RigidBodyHandle>,
        cap_center: Vector,
        cap_radius: f32,
        contact_radius: f32,
        physics: &PhysicsWorld,
        dt: f32,
        rate: f32,
        speed: f32,
        poured: Option<f32>,
    ) -> bool {
        if let Some(poured) = poured {
            self.last_poured = poured;
        }
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
                self.particles.push(Particle {
                    position: emitter.translation
                        + (tangent * offset_angle.cos() + bitangent * offset_angle.sin())
                            * offset_radius,
                    velocity: direction * speed,
                    remaining: reference.lifetime_seconds,
                });
            }
        }
        let mut contact = false;
        let mut index = 0;
        while index < self.particles.len() {
            let particle = &mut self.particles[index];
            particle.remaining -= dt;
            if particle.remaining <= 0. {
                self.particles.swap_remove(index);
                continue;
            }
            let velocity =
                particle.velocity + Vector::new(0., -9.81 * reference.gravity_multiplier, 0.) * dt;
            let travel = velocity * dt;
            particle.velocity = velocity;
            let radius = reference.radius;
            let cap_hit = sphere_hit(particle.position, travel, cap_center, cap_radius + radius);
            let surfaces = |_: ColliderHandle, collider: &rapier3d::prelude::Collider| {
                collider.is_enabled()
                    && !collider.is_sensor()
                    && collider.collision_groups().memberships.bits() & reference.collision_mask
                        != 0
                    && source_body.is_none_or(|body| collider.parent() != Some(body))
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
                )
                .map(|(_, hit)| hit);
            if let Some(time) = cap_hit
                && (particle.position + travel * time - cap_center).length() < contact_radius
                && world_hit
                    .as_ref()
                    .is_none_or(|obstruction| time < obstruction.time_of_impact)
            {
                contact = true;
                self.particles.swap_remove(index);
            } else if let Some(hit) = world_hit {
                if hit.time_of_impact <= 0. {
                    self.particles.swap_remove(index);
                    continue;
                }
                let normal = hit.normal2.normalize_or_zero();
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
        contact
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
