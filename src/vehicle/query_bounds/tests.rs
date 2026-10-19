use super::*;
use rapier3d::math::{Rotation, Vector};
use rapier3d::parry::bounding_volume::BoundingVolume;
use rapier3d::prelude::{
    ColliderBuilder, QueryFilter, QueryPipeline, Ray, RigidBodyBuilder, SharedShape,
};
use std::sync::Arc;

#[test]
fn direct_mutations_refresh_hits_without_rebuilding_unchanged_bounds() {
    let mut physics = empty_world();
    let collider = physics.insert_collider(ColliderBuilder::ball(1.0));
    assert_eq!(physics.query_bounds.refresh(&mut physics.physics), 1);
    assert_eq!(ray_hit(&physics, 0.0, 0.0), Some(collider));
    let capacities = (
        physics.query_bounds.states.capacity(),
        physics.query_bounds.queue_capacities(),
        physics.query_bounds.retained_bytes(),
    );
    for _ in 0..8 {
        assert_eq!(physics.query_bounds.refresh(&mut physics.physics), 0);
    }
    assert_eq!(
        capacities,
        (
            physics.query_bounds.states.capacity(),
            physics.query_bounds.queue_capacities(),
            physics.query_bounds.retained_bytes()
        )
    );

    physics.edit_collider(collider, |collider| {
        collider.set_translation(Vector::new(5.0, 0.0, 0.0))
    });
    assert_eq!(physics.query_bounds.refresh(&mut physics.physics), 1);
    assert_eq!(ray_hit(&physics, 0.0, 0.0), None);
    assert_eq!(ray_hit(&physics, 5.0, 0.0), Some(collider));
    physics.edit_collider(collider, |collider| {
        collider.set_shape(SharedShape::ball(2.0))
    });
    assert_eq!(physics.query_bounds.refresh(&mut physics.physics), 1);
    assert_eq!(ray_hit(&physics, 6.5, 0.0), Some(collider));
    physics.edit_collider(collider, |collider| {
        collider.shape_mut().as_ball_mut().unwrap().radius = 3.0;
    });
    assert_eq!(physics.query_bounds.refresh(&mut physics.physics), 1);
    assert_eq!(ray_hit(&physics, 7.5, 0.0), Some(collider));
    physics.edit_collider(collider, |collider| collider.set_enabled(false));
    assert_eq!(physics.query_bounds.refresh(&mut physics.physics), 0);
    assert_eq!(ray_hit(&physics, 5.0, 0.0), None);
    physics.edit_collider(collider, |collider| collider.set_enabled(true));
    assert_eq!(physics.query_bounds.refresh(&mut physics.physics), 1);
    assert_eq!(ray_hit(&physics, 5.0, 0.0), Some(collider));

    let retired_shape = Arc::downgrade(&physics.rapier().colliders[collider].shared_shape().0);
    physics.remove_collider(collider);
    let replacement = physics
        .insert_collider(ColliderBuilder::ball(0.5).translation(Vector::new(-5.0, 0.0, 0.0)));
    assert_eq!(collider.into_raw_parts().0, replacement.into_raw_parts().0);
    assert_ne!(collider, replacement);
    assert_eq!(physics.query_bounds.refresh(&mut physics.physics), 1);
    assert!(retired_shape.upgrade().is_none());
    assert_eq!(ray_hit(&physics, 5.0, 0.0), None);
    assert_eq!(ray_hit(&physics, -5.0, 0.0), Some(replacement));
    physics.remove_collider(replacement);
    assert_eq!(physics.query_bounds.refresh(&mut physics.physics), 0);
    assert!(physics.query_bounds.bvh.is_empty());
    assert!(physics.query_bounds.states.iter().all(Option::is_none));
}

