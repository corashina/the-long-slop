use super::*;
use rapier3d::math::{Rotation, Vector};
use rapier3d::parry::bounding_volume::BoundingVolume;
use rapier3d::prelude::{ColliderBuilder, QueryFilter, QueryPipeline, Ray, RigidBodyBuilder};

fn hit(bounds: &QueryBounds, physics: &PhysicsWorld, x: f32, z: f32) -> Option<ColliderHandle> {
    QueryPipeline {
        dispatcher: physics.narrow_phase.query_dispatcher(),
        bvh: &bounds.bvh,
        bodies: &physics.bodies,
        colliders: &physics.colliders,
        filter: QueryFilter::default(),
    }
    .cast_ray(&Ray::new(Vector::new(x, 10.0, z), -Vector::Y), 20.0, true)
    .map(|hit| hit.0)
}

#[test]
fn direct_mutations_refresh_hits_without_rebuilding_unchanged_bounds() {
    let mut physics = PhysicsWorld::new();
    let mut bounds = QueryBounds::default();
    let collider = physics.colliders.insert(ColliderBuilder::ball(1.0));
    assert_eq!(bounds.refresh(&mut physics), 1);
    assert_eq!(hit(&bounds, &physics, 0.0, 0.0), Some(collider));
    let capacities = (
        bounds.states.capacity(),
        bounds.changed.capacity(),
        bounds.retained_bytes(),
    );
    for _ in 0..8 {
        assert_eq!(bounds.refresh(&mut physics), 0);
    }
    assert_eq!(
        capacities,
        (
            bounds.states.capacity(),
            bounds.changed.capacity(),
            bounds.retained_bytes()
        )
    );

    physics.colliders[collider].set_translation(Vector::new(5.0, 0.0, 0.0));
    assert_eq!(bounds.refresh(&mut physics), 1);
    assert_eq!(hit(&bounds, &physics, 0.0, 0.0), None);
    assert_eq!(hit(&bounds, &physics, 5.0, 0.0), Some(collider));
    physics.colliders[collider].set_shape(SharedShape::ball(2.0));
    assert_eq!(bounds.refresh(&mut physics), 1);
    assert_eq!(hit(&bounds, &physics, 6.5, 0.0), Some(collider));
    physics.colliders[collider]
        .shape_mut()
        .as_ball_mut()
        .unwrap()
        .radius = 3.0;
    assert_eq!(bounds.refresh(&mut physics), 1);
    assert_eq!(hit(&bounds, &physics, 7.5, 0.0), Some(collider));
    physics.colliders[collider].set_enabled(false);
    assert_eq!(bounds.refresh(&mut physics), 0);
    assert_eq!(hit(&bounds, &physics, 5.0, 0.0), None);
    physics.colliders[collider].set_enabled(true);
    assert_eq!(bounds.refresh(&mut physics), 1);
    assert_eq!(hit(&bounds, &physics, 5.0, 0.0), Some(collider));

    let retired_shape = Arc::downgrade(&physics.colliders[collider].shared_shape().0);
    physics.remove_collider(collider);
    let replacement = physics
        .colliders
        .insert(ColliderBuilder::ball(0.5).translation(Vector::new(-5.0, 0.0, 0.0)));
    assert_eq!(collider.into_raw_parts().0, replacement.into_raw_parts().0);
    assert_ne!(collider, replacement);
    assert_eq!(bounds.refresh(&mut physics), 1);
    assert!(retired_shape.upgrade().is_none());
    assert_eq!(hit(&bounds, &physics, 5.0, 0.0), None);
    assert_eq!(hit(&bounds, &physics, -5.0, 0.0), Some(replacement));
    physics.remove_collider(replacement);
    assert_eq!(bounds.refresh(&mut physics), 0);
    assert!(bounds.bvh.is_empty());
    assert!(bounds.states.iter().all(Option::is_none));
}

