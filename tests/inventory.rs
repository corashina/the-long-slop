//! Critical ownership check against the installed export, without test asset fixtures.
use my_summer_slop::{
    inventory::InventoryCommand,
    reference::ReferenceManifest,
    save::{CameraSave, SourceHash, SourceStamp},
    simulation::FrameInput,
    starting_area::GarageRuntime,
};
#[test]
#[ignore = "Requires the installed runtime export"]
fn source_transfers_reject_atomically_and_restore_both_hands() {
    let bytes = std::fs::read("web/assets/reference/manifest.json").unwrap();
    let r = ReferenceManifest::from_json(&bytes).unwrap();
    let digest = std::process::Command::new("sha256sum")
        .arg("web/assets/reference/manifest.json")
        .output()
        .unwrap();
    assert!(digest.status.success());
    let stamp = SourceStamp {
        app_id: r.app_id,
        build_id: r.build_id,
        manifest_sha256: String::from_utf8(digest.stdout)
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap()
            .into(),
        source_hashes: r
            .source_hashes
            .iter()
            .map(|h| SourceHash {
                file: h.file.clone(),
                sha256: h.sha256.clone(),
            })
            .collect(),
    };
    let mut game = GarageRuntime::new(&r).unwrap();
    let oil = r.items.iter().find(|i| i.name == "OilCan").unwrap().id;
    let light = r
        .flashlights
        .instances
        .iter()
        .find(|f| f.startup_active)
        .unwrap()
        .item;
    let food = r.survival.starter_food;
    for (id, select) in [(oil, 1), (light, 0)] {
        let point = game.area.items.get(id).unwrap().pose.position;
        if game.area.interaction.storage.stored(id) {
            game.area
                .interaction
                .pickup_stored_item(
                    id,
                    point,
                    &game.area.player,
                    &mut game.area.items,
                    &mut game.physics,
                )
                .unwrap();
        } else {
            game.area
                .interaction
                .pickup_item(
                    id,
                    point,
                    &game.area.player,
                    &mut game.area.items,
                    &mut game.physics,
                )
                .unwrap();
        }
        game.area.sync_inventory();
        assert!(
            game.area
                .apply_inventory_command(InventoryCommand::Select(select), &r, &mut game.physics)
                .unwrap()
        );
    }
    assert!(
        game.area
            .apply_inventory_command(InventoryCommand::Select(1), &r, &mut game.physics)
            .unwrap()
    );
    assert_eq!(game.area.interaction.equipment.item(), Some(light));
    let point = game.area.items.get(food).unwrap().pose.position;
    game.area
        .interaction
        .pickup_item(
            food,
            point,
            &game.area.player,
            &mut game.area.items,
            &mut game.physics,
        )
        .unwrap();
    game.area.sync_inventory();
    let handle = game.area.items.get(food).unwrap().body.unwrap();
    let before = (
        *game.physics.rapier().bodies[handle].position(),
        game.physics.rapier().bodies[handle].linvel(),
        game.physics.rapier().impulse_joints.len(),
        game.area.inventory.slots,
    );
    for slot in [0, 1] {
        assert!(
            !game
                .area
                .apply_inventory_command(InventoryCommand::Select(slot), &r, &mut game.physics)
                .unwrap()
        );
    }
    assert_eq!(
        before,
        (
            *game.physics.rapier().bodies[handle].position(),
            game.physics.rapier().bodies[handle].linvel(),
            game.physics.rapier().impulse_joints.len(),
            game.area.inventory.slots
        )
    );
    assert!(game.consume(&r, food).unwrap());
    let saved = game
        .capture_save(&r, &stamp, CameraSave::FirstPerson, "2026-10-15T12:00:00Z")
        .unwrap();
    let mut loaded = GarageRuntime::from_save(&r, &stamp, &saved).unwrap();
    let recaptured = loaded
        .capture_save(&r, &stamp, CameraSave::FirstPerson, "2026-10-15T12:00:00Z")
        .unwrap();
    for (before, after) in saved.items.iter().zip(&recaptured.items) {
        if matches!(
            before.location,
            my_summer_slop::save::ItemLocationSave::Stored { .. }
        ) {
            assert_eq!(before.instance, after.instance);
            for axis in 0..3 {
                assert!((before.pose.position[axis] - after.pose.position[axis]).abs() < 0.0001);
            }
        }
    }
    assert_eq!(loaded.area.inventory.slots, game.area.inventory.slots);
    assert_eq!(loaded.area.interaction.held(), Some(food));
    assert_eq!(loaded.area.interaction.equipment.item(), Some(light));
    assert_eq!(loaded.area.items.get(food).unwrap().food.unwrap().bite, 1);
    let oil_body = loaded.area.items.get(oil).unwrap().body.unwrap();
    assert!(!loaded.physics.rapier().bodies[oil_body].is_enabled());
    let mut invalid = saved.clone();
    invalid.inventory.slots[0] = invalid.inventory.slots[1].clone();
    assert!(invalid.validate(&r, &stamp).is_err());
    let mut invalid = saved.clone();
    invalid.inventory.equipped = invalid.player.carry.as_ref().map(|c| c.item.clone());
    assert!(invalid.validate(&r, &stamp).is_err());
    let mut invalid = saved.clone();
    invalid.schema_version = 1;
    assert!(invalid.validate(&r, &stamp).is_err());
    loaded.started = true;
    loaded.paused = false;
    loaded.step(&r, FrameInput::default());
    assert!(!loaded.physics.rapier().bodies[oil_body].is_enabled());
    assert_eq!(
        loaded
            .area
            .items
            .get(oil)
            .unwrap()
            .fluid
            .as_ref()
            .unwrap()
            .mixture
            .quantities_liters,
        game.area
            .items
            .get(oil)
            .unwrap()
            .fluid
            .as_ref()
            .unwrap()
            .mixture
            .quantities_liters
    );
    // Stored food retains its bite, and exhausted food does not return on load.
    loaded
        .area
        .interaction
        .drop_held(&mut loaded.area.items, &mut loaded.physics);
    loaded.area.sync_inventory();
    loaded
        .area
        .apply_inventory_command(InventoryCommand::Select(0), &r, &mut loaded.physics)
        .unwrap();
    loaded
        .area
        .apply_inventory_command(InventoryCommand::Select(2), &r, &mut loaded.physics)
        .unwrap();
    let point = loaded.area.items.get(food).unwrap().pose.position;
    loaded
        .area
        .interaction
        .pickup_item(
            food,
            point,
            &loaded.area.player,
            &mut loaded.area.items,
            &mut loaded.physics,
        )
        .unwrap();
    loaded.area.sync_inventory();
    loaded
        .area
        .apply_inventory_command(InventoryCommand::Select(3), &r, &mut loaded.physics)
        .unwrap();
    let stored = loaded
        .capture_save(&r, &stamp, CameraSave::FirstPerson, "2026-10-15T12:00:01Z")
        .unwrap();
    let mut restored = GarageRuntime::from_save(&r, &stamp, &stored).unwrap();
    assert_eq!(restored.area.items.get(food).unwrap().food.unwrap().bite, 1);
    restored
        .area
        .apply_inventory_command(InventoryCommand::Select(2), &r, &mut restored.physics)
        .unwrap();
    let bites = r.survival.foods[restored
        .area
        .items
        .get(food)
        .unwrap()
        .food
        .unwrap()
        .prototype_index]
        .bites;
    for _ in 1..bites {
        assert!(restored.consume(&r, food).unwrap());
    }
    assert!(!restored.area.inventory.slots.contains(&Some(food)));
    let removed = restored
        .capture_save(&r, &stamp, CameraSave::FirstPerson, "2026-10-15T12:00:02Z")
        .unwrap();
    let restored = GarageRuntime::from_save(&r, &stamp, &removed).unwrap();
    assert!(restored.area.items.get(food).unwrap().removed);
}

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
    let runtime = GarageRuntime::new(&reference).unwrap();
    (reference, stamp, runtime)
}

fn pickup(runtime: &mut GarageRuntime, item: my_summer_slop::reference::ItemId) {
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
    item: my_summer_slop::reference::ItemId,
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

fn contents(runtime: &GarageRuntime, item: my_summer_slop::reference::ItemId) -> String {
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
    use my_summer_slop::{interaction::items::ItemLocation, reference::ItemId};
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
    use my_summer_slop::{
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