#[test]
fn body_attachment_and_enablement_changes_keep_queries_current() {
    let mut physics = empty_world();
    physics.set_gravity(Vector::ZERO);
    let (body, collider) = physics.insert(
        RigidBodyBuilder::dynamic(),
        ColliderBuilder::cuboid(2.0, 0.5, 0.25),
    );
    let other =
        physics.insert_body(RigidBodyBuilder::fixed().translation(Vector::new(-8.0, 0.0, 0.0)));
    physics.query_bounds.refresh(&mut physics.physics);
    physics.step_physics();
    physics.edit_body(body, |body| {
        body.set_translation(Vector::new(5.0, 0.0, 0.0), true)
    });
    assert_eq!(physics.query_bounds.refresh(&mut physics.physics), 1);
    assert_eq!(ray_hit(&physics, 5.0, 0.0), Some(collider));
    physics.edit_body(body, |body| {
        body.set_rotation(Rotation::from_rotation_y(std::f32::consts::FRAC_PI_2), true)
    });
    physics.query_bounds.refresh(&mut physics.physics);
    assert_eq!(ray_hit(&physics, 5.0, 1.5), Some(collider));
    physics.step_physics();
    physics.edit_collider(collider, |collider| {
        collider.set_translation_wrt_parent(Vector::new(3.0, 0.0, 0.0))
    });
    assert_eq!(physics.query_bounds.refresh(&mut physics.physics), 1);
    assert_eq!(ray_hit(&physics, 5.0, -3.0), Some(collider));
    physics.set_collider_parent(collider, Some(other));
    physics.query_bounds.refresh(&mut physics.physics);
    assert_eq!(ray_hit(&physics, -5.0, 0.0), Some(collider));
    assert_eq!(ray_hit(&physics, 5.0, -3.0), None);
    physics.edit_body(other, |body| body.set_enabled(false));
    physics.query_bounds.refresh(&mut physics.physics);
    assert_eq!(ray_hit(&physics, -5.0, 0.0), None);
    physics.step_physics();
    physics.edit_body(other, |body| body.set_enabled(true));
    physics.step_physics();
    physics.query_bounds.refresh(&mut physics.physics);
    assert_eq!(ray_hit(&physics, -5.0, 0.0), Some(collider));
    physics.set_collider_parent(collider, None);
    physics.query_bounds.refresh(&mut physics.physics);
    physics.edit_collider(collider, |collider| {
        collider.set_translation(Vector::new(-12.0, 0.0, 0.0))
    });
    physics.query_bounds.refresh(&mut physics.physics);
    assert_eq!(ray_hit(&physics, -12.0, 0.0), Some(collider));
    assert_eq!(physics.query_bounds.refresh(&mut physics.physics), 0);
}

#[test]
fn query_refresh_keeps_new_bodies_and_contacts_in_the_physics_pipeline() {
    let mut physics = empty_world();
    physics.insert(
        RigidBodyBuilder::fixed().translation(Vector::new(0.0, -0.5, 0.0)),
        ColliderBuilder::cuboid(10.0, 0.5, 10.0),
    );
    let (body, collider) = physics.insert(
        RigidBodyBuilder::dynamic().translation(Vector::new(0.0, 3.0, 0.0)),
        ColliderBuilder::ball(0.5),
    );
    physics.query_bounds.refresh(&mut physics.physics);
    let mut scan = super::reference::ScanBounds::default();
    assert_matches_scan(&mut physics, &mut scan, &[Vector::Y * 3.0]);
    for _ in 0..240 {
        physics.query_bounds.refresh(&mut physics.physics);
        physics.step_physics();
        let point = physics.rapier().bodies[body].translation();
        assert_eq!(ray_hit(&physics, point.x, point.z), Some(collider));
        assert_matches_scan(&mut physics, &mut scan, &[point]);
    }
    assert!((physics.rapier().bodies[body].translation().y - 0.5).abs() < 0.05);
    assert!(physics.rapier().bodies[body].is_sleeping());
    assert_eq!(ray_hit(&physics, 0.0, 0.0), Some(collider));
    assert!(
        physics
            .rapier()
            .narrow_phase
            .contact_pairs()
            .any(|pair| pair.has_any_active_contact())
    );
    physics.edit_body(body, |body| {
        body.set_translation(Vector::new(0.0, 5.0, 0.0), true)
    });
    physics.query_bounds.refresh(&mut physics.physics);
    physics.step_physics();
    assert!(physics.rapier().bodies[body].translation().y < 5.0);
}

#[test]
fn public_queries_follow_step_without_an_extra_caller_refresh() {
    let mut world = empty_world();
    world.set_gravity(Vector::ZERO);
    let (body, collider) = world.insert(
        RigidBodyBuilder::dynamic().linvel(Vector::new(60.0, 0.0, 0.0)),
        ColliderBuilder::ball(0.01),
    );
    world.refresh_queries();
    world.step_physics();
    let x = world.rapier().bodies[body].translation().x;
    let ray = Ray::new(Vector::new(x, 10.0, 0.0), -Vector::Y);
    assert_eq!(
        world
            .queries(QueryFilter::default())
            .cast_ray(&ray, 20.0, true)
            .map(|hit| hit.0),
        Some(collider)
    );
    assert!(
        world
            .queries(QueryFilter::default())
            .cast_ray(
                &Ray::new(Vector::new(0.0, 10.0, 0.0), -Vector::Y),
                20.0,
                true
            )
            .is_none()
    );
}

