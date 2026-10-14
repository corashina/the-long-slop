mod common;
use my_summer_slop::{reference::ParticleReference, vehicle::PhysicsWorld, weather::ParticleState};
use rapier3d::math::Vector;
use rapier3d::prelude::*;

fn setup() -> (ParticleReference, PhysicsWorld) {
    let r = common::reference();
    let poses = my_summer_slop::starting_area::initial_vehicle_poses(&r).unwrap();
    let physics = PhysicsWorld::new(&r, &poses).unwrap();
    let mut p = r.conditions.sprays[0].particles[0].clone();
    p.rate_over_time = 10.;
    p.local_pose.translation = [0.; 3];
    p.curves.get_mut("InitialModule.startSpeed").unwrap().scalar = 0.;
    p.curves
        .get_mut("InitialModule.startLifetime")
        .unwrap()
        .scalar = 4.;
    p.scalars.insert("ShapeModule.radius.value".into(), 0.);
    p.curves.get_mut("SizeModule.curve").unwrap().mode = 0;
    p.curves.get_mut("SizeModule.curve").unwrap().scalar = 1.;
    for name in ["VelocityModule.x", "VelocityModule.y", "VelocityModule.z"] {
        let c = p.curves.get_mut(name).unwrap();
        c.mode = 0;
        c.scalar = 0.;
    }
    (p, physics)
}
const ROOT: [f64; 3] = [1000.; 3];

#[test]
fn local_channels_move_particles() {
    let (mut r, physics) = setup();
    r.curves.get_mut("VelocityModule.x").unwrap().scalar = 1.;
    r.curves.get_mut("VelocityModule.y").unwrap().scalar = 2.;
    r.curves.get_mut("VelocityModule.z").unwrap().scalar = 3.;
    for (rotation, deltas) in [
        ([0., 0., 0., 1.], [0.1, 0.2, -0.3]),
        (
            [
                0.,
                std::f32::consts::FRAC_1_SQRT_2,
                0.,
                std::f32::consts::FRAC_1_SQRT_2,
            ],
            [-0.3, 0.2, -0.1],
        ),
    ] {
        r.local_pose.rotation = rotation;
        let mut p = ParticleState::new(&r, 1);
        p.advance(0.1, ROOT, 1., &[], &physics);
        let position = p.particles()[0].absolute_position;
        for (axis, delta) in deltas.into_iter().enumerate() {
            assert!((position[axis] - ROOT[axis] - delta).abs() < 1e-5);
        }
    }
}
#[test]
fn initial_emitter_motion_is_retained_without_new_emission() {
    let (r, physics) = setup();
    let mut p = ParticleState::new(&r, 1);
    p.advance(0.1, ROOT, 0., &[], &physics);
    p.advance(0.1, [1001., 1000., 1000.], 1., &[], &physics);
    assert!((p.particles()[0].absolute_position[0] - 1001.5).abs() < 1e-5);
    p.advance(0.1, [1001., 1000., 1000.], 0., &[], &physics);
    assert!((p.particles()[0].absolute_position[0] - 1002.).abs() < 1e-5);
}

