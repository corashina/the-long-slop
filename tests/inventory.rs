//! Critical ownership check against the installed export, without test asset fixtures.
use the_long_slop::{
    inventory::InventoryCommand,
    reference::ReferenceManifest,
    save::{SourceHash, SourceStamp},
    starting_area::GarageRuntime,
};
fn fixture() -> (ReferenceManifest, SourceStamp, GarageRuntime) {
    let bytes = std::fs::read("web/assets/reference/manifest.json").unwrap();
    let reference = ReferenceManifest::from_json(&bytes).unwrap();
    let digest = std::process::Command::new("sha256sum")
        .arg("web/assets/reference/manifest.json")
        .output()
        .unwrap();
    assert!(digest.status.success());
    let stamp = SourceStamp {
        app_id: reference.app_id,
        build_id: reference.build_id,
        manifest_sha256: String::from_utf8(digest.stdout)
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
            .into(),
        source_hashes: reference
            .source_hashes
            .iter()
            .map(|hash| SourceHash {
                file: hash.file.clone(),
                sha256: hash.sha256.clone(),
            })
            .collect(),
    };
    let runtime = GarageRuntime::new(
        &reference,
        the_long_slop::world::GameMode::default(),
        reference.world.draw_distance.default,
    )
    .unwrap();
    (reference, stamp, runtime)
}

fn pickup(runtime: &mut GarageRuntime, item: the_long_slop::reference::ItemId) {
    let point = runtime.area.items.get(item).unwrap().pose.position;
    if runtime.area.interaction.storage.stored(item) {
        runtime
            .area
            .interaction
            .pickup_stored_item(
                item,
                point,
                &runtime.area.player,
                &mut runtime.area.items,
                &mut runtime.physics,
            )
            .unwrap();
    } else {
        runtime
            .area
            .interaction
            .pickup_item(
                item,
                point,
                &runtime.area.player,
                &mut runtime.area.items,
                &mut runtime.physics,
            )
            .unwrap();
    }
    runtime.area.sync_inventory();
}

#[derive(Debug, PartialEq)]
struct OwnershipSnapshot {
    membership: String,
    item: String,
    body: String,
    colliders: Vec<String>,
    joints: Vec<String>,
    contents: String,
}

fn capture_ownership(
    runtime: &GarageRuntime,
    item: the_long_slop::reference::ItemId,
) -> OwnershipSnapshot {
    let state = runtime.area.items.get(item).unwrap();
    let body = state
        .body
        .and_then(|body| runtime.physics.rapier().bodies.get(body));
    OwnershipSnapshot {
        membership: format!(
            "{:?}",
            (
                runtime.area.inventory.slots,
                runtime.area.inventory.selected,
                runtime.area.interaction.held(),
                runtime.area.interaction.equipment.item()
            )
        ),
        item: format!("{state:?}"),
        body: format!("{body:?}"),
        colliders: body
            .into_iter()
            .flat_map(|body| body.colliders())
            .map(|handle| {
                format!(
                    "{:?}",
                    (handle, runtime.physics.rapier().colliders.get(*handle))
                )
            })
            .collect(),
        joints: runtime
            .physics
            .rapier()
            .impulse_joints
            .iter()
            .map(|joint| format!("{joint:?}"))
            .collect(),
        contents: contents(runtime, item),
    }
}

fn contents(runtime: &GarageRuntime, item: the_long_slop::reference::ItemId) -> String {
    let state = runtime.area.items.get(item).unwrap();
    format!(
        "{:?}",
        (
            state.id,
            &state.fluid,
            state.food,
            state.generated_condition,
            runtime.area.conditions.item_quality(item),
            runtime
                .area
                .interaction
                .cleaning
                .states
                .iter()
                .find(|tool| tool.item == item)
        )
    )
}

