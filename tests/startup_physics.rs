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
    use my_summer_slop::world::{RequiredView, WorldStream};
    let camera = [
        [1., 0., 0., 1e8],
        [-1., 0., 0., 1e8],
        [0., 1., 0., 1e8],
        [0., -1., 0., 1e8],
        [0., 0., 1., 1e8],
        [0., 0., -1., 1e8],
    ];
    let views = [RequiredView {
        camera,
        shadow: my_summer_slop::reference::shadow_candidates::DirectionalHull::new(
            camera,
            [1., 0., 0.],
        )
        .unwrap(),
        mask: u32::MAX,
        light_mask: u32::MAX,
        origin: [0.; 3],
    }];
    let mut replay = WorldStream::new(&reference, 42).unwrap();
    replay.request_at([0.; 3], [0.; 3]);
    for _ in 0..300 {
        replay.advance_generation();
        replay.prepare(0.);
        replay.select_required(&views);
        if replay.frame().startup.is_some() && !replay.required_content().roads.is_empty() {
            break;
        }
        assert!(
            replay.frame().failure.is_none(),
            "{:?}",
            replay.frame().failure
        );
    }
    assert!(
        replay.frame().startup.is_some(),
        "ordinary generation must not reset startup replay"
    );
    assert!(
        !replay.required_content().roads.is_empty(),
        "selected replay must advance under ordinary generation"
    );
    assert!(replay.coverage().road_end < 400);

    assert!(
        !run.stream.required_ready(),
        "empty unresolved startup is not ready"
    );
    let start_pose = run.physics.snapshot(0).pose;
    assert!(run.stream.recovery_target([0.; 3], true).unwrap().is_none());
    assert_eq!(run.physics.snapshot(0).pose, start_pose);
    assert!(run.stream.frame().failure.is_none());
    assert!(!run.stream.supports(my_summer_slop::world::WorldBounds {
        min: [-8.; 3],
        max: [8.; 3]
    }));
    for _ in 0..10000 {
        run.stream.prepare(100.);
        run.stream.select_required(&views);
        if run.stream.frame().startup.is_some() {
            break;
        }
        assert!(run.stream.frame().failure.is_none());
    }
    assert_eq!(run.stream.coverage().road_end, 100);
    assert!(!run.stream.required_ready());
    let before = run.stream.coverage();
    for _ in 0..8 {
        run.stream.prepare(100.);
        run.stream.select_required(&views);
    }
    assert_eq!(
        run.stream.coverage(),
        before,
        "prepare must not advance generation"
    );
    let revision = run.stream.frame().request.revision;
    run.stream.advance_generation();
    assert_eq!(run.stream.coverage().road_end, 101);
    assert_eq!(
        run.stream.frame().request.revision,
        revision,
        "irrelevant growth must preserve selection"
    );
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
    let settle_seconds = 2.0_f32;
    let settle_steps = (settle_seconds / FIXED_DT).round() as usize;
    for _ in 0..settle_steps {
        run.area.player.step(
            FrameInput::default(),
            Default::default(),
            &mut run.physics,
            FIXED_DT,
        );
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
    let mut roads = my_summer_slop::world::RoadIndex::new(
        my_summer_slop::world::RoadSequence::new(&reference.world, 42).unwrap(),
        my_summer_slop::world::terrain_evaluator(&reference.world, 42),
    );
    roads.prepare(102);
    let frontier = roads.segment(101).unwrap().position();
    run.stream.request_at(frontier, frontier);
    let revision = run.stream.frame().request.revision;
    run.stream.advance_generation();
    assert_eq!(run.stream.coverage().road_end, 102);
    assert!(
        run.stream.frame().request.revision > revision,
        "stationary growth must select new local identities"
    );
    assert!(!run.stream.required_ready());
    for _ in 0..200 {
        run.stream.prepare(100.);
        run.stream.select_required(&views);
        if run.stream.required_content().roads.contains(&101) {
            break;
        }
        assert!(
            run.stream.frame().failure.is_none(),
            "{:?}",
            run.stream.frame().failure
        );
    }
    assert!(run.stream.required_content().roads.contains(&101));
    assert_eq!(run.stream.coverage().road_end, 102);
    eprintln!(
        "Spawn offset {offset} m; peak car angular speed {peak_angular_speed} rad/s; four wheel contacts."
    );
}