#[test]
fn blocking_surface_uses_mask_bounce_dampen_and_lifetime_loss() {
    for (membership, blocked, sensor) in [
        (1u32, true, false),
        (1 << 8, false, false),
        (1, false, true),
    ] {
        let (mut r, mut physics) = setup();
        r.curves.get_mut("VelocityModule.x").unwrap().scalar = 1.;
        r.curves.get_mut("VelocityModule.z").unwrap().scalar = -1.;
        physics.rapier_mut().colliders.insert(
            ColliderBuilder::cuboid(5., 5., 0.1)
                .translation(Vector::new(1000., 1000., 1000.4))
                .sensor(sensor)
                .collision_groups(InteractionGroups::new(
                    Group::from_bits_retain(membership),
                    Group::ALL,
                    InteractionTestMode::And,
                )),
        );
        physics.refresh_queries();
        let mut p = ParticleState::new(&r, 1);
        p.advance(0.1, ROOT, 1., &[], &physics);
        p.advance(0.4, ROOT, 0., &[], &physics);
        let hit = p.particles()[0];
        if blocked {
            assert!(hit.absolute_position[2] < 1000.3);
            assert!((hit.age - 2.5).abs() < 1e-5);
            p.advance(0.1, ROOT, 0., &[], &physics);
            let bounced = p.particles()[0];
            assert!((bounced.absolute_position[0] - hit.absolute_position[0] - 0.07).abs() < 1e-5);
            assert!((bounced.absolute_position[2] - hit.absolute_position[2] + 0.04).abs() < 1e-5);
        } else {
            assert!((hit.absolute_position[2] - 1000.5).abs() < 1e-5);
            assert!((hit.age - 0.5).abs() < 1e-5);
        }
    }
}
#[test]
fn emitter_reset_and_rebase_preserve_particles_without_false_motion() {
    let (r, physics) = setup();
    for reset in 0..3 {
        let mut p = ParticleState::new(&r, 1);
        p.advance(0.1, ROOT, 1., &[], &physics);
        let original = p.particles()[0];
        match reset {
            0 => p.reset_emitter(), // Pause, drop, and load contact reset.
            1 => p.set_origin([500., 0., 0.]),
            _ => p.advance(0., ROOT, 0., &[], &physics),
        }
        assert_eq!(
            p.particles()[0].absolute_position,
            original.absolute_position
        );
        p.advance(0.1, [1001., 1000., 1000.], 1., &[], &physics);
        assert!((p.particles()[1].absolute_position[0] - 1001.).abs() < 1e-5);
        assert!((p.particles()[0].age - 0.2).abs() < 1e-5);
    }
}
#[test]
fn disabled_motion_modules_need_no_payload() {
    let (mut r, physics) = setup();
    for name in ["VelocityModule", "InheritVelocityModule", "CollisionModule"] {
        r.module_flags.insert(name.into(), false);
        r.curves.retain(|key, _| !key.starts_with(name));
        r.scalars.retain(|key, _| !key.starts_with(name));
        r.flags.retain(|key, _| !key.starts_with(name));
    }
    let mut p = ParticleState::new(&r, 1);
    p.advance(0.1, ROOT, 1., &[], &physics);
    p.advance(0.1, [1001., 1000., 1000.], 0., &[], &physics);
    assert_eq!(p.particles()[0].absolute_position, ROOT);
}

#[test]
fn dynamic_source_layer_colliders_block_and_lifetime_loss_can_kill() {
    let (mut r, mut physics) = setup();
    r.curves.get_mut("VelocityModule.z").unwrap().scalar = -1.;
    r.curves
        .get_mut("CollisionModule.m_EnergyLossOnCollision")
        .unwrap()
        .scalar = 1.;
    let body = physics
        .rapier_mut()
        .bodies
        .insert(RigidBodyBuilder::dynamic().translation(Vector::new(1000., 1000., 1000.4)));
    let world = physics.rapier_mut();
    world.colliders.insert_with_parent(
        ColliderBuilder::cuboid(5., 5., 0.1),
        body,
        &mut world.bodies,
    );
    physics.refresh_queries();
    let mut p = ParticleState::new(&r, 1);
    p.advance(0.1, ROOT, 1., &[], &physics);
    p.advance(0.4, ROOT, 0., &[], &physics);
    assert!(p.particles().is_empty());
}
#[test]
fn authored_emitter_offset_is_retained() {
    let (mut r, physics) = setup();
    r.local_pose.translation = [0.2, 0.3, -0.4];
    let mut p = ParticleState::new(&r, 1);
    p.advance(0.1, ROOT, 1., &[], &physics);
    for (axis, delta) in [0.2, 0.3, -0.4].into_iter().enumerate() {
        assert!((p.particles()[0].absolute_position[axis] - ROOT[axis] - delta).abs() < 1e-5);
    }
}