#[test]
#[ignore = "Requires the installed runtime export"]
fn g08_transfer_cycles_and_atomic_rejection() {
    use the_long_slop::{interaction::items::ItemLocation, reference::ItemId};
    let (reference, _, mut game) = fixture();
    let tools: Vec<_> = reference
        .items
        .iter()
        .filter(|item| {
            item.pickup
                .as_ref()
                .is_some_and(|rule| rule.can_hold_right_hand)
        })
        .map(|item| item.id)
        .collect();
    assert_eq!(tools.len(), 8);
    for tool in tools {
        // The second exported flashlight starts disabled. Activate its retained body for this transfer test.
        let body = game.area.items.get(tool).unwrap().body.unwrap();
        game.physics.edit_body(body, |body| body.set_enabled(true));
        let original_contents = contents(&game, tool);
        let original_conditions = format!(
            "{:?}",
            game.area.conditions.capture_save(&reference).unwrap()
        );
        for _ in 0..3 {
            pickup(&mut game, tool);
            let original_slots = game.area.inventory.slots;
            assert!(game.area.equip_carried(&mut game.physics).unwrap());
            assert_eq!(game.area.interaction.held(), None);
            assert_eq!(game.area.interaction.equipment.item(), Some(tool));
            assert_eq!(game.area.inventory.slots, original_slots);
            assert!(game.area.release_equipped(&mut game.physics).unwrap());
            assert_eq!(game.area.interaction.held(), Some(tool));
            assert_eq!(game.area.interaction.equipment.item(), None);
            assert_eq!(game.area.inventory.slots, original_slots);
            let selected = game.area.inventory.selected;
            let other = (selected + 1) % 4;
            assert!(
                game.area
                    .apply_inventory_command(
                        InventoryCommand::Select(other),
                        &reference,
                        &mut game.physics
                    )
                    .unwrap()
            );
            assert_eq!(
                game.area.items.get(tool).unwrap().location,
                ItemLocation::Stored(selected)
            );
            assert!(
                game.area
                    .apply_inventory_command(
                        InventoryCommand::Select(selected),
                        &reference,
                        &mut game.physics
                    )
                    .unwrap()
            );
            assert_eq!(game.area.interaction.equipment.item(), Some(tool));
            assert!(game.area.release_equipped(&mut game.physics).unwrap());
            assert_eq!(
                game.area
                    .interaction
                    .drop_held(&mut game.area.items, &mut game.physics),
                Some(tool)
            );
            game.area.sync_inventory();
            assert!(!game.area.inventory.slots.contains(&Some(tool)));
            assert_eq!(contents(&game, tool), original_contents);
            assert_eq!(
                format!(
                    "{:?}",
                    game.area.conditions.capture_save(&reference).unwrap()
                ),
                original_conditions
            );
        }
    }
    let tool = ItemId(128);
    pickup(&mut game, tool);
    assert!(game.area.equip_carried(&mut game.physics).unwrap());
    let other = reference.survival.starter_food;
    pickup(&mut game, other);
    let before = capture_ownership(&game, tool);
    let carry_before = capture_ownership(&game, other);
    assert!(!game.area.release_equipped(&mut game.physics).unwrap());
    assert_eq!(capture_ownership(&game, tool), before);
    assert_eq!(capture_ownership(&game, other), carry_before);
    game.area
        .interaction
        .drop_held(&mut game.area.items, &mut game.physics);
    for invalid in 0..5 {
        let body = game.area.items.get(tool).unwrap().body.unwrap();
        match invalid {
            0 => game.area.items.get_mut(tool).unwrap().removed = true,
            1 => game.physics.edit_body(body, |body| body.set_enabled(false)),
            2 => game.area.items.get_mut(tool).unwrap().body = None,
            3 => game.area.items.get_mut(tool).unwrap().location = ItemLocation::Attached(other),
            _ => game.physics.edit_body(body, |body| {
                body.set_body_type(rapier3d::prelude::RigidBodyType::Dynamic, false)
            }),
        }
        let before = capture_ownership(&game, tool);
        assert!(!game.area.release_equipped(&mut game.physics).unwrap());
        assert_eq!(capture_ownership(&game, tool), before);
        assert!(
            !game
                .area
                .apply_inventory_command(InventoryCommand::Select(1), &reference, &mut game.physics)
                .unwrap()
        );
        assert_eq!(capture_ownership(&game, tool), before);
        game.area.items.get_mut(tool).unwrap().removed = false;
        game.area.items.get_mut(tool).unwrap().body = Some(body);
        game.area.items.get_mut(tool).unwrap().location = ItemLocation::Equipped;
        game.physics.edit_body(body, |body| {
            body.set_enabled(true);
            body.set_body_type(
                rapier3d::prelude::RigidBodyType::KinematicPositionBased,
                false,
            );
        });
    }
    assert!(game.area.release_equipped(&mut game.physics).unwrap());
    for invalid in 0..4 {
        let body = game.area.items.get(tool).unwrap().body.unwrap();
        match invalid {
            0 => game.area.items.get_mut(tool).unwrap().removed = true,
            1 => game.physics.edit_body(body, |body| body.set_enabled(false)),
            2 => game.area.items.get_mut(tool).unwrap().body = None,
            _ => game.area.items.get_mut(tool).unwrap().location = ItemLocation::Attached(other),
        }
        let before = capture_ownership(&game, tool);
        assert!(!game.area.equip_carried(&mut game.physics).unwrap());
        assert_eq!(capture_ownership(&game, tool), before);
        assert!(
            !game
                .area
                .apply_inventory_command(InventoryCommand::Select(1), &reference, &mut game.physics)
                .unwrap()
        );
        assert_eq!(capture_ownership(&game, tool), before);
        game.area.items.get_mut(tool).unwrap().removed = false;
        game.area.items.get_mut(tool).unwrap().body = Some(body);
        game.area.items.get_mut(tool).unwrap().location = ItemLocation::Held;
        game.physics.edit_body(body, |body| body.set_enabled(true));
    }
    // A stored destination with a retired body must not change selection or ownership.
    let selected = game.area.inventory.selected;
    assert!(
        game.area
            .apply_inventory_command(
                InventoryCommand::Select((selected + 1) % 4),
                &reference,
                &mut game.physics
            )
            .unwrap()
    );
    let body = game.area.items.get(tool).unwrap().body.unwrap();
    game.physics.remove_body(body);
    let before = capture_ownership(&game, tool);
    assert!(
        !game
            .area
            .apply_inventory_command(
                InventoryCommand::Select(selected),
                &reference,
                &mut game.physics
            )
            .unwrap()
    );
    assert_eq!(capture_ownership(&game, tool), before);
}

