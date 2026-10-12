//! Source sandstorm simulation. Presentation consumes this state without advancing it.
mod particles;
use crate::reference::{Bounds, WeatherReference};
use crate::vehicle::PhysicsWorld;
pub use particles::{ParticleOutput, ParticleState};
pub(crate) use particles::{color as particle_color, curve as particle_curve};
use rapier3d::math::{Pose, Vector};
use rapier3d::prelude::{Ball, Collider, ColliderHandle, QueryFilter, RigidBodyHandle};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StormId(pub u32);

#[derive(Clone, Copy, Debug, Default)]
pub struct StormInfluence {
    pub id: Option<StormId>,
    pub strength: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct ShelterVolume {
    pub root: RigidBodyHandle,
    pub bounds: Bounds,
}

#[derive(Clone, Copy, Debug)]
struct WindTarget {
    body: RigidBodyHandle,
    root: RigidBodyHandle,
    extent: f32,
}

#[derive(Debug)]
pub struct Storm {
    pub id: StormId,
    pub absolute_position: [f64; 3],
    pub yaw_degrees: f32,
    pub size: f32,
    pub life: f32,
    pub direction: [f32; 2],
    pub speed: f32,
    /// Source currentDistance, consumed by Move before this step recomputes it.
    pub previous_distance: f32,
    /// Source llerp, sampled before Move changes life.
    pub life_strength: f32,
    pub strength: f32,
    pub current_force: [f32; 3],
    pub next_force: [f32; 3],
    next_wind_time: f64,
    next_scan_time: f64,
    scan_position: [f64; 3],
    wind_strength: f32,
    wind_force: f32,
    wind_bounds_range: [f32; 2],
    targets: Vec<WindTarget>,
    pub particles: [ParticleState; 2],
    last_player: [f64; 3],
}

pub struct WeatherState {
    storms: Vec<Storm>,
    influence: StormInfluence,
    next_spawn_time: f64,
    next_id: u32,
    random: StormRandom,
    origin: [f64; 3],
    body_capacity: usize,
    ambient_modifier: f32,
    ambient_only_modifier: f32,
}

impl WeatherState {
    pub fn new(reference: &WeatherReference, seed: u64) -> Self {
        let mut random = StormRandom(seed ^ 0x7374_6f72_6d73_0001);
        let next_spawn_time = random.range(
            reference.min_interval_seconds,
            reference.max_interval_seconds,
        ) as f64;
        Self {
            storms: Vec::new(),
            influence: StormInfluence::default(),
            next_spawn_time,
            next_id: 0,
            random,
            origin: [0.; 3],
            body_capacity: 0,
            ambient_modifier: reference.ambient.modifier,
            ambient_only_modifier: reference.ambient.only_modifier,
        }
    }

    /// Source desert day/night channels, before the fixed update recovers both modifiers.
    pub fn ambient_gains(
        &mut self,
        reference: &WeatherReference,
        cycle: &crate::lighting::CycleOutput,
        dt: f32,
    ) -> [f32; 2] {
        use crate::lighting::CyclePhase;
        let night = match cycle.phase {
            CyclePhase::Night => 1.,
            CyclePhase::Day => 0.,
            CyclePhase::Dawn => 1. - cycle.progress,
            CyclePhase::Dusk => cycle.progress,
        };
        let biome_gain = cycle.biome_color[0] - cycle.biome_color[2];
        let modifier = self.ambient_modifier * self.ambient_only_modifier * biome_gain;
        let reduction = reference.storm.ambient_decrease * self.influence.strength.clamp(0., 1.);
        let gains = [reference.ambient.day_volume, reference.ambient.night_volume]
            .map(|v| ((v[0] + (v[1] - v[0]) * night) * modifier - reduction).clamp(0., 1.));
        self.ambient_modifier =
            (self.ambient_modifier + dt * reference.ambient.recovery_per_second).clamp(0., 1.);
        self.ambient_only_modifier =
            (self.ambient_only_modifier + dt * reference.ambient.recovery_per_second).clamp(0., 1.);
        gains
    }
    pub fn storms(&self) -> &[Storm] {
        &self.storms
    }
    pub fn influence(&self) -> StormInfluence {
        self.influence
    }
    pub fn next_spawn_time(&self) -> f64 {
        self.next_spawn_time
    }

    /// Call when bodies enter the world, before a scan. Storm creation reserves this capacity.
    pub fn reserve_bodies(&mut self, count: usize) {
        if count <= self.body_capacity {
            return;
        }
        self.body_capacity = count;
        for storm in &mut self.storms {
            storm
                .targets
                .reserve(count.saturating_sub(storm.targets.len()));
        }
    }

    /// Absolute storm positions and deadlines do not change with the physics origin.
    pub fn set_origin(&mut self, origin: [f64; 3]) {
        self.origin = origin;
    }

