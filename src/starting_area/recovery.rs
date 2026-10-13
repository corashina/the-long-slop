//! Recovery clearance is complete before any transported body changes pose.
use super::*;
use rapier3d::{
    math::{Pose, Vector},
    parry::{query::intersection_test, shape::Shape},
    prelude::{Collider, ColliderHandle, QueryFilter},
};
#[derive(Clone, Copy)]
pub struct RecoveryTransport {
    pub vehicle: VehiclePose,
    pub player: Option<VehiclePose>,
    pub held_delta: Pose,
}
impl GarageRuntime {
    pub fn collect_recovery_ownership(&mut self, reference: &ReferenceManifest) {
        self.recovery_bodies.clear();
        self.recovery_held_bodies.clear();
        let chassis = self.physics.chassis_handle();
        self.recovery_bodies.push(chassis);
        let inverse = self.physics.rapier().bodies[chassis].position().inverse();
        let held = self.area.interaction.held();
        for (&id, &handle) in &self.area.bodies {
            if belongs_to_vehicle(reference, id) && !self.recovery_bodies.contains(&handle) {
                self.recovery_bodies.push(handle);
            }
        }
        for item in self.area.items.iter() {
            let Some(handle) = item.body else { continue };
            if held.is_some_and(|root| self.area.items.attached_to(item.id, root)) {
                if !self.recovery_held_bodies.contains(&handle) {
                    self.recovery_held_bodies.push(handle);
                }
                continue;
            }
            if self.recovery_bodies.contains(&handle) {
                continue;
            }
            let body = &self.physics.rapier().bodies[handle];
            let point = inverse * body.position().translation;
            let cargo = body.is_enabled()
                && body.is_dynamic()
                && (0..3).all(|axis| {
                    point[axis] >= self.recovery_bounds.min[axis]
                        && point[axis] <= self.recovery_bounds.max[axis]
                });
            if self.area.items.mounted(item.id) || cargo {
                self.recovery_bodies.push(handle);
            }
        }
    }
    pub fn recovery_clearance(
        &self,
        reference: &ReferenceManifest,
        pose: VehiclePose,
    ) -> Option<RecoveryTransport> {
        recovery_clearance(
            reference,
            &self.area,
            &self.physics,
            &self.recovery_bodies,
            &self.recovery_held_bodies,
            pose,
        )
    }
    pub fn advance_recovery(
        &mut self,
        reference: &ReferenceManifest,
    ) -> Result<bool, ReferenceError> {
        if !self.recovering {
            return Ok(false);
        }
        self.collect_recovery_ownership(reference);
        let origin = self.stream.origin;
        let area = &self.area;
        let bodies = &self.recovery_bodies;
        let held = &self.recovery_held_bodies;
        let Some(target) =
            self.stream
                .prepare_recovery_pose(&self.physics, |physics, absolute| {
                    recovery_clearance(
                        reference,
                        area,
                        physics,
                        bodies,
                        held,
                        VehiclePose {
                            position: origin.local(absolute.position),
                            rotation: absolute.rotation,
                        },
                    )
                    .is_some()
                })?
        else {
            return Ok(false);
        };
        let plan = self
            .recovery_clearance(
                reference,
                VehiclePose {
                    position: origin.local(target.position),
                    rotation: target.rotation,
                },
            )
            .ok_or_else(|| ReferenceError("Source recovery clearance changed".into()))?;
        let player = plan
            .player
            .map_or(target.position, |p| origin.absolute(p.position));
        let permit = self
            .stream
            .preflight_recovery(target, player, target.position)?;
        self.clear_input();
        self.aim = ResolvedAim::default();
        let Self {
            stream,
            area,
            physics,
            wheels,
            recovery_bodies,
            recovery_held_bodies,
            ..
        } = self;
        stream.finish_recovery(permit, || {
            transport_prepared(
                reference,
                area,
                physics,
                wheels,
                recovery_bodies,
                recovery_held_bodies,
                plan,
            )
        })?;
        self.recovering = false;
        Ok(true)
    }
}
fn transport_prepared(
    reference: &ReferenceManifest,
    area: &mut StartingArea,
    physics: &mut PhysicsWorld,
    wheels: &mut crate::interaction::wheels::WheelAssemblies,
    bodies: &[RigidBodyHandle],
    held: &[RigidBodyHandle],
    plan: RecoveryTransport,
) {
    area.interaction
        .finish_cancelled_input(&mut area.player, physics);
    let old = *physics.rapier().bodies[physics.chassis_handle()].position();
    let delta = to_rapier(plan.vehicle) * old.inverse();
    area.interaction.drop_held(&mut area.items, physics);
    physics.reset(plan.vehicle);
    for &body in bodies {
        if body != physics.chassis_handle() {
            transport_body(physics, body, delta);
        }
    }
    area.interaction.sync_mechanisms(physics);
    area.sync_attachments(physics, false);
    area.sync_compound(physics);
    area.interaction
        .sync_parts(&mut area.items, physics)
        .expect("bound parts");
    wheels
        .sync(&mut area.items, physics)
        .expect("validated wheel bindings");
    if let Some(pose) = plan.player {
        physics.rapier_mut().remove_body(area.player.body_handle());
        area.player = PlayerState::new(&reference.player, pose, physics);
        area.player.configure_seats(&reference.seats);
        area.interaction.bind_player_body(area.player.body_handle());
    } else {
        transport_body(physics, area.player.body_handle(), delta);
    }
    for &body in held {
        transport_body(physics, body, plan.held_delta);
    }
    area.interaction
        .sync_parts(&mut area.items, physics)
        .expect("bound parts");
    physics.refresh_queries();
    area.items.sync_poses(physics);
    area.commit_poses(physics, true);
}