#[test]
#[ignore = "Requires the installed runtime export"]
fn g08_transfer_hand_pose_is_frame_timed() {
    use the_long_slop::{
        reference::ItemId,
        vehicle::{VehiclePose, to_rapier},
    };
    let (reference, _, mut game) = fixture();
    let tool = ItemId(128);
    pickup(&mut game, tool);
    game.area.sync_equipment(&reference, &mut game.physics);
    let body = game.area.items.get(tool).unwrap().body.unwrap();
    let handles = game.physics.rapier().bodies[body].colliders().to_vec();
    assert!(!handles.is_empty());
    game.physics
        .edit_collider(handles[0], |collider| collider.set_enabled(false));
    let enabled: Vec<_> = handles
        .iter()
        .map(|handle| game.physics.rapier().colliders[*handle].is_enabled())
        .collect();
    let rule = game.area.interaction.pickup_rule(tool).unwrap().clone();
    let offset = if rule.disable_rh_fix {
        VehiclePose {
            position: [0.; 3],
            rotation: [0., 0., 0., 1.],
        }
    } else {
        VehiclePose {
            position: rule.right_hand_position,
            rotation: rule.right_hand_rotation,
        }
    };
    let target = to_rapier(
        game.area
            .pose
            .anchor(reference.clothing.pose.anchors["RH"].node)
            .unwrap(),
    ) * to_rapier(offset)
        * to_rapier(game.area.items.get(tool).unwrap().local_pose).inverse();
    let initial = *game.physics.rapier().bodies[body].position();
    assert!(game.area.equip_carried(&mut game.physics).unwrap());
    game.area.sync_equipment(&reference, &mut game.physics);
    assert!(
        (game.physics.rapier().bodies[body].position().translation - initial.translation).length()
            < 0.0001
    );
    let dt = 0.01;
    game.area.interaction.equipment.advance_hand(
        dt,
        &game.area.pose,
        &game.area.items,
        &rule,
        &mut game.physics,
    );
    let sample = *game.physics.rapier().bodies[body].position();
    let sample_rotation = initial.rotation.lerp(
        target.rotation,
        dt * reference.inventory.hand_interpolation_factor,
    );
    assert!(sample.rotation.dot(sample_rotation).abs() > 0.99999);
    game.area.sync_equipment(&reference, &mut game.physics);
    assert!(
        (game.physics.rapier().bodies[body].position().translation - sample.translation).length()
            < 0.0001
    );
    let expected = initial.translation.lerp(
        target.translation,
        dt * reference.inventory.hand_interpolation_factor,
    );
    assert!((sample.translation - expected).length() < 0.0001);
    assert!(
        (sample.translation - target.translation).length()
            < (initial.translation - target.translation).length()
    );
    game.area.interaction.equipment.advance_hand(
        reference.inventory.hand_interpolation_seconds,
        &game.area.pose,
        &game.area.items,
        &rule,
        &mut game.physics,
    );
    assert!(
        (game.physics.rapier().bodies[body].position().translation - target.translation).length()
            < 0.0001
    );
    assert!(
        game.physics.rapier().bodies[body]
            .position()
            .rotation
            .dot(target.rotation)
            .abs()
            > 0.99999
    );
    assert!(game.area.release_equipped(&mut game.physics).unwrap());
    assert_eq!(
        handles
            .iter()
            .map(|handle| game.physics.rapier().colliders[*handle].is_enabled())
            .collect::<Vec<_>>(),
        enabled
    );
    game.physics
        .edit_collider(handles[0], |collider| collider.set_enabled(true));
    assert!(game.area.equip_carried(&mut game.physics).unwrap());
    game.area.interaction.equipment.advance_hand(
        0.01,
        &game.area.pose,
        &game.area.items,
        &rule,
        &mut game.physics,
    );
    assert!(game.area.release_equipped(&mut game.physics).unwrap());
    assert!(game.physics.rapier().colliders[handles[0]].is_enabled());
    let selected = game.area.inventory.selected;
    game.area
        .apply_inventory_command(
            InventoryCommand::Select((selected + 1) % 4),
            &reference,
            &mut game.physics,
        )
        .unwrap();
    game.area
        .apply_inventory_command(
            InventoryCommand::Select(selected),
            &reference,
            &mut game.physics,
        )
        .unwrap();
    assert!(
        (game.physics.rapier().bodies[body].position().translation - target.translation).length()
            < 0.0001
    );
    assert!(
        game.physics.rapier().bodies[body]
            .position()
            .rotation
            .dot(target.rotation)
            .abs()
            > 0.99999
    );
    let immediate = *game.physics.rapier().bodies[body].position();
    game.area.interaction.equipment.advance_hand(
        0.01,
        &game.area.pose,
        &game.area.items,
        &rule,
        &mut game.physics,
    );
    assert_eq!(*game.physics.rapier().bodies[body].position(), immediate);
}

