mod common;
use my_summer_slop::interaction::feedback::{ActionMask, FeedbackContext};
use my_summer_slop::interaction::items::ItemStore;
use my_summer_slop::interaction::{
    AimTarget, InteractionAction, InteractionState, LightAim, ResolvedAim,
};
use my_summer_slop::lighting::LightingState;
use my_summer_slop::player::PlayerState;
use my_summer_slop::reference::{ColliderId, LightControlId, LightControllerKind};
use my_summer_slop::simulation::{FrameInput, keys};
use my_summer_slop::vehicle::{PhysicsWorld, VehiclePose};
use rapier3d::math::Vector;
use rapier3d::prelude::{ColliderBuilder, RigidBodyBuilder};
fn pose() -> VehiclePose {
    VehiclePose {
        position: [10., 2., 0.],
        rotation: [0., 0., 0., 1.],
    }
}
fn target(id: u32) -> ResolvedAim {
    let r = common::reference();
    let c = r
        .lighting
        .controllers
        .iter()
        .find(|c| c.id == LightControlId(id))
        .unwrap();
    ResolvedAim {
        light_target: Some(LightAim {
            control: r
                .lighting
                .controllers
                .iter()
                .filter(|other| other.aim_colliders.contains(&c.aim_colliders[0]))
                .map(|other| other.id)
                .min_by_key(|id| id.0)
                .unwrap(),
            node: r
                .colliders
                .iter()
                .find(|record| record.id == c.aim_colliders[0])
                .unwrap()
                .node,
            collider: c.aim_colliders[0],
            distance: 1.,
            point: [10., 2., -1.],
        }),
        ..Default::default()
    }
}
fn fixture() -> (InteractionState, ItemStore, PhysicsWorld, PlayerState) {
    let r = common::reference();
    let mut p = common::world();
    let player = PlayerState::new(&r.player, pose(), &mut p);
    (
        InteractionState::new(&r).unwrap(),
        ItemStore::new(&r).unwrap(),
        p,
        player,
    )
}
fn step(
    i: &mut InteractionState,
    t: ResolvedAim,
    p: &mut PlayerState,
    s: &mut ItemStore,
    w: &mut PhysicsWorld,
    pressed: u32,
) -> Option<InteractionAction> {
    i.step(
        FrameInput {
            held: pressed,
            pressed,
            ..Default::default()
        },
        t,
        p,
        s,
        w,
        0.016,
        Default::default(),
        1.,
    )
}
#[test]
fn switch_aim_obeys_reach_and_occlusion() {
    let (i, s, mut w, p) = fixture();
    w.rapier_mut().insert(
        RigidBodyBuilder::fixed().translation(Vector::new(10., 2., -1.)),
        ColliderBuilder::ball(0.1).user_data(10u128 << 64),
    );
    w.refresh_queries();
    let mut short = common::reference();
    short.player.aim_reach = 0.5;
    assert!(
        InteractionState::new(&short)
            .unwrap()
            .resolve(&w, pose(), &s)
            .light_target
            .is_none()
    );
    let t = i.resolve(&w, pose(), &s);
    assert_eq!(t.light_target.unwrap().collider, ColliderId(10));
    assert_eq!(
        i.available_actions(&t, &p, &s, &w, Default::default(),)
            .light_control,
        Some(LightControlId(1))
    );
    w.rapier_mut().insert(
        RigidBodyBuilder::fixed().translation(Vector::new(10., 2., -0.4)),
        ColliderBuilder::cuboid(0.5, 0.5, 0.05),
    );
    w.refresh_queries();
    let blocked = i.resolve(&w, pose(), &s);
    assert_eq!(
        i.available_actions(&blocked, &p, &s, &w, Default::default(),)
            .light_control,
        None
    );
}
#[test]
fn use_and_click_toggle_once() {
    let r = common::reference();
    let (mut i, mut s, mut w, mut p) = fixture();
    let mut lighting = LightingState::new(&r.lighting);
    let action = step(
        &mut i,
        target(1),
        &mut p,
        &mut s,
        &mut w,
        keys::USE | keys::PRIMARY,
    );
    assert_eq!(
        action,
        Some(InteractionAction::LightControl(LightControlId(1)))
    );
    if let Some(InteractionAction::LightControl(id)) = action {
        lighting.use_control(&r.lighting, id);
    }
    assert!(
        lighting
            .light_outputs()
            .iter()
            .find(|l| l.id == r.lighting.controllers[0].states[0].lights[0])
            .unwrap()
            .enabled
    );
    assert_eq!(
        i.step(
            FrameInput {
                held: keys::USE | keys::PRIMARY,
                ..Default::default()
            },
            target(1),
            &mut p,
            &mut s,
            &mut w,
            0.016,
            Default::default(),
            1.
        ),
        None
    );
}
#[test]
fn cabin_switch_does_not_exit_seat() {
    let r = common::reference();
    let (mut i, mut s, mut w, mut p) = fixture();
    p.configure_seats(&r.seats);
    let seat = &r.seats[0];
    assert!(p.enter(seat.id, pose(), &mut w));
    assert_eq!(
        step(&mut i, target(7), &mut p, &mut s, &mut w, keys::USE),
        Some(InteractionAction::LightControl(LightControlId(7)))
    );
    assert_eq!(p.seat(), Some(seat.id));
}
#[test]
fn switch_feedback_matches_execution() {
    let (mut i, mut s, mut w, mut p) = fixture();
    for id in [1, 2, 3, 4, 5, 7, 8] {
        let t = target(id);
        let a = i.available_actions(&t, &p, &s, &w, Default::default());
        assert_eq!(a.context, FeedbackContext::LightSwitch);
        assert_ne!(a.actions.0 & ActionMask::USE, 0);
        assert_eq!(
            step(&mut i, t, &mut p, &mut s, &mut w, keys::USE),
            a.light_control.map(InteractionAction::LightControl)
        );
    }
    let mut t = target(1);
    t.light_target.as_mut().unwrap().distance = 100.;
    let a = i.available_actions(&t, &p, &s, &w, Default::default());
    assert_eq!(a.light_control, None);
    assert_eq!(step(&mut i, t, &mut p, &mut s, &mut w, keys::USE), None);
}
#[test]
fn house_lamps_materials_visibility_and_current_pose_lerp() {
    let mut r = common::reference().lighting;
    let mut state = LightingState::new(&r);
    let house: Vec<_> = r
        .controllers
        .iter()
        .filter(|c| c.kind == LightControllerKind::House)
        .collect();
    assert_eq!(house.iter().map(|c| c.states.len()).sum::<usize>(), 6);
    for c in house {
        for lamp in &c.states {
            assert!(
                !state
                    .light_outputs()
                    .iter()
                    .find(|o| o.id == lamp.lights[0])
                    .unwrap()
                    .enabled
            );
        }
        state.use_control(&r, c.id);
        for lamp in &c.states {
            assert!(
                state
                    .light_outputs()
                    .iter()
                    .find(|o| o.id == lamp.lights[0])
                    .unwrap()
                    .enabled
            );
            assert_eq!(
                state
                    .material_outputs()
                    .iter()
                    .find(|o| o.renderer == lamp.renderers[0])
                    .unwrap()
                    .material_index,
                lamp.on_material.as_ref().unwrap().gltf_material
            );
        }
    }
    let p = &r.controllers[0].animated_poses[0];
    let off = p.states[0].translation;
    let on = p.states[1].translation;
    state.advance_fades(&r, 0.01);
    state.advance_fades(&r, 0.01);
    let actual = state
        .control_poses()
        .iter()
        .find(|o| o.node == p.node)
        .unwrap()
        .local_pose
        .translation;
    for n in 0..3 {
        assert!((actual[n] - (off[n] + (on[n] - off[n]) * 0.25)).abs() < 1e-7);
    }
    r.controllers[0].states[0].broken = true;
    let lamp = &r.controllers[0].states[0];
    state.use_control(&r, LightControlId(1));
    state.use_control(&r, LightControlId(1));
    assert!(
        !state
            .light_outputs()
            .iter()
            .find(|o| o.id == lamp.lights[0])
            .unwrap()
            .enabled
    );
    assert!(
        !state
            .visibility_outputs()
            .iter()
            .find(|o| o.node == lamp.working_nodes[0])
            .unwrap()
            .visible
    );
}
#[test]
fn cabin_modes_override_strict_door_threshold_before_fade() {
    let r = common::reference().lighting;
    let cabin = r
        .controllers
        .iter()
        .position(|c| c.kind == LightControllerKind::Cabin)
        .unwrap();
    let c = &r.controllers[cabin];
    let threshold = c.cabin_input.as_ref().unwrap().threshold;
    let mut l = LightingState::new(&r);
    l.update_cabin(&r, cabin, [threshold; 4]);
    for _ in 0..3 {
        l.advance_fades(&r, 0.1);
    }
    let id = c.states[1].lights[0];
    assert!(
        !l.light_outputs()
            .iter()
            .find(|o| o.id == id)
            .unwrap()
            .enabled
    );
    l.update_cabin(&r, cabin, [threshold + 0.001, 0., 0., 0.]);
    for _ in 0..3 {
        l.advance_fades(&r, 0.1);
    }
    assert!(
        l.light_outputs()
            .iter()
            .find(|o| o.id == id)
            .unwrap()
            .enabled
    );
    l.use_control(&r, c.id);
    l.update_cabin(&r, cabin, [0.; 4]);
    for _ in 0..3 {
        l.advance_fades(&r, 0.1);
    }
    assert!(
        l.light_outputs()
            .iter()
            .find(|o| o.id == id)
            .unwrap()
            .enabled
    );
    l.use_control(&r, c.id);
    l.update_cabin(&r, cabin, [1.; 4]);
    for _ in 0..3 {
        l.advance_fades(&r, 0.1);
    }
    assert!(
        !l.light_outputs()
            .iter()
            .find(|o| o.id == id)
            .unwrap()
            .enabled
    );
}
#[test]
fn physical_headlight_switch_cycles_the_shared_car_request_once() {
    let r = common::reference();
    let (mut i, mut items, mut w, mut p) = fixture();
    let a = i.available_actions(&target(9), &p, &items, &w, Default::default());
    assert_eq!(a.light_control, Some(LightControlId(8)));
    let action = step(
        &mut i,
        target(9),
        &mut p,
        &mut items,
        &mut w,
        keys::USE | keys::PRIMARY,
    );
    assert_eq!(
        action,
        Some(InteractionAction::LightControl(LightControlId(8)))
    );
    let mut state = LightingState::new(&r.lighting);
    state.use_control(&r.lighting, a.light_control.unwrap());
    for _ in 0..3 {
        state.advance_fades(&r.lighting, 0.1);
    }
    for c in r
        .lighting
        .controllers
        .iter()
        .filter(|c| c.kind == LightControllerKind::Headlight)
    {
        assert!(
            state
                .light_outputs()
                .iter()
                .find(|o| o.id == c.states[1].lights[0])
                .unwrap()
                .enabled
        );
        assert!(
            !state
                .light_outputs()
                .iter()
                .find(|o| o.id == c.states[2].lights[0])
                .unwrap()
                .enabled
        );
    }
    let mut conflict = r.clone();
    conflict.lighting.controllers[6].aim_colliders =
        r.lighting.controllers[0].aim_colliders.clone();
    assert!(InteractionState::new(&conflict).is_err());
}
#[test]
#[ignore = "Requires exported Steam assets"]
fn exported_house_controls_use_native_confirmed_poses() {
    use my_summer_slop::reference::ReferenceManifest;
    use my_summer_slop::starting_area::GarageRuntime;
    use rapier3d::math::Rotation;
    use rapier3d::prelude::{QueryFilter, Ray};
    let r =
        ReferenceManifest::from_json(&std::fs::read("web/assets/reference/manifest.json").unwrap())
            .unwrap();
    let source = GarageRuntime::new(&r).unwrap();
    for c in r
        .lighting
        .controllers
        .iter()
        .filter(|c| c.kind == LightControllerKind::House)
    {
        let collider = source
            .physics
            .rapier()
            .colliders
            .iter()
            .find(|(_, col)| (col.user_data >> 64) as u32 == c.aim_colliders[0].0)
            .unwrap()
            .1;
        let center = collider.compute_aabb().center();
        let mut accepted = None;
        'candidates: for distance in [0.7, 1., 1.4] {
            for direction in [Vector::X, -Vector::X, Vector::Z, -Vector::Z] {
                let mut view = center + direction * distance;
                let ground = source
                    .physics
                    .queries(QueryFilter::default().exclude_sensors())
                    .cast_ray_and_get_normal(&Ray::new(view + Vector::Y, -Vector::Y), 4., true);
                let Some((_, ground)) = ground else {
                    continue;
                };
                view.y = view.y + 1. - ground.time_of_impact + r.player.standing_height + 0.02;
                let delta = center - view;
                let pitch = (-delta.y)
                    .atan2(Vector::new(delta.x, 0., delta.z).length())
                    .to_degrees();
                let yaw = Rotation::from_rotation_arc(-Vector::Z, -direction);
                let mut candidate = r.clone();
                candidate.area.player_spawn.translation =
                    (view - Vector::Y * (r.player.standing_height * 0.5)).to_array();
                candidate.area.player_spawn.rotation = yaw.to_array();
                let mut runtime = GarageRuntime::new(&candidate).unwrap();
                runtime.started = true;
                runtime.step(
                    &candidate,
                    FrameInput {
                        mouse: [0., pitch],
                        ..Default::default()
                    },
                );
                for _ in 0..30 {
                    runtime.step(&candidate, FrameInput::default());
                }
                let a = runtime.area.interaction.available_actions(
                    &runtime.aim,
                    &runtime.area.player,
                    &runtime.area.items,
                    &runtime.physics,
                    Default::default(),
                );
                if a.light_control != Some(c.id) {
                    continue;
                }
                for _ in 0..180 {
                    let eye = Vector::from_array(
                        runtime.area.player.view_pose(&runtime.physics).position,
                    );
                    let delta = center - eye;
                    let desired_yaw = delta.x.atan2(-delta.z).to_degrees();
                    let desired_pitch = (-delta.y)
                        .atan2(Vector::new(delta.x, 0., delta.z).length())
                        .to_degrees();
                    let mouse = [
                        (desired_yaw - runtime.area.player.yaw_degrees() + 180.).rem_euclid(360.)
                            - 180.,
                        desired_pitch - runtime.area.player.pitch_degrees(),
                    ];
                    let mut input = FrameInput {
                        mouse,
                        ..Default::default()
                    };
                    for _ in 0..6 {
                        runtime.step(&candidate, input.take_step());
                    }
                    assert_eq!(
                        runtime
                            .area
                            .interaction
                            .available_actions(
                                &runtime.aim,
                                &runtime.area.player,
                                &runtime.area.items,
                                &runtime.physics,
                                Default::default(),
                            )
                            .light_control,
                        Some(c.id)
                    );
                }
                runtime.step(
                    &candidate,
                    FrameInput {
                        held: keys::USE,
                        pressed: keys::USE,
                        ..Default::default()
                    },
                );
                assert!(
                    runtime
                        .lighting
                        .light_outputs()
                        .iter()
                        .find(|o| o.id == c.states[0].lights[0])
                        .unwrap()
                        .enabled
                );
                accepted = Some(
                    serde_json::json!({"control":c.id.0,"spawn":candidate.area.player_spawn,"pitch":pitch,"center":center.to_array()}),
                );
                break 'candidates;
            }
        }
        println!(
            "SWITCH_POSE {}",
            accepted.expect("source switch needs a reachable standing pose")
        );
    }
}
#[test]
#[ignore = "Requires exported Steam assets"]
fn exported_cabin_and_headlight_controls_work_from_driver_seat() {
    use my_summer_slop::reference::ReferenceManifest;
    use my_summer_slop::starting_area::GarageRuntime;
    use rapier3d::math::Rotation;
    let mut r =
        ReferenceManifest::from_json(&std::fs::read("web/assets/reference/manifest.json").unwrap())
            .unwrap();
    let tilt = (-85.0_f32).to_radians();
    let head = r.player.standing_height * 0.5;
    r.area.player_spawn.translation = [
        37.8,
        1.85 + head - head * tilt.cos(),
        -69.3 - head * tilt.sin(),
    ];
    r.area.player_spawn.rotation = [(tilt / 2.).sin(), 0., 0., (tilt / 2.).cos()];
    for id in [7, 8] {
        let c = r
            .lighting
            .controllers
            .iter()
            .find(|c| c.id == LightControlId(id))
            .unwrap();
        let mut runtime = GarageRuntime::new(&r).unwrap();
        runtime.started = true;
        runtime.step(
            &r,
            FrameInput {
                pressed: keys::USE,
                released: keys::USE,
                ..Default::default()
            },
        );
        assert!(runtime.area.player.driving());
        let seat = runtime.area.player.seat();
        let center = runtime
            .physics
            .rapier()
            .colliders
            .iter()
            .find(|(_, col)| (col.user_data >> 64) as u32 == c.aim_colliders[0].0)
            .unwrap()
            .1
            .compute_aabb()
            .center();
        let view = runtime.area.player.view_pose(&runtime.physics);
        let body = runtime.area.player.body_pose(&runtime.physics);
        let direction = Rotation::from_array(body.rotation).inverse()
            * (center - Vector::from_array(view.position)).normalize();
        let yaw = direction.x.atan2(-direction.z).to_degrees();
        let pitch = (-direction.y).asin().to_degrees();
        let mouse = [
            yaw - runtime.area.player.yaw_degrees(),
            pitch - runtime.area.player.pitch_degrees(),
        ];
        runtime.step(
            &r,
            FrameInput {
                mouse,
                ..Default::default()
            },
        );
        runtime.step(&r, FrameInput::default());
        assert_eq!(
            runtime
                .area
                .interaction
                .available_actions(
                    &runtime.aim,
                    &runtime.area.player,
                    &runtime.area.items,
                    &runtime.physics,
                    Default::default(),
                )
                .light_control,
            Some(c.id)
        );
        runtime.step(
            &r,
            FrameInput {
                held: keys::USE,
                pressed: keys::USE,
                ..Default::default()
            },
        );
        assert_eq!(runtime.area.player.seat(), seat);
        for _ in 0..3 {
            runtime.advance_light_fades(&r, 0.1);
        }
        let light = c.states[1].lights[0];
        assert!(
            runtime
                .lighting
                .light_outputs()
                .iter()
                .find(|o| o.id == light)
                .unwrap()
                .enabled
        );
        println!(
            "SEATED_SWITCH_POSE {}",
            serde_json::json!({"control":id,"mouse":mouse,"yaw":yaw,"pitch":pitch})
        );
    }
}
#[test]
#[ignore = "Requires exported Steam assets"]
fn exported_cabin_door_values_keep_all_four_source_mappings() {
    use my_summer_slop::reference::ReferenceManifest;
    use my_summer_slop::starting_area::GarageRuntime;
    let r =
        ReferenceManifest::from_json(&std::fs::read("web/assets/reference/manifest.json").unwrap())
            .unwrap();
    let cabin = r
        .lighting
        .controllers
        .iter()
        .find_map(|c| c.cabin_input.as_ref())
        .unwrap();
    let control = r
        .lighting
        .controllers
        .iter()
        .position(|c| c.cabin_input.is_some())
        .unwrap();
    for (index, source) in cabin.door_usable_sources.iter().enumerate() {
        let mechanism = r.mechanisms.iter().find(|m| m.source == *source).unwrap();
        let mut runtime = GarageRuntime::new(&r).unwrap();
        let (handle, col) = runtime
            .physics
            .rapier()
            .colliders
            .iter()
            .find(|(_, col)| (col.user_data as u32) == mechanism.item.0 + 1)
            .unwrap();
        let t = AimTarget {
            collider: handle,
            item: mechanism.item,
            distance: 1.,
            point: col.compute_aabb().center().to_array(),
            slot: None,
        };
        for sign in [1., -1.] {
            runtime.area.interaction.step(
                FrameInput {
                    held: keys::USE,
                    pressed: keys::USE,
                    mouse: [sign * 100., sign * 100.],
                    ..Default::default()
                },
                ResolvedAim {
                    use_target: Some(t),
                    ..Default::default()
                },
                &mut runtime.area.player,
                &mut runtime.area.items,
                &mut runtime.physics,
                0.016,
                Default::default(),
                1.,
            );
            if runtime
                .area
                .interaction
                .mechanisms
                .value(mechanism.item, false)
                .unwrap()
                > 0.1
            {
                break;
            }
        }
        let value = runtime
            .area
            .interaction
            .mechanisms
            .value(mechanism.item, false)
            .unwrap();
        assert!(value > 0.1);
        let values = runtime
            .area
            .interaction
            .cabin_door_values(control, &runtime.area.items);
        for n in 0..4 {
            assert!((values[n] - if n == index { value } else { 0. }).abs() < 1e-6);
        }
        runtime.lighting.update_cabin(&r.lighting, control, values);
        for _ in 0..3 {
            runtime.lighting.advance_fades(&r.lighting, 0.1);
        }
        let light = r
            .lighting
            .controllers
            .iter()
            .find(|c| c.kind == LightControllerKind::Cabin)
            .unwrap()
            .states[1]
            .lights[0];
        assert!(
            runtime
                .lighting
                .light_outputs()
                .iter()
                .find(|o| o.id == light)
                .unwrap()
                .enabled
        );
    }
}
fn held_container_fixture(
    drinkable: bool,
    full: bool,
) -> (InteractionState, ItemStore, PhysicsWorld, PlayerState) {
    use my_summer_slop::interaction::items::ItemLocation;
    use my_summer_slop::reference::{ItemId, OpeningId};
    let mut r = common::reference();
    r.tanks[0].no_drink = !drinkable;
    if full {
        r.tanks[0].quantities_liters = [r.tanks[0].capacity_liters, 0., 0., 0., 0., 0.];
    }
    r.openings[0].valve = 1.;
    let mut opening = r.openings[0].clone();
    opening.id = OpeningId(77);
    opening.tank = ItemId(1);
    opening.cap = ItemId(1);
    r.openings.push(opening);
    let mut w = common::world();
    let p = PlayerState::new(&r.player, pose(), &mut w);
    let mut i = InteractionState::new(&r).unwrap();
    let mut items = ItemStore::new(&r).unwrap();
    let body = w
        .rapier_mut()
        .insert_body(RigidBodyBuilder::dynamic().translation(Vector::new(10., 2., -0.5)));
    let item = items.get_mut(ItemId(1)).unwrap();
    item.body = Some(body);
    item.location = ItemLocation::World;
    i.pickup_item(ItemId(1), [10., 2., -0.5], &p, &mut items, &mut w)
        .unwrap();
    (i, items, w, p)
}
#[test]
fn drinkable_container_reserves_click_while_use_can_switch() {
    use my_summer_slop::reference::ItemId;
    let (mut i, mut items, mut w, mut p) = held_container_fixture(true, false);
    let aim = target(1);
    let a = i.available_actions(&aim, &p, &items, &w, Default::default());
    assert!(!a.lmb_use);
    assert_eq!(a.light_control, Some(LightControlId(1)));
    assert!(matches!(
        step(&mut i, aim, &mut p, &mut items, &mut w, keys::PRIMARY),
        Some(InteractionAction::Drink { source: ItemId(1), requested_liters }) if requested_liters > 0.
    ));
    assert_eq!(
        step(&mut i, aim, &mut p, &mut items, &mut w, keys::USE),
        Some(InteractionAction::LightControl(LightControlId(1)))
    );
}
#[test]
fn full_destination_reserves_click_after_fill_aim_changes() {
    use my_summer_slop::reference::ItemId;
    let (mut i, mut items, mut w, mut p) = held_container_fixture(false, true);
    let mut aim = target(1);
    aim.filling_target = Some(AimTarget {
        collider: rapier3d::prelude::ColliderHandle::invalid(),
        item: ItemId(19),
        distance: 1.,
        point: [10., 2., -1.],
        slot: None,
    });
    let a = i.available_actions(&aim, &p, &items, &w, Default::default());
    assert_eq!(a.fill_reservation, Some(common::reference().openings[0].id));
    assert_eq!(a.actions.0 & ActionMask::FILL, 0);
    assert!(!a.lmb_use);
    assert_eq!(
        step(&mut i, aim, &mut p, &mut items, &mut w, keys::PRIMARY),
        None
    );
    let switch = target(1);
    assert!(
        !i.available_actions(&switch, &p, &items, &w, Default::default(),)
            .lmb_use
    );
    assert_eq!(
        step(&mut i, switch, &mut p, &mut items, &mut w, keys::PRIMARY),
        None
    );
}
#[test]
fn house_switch_uses_resumed_intervals_and_snaps_at_completion() {
    let r = common::reference().lighting;
    let c = &r.controllers[0];
    let p = &c.animated_poses[0];
    let mut state = LightingState::new(&r);
    let initial = state
        .control_poses()
        .iter()
        .find(|o| o.node == p.node)
        .unwrap()
        .local_pose;
    let on = p.states[1];
    state.use_control(&r, c.id);
    state.advance_fades(&r, 0.1);
    let click = state
        .control_poses()
        .iter()
        .find(|o| o.node == p.node)
        .unwrap()
        .local_pose;
    assert_eq!(click.translation, initial.translation);
    state.advance_fades(&r, 0.01);
    let resumed = state
        .control_poses()
        .iter()
        .find(|o| o.node == p.node)
        .unwrap()
        .local_pose;
    for n in 0..3 {
        let expected = initial.translation[n] + (on.translation[n] - initial.translation[n]) * 0.25;
        assert!(
            (resumed.translation[n] - expected).abs() < 1e-7,
            "resumed axis {n}: expected {expected}, actual {}",
            resumed.translation[n]
        );
    }
    state.advance_fades(&r, 0.02);
    let later = state
        .control_poses()
        .iter()
        .find(|o| o.node == p.node)
        .unwrap()
        .local_pose;
    for n in 0..3 {
        let expected = resumed.translation[n] + (on.translation[n] - resumed.translation[n]) * 0.75;
        assert!(
            (later.translation[n] - expected).abs() < 1e-7,
            "later axis {n}"
        );
    }
    state.advance_fades(&r, 0.01);
    let completed = state
        .control_poses()
        .iter()
        .find(|o| o.node == p.node)
        .unwrap()
        .local_pose;
    assert_eq!(completed.translation, on.translation);
    assert_eq!(completed.rotation, on.rotation);
    state.advance_fades(&r, 0.5);
    let settled = state
        .control_poses()
        .iter()
        .find(|o| o.node == p.node)
        .unwrap()
        .local_pose;
    assert_eq!(settled.translation, on.translation);
    assert_eq!(settled.rotation, on.rotation);
}