pub(super) fn empty_world() -> crate::vehicle::PhysicsWorld {
    let mut physics = PhysicsWorld::new();
    physics.integration_parameters.dt = crate::simulation::FIXED_DT;
    crate::vehicle::PhysicsWorld {
        inactive_bodies: Default::default(),
        physics,
        query_bounds: QueryBounds::default(),
        chassis: Vec::new(),
        vehicles: Vec::new(),
    }
}

pub(super) fn ray_hit(
    world: &crate::vehicle::PhysicsWorld,
    x: f32,
    z: f32,
) -> Option<ColliderHandle> {
    world
        .queries(QueryFilter::default())
        .cast_ray(&Ray::new(Vector::new(x, 10.0, z), -Vector::Y), 20.0, true)
        .map(|hit| hit.0)
}

pub(super) fn assert_matches_scan(
    world: &mut crate::vehicle::PhysicsWorld,
    scan: &mut super::reference::ScanBounds,
    samples: &[Vector],
) {
    world.refresh_queries();
    scan.refresh(&mut world.physics);
    let physics = world.rapier();
    let candidate = world.queries(QueryFilter::default());
    let reference = QueryPipeline {
        dispatcher: physics.narrow_phase.query_dispatcher(),
        bvh: &scan.bvh,
        bodies: &physics.bodies,
        colliders: &physics.colliders,
        filter: QueryFilter::default(),
    };
    let ball = rapier3d::parry::shape::Ball::new(0.25);
    for &sample in samples {
        let origin = sample + Vector::Y * 10.0;
        let ray = Ray::new(origin, -Vector::Y);
        assert_eq!(
            candidate.cast_ray(&ray, 20.0, true),
            reference.cast_ray(&ray, 20.0, true),
            "ray at {sample:?}"
        );
        let shape_hit = |queries: &QueryPipeline<'_>| {
            queries
                .cast_shape(
                    &Pose::from_translation(origin),
                    -Vector::Y,
                    &ball,
                    rapier3d::parry::query::ShapeCastOptions {
                        max_time_of_impact: 20.0,
                        ..Default::default()
                    },
                )
                .map(|(handle, hit)| {
                    (
                        handle,
                        hit.time_of_impact,
                        hit.witness1,
                        hit.witness2,
                        hit.normal1,
                        hit.normal2,
                        hit.status,
                        hit.subshape1,
                        hit.subshape2,
                    )
                })
        };
        assert_eq!(
            shape_hit(&candidate),
            shape_hit(&reference),
            "ball cast at {sample:?}"
        );
        let overlap_handles = |queries: &QueryPipeline<'_>| {
            let mut handles: Vec<_> = queries
                .intersect_shape(Pose::from_translation(sample), &ball)
                .map(|(handle, _)| handle)
                .collect();
            handles.sort_unstable_by_key(|handle| handle.into_raw_parts());
            handles
        };
        assert_eq!(
            overlap_handles(&candidate),
            overlap_handles(&reference),
            "ball overlap at {sample:?}"
        );
    }

    let mut enabled: Vec<_> = physics
        .colliders
        .iter()
        .filter(|(_, collider)| {
            collider.is_enabled()
                && collider
                    .parent()
                    .is_none_or(|parent| physics.bodies[parent].is_enabled())
        })
        .map(|(handle, _)| handle)
        .collect();
    enabled.sort_unstable_by_key(|handle| handle.into_raw_parts());
    let mut candidate_handles: Vec<_> = world
        .query_bounds
        .states
        .iter()
        .flatten()
        .filter(|state| {
            world
                .query_bounds
                .bvh
                .leaf_node(state.handle.into_raw_parts().0)
                .is_some()
        })
        .map(|state| state.handle)
        .collect();
    candidate_handles.sort_unstable_by_key(|handle| handle.into_raw_parts());
    let mut reference_handles: Vec<_> = scan
        .states
        .iter()
        .flatten()
        .filter(|state| {
            scan.bvh
                .leaf_node(state.handle.into_raw_parts().0)
                .is_some()
        })
        .map(|state| state.handle)
        .collect();
    reference_handles.sort_unstable_by_key(|handle| handle.into_raw_parts());
    assert_eq!(candidate_handles, enabled, "candidate collider generations");
    assert_eq!(reference_handles, enabled, "scan collider generations");
    assert_eq!(world.query_bounds.bvh.leaf_count() as usize, enabled.len());
    assert_eq!(scan.bvh.leaf_count() as usize, enabled.len());
    for handle in enabled {
        let slot = handle.into_raw_parts().0;
        let candidate_aabb = world.query_bounds.bvh.leaf_node(slot).unwrap().aabb();
        let reference_aabb = scan.bvh.leaf_node(slot).unwrap().aabb();
        assert_eq!(candidate_aabb, reference_aabb, "bounds for {handle:?}");
        // Parry retains an old leaf when it contains the new bounds.
        assert!(candidate_aabb.contains(&physics.colliders[handle].compute_aabb()));
    }
}