    pub fn spawn_at(&mut self, reference: &WeatherReference, position: [f64; 3]) -> StormId {
        let r = &reference.storm;
        let id = StormId(self.next_id);
        self.next_id = self
            .next_id
            .checked_add(1)
            .expect("storm ID space exhausted");
        // SpawnAt draws prefab yaw before Start calls FStart: size, direction, then speed.
        let yaw_degrees = self.random.range(0., 360.);
        let size = self.random.range(r.size_range[0], r.size_range[1]);
        let angle = self.random.range(0., std::f32::consts::TAU);
        let speed = self.random.range(r.speed_range[0], r.speed_range[1]);
        self.storms.push(Storm {
            id,
            absolute_position: position,
            yaw_degrees,
            size,
            life: r.life,
            direction: [angle.cos(), angle.sin()],
            speed,
            previous_distance: 0.,
            life_strength: 0.,
            strength: r.distance_lerp,
            current_force: r.current_force,
            next_force: r.next_force,
            next_wind_time: 0.,
            next_scan_time: f64::NEG_INFINITY,
            scan_position: position,
            wind_strength: r.distance_lerp,
            wind_force: r.wind_force,
            wind_bounds_range: r.wind_bounds_range,
            targets: Vec::with_capacity(self.body_capacity),
            particles: std::array::from_fn(|i| {
                let role = if i == 0 { "storm_big" } else { "storm_small" };
                let mut p = ParticleState::new(
                    reference.particles.iter().find(|p| p.role == role).unwrap(),
                    id.0 as u64 * 2 + i as u64 + 1,
                );
                p.radius_scale = size;
                p.rotation = rapier3d::math::Rotation::from_rotation_y(-yaw_degrees.to_radians());
                p
            }),
            last_player: self.origin,
        });
        id
    }

    /// now is normal simulation time, including normal sleep time scale, without extra cycle time.
    pub fn advance(
        &mut self,
        reference: &WeatherReference,
        now: f64,
        dt: f32,
        player_absolute: [f64; 3],
        world_seed: u64,
    ) {
        if !now.is_finite()
            || !dt.is_finite()
            || dt <= 0.
            || !player_absolute.iter().all(|v| v.is_finite())
        {
            return;
        }
        if now >= self.next_spawn_time {
            self.next_spawn_time = now
                + self.random.range(
                    reference.min_interval_seconds,
                    reference.max_interval_seconds,
                ) as f64;
            let angle = self.random.range(0., std::f32::consts::TAU);
            let mut position = player_absolute;
            position[0] += (angle.cos() * reference.spawn_radius) as f64;
            position[2] += (angle.sin() * reference.spawn_radius) as f64;
            self.spawn_at(reference, position);
        }
        let r = &reference.storm;
        self.influence = StormInfluence::default();
        for storm in &mut self.storms {
            storm.life_strength = storm.size * storm.life.clamp(0., 1.);
            storm.scan_position = storm.absolute_position;
            let height = crate::world::terrain_height(world_seed, storm.absolute_position);
            storm.absolute_position[0] += (storm.direction[0] * storm.speed * dt) as f64;
            storm.absolute_position[1] = height as f64;
            storm.absolute_position[2] += (storm.direction[1] * storm.speed * dt) as f64;
            // The current procedural desert maps to source biome zero.
            if r.biome != 0 || storm.previous_distance >= r.destroy_dist {
                storm.life = (storm.life - dt * r.life_loss_on_other_biome).clamp(0., 1.);
            } else {
                storm.life = (storm.life + dt * r.life_gain).clamp(0., 1.);
            }
            if now > storm.next_wind_time {
                storm.next_wind_time = now
                    + self
                        .random
                        .range(r.wind_time_range[0], r.wind_time_range[1])
                        as f64;
                storm.next_force = self.random.inside_sphere();
            }
            for i in 0..3 {
                storm.current_force[i] +=
                    (storm.next_force[i] - storm.current_force[i]) * dt.clamp(0., 1.);
            }
            // Wind precedes the current Update's distanceLerp assignment.
            storm.wind_strength = storm.strength;
            storm.previous_distance = storm
                .absolute_position
                .iter()
                .zip(player_absolute)
                .map(|(a, b)| (a - b).powi(2))
                .sum::<f64>()
                .sqrt() as f32;
            storm.strength = ((r.radius - storm.previous_distance) / (r.radius * 0.5))
                .clamp(0., 1.)
                * storm.life_strength;
            if storm.strength > self.influence.strength {
                self.influence = StormInfluence {
                    id: Some(storm.id),
                    strength: storm.strength,
                };
            }
        }
        self.storms.retain(|storm| storm.life > 0.);
    }