pub fn recovery_clearance(
    reference: &ReferenceManifest,
    area: &StartingArea,
    physics: &PhysicsWorld,
    bodies: &[RigidBodyHandle],
    held: &[RigidBodyHandle],
    pose: VehiclePose,
) -> Option<RecoveryTransport> {
    let old_car = *physics.rapier().bodies[physics.chassis_handle()].position();
    let delta = to_rapier(pose) * old_car.inverse();
    let player_body = area.player.body_handle();
    let obstacle = |_: ColliderHandle, collider: &Collider| {
        collider.parent().is_none_or(|body| {
            body != player_body && !bodies.contains(&body) && !held.contains(&body)
        })
    };
    let clear_body = |body: RigidBodyHandle, shift: Pose| {
        physics.rapier().bodies[body]
            .colliders()
            .iter()
            .all(|handle| {
                let collider = &physics.rapier().colliders[*handle];
                collider.is_sensor()
                    || !collider.is_enabled()
                    || physics
                        .queries(
                            QueryFilter::default()
                                .exclude_sensors()
                                .predicate(&obstacle),
                        )
                        .intersect_shape(shift * *collider.position(), collider.shape())
                        .next()
                        .is_none()
            })
    };
    if !bodies.iter().all(|body| clear_body(*body, delta)) {
        return None;
    }
    let clear_transported = |target: Pose, shape: &dyn Shape| {
        bodies.iter().all(|body| {
            physics.rapier().bodies[*body]
                .colliders()
                .iter()
                .all(|handle| {
                    let collider = &physics.rapier().colliders[*handle];
                    collider.is_sensor()
                        || !collider.is_enabled()
                        || intersection_test(
                            &target,
                            shape,
                            &(delta * *collider.position()),
                            collider.shape(),
                        )
                        .is_ok_and(|hit| !hit.intersecting)
                })
        })
    };
    let old_view = to_rapier(area.player.view_pose(physics));
    if area.player.seat().is_some() {
        if !held.iter().all(|body| clear_body(*body, delta)) {
            return None;
        }
        return Some(RecoveryTransport {
            vehicle: pose,
            player: None,
            held_delta: delta,
        });
    }
    let standing = area.player.standing_shape();
    for distance in [3.0, 4.0, 5.0, 6.0] {
        for side in [-1.0, 1.0] {
            let mut candidate = to_rapier(pose);
            candidate.translation += candidate.rotation
                * Vector::new(
                    distance * side,
                    reference.player.standing_height * 0.5 - 1.1 + 0.05,
                    0.0,
                );
            if physics
                .queries(
                    QueryFilter::default()
                        .exclude_sensors()
                        .predicate(&obstacle),
                )
                .intersect_shape(candidate, standing)
                .next()
                .is_some()
                || !clear_transported(candidate, standing)
            {
                continue;
            }
            let mut view = candidate;
            view.translation +=
                view.rotation * (Vector::Y * reference.player.standing_height * 0.5);
            let held_delta = view * old_view.inverse();
            if !held.iter().all(|body| clear_body(*body, held_delta)) {
                continue;
            }
            if !held.iter().all(|body| {
                physics.rapier().bodies[*body]
                    .colliders()
                    .iter()
                    .all(|handle| {
                        let collider = &physics.rapier().colliders[*handle];
                        collider.is_sensor()
                            || !collider.is_enabled()
                            || clear_transported(
                                held_delta * *collider.position(),
                                collider.shape(),
                            )
                    })
            }) {
                continue;
            }
            return Some(RecoveryTransport {
                vehicle: pose,
                player: Some(crate::vehicle::from_rapier(&candidate)),
                held_delta,
            });
        }
    }
    None
}
