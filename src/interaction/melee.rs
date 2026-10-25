//! Source shovel swing: `weaponhandlingscript` (input, animator, swing sound) and
//! `meleeweaponscript` (hit box, force, hit particles, hit sound, digging).
//! Damage, breaking and food hurt are not ported.
use super::buried::BuriedItems;
use super::items::ItemStore;
use crate::animation::controller::{MotionController, StateEvent};
use crate::particles::ParticleState;
use crate::reference::{
    ColliderTag, ItemId, ReferenceError, ReferenceManifest, ShovelAcquisitionReference, SourceId,
};
use crate::simulation::FIXED_DT;
use crate::vehicle::{PhysicsWorld, VehiclePose, collider_tag, to_rapier};
use crate::world::unity_random::UnityRandom;
use rapier3d::math::{Pose, Rotation, Vector};
use rapier3d::prelude::{ColliderHandle, Cuboid, QueryFilter, RigidBodyHandle};

/// `mainscript.soundHitType`.
const METAL: usize = 0;
const BLOOD: usize = 1;
const PLANT: usize = 2;
const WOOD: usize = 3;
const SAND: usize = 4;
const GLASS: usize = 5;
/// Collider layers with their own hit type.
const PLAYER_LAYER: u32 = 9;
const SAND_LAYER: u32 = 17;
const GLASS_LAYER: u32 = 23;
/// Particle index in `MELEE_HIT_PARTICLES` order for each hit type. Glass has none.
const HIT_PARTICLE: [Option<usize>; 6] = [Some(0), Some(1), Some(2), Some(3), Some(4), None];
/// `weaponhandlingscript`: the swing sound waits `0.1 / speed` seconds.
const SWING_SOUND_DELAY: f32 = 0.1;
/// `meleeweaponscript`: the hit sound waits 0.125 s after the swing starts.
const HIT_SOUND_DELAY: f32 = 0.125;
const OVERLAP_CAPACITY: usize = 32;

/// `meleeweaponscript.attackType` triggers: front, left, right, up, stab.
const TRIGGERS: [&str; 5] = [
    "AttackForward",
    "AttackLeft",
    "AttackRight",
    "AttackBack",
    "AttackStab",
];

#[derive(Clone, Copy, Debug, Default)]
pub struct MeleeInput {
    pub pressed: bool,
    pub released: bool,
    pub held: bool,
    pub back: bool,
    pub left: bool,
    pub right: bool,
}
impl MeleeInput {
    /// `weaponhandlingscript`: S with A or D is a stab, then D, A, S, else front.
    fn attack(self) -> usize {
        if self.back && (self.right || self.left) {
            4
        } else if self.right {
            2
        } else if self.left {
            1
        } else if self.back {
            3
        } else {
            0
        }
    }
}

/// A one-shot sound for `mainscript.PlayClipAtPoint`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeleeSound {
    pub clip: u32,
    pub gain: f32,
    pub position: [f64; 3],
}

pub struct MeleeState {
    shovel: ShovelAcquisitionReference,
    controller: MotionController,
    triggers: [usize; 5],
    hit_trigger: usize,
    /// `T` and `Trigger` in the item root frame, and the `OverlapBox` half extents.
    tip: Pose,
    trigger: Pose,
    half_extents: Vector,
    swing_clips: Vec<u32>,
    /// `mainscript.meleeHitSounds` per hit type, with clip lengths.
    hit_clips: [Vec<(u32, f32)>; 6],
    hit_volumes: [f32; 6],
    particles: [ParticleState; 5],
    random: UnityRandom,
    active: bool,
    time: f32,
    speed: f32,
    attacking: i32,
    attacking_now: bool,
    started_attack: bool,
    swing_sounded: bool,
    attack_started: f32,
    last_tip: Option<Vector>,
    sounded: bool,
    has_to_sound: bool,
    next_sound: f32,
    attacked: f32,
    hit_type: usize,
    hit_point: Vector,
    overlaps: Vec<ColliderHandle>,
    sounds: Vec<MeleeSound>,
}

