mod common;
use my_summer_slop::mirrors::{MirrorFrame, MirrorInput, MirrorState};
use my_summer_slop::reference::{MirrorRecord, ReferenceManifest};
use rapier3d::math::{Rotation, Vector};

fn fixture() -> MirrorRecord {
    ReferenceManifest::from_json(include_bytes!("common/reference.json"))
        .unwrap()
        .mirrors
}

fn frame(head: [f32; 3]) -> MirrorFrame {
    MirrorFrame {
        player_position: [0.0, 0.0, 0.0],
        view_head_position: head,
        source_enabled: true,
        pointer_locked: true,
    }
}

fn input(z: f32) -> MirrorInput {
    MirrorInput {
        plane_position: [0.0, 0.0, z],
        plane_normal: [0.0, 1.0, 0.0],
        camera_up: [0.0, 0.0, -1.0],
        camera_position: [0.0, 0.0, 0.0],
        normal_view_visible: true,
    }
}

#[test]
fn source_projection_and_far_order() {
    let mut source = fixture();
    source.rules.fov_bounds_degrees = [10.0, 55.0];
    source.rules.distance_bounds_meters = [0.0, 4.0];
    source.rules.draw_distance_meters = 50.0;
    source.mirrors.truncate(1);
    source.mirrors[0].initial_far_plane = 1500.0;
    let mut state = MirrorState::new(&source);
    state.update(&source.rules, frame([0.0, 2.0, 0.0]), &[input(2.0)]);
    let first = &state.outputs()[0];
    assert!(first.capture);
    assert!((first.fov_degrees - 32.5).abs() < 0.001);
    let forward = Rotation::from_array(first.pose.rotation) * Vector::NEG_Z;
    let expected = Vector::new(0.0, 2.0, 4.0).normalize();
    assert!(forward.dot(expected) > 0.9999);
    assert!((first.capture_far_plane - 1500.0).abs() < 0.001);
    assert!((first.next_far_plane - 25.0).abs() < 0.001);
    state.update(&source.rules, frame([0.0, 2.0, 0.0]), &[input(2.0)]);
    assert!((state.outputs()[0].capture_far_plane - 25.0).abs() < 0.001);
}

#[test]
fn source_rank_cutoff_and_pause() {
    let mut source = fixture();
    while source.mirrors.len() < 6 {
        source.mirrors.push(source.mirrors[0].clone());
    }
    source.rules.distance_bounds_meters = [0.0, 4.0];
    let mut inputs = [
        input(0.5),
        input(1.0),
        input(1.0),
        input(2.0),
        input(3.0),
        input(3.5),
    ];
    inputs[0].normal_view_visible = false;
    inputs[3].plane_normal = [0.0, -1.0, 0.0];
    let mut state = MirrorState::new(&source);
    state.update(&source.rules, frame([0.0, 2.0, 0.0]), &inputs);
    let result = state.outputs();
    assert_eq!(
        result.iter().map(|o| o.rank).collect::<Vec<_>>(),
        [0, 1, 2, 3, 4, 5]
    );
    assert!(!result[0].capture);
    assert!(result[1].capture && result[2].capture);
    assert!(!result[3].capture);
    assert!(result[4].capture);
    assert!(!result[5].capture);

    inputs[1].plane_position[2] = 1.5;
    state.update(&source.rules, frame([0.0, 2.0, 0.0]), &inputs);
    inputs[1].plane_position[2] = 1.0;
    state.update(&source.rules, frame([0.0, 2.0, 0.0]), &inputs);
    assert!(state.outputs()[2].rank < state.outputs()[1].rank);
    inputs[4].plane_position = [0.0, 0.0, 5.0];
    state.update(&source.rules, frame([0.0, 2.0, 0.0]), &inputs);
    assert!(!state.outputs()[4].capture);
    let mut paused = frame([0.0, 2.0, 0.0]);
    paused.pointer_locked = false;
    state.update(&source.rules, paused, &inputs);
    assert!(state.outputs().iter().all(|o| !o.capture));
}

#[test]
fn coincident_view_is_safe() {
    let mut source = fixture();
    source.mirrors.truncate(1);
    let mut state = MirrorState::new(&source);
    let mut coincident = input(0.0);
    coincident.plane_position = [0.0, 2.0, 0.0];
    coincident.camera_position = coincident.plane_position;
    state.update(
        &source.rules,
        frame(coincident.plane_position),
        &[coincident],
    );
    let output = &state.outputs()[0];
    assert!(!output.capture);
    assert!(output.fov_degrees.is_finite());
    assert!(output.capture_near_plane.is_finite());
    assert!(output.capture_far_plane.is_finite());
    assert!(output.next_far_plane.is_finite());
    assert!(output.pose.rotation.iter().all(|v| v.is_finite()));
}

