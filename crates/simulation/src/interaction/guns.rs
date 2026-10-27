//! Source gun shots: `weaponscript` (trigger, CanShoot, Shot, flash, recoil, crosshair), the
//! gun kind shot rules, and `mainscript` tracers, hit particles and hit clips. Reload, break
//! barrel, revolver chambers and gun part poses come in PR 3. Decals render in PR 4.
use super::items::ItemStore;
use super::spray::Ammo;
use crate::damage::Breakables;
use crate::particles::{ParticleOutput, ParticleState};
use crate::reference::{
    BreakableReference, ColliderTag, GunBreakRule, GunKind, GunReference, ItemId, ReferenceError,
    ReferenceManifest, SourceId, TorqueCurve,
};
use crate::simulation::dials::unity_euler;
use crate::vehicle::{PhysicsWorld, collider_tag, to_rapier};
use crate::world::unity_random::UnityRandom;
use rapier3d::math::{Pose, Rotation, Vector};
use rapier3d::prelude::{ColliderHandle, QueryFilter, Ray, RigidBodyHandle};
use std::collections::HashMap;

/// Collider layers with their own shot rules.
const PLAYER_LAYER: u32 = 9;
const SAND_LAYERS: [u32; 2] = [17, 19];
const GLASS_LAYER: u32 = 23;
/// `MELEE_HIT_PARTICLES` index of `mainscript.particle.spark` and `sand`.
const SPARK: usize = 0;
const SAND: usize = 4;
/// `weaponscript.hit` is a `RaycastHit[32]`.
const HIT_CAPACITY: usize = 32;
/// `mainscript.PlayClipAtPoint` with `randomPitch`.
const RANDOM_PITCH: [f32; 2] = [0.85, 1.15];
/// `mainscript.GetDecal` order.
const DECAL_METAL: u8 = 1;
const DECAL_FLESH: u8 = 2;
const DECAL_WOOD: u8 = 3;
const DECAL_PLANT: u8 = 4;
const DECAL_GLASS: u8 = 5;

/// A one-shot clip for `mainscript.PlayClipAtPoint`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShotSound {
    pub clip: u32,
    pub gain: f32,
    pub pitch: f32,
    pub position: [f64; 3],
}

/// One bullet hole: `ShootDecals[kind]` at `position`, facing `normal`, turned `roll`
/// degrees around it, parented to `collider`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DecalSpawn {
    pub kind: u8,
    pub collider: ColliderHandle,
    pub position: [f64; 3],
    pub normal: Vector,
    pub roll: f32,
    pub scale: f32,
}

/// `fpscontroller` weapon crosshair: the icon and its color.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Crosshair<'a> {
    pub icon: Option<&'a SourceId>,
    pub color: [f32; 4],
}

/// One `weaponscript` record with its muzzle pose and clip ids.
struct Gun {
    record: GunReference,
    /// `S` in the item root frame.
    muzzle: Pose,
    mass: f32,
    velocity: f32,
    one_shot: Vec<u32>,
    false_shot: Vec<u32>,
    hit_sand: Vec<u32>,
    hit_concrete: Vec<u32>,
    hit_flesh: Vec<u32>,
    hit_glass: Vec<u32>,
    /// `TRecoil` target: converted `recoilPos` and `recoilRot`.
    recoil: Option<(Vector, Rotation)>,
}