    pub fn advance_particles(
        &mut self,
        reference: &WeatherReference,
        dt: f32,
        player: [f64; 3],
        night: f32,
        shelters: &[ShelterVolume],
    ) {
        for storm in &mut self.storms {
            let small = std::array::from_fn(|i| {
                (player[i]
                    + (player[i] - storm.last_player[i]) / dt.clamp(0.0001, 1.) as f64 * 2.
                    + storm.last_player[i]
                    + if i == 1 {
                        reference.storm.yoffset as f64
                    } else {
                        0.
                    })
                    * 0.5
            });
            for (i, p) in storm.particles.iter_mut().enumerate() {
                let first = crate::lighting::gradient(&reference.storm.day_color1, p.time());
                let second = crate::lighting::gradient(&reference.storm.day_color2, p.time());
                let draw = p.random_color_draw();
                p.start_color = Some(std::array::from_fn(|j| {
                    let day = first[j] + (second[j] - first[j]) * draw;
                    day + (reference.storm.night_color[j] - day) * night
                }));
                p.origin = self.origin;
                p.advance(
                    dt,
                    if i == 0 {
                        storm.absolute_position
                    } else {
                        small
                    },
                    if i == 0 {
                        storm.life_strength
                    } else {
                        storm.strength
                    },
                    shelters,
                );
            }
            storm.last_player = player;
        }
    }

    /// Refresh cached ownership before forces, independently of overlap scan deadlines.
    pub(crate) fn refresh_roots(
        &mut self,
        physics: &PhysicsWorld,
        roots: &[(RigidBodyHandle, RigidBodyHandle)],
    ) {
        for storm in &mut self.storms {
            // Root changes take effect without waiting for the next overlap scan.
            storm.targets.retain_mut(|target| {
                let Some((_, root)) = roots.iter().find(|(body, _)| *body == target.body) else {
                    return false;
                };
                target.root = *root;
                physics.rapier().bodies.contains(target.body)
            });
        }
    }

    pub fn scan_wind(
        &mut self,
        physics: &PhysicsWorld,
        roots: &[(RigidBodyHandle, RigidBodyHandle)],
        reference: &WeatherReference,
        now: f64,
    ) {
        self.refresh_roots(physics, roots);
        let r = &reference.storm;
        let mask = |_: ColliderHandle, collider: &Collider| {
            collider.is_enabled() && collider.collision_groups().memberships.bits() & r.wind_lm != 0
        };
        let queries = physics.queries(QueryFilter::default().predicate(&mask));
        for storm in &mut self.storms {
            if now < storm.next_scan_time {
                continue;
            }
            let center = if storm.next_scan_time == f64::NEG_INFINITY {
                storm.scan_position
            } else {
                storm.absolute_position
            };
            storm.next_scan_time = now + r.wind_scan_seconds as f64;
            storm.targets.clear();
            let position =
                Vector::from_array(std::array::from_fn(|i| (center[i] - self.origin[i]) as f32));
            for (_, collider) in
                queries.intersect_shape(Pose::from_translation(position), &Ball::new(r.radius))
            {
                let Some(body) = collider.parent() else {
                    continue;
                };
                let Some((_, root)) = roots.iter().find(|(handle, _)| *handle == body) else {
                    continue;
                };
                let bounds = collider.compute_aabb();
                let extent = (bounds.maxs - bounds.mins).element_sum();
                if let Some(target) = storm.targets.iter_mut().find(|target| target.body == body) {
                    target.extent += extent;
                } else {
                    assert!(
                        storm.targets.len() < storm.targets.capacity(),
                        "reserve wind bodies at creation"
                    );
                    storm.targets.push(WindTarget {
                        body,
                        root: *root,
                        extent,
                    });
                }
            }
        }
    }

    pub fn apply_wind(&mut self, physics: &mut PhysicsWorld, shelters: &[ShelterVolume], dt: f32) {
        if !dt.is_finite() || dt <= 0. {
            return;
        }
        let integration_dt = physics.rapier().integration_parameters.dt;
        for storm in &self.storms {
            for target in &storm.targets {
                let Some(body) = physics.rapier_mut().bodies.get_mut(target.body) else {
                    continue;
                };
                if !body.is_enabled() || !body.is_dynamic() {
                    continue;
                }
                let position = body.translation();
                if shelters.iter().any(|s| {
                    s.root != target.root
                        && (0..3).all(|i| {
                            position[i] >= s.bounds.min[i] && position[i] <= s.bounds.max[i]
                        })
                }) {
                    continue;
                }
                // Unity AddForce defaults to Force. Preserve its explicit delta, then integrate once.
                let scale = storm.wind_force
                    * dt
                    * target
                        .extent
                        .clamp(storm.wind_bounds_range[0], storm.wind_bounds_range[1])
                    * storm.wind_strength
                    * integration_dt;
                body.apply_impulse(Vector::from_array(storm.current_force) * scale, true);
            }
        }
    }
}

/// A separate deterministic stream. Unity's native random sequence is unrecovered.
#[derive(Debug)]
struct StormRandom(u64);
impl StormRandom {
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