#[test]
fn translated_scene_preserves_capture() {
    let mut source = fixture();
    source.mirrors.truncate(1);
    let mut state = MirrorState::new(&source);
    state.update(&source.rules, frame([0.0, 2.0, 0.0]), &[input(2.0)]);
    let original = state.outputs()[0];
    state.reset(&source);
    let shift = [100_000.0, -100_000.0, 50_000.0];
    let mut moved_frame = frame([shift[0], shift[1] + 2.0, shift[2]]);
    moved_frame.player_position = shift;
    let mut moved_input = input(2.0);
    moved_input.plane_position = [shift[0], shift[1], shift[2] + 2.0];
    moved_input.camera_position = shift;
    state.update(&source.rules, moved_frame, &[moved_input]);
    let moved = state.outputs()[0];
    assert!(moved.capture);
    assert!((moved.fov_degrees - original.fov_degrees).abs() < 0.01);
    assert!((moved.next_far_plane - original.next_far_plane).abs() < 0.01);
    let dot: f32 = moved
        .pose
        .rotation
        .iter()
        .zip(original.pose.rotation)
        .map(|(a, b)| a * b)
        .sum();
    assert!(dot.abs() > 0.9999, "{dot}");
}

#[test]
fn signed_scale_keeps_transform_axes_rotation_only() {
    let mut source = fixture();
    source.mirrors.truncate(1);
    let mut state = MirrorState::new(&source);

    let parent_rotation = Rotation::from_rotation_y(0.37) * Rotation::from_rotation_x(-0.21);
    let local_rotation = Rotation::from_rotation_y(0.71) * Rotation::from_rotation_z(0.43);
    let plane_rotation = parent_rotation * local_rotation;
    let signed_scale = Vector::new(-2.0, 1.2, 0.7);
    let parent_position = Vector::new(9.0, -3.0, 7.0);
    let local_position = Vector::new(0.4, 0.2, -0.8);
    let plane = parent_position + parent_rotation * (signed_scale * local_position);
    let normal = plane_rotation * Vector::Y;
    let camera_up = plane_rotation * Vector::NEG_Z;
    let scaled_normal = parent_rotation * (signed_scale * (local_rotation * Vector::Y));
    assert!(normal.dot(scaled_normal.normalize()) < 0.99);

    let player = plane + normal * 1.5;
    let head = plane + normal * 2.0 + camera_up * 0.4;
    let camera_local = local_position + local_rotation * Vector::new(0.1, 0.2, 0.3);
    let camera = parent_position + parent_rotation * (signed_scale * camera_local);
    let input = MirrorInput {
        plane_position: plane.to_array(),
        plane_normal: normal.to_array(),
        camera_up: camera_up.to_array(),
        camera_position: camera.to_array(),
        normal_view_visible: true,
    };
    state.update(
        &source.rules,
        MirrorFrame {
            player_position: player.to_array(),
            view_head_position: head.to_array(),
            source_enabled: true,
            pointer_locked: true,
        },
        &[input],
    );
    let output = &state.outputs()[0];
    assert!(output.capture);
    let view = plane - head;
    let reflected = view - normal * (2.0 * view.dot(normal));
    let expected_forward = (plane + reflected - camera).normalize();
    let expected_up = (camera_up - expected_forward * camera_up.dot(expected_forward)).normalize();
    let camera_rotation = Rotation::from_array(output.pose.rotation);
    assert!((camera_rotation * Vector::NEG_Z).dot(expected_forward) > 0.9999);
    assert!((camera_rotation * Vector::Y).dot(expected_up) > 0.9999);
}

#[test]
fn invalid_source_clip_range_disables_render_without_changing_state() {
    let mut source = fixture();
    source.mirrors.truncate(1);
    let near = source.mirrors[0].near_plane;
    for far in [near, near * 0.5] {
        source.mirrors[0].initial_far_plane = far;
        let mut state = MirrorState::new(&source);
        state.update(&source.rules, frame([0.0, 2.0, 0.0]), &[input(2.0)]);
        let output = state.outputs()[0];
        assert!(output.capture);
        assert!(!output.has_valid_clip_range());
        assert_eq!(output.capture_far_plane, far);
        assert!(output.next_far_plane > near);
        state.update(&source.rules, frame([0.0, 2.0, 0.0]), &[input(2.0)]);
        assert!(state.outputs()[0].has_valid_clip_range());
        assert_eq!(state.outputs()[0].capture_far_plane, output.next_far_plane);
    }
}

