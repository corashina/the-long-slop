//! Selected Unity particle modules. Absolute positions keep custom space stable across rebases.
use crate::reference::{ParticleColor, ParticleCurve, ParticleReference};
use crate::vehicle::PhysicsWorld;
use rapier3d::math::{Pose, Rotation, Vector};
use rapier3d::parry::query::ShapeCastOptions;
use rapier3d::prelude::{Ball, ColliderHandle, QueryFilter};

#[derive(Clone, Copy, Debug)]
pub struct ParticleOutput {
    pub absolute_position: [f64; 3],
    pub size: f32,
    pub color: [f32; 4],
    pub rotation: f32,
    pub age: f32,
    pub lifetime: f32,
    velocity: Vector,
    force: Vector,
    start_size: f32,
    start_color: [f32; 4],
    rotation_speed: f32,
    size_draw: f32,
    velocity_draw: [f32; 3],
    inherit_draw: f32,
    collision_draw: f32,
    uv_draw: f32,
}

#[derive(Debug)]
pub struct ParticleState {
    reference: ParticleReference,
    particles: Vec<ParticleOutput>,
    random: ParticleRandom,
    remainder: f32,
    elapsed: f32,
    last_emitter: Option<[f64; 3]>,
    pub(crate) rotation: Rotation,
    pub(crate) emission_rotation: Option<Rotation>,
    /// Scales the shape radius. Storms set it to their size.
    pub(crate) radius_scale: f32,
    pub(crate) start_color: Option<[f32; 4]>,
    pub(crate) origin: [f64; 3],
    burst_started: bool,
}
impl ParticleState {
    pub fn new(reference: &ParticleReference, seed: u64) -> Self {
        Self {
            reference: reference.clone(),
            particles: Vec::with_capacity(reference.capacity as usize),
            random: ParticleRandom(seed),
            remainder: 0.,
            elapsed: 0.,
            last_emitter: None,
            rotation: Rotation::IDENTITY,
            emission_rotation: None,
            radius_scale: 1.,
            start_color: None,
            origin: [0.; 3],
            burst_started: false,
        }
    }
    pub fn particles(&self) -> &[ParticleOutput] {
        &self.particles
    }
    /// Source `ParticleSystem.time`: seconds into the current duration.
    pub fn time(&self) -> f32 {
        self.elapsed
    }
    /// One draw from this system's stream, for a start color set by its owner.
    pub(crate) fn random_draw(&mut self) -> f32 {
        self.random.unit()
    }
    /// Base emission rates. `advance` scales them by its strength.
    pub(crate) fn set_rates(&mut self, over_time: f32, over_distance: f32) {
        self.reference.rate_over_time = over_time;
        self.reference.rate_over_distance = over_distance;
    }
    /// Source trigger module with the Kill inside action: removes particles that touch an
    /// interior collider. `interior` selects those colliders. Call after `advance`.
    pub fn remove_inside(
        &mut self,
        physics: &PhysicsWorld,
        interior: impl Fn(ColliderHandle) -> bool,
    ) {
        let r = &self.reference;
        if r.module_flags.get("TriggerModule") != Some(&true)
            || r.scalars["TriggerModule.inside"] != 1.
        {
            return;
        }
        let scale = 0.5 * r.scalars["TriggerModule.radiusScale"];
        let predicate = |handle: ColliderHandle, _: &rapier3d::prelude::Collider| interior(handle);
        let queries = physics.queries(QueryFilter::default().predicate(&predicate));
        let origin = self.origin;
        self.particles.retain(|p| {
            let position = Vector::from_array(std::array::from_fn(|i| {
                (p.absolute_position[i] - origin[i]) as f32
            }));
            let ball = Ball::new((p.size * scale).max(f32::EPSILON));
            queries
                .intersect_shape(Pose::from_translation(position), &ball)
                .next()
                .is_none()
        });
    }
    /// A new Instantiate of the system: its burst and duration start again.
    pub fn restart(&mut self) {
        self.burst_started = false;
        self.elapsed = 0.;
        self.remainder = 0.;
    }
    pub fn reset_emitter(&mut self) {
        self.last_emitter = None;
    }
    pub fn set_origin(&mut self, origin: [f64; 3]) {
        if self.origin != origin {
            self.reset_emitter();
            self.origin = origin;
        }
    }
    pub fn advance(
        &mut self,
        dt: f32,
        emitter_absolute: [f64; 3],
        strength: f32,
        physics: &PhysicsWorld,
    ) {
        if !self.reference.flags["gameObject.active"] || !dt.is_finite() || dt <= 0. {
            self.reset_emitter();
            return;
        }
        let r = &self.reference;
        let local = Rotation::from_xyzw(
            r.local_pose.rotation[0],
            r.local_pose.rotation[1],
            r.local_pose.rotation[2],
            r.local_pose.rotation[3],
        );
        let rotation = self.emission_rotation.unwrap_or(self.rotation * local);
        let offset = self.rotation * Vector::from_array(r.local_pose.translation);
        let emitter_absolute = std::array::from_fn(|i| emitter_absolute[i] + offset[i] as f64);
        let emitter_velocity = self.last_emitter.map_or(Vector::ZERO, |p| {
            Vector::from_array(std::array::from_fn(|i| {
                ((emitter_absolute[i] - p[i]) / dt as f64) as f32
            }))
        });
        let distance = self.last_emitter.map_or(0., |p| {
            (0..3)
                .map(|i| (p[i] - emitter_absolute[i]).powi(2))
                .sum::<f64>()
                .sqrt() as f32
        });
        self.last_emitter = Some(emitter_absolute);
        self.elapsed = (self.elapsed + dt) % r.duration;
        self.remainder +=
            (r.rate_over_time * dt + r.rate_over_distance * distance) * strength.max(0.);
        let mut count = self.remainder.floor() as usize;
        self.remainder -= count as f32;
        // Authored splash starts with one burst. No managed collision-to-splash link exists.
        if !self.burst_started && r.scalars.get("EmissionModule.m_BurstCount") == Some(&1.) {
            count += r.curves["EmissionModule.m_Bursts.0.countCurve"].scalar as usize;
        }
        self.burst_started = true;
        for _ in 0..count.min(r.capacity as usize - self.particles.len()) {
            let a = self.random.unit() * std::f32::consts::TAU;
            let hemisphere = r.scalars["ShapeModule.type"] == 2.;
            let z = if hemisphere {
                self.random.unit()
            } else {
                r.scalars["ShapeModule.angle"].to_radians().cos()
                    + (1. - r.scalars["ShapeModule.angle"].to_radians().cos()) * self.random.unit()
            };
            let radial = (1. - z * z).sqrt();
            // Source +Z is converted to -Z. Shape rotation uses Unity's ZXY order.
            let direction = Vector::new(a.cos() * radial, a.sin() * radial, -z);
            let thickness = r.scalars["ShapeModule.radiusThickness"];
            let inner = (1. - thickness).powi(3);
            let shape_radius = r.scalars["ShapeModule.radius.value"] * self.radius_scale;
            let radius = shape_radius * (inner + (1. - inner) * self.random.unit()).cbrt();
            let mut position = if hemisphere {
                direction * radius
            } else {
                Vector::new(a.cos(), a.sin(), 0.) * shape_radius * self.random.unit().sqrt()
            };
            // Native EmitterStoreData adds a random unit vector times randomPositionAmount.
            let amount = r.scalars["ShapeModule.randomPositionAmount"];
            if amount > 0. {
                position +=
                    Vector::from_array(self.random.inside_sphere()).normalize_or_zero() * amount;
            }
            position *= Vector::from_array(r.vectors["ShapeModule.m_Scale"]);
            let e = r.vectors["ShapeModule.m_Rotation"];
            let shape = Rotation::from_rotation_y(-e[1].to_radians())
                * Rotation::from_rotation_x(-e[0].to_radians())
                * Rotation::from_rotation_z(e[2].to_radians());
            position = rotation * (shape * position);
            let speed = particle_curve(
                &r.curves["InitialModule.startSpeed"],
                self.elapsed / r.duration,
                self.random.unit(),
            );
            let lifetime = particle_curve(
                &r.curves["InitialModule.startLifetime"],
                self.elapsed / r.duration,
                self.random.unit(),
            );
            let size = particle_curve(
                &r.curves["InitialModule.startSize"],
                self.elapsed / r.duration,
                self.random.unit(),
            );
            let color = self.start_color.unwrap_or_else(|| {
                particle_color(
                    &r.colors["InitialModule.startColor"],
                    0.,
                    self.random.unit(),
                )
            });
            let mut force = Vector::ZERO;
            if r.module_flags.get("ForceModule") == Some(&true) {
                force = Vector::new(
                    particle_curve(&r.curves["ForceModule.x"], 0., self.random.unit()),
                    particle_curve(&r.curves["ForceModule.y"], 0., self.random.unit()),
                    -particle_curve(&r.curves["ForceModule.z"], 0., self.random.unit()),
                );
                if !r.flags["ForceModule.inWorldSpace"] {
                    force = rotation * force;
                }
            }
            force.y -= 9.81
                * particle_curve(
                    &r.curves["InitialModule.gravityModifier"],
                    0.,
                    self.random.unit(),
                );
            let velocity_draw = if r.module_flags.get("VelocityModule") == Some(&true) {
                std::array::from_fn(|_| self.random.unit())
            } else {
                [0.; 3]
            };
            let inherit_draw = if r.module_flags.get("InheritVelocityModule") == Some(&true) {
                self.random.unit()
            } else {
                0.
            };
            let inherited = if r.module_flags.get("InheritVelocityModule") == Some(&true)
                && r.scalars["InheritVelocityModule.m_Mode"] == 0.
            {
                emitter_velocity
                    * particle_curve(&r.curves["InheritVelocityModule.m_Curve"], 0., inherit_draw)
            } else {
                Vector::ZERO
            };
            let collision_draw = if r.module_flags.get("CollisionModule") == Some(&true) {
                self.random.unit()
            } else {
                0.
            };
            let uv_draw = if r.module_flags.get("UVModule") == Some(&true) {
                self.random.unit()
            } else {
                0.
            };
            self.particles.push(ParticleOutput {
                absolute_position: std::array::from_fn(|i| {
                    emitter_absolute[i] + position[i] as f64
                }),
                size,
                color,
                rotation: particle_curve(
                    &r.curves["InitialModule.startRotation"],
                    0.,
                    self.random.unit(),
                ),
                age: 0.,
                lifetime,
                velocity: rotation
                    * (shape
                        * (direction * Vector::from_array(r.vectors["ShapeModule.m_Scale"]))
                            .normalize_or_zero())
                    * speed
                    + inherited,
                force,
                start_size: size,
                start_color: color,
                rotation_speed: r
                    .curves
                    .get("RotationModule.curve")
                    .map_or(0., |c| particle_curve(c, 0., self.random.unit())),
                size_draw: self.random.unit(),
                velocity_draw,
                inherit_draw,
                collision_draw,
                uv_draw,
            });
        }
        let ring = r.scalars["ringBufferMode"] == 1.;
        self.particles.retain_mut(|p| {
            if p.age >= p.lifetime {
                return ring;
            }
            p.age = (p.age + dt).min(p.lifetime);
            if p.age >= p.lifetime && !ring {
                return false;
            }
            p.velocity += p.force * dt;
            let t = p.age / p.lifetime;
            let mut module_velocity = Vector::ZERO;
            let mut speed_modifier = 1.;
            if r.module_flags.get("VelocityModule") == Some(&true) {
                module_velocity = Vector::new(
                    particle_curve(&r.curves["VelocityModule.x"], t, p.velocity_draw[0]),
                    particle_curve(&r.curves["VelocityModule.y"], t, p.velocity_draw[1]),
                    -particle_curve(&r.curves["VelocityModule.z"], t, p.velocity_draw[2]),
                );
                if !r.flags["VelocityModule.inWorldSpace"] {
                    module_velocity = rotation * module_velocity;
                }
                speed_modifier = particle_curve(
                    &r.curves["VelocityModule.speedModifier"],
                    t,
                    p.velocity_draw[0],
                );
            }
            if r.module_flags.get("InheritVelocityModule") == Some(&true)
                && r.scalars["InheritVelocityModule.m_Mode"] == 1.
            {
                module_velocity += emitter_velocity
                    * particle_curve(
                        &r.curves["InheritVelocityModule.m_Curve"],
                        t,
                        p.inherit_draw,
                    );
            }
            let velocity = (p.velocity + module_velocity) * speed_modifier;
            let travel = velocity * dt;
            let mut position = Vector::from_array(std::array::from_fn(|i| {
                (p.absolute_position[i] - self.origin[i]) as f32
            }));
            let mut collided = false;
            // Disabled modules do not access their payload or issue physics queries.
            if r.module_flags.get("CollisionModule") == Some(&true) {
                let mask = r.scalars["CollisionModule.collidesWith.m_Bits"] as u32;
                let surfaces = |_: ColliderHandle, collider: &rapier3d::prelude::Collider| {
                    collider.is_enabled()
                        && !collider.is_sensor()
                        && collider.collision_groups().memberships.bits() & mask != 0
                        && (r.flags["CollisionModule.collidesWithDynamic"]
                            || collider
                                .parent()
                                .is_none_or(|body| !physics.rapier().bodies[body].is_dynamic()))
                };
                let size = p.start_size
                    * r.curves
                        .get("SizeModule.curve")
                        .map_or(1., |c| particle_curve(c, t, p.size_draw));
                if let Some((_, hit)) = physics
                    .queries(QueryFilter::default().predicate(&surfaces))
                    .cast_shape(
                        &Pose::from_parts(position, Rotation::IDENTITY),
                        travel,
                        &Ball::new(size * 0.5 * r.scalars["CollisionModule.radiusScale"]),
                        ShapeCastOptions {
                            max_time_of_impact: 1.,
                            stop_at_penetration: true,
                            ..Default::default()
                        },
                    )
                {
                    // Match the existing fluid swept-sphere route for initial penetration.
                    if hit.time_of_impact <= 0. {
                        return false;
                    }
                    p.age += p.lifetime
                        * particle_curve(
                            &r.curves["CollisionModule.m_EnergyLossOnCollision"],
                            t,
                            p.collision_draw,
                        );
                    if p.age >= p.lifetime {
                        return false;
                    }
                    let normal = hit.normal1.normalize_or_zero();
                    let inward = velocity.dot(normal).min(0.);
                    let bounced = (velocity - normal * inward)
                        * (1.
                            - particle_curve(
                                &r.curves["CollisionModule.m_Dampen"],
                                t,
                                p.collision_draw,
                            ))
                        - normal
                            * inward
                            * particle_curve(
                                &r.curves["CollisionModule.m_Bounce"],
                                t,
                                p.collision_draw,
                            );
                    let speed = bounced.length();
                    if speed < r.scalars["CollisionModule.minKillSpeed"]
                        || speed > r.scalars["CollisionModule.maxKillSpeed"]
                    {
                        return false;
                    }
                    position += travel * hit.time_of_impact + normal * 0.0001;
                    // Preserve the response when lifetime velocity is applied next frame.
                    p.velocity = if speed_modifier > 0. {
                        bounced / speed_modifier - module_velocity
                    } else {
                        Vector::ZERO
                    };
                    collided = true;
                }
            }
            if collided {
                p.absolute_position = std::array::from_fn(|i| self.origin[i] + position[i] as f64);
            } else {
                for i in 0..3 {
                    p.absolute_position[i] += travel[i] as f64;
                }
            }
            p.rotation += p.rotation_speed * dt;
            let t = p.age / p.lifetime;
            p.size = p.start_size
                * r.curves
                    .get("SizeModule.curve")
                    .map_or(1., |c| particle_curve(c, t, p.size_draw));
            let c = r
                .colors
                .get("ColorModule.gradient")
                .map_or([1.; 4], |c| particle_color(c, t, p.size_draw));
            p.color = std::array::from_fn(|i| p.start_color[i] * c[i]);
            true
        });
    }
}