#[test]
fn scan_comparison_covers_hits_overlaps_and_bounds() {
    let mut world = empty_world();
    let first = world.insert_collider(ColliderBuilder::ball(1.0));
    let second =
        world.insert_collider(ColliderBuilder::ball(1.0).translation(Vector::new(8.0, 0.0, 0.0)));
    let (body, _) = world.insert(
        RigidBodyBuilder::fixed().translation(Vector::new(-8.0, 0.0, 0.0)),
        ColliderBuilder::cuboid(1.0, 0.5, 0.75),
    );
    let samples =
        [0.0, 1.1, 4.0, 5.1, 8.0, 12.0, -8.0, -4.0, -12.0, 20.0].map(|x| Vector::new(x, 0.0, 0.0));
    let mut scan = super::reference::ScanBounds::default();
    world.refresh_queries();
    assert_eq!(ray_hit(&world, 0.0, 0.0), Some(first));
    assert_eq!(ray_hit(&world, 8.0, 0.0), Some(second));
    assert_eq!(ray_hit(&world, 20.0, 0.0), None);
    assert_matches_scan(&mut world, &mut scan, &samples);

    world.edit_collider(first, |collider| {
        collider.set_translation(Vector::new(4.0, 0.0, 0.0))
    });
    world.edit_collider(second, |collider| {
        collider.set_translation(Vector::new(12.0, 0.0, 0.0))
    });
    world.edit_body(body, |body| {
        body.set_translation(Vector::new(-4.0, 0.0, 0.0), true)
    });
    assert_matches_scan(&mut world, &mut scan, &samples);

    world.edit_collider(first, |collider| collider.set_enabled(false));
    world.edit_body(body, |body| body.set_enabled(false));
    assert_matches_scan(&mut world, &mut scan, &samples);

    world.remove_collider(first);
    let replacement =
        world.insert_collider(ColliderBuilder::ball(0.5).translation(Vector::new(-12.0, 0.0, 0.0)));
    assert_eq!(first.into_raw_parts().0, replacement.into_raw_parts().0);
    assert_ne!(first, replacement);
    assert_matches_scan(&mut world, &mut scan, &samples);
}

#[test]
fn tracked_edits_cover_optional_handles_and_lifecycle() {
    let mut world = empty_world();
    let parent = world.insert_body(RigidBodyBuilder::fixed());
    let child = world.insert_collider_with_parent(ColliderBuilder::ball(1.0), parent);
    world.refresh_queries();
    assert_eq!(world.query_bounds.pending_counts(), (0, 0, 0));
    assert_eq!(world.query_bounds.step_counts(), (1, 1));
    for _ in 0..3 {
        world.edit_body(parent, |body| body.set_enabled(true));
        world.edit_collider(child, |collider| collider.set_enabled(true));
    }
    assert_eq!(world.query_bounds.pending_counts(), (1, 1, 0));
    assert_eq!(world.query_bounds.step_counts(), (1, 1));
    world.refresh_queries();
    assert_eq!(world.query_bounds.pending_counts(), (0, 0, 0));
    assert_eq!(world.query_bounds.step_counts(), (1, 1));
    assert_eq!(
        world.try_edit_body(RigidBodyHandle::invalid(), |_| panic!(
            "invalid body closure"
        )),
        None::<()>
    );
    assert_eq!(
        world.try_edit_collider(ColliderHandle::invalid(), |_| panic!(
            "invalid collider closure"
        )),
        None::<()>
    );
    assert!(world.remove_body(parent).is_some());
    assert!(!world.rapier().colliders.contains(child));
    assert_eq!(world.query_bounds.pending_counts(), (1, 0, 1));
    assert_eq!(world.query_bounds.removals.handles, [child]);
    world.refresh_queries();
    assert_eq!(world.query_bounds.pending_counts(), (0, 0, 0));
    assert_eq!(ray_hit(&world, 0.0, 0.0), None);
}

