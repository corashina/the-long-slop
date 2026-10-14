mod common;
use my_summer_slop::simulation::{FrameInput, keys};
use my_summer_slop::starting_area::GarageRuntime;

fn aim_at(
    runtime: &mut GarageRuntime,
    reference: &my_summer_slop::reference::ReferenceManifest,
    item: my_summer_slop::reference::ItemId,
) -> my_summer_slop::interaction::AimTarget {
    let center = runtime
        .physics
        .rapier()
        .colliders
        .iter()
        .find(|(_, collider)| collider.user_data as u32 == item.0 + 1)
        .map(|(_, collider)| collider.compute_aabb().center().to_array())
        .expect("item collider");
    let standing = rapier3d::prelude::SharedShape::capsule_y(
        reference.player.standing_height * 0.5 - reference.player.standing_radius,
        reference.player.standing_radius,
    );
    for index in 0..24 {
        let angle = index as f32 * std::f32::consts::TAU / 24.0;
        let position = [
            center[0] + angle.cos() * 1.0,
            reference.area.driveway_height + reference.player.standing_height * 0.5 + 0.05,
            center[2] + angle.sin() * 1.0,
        ];
        let old = runtime.area.player.body_handle();
        let pose = my_summer_slop::vehicle::VehiclePose {
            position,
            rotation: [0.0, 0.0, 0.0, 1.0],
        };
        if runtime
            .physics
            .queries(
                rapier3d::prelude::QueryFilter::default()
                    .exclude_sensors()
                    .exclude_rigid_body(old),
            )
            .intersect_shape(my_summer_slop::vehicle::to_rapier(pose), standing.as_ref())
            .next()
            .is_some()
        {
            continue;
        }
        runtime.physics.rapier_mut().remove_body(old);
        runtime.area.player =
            my_summer_slop::player::PlayerState::new(&reference.player, pose, &mut runtime.physics);
        runtime.area.player.configure_seats(&reference.seats);
        runtime
            .area
            .interaction
            .bind_player_body(runtime.area.player.body_handle());
        let view = runtime.area.player.view_pose(&runtime.physics);
        let delta = std::array::from_fn::<_, 3, _>(|i| center[i] - view.position[i]);
        let yaw = delta[0].atan2(-delta[2]).to_degrees();
        let pitch = (-delta[1])
            .atan2((delta[0] * delta[0] + delta[2] * delta[2]).sqrt())
            .to_degrees();
        runtime.area.player.step(
            FrameInput {
                mouse: [yaw, pitch],
                ..FrameInput::default()
            },
            &mut runtime.physics,
            1.0 / 60.0,
        );
        if let Some(target) = common::focus(
            &runtime.area.interaction,
            &runtime.physics,
            runtime.area.player.view_pose(&runtime.physics),
        ) && target.item == item
        {
            return target;
        }
    }
    panic!("cannot aim at source item {}", item.0);
}

#[test]
#[ignore = "requires installed reference export"]
fn production_left_click_drag_moves_door_and_holds_camera() {
    let reference = my_summer_slop::reference::ReferenceManifest::from_json(
        &std::fs::read("web/assets/reference/manifest.json").unwrap(),
    )
    .unwrap();
    let mut runtime = GarageRuntime::new(&reference).unwrap();
    runtime.started = true;
    runtime.step(&reference, FrameInput::default());
    let door = reference
        .mechanisms
        .iter()
        .find(|m| m.source.path_id == 76010)
        .unwrap();
    aim_at(&mut runtime, &reference, door.item);
    let yaw = runtime.area.player.yaw_degrees();
    let pitch = runtime.area.player.pitch_degrees();
    let before = runtime
        .area
        .interaction
        .mechanisms
        .rotation_degrees(door.item);
    for step in 0..3 {
        runtime.step(
            &reference,
            FrameInput {
                held: keys::PRIMARY,
                pressed: if step == 0 { keys::PRIMARY } else { 0 },
                mouse: [-10.0, -10.0],
                ..Default::default()
            },
        );
        assert_eq!(
            runtime.area.interaction.dragging_mechanism(),
            Some(door.item)
        );
        assert_eq!(runtime.area.player.yaw_degrees(), yaw);
        assert_eq!(runtime.area.player.pitch_degrees(), pitch);
    }
    let after = runtime
        .area
        .interaction
        .mechanisms
        .rotation_degrees(door.item);
    assert_ne!(after, before);
    runtime.step(
        &reference,
        FrameInput {
            released: keys::PRIMARY,
            mouse: [1.0, 1.0],
            ..Default::default()
        },
    );
    assert_eq!(runtime.area.interaction.dragging_mechanism(), None);
    assert_ne!(runtime.area.player.yaw_degrees(), yaw);
    assert_ne!(runtime.area.player.pitch_degrees(), pitch);
    assert_eq!(
        runtime
            .area
            .interaction
            .mechanisms
            .rotation_degrees(door.item),
        after
    );
}
