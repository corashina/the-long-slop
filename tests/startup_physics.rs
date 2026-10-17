use my_summer_slop::{
    reference::{ReferenceManifest, fixed_reservation::fixed_room},
    simulation::{DriveInput, FIXED_DT, FrameInput},
    starting_area::GarageRuntime,
    vehicle::{DriveTorques, from_rapier},
};
use rapier3d::math::{Rotation, Vector};

#[test]
#[ignore = "requires the installed source export"]
fn installed_spawn_alignment_and_settling() {
    let bytes = std::fs::read("web/assets/reference/manifest.json").unwrap();
    let reference = ReferenceManifest::from_runtime_json(&bytes).unwrap();
    let mut run = GarageRuntime::new(&reference).unwrap();
    for _ in 0..10000 {
        run.stream.prepare(100.);
        if run.stream.frame().startup.is_some() {
            break;
        }
        assert!(run.stream.frame().failure.is_none());
    }
    let offset = run.stream.frame().startup.unwrap().player_offset;
    assert!(run.align_startup(&reference).unwrap());
    let player = run.area.player.body_handle();
    let physical = from_rapier(run.physics.rapier().bodies[player].position());
    assert_eq!(run.area.player.body_pose(&run.physics), physical);
    let initial = run.physics.snapshot(0).pose;
    assert!((initial.position[1] - reference.area.car_pose.translation[1] - offset).abs() < 0.0001);
    assert!(run.align_startup(&reference).unwrap());
    assert_eq!(run.physics.snapshot(0).pose, initial);
    assert_eq!(run.area.player.body_pose(&run.physics), physical);

    let mut peak_angular_speed = 0_f32;
    for _ in 0..120 {
        run.area
            .player
            .step(FrameInput::default(), &mut run.physics, FIXED_DT);
        run.area.sync_attachments(&mut run.physics, true);
        run.area.sync_compound(&mut run.physics);
        run.physics.apply_vehicle_forces(
            0,
            DriveInput {
                handbrake: 1.,
                ..Default::default()
            },
            DriveTorques::default(),
        );
        run.physics.step_physics();
        run.area.sync_attachments(&mut run.physics, false);
        run.physics.refresh_queries();
        let snapshot = run.physics.snapshot(0);
        peak_angular_speed =
            peak_angular_speed.max(Vector::from_array(snapshot.angular_velocity).length());
    }
    let settled = run.physics.snapshot(0);
    let up = Rotation::from_array(settled.pose.rotation) * Vector::Y;
    assert!(
        peak_angular_speed < 2.,
        "spawn angular speed: {peak_angular_speed}"
    );
    assert!(up.y > 0.95, "car must settle upright: {settled:?}");
    assert!(settled.wheels_contact.into_iter().all(|contact| contact));
    assert!(run.area.player.grounded());
    let settled_position = run.stream.origin.absolute(settled.pose.position);
    run.sync_world_activity(&reference).unwrap();
    assert_eq!(
        run.persistent_world
            .absolute_vehicle_position(&run.physics, &run.stream.origin, 0),
        settled_position,
        "first activity update must retain the aligned live pose"
    );
    let mut allocation = run.stream.presentation_allocation();
    allocation.prototype_gpu = 600 * 1024 * 1024;
    run.stream.set_presentation_allocation(allocation).unwrap();
    assert!(fixed_room(&run.stream, 1024, 0, 1024, true).unwrap());
    eprintln!(
        "Spawn offset {offset} m; peak car angular speed {peak_angular_speed} rad/s; four wheel contacts."
    );
}