#[test]
fn body_distance_and_active_camera_position_have_separate_roles() {
    let mut source = fixture();
    source.mirrors.truncate(2);
    source.rules.distance_bounds_meters = [0.0, 10.0];
    source.rules.fov_bounds_degrees = [10.0, 55.0];
    let inputs = [input(2.0), input(6.0)];
    let mut state = MirrorState::new(&source);
    let mut view = frame([0.0, 2.0, 0.0]);
    state.update(&source.rules, view, &inputs);
    let original = [state.outputs()[0], state.outputs()[1]];

    // Chase-camera motion changes reflection and FOV, but not body rank or far.
    view.view_head_position = [0.0, 3.0, 5.0];
    state.update(&source.rules, view, &inputs);
    let moved_camera = [state.outputs()[0], state.outputs()[1]];
    for index in 0..2 {
        assert_eq!(moved_camera[index].rank, original[index].rank);
        assert_eq!(
            moved_camera[index].next_far_plane,
            original[index].next_far_plane
        );
        assert_ne!(moved_camera[index].fov_degrees, original[index].fov_degrees);
        assert_ne!(
            moved_camera[index].pose.rotation,
            original[index].pose.rotation
        );
    }

    // Body motion changes rank and far, while the active camera remains fixed.
    view.player_position = [0.0, 0.0, 7.0];
    state.update(&source.rules, view, &inputs);
    for index in 0..2 {
        let moved_body = state.outputs()[index];
        assert_ne!(moved_body.rank, moved_camera[index].rank);
        assert_ne!(
            moved_body.next_far_plane,
            moved_camera[index].next_far_plane
        );
        assert_eq!(moved_body.fov_degrees, moved_camera[index].fov_degrees);
        assert_eq!(moved_body.pose.rotation, moved_camera[index].pose.rotation);
    }
}