/// Unity texture sheet animation over lifetime on a whole grid sheet.
#[derive(Clone, Debug)]
pub struct TextureSheet {
    pub tiles: [u32; 2],
    frame_over_time: ParticleCurve,
    start_frame: ParticleCurve,
    cycles: f32,
}
impl TextureSheet {
    /// `None` when the UV module is off.
    pub fn new(r: &ParticleReference) -> Option<Self> {
        (r.module_flags.get("UVModule") == Some(&true)).then(|| Self {
            tiles: [
                r.scalars["UVModule.tilesX"] as u32,
                r.scalars["UVModule.tilesY"] as u32,
            ],
            frame_over_time: r.curves["UVModule.frameOverTime"].clone(),
            start_frame: r.curves["UVModule.startFrame"].clone(),
            cycles: r.scalars["UVModule.cycles"],
        })
    }
    pub fn frame(&self, p: &ParticleOutput) -> u32 {
        let t = p.age / p.lifetime;
        sheet_frame(
            particle_curve(&self.frame_over_time, t, p.uv_draw),
            particle_curve(&self.start_frame, t, p.uv_draw),
            self.cycles,
            self.tiles[0] * self.tiles[1],
        )
    }
}
/// Frame from normalized curve values. Frame 0 is the top-left tile, row by row.
fn sheet_frame(over_time: f32, start: f32, cycles: f32, frames: u32) -> u32 {
    let phase = (over_time * cycles + start).rem_euclid(1.);
    ((phase * frames as f32) as u32).min(frames - 1)
}