/// `weaponscript` runtime of one gun item.
#[derive(Clone, Copy, Debug)]
pub struct GunRuntime {
    gun: usize,
    pub ammo: Ammo,
    /// `bbgunscript.power`.
    pub power: u32,
    pub has_mag: bool,
    /// `FRValue`.
    pub fr: f32,
    shot_time: f32,
    actually_shot_time: f32,
    pulling: bool,
    last_pulling: bool,
    already_shot_false: bool,
    current_shot_count: u32,
}
impl GunRuntime {
    fn new(gun: usize, r: &GunReference) -> Self {
        let (power, has_mag) = match r.kind {
            GunKind::BbGun { power, .. } => (power, false),
            GunKind::Magazine { has_mag, .. } => (0, has_mag),
            _ => (0, false),
        };
        Self {
            gun,
            ammo: Ammo::new(&r.ammo),
            power,
            has_mag,
            fr: r.fr.value,
            shot_time: f32::NEG_INFINITY,
            actually_shot_time: f32::NEG_INFINITY,
            pulling: false,
            last_pulling: false,
            already_shot_false: false,
            current_shot_count: 0,
        }
    }
    /// `weaponscript.HasPower`.
    fn has_power(&self, kind: &GunKind) -> bool {
        match kind {
            GunKind::BbGun { dont_use_power, .. } => self.power > 0 || *dont_use_power,
            GunKind::Revolver { .. } | GunKind::Magazine { .. } => true,
            GunKind::None => false,
        }
    }
    /// `weaponscript.CanShoot`.
    fn can_shoot(&self, r: &GunReference) -> bool {
        self.has_power(&r.kind) && fr_allows(&r.fr.shoot, self.fr)
    }
    /// `bbgunscript`, `revolverScript` and `geppityuscript.Shot`. True fires.
    fn kind_shot(&mut self, r: &GunReference, use_ammo: bool) -> bool {
        let fires = match r.kind {
            GunKind::None => return false,
            GunKind::BbGun { dont_use_power, .. } => {
                if !self.has_power(&r.kind) {
                    return false;
                }
                if !dont_use_power {
                    self.power -= 1;
                }
                true
            }
            // PR 3 adds the chambers: a full chamber under ShootBulletPos.
            GunKind::Revolver { .. } => self.can_shoot(r),
            GunKind::Magazine { .. } => self.has_mag,
        };
        if !fires || self.ammo.count <= 0. {
            return false;
        }
        if use_ammo {
            self.ammo.decrease(1.);
        }
        true
    }
}

/// One ray hit.
#[derive(Clone, Copy)]
struct Hit {
    distance: f32,
    collider: ColliderHandle,
    normal: Vector,
}

/// The world that one shot touches.
pub struct ShotWorld<'a> {
    pub items: &'a ItemStore,
    pub physics: &'a mut PhysicsWorld,
    pub breakables: &'a mut Breakables,
    pub records: &'a [BreakableReference],
    /// `mainscript.Particles` in `MELEE_HIT_PARTICLES` order.
    pub hit_particles: &'a mut [ParticleState; 5],
    pub player: Option<RigidBodyHandle>,
    pub origin: [f64; 3],
}

/// The held gun of one frame.
#[derive(Clone, Copy, Debug)]
pub struct HeldGun {
    pub item: ItemId,
    pub gun: usize,
    /// `fpscontroller.Th` in physics space.
    pub view: Pose,
    /// `partconditionscript.state`, or `None` without a condition.
    pub condition: Option<f32>,
}

pub struct GunState {
    guns: Vec<Gun>,
    runtimes: HashMap<ItemId, GunRuntime>,
    current: Option<ItemId>,
    /// `ParticleShootStart` and `ParticleShootEnd`.
    tracers: [ParticleState; 2],
    random: UnityRandom,
    time: f32,
    hits: Vec<Hit>,
    pushed: Vec<RigidBodyHandle>,
    sounds: Vec<ShotSound>,
    decals: Vec<DecalSpawn>,
    foods: Vec<ItemId>,
}

