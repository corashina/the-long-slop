//! Source creature AI: `aispawn`, `aiscript`, `movecreaturescript`, `sensoryscript`,
//! `stimulscript` and `nyulscript`. Live rabbits run before the physics step.
pub mod brain;
pub mod motion;
pub mod rabbit;
pub mod rig;
pub mod senses;
pub mod spawner;
pub mod stimuli;

use crate::reference::{RabbitReference, ReferenceError, ReferenceManifest};
use crate::save::{AbsolutePose, RabbitSnapshot};
use crate::vehicle::PhysicsWorld;
use crate::weather::StormRandom;
use crate::world::WorldOrigin;
use brain::BrainInput;
use rabbit::{Rabbit, Size};
use rapier3d::math::{Pose, Rotation, Vector};
use rapier3d::prelude::{Ray, RigidBodyHandle};
use rig::{Rig, RigPose};
use senses::{ScanInput, cast};
use spawner::{Player, Spawner};
use stimuli::{PlayerStimuli, Stimuli};

/// Seconds between the world collision checks of each rabbit.
const SUPPORT_INTERVAL: f64 = 1.;

/// What the rabbits read of the world this step.
pub struct StepInput<'a> {
    pub now: f64,
    pub dt: f32,
    /// The `aispawn` transform.
    pub player: Player,
    /// The player head stimulus position and the player body.
    pub head: (Vector, RigidBodyHandle),
    pub origin: &'a WorldOrigin,
    /// True when the world has collision around a local point.
    pub supports: &'a dyn Fn(Vector) -> bool,
}

/// One rabbit for the renderer.
#[derive(Clone, Copy, Debug)]
pub struct RabbitView {
    pub id: u32,
    pub pose: Pose,
    pub scale: f32,
    pub phase: f32,
    pub value: f32,
    pub visible: bool,
}

pub struct Creatures {
    rig: Rig,
    /// Per live collider: it sits under a ragdoll bone.
    bones: Vec<bool>,
    eye: usize,
    sources: PlayerStimuli,
    stimuli: Stimuli,
    spawner: Spawner,
    rabbits: Vec<Rabbit>,
    next_id: u32,
    pose: RigPose,
    random: StormRandom,
}

impl Creatures {
    pub fn new(reference: &ReferenceManifest, seed: u64, now: f64) -> Result<Self, ReferenceError> {
        let c = &reference.creatures;
        let r = &c.rabbit;
        let rig = Rig::new(&reference.nodes, r.root, &r.animator)?;
        let bone_nodes: Vec<_> = r.ragdoll.bodies.iter().map(|b| b.node).collect();
        let bones = r
            .body
            .colliders
            .iter()
            .map(|collider| {
                let mut node = Some(collider.node);
                while let Some(id) = node {
                    if bone_nodes.contains(&id) {
                        return true;
                    }
                    node = reference
                        .nodes
                        .iter()
                        .find(|n| n.id == id)
                        .and_then(|n| n.parent);
                }
                false
            })
            .collect();
        let eye = rig
            .index(r.senses.eyes)
            .ok_or_else(|| ReferenceError("Rabbit eye outside the rig".into()))?;
        let sources = PlayerStimuli::new(c)
            .ok_or_else(|| ReferenceError("Missing player stimulus sources".into()))?;
        Ok(Self {
            pose: rig.pose(),
            rig,
            bones,
            eye,
            sources,
            stimuli: Stimuli::new(),
            spawner: Spawner::new(now),
            rabbits: Vec::with_capacity(r.spawner.max_count as usize),
            next_id: 1,
            random: StormRandom(seed ^ 0x7261_6262_6974_0001),
        })
    }

    pub fn rabbits(&self) -> impl Iterator<Item = &Rabbit> {
        self.rabbits.iter()
    }

    /// `PlayClipAtPoint`: a sound stimulus at `position` for `seconds`.
    pub fn sound(&mut self, position: [f64; 3], volume: f32, now: f64, seconds: f32) {
        self.stimuli.sound(position, volume, now, seconds);
    }