fn installed_mirror() -> (
    ReferenceManifest,
    my_summer_slop::starting_area::GarageRuntime,
) {
    let r =
        ReferenceManifest::from_json(&std::fs::read("web/assets/reference/manifest.json").unwrap())
            .unwrap();
    let g = my_summer_slop::starting_area::GarageRuntime::new(&r).unwrap();
    (r, g)
}
fn bound_input(
    r: &ReferenceManifest,
    g: &my_summer_slop::starting_area::GarageRuntime,
) -> MirrorInput {
    let m = &r.mirrors.mirrors[0];
    let plane = g
        .area
        .interaction
        .mechanisms
        .node_pose(m.plane_node)
        .unwrap();
    let camera = g
        .area
        .interaction
        .mechanisms
        .node_pose(m.camera_node)
        .unwrap();
    let rotation = Rotation::from_array(plane.rotation);
    MirrorInput {
        plane_position: plane.position,
        plane_normal: (rotation * Vector::Y).to_array(),
        camera_up: (rotation * Vector::NEG_Z).to_array(),
        camera_position: camera.position,
        normal_view_visible: true,
    }
}
fn check_bound(
    r: &ReferenceManifest,
    g: &my_summer_slop::starting_area::GarageRuntime,
) -> my_summer_slop::mirrors::MirrorOutput {
    let input = bound_input(r, g);
    let plane = Vector::from_array(input.plane_position);
    let normal = Vector::from_array(input.plane_normal);
    let head = plane + normal * 1.5 + Vector::from_array(input.camera_up) * 0.2;
    let mut source = r.mirrors.clone();
    source.mirrors.truncate(1);
    let mut state = MirrorState::new(&source);
    state.update(
        &source.rules,
        MirrorFrame {
            player_position: (plane + normal).to_array(),
            view_head_position: head.to_array(),
            source_enabled: true,
            pointer_locked: true,
        },
        &[input],
    );
    let output = state.outputs()[0];
    assert!(output.capture);
    let direction = plane - head;
    let expected = (plane + direction
        - 2. * direction.dot(normal) * normal
        - Vector::from_array(input.camera_position))
    .normalize();
    assert!((Rotation::from_array(output.pose.rotation) * Vector::NEG_Z).dot(expected) > 0.9999);
    assert_eq!(output.pose.position, input.camera_position);
    output
}
#[test]
#[ignore = "requires installed source export"]
fn mirror_follows_part_transactions() {
    use my_summer_slop::simulation::{FrameInput, keys};
    let (r, mut g) = installed_mirror();
    let part = r
        .parts
        .iter()
        .find(|p| p.part_types.iter().any(|t| t == "rightmirror"))
        .unwrap();
    let mirror = part.item;
    let usable = part.usable.unwrap();
    let door = r
        .parts
        .iter()
        .find(|p| p.part_types.iter().any(|t| t == "frdoor"))
        .unwrap()
        .item;
    let slot = r
        .part_slots
        .iter()
        .find(|s| s.part == Some(mirror))
        .unwrap()
        .id;
    let before = bound_input(&r, &g);
    g.area.interaction.mechanisms.step(
        usable,
        FrameInput {
            held: keys::USE,
            mouse: [15., 20.],
            ..Default::default()
        },
        g.area.player.view_pose(&g.physics),
        0.02,
    );
    g.area
        .interaction
        .sync_parts(&mut g.area.items, &mut g.physics)
        .unwrap();
    assert_ne!(bound_input(&r, &g).plane_normal, before.plane_normal);
    let adjustment = g.area.interaction.mechanisms.value(usable, false);
    check_bound(&r, &g);
    common::detach(&mut g, door);
    check_bound(&r, &g);
    assert_eq!(
        g.area.interaction.mechanisms.value(usable, false),
        adjustment
    );
    let attached = bound_input(&r, &g);
    common::detach(&mut g, mirror);
    let detached = bound_input(&r, &g);
    assert!(
        (Vector::from_array(attached.plane_position) - Vector::from_array(detached.plane_position))
            .length()
            < 0.001
    );
    assert!(
        Vector::from_array(attached.plane_normal).dot(Vector::from_array(detached.plane_normal))
            > 0.9999
    );
    check_bound(&r, &g);
    let point = g.area.items.get(mirror).unwrap().pose.position;
    g.area
        .interaction
        .pickup_item(
            mirror,
            point,
            &g.area.player,
            &mut g.area.items,
            &mut g.physics,
        )
        .unwrap();
    g.area
        .interaction
        .sync_parts(&mut g.area.items, &mut g.physics)
        .unwrap();
    check_bound(&r, &g);
    let carried = bound_input(&r, &g).camera_position;
    let body = g.area.items.get(mirror).unwrap().body.unwrap();
    let moved = g.physics.rapier().bodies[body].translation() + Vector::new(0.4, 0.3, -0.2);
    g.physics.rapier_mut().bodies[body].set_translation(moved, true);
    g.area
        .interaction
        .sync_parts(&mut g.area.items, &mut g.physics)
        .unwrap();
    assert_ne!(bound_input(&r, &g).camera_position, carried);
    check_bound(&r, &g);
    assert_eq!(
        g.area
            .interaction
            .drop_held(&mut g.area.items, &mut g.physics),
        Some(mirror)
    );
    g.area
        .interaction
        .sync_parts(&mut g.area.items, &mut g.physics)
        .unwrap();
    check_bound(&r, &g);
    g.area
        .interaction
        .parts
        .install(mirror, slot, &mut g.area.items, &mut g.physics)
        .unwrap();
    g.area
        .interaction
        .parts
        .mount_mechanism(mirror, &mut g.area.interaction.mechanisms);
    g.area
        .interaction
        .sync_parts(&mut g.area.items, &mut g.physics)
        .unwrap();
    check_bound(&r, &g);
    assert_eq!(
        g.area.interaction.mechanisms.value(usable, false),
        Some(0.5)
    );
}
#[test]
#[ignore = "requires installed source export"]
fn mirror_rebase_keeps_projection() {
    let (r, mut g) = installed_mirror();
    for loose in [false, true] {
        if loose {
            common::detach(
                &mut g,
                r.parts
                    .iter()
                    .find(|p| p.part_types.iter().any(|t| t == "rightmirror"))
                    .unwrap()
                    .item,
            );
        }
        let before = check_bound(&r, &g);
        let shift = [1280., 0., -1280.];
        g.physics.rebase(shift);
        g.stream.origin.shift(shift);
        g.area.sync_origin(g.stream.origin.offset);
        g.area
            .interaction
            .sync_parts(&mut g.area.items, &mut g.physics)
            .unwrap();
        let after = check_bound(&r, &g);
        assert!(
            (Vector::from_array(after.pose.position)
                - (Vector::from_array(before.pose.position) - Vector::from_array(shift)))
            .length()
                < 0.001
        );
        assert!(
            Rotation::from_array(after.pose.rotation)
                .dot(Rotation::from_array(before.pose.rotation))
                .abs()
                > 0.99999
        );
        assert!((after.fov_degrees - before.fov_degrees).abs() < 0.02);
        assert!((after.next_far_plane - before.next_far_plane).abs() < 0.02);
    }
}