impl GunState {
    pub fn new(r: &ReferenceManifest) -> Result<Self, ReferenceError> {
        let clips = super::melee::clip_table(r);
        let guns = r
            .guns
            .weapons
            .iter()
            .map(|w| gun(r, w, &clips))
            .collect::<Result<_, _>>()?;
        let tracer = &r.guns.tracer;
        Ok(Self {
            guns,
            runtimes: HashMap::new(),
            current: None,
            tracers: [
                ParticleState::emit_only(&tracer.start, 0x6775_6e73),
                ParticleState::emit_only(&tracer.end, 0x6775_6e74),
            ],
            random: UnityRandom::new(0x6775_6e73),
            time: 0.,
            hits: Vec::with_capacity(HIT_CAPACITY + 1),
            pushed: Vec::with_capacity(HIT_CAPACITY),
            sounds: Vec::with_capacity(16),
            decals: Vec::with_capacity(HIT_CAPACITY),
            foods: Vec::with_capacity(HIT_CAPACITY),
        })
    }
    /// The gun whose pickup rule is the held item's rule.
    pub fn weapon_for(&self, pickup: &SourceId) -> Option<usize> {
        self.guns.iter().position(|g| g.record.pickup == *pickup)
    }
    pub fn runtime(&self, item: ItemId) -> Option<&GunRuntime> {
        self.runtimes.get(&item)
    }
    pub fn runtime_mut(&mut self, item: ItemId) -> Option<&mut GunRuntime> {
        self.runtimes.get_mut(&item)
    }
    pub fn tracers(&self, index: usize) -> &[ParticleOutput] {
        self.tracers[index].particles()
    }
    pub fn take_sounds(&mut self) -> impl Iterator<Item = ShotSound> + '_ {
        self.sounds.drain(..)
    }
    /// Bullet holes of this frame. PR 4 stores and renders them.
    pub fn take_decals(&mut self) -> impl Iterator<Item = DecalSpawn> + '_ {
        self.decals.drain(..)
    }
    /// The next food item that a shot hits. The caller takes every hit after each step.
    pub fn take_food(&mut self) -> Option<ItemId> {
        self.foods.pop()
    }
    /// `fpscontroller` crosshair of the held gun.
    pub fn crosshair(&self) -> Option<Crosshair<'_>> {
        let rt = self.runtimes.get(&self.current?)?;
        let r = &self.guns[rt.gun].record;
        let ready = rt.can_shoot(r) && (r.infinite || rt.ammo.count > 0.);
        Some(Crosshair {
            icon: r.crosshair.icon.as_ref(),
            color: if ready {
                r.crosshair.color
            } else {
                r.crosshair.empty_color
            },
        })
    }
    /// `flash.intensity` of a gun item, or `None` while the flash is off.
    pub fn flash(&self, item: ItemId) -> Option<f32> {
        let rt = self.runtimes.get(&item)?;
        let f = self.guns[rt.gun].record.flash.as_ref()?;
        let start = rt.actually_shot_time;
        (self.time - start <= f.time)
            .then(|| curve(&f.curve, inverse_lerp(start, start + f.time, self.time)) * f.intensity)
    }
    /// `TRecoil` node and its local pose of a gun item.
    pub fn recoil(&self, item: ItemId) -> Option<(&SourceId, Pose)> {
        let rt = self.runtimes.get(&item)?;
        let gun = &self.guns[rt.gun];
        let (r, (translation, rotation)) = (gun.record.recoil.as_ref()?, gun.recoil?);
        let start = rt.actually_shot_time;
        let t = curve(&r.curve, inverse_lerp(start, start + r.duration, self.time));
        Some((
            &r.node,
            Pose::from_parts(
                Vector::ZERO.lerp(translation, t),
                Rotation::IDENTITY.lerp(rotation, t),
            ),
        ))
    }
    /// One frame of `weaponscript.Update`. `pull` is `PullTrigger` for the held gun.
    pub fn step(&mut self, dt: f32, pull: bool, held: Option<HeldGun>, world: &mut ShotWorld) {
        self.time += dt;
        for p in &mut self.tracers {
            p.set_origin(world.origin);
            p.advance(dt, world.origin, 0., world.physics);
        }
        let item = held.map(|h| h.item);
        if self.current != item
            && let Some(rt) = self.current.and_then(|id| self.runtimes.get_mut(&id))
        {
            rt.last_pulling = false;
            rt.already_shot_false = false;
        }
        self.current = item;
        let Some(h) = held else {
            return;
        };
        let gun = &self.guns[h.gun];
        let mut rt = *self
            .runtimes
            .entry(h.item)
            .or_insert_with(|| GunRuntime::new(h.gun, &gun.record));
        rt.pulling = pull;
        let r = &gun.record;
        if rt.pulling
            && fires(
                r.automatic,
                rt.last_pulling,
                self.time,
                rt.shot_time,
                r.min_shoot_time,
            )
        {
            rt.current_shot_count = pellets(r.shot_count);
            let root = world
                .items
                .get(h.item)
                .and_then(|i| Some((i.body?, i.local_pose)))
                .and_then(|(b, local)| {
                    Some(*world.physics.rapier().bodies.get(b)?.position() * to_rapier(local))
                });
            if let Some(root) = root {
                for k in 0..rt.current_shot_count {
                    self.shot(h, root, k, &mut rt, world);
                }
            }
        }
        if !rt.pulling {
            rt.already_shot_false = false;
        }
        rt.last_pulling = rt.pulling;
        rt.pulling = false;
        self.runtimes.insert(h.item, rt);
    }
    /// `weaponscript.Shot(k)`.
    fn shot(&mut self, h: HeldGun, root: Pose, k: u32, rt: &mut GunRuntime, world: &mut ShotWorld) {
        let gun = &self.guns[h.gun];
        let r = &gun.record;
        let condition = h.condition.filter(|_| r.condition.is_some());
        let force = power(gun.mass, gun.velocity, r.max_condition_weakness, condition);
        if !rt.can_shoot(r) && !r.fr.play_sound_even_if_cant_shoot {
            return;
        }
        rt.shot_time = self.time;
        let muzzle = root * gun.muzzle;
        let at = absolute(world.origin, muzzle.translation);
        if !(rt.kind_shot(r, k + 1 >= rt.current_shot_count) || r.infinite) {
            if !r.automatic_false_shot && rt.already_shot_false {
                return;
            }
            rt.already_shot_false = true;
            if k == 0 {
                play(
                    &mut self.random,
                    &mut self.sounds,
                    &gun.false_shot,
                    r.audio_m,
                    at,
                    true,
                );
            }
            return;
        }
        rt.actually_shot_time = self.time;
        if k == 0 {
            play(
                &mut self.random,
                &mut self.sounds,
                &gun.one_shot,
                r.audio_m,
                at,
                true,
            );
        }
        let forward = muzzle.rotation * -Vector::Z;
        let mut direction = h.view.rotation * -Vector::Z;
        if r.shot_count > 1 {
            let s = self.random.inside_unit_sphere();
            direction = direction * 10. + Vector::new(s[0], s[1], -s[2]) * r.shot_count_spread;
        }
        let direction = direction.normalize_or_zero();
        let mask = r.mask;
        self.cast(h, direction, mask, world);
        self.pushed.clear();
        let gun = &self.guns[h.gun];
        if self.hits.is_empty() {
            shoot_start(
                &mut self.tracers[0],
                world.origin,
                muzzle.translation,
                direction,
            );
        }
        for i in 0..self.hits.len() {
            let hit = self.hits[i];
            let point = h.view.translation + direction * hit.distance;
            shoot_start(
                &mut self.tracers[0],
                world.origin,
                muzzle.translation,
                point - muzzle.translation,
            );
            let rapier = world.physics.rapier();
            let collider = &rapier.colliders[hit.collider];
            let tag = collider_tag(collider.user_data);
            let layer = collider
                .collision_groups()
                .memberships
                .bits()
                .trailing_zeros();
            let parent = collider.parent();
            let item = ((collider.user_data & 0xffff_ffff) as u32)
                .checked_sub(1)
                .map(ItemId)
                .or_else(|| {
                    parent
                        .and_then(|b| world.items.iter().find(|i| i.body == Some(b)).map(|i| i.id))
                });
            if skips(tag, layer) {
                continue;
            }
            // ShootEnd: the end tracer starts 3 m before the hit.
            let back = point - direction * 3.;
            self.tracers[1].emit(1, absolute(world.origin, back), look(direction));
            let at = absolute(world.origin, point);
            let normal_rotation = look(hit.normal);
            if SAND_LAYERS.contains(&layer) {
                let count = self.random.range_int(10, 20) as usize;
                world.hit_particles[SAND].emit(count, at, normal_rotation);
                play(
                    &mut self.random,
                    &mut self.sounds,
                    &gun.hit_sand,
                    1.,
                    at,
                    false,
                );
            } else {
                let kind = decal(tag, layer);
                let clips = if layer == GLASS_LAYER {
                    &gun.hit_glass
                } else {
                    match tag {
                        Some(ColliderTag::Flesh | ColliderTag::Plant) => &gun.hit_flesh,
                        Some(ColliderTag::Wood) => &gun.hit_concrete,
                        _ => {
                            let count = self.random.range_int(10, 20) as usize;
                            world.hit_particles[SPARK].emit(count, at, normal_rotation);
                            &gun.hit_concrete
                        }
                    }
                };
                play(&mut self.random, &mut self.sounds, clips, 1., at, false);
                let scale = 0.3 * self.random.range(0.6, 1.2);
                let roll = self.random.range(0., 360.);
                self.decals.push(DecalSpawn {
                    kind,
                    collider: hit.collider,
                    position: absolute(world.origin, point + hit.normal * 0.01),
                    normal: hit.normal,
                    roll,
                    scale,
                });
            }
            if let Some(target) = world.breakables.target_of(world.physics, hit.collider) {
                world
                    .breakables
                    .try_break_shoot(world.records, target, force);
            }
            // ediblescript.Shot(onlySeparate: false). No food item is attachable.
            if let Some(id) = item
                && world.items.get(id).is_some_and(|s| s.food.is_some())
                && !self.foods.contains(&id)
            {
                self.foods.push(id);
            }
            if let Some(body) = parent
                && !self.pushed.contains(&body)
            {
                self.pushed.push(body);
                if world.physics.rapier().bodies[body].is_dynamic() {
                    let impulse = forward * force.sqrt() * 5.;
                    world
                        .physics
                        .edit_body(body, |b| b.apply_impulse_at_point(impulse, point, true));
                }
            }
            if !passes(tag, layer) {
                break;
            }
        }
    }
    /// `Physics.RaycastNonAlloc` with `LM`: the nearest `HIT_CAPACITY` hits, sorted.
    fn cast(&mut self, h: HeldGun, direction: Vector, mask: u32, world: &ShotWorld) {
        self.hits.clear();
        let gun_body = world.items.get(h.item).and_then(|i| i.body);
        let predicate = |_, c: &rapier3d::prelude::Collider| {
            c.is_enabled()
                && c.collision_groups().memberships.bits() & mask != 0
                && (c.parent().is_none() || c.parent() != gun_body)
        };
        let mut filter = QueryFilter::default();
        if let Some(player) = world.player {
            filter = filter.exclude_rigid_body(player);
        }
        filter.predicate = Some(&predicate);
        let ray = Ray::new(h.view.translation, direction);
        let hits = &mut self.hits;
        for (collider, _, hit) in world
            .physics
            .queries(filter)
            .intersect_ray(ray, f32::MAX, true)
        {
            // A ray that starts inside a collider does not hit it.
            if hit.time_of_impact <= 0. {
                continue;
            }
            let at = hits.partition_point(|h| h.distance <= hit.time_of_impact);
            if at < HIT_CAPACITY {
                hits.insert(
                    at,
                    Hit {
                        distance: hit.time_of_impact,
                        collider,
                        normal: hit.normal,
                    },
                );
                hits.truncate(HIT_CAPACITY);
            }
        }
    }
}

