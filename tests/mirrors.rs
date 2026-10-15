mod common;
use my_summer_slop::mirrors::{MirrorFrame, MirrorInput, MirrorState};
use my_summer_slop::reference::ReferenceManifest;
use rapier3d::math::{Rotation, Vector};

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
