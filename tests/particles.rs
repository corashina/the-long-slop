use the_long_slop::{
    particles::ParticleState, reference::ReferenceManifest, vehicle::PhysicsWorld,
};

#[test]
#[ignore = "requires the installed source export"]
fn cleaning_particles_keep_source_force_acceleration() {
    let bytes = std::fs::read("web/assets/reference/manifest.json").unwrap();
    let reference = ReferenceManifest::from_runtime_json(&bytes).unwrap();
    let mut particle = reference
        .conditions
        .brushes
        .iter()
        .flat_map(|brush| &brush.particles)
        .find(|particle| particle.module_flags.get("ForceModule") == Some(&true))
        .expect("selected cleaning effects use particle forces")
        .clone();
    particle.capacity = 1;
    particle.rate_over_time = 10.;
    particle.rate_over_distance = 0.;
    particle.local_pose.translation = [0.; 3];
    particle.local_pose.rotation = [0., 0., 0., 1.];
    particle.flags.insert("gameObject.active".into(), true);
    particle
        .flags
        .insert("ForceModule.inWorldSpace".into(), true);
    for module in ["VelocityModule", "InheritVelocityModule", "CollisionModule"] {
        particle.module_flags.insert(module.into(), false);
    }
    particle.scalars.remove("EmissionModule.m_BurstCount");
    particle
        .scalars
        .insert("ShapeModule.radius.value".into(), 0.);
    particle
        .scalars
        .insert("ShapeModule.randomPositionAmount".into(), 0.);
    for (name, value) in [
        ("InitialModule.startSpeed", 0.),
        ("InitialModule.startLifetime", 10.),
        ("InitialModule.gravityModifier", 0.),
        ("ForceModule.x", 4.),
        ("ForceModule.y", 0.),
        ("ForceModule.z", 0.),
    ] {
        let curve = particle.curves.get_mut(name).unwrap();
        curve.mode = 0;
        curve.scalar = value;
    }
    let physics = PhysicsWorld::new(&reference);
    let mut state = ParticleState::new(&particle, 1);
    state.advance(0.1, [0.; 3], 1., &physics);
    assert_eq!(state.particles().len(), 1);
    assert!((state.particles()[0].absolute_position[0] - 0.04).abs() < 1e-6);
    state.advance(0.1, [0.; 3], 0., &physics);
    assert!((state.particles()[0].absolute_position[0] - 0.12).abs() < 1e-6);
}
