mod common;
use my_summer_slop::{reference::ParticleReference, vehicle::PhysicsWorld, weather::ParticleState};

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