pub(crate) fn particle_curve(c: &ParticleCurve, t: f32, draw: f32) -> f32 {
    use crate::simulation::engine::torque;
    match c.mode {
        0 => c.scalar,
        1 => c.scalar * torque(&c.max_curve, t),
        2 => {
            c.scalar
                * (torque(&c.min_curve, t)
                    + (torque(&c.max_curve, t) - torque(&c.min_curve, t)) * draw)
        }
        3 => c.min_scalar + (c.scalar - c.min_scalar) * draw,
        _ => unreachable!("validated particle curve"),
    }
}
pub(crate) fn particle_color(c: &ParticleColor, t: f32, draw: f32) -> [f32; 4] {
    let mix = |a: [f32; 4], b: [f32; 4]| std::array::from_fn(|i| a[i] + (b[i] - a[i]) * draw);
    match c.mode {
        0 => c.max_color,
        1 => crate::lighting::gradient(&c.max_gradient, t),
        2 => mix(c.min_color, c.max_color),
        3 => mix(
            crate::lighting::gradient(&c.min_gradient, t),
            crate::lighting::gradient(&c.max_gradient, t),
        ),
        4 => crate::lighting::gradient(&c.max_gradient, draw),
        _ => unreachable!("validated particle color"),
    }
}

/// A separate deterministic stream. Unity's native random sequence is unrecovered.
#[derive(Debug)]
struct ParticleRandom(u64);
impl ParticleRandom {
    fn unit(&mut self) -> f32 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        (crate::world::hash(self.0) >> 40) as f32 / 16_777_216.
    }
    fn range(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.unit()
    }
    fn inside_sphere(&mut self) -> [f32; 3] {
        let vertical = self.range(-1., 1.);
        let angle = self.range(0., std::f32::consts::TAU);
        let radius = self.unit().cbrt();
        let horizontal = (1. - vertical * vertical).sqrt();
        [
            horizontal * angle.cos() * radius,
            vertical * radius,
            horizontal * angle.sin() * radius,
        ]
    }
}

