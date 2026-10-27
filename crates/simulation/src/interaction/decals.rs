//! Bullet holes: `mainscript.ShootDecals` instances parented to the hit collider, and the
//! `DecalDestroyer` lifetime. Not saved.
use super::guns::DecalSpawn;
use crate::boundary::{DecalRow, PoseRow};
use crate::reference::{GUN_DECALS, ReferenceManifest};
use crate::vehicle::{PhysicsWorld, from_rapier};
use rapier3d::math::{Pose, Vector};
use rapier3d::prelude::{ColliderHandle, RigidBodyHandle};
use std::collections::VecDeque;

/// Live decal cap. A new decal drops the oldest. The source keeps every decal.
pub const DECAL_CAP: usize = 256;

#[derive(Clone, Copy, Debug)]
struct Decal {
    kind: u8,
    scale: f32,
    age: f32,
    collider: ColliderHandle,
    /// The collider's parent body. `local` is relative to it, else in stream space.
    body: Option<RigidBodyHandle>,
    local: Pose,
    /// The renderer interpolates the body. Other decals copy `current` into `previous`.
    smooth: bool,
    previous: Pose,
    current: Pose,
}

pub struct DecalStore {
    lifetimes: [Option<f32>; GUN_DECALS],
    /// Oldest first.
    decals: VecDeque<Decal>,
}

impl DecalStore {
    pub fn new(reference: &ReferenceManifest) -> Self {
        Self {
            lifetimes: std::array::from_fn(|i| reference.guns.decals[i].lifetime),
            decals: VecDeque::with_capacity(DECAL_CAP),
        }
    }
    /// Parents `spawn` to its collider. `smooth`: the renderer interpolates the collider's body.
    pub fn spawn(&mut self, spawn: DecalSpawn, physics: &PhysicsWorld, smooth: bool) {
        let Some(collider) = physics.rapier().colliders.get(spawn.collider) else {
            return;
        };
        let body = collider.parent();
        let local = match body {
            Some(b) => physics.rapier().bodies[b].position().inverse() * spawn.pose,
            None => spawn.pose,
        };
        if self.decals.len() == DECAL_CAP {
            self.decals.pop_front();
        }
        self.decals.push_back(Decal {
            kind: spawn.kind,
            scale: spawn.scale,
            age: 0.,
            collider: spawn.collider,
            body,
            local,
            smooth,
            previous: spawn.pose,
            current: spawn.pose,
        });
    }
    /// `DecalDestroyer`: drops decals past their lifetime.
    pub fn advance(&mut self, dt: f32) {
        let lifetimes = self.lifetimes;
        self.decals.retain_mut(|d| {
            d.age += dt;
            lifetimes[d.kind as usize].is_none_or(|l| d.age < l)
        });
    }
    /// Follows the parent bodies after a step. Drops decals whose collider or body left.
    pub fn commit(&mut self, physics: &PhysicsWorld, teleport: bool) {
        let rapier = physics.rapier();
        self.decals.retain_mut(|d| {
            if !rapier.colliders.contains(d.collider) {
                return false;
            }
            let pose = match d.body {
                Some(b) if !physics.body_active(b) => return false,
                Some(b) => match rapier.bodies.get(b) {
                    Some(body) => body.position() * d.local,
                    None => return false,
                },
                None => d.local,
            };
            d.previous = if teleport || !d.smooth {
                pose
            } else {
                d.current
            };
            d.current = pose;
            true
        });
    }
    pub fn rebase(&mut self, shift: [f32; 3]) {
        let shift = Vector::from_array(shift);
        for d in &mut self.decals {
            if d.body.is_none() {
                d.local.translation -= shift;
            }
            d.previous.translation -= shift;
            d.current.translation -= shift;
        }
    }
    /// Frame rows, oldest first. Poses are in stream space.
    pub fn rows(&self) -> impl Iterator<Item = DecalRow> + '_ {
        self.decals.iter().map(|d| DecalRow {
            previous: PoseRow::new(from_rapier(&d.previous), 0),
            current: PoseRow::new(from_rapier(&d.current), 0),
            kind: u32::from(d.kind),
            scale: d.scale,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rapier3d::math::Rotation;
    use rapier3d::prelude::ColliderBuilder;

    fn store(lifetimes: [Option<f32>; GUN_DECALS]) -> DecalStore {
        DecalStore {
            lifetimes,
            decals: VecDeque::with_capacity(DECAL_CAP),
        }
    }

    fn spawn(kind: u8, collider: ColliderHandle, x: f32) -> DecalSpawn {
        DecalSpawn {
            kind,
            collider,
            pose: Pose::from_parts(Vector::new(x, 0., 0.), Rotation::IDENTITY),
            scale: 0.3,
        }
    }

    #[test]
    fn cap_drops_oldest_and_lifetime_expires() {
        let mut physics = PhysicsWorld::empty(0, 0);
        let wall = physics.insert_collider(ColliderBuilder::cuboid(1., 1., 1.));
        let mut decals = store([None, Some(1.), None, None, None, None]);
        for i in 0..DECAL_CAP + 2 {
            decals.spawn(spawn(0, wall, i as f32), &physics, false);
        }
        let x: Vec<f32> = decals.rows().map(|r| r.current.position[0]).collect();
        assert_eq!(x.len(), DECAL_CAP);
        assert_eq!((x[0], x[DECAL_CAP - 1]), (2., (DECAL_CAP + 1) as f32));
        decals.spawn(spawn(1, wall, -1.), &physics, false);
        decals.advance(0.5);
        assert_eq!(decals.rows().filter(|r| r.kind == 1).count(), 1);
        decals.advance(0.5);
        assert_eq!(decals.rows().filter(|r| r.kind == 1).count(), 0);
        assert_eq!(decals.rows().count(), DECAL_CAP - 1);
        physics.remove_collider(wall);
        decals.commit(&physics, false);
        assert_eq!(decals.rows().count(), 0);
    }
}