fn tool_frame(
    _game: &GarageRuntime,
    view: the_long_slop::vehicle::VehiclePose,
) -> the_long_slop::interaction::cleaning::ToolFrame {
    the_long_slop::interaction::cleaning::ToolFrame {
        lmb: true,
        active: true,
        dt: 0.04,
        view,
        origin: [0.; 3],
    }
}

fn tool_surface(
    game: &mut GarageRuntime,
    reference: &ReferenceManifest,
    exclude: Option<the_long_slop::reference::ConditionId>,
    tool: u8,
) -> (
    the_long_slop::reference::ConditionId,
    the_long_slop::vehicle::VehiclePose,
    rapier3d::prelude::ColliderHandle,
) {
    use rapier3d::math::{Rotation, Vector};
    let (target, handle, bounds) = game
        .physics
        .rapier()
        .colliders
        .iter()
        .filter_map(|(handle, collider)| {
            let id = the_long_slop::reference::ColliderId((collider.user_data >> 64) as u32);
            let binding = reference
                .conditions
                .collider_bindings
                .iter()
                .find(|b| b.collider == id)?;
            let target = binding.ancestor_condition?;
            let bounds = collider.compute_aabb();
            let size = bounds.maxs - bounds.mins;
            (Some(target) != exclude
                && collider.is_enabled()
                && !collider.is_sensor()
                && collider.collision_groups().memberships.bits()
                    & reference.conditions.brushes[0].mask
                    != 0
                && game.area.conditions.can_repair(target, tool)
                && size.x > if tool == 0 { 0.1 } else { 0.6 }
                && size.z > if tool == 0 { 0.1 } else { 0.6 })
            .then_some((target, handle, bounds))
        })
        .max_by(|a, b| {
            let area = |v: &(_, _, rapier3d::parry::bounding_volume::Aabb)| {
                (v.2.maxs.x - v.2.mins.x) * (v.2.maxs.z - v.2.mins.z)
            };
            area(a).total_cmp(&area(b))
        })
        .expect("exported repairable contact surface");
    let center = (bounds.mins + bounds.maxs) * 0.5;
    let handles: Vec<_> = game
        .physics
        .rapier()
        .colliders
        .iter()
        .map(|(h, _)| h)
        .collect();
    // Keep the real exported contact geometry. Exclude unrelated occluders.
    for other in handles {
        game.physics
            .edit_collider(other, |c| c.set_enabled(other == handle));
    }
    game.physics.refresh_queries();
    (
        target,
        the_long_slop::vehicle::VehiclePose {
            position: [center.x, bounds.maxs.y + 0.5, center.z],
            rotation: Rotation::from_rotation_arc(-Vector::Z, -Vector::Y).to_array(),
        },
        handle,
    )
}