/// The same collision owner serves the player outlet and container outlets.
pub(crate) fn fluid_reference(
    p: &crate::reference::ParticleReference,
) -> crate::reference::FluidParticleReference {
    crate::reference::FluidParticleReference {
        source: p.source.clone(),
        local_pose: p.local_pose.clone(),
        capacity: p.capacity,
        cone_degrees: p.scalars["ShapeModule.angle"],
        cone_radius: p.scalars["ShapeModule.radius.value"],
        lifetime_seconds: p.curves["InitialModule.startLifetime"].scalar,
        radius: p.curves["InitialModule.startSize"].scalar
            * 0.5
            * p.scalars["CollisionModule.radiusScale"],
        gravity_multiplier: p.curves["InitialModule.gravityModifier"].scalar,
        collision_mask: p.scalars["CollisionModule.collidesWith.m_Bits"] as u32,
        bounce: p.curves["CollisionModule.m_Bounce"].scalar,
        dampen: p.curves["CollisionModule.m_Dampen"].scalar,
        max_speed: p.scalars["outlet.maxSpeed"],
        min_rate: p.scalars["outlet.minRate"],
        max_rate: p.scalars["outlet.maxRate"],
    }
}

#[cfg(test)]
mod tests {
    use super::sheet_frame;

    #[test]
    fn texture_sheet_frames() {
        assert_eq!(sheet_frame(0., 0., 1., 25), 0);
        assert_eq!(sheet_frame(0.9999, 0., 1., 25), 24);
        assert_eq!(sheet_frame(0.5, 0., 1., 25), 12);
        // Two cycles wrap back to the first frame at half life.
        assert_eq!(sheet_frame(0.5, 0., 2., 25), 0);
        assert_eq!(sheet_frame(0.9, 0.2, 1., 25), 2);
    }
}