#[test]
fn idle_refresh_and_sparse_edits_visit_only_queued_handles() {
    let mut world = empty_world();
    // Warm body and removal storage before the standalone scene fixes membership.
    let (temporary, _) = world.insert(RigidBodyBuilder::fixed(), ColliderBuilder::ball(0.5));
    world.refresh_queries();
    world.remove_body(temporary);
    world.refresh_queries();
    let mut edited = None;
    for i in 0..1024 {
        edited =
            Some(world.insert_collider(
                ColliderBuilder::ball(1.0).translation(Vector::X * (i as f32 * 4.0)),
            ));
    }
    world.refresh_queries();
    world.step_physics();
    world.refresh_queries();
    assert_eq!(
        world.query_bounds.last_work(),
        RefreshWork {
            bodies: 0,
            colliders: 0,
            aabbs: 0
        }
    );
    let edited = edited.unwrap();
    for radius in [1.5, 2.0] {
        world.edit_collider(edited, |collider| {
            collider.shape_mut().as_ball_mut().unwrap().radius = radius
        });
    }
    world.refresh_queries();
    assert_eq!(
        world.query_bounds.last_work(),
        RefreshWork {
            bodies: 0,
            colliders: 1,
            aabbs: 1
        }
    );
    world.step_physics();
    let (parent, _) = world.insert(
        RigidBodyBuilder::fixed().translation(Vector::Z * 8.0),
        ColliderBuilder::ball(0.5),
    );
    world.refresh_queries();
    world.step_physics();
    world.edit_body(parent, |body| body.user_data = 1);
    world.refresh_queries();
    assert_eq!(
        world.query_bounds.last_work(),
        RefreshWork {
            bodies: 1,
            colliders: 1,
            aabbs: 0
        }
    );
    world.step_physics();
    let before = world.query_bounds.queue_capacities();
    for _ in 0..8 {
        world.refresh_queries();
        assert_eq!(
            world.query_bounds.last_work(),
            RefreshWork {
                bodies: 0,
                colliders: 0,
                aabbs: 0
            }
        );
        assert_eq!(before, world.query_bounds.queue_capacities());
    }
}

#[test]
fn in_place_shape_and_local_pose_edits_match_scan() {
    let mut world = empty_world();
    world.set_gravity(Vector::ZERO);
    let ball = world.insert_collider(ColliderBuilder::ball(1.0));
    let (parent, child) = world.insert(
        RigidBodyBuilder::fixed().translation(Vector::X * 10.0),
        ColliderBuilder::cuboid(0.5, 0.5, 0.5),
    );
    let samples = [0.0, 2.5, 10.0, 14.0].map(|x| Vector::X * x);
    let mut scan = super::reference::ScanBounds::default();
    assert_matches_scan(&mut world, &mut scan, &samples);
    world.edit_collider(ball, |collider| {
        collider.shape_mut().as_ball_mut().unwrap().radius = 3.0
    });
    world.edit_collider(child, |collider| {
        collider.set_translation_wrt_parent(Vector::X * 4.0)
    });
    let parent_pose = *world.rapier().bodies[parent].position();
    assert_matches_scan(&mut world, &mut scan, &samples);
    assert_eq!(ray_hit(&world, 2.5, 0.0), Some(ball));
    assert_eq!(ray_hit(&world, 10.0, 0.0), None);
    assert_eq!(ray_hit(&world, 14.0, 0.0), Some(child));
    world.step_physics();
    assert_eq!(*world.rapier().bodies[parent].position(), parent_pose);
    assert_eq!(ray_hit(&world, 14.0, 0.0), Some(child));
    assert_matches_scan(&mut world, &mut scan, &samples);
}

