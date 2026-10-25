//! Source sandstorm simulation on the fixed step. Presentation reads this state and does not advance it.
use crate::particles::ParticleState;
use crate::reference::WeatherReference;
use crate::save::{StormSnapshot, WeatherSnapshot};
use crate::vehicle::PhysicsWorld;
use rapier3d::math::{Pose, Rotation, Vector};
use rapier3d::prelude::{Ball, Collider, ColliderHandle, QueryFilter, RigidBodyHandle};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StormId(pub u32);

#[derive(Clone, Copy, Debug, Default)]
pub struct StormInfluence {
    pub id: Option<StormId>,
    pub strength: f32,
}

/// A live interior volume and the root body that owns it. `None` is a static building.
#[derive(Clone, Copy, Debug)]
pub struct InteriorVolume {
    pub collider: ColliderHandle,
    pub root: Option<RigidBodyHandle>,
}

#[derive(Clone, Copy, Debug)]
struct WindTarget {
    body: RigidBodyHandle,
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
    /// Source currentDistance, read by Move before this step computes it again.
    pub previous_distance: f32,
    /// Source llerp, sampled before Move changes life.
    pub life_strength: f32,
    /// Source distanceLerp.
    pub strength: f32,
    pub current_force: [f32; 3],
    pub next_force: [f32; 3],
    next_wind_time: f64,
    next_scan_time: f64,
    scan_position: [f64; 3],
    /// Source distanceLerp when Wind runs, before this step computes it again.
    wind_strength: f32,
    targets: Vec<WindTarget>,
    /// Big and small dust systems.
    pub particles: [ParticleState; 2],
    /// Source lastPos: the player position of the previous particle step.
    last_player: Option<[f64; 3]>,
    /// Small dust emitter. It stays in place while the player is outside `radius`.
    small_position: [f64; 3],
}

pub struct WeatherState {
    storms: Vec<Storm>,
    influence: StormInfluence,
    /// Source lastTime.
    last_spawn_time: f64,
    /// Source nextTime: the wait from lastTime to the next spawn.
    wait: f32,
    next_id: u32,
    random: StormRandom,
    origin: [f64; 3],
    body_capacity: usize,
}

impl WeatherState {
    pub fn new(reference: &WeatherReference, seed: u64) -> Self {
        let mut random = StormRandom(seed ^ 0x7374_6f72_6d73_0001);
        let wait = random.range(
            reference.min_interval_seconds,
            reference.max_interval_seconds,
        );
        Self {
            storms: Vec::new(),
            influence: StormInfluence::default(),
            last_spawn_time: 0.,
            wait,
            next_id: 0,
            random,
            origin: [0.; 3],
            body_capacity: 0,
        }
    }
    pub fn storms(&self) -> &[Storm] {
        &self.storms
    }
    pub fn influence(&self) -> StormInfluence {
        self.influence
    }
    pub fn next_spawn_time(&self) -> f64 {
        self.last_spawn_time + self.wait as f64
    }

    /// Source save_sandstorm. `now` is the simulation time that `advance` reads.
    pub fn snapshot(&self, reference: &WeatherReference, now: f64) -> WeatherSnapshot {
        WeatherSnapshot {
            since_spawn: (now - self.last_spawn_time) as f32,
            wait: self.wait,
            storms: self
                .storms
                .iter()
                .map(|storm| StormSnapshot {
                    position: storm.absolute_position,
                    yaw: storm.yaw_degrees,
                    prefab: reference.prefab_id,
                    life: storm.life,
                    size: storm.size,
                    speed: storm.speed,
                    direction: storm.direction,
                })
                .collect(),
        }
    }

    /// Source load: storms start with saved values and do not draw new ones.
    /// Wind targets, gust force and particles start empty and rebuild.
    pub fn restore(&mut self, reference: &WeatherReference, now: f64, saved: &WeatherSnapshot) {
        self.last_spawn_time = now - saved.since_spawn as f64;
        self.wait = saved.wait;
        self.storms.clear();
        self.influence = StormInfluence::default();
        for storm in &saved.storms {
            self.push_storm(reference, storm);
        }
    }

    /// Call when bodies enter the world. Wind scans then do not grow target buffers.
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

    /// Storm positions and deadlines are absolute. The origin only maps them to physics space.
    pub fn set_origin(&mut self, origin: [f64; 3]) {
        self.origin = origin;
    }

