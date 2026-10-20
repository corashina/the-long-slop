//! Fixed query union. Physical shapes remain separate, uniquely owned colliders.
use crate::reference::{PlayerPostureReference, PlayerReference};
use rapier3d::{
    math::{Pose, Vector},
    parry::{
        bounding_volume::{Aabb, BoundingSphere, BoundingVolume},
        mass_properties::MassProperties,
        partitioning::{Bvh, BvhBuildStrategy},
        query::{
            PointProjection, PointQuery, Ray, RayCast, RayIntersection, details::NormalConstraints,
        },
        shape::{
            Ball, Capsule, CompositeShape, CompositeShapeRef, FeatureId, Shape, ShapeType,
            TypedCompositeShape, TypedShape,
        },
    },
};

#[derive(Clone)]
pub struct PlayerSweepShape {
    capsule: Capsule,
    capsule_pose: Pose,
    feet: Ball,
    feet_pose: Pose,
    feet_enabled: bool,
    bvh: Bvh,
}
impl PlayerSweepShape {
    pub fn new(reference: &PlayerReference) -> Self {
        Self::from_posture(&reference.posture)
    }
    fn from_posture(reference: &PlayerPostureReference) -> Self {
        let mut shape = Self {
            capsule: Capsule::new_y(0.0, reference.minimum_radius),
            capsule_pose: Pose::IDENTITY,
            feet: Ball::new(reference.minimum_radius),
            feet_pose: Pose::from_translation(Vector::from_array(reference.feet.center)),
            feet_enabled: true,
            bvh: Bvh::default(),
        };
        let lower = Vector::from_array(reference.prone_head.translation);
        let upper = Vector::from_array(reference.standing_head.translation);
        let radius = Vector::splat(reference.maximum_radius);
        let capsule_bounds = Aabb::new(lower.min(upper) - radius, lower.max(upper) + radius);
        shape.update(reference, 1.0);
        let feet_bounds = shape.feet.compute_aabb(&shape.feet_pose);
        shape.bvh = Bvh::from_iter(
            BvhBuildStrategy::Binned,
            [(0, capsule_bounds), (1, feet_bounds)],
        );
        shape
    }
    pub fn set_posture(&mut self, reference: &PlayerReference, value: f32) {
        self.update(&reference.posture, value);
    }
    fn update(&mut self, reference: &PlayerPostureReference, value: f32) {
        let lower = Vector::from_array(reference.prone_head.translation);
        let head = lower.lerp(
            Vector::from_array(reference.standing_head.translation),
            value,
        );
        let radius = reference.minimum_radius
            + (reference.maximum_radius - reference.minimum_radius) * value;
        // Unity clamps capsule height to at least its diameter.
        self.capsule = Capsule::new_y(((head - lower).length() * 0.5 - radius).max(0.0), radius);
        self.capsule_pose = Pose::from_translation((head + lower) * 0.5);
        self.feet.radius = radius;
        self.feet_enabled = value > reference.ctrl_value;
    }
    pub fn feet_enabled(&self) -> bool {
        self.feet_enabled
    }
    pub fn radius(&self) -> f32 {
        self.feet.radius
    }
    pub fn capsule(&self) -> &Capsule {
        &self.capsule
    }
    pub fn capsule_pose(&self) -> Pose {
        self.capsule_pose
    }
    pub(super) fn feet(&self) -> Ball {
        self.feet
    }
    pub(super) fn feet_pose(&self) -> Pose {
        self.feet_pose
    }
}
impl CompositeShape for PlayerSweepShape {
    fn map_part_at(
        &self,
        id: u32,
        f: &mut dyn FnMut(Option<&Pose>, &dyn Shape, Option<&dyn NormalConstraints>),
    ) {
        self.map_untyped_part_at(id, f);
    }
    fn bvh(&self) -> &Bvh {
        &self.bvh
    }
}
impl TypedCompositeShape for PlayerSweepShape {
    type PartShape = dyn Shape;
    type PartNormalConstraints = dyn NormalConstraints;
    fn map_typed_part_at<T>(
        &self,
        id: u32,
        f: impl FnMut(Option<&Pose>, &Self::PartShape, Option<&Self::PartNormalConstraints>) -> T,
    ) -> Option<T> {
        self.map_untyped_part_at(id, f)
    }
    fn map_untyped_part_at<T>(
        &self,
        id: u32,
        mut f: impl FnMut(Option<&Pose>, &dyn Shape, Option<&dyn NormalConstraints>) -> T,
    ) -> Option<T> {
        match id {
            0 => Some(f(Some(&self.capsule_pose), &self.capsule, None)),
            1 if self.feet_enabled => Some(f(Some(&self.feet_pose), &self.feet, None)),
            _ => None,
        }
    }
}
impl RayCast for PlayerSweepShape {
    fn cast_local_ray(&self, ray: &Ray, max: f32, solid: bool) -> Option<f32> {
        CompositeShapeRef(self)
            .cast_local_ray(ray, max, solid)
            .map(|hit| hit.1)
    }
    fn cast_local_ray_and_get_normal(
        &self,
        ray: &Ray,
        max: f32,
        solid: bool,
    ) -> Option<RayIntersection> {
        CompositeShapeRef(self)
            .cast_local_ray_and_get_normal(ray, max, solid)
            .map(|(id, hit)| hit.with_subshape(id))
    }
}
impl PointQuery for PlayerSweepShape {
    fn project_local_point(&self, point: Vector, solid: bool) -> PointProjection {
        CompositeShapeRef(self)
            .project_local_point(point, f32::MAX, solid)
            .map(|(id, projection)| projection.with_subshape(id))
            .unwrap_or(PointProjection::new(false, point))
    }
    fn project_local_point_and_get_feature(&self, point: Vector) -> (PointProjection, FeatureId) {
        CompositeShapeRef(self)
            .project_local_point_and_get_feature(point, f32::MAX)
            .map(|(id, projection, feature)| (projection.with_subshape(id), feature))
            .unwrap_or((PointProjection::new(false, point), FeatureId::Unknown))
    }
}
impl Shape for PlayerSweepShape {
    fn compute_local_aabb(&self) -> Aabb {
        let capsule = self.capsule.compute_aabb(&self.capsule_pose);
        if self.feet_enabled {
            capsule.merged(&self.feet.compute_aabb(&self.feet_pose))
        } else {
            capsule
        }
    }
    fn compute_local_bounding_sphere(&self) -> BoundingSphere {
        self.compute_local_aabb().bounding_sphere()
    }
    fn clone_dyn(&self) -> Box<dyn Shape> {
        Box::new(self.clone())
    }
    fn scale_dyn(&self, _: Vector, _: u32) -> Option<Box<dyn Shape>> {
        None
    }
    fn mass_properties(&self, _: f32) -> MassProperties {
        MassProperties::default()
    }
    fn shape_type(&self) -> ShapeType {
        ShapeType::Custom
    }
    fn as_typed_shape(&self) -> TypedShape<'_> {
        TypedShape::Custom(self)
    }
    fn ccd_thickness(&self) -> f32 {
        self.radius()
    }
    fn ccd_angular_thickness(&self) -> f32 {
        std::f32::consts::PI
    }
    fn as_composite_shape(&self) -> Option<&(dyn CompositeShape + Sync)> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rapier3d::parry::{
        query::{ShapeCastOptions, cast_shapes, intersection_test},
        shape::{Compound, Cuboid, SharedShape},
    };
    use rapier3d::{
        control::KinematicCharacterController,
        prelude::{ColliderBuilder, PhysicsWorld, RigidBodyBuilder},
    };
    fn compound(shape: &PlayerSweepShape) -> Compound {
        let mut parts = vec![(shape.capsule_pose, SharedShape::new(shape.capsule))];
        if shape.feet_enabled {
            parts.push((shape.feet_pose, SharedShape::new(shape.feet)));
        }
        Compound::new(parts)
    }
    #[test]
    fn adapter_queries_match_standard_compounds_at_source_postures() {
        let reference = super::super::posture::test_reference();
        let mut shape = PlayerSweepShape::from_posture(&reference);
        for value in [0., 0.17, 0.3, 0.30001, 0.6, 1.] {
            shape.update(&reference, value);
            assert_eq!(shape.feet_enabled, value > 0.3);
            let standard = compound(&shape);
            assert_eq!(shape.compute_local_aabb(), standard.compute_local_aabb());
            for ray in [
                Ray::new(Vector::new(-2., -0.62, 0.), Vector::X),
                Ray::new(Vector::new(0., 3., 0.), -Vector::Y),
            ] {
                let a = shape.cast_local_ray(&ray, 10., true).unwrap();
                let b = standard.cast_local_ray(&ray, 10., true).unwrap();
                assert!((a - b).abs() < 1e-5);
            }
            for point in [
                Vector::new(1., 0., 0.),
                Vector::new(0., -2., 0.),
                Vector::new(0., 2., 0.),
            ] {
                assert_eq!(
                    shape.project_local_point(point, true).subshape,
                    standard.project_local_point(point, true).subshape
                );
                assert_eq!(
                    shape.project_local_point_and_get_feature(point).0.subshape,
                    standard
                        .project_local_point_and_get_feature(point)
                        .0
                        .subshape
                );
                assert!(
                    (shape.project_local_point(point, true).point
                        - standard.project_local_point(point, true).point)
                        .length()
                        < 1e-5
                );
            }
            for obstacle in [
                Pose::from_translation(Vector::new(1., -0.7, 0.)),
                Pose::from_translation(Vector::new(1., 0.3, 0.)),
            ] {
                let target = Cuboid::new(Vector::splat(0.05));
                let a = cast_shapes(
                    &Pose::IDENTITY,
                    Vector::X,
                    &shape,
                    &obstacle,
                    Vector::ZERO,
                    &target,
                    ShapeCastOptions::default(),
                )
                .unwrap();
                let b = cast_shapes(
                    &Pose::IDENTITY,
                    Vector::X,
                    &standard,
                    &obstacle,
                    Vector::ZERO,
                    &target,
                    ShapeCastOptions::default(),
                )
                .unwrap();
                assert_eq!(a.is_some(), b.is_some());
                if let (Some(a), Some(b)) = (a, b) {
                    assert!((a.time_of_impact - b.time_of_impact).abs() < 1e-5);
                }
            }
        }
    }
    #[test]
    fn foot_only_obstacle_and_threshold_use_the_complete_union() {
        let reference = super::super::posture::test_reference();
        let mut shape = PlayerSweepShape::from_posture(&reference);
        shape.update(&reference, 0.31);
        let obstacle = Pose::from_translation(
            shape.feet_pose.translation - Vector::Y * (shape.radius() - 0.005),
        );
        let ball = Ball::new(0.002);
        assert!(
            intersection_test(&Pose::IDENTITY, &shape, &obstacle, &ball)
                .unwrap()
                .intersecting
        );
        assert!(
            !intersection_test(&shape.capsule_pose, &shape.capsule, &obstacle, &ball)
                .unwrap()
                .intersecting
        );
        shape.update(&reference, 0.3);
        assert!(
            !intersection_test(&Pose::IDENTITY, &shape, &obstacle, &ball)
                .unwrap()
                .intersecting
        );
    }
    #[test]
    fn controller_ground_and_dynamic_contact_match_standard_compound() {
        let mut world = PhysicsWorld::new();
        world.gravity = Vector::ZERO;
        world.insert(
            RigidBodyBuilder::fixed(),
            ColliderBuilder::cuboid(10., 0.1, 10.).translation(Vector::new(0., -0.1, 0.)),
        );
        let (_, dynamic) = world.insert(
            RigidBodyBuilder::dynamic().translation(Vector::new(1., 1., 0.)),
            ColliderBuilder::cuboid(0.1, 1., 1.),
        );
        world.step();
        let reference = super::super::posture::test_reference();
        let mut shape = PlayerSweepShape::from_posture(&reference);
        let controller = KinematicCharacterController::default();
        for value in [0., 0.17, 0.3, 1.] {
            shape.update(&reference, value);
            let standard = compound(&shape);
            let position =
                Pose::from_translation(Vector::Y * (-shape.compute_local_aabb().mins.y + 0.02));
            let movement = Vector::new(1., -0.1, 0.);
            let mut dynamic_hit = false;
            let a = controller.move_shape(
                0.02,
                &world.query_pipeline(),
                &shape,
                &position,
                movement,
                |c| {
                    dynamic_hit |= c.handle == dynamic;
                },
            );
            let b = controller.move_shape(
                0.02,
                &world.query_pipeline(),
                &standard,
                &position,
                movement,
                |_| {},
            );
            assert!((a.translation - b.translation).length() < 1e-4);
            assert!(a.grounded);
            assert_eq!(a.grounded, b.grounded);
            assert!(dynamic_hit);
        }
    }
}