#[test]
fn generation_reuse_does_not_remove_replacement() {
    let mut world = empty_world();
    let samples = [0.0, 8.0, 16.0, 24.0].map(|x| Vector::X * x);
    let mut scan = super::reference::ScanBounds::default();
    let (first_parent, first) = world.insert(RigidBodyBuilder::fixed(), ColliderBuilder::ball(0.5));
    assert_matches_scan(&mut world, &mut scan, &samples);
    world.remove_body(first_parent);
    let (second_parent, second) = world.insert(
        RigidBodyBuilder::fixed().translation(Vector::X * 8.0),
        ColliderBuilder::ball(0.5),
    );
    world.remove_body(second_parent);
    let (_, third) = world.insert(
        RigidBodyBuilder::fixed().translation(Vector::X * 16.0),
        ColliderBuilder::ball(0.5),
    );
    assert_eq!(first.into_raw_parts().0, second.into_raw_parts().0);
    assert_eq!(second.into_raw_parts().0, third.into_raw_parts().0);
    assert_ne!(first, second);
    assert_ne!(second, third);
    assert_matches_scan(&mut world, &mut scan, &samples);
    assert_eq!(ray_hit(&world, 0.0, 0.0), None);
    assert_eq!(ray_hit(&world, 8.0, 0.0), None);
    assert_eq!(ray_hit(&world, 16.0, 0.0), Some(third));
    for _ in 0..2 {
        world.query_bounds.requeue_step_mutations();
    }
    assert_eq!(world.query_bounds.pending_counts(), (1, 1, 0));
    world.refresh_queries();
    assert_eq!(
        world.query_bounds.last_work(),
        RefreshWork {
            bodies: 1,
            colliders: 1,
            aabbs: 0
        }
    );
    world.step_physics();
    assert_eq!(
        world.query_bounds.last_work(),
        RefreshWork {
            bodies: 1,
            colliders: 1,
            aabbs: 0
        }
    );
    assert_eq!(world.query_bounds.step_counts(), (0, 0));
    assert_eq!(ray_hit(&world, 16.0, 0.0), Some(third));
    assert_matches_scan(&mut world, &mut scan, &samples);
    let old_parent = world.insert_body(RigidBodyBuilder::fixed());
    let new_parent = world.insert_body(RigidBodyBuilder::fixed());
    world.set_collider_parent(third, Some(old_parent));
    world.set_collider_parent(third, Some(new_parent));
    world.remove_body(old_parent);
    assert!(world.rapier().colliders.contains(third));
    assert_matches_scan(&mut world, &mut scan, &samples);
    world.remove_body(new_parent);
    let (fourth_parent, fourth) = world.insert(
        RigidBodyBuilder::fixed().translation(Vector::X * 24.0),
        ColliderBuilder::ball(0.5),
    );
    world.remove_body(fourth_parent);
    let fifth = world.insert_collider(ColliderBuilder::ball(0.5).translation(Vector::X * 8.0));
    assert_eq!(third.into_raw_parts().0, fourth.into_raw_parts().0);
    assert_eq!(fourth.into_raw_parts().0, fifth.into_raw_parts().0);
    assert_ne!(third, fourth);
    assert_ne!(fourth, fifth);
    assert_matches_scan(&mut world, &mut scan, &samples);
    assert_eq!(ray_hit(&world, 16.0, 0.0), None);
    assert_eq!(ray_hit(&world, 24.0, 0.0), None);
    assert_eq!(ray_hit(&world, 8.0, 0.0), Some(fifth));
}

#[test]
fn fixed_parent_enablement_survives_intervening_refresh() {
    let mut world = empty_world();
    let (parent, child) = world.insert(RigidBodyBuilder::fixed(), ColliderBuilder::ball(0.5));
    let other = world.insert_body(RigidBodyBuilder::fixed().translation(Vector::X * 8.0));
    let samples = [Vector::ZERO, Vector::X * 8.0];
    let mut scan = super::reference::ScanBounds::default();
    assert_matches_scan(&mut world, &mut scan, &samples);
    world.step_physics();
    assert_matches_scan(&mut world, &mut scan, &samples);
    for enabled in [false, true, false] {
        world.edit_body(parent, |body| body.set_enabled(enabled));
        assert_matches_scan(&mut world, &mut scan, &samples);
        if !enabled {
            assert_eq!(ray_hit(&world, 0.0, 0.0), None);
        }
        world.step_physics();
        assert_eq!(ray_hit(&world, 0.0, 0.0), enabled.then_some(child));
        assert_matches_scan(&mut world, &mut scan, &samples);
    }
    assert!(!world.rapier().colliders[child].is_enabled());
    world.set_collider_parent(child, Some(other));
    assert_matches_scan(&mut world, &mut scan, &samples);
    world.step_physics();
    assert_eq!(ray_hit(&world, 8.0, 0.0), None);
    assert_matches_scan(&mut world, &mut scan, &samples);
    // Rapier retains DisabledByParent on reparenting until an enablement notification.
    for enabled in [false, true] {
        world.edit_body(other, |body| body.set_enabled(enabled));
        assert_matches_scan(&mut world, &mut scan, &samples);
        world.step_physics();
        assert_eq!(ray_hit(&world, 8.0, 0.0), enabled.then_some(child));
        assert_matches_scan(&mut world, &mut scan, &samples);
    }
}