impl MeleeState {
    pub fn new(r: &ReferenceManifest) -> Result<Self, ReferenceError> {
        let shovel = r.acquisition.shovel.clone();
        let motion = &shovel.motion;
        let parameter = |name| {
            MotionController::parameter(motion, name)
                .ok_or_else(|| ReferenceError(format!("Missing shovel parameter {name}")))
        };
        let mut triggers = [0; 5];
        for (slot, name) in triggers.iter_mut().zip(TRIGGERS) {
            *slot = parameter(name)?;
        }
        let hit_trigger = parameter("Hit")?;
        let asset = r
            .item_prototypes
            .iter()
            .find(|p| p.id == shovel.prototype)
            .ok_or_else(|| ReferenceError("Missing shovel prototype".into()))?
            .asset;
        let prototype = r
            .world
            .prototypes
            .iter()
            .find(|p| p.asset == asset)
            .ok_or_else(|| ReferenceError("Missing shovel world prototype".into()))?;
        let node = |source: &SourceId| {
            prototype
                .nodes
                .iter()
                .position(|n| n.source == *source)
                .map(|i| root_relative(&prototype.nodes, i))
                .ok_or_else(|| ReferenceError("Missing shovel node".into()))
        };
        let (tip, _) = node(&shovel.target)?;
        let (trigger, scale) = node(&shovel.trigger)?;
        let clips = clip_table(r);
        let clip = |source: &SourceId| {
            clips
                .iter()
                .find(|(s, _, _)| s == source)
                .map(|(_, id, length)| (*id, *length))
                .ok_or_else(|| ReferenceError("Missing shovel clip".into()))
        };
        if shovel.contact_clips.len() != 6 {
            return Err(ReferenceError("Shovel hit sound groups differ".into()));
        }
        let mut hit_clips: [Vec<(u32, f32)>; 6] = Default::default();
        for (group, sources) in hit_clips.iter_mut().zip(&shovel.contact_clips) {
            *group = sources.iter().map(clip).collect::<Result<_, _>>()?;
        }
        let swing_clips = shovel
            .attack_clips
            .iter()
            .map(|s| clip(s).map(|(id, _)| id))
            .collect::<Result<_, _>>()?;
        Ok(Self {
            controller: MotionController::new(motion),
            triggers,
            hit_trigger,
            tip,
            trigger,
            half_extents: scale.abs(),
            swing_clips,
            hit_clips,
            hit_volumes: r.melee.hit_volumes,
            particles: std::array::from_fn(|i| {
                ParticleState::emit_only(&r.melee.particles[i], 0x6d65_6c65 + i as u64)
            }),
            random: UnityRandom::new(0x6d65_6c65),
            active: false,
            time: 0.,
            speed: 1.,
            attacking: 0,
            attacking_now: false,
            started_attack: false,
            swing_sounded: false,
            attack_started: 0.,
            last_tip: None,
            sounded: false,
            has_to_sound: false,
            next_sound: 0.,
            attacked: 0.,
            hit_type: METAL,
            hit_point: Vector::ZERO,
            overlaps: Vec::with_capacity(OVERLAP_CAPACITY),
            sounds: Vec::with_capacity(4),
            shovel,
        })
    }
    /// The held item is a shovel when its pickup rule is the shovel prototype rule.
    pub fn is_shovel(&self, pickup: &SourceId) -> bool {
        *pickup == self.shovel.pickup
    }
    pub fn particles(&self, index: usize) -> &[crate::particles::ParticleOutput] {
        self.particles[index].particles()
    }
    pub fn take_sounds(&mut self) -> impl Iterator<Item = MeleeSound> + '_ {
        self.sounds.drain(..)
    }
    pub fn rebase(&mut self, shift: [f32; 3]) {
        if let Some(tip) = &mut self.last_tip {
            *tip += Vector::from_array(shift);
        }
        self.hit_point += Vector::from_array(shift);
    }
    /// Animated RH pose in its parent frame, or `None` for `defRH`.
    pub fn hand(&self) -> Option<VehiclePose> {
        self.active
            .then(|| self.controller.pose(&self.shovel.motion))
    }
    /// One frame. `shovel` is the item in the right hand when it is a shovel.
    #[allow(clippy::too_many_arguments)]
    pub fn step(
        &mut self,
        input: MeleeInput,
        dt: f32,
        shovel: Option<ItemId>,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
        buried: &mut BuriedItems,
        player: Option<RigidBodyHandle>,
        origin: [f64; 3],
    ) {
        self.time += dt;
        for p in &mut self.particles {
            p.set_origin(origin);
            p.advance(dt, origin, 0., physics);
        }
        let Some(root) = shovel.and_then(|id| {
            let item = items.get(id)?;
            let body = physics.rapier().bodies.get(item.body?)?;
            Some(*body.position() * to_rapier(item.local_pose))
        }) else {
            if self.active {
                self.leave();
            }
            return;
        };
        self.active = true;
        let melee_speed = self.shovel.speed;
        self.controller.set_speed(self.speed);
        if input.pressed {
            self.speed = melee_speed;
            self.controller.reset_trigger(self.hit_trigger);
            self.controller.set_trigger(self.triggers[input.attack()]);
            self.started_attack = true;
        }
        if input.released {
            self.speed = melee_speed;
            self.attack_started = self.time;
        }
        if self.attacking_now
            && !self.swing_sounded
            && self.time - self.attack_started > SWING_SOUND_DELAY / melee_speed
            && !self.swing_clips.is_empty()
        {
            self.swing_sounded = true;
            let index = self.random.range_int(0, self.swing_clips.len() as i32) as usize;
            self.sounds.push(MeleeSound {
                clip: self.swing_clips[index],
                gain: self.shovel.attack_volume,
                position: absolute(origin, root.translation),
            });
        }
        self.attacking_now = self.attacking > 0 && self.speed > 0.;
        let tip = (root * self.tip).translation;
        if self.attacking_now {
            self.hit(
                tip,
                root * self.trigger,
                dt,
                items,
                physics,
                buried,
                player,
                origin,
            );
        } else {
            self.sounded = false;
            self.attacked = self.time;
        }
        self.last_tip = Some(tip);
        if self.has_to_sound
            && !self.sounded
            && self.time > self.next_sound
            && self.time - self.attacked > HIT_SOUND_DELAY
        {
            self.sounded = true;
            self.has_to_sound = false;
            let group = &self.hit_clips[self.hit_type];
            if !group.is_empty() {
                let (clip, length) = group[self.random.range_int(0, group.len() as i32) as usize];
                self.next_sound = self.time + length;
                self.sounds.push(MeleeSound {
                    clip,
                    gain: self.hit_volumes[self.hit_type],
                    position: absolute(origin, self.hit_point),
                });
            }
        }
        let held = input.held;
        let (attacking, speed, swing_sounded, started_attack) = (
            &mut self.attacking,
            &mut self.speed,
            &mut self.swing_sounded,
            &mut self.started_attack,
        );
        // animationEventScript: OnStateEnter is AttackStart, OnStateExit is AttackEnd.
        self.controller
            .advance(&self.shovel.motion, dt, |event| match event {
                StateEvent::Enter(_) => {
                    *attacking += 1;
                    if held {
                        *speed = 0.;
                    }
                    *swing_sounded = false;
                }
                StateEvent::Exit(_) => {
                    *attacking -= 1;
                    *started_attack = false;
                }
            });
    }
    /// The shovel left the hand: the animator is off and RH returns to `defRH`.
    fn leave(&mut self) {
        self.active = false;
        self.speed = 1.;
        self.swing_sounded = false;
        self.attacking_now = false;
        self.started_attack = false;
        self.attacking = 0;
        self.last_tip = None;
        self.controller.reset(&self.shovel.motion);
    }
    #[allow(clippy::too_many_arguments)]
    fn hit(
        &mut self,
        tip: Vector,
        trigger: Pose,
        dt: f32,
        items: &mut ItemStore,
        physics: &mut PhysicsWorld,
        buried: &mut BuriedItems,
        player: Option<RigidBodyHandle>,
        origin: [f64; 3],
    ) {
        let mask = self.shovel.contact_mask;
        let mut filter = QueryFilter::default();
        if let Some(player) = player {
            filter = filter.exclude_rigid_body(player);
        }
        let predicate = |_, c: &rapier3d::prelude::Collider| {
            let layers = c.collision_groups().memberships.bits();
            c.is_enabled() && layers & mask != 0 && layers != 1 << PLAYER_LAYER
        };
        filter.predicate = Some(&predicate);
        self.overlaps.clear();
        let shape = Cuboid::new(self.half_extents);
        self.overlaps.extend(
            physics
                .queries(filter)
                .intersect_shape(trigger, &shape)
                .map(|(h, _)| h)
                .take(OVERLAP_CAPACITY),
        );
        let delta = tip - self.last_tip.unwrap_or(tip);
        let force = (delta.normalize_or_zero() * self.shovel.contact.force + delta) * 0.5 * dt;
        let contact = &self.shovel.contact;
        for i in 0..self.overlaps.len() {
            let rapier = physics.rapier();
            let collider = &rapier.colliders[self.overlaps[i]];
            let aabb = collider.compute_aabb();
            let point = tip.clamp(aabb.mins, aabb.maxs);
            let layer = collider
                .collision_groups()
                .memberships
                .bits()
                .trailing_zeros();
            let tag = collider_tag(collider.user_data);
            let parent = collider.parent();
            let owned = (collider.user_data & 0xffff_ffff) as u32;
            let body_item =
                parent.and_then(|b| items.iter().find(|i| i.body == Some(b)).map(|i| i.id));
            let mut item = owned.checked_sub(1).map(ItemId).or(body_item);
            if let Some(body) = parent
                && item.is_none_or(|id| !buried.recently_unburied(id))
            {
                if body_item.is_some() && body_item != item {
                    item = body_item;
                } else if rapier.bodies[body].is_dynamic() {
                    // Update AddForce acts over the next fixed step.
                    physics.edit_body(body, |b| b.apply_impulse(force * FIXED_DT, true));
                }
            }
            let strength = force.length() * contact.all_particles;
            let low = strength;
            let high = strength * self.random.range(0.9, 1.1);
            let sparks = self.random.range(low, high) as i32;
            let (hit_type, factor) = if layer == SAND_LAYER {
                (SAND, contact.sand)
            } else if layer == GLASS_LAYER {
                (GLASS, 0.)
            } else {
                match tag {
                    Some(ColliderTag::Flesh) => (BLOOD, contact.blood),
                    Some(ColliderTag::Plant) => (PLANT, contact.plant),
                    Some(ColliderTag::Wood) => (WOOD, contact.wood),
                    _ => (METAL, contact.spark),
                }
            };
            self.hit_type = hit_type;
            if let Some(system) = HIT_PARTICLE[hit_type] {
                let direction = tip - point;
                let rotation = if direction == Vector::ZERO {
                    self.particles[system].rotation
                } else {
                    look_rotation(direction)
                };
                self.particles[system].emit(
                    (sparks as f32 * factor).max(0.) as usize,
                    absolute(origin, point),
                    rotation,
                );
            }
            if let Some(id) = item
                && self.shovel.shovel_speed != 0.
                && (items.get(id).is_some_and(|i| i.buried) || buried.recently_unburied(id))
            {
                buried.lift(id, dt, self.shovel.shovel_speed, items, physics, player);
            }
            if !self.sounded {
                self.has_to_sound = true;
                self.hit_point = point;
            }
        }
    }
}

