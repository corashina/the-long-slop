//! Source `nyulscript`: size values and `SetAnims`. The live body is `ragdollactivatescript.RB`.
use super::brain::Brain;
use super::rig::Rig;
use super::senses::Senses;
use crate::reference::{RabbitReference, ReferenceError};
use crate::vehicle::{PhysicsWorld, collider_builder, step_surface_bits, tag_bits};
use crate::weather::StormRandom;
use rapier3d::math::{Pose, Vector};
use rapier3d::prelude::{
    ColliderHandle, Group, InteractionGroups, InteractionTestMode, RigidBodyBuilder,
    RigidBodyHandle,
};

/// Source `SetBasedOnSize` values. Each is `lerp(min, max, size)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Size {
    pub size: f32,
    pub scale: f32,
    /// The rabbit mass. The live body gets this over the bone count (`SetMass`).
    pub mass: f32,
    pub health: f32,
    pub shoot_health: f32,
    pub speed: f32,
    pub rot_speed: f32,
}

fn lerp(r: [f32; 2], t: f32) -> f32 {
    r[0] + (r[1] - r[0]) * t
}

impl Size {
    pub fn new(r: &RabbitReference, size: f32) -> Self {
        let z = &r.size;
        Self {
            size,
            scale: lerp(z.scale, size),
            mass: lerp(z.mass, size),
            health: lerp(z.health, size),
            shoot_health: lerp(z.shoot_health, size),
            speed: lerp(z.speed, size),
            rot_speed: lerp(z.rot_speed, size),
        }
    }
    /// `SetMass`: the loop writes `RB.mass` for each bone, so the live body keeps one share.
    pub fn live_mass(&self, r: &RabbitReference) -> f32 {
        self.mass / r.ragdoll.bodies.len() as f32
    }
}

/// One live anim collider and its rig node.
pub struct LiveCollider {
    pub handle: ColliderHandle,
    pub node: usize,
    /// Geometry frame in node space.
    pub offset: Pose,
    /// The collider sits on an animated bone and follows the rig each step.
    pub bone: bool,
}

pub struct Rabbit {
    /// Runtime id for frame rows.
    pub id: u32,
    pub body: RigidBodyHandle,
    pub size: Size,
    pub shoot_health: f32,
    /// `nyulscript.runValue`.
    pub run_value: f32,
    /// Normalized time of the blend cycle.
    pub phase: f32,
    pub senses: Senses,
    pub brain: Brain,
    pub alive: bool,
    /// False while the world has no collision around the rabbit. The body is off.
    pub active: bool,
    pub next_support_check: f64,
    pub colliders: Vec<LiveCollider>,
}

impl Rabbit {
    /// The animator `RunValue`.
    pub fn blend_value(&self) -> f32 {
        (self.run_value * 2.).clamp(0., 1.)
    }
}

/// Inserts the live body at `pose` with the anim colliders at their rig rest frames.
/// Bone colliders sit on kinematic bones in the source: they take no contact forces here.
pub fn insert_body(
    r: &RabbitReference,
    rig: &Rig,
    bones: &[bool],
    size: &Size,
    pose: Pose,
    physics: &mut PhysicsWorld,
) -> Result<(RigidBodyHandle, Vec<LiveCollider>), ReferenceError> {
    let b = &r.body;
    let builder = RigidBodyBuilder::dynamic()
        .pose(pose)
        .ccd_enabled(true)
        .lock_rotations()
        .additional_mass(size.live_mass(r))
        .linear_damping(b.linear_damping)
        .angular_damping(b.angular_damping);
    let body = physics.insert_body(builder);
    let mut frames = rig.pose();
    rig.sample(&r.animator, 0., 0., size.scale, &mut frames);
    let mut colliders = Vec::with_capacity(b.colliders.len());
    for (c, bone) in b.colliders.iter().zip(bones) {
        let node = rig
            .index(c.node)
            .ok_or_else(|| ReferenceError("Rabbit collider outside the rig".into()))?;
        let f = frames.model[node];
        let geometry = crate::world::scaled_geometry(&c.geometry, [size.scale; 3]);
        let mut collider = collider_builder(&geometry)?;
        let offset = collider.position;
        let local = Pose::from_parts(f.position, f.rotation) * offset;
        collider = collider
            .position(local)
            .friction(c.friction)
            .restitution(c.restitution)
            .sensor(c.sensor)
            .density(0.)
            .collision_groups(InteractionGroups::new(
                Group::from_bits_truncate(c.membership),
                Group::from_bits_truncate(c.filter),
                InteractionTestMode::And,
            ))
            .user_data(step_surface_bits(c.step_surface) | tag_bits(c.tag));
        if *bone {
            collider = collider.solver_groups(InteractionGroups::none());
        }
        let handle = physics.insert_collider_with_parent(collider.build(), body);
        colliders.push(LiveCollider {
            handle,
            node,
            offset,
            bone: *bone,
        });
    }
    physics.refresh_queries();
    Ok((body, colliders))
}

impl Rabbit {
    pub(crate) fn new(
        id: u32,
        r: &RabbitReference,
        body: RigidBodyHandle,
        colliders: Vec<LiveCollider>,
        size: Size,
        position: Vector,
        now: f64,
        random: &mut StormRandom,
    ) -> Self {
        Self {
            id,
            body,
            shoot_health: size.shoot_health,
            size,
            run_value: 0.,
            phase: 0.,
            senses: Senses::new(&r.senses, random, now),
            brain: Brain::new(position),
            alive: true,
            active: true,
            next_support_check: now,
            colliders,
        }
    }
}