#[test]
fn simulation_motion_sleep_wake_and_kinematic_queries_match_scan() {
    let mut world = empty_world();
    world.set_gravity(Vector::ZERO);
    let (dynamic, dc) = world.insert(
        RigidBodyBuilder::dynamic().linvel(Vector::X * 3.0),
        ColliderBuilder::ball(0.1),
    );
    let (velocity, vc) = world.insert(
        RigidBodyBuilder::kinematic_velocity_based()
            .translation(Vector::X * 8.0)
            .linvel(Vector::X * 6.0),
        ColliderBuilder::ball(0.1),
    );
    let (position, pc) = world.insert(
        RigidBodyBuilder::kinematic_position_based().translation(Vector::X * 16.0),
        ColliderBuilder::ball(0.1),
    );
    let mut scan = super::reference::ScanBounds::default();
    let initial = [0.0, 8.0, 16.0, 20.0].map(|x| Vector::X * x);
    assert_matches_scan(&mut world, &mut scan, &initial);
    world.edit_body(position, |body| {
        body.set_next_kinematic_translation(Vector::X * 20.0)
    });
    world.refresh_queries();
    world.step_physics();
    let samples: Vec<_> = [dynamic, velocity, position]
        .map(|body| world.rapier().bodies[body].translation())
        .into_iter()
        .collect();
    for (sample, collider) in samples.iter().zip([dc, vc, pc]) {
        assert_eq!(ray_hit(&world, sample.x, sample.z), Some(collider));
    }
    assert!(samples[0].x > 0.0 && samples[1].x > 8.0);
    assert_eq!(samples[2].x, 20.0);
    assert_matches_scan(&mut world, &mut scan, &samples);
    world.edit_body(dynamic, |body| body.sleep());
    world.refresh_queries();
    world.step_physics();
    assert!(world.rapier().bodies[dynamic].is_sleeping());
    assert_matches_scan(&mut world, &mut scan, &samples);

    // A moving body wakes a sleeping body through contact, without an owner edit.
    let (sleeper, sleeper_collider) = world.insert(
        RigidBodyBuilder::dynamic()
            .translation(Vector::X * 40.0)
            .sleeping(true),
        ColliderBuilder::ball(0.5),
    );
    world.insert(
        RigidBodyBuilder::dynamic()
            .translation(Vector::X * 38.9)
            .linvel(Vector::X * 10.0),
        ColliderBuilder::ball(0.5),
    );
    assert_matches_scan(&mut world, &mut scan, &[Vector::X * 40.0]);
    let mut woke = false;
    for _ in 0..12 {
        world.step_physics();
        let point = world.rapier().bodies[sleeper].translation();
        assert_eq!(ray_hit(&world, point.x, point.z), Some(sleeper_collider));
        assert_matches_scan(&mut world, &mut scan, &[point]);
        woke |= !world.rapier().bodies[sleeper].is_sleeping();
    }
    assert!(woke);
    assert!(world.rapier().bodies[sleeper].translation().x > 40.0);
    assert!(
        world
            .rapier()
            .narrow_phase
            .contact_pairs()
            .any(|pair| pair.has_any_active_contact())
    );
}