/// Muzzle pose, power inputs, clips and recoil of one record.
fn gun(
    r: &ReferenceManifest,
    record: &GunReference,
    clips: &[(SourceId, u32, f32)],
) -> Result<Gun, ReferenceError> {
    let prototype = r
        .world
        .prototypes
        .iter()
        .find(|p| p.source == record.prototype.0)
        .ok_or_else(|| ReferenceError("Missing gun world prototype".into()))?;
    let muzzle = prototype
        .nodes
        .iter()
        .position(|n| n.source == record.muzzle)
        .map(|i| prototype.root_relative(i).0)
        .ok_or_else(|| ReferenceError("Missing gun muzzle".into()))?;
    let ids = |sources: &[SourceId]| {
        sources
            .iter()
            .map(|s| {
                clips
                    .iter()
                    .find(|(c, _, _)| c == s)
                    .map(|(_, id, _)| *id)
                    .ok_or_else(|| ReferenceError("Missing gun clip".into()))
            })
            .collect::<Result<Vec<_>, _>>()
    };
    let c = &record.clips;
    let kind = record.ammo.r#type as usize;
    Ok(Gun {
        muzzle,
        mass: r.guns.ammo_mass[kind],
        velocity: r.guns.ammo_velocity[kind],
        one_shot: ids(&c.one_shot)?,
        false_shot: ids(&c.false_shot)?,
        hit_sand: ids(&c.hit_sand)?,
        hit_concrete: ids(&c.hit_concrete)?,
        hit_flesh: ids(&c.hit_flesh)?,
        hit_glass: ids(&c.hit_glass)?,
        recoil: record.recoil.as_ref().map(|v| {
            (
                Vector::new(v.pos[0], v.pos[1], -v.pos[2]),
                Rotation::from_array(unity_euler(v.rot)),
            )
        }),
        record: record.clone(),
    })
}