    pub fn rebase(&mut self, shift: Vector) {
        for rabbit in &mut self.rabbits {
            rabbit.brain.rebase(shift);
        }
    }

    /// One fixed step: the spawner, then each live rabbit sets its body velocity and rotation.
    pub fn step(&mut self, r: &RabbitReference, physics: &mut PhysicsWorld, input: &StepInput) {
        let now = input.now;
        let origin = *input.origin;
        self.stimuli.rebuild(&self.sources, input.head, now, |p| {
            Vector::from_array(origin.local(p))
        });
        let count = self.rabbits.len();
        let mut spawned = Vec::new();
        self.spawner.step(
            &r.spawner,
            now,
            count,
            &input.player,
            physics,
            &mut self.random,
            |place, random| {
                let yaw = random.range(0., 360.);
                spawned.push((place, Size::new(r, random.unit()), yaw));
            },
        );
        for (place, size, yaw) in spawned {
            let pose = Pose::from_parts(place, Rotation::from_rotation_y(-yaw.to_radians()));
            self.spawn(r, physics, pose, size, size.shoot_health, now)
                .expect("validated rabbit reference");
        }
        for rabbit in &mut self.rabbits {
            if !rabbit.alive {
                continue;
            }
            if now >= rabbit.next_support_check {
                rabbit.next_support_check = now + SUPPORT_INTERVAL;
                let position = physics.rapier().bodies[rabbit.body].translation();
                let active = (input.supports)(position);
                if active != rabbit.active {
                    rabbit.active = active;
                    physics.edit_body(rabbit.body, |b| b.set_enabled(active));
                }
            }
            if !rabbit.active {
                continue;
            }
            step_rabbit(
                r,
                &self.rig,
                &mut self.pose,
                self.eye,
                rabbit,
                self.stimuli.list(),
                physics,
                input.dt,
                now,
                &mut self.random,
            );
        }
    }

    fn spawn(
        &mut self,
        r: &RabbitReference,
        physics: &mut PhysicsWorld,
        pose: Pose,
        size: Size,
        shoot_health: f32,
        now: f64,
    ) -> Result<&mut Rabbit, ReferenceError> {
        let (body, colliders) =
            rabbit::insert_body(r, &self.rig, &self.bones, &size, pose, physics)?;
        let mut rabbit = Rabbit::new(
            self.next_id,
            r,
            body,
            colliders,
            size,
            pose.translation,
            now,
            &mut self.random,
        );
        rabbit.shoot_health = shoot_health;
        self.next_id += 1;
        self.rabbits.push(rabbit);
        Ok(self.rabbits.last_mut().unwrap())
    }

    /// Rabbits for the renderer.
    pub fn views<'a>(&'a self, physics: &'a PhysicsWorld) -> impl Iterator<Item = RabbitView> + 'a {
        self.rabbits.iter().map(|rabbit| RabbitView {
            id: rabbit.id,
            pose: *physics.rapier().bodies[rabbit.body].position(),
            scale: rabbit.size.scale,
            phase: rabbit.phase,
            value: rabbit.blend_value(),
            visible: rabbit.active,
        })
    }

    /// Source `save_nyulscript` and the prefab pose of each rabbit.
    pub fn snapshot(&self, physics: &PhysicsWorld, origin: &WorldOrigin) -> Vec<RabbitSnapshot> {
        self.rabbits
            .iter()
            .map(|rabbit| {
                let pose = physics.rapier().bodies[rabbit.body].position();
                RabbitSnapshot {
                    pose: AbsolutePose {
                        position: origin.absolute(pose.translation.to_array()),
                        rotation: pose.rotation.to_array(),
                    },
                    size: rabbit.size.size,
                    shoot_health: rabbit.shoot_health,
                    alive: rabbit.alive,
                    eyes: rabbit.senses.has_eyes,
                    ears: rabbit.senses.has_ears,
                }
            })
            .collect()
    }