#[test]
#[ignore = "Requires the installed runtime export"]
fn g08_tools_require_the_correct_owner() {
    use the_long_slop::{
        reference::ItemId,
        vehicle::{from_rapier, to_rapier},
    };
    let (reference, _, mut game) = fixture();
    game.started = true;
    game.paused = false;
    let brush = ItemId(128);
    pickup(&mut game, brush);
    let (target, view, _) = tool_surface(&mut game, &reference, None, 1);
    let before = game.area.conditions.get(target).repair_progress;
    let quantity = game.area.interaction.cleaning.get(brush).unwrap().quantity;
    for i in 0..8 {
        let mut v = view;
        v.position[0] += if i % 2 == 0 { -0.2 } else { 0.2 };
        let f = tool_frame(&game, v);
        advance_tool_owners(&mut game, &reference, f);
    }
    assert_eq!(
        game.area.conditions.get(target).repair_progress,
        before,
        "carried brush cannot repair"
    );
    assert_eq!(
        game.area.interaction.cleaning.get(brush).unwrap().quantity,
        quantity
    );
    // Restore the brush body, disabled only by this test's occluder isolation.
    let body = game.area.items.get(brush).unwrap().body.unwrap();
    game.physics.edit_body(body, |b| b.set_enabled(true));
    assert!(game.area.equip_carried(&mut game.physics).unwrap());
    for i in 0..8 {
        let mut v = view;
        v.position[0] += if i % 2 == 0 { -0.2 } else { 0.2 };
        let f = tool_frame(&game, v);
        advance_tool_owners(&mut game, &reference, f);
    }
    assert_ne!(
        game.area.conditions.get(target).repair_progress,
        before,
        "equipped brush repairs after rubbing"
    );
    // Owner phases consume retained contact force. The model and audio update only in the tail.
    use the_long_slop::interaction::cleaning::ToolOwner;
    let frame = tool_frame(&game, view);
    let model_before = to_rapier(
        game.area
            .interaction
            .cleaning
            .model_pose(0, &game.area.items),
    );
    let gain_before = game.area.interaction.cleaning.signals()[0].voice_gain;
    assert!(gain_before > 0.);
    game.area.interaction.cleaning.begin_frame(frame);
    assert!(game.area.interaction.cleaning.advance_owner(
        ToolOwner::Equipped(brush),
        frame,
        &reference,
        &mut game.area.conditions,
        &game.area.items,
        &game.physics,
        &game.area.interaction.parts
    ));
    assert_eq!(
        to_rapier(
            game.area
                .interaction
                .cleaning
                .model_pose(0, &game.area.items)
        ),
        model_before
    );
    assert_eq!(
        game.area.interaction.cleaning.signals()[0].voice_gain,
        gain_before
    );
    assert!(game.area.release_equipped(&mut game.physics).unwrap());
    assert!(!game.area.interaction.cleaning.advance_owner(
        ToolOwner::Carried(brush),
        frame,
        &reference,
        &mut game.area.conditions,
        &game.area.items,
        &game.physics,
        &game.area.interaction.parts
    ));
    assert_eq!(
        to_rapier(
            game.area
                .interaction
                .cleaning
                .model_pose(0, &game.area.items)
        ),
        model_before
    );
    assert_eq!(
        game.area.interaction.cleaning.signals()[0].voice_gain,
        gain_before
    );
    let rule = &reference.conditions.brushes[0];
    let original_root = to_rapier(
        the_long_slop::interaction::items::node_pose(
            &reference,
            reference.items.iter().find(|i| i.id == brush).unwrap().node,
        )
        .unwrap(),
    );
    let equip = original_root.inverse()
        * to_rapier(
            the_long_slop::interaction::items::node_pose(&reference, rule.model_equip).unwrap(),
        );
    let default = original_root.inverse()
        * to_rapier(
            the_long_slop::interaction::items::node_pose(&reference, rule.model_default).unwrap(),
        );
    let root = to_rapier(game.area.items.get(brush).unwrap().pose);
    let expected_position = model_before.translation.lerp(
        (root * equip).translation,
        (rule.position_lerp * frame.dt).clamp(0., 1.),
    );
    let expected_rotation = model_before.rotation.slerp(
        (root * default).rotation,
        (rule.rotation_lerp * frame.dt).clamp(0., 1.),
    );
    game.area.interaction.cleaning.finish_frame(
        frame,
        &reference,
        &mut game.area.conditions,
        &game.area.items,
        &game.physics,
        &game.area.interaction.parts,
    );
    let model_after = to_rapier(
        game.area
            .interaction
            .cleaning
            .model_pose(0, &game.area.items),
    );
    assert!((model_after.translation - expected_position).length() < 0.00001);
    assert!(model_after.rotation.dot(expected_rotation).abs() > 0.99999);
    assert_eq!(game.area.interaction.cleaning.signals()[0].voice_gain, 0.);
    assert!(!game.area.interaction.cleaning.signals()[0].playing);
    assert!(game.area.equip_carried(&mut game.physics).unwrap());
    for i in 0..4 {
        let mut v = view;
        v.position[0] += if i % 2 == 0 { -0.2 } else { 0.2 };
        let f = tool_frame(&game, v);
        advance_tool_owners(&mut game, &reference, f);
    }
    let progress = game.area.conditions.get(target).repair_progress;
    game.paused = true;
    let f = tool_frame(&game, view);
    advance_tool_owners(&mut game, &reference, f);
    assert!(
        game.area
            .interaction
            .cleaning
            .signals()
            .iter()
            .all(|s| !s.playing && s.force == 0.)
    );
    game.paused = false;
    advance_tool_owners(&mut game, &reference, f);
    assert_eq!(game.area.conditions.get(target).repair_progress, progress);
    let selected = game.area.inventory.selected;
    assert!(
        game.area
            .apply_inventory_command(
                InventoryCommand::Select((selected + 1) % 4),
                &reference,
                &mut game.physics
            )
            .unwrap()
    );
    let f = tool_frame(&game, view);
    advance_tool_owners(&mut game, &reference, f);
    assert!(
        game.area
            .interaction
            .cleaning
            .signals()
            .iter()
            .all(|s| !s.playing && s.force == 0. && !s.doing_work)
    );
    assert!(
        game.area
            .apply_inventory_command(
                InventoryCommand::Select(selected),
                &reference,
                &mut game.physics
            )
            .unwrap()
    );
    assert!(game.area.release_equipped(&mut game.physics).unwrap());
    let f = tool_frame(&game, view);
    advance_tool_owners(&mut game, &reference, f);
    assert!(
        game.area
            .interaction
            .cleaning
            .signals()
            .iter()
            .all(|s| !s.playing && s.force == 0.)
    );
    game.area
        .interaction
        .drop_held(&mut game.area.items, &mut game.physics);
    let spray = reference.conditions.sprays[0].item;
    let body = game.area.items.get(spray).unwrap().body.unwrap();
    game.physics.edit_body(body, |b| b.set_enabled(true));
    pickup(&mut game, spray);
    assert!(game.area.equip_carried(&mut game.physics).unwrap());
    // Each owner aims at a different existing surface. Re-enable geometry to choose the second surface.
    let handles: Vec<_> = game
        .physics
        .rapier()
        .colliders
        .iter()
        .map(|(h, _)| h)
        .collect();
    for h in handles {
        game.physics.edit_collider(h, |c| c.set_enabled(true));
    }
    let (target, view, head_handle) = tool_surface(&mut game, &reference, None, 0);
    let handles: Vec<_> = game
        .physics
        .rapier()
        .colliders
        .iter()
        .map(|(h, _)| h)
        .collect();
    for h in handles {
        game.physics.edit_collider(h, |c| c.set_enabled(true));
    }
    let (nozzle_target, mut nozzle_view, nozzle_handle) =
        tool_surface(&mut game, &reference, Some(target), 0);
    let collider = &game.physics.rapier().colliders[nozzle_handle];
    let mut relocated = *collider.position();
    relocated.translation.x += 10.;
    let parent = *game.physics.rapier().bodies[collider.parent().unwrap()].position();
    game.physics.edit_collider(nozzle_handle, |collider| {
        collider.set_position_wrt_parent(parent.inverse() * relocated)
    });
    nozzle_view.position[0] += 10.;
    game.physics
        .edit_collider(head_handle, |collider| collider.set_enabled(true));
    game.physics.refresh_queries();
    // The nozzle transform is relative to the exported item root.
    let root = to_rapier(
        the_long_slop::interaction::items::node_pose(
            &reference,
            reference.items.iter().find(|i| i.id == spray).unwrap().node,
        )
        .unwrap(),
    );
    let nozzle = root.inverse()
        * to_rapier(
            the_long_slop::interaction::items::node_pose(
                &reference,
                reference.conditions.sprays[0].nozzle,
            )
            .unwrap(),
        );
    let spray_pose = to_rapier(nozzle_view) * nozzle.inverse();
    game.area.items.get_mut(spray).unwrap().pose = from_rapier(&spray_pose);
    game.physics
        .edit_body(body, |body| body.set_position(spray_pose, true));
    let head_before = game.area.conditions.get(target).color;
    let nozzle_before = game.area.conditions.get(nozzle_target).color;
    let f = tool_frame(&game, view);
    advance_tool_owners(&mut game, &reference, f);
    assert_ne!(
        game.area.conditions.get(target).color,
        head_before,
        "equipped spray uses head ray"
    );
    assert_eq!(game.area.conditions.get(nozzle_target).color, nozzle_before);
    assert!(game.area.release_equipped(&mut game.physics).unwrap());
    let head_before = game.area.conditions.get(target).color;
    let f = tool_frame(&game, view);
    advance_tool_owners(&mut game, &reference, f);
    assert_eq!(game.area.conditions.get(target).color, head_before);
    assert_ne!(
        game.area.conditions.get(nozzle_target).color,
        nozzle_before,
        "carried spray uses nozzle ray"
    );
}