#[test]
fn body_attachment_and_enablement_changes_keep_queries_current() {
    let mut physics = PhysicsWorld::new();
    physics.gravity = Vector::ZERO;
    let (body, collider) = physics.insert(
        RigidBodyBuilder::dynamic(),
        ColliderBuilder::cuboid(2.0, 0.5, 0.25),
    );
    let other =
        physics.insert_body(RigidBodyBuilder::fixed().translation(Vector::new(-8.0, 0.0, 0.0)));
    let mut bounds = QueryBounds::default();
    bounds.refresh(&mut physics);
    physics.step();
    physics.bodies[body].set_translation(Vector::new(5.0, 0.0, 0.0), true);
    assert_eq!(bounds.refresh(&mut physics), 1);
    assert_eq!(hit(&bounds, &physics, 5.0, 0.0), Some(collider));
    physics.bodies[body].set_rotation(Rotation::from_rotation_y(std::f32::consts::FRAC_PI_2), true);
    bounds.refresh(&mut physics);
    assert_eq!(hit(&bounds, &physics, 5.0, 1.5), Some(collider));
    physics.step();
    physics.colliders[collider].set_translation_wrt_parent(Vector::new(3.0, 0.0, 0.0));
    assert_eq!(bounds.refresh(&mut physics), 1);
    assert_eq!(hit(&bounds, &physics, 5.0, -3.0), Some(collider));
    physics
        .colliders
        .set_parent(collider, Some(other), &mut physics.bodies);
    bounds.refresh(&mut physics);
    assert_eq!(hit(&bounds, &physics, -5.0, 0.0), Some(collider));
    assert_eq!(hit(&bounds, &physics, 5.0, -3.0), None);
    physics.bodies[other].set_enabled(false);
    bounds.refresh(&mut physics);
    assert_eq!(hit(&bounds, &physics, -5.0, 0.0), None);
    physics.step();
    physics.bodies[other].set_enabled(true);
    physics.step();
    bounds.refresh(&mut physics);
    assert_eq!(hit(&bounds, &physics, -5.0, 0.0), Some(collider));
    physics
        .colliders
        .set_parent(collider, None, &mut physics.bodies);
    bounds.refresh(&mut physics);
    physics.colliders[collider].set_translation(Vector::new(-12.0, 0.0, 0.0));
    bounds.refresh(&mut physics);
    assert_eq!(hit(&bounds, &physics, -12.0, 0.0), Some(collider));
    assert_eq!(bounds.refresh(&mut physics), 0);
}

#[test]
fn query_refresh_keeps_new_bodies_and_contacts_in_the_physics_pipeline() {
    let mut physics = PhysicsWorld::new();
    physics.insert(
        RigidBodyBuilder::fixed().translation(Vector::new(0.0, -0.5, 0.0)),
        ColliderBuilder::cuboid(10.0, 0.5, 10.0),
    );
    let (body, collider) = physics.insert(
        RigidBodyBuilder::dynamic().translation(Vector::new(0.0, 3.0, 0.0)),
        ColliderBuilder::ball(0.5),
    );
    let mut bounds = QueryBounds::default();
    bounds.refresh(&mut physics);
    for _ in 0..240 {
        bounds.refresh(&mut physics);
        physics.step();
        bounds.refresh(&mut physics);
    }
    assert!((physics.bodies[body].translation().y - 0.5).abs() < 0.05);
    assert_eq!(hit(&bounds, &physics, 0.0, 0.0), Some(collider));
    assert!(
        physics
            .narrow_phase
            .contact_pairs()
            .any(|pair| pair.has_any_active_contact())
    );
    physics.bodies[body].set_translation(Vector::new(0.0, 5.0, 0.0), true);
    bounds.refresh(&mut physics);
    physics.step();
    assert!(physics.bodies[body].translation().y < 5.0);
}

#[test]
fn public_queries_follow_step_without_an_extra_caller_refresh() {
    let mut world = super::super::PhysicsWorld {
        inactive_bodies: Default::default(),
        physics: PhysicsWorld::new(),
        query_bounds: QueryBounds::default(),
        chassis: Vec::new(),
        vehicles: Vec::new(),
    };
    world.rapier_mut().gravity = Vector::ZERO;
    let (body, collider) = world.rapier_mut().insert(
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
    assert_eq!(
        world
            .queries_mut(QueryFilter::default())
            .as_ref()
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
    crate::vehicle::PhysicsWorld {
        inactive_bodies: Default::default(),
        physics: PhysicsWorld::new(),
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
    let first = world
        .rapier_mut()
        .colliders
        .insert(ColliderBuilder::ball(1.0));
    let second = world
        .rapier_mut()
        .colliders
        .insert(ColliderBuilder::ball(1.0).translation(Vector::new(8.0, 0.0, 0.0)));
    let (body, _) = world.rapier_mut().insert(
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

    world.rapier_mut().colliders[first].set_translation(Vector::new(4.0, 0.0, 0.0));
    world.rapier_mut().colliders[second].set_translation(Vector::new(12.0, 0.0, 0.0));
    world.rapier_mut().bodies[body].set_translation(Vector::new(-4.0, 0.0, 0.0), true);
    assert_matches_scan(&mut world, &mut scan, &samples);

    world.rapier_mut().colliders[first].set_enabled(false);
    world.rapier_mut().bodies[body].set_enabled(false);
    assert_matches_scan(&mut world, &mut scan, &samples);

    world.rapier_mut().remove_collider(first);
    let replacement = world
        .rapier_mut()
        .colliders
        .insert(ColliderBuilder::ball(0.5).translation(Vector::new(-12.0, 0.0, 0.0)));
    assert_eq!(first.into_raw_parts().0, replacement.into_raw_parts().0);
    assert_ne!(first, replacement);
    assert_matches_scan(&mut world, &mut scan, &samples);
}