#[test]
fn private_controller_impulses_keep_post_step_queries_current() {
    use crate::reference::{
        AnchorId, ItemId, SourceId, WheelFrictionCurve, WheelHubReference, WheelSlotId,
        WheelSlotReference,
    };
    use crate::vehicle::{DriveTorques, InstalledWheel, VehicleBody};
    use rapier3d::control::{
        DynamicRayCastVehicleController, KinematicCharacterController, WheelTuning,
    };
    let mut world = empty_world();
    world.set_gravity(Vector::ZERO);
    world.insert(
        RigidBodyBuilder::fixed().translation(Vector::new(0.0, -0.5, 0.0)),
        ColliderBuilder::cuboid(10.0, 0.5, 10.0),
    );
    let (chassis, chassis_collider) = world.insert(
        RigidBodyBuilder::dynamic().translation(Vector::Y * 0.75),
        ColliderBuilder::cuboid(0.25, 0.1, 0.25),
    );
    let mut controller = DynamicRayCastVehicleController::new(chassis);
    controller.index_forward_axis = 2;
    controller.add_wheel(
        Vector::ZERO,
        -Vector::Y,
        Vector::X,
        0.75,
        0.25,
        &WheelTuning::default(),
    );
    let resting_wheel = controller.wheels()[0];
    let friction = WheelFrictionCurve {
        extremum_slip: 1.0,
        extremum_value: 1.0,
        asymptote_slip: 2.0,
        asymptote_value: 0.5,
        stiffness: 1.0,
    };
    let reference = WheelSlotReference {
        id: WheelSlotId(0),
        source: SourceId {
            file: "fixture".into(),
            path_id: 0,
        },
        anchor: AnchorId(0),
        mount_anchor: AnchorId(0),
        wheel: ItemId(0),
        tire: ItemId(0),
        rim: ItemId(0),
        radius: 0.25,
        suspension_distance: 1.0,
        center: [0.0; 3],
        spring: 100.0,
        damper: 1.0,
        target_position: 0.5,
        steer: false,
        drive: false,
        brake: false,
        handbrake: false,
        forward_friction: friction,
        sideways_friction: friction,
        wheel_mass_kg: 1.0,
        rotational_damping: 0.0,
        force_application_distance_m: 0.0,
        sideways_stiffness_modifier: 1.0,
        hub: WheelHubReference {
            radius_m: 0.1,
            mass_kg: 1.0,
            forward_stiffness: 1.0,
            sideways_stiffness: 1.0,
        },
        tire_condition_range: [0, 100],
        rim_condition_range: [0, 100],
        rim_main_types: Vec::new(),
        rim_types: Vec::new(),
    };
    world.chassis.push(chassis);
    world.vehicles.push(VehicleBody {
        chassis,
        controller,
        resting_wheels: [resting_wheel; 4],
        wheel_references: std::array::from_fn(|_| reference.clone()),
        suspension_lengths: [0.75; 4],
        max_steer: 0.0,
        brake_torque: 0.0,
        handbrake_torque: 0.0,
        active_slots: vec![0],
        installed: [Some(InstalledWheel {
            radius: 0.25,
            condition: 1.0,
        }); 4],
        wheel_rotations: [0.0; 4],
        wheel_rpm: [0.0; 4],
        last_handbrake: 0.0,
    });
    let mut scan = super::reference::ScanBounds::default();
    assert_matches_scan(&mut world, &mut scan, &[Vector::Y * 0.75]);
    let before = world.rapier().bodies[chassis].linvel();
    world.apply_vehicle_forces(
        0,
        crate::simulation::DriveInput::default(),
        DriveTorques::default(),
    );
    assert_ne!(world.rapier().bodies[chassis].linvel(), before);
    assert!(
        world.vehicles[0].controller.wheels()[0]
            .raycast_info()
            .is_in_contact
    );
    world.step_physics();
    let point = world.rapier().bodies[chassis].translation();
    assert_eq!(ray_hit(&world, point.x, point.z), Some(chassis_collider));
    assert_matches_scan(&mut world, &mut scan, &[point]);
    world.apply_vehicle_forces(
        0,
        crate::simulation::DriveInput::default(),
        DriveTorques::default(),
    );
    assert!(
        world.vehicles[0].controller.wheels()[0]
            .raycast_info()
            .is_in_contact
    );

    let mut character_world = empty_world();
    character_world.set_gravity(Vector::ZERO);
    let (pushed, pushed_collider) = character_world.insert(
        RigidBodyBuilder::dynamic()
            .translation(Vector::X * 2.0)
            .sleeping(true),
        ColliderBuilder::ball(0.5),
    );
    let controller = KinematicCharacterController::default();
    let shape = rapier3d::parry::shape::Ball::new(0.5);
    let mut character_scan = super::reference::ScanBounds::default();
    assert_matches_scan(
        &mut character_world,
        &mut character_scan,
        &[Vector::X * 2.0],
    );
    let mut collisions = Vec::new();
    controller.move_shape(
        crate::simulation::FIXED_DT,
        &character_world.queries(QueryFilter::default()),
        &shape,
        &Pose::IDENTITY,
        Vector::X * 3.0,
        |collision| collisions.push(collision),
    );
    assert!(!collisions.is_empty());
    character_world.solve_character_collision_impulses(
        &controller,
        crate::simulation::FIXED_DT,
        QueryFilter::default(),
        &shape,
        80.0,
        &collisions,
    );
    assert!(character_world.rapier().bodies[pushed].linvel().x > 0.0);
    assert!(!character_world.rapier().bodies[pushed].is_sleeping());
    character_world.step_physics();
    let point = character_world.rapier().bodies[pushed].translation();
    assert_eq!(
        ray_hit(&character_world, point.x, point.z),
        Some(pushed_collider)
    );
    assert_matches_scan(&mut character_world, &mut character_scan, &[point]);
}