#[test]
#[ignore = "Requires the installed runtime export"]
fn g08_tools_two_sprays_and_one_shared_tail() {
    let (reference, _, mut game) = fixture();
    game.started = true;
    game.paused = false;
    let equipped = reference.conditions.sprays[0].item;
    let carried = reference.conditions.sprays[1].item;
    pickup(&mut game, equipped);
    assert!(game.area.equip_carried(&mut game.physics).unwrap());
    pickup(&mut game, carried);
    let view = game.area.items.get(equipped).unwrap().pose;
    let quantities = [equipped, carried].map(|id| {
        game.area
            .interaction
            .cleaning
            .get(id)
            .unwrap()
            .quantity
            .unwrap()
    });
    let frame = tool_frame(&game, view);
    advance_tool_owners(&mut game, &reference, frame);
    for (i, id) in [equipped, carried].into_iter().enumerate() {
        let tool = game.area.interaction.cleaning.get(id).unwrap();
        let rule = &reference.conditions.sprays[i];
        let valve = (rule.valve_speed * frame.dt).clamp(0., 1.);
        assert!((tool.valve - valve).abs() < 0.00001);
        assert!(
            (quantities[i] - tool.quantity.unwrap() - rule.paint_loss * valve * frame.dt).abs()
                < 0.00001
        );
        let signal = game
            .area
            .interaction
            .cleaning
            .signals()
            .iter()
            .find(|s| s.item == id)
            .unwrap();
        assert!((signal.emission[0] - valve).abs() < 0.00001);
    }
    let previous = game
        .area
        .interaction
        .cleaning
        .spray_particles(0, 0)
        .first()
        .expect("spray emitted particles")
        .age;
    advance_tool_owners(&mut game, &reference, frame);
    let particle = &game.area.interaction.cleaning.spray_particles(0, 0)[0];
    assert!((particle.age - previous - 0.04).abs() < 0.00001);
    let open_valve = game.area.interaction.cleaning.get(equipped).unwrap().valve;
    let mut released_frame = frame;
    released_frame.lmb = false;
    advance_tool_owners(&mut game, &reference, released_frame);
    let tool = game.area.interaction.cleaning.get(equipped).unwrap();
    assert!((tool.valve - open_valve * 0.6).abs() < 0.00001);
    let signal = game
        .area
        .interaction
        .cleaning
        .signals()
        .iter()
        .find(|s| s.item == equipped)
        .unwrap();
    assert!((signal.emission[0] - open_valve).abs() < 0.00001);
    // A release can expose one spray to both use phases. Each Fire call consumes paint.
    use the_long_slop::interaction::cleaning::ToolOwner;
    let before_quantity = tool.quantity.unwrap();
    let mut expected_valve = tool.valve;
    let previous_age = game.area.interaction.cleaning.spray_particles(0, 0)[0].age;
    let mut expected_loss = 0.;
    game.area.interaction.cleaning.begin_frame(frame);
    for owner in [ToolOwner::Equipped(equipped), ToolOwner::Carried(equipped)] {
        expected_valve += (1. - expected_valve) * 0.4;
        expected_loss += reference.conditions.sprays[0].paint_loss * expected_valve * frame.dt;
        assert!(!game.area.interaction.cleaning.advance_owner(
            owner,
            frame,
            &reference,
            &mut game.area.conditions,
            &game.area.items,
            &game.physics,
            &game.area.interaction.parts
        ));
    }
    game.area.interaction.cleaning.finish_frame(
        frame,
        &reference,
        &mut game.area.conditions,
        &game.area.items,
        &game.physics,
        &game.area.interaction.parts,
    );
    let tool = game.area.interaction.cleaning.get(equipped).unwrap();
    assert!((before_quantity - tool.quantity.unwrap() - expected_loss).abs() < 0.00001);
    assert!((tool.valve - expected_valve).abs() < 0.00001);
    assert!(
        (game.area.interaction.cleaning.spray_particles(0, 0)[0].age - previous_age - frame.dt)
            .abs()
            < 0.00001
    );
    game.paused = true;
    advance_tool_owners(&mut game, &reference, frame);
    assert!(
        game.area
            .interaction
            .cleaning
            .signals()
            .iter()
            .all(|s| !s.playing && s.emission == [0.; 2] && !s.doing_work)
    );
    game.paused = false;
    game.area
        .interaction
        .drop_held(&mut game.area.items, &mut game.physics);
    assert!(game.area.release_equipped(&mut game.physics).unwrap());
    game.area
        .interaction
        .drop_held(&mut game.area.items, &mut game.physics);
    let frame = tool_frame(&game, view);
    advance_tool_owners(&mut game, &reference, frame);
    assert!(
        game.area
            .interaction
            .cleaning
            .signals()
            .iter()
            .all(|s| !s.playing && !s.doing_work && s.force == 0.)
    );
    pickup(&mut game, equipped);
    assert!(game.area.equip_carried(&mut game.physics).unwrap());
    let frame = tool_frame(&game, view);
    advance_tool_owners(&mut game, &reference, frame);
    let other = (game.area.inventory.selected + 1) % 4;
    game.area
        .apply_inventory_command(
            InventoryCommand::Select(other),
            &reference,
            &mut game.physics,
        )
        .unwrap();
    let frame = tool_frame(&game, view);
    advance_tool_owners(&mut game, &reference, frame);
    assert!(
        game.area
            .interaction
            .cleaning
            .signals()
            .iter()
            .all(|s| !s.playing && !s.doing_work && s.force == 0.)
    );
}