/// `weaponscript.Shot` tempForce: kinetic energy at the condition factor.
pub fn power(mass: f32, velocity: f32, weakness: f32, condition: Option<f32>) -> f32 {
    let factor = condition.map_or(1., |state| {
        let t = inverse_lerp(4., 0., state);
        weakness + (1. - weakness) * t
    });
    0.5 * mass * (velocity * factor).powi(2)
}

/// `currentShotCount`: at least one ray.
pub fn pellets(shot_count: u32) -> u32 {
    shot_count.max(1)
}

/// `weaponscript.Update`: a pulled trigger fires when automatic or newly pulled, and after
/// `minShootTime`.
pub fn fires(automatic: bool, last_pulling: bool, time: f32, shot_time: f32, min: f32) -> bool {
    (automatic || !last_pulling) && time - shot_time >= min
}

/// `CanShoot` FR checks.
pub fn fr_allows(rule: &GunBreakRule, value: f32) -> bool {
    (rule.equal && value == rule.value)
        || (rule.bigger && value > rule.value)
        || (rule.smaller && value < rule.value)
        || (rule.near && (value - rule.value).abs() < rule.near_value)
}

/// The shot ignores this hit: tag `Ignore` or the player layer.
pub fn skips(tag: Option<ColliderTag>, layer: u32) -> bool {
    tag == Some(ColliderTag::Ignore) || layer == PLAYER_LAYER
}

