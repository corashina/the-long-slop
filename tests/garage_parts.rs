//! Critical G02 transactions against the installed build 11140890 export.
use my_summer_slop::{
    interaction::{InteractionAction, InteractionBlock, ResolvedAim, items::ItemLocation},
    reference::{ItemId, PartSlotId, ReferenceManifest},
    simulation::{FIXED_DT, FrameInput, keys},
    starting_area::GarageRuntime,
};

fn sync(run: &mut GarageRuntime) {
    run.area
        .interaction
        .sync_parts(&mut run.area.items, &mut run.physics)
        .unwrap();
    run.wheels
        .sync(&mut run.area.items, &mut run.physics)
        .unwrap();
}

fn remove(run: &mut GarageRuntime, item: ItemId) {
    let area = &mut run.area;
    area.interaction
        .parts
        .can_remove(item, &area.player, &area.items)
        .unwrap();
    area.interaction
        .parts
        .remove(item, &mut area.items, &mut run.physics)
        .unwrap();
    area.interaction.parts.detach_mechanism(
        item,
        &mut area.interaction.mechanisms,
        &mut run.physics,
    );
    sync(run);
}

fn install(run: &mut GarageRuntime, item: ItemId, slot: PartSlotId) {
    run.area
        .interaction
        .parts
        .install(item, slot, &mut run.area.items, &mut run.physics)
        .unwrap();
    run.area
        .interaction
        .parts
        .mount_mechanism(item, &mut run.area.interaction.mechanisms);
    sync(run);
}

fn near(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.005, "{actual} != {expected}");
}