/// Pose of a prototype node in its root frame, and the node local scale.
fn root_relative(nodes: &[crate::reference::WorldPrototypeNode], index: usize) -> (Pose, Vector) {
    let scale = Vector::from_array(nodes[index].transform.scale);
    let mut pose = Pose::IDENTITY;
    let mut node = index;
    while let Some(parent) = nodes[node].parent {
        let t = &nodes[node].transform;
        let rotation =
            Rotation::from_xyzw(t.rotation[0], t.rotation[1], t.rotation[2], t.rotation[3]);
        pose = Pose::from_parts(
            Vector::from_array(t.translation)
                + rotation * (Vector::from_array(t.scale) * pose.translation),
            rotation * pose.rotation,
        );
        node = parent as usize;
    }
    (pose, scale)
}

/// Source `Quaternion.LookRotation(dir, up)` in converted space: source +Z is -Z.
fn look_rotation(direction: Vector) -> Rotation {
    let back = -direction.normalize();
    let right = Vector::Y.cross(back).try_normalize().unwrap_or(Vector::X);
    let up = back.cross(right);
    Rotation::from_mat3(&rapier3d::glamx::Mat3::from_cols(right, up, back))
}

fn absolute(origin: [f64; 3], local: Vector) -> [f64; 3] {
    std::array::from_fn(|i| origin[i] + local[i] as f64)
}

/// Exported audio clips: source, id and source length.
fn clip_table(r: &ReferenceManifest) -> Vec<(SourceId, u32, f32)> {
    r.audio["clips"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|c| {
            Some((
                serde_json::from_value(c["source"].clone()).ok()?,
                c["id"].as_u64()? as u32,
                c["source_duration_seconds"].as_f64()? as f32,
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attack_type_follows_source_key_order() {
        let input = |back, left, right| MeleeInput {
            back,
            left,
            right,
            ..Default::default()
        };
        assert_eq!(input(false, false, false).attack(), 0);
        assert_eq!(input(false, true, false).attack(), 1);
        assert_eq!(input(false, false, true).attack(), 2);
        assert_eq!(input(false, true, true).attack(), 2);
        assert_eq!(input(true, false, false).attack(), 3);
        assert_eq!(input(true, true, false).attack(), 4);
        assert_eq!(input(true, false, true).attack(), 4);
    }

    #[test]
    fn look_rotation_points_converted_forward_at_the_direction() {
        let direction = Vector::new(1., 2., -3.).normalize();
        let forward = look_rotation(direction) * -Vector::Z;
        assert!((forward - direction).length() < 1e-5);
    }
}
