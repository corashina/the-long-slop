use rapier3d::math::Pose;
use rapier3d::parry::partitioning::Bvh;
use rapier3d::prelude::{ColliderHandle, PhysicsWorld, RigidBodyHandle, SharedShape};
use std::sync::Arc;

#[derive(Default)]
pub(super) struct QueryBounds {
    pub bvh: Bvh,
    states: Vec<Option<ColliderState>>,
    changed: Vec<(ColliderHandle, Pose)>,
}

struct ColliderState {
    handle: ColliderHandle,
    position: Pose,
    parent: Option<(RigidBodyHandle, Pose)>,
    enabled: bool,
    // Owning the prior shape forces shape_mut to copy before changing its geometry.
    shape: SharedShape,
}

impl QueryBounds {
    pub fn retained_bytes(&self) -> usize {
        self.bvh.heap_memory_size()
            + self.states.capacity() * std::mem::size_of::<Option<ColliderState>>()
            + self.changed.capacity() * std::mem::size_of::<(ColliderHandle, Pose)>()
    }

    pub fn refresh(&mut self, physics: &mut PhysicsWorld) -> usize {
        // Keep body, island and collision notifications for PhysicsPipeline::step.
        physics
            .bodies
            .propagate_modified_body_positions_to_colliders(&mut physics.colliders);
        self.changed.clear();
        // Remove old generations before a new collider can reuse the same tree leaf.
        for state in &mut self.states {
            if state
                .as_ref()
                .is_some_and(|state| !physics.colliders.contains(state.handle))
            {
                self.bvh
                    .remove(state.as_ref().unwrap().handle.into_raw_parts().0);
                *state = None;
            }
        }
        for (handle, collider) in physics.colliders.iter() {
            let slot = handle.into_raw_parts().0 as usize;
            let previous = self.states.get(slot).and_then(Option::as_ref);
            let parent = collider
                .parent()
                .zip(collider.position_wrt_parent().copied());
            let enabled = collider.is_enabled()
                && parent.is_none_or(|(handle, _)| physics.bodies[handle].is_enabled());
            // A local attachment edit does not move its body. Synchronize that edit here.
            let position = if previous.is_none_or(|state| state.parent != parent) {
                parent.map_or(*collider.position(), |(handle, local)| {
                    *physics.bodies[handle].position() * local
                })
            } else {
                *collider.position()
            };
            if previous.is_none_or(|state| {
                state.handle != handle
                    || state.position != position
                    || state.parent != parent
                    || state.enabled != enabled
                    || !Arc::ptr_eq(&state.shape.0, &collider.shared_shape().0)
            }) {
                self.changed.push((handle, position));
            }
        }
        let mut updates = 0;
        for &(handle, position) in &self.changed {
            if physics.colliders[handle].position() != &position {
                physics.colliders[handle].set_position(position);
            }
            let collider = &physics.colliders[handle];
            let parent = collider
                .parent()
                .zip(collider.position_wrt_parent().copied());
            let enabled = collider.is_enabled()
                && parent.is_none_or(|(handle, _)| physics.bodies[handle].is_enabled());
            let slot = handle.into_raw_parts().0 as usize;
            if enabled {
                self.bvh.reinsert_or_update_with_change_detection(
                    collider.compute_aabb(),
                    slot as u32,
                    0.0,
                );
                updates += 1;
            } else {
                self.bvh.remove(slot as u32);
            }
            if self.states.len() <= slot {
                self.states.resize_with(slot + 1, || None);
            }
            self.states[slot] = Some(ColliderState {
                handle,
                position,
                parent,
                enabled,
                shape: collider.shared_shape().clone(),
            });
        }
        updates
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rapier3d::math::{Rotation, Vector};
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
        physics.bodies[body]
            .set_rotation(Rotation::from_rotation_y(std::f32::consts::FRAC_PI_2), true);
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
}