#[test]
#[ignore = "requires the installed single-Lada export"]
fn lada_parts_retain_contents_mass_collision_and_identity() {
    let bytes = std::fs::read("web/assets/reference/manifest.json").unwrap();
    let r = ReferenceManifest::from_runtime_json(&bytes).unwrap();
    assert_eq!(r.build_id, 11140890);
    assert_eq!(r.vehicles.len(), 1);
    let mut run = GarageRuntime::new(&r).unwrap();
    let chassis = run.physics.chassis_handle(0);
    let initial_mass = run.physics.rapier().bodies[chassis].mass();
    let bodies = run.physics.rapier().bodies.len();
    let colliders = run.physics.rapier().colliders.len();
    let fluids: Vec<_> = r
        .tanks
        .iter()
        .map(|tank| {
            (
                tank.item,
                run.area.items.get(tank.item).unwrap().fluid.clone(),
            )
        })
        .collect();
    // Hubcaps and wheel assembly belong to G03.
    for part in r
        .parts
        .iter()
        .filter(|p| !p.part_types.iter().any(|t| t == "disztarcsa"))
    {
        let slot = r
            .part_slots
            .iter()
            .find(|s| s.part == Some(part.item))
            .unwrap();
        println!("cycle {:?}: {:?}", part.item, part.part_types);
        let handles: Vec<_> = run
            .physics
            .rapier()
            .colliders
            .iter()
            .filter(|(_, c)| {
                part.colliders
                    .iter()
                    .any(|id| id.0 == (c.user_data >> 64) as u32)
            })
            .map(|(h, c)| (h, c.is_enabled()))
            .collect();
        assert!(!handles.is_empty());
        for _ in 0..3 {
            remove(&mut run, part.item);
            let state = run.area.items.get(part.item).unwrap();
            assert_eq!(state.location, ItemLocation::World);
            let loose = state.body.unwrap();
            assert_ne!(loose, chassis);
            assert!(run.physics.rapier().bodies[loose].is_enabled());
            let detached_mass = run.physics.rapier().bodies[loose].mass();
            assert!(detached_mass > 0.0);
            near(
                run.physics.rapier().bodies[chassis].mass() + detached_mass,
                initial_mass,
            );
            for &(handle, enabled) in &handles {
                let collider = &run.physics.rapier().colliders[handle];
                // Plates keep their separate attachment transaction.
                if !r.plates.plates.iter().any(|plate| {
                    plate
                        .colliders
                        .iter()
                        .any(|id| id.0 == (collider.user_data >> 64) as u32)
                }) {
                    assert_eq!(collider.parent(), Some(loose));
                    assert_eq!(collider.is_enabled(), enabled);
                }
            }
            assert_eq!(run.area.interaction.parts.slot_item(slot.id), None);
            assert!(
                run.area
                    .interaction
                    .parts
                    .remove(part.item, &mut run.area.items, &mut run.physics)
                    .is_err()
            );
            install(&mut run, part.item, slot.id);
            assert!(!run.physics.rapier().bodies[loose].is_enabled());
            assert_eq!(run.area.items.get(part.item).unwrap().body, Some(chassis));
            assert_eq!(
                run.area.interaction.parts.slot_item(slot.id),
                Some(part.item)
            );
            assert!(
                run.area
                    .interaction
                    .parts
                    .install(part.item, slot.id, &mut run.area.items, &mut run.physics)
                    .is_err()
            );
            near(run.physics.rapier().bodies[chassis].mass(), initial_mass);
            assert_eq!(run.physics.rapier().bodies.len(), bodies);
            assert_eq!(run.physics.rapier().colliders.len(), colliders);
            for (id, fluid) in &fluids {
                assert_eq!(&run.area.items.get(*id).unwrap().fluid, fluid);
            }
        }
    }
    let engine = r.vehicles[0].engine.item;
    let coolant = r.vehicles[0].engine.coolant_tank;
    let engine_slot = r
        .part_slots
        .iter()
        .find(|s| s.part == Some(engine))
        .unwrap()
        .id;
    remove(&mut run, engine);
    remove(&mut run, coolant);
    assert_eq!(
        run.area
            .interaction
            .parts
            .can_install(coolant, engine_slot, &run.area.items),
        Err(InteractionBlock::Incompatible)
    );
    install(&mut run, engine, engine_slot);
    let coolant_slot = r
        .part_slots
        .iter()
        .find(|s| s.part == Some(coolant))
        .unwrap()
        .id;
    install(&mut run, coolant, coolant_slot);
    near(run.physics.rapier().bodies[chassis].mass(), initial_mass);
    // Lada seats are fixed source controls, not removable part objects.
    for seat in &r.seats {
        assert!(!run.area.interaction.parts.contains(seat.item));
        assert!(
            run.area
                .interaction
                .parts
                .can_remove(seat.item, &run.area.player, &run.area.items)
                .is_err()
        );
    }
}

#[test]
#[ignore = "requires the installed single-Lada export"]
fn lada_plates_retain_source_owners_without_duplicate_members() {
    let bytes = std::fs::read("web/assets/reference/manifest.json").unwrap();
    let r = ReferenceManifest::from_runtime_json(&bytes).unwrap();
    assert_eq!(r.vehicles.len(), 1);
    let mut run = GarageRuntime::new(&r).unwrap();
    let chassis = run.physics.chassis_handle(0);
    let initial_mass = run.physics.rapier().bodies[chassis].mass();
    for plate in &r.plates.plates {
        let target = r
            .plates
            .targets
            .iter()
            .find(|t| t.id == plate.startup_target)
            .unwrap();
        let owner_slot = r
            .part_slots
            .iter()
            .find(|s| s.part == Some(target.owner))
            .map(|s| s.id);
        for _ in 0..3 {
            if owner_slot.is_some() {
                remove(&mut run, target.owner);
            }
            let owner_body = run.area.items.get(target.owner).unwrap().body.unwrap();
            assert_eq!(
                run.area.items.get(plate.item).unwrap().body,
                Some(owner_body)
            );
            run.area
                .interaction
                .attachments
                .remove(plate.item, &mut run.area.items, &mut run.physics)
                .unwrap();
            sync(&mut run);
            let loose = run.area.items.get(plate.item).unwrap().body.unwrap();
            assert_ne!(loose, owner_body);
            assert!(run.physics.rapier().bodies[loose].is_enabled());
            near(run.physics.rapier().bodies[loose].mass(), plate.mass_kg);
            run.area
                .interaction
                .attachments
                .mount(plate.item, target.id, &mut run.area.items, &mut run.physics)
                .unwrap();
            sync(&mut run);
            assert_eq!(
                run.area
                    .interaction
                    .attachments
                    .members(target.id)
                    .filter(|id| *id == plate.item)
                    .count(),
                1
            );
            assert!(
                run.area
                    .interaction
                    .attachments
                    .mount(plate.item, target.id, &mut run.area.items, &mut run.physics)
                    .is_err()
            );
            if let Some(slot) = owner_slot {
                install(&mut run, target.owner, slot);
            }
            assert_eq!(run.area.items.get(plate.item).unwrap().body, Some(chassis));
            near(run.physics.rapier().bodies[chassis].mass(), initial_mass);
        }
    }
}