/// The shot continues through `Plant`, glass and `Cobweb`.
pub fn passes(tag: Option<ColliderTag>, layer: u32) -> bool {
    matches!(tag, Some(ColliderTag::Plant | ColliderTag::Cobweb)) || layer == GLASS_LAYER
}

/// `mainscript.GetDecal`.
fn decal(tag: Option<ColliderTag>, layer: u32) -> u8 {
    match tag {
        Some(ColliderTag::Metal) => DECAL_METAL,
        Some(ColliderTag::Flesh) => DECAL_FLESH,
        Some(ColliderTag::Wood) => DECAL_WOOD,
        Some(ColliderTag::Plant) => DECAL_PLANT,
        _ if layer == GLASS_LAYER => DECAL_GLASS,
        _ => 0,
    }
}

/// `mainscript.ShootStart`.
fn shoot_start(p: &mut ParticleState, origin: [f64; 3], start: Vector, direction: Vector) {
    p.emit(1, absolute(origin, start), look(direction));
}

/// `Quaternion.LookRotation(dir, up)`, keeping a zero direction's identity.
fn look(direction: Vector) -> Rotation {
    if direction == Vector::ZERO {
        Rotation::IDENTITY
    } else {
        super::melee::look_rotation(direction)
    }
}

/// `mainscript.PlayClipAtPoint` of a random clip.
fn play(
    random: &mut UnityRandom,
    sounds: &mut Vec<ShotSound>,
    clips: &[u32],
    gain: f32,
    position: [f64; 3],
    random_pitch: bool,
) {
    if clips.is_empty() {
        return;
    }
    let clip = clips[random.range_int(0, clips.len() as i32) as usize];
    let pitch = if random_pitch {
        random.range(RANDOM_PITCH[0], RANDOM_PITCH[1])
    } else {
        1.
    };
    sounds.push(ShotSound {
        clip,
        gain,
        pitch,
        position,
    });
}