// Local owner phases isolate tools. Library frame tests cover runtime guards.
fn advance_tool_owners(
    game: &mut GarageRuntime,
    reference: &ReferenceManifest,
    mut frame: the_long_slop::interaction::cleaning::ToolFrame,
) {
    use the_long_slop::interaction::cleaning::ToolOwner;
    frame.active &= game.started && !game.paused && !game.needs_output().dead && !game.sleeping();
    game.area
        .interaction
        .cleaning
        .sync_wash_colliders(&game.area.conditions, &mut game.physics);
    game.area.interaction.cleaning.begin_frame(frame);
    for owner in [
        game.area
            .interaction
            .equipment
            .item()
            .map(ToolOwner::Equipped),
        game.area.interaction.held().map(ToolOwner::Carried),
    ]
    .into_iter()
    .flatten()
    {
        game.area.interaction.cleaning.advance_owner(
            owner,
            frame,
            reference,
            &mut game.area.conditions,
            &game.area.items,
            &game.physics,
            &game.area.interaction.parts,
        );
    }
    game.area.interaction.cleaning.finish_frame(
        frame,
        reference,
        &mut game.area.conditions,
        &game.area.items,
        &game.physics,
        &game.area.interaction.parts,
    );
    game.area
        .interaction
        .cleaning
        .sync_wash_colliders(&game.area.conditions, &mut game.physics);
}