#[test]
#[ignore = "requires the installed single-Lada export"]
fn incompatible_part_scan_allows_source_drop_instead_of_trapping_the_part() {
    let bytes = std::fs::read("web/assets/reference/manifest.json").unwrap();
    let r = ReferenceManifest::from_runtime_json(&bytes).unwrap();
    let mut run = GarageRuntime::new(&r).unwrap();
    let bumper = |kind: &str| {
        r.parts
            .iter()
            .find(|p| p.part_types.iter().any(|t| t == kind))
            .unwrap()
            .item
    };
    let rear = bumper("rearbumper");
    let front = bumper("frontbumper");
    remove(&mut run, rear);
    let pose = run.area.items.get(front).unwrap().pose;
    let view = run.area.player.view(&run.physics).camera_pose.position;
    run.area.player.rebase([
        view[0] - pose.position[0],
        view[1] - pose.position[1] - 0.5,
        view[2] - pose.position[2] - 1.0,
    ]);
    let player_pose = run.area.player.body_pose(&run.physics);
    run.physics.edit_body(run.area.player.body_handle(), |b| {
        b.set_position(my_summer_slop::vehicle::to_rapier(player_pose), true)
    });
    let body = run.area.items.get(rear).unwrap().body.unwrap();
    run.physics.edit_body(body, |b| {
        b.set_position(my_summer_slop::vehicle::to_rapier(pose), true)
    });
    run.area.items.sync_poses(&run.physics);
    sync(&mut run);
    run.area
        .interaction
        .pickup_item(
            rear,
            pose.position,
            &run.area.player,
            &mut run.area.items,
            &mut run.physics,
        )
        .unwrap();
    run.area.interaction.step(
        FrameInput::default(),
        ResolvedAim::default(),
        &mut run.area.player,
        &mut run.area.items,
        &mut run.physics,
        FIXED_DT,
        Default::default(),
        1.0,
    );
    assert_eq!(
        run.area.interaction.held(),
        Some(rear),
        "part must remain in reach before the tested input"
    );
    let installation_target = run
        .area
        .interaction
        .parts
        .scan(rear, &run.area.items, &run.physics);
    assert!(
        installation_target.is_none(),
        "rear bumper must not fit front slots"
    );
    let aim = ResolvedAim {
        installation_target,
        ..Default::default()
    };
    let area = &mut run.area;
    area.interaction.step(
        FrameInput {
            pressed: keys::GRAB,
            held: keys::GRAB,
            ..Default::default()
        },
        aim,
        &mut area.player,
        &mut area.items,
        &mut run.physics,
        FIXED_DT,
        Default::default(),
        1.0,
    );
    let action = area.interaction.step(
        FrameInput {
            released: keys::GRAB,
            ..Default::default()
        },
        aim,
        &mut area.player,
        &mut area.items,
        &mut run.physics,
        FIXED_DT,
        Default::default(),
        1.0,
    );
    assert_eq!(action.action, Some(InteractionAction::Drop(rear)));
    assert_eq!(area.items.get(rear).unwrap().location, ItemLocation::World);
    assert_eq!(area.interaction.held(), None);
}