    pub fn spawn_at(&mut self, reference: &WeatherReference, position: [f64; 3]) -> StormId {
        let r = &reference.storm;
        // SpawnAt draws prefab yaw before Start calls FStart: size, direction, then speed.
        let yaw_degrees = self.random.range(0., 360.);
        let size = self.random.range(r.size_range[0], r.size_range[1]);
        let angle = self.random.range(0., std::f32::consts::TAU);
        let speed = self.random.range(r.speed_range[0], r.speed_range[1]);
        self.push_storm(
            reference,
            &StormSnapshot {
                position,
                yaw: yaw_degrees,
                prefab: reference.prefab_id,
                life: r.life,
                size,
                speed,
                direction: [angle.cos(), angle.sin()],
            },
        )
    }

    /// Source Start with started set: the storm keeps the given values.
    fn push_storm(&mut self, reference: &WeatherReference, saved: &StormSnapshot) -> StormId {
        let r = &reference.storm;
        let &StormSnapshot {
            position,
            yaw: yaw_degrees,
            life,
            size,
            speed,
            direction,
            ..
        } = saved;
        let id = StormId(self.next_id);
        self.next_id = self
            .next_id
            .checked_add(1)
            .expect("storm ID space exhausted");
        self.storms.push(Storm {
            id,
            absolute_position: position,
            yaw_degrees,
            size,
            life,
            direction,
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
            targets: Vec::with_capacity(self.body_capacity),
            particles: std::array::from_fn(|i| {
                // Validated order: big, then small.
                let mut p =
                    ParticleState::new(&reference.particles[i], id.0 as u64 * 2 + i as u64 + 1);
                p.rotation = Rotation::from_rotation_y(-yaw_degrees.to_radians());
                p.radius_scale = size;
                // Update sets emission rates from defBRate* and defSRate*, scaled by strength.
                let [time, distance] = if i == 0 {
                    [r.def_b_rate_t, r.def_b_rate_d]
                } else {
                    [r.def_s_rate_t, r.def_s_rate_d]
                };
                p.set_rates(time, distance);
                p
            }),
            last_player: None,
            small_position: position,
        });
        id
    }