#[test]
#[ignore = "Requires the installed runtime export"]
fn g08_frame_context_hints_match_available_actions() {
    use the_long_slop::{interaction::targets::ResolvedAim, reference::ItemId};
    const EQUIP: u32 = 1 << 24;
    const RECENTER: u32 = 1 << 25;
    const RELEASE: u32 = 1 << 26;
    const SEATED_RELEASE: u32 = 1 << 27;
    fn actions(game: &GarageRuntime) -> u32 {
        game.area
            .interaction
            .available_actions(
                &ResolvedAim::default(),
                &game.area.player,
                &game.area.items,
                &game.physics,
                game.needs_output(),
            )
            .actions
            .0
    }
    let (reference, _, mut game) = fixture();
    assert_eq!(
        actions(&game) & (EQUIP | RECENTER | RELEASE | SEATED_RELEASE),
        0
    );
    pickup(&mut game, ItemId(128));
    assert_eq!(
        actions(&game) & (EQUIP | RECENTER),
        EQUIP | RECENTER,
        "eligible carry advertises equip and recenter"
    );
    assert_eq!(actions(&game) & (RELEASE | SEATED_RELEASE), 0);
    let carried = ItemId(128);
    let body = game.area.items.get(carried).unwrap().body.unwrap();
    for invalid in 0..5 {
        match invalid {
            0 => game.physics.edit_body(body, |body| body.set_enabled(false)),
            1 => game.area.items.get_mut(carried).unwrap().body = None,
            2 => game.area.items.get_mut(carried).unwrap().removed = true,
            3 => game.area.items.get_mut(carried).unwrap().pending_attachment = true,
            _ => game.physics.edit_body(body, |body| {
                body.set_body_type(
                    rapier3d::prelude::RigidBodyType::KinematicPositionBased,
                    false,
                )
            }),
        }
        assert_eq!(
            actions(&game) & EQUIP,
            0,
            "invalid carried state {invalid} cannot advertise equip"
        );
        assert!(!game.area.equip_carried(&mut game.physics).unwrap());
        let item = game.area.items.get_mut(carried).unwrap();
        item.body = Some(body);
        item.removed = false;
        item.pending_attachment = false;
        game.physics.edit_body(body, |body| {
            body.set_enabled(true);
            body.set_body_type(rapier3d::prelude::RigidBodyType::Dynamic, false);
        });
    }
    assert_eq!(actions(&game) & EQUIP, EQUIP);
    assert!(game.area.equip_carried(&mut game.physics).unwrap());
    assert_eq!(
        actions(&game) & (EQUIP | RECENTER | RELEASE | SEATED_RELEASE),
        RELEASE
    );
    pickup(&mut game, ItemId(130));
    assert_eq!(
        actions(&game) & (EQUIP | RECENTER | RELEASE | SEATED_RELEASE),
        RECENTER
    );
    game.area
        .interaction
        .drop_held(&mut game.area.items, &mut game.physics);
    let seat = &reference.seats[0];
    let anchor = reference
        .anchors
        .iter()
        .find(|a| a.id == seat.anchor)
        .unwrap();
    assert!(game.area.player.enter(
        seat.id,
        the_long_slop::vehicle::VehiclePose {
            position: anchor.transform.translation,
            rotation: anchor.transform.rotation,
        },
        &mut game.physics
    ));
    assert_eq!(
        actions(&game) & (EQUIP | RECENTER | RELEASE | SEATED_RELEASE),
        SEATED_RELEASE
    );
    let body = game.area.items.get(ItemId(128)).unwrap().body.unwrap();
    game.physics.edit_body(body, |body| body.set_enabled(false));
    assert_eq!(
        actions(&game) & (RELEASE | SEATED_RELEASE),
        0,
        "invalid equipment cannot release"
    );
    let (_, _, mut game) = fixture();
    pickup(&mut game, ItemId(120));
    assert_eq!(
        actions(&game) & (EQUIP | RECENTER | RELEASE | SEATED_RELEASE),
        RECENTER,
        "ineligible carry cannot equip"
    );
}