    /// Spawns the saved rabbits. `SetBasedOnSize` runs with the saved size, then `SetHealth`.
    pub fn restore(
        &mut self,
        r: &RabbitReference,
        physics: &mut PhysicsWorld,
        origin: &WorldOrigin,
        saved: &[RabbitSnapshot],
        now: f64,
    ) -> Result<(), ReferenceError> {
        for s in saved {
            let pose = Pose::from_parts(
                Vector::from_array(origin.local(s.pose.position)),
                Rotation::from_array(s.pose.rotation),
            );
            let rabbit = self.spawn(r, physics, pose, Size::new(r, s.size), s.shoot_health, now)?;
            rabbit.alive = s.alive;
            rabbit.senses.has_eyes = s.eyes;
            rabbit.senses.has_ears = s.ears;
        }
        Ok(())
    }
}

/// `aiscript.Update`, `movecreaturescript.Update` and `nyulscript.SetAnims` of one rabbit.
#[allow(clippy::too_many_arguments)]
fn step_rabbit(
    r: &RabbitReference,
    rig: &Rig,
    pose: &mut RigPose,
    eye: usize,
    rabbit: &mut Rabbit,
    stimuli: &[stimuli::Stimulus],
    physics: &mut PhysicsWorld,
    dt: f32,
    now: f64,
    random: &mut StormRandom,
) {
    let body = &physics.rapier().bodies[rabbit.body];
    let root = *body.position();
    let velocity = body.linvel();
    rig.sample(
        &r.animator,
        rabbit.phase,
        rabbit.blend_value(),
        rabbit.size.scale,
        pose,
    );
    if now >= rabbit.senses.next_scan {
        let e = pose.model[eye];
        let input = ScanInput {
            physics,
            own: rabbit.body,
            root: root.translation,
            eye: root * Pose::from_parts(e.position, e.rotation),
            stimuli,
        };
        rabbit.senses.scan(&r.senses, &input);
        rabbit.senses.next_scan = now + f64::from(r.senses.scan_interval);
    }
    let forward = root.rotation * -Vector::Z;
    let ray = |direction: Vector, length: f32, mask: u32| {
        cast(
            physics,
            rabbit.body,
            None,
            Ray::new(root.translation, direction),
            length,
            mask,
            true,
        )
        .is_some()
    };
    let jump_ray = || ray(forward, r.brain.jump_sense_distance, r.brain.jump_mask);
    let steer = rabbit.brain.update(
        &r.brain,
        &BrainInput {
            now,
            position: root.translation,
            forward,
            right: root.rotation * Vector::X,
            sensed: &rabbit.senses.sensed,
            ray_sum: rabbit.senses.ray_sum(),
            speed: velocity.length(),
            has_eyes: rabbit.senses.has_eyes,
            jump_ray: &jump_ray,
        },
        random,
    );
    let m = &r.motion;
    let mut velocity = velocity;
    if steer.jump && ray(-Vector::Y, m.grounded_ray_distance, m.grounded_mask) {
        velocity.y += m.jump_force;
    }
    let velocity = motion::velocity(
        velocity,
        steer.move_dir.unwrap_or(Vector::ZERO),
        rabbit.size.speed,
        m.acceleration,
        dt,
    );
    let rotation = motion::turn(
        root.rotation,
        steer.rot.unwrap_or(0.),
        rabbit.size.rot_speed,
        dt,
    );
    rabbit.run_value +=
        (steer.move_speed - rabbit.run_value) * (dt * r.size.run_lerp).clamp(0., 1.);
    let duration = rig.duration(&r.animator, rabbit.blend_value(), pose);
    if duration > 0. {
        rabbit.phase = (rabbit.phase + dt / duration).rem_euclid(1.);
    }
    physics.edit_body(rabbit.body, |b| {
        b.set_linvel(velocity, true);
        b.set_rotation(rotation, true);
    });
    for c in rabbit.colliders.iter().filter(|c| c.bone) {
        let f = pose.model[c.node];
        let local = Pose::from_parts(f.position, f.rotation) * c.offset;
        physics.edit_collider(c.handle, |collider| collider.set_position_wrt_parent(local));
    }
}