    /// `now` is normal simulation time, with the sleep time scale and no extra cycle time.
    /// `terrain` gives the terrain height and color at an absolute position.
    pub fn advance(
        &mut self,
        reference: &WeatherReference,
        now: f64,
        dt: f32,
        player_absolute: [f64; 3],
        mut terrain: impl FnMut([f64; 3]) -> (f32, [f32; 4]),
    ) {
        if !now.is_finite()
            || !dt.is_finite()
            || dt <= 0.
            || !player_absolute.iter().all(|v| v.is_finite())
        {
            return;
        }
        if now >= self.next_spawn_time() {
            self.last_spawn_time = now;
            self.wait = self.random.range(
                reference.min_interval_seconds,
                reference.max_interval_seconds,
            );
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
            let (height, color) = terrain(storm.absolute_position);
            storm.absolute_position[0] += (storm.direction[0] * storm.speed * dt) as f64;
            storm.absolute_position[1] = height as f64;
            storm.absolute_position[2] += (storm.direction[1] * storm.speed * dt) as f64;
            // The terrain color already holds the game mode override, as for footsteps.
            let biome = crate::world::biome(color, None);
            if r.biome != biome || storm.previous_distance >= r.destroy_dist {
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
            // Wind runs before this Update sets distanceLerp.
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

    /// Source dust after `advance`. `night` is the SetColorLerp weight: 0 day, 1 night.
    /// `interior` selects interior colliders. Particles that touch them are removed.
    pub fn advance_particles(
        &mut self,
        reference: &WeatherReference,
        dt: f32,
        player_absolute: [f64; 3],
        night: f32,
        physics: &PhysicsWorld,
        interior: impl Fn(ColliderHandle) -> bool,
    ) {
        if !dt.is_finite() || dt <= 0. {
            return;
        }
        let r = &reference.storm;
        for storm in &mut self.storms {
            let last = storm.last_player.unwrap_or(player_absolute);
            storm.last_player = Some(player_absolute);
            if storm.previous_distance < r.radius {
                storm.small_position = small_emitter(player_absolute, last, dt, r.yoffset);
            }
            let emitters = [
                (storm.absolute_position, storm.life_strength),
                (storm.small_position, storm.strength),
            ];
            for (p, (position, strength)) in storm.particles.iter_mut().zip(emitters) {
                let first = crate::lighting::gradient(&r.day_color1, p.time());
                let second = crate::lighting::gradient(&r.day_color2, p.time());
                let draw = p.random_draw();
                p.start_color = Some(std::array::from_fn(|j| {
                    let day = first[j] + (second[j] - first[j]) * draw;
                    day + (r.night_color[j] - day) * night
                }));
                p.set_origin(self.origin);
                p.advance(dt, position, strength, physics);
                p.remove_inside(physics, &interior);
            }
        }
    }

    /// Live dust particles of all storms.
    pub fn particle_count(&self) -> usize {
        self.storms
            .iter()
            .flat_map(|s| &s.particles)
            .map(|p| p.particles().len())
            .sum()
    }

    /// Collects wind targets once per `wind_scan_seconds` for each storm.
    /// `root_of` gives a body's root, or `None` for a body that wind does not move.
    pub fn scan_wind(
        &mut self,
        physics: &PhysicsWorld,
        reference: &WeatherReference,
        now: f64,
        root_of: impl Fn(RigidBodyHandle) -> Option<RigidBodyHandle>,
    ) {
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
                let Some(body) = collider.parent().filter(|body| root_of(*body).is_some()) else {
                    continue;
                };
                let bounds = collider.compute_aabb();
                let extent = (bounds.maxs - bounds.mins).element_sum();
                if let Some(target) = storm.targets.iter_mut().find(|t| t.body == body) {
                    target.extent += extent;
                } else {
                    storm.targets.push(WindTarget { body, extent });
                }
            }
        }
    }

    /// Source Wind. Skips a body inside an interior of another root.
    pub fn apply_wind(
        &self,
        physics: &mut PhysicsWorld,
        reference: &WeatherReference,
        interiors: &[InteriorVolume],
        dt: f32,
        root_of: impl Fn(RigidBodyHandle) -> Option<RigidBodyHandle>,
    ) {
        if !dt.is_finite() || dt <= 0. {
            return;
        }
        let r = &reference.storm;
        let integration_dt = physics.rapier().integration_parameters.dt;
        for storm in &self.storms {
            for target in &storm.targets {
                let Some(root) = root_of(target.body) else {
                    continue;
                };
                let world = physics.rapier();
                let Some(body) = world.bodies.get(target.body) else {
                    continue;
                };
                if !body.is_enabled() || !body.is_dynamic() {
                    continue;
                }
                let position = body.translation();
                if interiors.iter().any(|interior| {
                    interior.root != Some(root)
                        && world
                            .colliders
                            .get(interior.collider)
                            .is_some_and(|c| c.shape().contains_point(c.position(), position))
                }) {
                    continue;
                }
                // Unity AddForce uses Force mode. Keep its explicit dt, then integrate one step.
                let scale = r.wind_force
                    * dt
                    * target
                        .extent
                        .clamp(r.wind_bounds_range[0], r.wind_bounds_range[1])
                    * storm.wind_strength
                    * integration_dt;
                let impulse = Vector::from_array(storm.current_force) * scale;
                physics.edit_body(target.body, |body| body.apply_impulse(impulse, true));
            }
        }
    }
}

/// Source small dust position: halfway between lastPos and the player, led by twice its velocity.
fn small_emitter(player: [f64; 3], last: [f64; 3], dt: f32, yoffset: f32) -> [f64; 3] {
    let dt = dt.clamp(0.0001, 1.) as f64;
    std::array::from_fn(|i| {
        let up = if i == 1 { yoffset as f64 } else { 0. };
        (player[i] + (player[i] - last[i]) / dt * 2. + up + last[i]) * 0.5
    })
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

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::reference::{
        GradientReference, ManagedEvidence, ParticleReference, SourceId, StormReference, Transform,
    };
    use rapier3d::prelude::{ColliderBuilder, InteractionGroups, RigidBodyBuilder};

    const DESERT: [f32; 4] = [1., 0., 0., 1.];
    const FOREST: [f32; 4] = [0., 0., 1., 1.];

    fn source() -> SourceId {
        SourceId {
            file: "level1".into(),
            path_id: 1,
        }
    }
    fn particle(role: &str) -> ParticleReference {
        ParticleReference {
            source: source(),
            role: role.into(),
            capacity: 4,
            rate_over_time: 0.,
            rate_over_distance: 0.,
            duration: 1.,
            simulation_space: 1,
            local_pose: Transform {
                translation: [0.; 3],
                rotation: [0., 0., 0., 1.],
                scale: [1.; 3],
            },
            module_flags: Default::default(),
            curves: Default::default(),
            colors: Default::default(),
            vectors: Default::default(),
            scalars: Default::default(),
            flags: Default::default(),
            links: Default::default(),
            materials: Vec::new(),
            limits: Vec::new(),
        }
    }
    pub(crate) fn reference() -> WeatherReference {
        let gradient = || GradientReference {
            mode: 0,
            colors: Vec::new(),
            alphas: Vec::new(),
        };
        WeatherReference {
            source: source(),
            prefab_id: 0,
            min_interval_seconds: 1200.,
            max_interval_seconds: 5400.,
            spawn_radius: 15000.,
            storm: StormReference {
                source: source(),
                biome: 0,
                life_loss_on_other_biome: 0.5,
                life_gain: 0.25,
                destroy_dist: 20000.,
                life: 0.5,
                radius: 1000.,
                fog_density: 0.,
                atmosphere_thickness: 0.,
                intensity_multipler: 0.,
                ambient_multipler: 0.,
                volume: 1.,
                ambient_decrease: 0.,
                yoffset: 0.,
                wind_force: 2.,
                current_force: [0.; 3],
                next_force: [0.; 3],
                distance_lerp: 0.,
                def_s_rate_t: 0.,
                def_s_rate_d: 0.,
                def_b_rate_t: 0.,
                def_b_rate_d: 0.,
                night_color: [0.; 4],
                sky_color: [0.; 4],
                sky_ground_color: [0.; 4],
                fog_color: [0.; 4],
                cloud_color: [0.; 4],
                day_color1: gradient(),
                day_color2: gradient(),
                wind_lm: 1,
                size_range: [0.5, 1.5],
                speed_range: [3., 12.],
                wind_time_range: [0.1, 1.5],
                wind_scan_seconds: 1.,
                wind_bounds_range: [1., 30.],
            },
            evidence: ManagedEvidence {
                assembly_sha256: String::new(),
                methods: Vec::new(),
            },
            particles: vec![particle("storm_big"), particle("storm_small")],
        }
    }
    /// One still storm at the origin. The first spawn is far in the future.
    fn state(r: &WeatherReference) -> WeatherState {
        let mut weather = WeatherState::new(r, 7);
        weather.spawn_at(r, [0.; 3]);
        weather.storms[0].speed = 0.;
        weather.storms[0].size = 1.;
        weather
    }

    #[test]
    fn life_follows_biome_and_distance() {
        let r = reference();
        let mut weather = state(&r);
        weather.advance(&r, 0., 1., [0.; 3], |_| (5., DESERT));
        assert_eq!(weather.storms[0].life, 0.75);
        assert_eq!(weather.storms[0].absolute_position[1], 5.);
        weather.advance(&r, 0., 1., [0.; 3], |_| (0., FOREST));
        assert_eq!(weather.storms[0].life, 0.25);
        // Past destroyDist: the life check reads the previous step distance.
        let far = [r.storm.destroy_dist as f64 + 1., 0., 0.];
        weather.advance(&r, 0., 0.125, far, |_| (0., DESERT));
        assert_eq!(weather.storms[0].life, 0.28125);
        weather.advance(&r, 0., 0.125, far, |_| (0., DESERT));
        assert_eq!(weather.storms[0].life, 0.21875);
        weather.advance(&r, 0., 1., far, |_| (0., FOREST));
        assert!(weather.storms.is_empty());
    }

    #[test]
    fn influence_follows_distance() {
        let r = reference();
        let mut weather = state(&r);
        weather.storms[0].life = 1.;
        weather.advance(&r, 0., 0.01, [r.storm.radius as f64 + 1., 0., 0.], |_| {
            (0., DESERT)
        });
        assert_eq!(weather.storms[0].strength, 0.);
        assert!(weather.influence().id.is_none());
        weather.advance(&r, 0., 0.01, [r.storm.radius as f64 * 0.5, 0., 0.], |_| {
            (0., DESERT)
        });
        let storm = &weather.storms[0];
        assert_eq!(storm.strength, storm.life_strength);
        assert_eq!(storm.life_strength, 1.);
    }

    #[test]
    fn strongest_storm_is_selected() {
        let r = reference();
        let mut weather = state(&r);
        let near = weather.spawn_at(&r, [100., 0., 0.]);
        weather.storms[1].speed = 0.;
        weather.storms[0].absolute_position = [800., 0., 0.];
        for storm in &mut weather.storms {
            storm.size = 1.;
            storm.life = 1.;
        }
        weather.advance(&r, 0., 0.01, [0.; 3], |_| (0., DESERT));
        assert_eq!(weather.influence().id, Some(near));
        assert_eq!(weather.influence().strength, 1.);
    }

    #[test]
    fn spawn_uses_ring_and_interval() {
        let r = reference();
        let mut weather = WeatherState::new(&r, 3);
        let first = weather.next_spawn_time();
        assert!((1200. ..=5400.).contains(&first));
        let player = [10., 2., -20.];
        weather.advance(&r, first, 0.01, player, |_| (0., DESERT));
        let wait = weather.next_spawn_time() - first;
        assert!((1200. ..=5400.).contains(&wait));
        let storm = &weather.storms()[0];
        let d = ((storm.absolute_position[0] - player[0]).powi(2)
            + (storm.absolute_position[2] - player[2]).powi(2))
        .sqrt();
        // Move adds at most one step of travel to the ring.
        assert!((d - 15000.).abs() < 0.2, "{d}");
        assert!((0.5..=1.5).contains(&storm.size));
        assert!((3. ..=12.).contains(&storm.speed));
    }

    #[test]
    fn save_keeps_timer_and_storms() {
        let r = reference();
        let mut weather = WeatherState::new(&r, 5);
        let first = weather.next_spawn_time();
        weather.advance(&r, first, 0.5, [10., 0., 0.], |_| (0., DESERT));
        weather.spawn_at(&r, [-40., 3., 25.]);
        let now = first + 30.;
        weather.advance(&r, now, 0.5, [10., 0., 0.], |_| (0., DESERT));
        let saved = weather.snapshot(&r, now);
        assert_eq!(saved.since_spawn, 30.);
        assert_eq!(saved.storms.len(), 2);
        // A load starts at another time. The wait and storms do not draw new values.
        let mut loaded = WeatherState::new(&r, 9);
        loaded.restore(&r, 100., &saved);
        assert_eq!(loaded.snapshot(&r, 100.), saved);
        assert_eq!(loaded.next_spawn_time(), 100. - 30. + saved.wait as f64);
        let storm = &loaded.storms()[1];
        assert!(storm.targets.is_empty() && storm.particles[0].particles().is_empty());
        loaded.advance(&r, 101., 1., [10., 0., 0.], |_| (0., DESERT));
        let storm = &loaded.storms()[1];
        let before = saved.storms[1];
        assert_eq!(
            (storm.size, storm.speed, storm.direction),
            (before.size, before.speed, before.direction)
        );
        assert_eq!(
            storm.absolute_position[0],
            before.position[0] + (before.direction[0] * before.speed) as f64
        );
    }

    /// Storm reference with emitting dust systems and flat day colors.
    fn dusty() -> WeatherReference {
        let mut r = reference();
        let flat = |value: [f32; 4]| GradientReference {
            mode: 0,
            colors: vec![crate::reference::GradientColor { time: 0., value }],
            alphas: vec![crate::reference::GradientAlpha {
                time: 0.,
                value: value[3],
            }],
        };
        r.storm.day_color1 = flat([1., 0., 0., 1.]);
        r.storm.day_color2 = flat([1., 0., 0., 1.]);
        r.storm.night_color = [0., 0., 1., 1.];
        r.storm.def_b_rate_t = 20.;
        r.storm.def_s_rate_t = 10.;
        r.storm.yoffset = 2.;
        r.particles = ["storm_big", "storm_small"]
            .map(|role| ParticleReference {
                role: role.into(),
                ..crate::exhaust::tests::smoke(1000, 100.)
            })
            .into();
        r
    }

    #[test]
    fn small_dust_follows_player_and_rates_follow_distance() {
        let r = dusty();
        let physics = PhysicsWorld::empty_for_tests();
        let mut weather = state(&r);
        weather.storms[0].life = 1.;
        let step = |weather: &mut WeatherState, player: [f64; 3]| {
            weather.advance(&r, 0., 1., player, |_| (0., DESERT));
            weather.advance_particles(&r, 1., player, 0.5, &physics, |_| false);
        };
        // distanceLerp 0.5 at 750 m. Small dust leads the player by twice its velocity.
        step(&mut weather, [0., 0., 750.]);
        assert_eq!(weather.storms[0].small_position, [0., 1., 750.]);
        step(&mut weather, [0., 0., 751.]);
        assert_eq!(weather.storms[0].small_position, [0., 1., 751.5]);
        let count =
            |weather: &WeatherState, i: usize| weather.storms[0].particles[i].particles().len();
        assert_eq!(count(&weather, 0), 40, "defBRateT * llerp");
        assert!(
            (count(&weather, 1) as i32 - 10).abs() <= 1,
            "defSRateT * distanceLerp"
        );
        // Day red lerped halfway to the night color.
        let color = weather.storms[0].particles[0].particles()[0].color;
        assert_eq!(color, [0.5, 0., 0.5, 1.]);
        // Outside radius: the small emitter stops and stays.
        let small = count(&weather, 1);
        step(&mut weather, [0., 0., 1100.]);
        assert_eq!(weather.storms[0].small_position, [0., 1., 751.5]);
        assert_eq!(count(&weather, 1), small);
        assert_eq!(count(&weather, 0), 60);
    }

    #[test]
    fn interior_removes_dust() {
        let mut r = dusty();
        for p in &mut r.particles {
            p.module_flags.insert("TriggerModule".into(), true);
            p.scalars.insert("TriggerModule.inside".into(), 1.);
            p.scalars.insert("TriggerModule.radiusScale".into(), 1.);
        }
        let mut physics = PhysicsWorld::empty_for_tests();
        let interior = physics.insert_collider(ColliderBuilder::cuboid(5., 5., 5.).sensor(true));
        physics.refresh_queries();
        let mut weather = state(&r);
        weather.storms[0].life = 1.;
        weather.advance(&r, 0., 0.1, [0., 0., 750.], |_| (0., DESERT));
        weather.advance_particles(&r, 0.1, [0., 0., 750.], 0., &physics, |_| false);
        assert_eq!(weather.storms[0].particles[0].particles().len(), 2);
        weather.advance_particles(&r, 0.1, [0., 0., 750.], 0., &physics, |c| c == interior);
        assert!(weather.storms[0].particles[0].particles().is_empty());
    }

    #[test]
    fn wind_pushes_targets_outside_other_interiors() {
        let r = reference();
        let mut physics = PhysicsWorld::empty_for_tests();
        let groups = InteractionGroups::all().with_memberships(1.into());
        let body = |physics: &mut PhysicsWorld, x: f32| {
            let handle = physics.insert_body(
                RigidBodyBuilder::dynamic()
                    .translation(Vector::new(x, 0., 0.))
                    .build(),
            );
            // Two colliders of one body. Each bounds size sums to 3 + 3 + 3.
            for _ in 0..2 {
                physics.insert_collider_with_parent(
                    ColliderBuilder::cuboid(1.5, 1.5, 1.5)
                        .density(0.)
                        .mass(0.5)
                        .collision_groups(groups),
                    handle,
                );
            }
            handle
        };
        let sheltered = body(&mut physics, 0.);
        let open = body(&mut physics, 50.);
        let shelter = physics.insert_collider(
            ColliderBuilder::cuboid(5., 5., 5.)
                .sensor(true)
                .collision_groups(InteractionGroups::all().with_memberships(2.into())),
        );
        physics.refresh_queries();
        let mut weather = state(&r);
        weather.reserve_bodies(2);
        let storm = &mut weather.storms[0];
        storm.wind_strength = 0.5;
        storm.current_force = [1., 0., 0.];
        let root_of = |body: RigidBodyHandle| Some(body);
        weather.scan_wind(&physics, &r, 0., root_of);
        assert_eq!(weather.storms[0].targets.len(), 2);
        assert_eq!(weather.storms[0].targets[0].extent, 18.);
        let interiors = [InteriorVolume {
            collider: shelter,
            root: None,
        }];
        weather.apply_wind(&mut physics, &r, &interiors, 0.1, root_of);
        let speed =
            |physics: &PhysicsWorld, h: RigidBodyHandle| physics.rapier().bodies[h].linvel().x;
        let dt = physics.rapier().integration_parameters.dt;
        // force * windForce * dt * clamp(extent) * distanceLerp, integrated once, over 1 kg.
        let expected = 2. * 0.1 * 18. * 0.5 * dt;
        assert!((speed(&physics, open) - expected).abs() < 1e-6);
        assert_eq!(speed(&physics, sheltered), 0.);
        // An interior of the body's own root does not shelter it.
        let own = [InteriorVolume {
            collider: shelter,
            root: Some(sheltered),
        }];
        weather.apply_wind(&mut physics, &r, &own, 0.1, root_of);
        assert!((speed(&physics, sheltered) - expected).abs() < 1e-6);
    }
}
