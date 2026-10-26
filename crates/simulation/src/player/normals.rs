//! Character shape casts with contact-manifold normals.
//!
//! Rapier takes a touching cast's normal from GJK closest points. That normal degrades against
//! large boxes and at far coordinates, so a flat floor near 2000 units can read as a wall.
use std::cell::RefCell;

use rapier3d::math::{Pose, Real, Vector};
use rapier3d::parry::query::{
    ClosestPoints, Contact, ContactManifold, DefaultQueryDispatcher, NonlinearRigidMotion,
    PersistentQueryDispatcher, QueryDispatcher, ShapeCastHit, ShapeCastOptions, ShapeDistance,
    ShapeIntersection, Unsupported, details,
};
use rapier3d::parry::shape::Shape;

/// Parry treats a cast shorter than this as starting in contact.
const TOUCHING_TIME_OF_IMPACT: Real = 1.0e-4;
/// Manifold margin beyond the cast target distance.
const MANIFOLD_MARGIN: Real = 1.0e-3;

thread_local! {
    static MANIFOLD: RefCell<ContactManifold<(), ()>> = RefCell::new(ContactManifold::new());
}

pub struct CharacterQueryDispatcher;

impl QueryDispatcher for CharacterQueryDispatcher {
    fn intersection_test(
        &self,
        pos12: &Pose,
        g1: &dyn Shape,
        g2: &dyn Shape,
    ) -> Result<ShapeIntersection, Unsupported> {
        DefaultQueryDispatcher.intersection_test(pos12, g1, g2)
    }
    fn distance(
        &self,
        pos12: &Pose,
        g1: &dyn Shape,
        g2: &dyn Shape,
    ) -> Result<ShapeDistance, Unsupported> {
        DefaultQueryDispatcher.distance(pos12, g1, g2)
    }
    fn contact(
        &self,
        pos12: &Pose,
        g1: &dyn Shape,
        g2: &dyn Shape,
        prediction: Real,
    ) -> Result<Option<Contact>, Unsupported> {
        DefaultQueryDispatcher.contact(pos12, g1, g2, prediction)
    }
    fn closest_points(
        &self,
        pos12: &Pose,
        g1: &dyn Shape,
        g2: &dyn Shape,
        max_dist: Real,
    ) -> Result<ClosestPoints, Unsupported> {
        DefaultQueryDispatcher.closest_points(pos12, g1, g2, max_dist)
    }
    fn cast_shapes_nonlinear(
        &self,
        motion1: &NonlinearRigidMotion,
        g1: &dyn Shape,
        motion2: &NonlinearRigidMotion,
        g2: &dyn Shape,
        start_time: Real,
        end_time: Real,
        stop_at_penetration: bool,
    ) -> Result<Option<ShapeCastHit>, Unsupported> {
        DefaultQueryDispatcher.cast_shapes_nonlinear(
            motion1,
            g1,
            motion2,
            g2,
            start_time,
            end_time,
            stop_at_penetration,
        )
    }
    fn cast_shapes(
        &self,
        pos12: &Pose,
        vel12: Vector,
        g1: &dyn Shape,
        g2: &dyn Shape,
        options: ShapeCastOptions,
    ) -> Result<Option<ShapeCastHit>, Unsupported> {
        // Split composites here so their parts dispatch back through this type.
        if let Some(c1) = g1.as_composite_shape() {
            return Ok(details::cast_shapes_composite_shape_shape(
                self, pos12, vel12, c1, g2, options,
            ));
        }
        if let Some(h2) = g2.as_heightfield() {
            return details::cast_shapes_shape_heightfield(self, pos12, vel12, g1, h2, options);
        }
        if let Some(c2) = g2.as_composite_shape() {
            return Ok(details::cast_shapes_shape_composite_shape(
                self, pos12, vel12, g1, c2, options,
            ));
        }
        // Keep touching hits so the separation test below uses the corrected normal.
        let Some(mut hit) = DefaultQueryDispatcher.cast_shapes(
            pos12,
            vel12,
            g1,
            g2,
            ShapeCastOptions {
                stop_at_penetration: true,
                ..options
            },
        )?
        else {
            return Ok(None);
        };
        let at_impact = Pose::from_translation(-vel12 * hit.time_of_impact) * *pos12;
        MANIFOLD.with_borrow_mut(|manifold| {
            manifold.clear();
            // Composites are split above, so the remaining pairs are convex.
            if DefaultQueryDispatcher
                .contact_manifold_convex_convex(
                    &at_impact,
                    g1,
                    g2,
                    None,
                    None,
                    options.target_distance + MANIFOLD_MARGIN,
                    manifold,
                )
                .is_ok()
                && !manifold.points.is_empty()
            {
                hit.normal1 = manifold.local_n1;
                hit.normal2 = manifold.local_n2;
            }
        });
        let touching = hit.time_of_impact < TOUCHING_TIME_OF_IMPACT;
        if touching && !options.stop_at_penetration && hit.normal1.dot(vel12) >= 0.0 {
            return Ok(None);
        }
        Ok(Some(hit))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rapier3d::control::KinematicCharacterController;
    use rapier3d::prelude::*;

    /// Walks a capsule with a feet ball, like the player sweep, and returns every hit normal.
    fn walk(
        base: Vector,
        obstacle: Option<Collider>,
        direction: Vector,
        fixed: bool,
    ) -> Vec<Vector> {
        let mut bodies = RigidBodySet::new();
        let mut colliders = ColliderSet::new();
        colliders
            .insert(ColliderBuilder::cuboid(200., 0.1, 200.).translation(base - Vector::Y * 0.1));
        if let Some(mut obstacle) = obstacle {
            obstacle.set_translation(base + obstacle.translation());
            colliders.insert(obstacle);
        }
        let mut broad_phase = BroadPhaseBvh::new();
        let mut narrow_phase = NarrowPhase::new();
        PhysicsPipeline::new().step(
            Vector::Y * -9.81,
            &IntegrationParameters::default(),
            &mut IslandManager::new(),
            &mut broad_phase,
            &mut narrow_phase,
            &mut bodies,
            &mut colliders,
            &mut ImpulseJointSet::new(),
            &mut MultibodyJointSet::new(),
            &mut SoftBodySet::new(),
            &mut CCDSolver::new(),
            &(),
            &(),
        );
        let shape = SharedShape::compound(vec![
            (
                Pose::from_translation(Vector::Y * 0.1),
                SharedShape::capsule_y(0.25, 0.35),
            ),
            (
                Pose::from_translation(Vector::Y * -0.55),
                SharedShape::ball(0.35),
            ),
        ]);
        let controller = KinematicCharacterController::default();
        let mut pose = Pose::from_translation(base + Vector::Y * 0.905);
        let mut vertical = 0.0;
        let dt = 1. / 60.;
        let mut normals = Vec::new();
        for _ in 0..600 {
            vertical = (vertical - 9.81 * dt).max(-50.);
            let mut queries = broad_phase.as_query_pipeline(
                narrow_phase.query_dispatcher(),
                &bodies,
                &colliders,
                QueryFilter::default(),
            );
            if fixed {
                queries.dispatcher = &CharacterQueryDispatcher;
            }
            let desired = direction * 4. * dt + Vector::Y * vertical * dt;
            let movement = controller.move_shape(dt, &queries, &*shape, &pose, desired, |hit| {
                normals.push(hit.hit.normal1)
            });
            pose.translation += movement.translation;
            if movement.grounded {
                vertical = 0.;
            }
        }
        normals
    }

    /// Normals whose height matches none of the surfaces. Ramp edges blend slightly.
    fn stray(normals: &[Vector], allowed: &[f32]) -> Vec<Vector> {
        normals
            .iter()
            .copied()
            .filter(|normal| allowed.iter().all(|y| (normal.y - y).abs() >= 5.0e-3))
            .collect()
    }

    const BASES: [f32; 5] = [0., 500., 1000., 2000., 2040.];

    #[test]
    fn flat_floor_stays_flat_far_from_origin() {
        let direction = Vector::new(0.6, 0., -1.).normalize();
        assert!(
            BASES.iter().any(|&b| !stray(
                &walk(Vector::new(b, 0., b), None, direction, false),
                &[1.]
            )
            .is_empty()),
            "Default dispatcher no longer shows the far-coordinate normal error"
        );
        for b in BASES {
            let normals = walk(Vector::new(b, 0., b), None, direction, true);
            assert!(!normals.is_empty());
            assert_eq!(stray(&normals, &[1.]), [], "base {b}");
        }
    }

    #[test]
    fn ramp_and_wall_normals_far_from_origin() {
        let ramp_y = 25f32.to_radians().cos();
        for b in BASES {
            let ramp = ColliderBuilder::cuboid(3., 0.1, 30.)
                .rotation(Vector::X * 25f32.to_radians())
                .translation(Vector::Z * -20.)
                .build();
            let normals = walk(Vector::new(b, 0., b), Some(ramp), -Vector::Z, true);
            assert!(normals.iter().any(|n| (n.y - ramp_y).abs() < 1.0e-3));
            // The crease where the ramp meets the floor gives normals between the two faces.
            let steep: Vec<_> = normals.iter().filter(|n| n.y < ramp_y - 5.0e-3).collect();
            assert_eq!(steep, [] as [&Vector; 0], "ramp base {b}");

            let wall = ColliderBuilder::cuboid(50., 3., 0.1)
                .translation(Vector::Z * -10.)
                .build();
            let direction = Vector::new(0.6, 0., -1.).normalize();
            let normals = walk(Vector::new(b, 0., b), Some(wall), direction, true);
            assert!(normals.iter().any(|n| n.y.abs() < 1.0e-3));
            assert_eq!(stray(&normals, &[1., 0.]), [], "wall base {b}");
        }
    }
}