fn curve(c: &TorqueCurve, t: f32) -> f32 {
    if c.keys.is_empty() {
        0.
    } else {
        crate::simulation::engine::torque(c, t)
    }
}

/// Unity `Mathf.InverseLerp`.
fn inverse_lerp(a: f32, b: f32, value: f32) -> f32 {
    if a == b {
        0.
    } else {
        ((value - a) / (b - a)).clamp(0., 1.)
    }
}

fn absolute(origin: [f64; 3], local: Vector) -> [f64; 3] {
    std::array::from_fn(|i| origin[i] + local[i] as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn power_follows_mass_velocity_and_condition() {
        assert_eq!(power(0.01, 300., 0.5, None), 450.);
        // State 0 is new; state 4 is the full weakness.
        assert_eq!(power(0.01, 300., 0.5, Some(0.)), 450.);
        assert!((power(0.01, 300., 0.5, Some(4.)) - 112.5).abs() < 1e-3);
        assert!((power(0.01, 300., 0.5, Some(2.)) - 253.125).abs() < 1e-3);
    }

    #[test]
    fn a_shot_has_at_least_one_pellet() {
        assert_eq!(pellets(0), 1);
        assert_eq!(pellets(1), 1);
        assert_eq!(pellets(8), 8);
    }

    #[test]
    fn shots_skip_ignore_and_pass_plants_glass_and_cobwebs() {
        assert!(skips(Some(ColliderTag::Ignore), 0));
        assert!(skips(None, PLAYER_LAYER));
        assert!(!skips(Some(ColliderTag::Plant), 0));
        assert!(passes(Some(ColliderTag::Plant), 0));
        assert!(passes(Some(ColliderTag::Cobweb), 0));
        assert!(passes(None, GLASS_LAYER));
        assert!(!passes(Some(ColliderTag::Wood), 0));
        assert!(!passes(None, 0));
        assert_eq!(decal(Some(ColliderTag::Metal), GLASS_LAYER), DECAL_METAL);
        assert_eq!(decal(None, GLASS_LAYER), DECAL_GLASS);
        assert_eq!(decal(Some(ColliderTag::Sand), 0), 0);
    }

    #[test]
    fn automatic_guns_fire_while_held_and_others_once_per_press() {
        assert!(fires(true, true, 1., 0.9, 0.1));
        assert!(!fires(true, true, 1., 0.95, 0.1));
        assert!(!fires(false, true, 1., 0., 0.1));
        assert!(fires(false, false, 1., 0., 0.1));
        assert!(fires(false, false, 0., f32::NEG_INFINITY, 0.1));
    }

    #[test]
    fn fr_rules_compare_like_can_shoot() {
        let rule = |equal, bigger, smaller, near| GunBreakRule {
            value: 0.,
            equal,
            bigger,
            smaller,
            near,
            near_value: 0.1,
        };
        assert!(fr_allows(&rule(true, false, false, false), 0.));
        assert!(!fr_allows(&rule(true, false, false, false), 0.5));
        assert!(fr_allows(&rule(false, true, false, false), 0.5));
        assert!(fr_allows(&rule(false, false, false, true), 0.05));
        assert!(!fr_allows(&rule(false, false, false, true), 0.2));
        assert!(!fr_allows(&rule(false, false, false, false), 0.));
    }
}
