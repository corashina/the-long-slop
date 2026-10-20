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
            .mixture()
            .quantities_liters,
        game.area
            .items
            .get(oil)
            .unwrap()
            .fluid
            .as_ref()
            .unwrap()
            .mixture()
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
