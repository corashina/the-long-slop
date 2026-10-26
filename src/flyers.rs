//! Source randomFlyer: bats that fly around their start point. Not saved.
use crate::interaction::items::{ItemLocation, ItemStore};
use crate::reference::{FlyerReference, ItemId};
use crate::vehicle::PhysicsWorld;
use crate::weather::StormRandom;
use crate::world::StructureInstance;
use rapier3d::math::{Rotation, Vector};
use rapier3d::prelude::RigidBodyHandle;

/// One live bat item. Positions are local physics positions.
#[derive(Debug)]
pub struct Flyer {
    pub item: ItemId,
    pub body: RigidBodyHandle,
    start: Vector,
    target: Vector,
    next_time: f64,
    speed: f32,
    next_sound_time: f64,
}

/// One source randomFlyer.Sound call of this step.
#[derive(Clone, Copy, Debug)]
pub struct Squeak {
    pub item: ItemId,
    pub clip: u32,
    pub gain: f32,
    pub pitch: f32,
}

pub struct Flyers {
    flyers: Vec<Flyer>,
    squeaks: Vec<Squeak>,
    /// Structure and item membership revisions of the last sync.
    revision: Option<(u64, u64)>,
    random: StormRandom,
}

impl Flyers {
    pub fn new(seed: u64) -> Self {
        Self {
            flyers: Vec::with_capacity(64),
            squeaks: Vec::with_capacity(64),
            revision: None,
            random: StormRandom(seed ^ 0x666c_7965_7200_0001),
        }
    }
    pub fn flyers(&self) -> &[Flyer] {
        &self.flyers
    }
    /// Squeaks of the last step.
    pub fn squeaks(&self) -> &[Squeak] {
        &self.squeaks
    }

    /// Adds a flyer for each new live bat and drops flyers whose body is gone.
    /// A bat is a structure of the flyer prototype with a bound item body.
    /// `revision` is the structure revision. Sync runs only when it or the item membership changes.
    pub fn sync(
        &mut self,
        r: &FlyerReference,
        now: f64,
        revision: u64,
        structures: &[StructureInstance],
        items: &ItemStore,
        physics: &PhysicsWorld,
    ) {
        let key = (revision, items.membership_revision());
        if self.revision == Some(key) {
            return;
        }
        self.revision = Some(key);
        // A new body of the same item is a new source object: it starts again.
        self.flyers
            .retain(|f| items.get(f.item).is_some_and(|i| i.body == Some(f.body)));
        for structure in structures
            .iter()
            .filter(|s| s.placement.prototype == r.prefab)
        {
            let Some(item) = items.generated_item_id(&structure.placement.id) else {
                continue;
            };
            let Some(body) = items.get(item).and_then(|i| i.body) else {
                continue;
            };
            if self.flyers.iter().any(|f| f.item == item) {
                continue;
            }
            // Source Start.
            let start = physics.rapier().bodies[body].translation();
            let mut flyer = Flyer {
                item,
                body,
                start,
                target: start,
                next_time: now,
                speed: 0.,
                next_sound_time: now,
            };
            next(&mut flyer, r, &mut self.random, now, start);
            next_sound(&mut flyer, r, &mut self.random, now);
            self.flyers.push(flyer);
        }
    }

    /// Source randomFlyer.Update. Sets the body velocity and rotation before the physics step.
    /// A carried, stored, frozen or inactive bat does not fly.
    pub fn step(
        &mut self,
        r: &FlyerReference,
        now: f64,
        dt: f32,
        items: &ItemStore,
        physics: &mut PhysicsWorld,
    ) {
        self.squeaks.clear();
        for f in &mut self.flyers {
            if !items
                .get(f.item)
                .is_some_and(|i| i.location == ItemLocation::World && i.inactive_pose.is_none())
                || !physics.body_active(f.body)
            {
                continue;
            }
            let Some(body) = physics.rapier().bodies.get(f.body) else {
                continue;
            };
            if !body.is_dynamic() || !body.is_enabled() {
                continue;
            }
            let position = body.translation();
            if now > f.next_time {
                next(f, r, &mut self.random, now, position);
            }
            if now > f.next_sound_time {
                next_sound(f, r, &mut self.random, now);
                let clip = r.clips
                    [((self.random.unit() * r.clips.len() as f32) as usize).min(r.clips.len() - 1)];
                let gain = self.random.range(r.volume_range[0], r.volume_range[1]);
                let pitch = self.random.range(r.pitch_range[0], r.pitch_range[1]);
                if self.squeaks.len() < self.squeaks.capacity() {
                    self.squeaks.push(Squeak {
                        item: f.item,
                        clip,
                        gain,
                        pitch,
                    });
                }
            }
            let dir = (f.target - position).clamp_length_max(f.speed);
            let velocity = steer(body.linvel(), dir, dt * r.velocity_lerp);
            let rotation =
                (dir != Vector::ZERO).then(|| turn(*body.rotation(), dir, dt * r.rot_lerp));
            // Waking each step keeps the bat flying, as the source does.
            physics.edit_body(f.body, |body| {
                body.set_linvel(velocity, true);
                if let Some(rotation) = rotation {
                    body.set_rotation(rotation, true);
                }
            });
        }
    }

    /// Moves the local positions with the physics origin.
    pub fn rebase(&mut self, shift: Vector) {
        for f in &mut self.flyers {
            f.start -= shift;
            f.target -= shift;
        }
    }
}

/// Source randomFlyer.Next.
fn next(f: &mut Flyer, r: &FlyerReference, random: &mut StormRandom, now: f64, position: Vector) {
    f.next_time = now + f64::from(random.range(r.time_range[0], r.time_range[1]));
    let offset = Vector::from_array(random.on_unit_sphere()) * random.range(r.range[0], r.range[1]);
    f.target = f.start + (position + offset - f.start).clamp_length_max(r.max_range_from_start);
    f.speed = random.range(r.speed_range[0], r.speed_range[1]);
}

/// Source randomFlyer.NextSound.
fn next_sound(f: &mut Flyer, r: &FlyerReference, random: &mut StormRandom, now: f64) {
    f.next_sound_time = now + f64::from(random.range(r.sound_time_range[0], r.sound_time_range[1]));
}

/// Unity Vector3.Lerp: `t` clamps to [0, 1].
fn steer(velocity: Vector, dir: Vector, t: f32) -> Vector {
    velocity.lerp(dir, t.clamp(0., 1.))
}

/// Unity Quaternion.Lerp to LookRotation(dir) with up +Y.
/// Source +Z is port -Z, so the port -Z of the result faces `dir`.
fn turn(rotation: Rotation, dir: Vector, t: f32) -> Rotation {
    let back = -dir.normalize();
    let right = Vector::Y.cross(back);
    let look = if right.length_squared() > 1e-12 {
        let right = right.normalize();
        Rotation::from_mat3(&rapier3d::glamx::Mat3::from_cols(
            right,
            back.cross(right),
            back,
        ))
    } else {
        Rotation::from_rotation_arc(Vector::NEG_Z, -back)
    };
    rotation.lerp(look, t.clamp(0., 1.))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flyer(start: Vector) -> Flyer {
        Flyer {
            item: ItemId(0),
            body: RigidBodyHandle::invalid(),
            start,
            target: start,
            next_time: 0.,
            speed: 0.,
            next_sound_time: 0.,
        }
    }

    #[test]
    fn targets_stay_near_start_and_speeds_in_range() {
        let r = FlyerReference::fixture();
        let mut random = StormRandom(1);
        let start = Vector::new(10., 50., -20.);
        let mut f = flyer(start);
        for i in 0..1000 {
            let far = Vector::from_array(random.on_unit_sphere()) * (i as f32 * 3.);
            next(&mut f, &r, &mut random, 0., start + far);
            assert!((f.target - start).length() <= r.max_range_from_start + 1e-3);
            assert!((r.speed_range[0]..=r.speed_range[1]).contains(&f.speed));
        }
    }

    #[test]
    fn velocity_moves_toward_dir() {
        let r = FlyerReference::fixture();
        let dir = Vector::new(0., 0., -2.);
        let t = 0.02 * r.velocity_lerp;
        let velocity = steer(Vector::ZERO, dir, t);
        assert!((velocity - dir * t).length() < 1e-6);
        let rotation = turn(Rotation::IDENTITY, Vector::X, 1.);
        assert!((rotation * Vector::NEG_Z - Vector::X).length() < 1e-5);
    }
}
