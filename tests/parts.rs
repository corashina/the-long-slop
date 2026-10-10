mod common;
use my_summer_slop::interaction::{AimTarget, InteractionBlock, ResolvedAim};
use my_summer_slop::reference::{ItemId, PartSlotId, ReferenceManifest};
use my_summer_slop::simulation::{FrameInput, keys};
use my_summer_slop::starting_area::GarageRuntime;
use rapier3d::math::{Pose, Vector};

fn installed() -> ReferenceManifest {
    ReferenceManifest::from_json(&std::fs::read("web/assets/reference/manifest.json").unwrap())
        .unwrap()
}
fn part(r: &ReferenceManifest, tag: &str) -> ItemId {
    r.parts
        .iter()
        .find(|p| p.part_types.iter().any(|t| t == tag))
        .unwrap()
        .item
}
fn detach(g: &mut GarageRuntime, item: ItemId) {
    g.area
        .interaction
        .parts
        .can_remove(item, &g.area.player, &g.area.items)
        .unwrap();
    g.area
        .interaction
        .parts
        .remove(item, &mut g.area.items, &mut g.physics)
        .unwrap();
    g.area.interaction.parts.detach_mechanism(
        item,
        &mut g.area.interaction.mechanisms,
        &mut g.physics,
    );
    g.area
        .interaction
        .sync_parts(&mut g.area.items, &mut g.physics)
        .unwrap();
}
#[test]
#[ignore = "requires installed source export"]
fn hubcap_cover_blocks_wheel_removal_until_removed() {
    let r = installed();
    let mut g = GarageRuntime::new(&r).unwrap();
    for slot in &r.wheel_slots {
        assert!(
            g.wheels
                .remove(slot.id, &mut g.area.items, &mut g.physics)
                .is_err()
        );
        let cover = g.area.interaction.parts.covering_item(slot.rim).unwrap();
        detach(&mut g, cover);
        assert!(!g.area.interaction.parts.covered(slot.rim));
        g.wheels
            .remove(slot.id, &mut g.area.items, &mut g.physics)
            .unwrap();
    }
}
#[test]
#[ignore = "requires installed source export"]
fn wrong_or_occupied_slot_keeps_held_part_and_contents() {
    let r = installed();
    let mut g = GarageRuntime::new(&r).unwrap();
    let engine = part(&r, "coolanttank");
    g.area
        .items
        .get_mut(engine)
        .unwrap()
        .fluid
        .as_mut()
        .unwrap()
        .mixture
        .quantities_liters[3] = 2.1;
    detach(&mut g, engine);
    let point = g.area.items.get(engine).unwrap().pose.position;
    g.area
        .interaction
        .pickup_item(
            engine,
            point,
            &g.area.player,
            &mut g.area.items,
            &mut g.physics,
        )
        .unwrap();
    g.area.items.sync_poses(&g.physics);
    let before = g.area.items.get(engine).unwrap().clone();
    assert_eq!(
        g.area
            .interaction
            .parts
            .can_install(engine, PartSlotId(71602), &g.area.items),
        Err(InteractionBlock::Occupied)
    );
    assert_eq!(
        g.area
            .interaction
            .parts
            .can_install(engine, PartSlotId(73311), &g.area.items),
        Err(InteractionBlock::Incompatible)
    );
    let source_slot = r
        .part_slots
        .iter()
        .find(|s| s.id == PartSlotId(73311))
        .unwrap();
    let handle = g
        .physics
        .rapier()
        .colliders
        .iter()
        .find(|(_, c)| (c.user_data >> 64) as u32 == source_slot.colliders[0].0)
        .unwrap()
        .0;
    g.area.interaction.step(
        FrameInput::default(),
        ResolvedAim::default(),
        &mut g.area.player,
        &mut g.area.items,
        &mut g.physics,
        0.02,
    );
    g.area.interaction.step(
        FrameInput {
            held: keys::GRAB,
            pressed: keys::GRAB,
            ..Default::default()
        },
        ResolvedAim {
            installation_target: Some(AimTarget {
                collider: handle,
                item: engine,
                point,
                distance: 1.,
                slot: Some(my_summer_slop::interaction::SlotTarget::Part(
                    source_slot.id,
                )),
            }),
            ..Default::default()
        },
        &mut g.area.player,
        &mut g.area.items,
        &mut g.physics,
        0.02,
    );
    for input in [
        FrameInput {
            held: keys::GRAB,
            ..Default::default()
        },
        FrameInput {
            released: keys::GRAB,
            ..Default::default()
        },
    ] {
        g.area.interaction.step(
            input,
            ResolvedAim::default(),
            &mut g.area.player,
            &mut g.area.items,
            &mut g.physics,
            0.2,
        );
        assert_eq!(g.area.interaction.throw_charge(), 0.0);
        assert_eq!(g.area.interaction.held(), Some(engine));
    }
    let after = g.area.items.get(engine).unwrap();
    assert_eq!(g.area.interaction.held(), Some(engine));
    assert_eq!(after.location, before.location);
    assert_eq!(after.pose, before.pose);
    assert_eq!(after.fluid, before.fluid);
    assert_eq!(after.condition, before.condition);
}
#[test]
#[ignore = "requires installed source export"]
fn nested_part_moves_with_parent() {
    let r = installed();
    let mut g = GarageRuntime::new(&r).unwrap();
    let door = part(&r, "frdoor");
    let mirror = part(&r, "rightmirror");
    let count = g.physics.rapier().colliders.len();
    detach(&mut g, door);
    let body = g.area.items.get(door).unwrap().body.unwrap();
    assert_eq!(g.area.items.get(mirror).unwrap().body, Some(body));
    let before = g.area.items.get(mirror).unwrap().pose.position;
    let old = *g.physics.rapier().bodies[body].position();
    g.physics.rapier_mut().bodies[body]
        .set_position(Pose::from_translation(Vector::X * 2.) * old, true);
    g.area
        .interaction
        .sync_parts(&mut g.area.items, &mut g.physics)
        .unwrap();
    let after = g.area.items.get(mirror).unwrap().pose.position;
    assert!((after[0] - before[0] - 2.).abs() < 1e-4);
    assert_eq!(g.physics.rapier().colliders.len(), count);
    assert!(!g.area.items.mounted(mirror));
    for p in r.parts.iter().filter(|p| [door, mirror].contains(&p.item)) {
        for id in &p.colliders {
            let c = g
                .physics
                .rapier()
                .colliders
                .iter()
                .find(|(_, c)| (c.user_data >> 64) as u32 == id.0)
                .unwrap()
                .1;
            assert_eq!(c.parent(), Some(body));
        }
    }
}
#[test]
#[ignore = "requires installed source export"]
fn engine_and_radiator_removal_preserve_contents_and_change_operation() {
    let r = installed();
    let mut g = GarageRuntime::new(&r).unwrap();
    let engine = part(&r, "engine");
    let radiator = part(&r, "coolanttank");
    g.area
        .items
        .get_mut(r.engine.oil_tank)
        .unwrap()
        .fluid
        .as_mut()
        .unwrap()
        .mixture
        .quantities_liters[r.engine.oil_fluid as usize] = 1.7;
    g.area
        .items
        .get_mut(radiator)
        .unwrap()
        .fluid
        .as_mut()
        .unwrap()
        .mixture
        .quantities_liters[3] = 2.1;
    g.wheels.sync(&mut g.area.items, &mut g.physics).unwrap();
    let chassis = g.physics.chassis_handle();
    let before_mass = g.physics.rapier().bodies[chassis].mass();
    let oil = g.area.items.get(r.engine.oil_tank).unwrap().fluid.clone();
    let coolant = g.area.items.get(radiator).unwrap().fluid.clone();
    detach(&mut g, engine);
    detach(&mut g, radiator);
    g.wheels.sync(&mut g.area.items, &mut g.physics).unwrap();
    assert!(g.physics.rapier().bodies[chassis].mass() < before_mass - 3.8);
    assert!(!g.area.items.mounted(engine));
    assert!(!g.area.items.mounted(radiator));
    let output = g.engine.step(
        my_summer_slop::simulation::engine::EngineInput {
            engine_present: false,
            radiator_present: false,
            ignition_held: true,
            throttle: 1.,
            ..Default::default()
        },
        g.area.items.vehicle_fluids().unwrap(),
        &r.engine,
        0.,
        0.02,
    );
    assert!(!output.running);
    assert_eq!(output.torque_nm, 0.);
    assert_eq!(g.area.items.get(r.engine.oil_tank).unwrap().fluid, oil);
    assert_eq!(g.area.items.get(radiator).unwrap().fluid, coolant);
    for (item, slot) in [(engine, PartSlotId(71602)), (radiator, PartSlotId(77055))] {
        g.area
            .interaction
            .parts
            .install(item, slot, &mut g.area.items, &mut g.physics)
            .unwrap();
        g.area
            .interaction
            .parts
            .mount_mechanism(item, &mut g.area.interaction.mechanisms);
    }
    g.area
        .interaction
        .sync_parts(&mut g.area.items, &mut g.physics)
        .unwrap();
    assert!(g.area.items.mounted(engine));
    assert!(g.area.items.mounted(radiator));
}
#[test]
#[ignore = "requires installed source export"]
fn occupied_seat_cannot_be_removed() {
    let r = installed();
    let mut g = GarageRuntime::new(&r).unwrap();
    let seat = &r.seats[0];
    assert!(g.area.player.enter(
        seat.id,
        g.area.items.get(seat.item).unwrap().pose,
        &mut g.physics
    ));
    assert!(
        g.area
            .interaction
            .parts
            .can_remove(seat.item, &g.area.player, &g.area.items)
            .is_err()
    );
    assert!(!r.parts.iter().any(|p| p.item == seat.item));
}
#[test]
fn occupied_removable_seat_fixture_rejects_the_transaction() {
    use my_summer_slop::reference::*;
    let mut r = common::reference();
    let seat = r.seats[0].clone();
    let pickup = r.items.iter().find_map(|i| i.pickup.clone()).unwrap();
    let item = r.items.iter_mut().find(|i| i.id == seat.item).unwrap();
    item.pickup = Some(pickup);
    let node = item.node;
    r.parts.push(PartReference {
        usable: None,
        item: seat.item,
        source: seat.source.clone(),
        node,
        main_types: vec![],
        part_types: vec!["seat".into()],
        cover_slot: None,
        local_rotation_after_mount: [0., 0., 0., 1.],
        colliders: vec![],
        linear_damping: 0.,
        angular_damping: 0.,
        enable_if_mounted: vec![],
        enable_if_dismounted: vec![],
        disable_if_mounted: vec![],
        disable_if_dismounted: vec![],
    });
    r.part_slots.push(PartSlotReference {
        id: PartSlotId(1),
        source: seat.source.clone(),
        node,
        owner: None,
        part: Some(seat.item),
        mount_anchor: seat.anchor,
        detach_anchor: None,
        fixed_detach: false,
        detach_position: [0.; 3],
        detach_rotation: [0., 0., 0., 1.],
        main_types: vec![],
        part_types: vec!["seat".into()],
        cant_craft: false,
        cant_uncraft: false,
        disable_colliders_on_craft: false,
        colliders: vec![],
    });
    let mut g = GarageRuntime::new(&r).unwrap();
    assert!(
        g.area
            .interaction
            .parts
            .can_remove(seat.item, &g.area.player, &g.area.items)
            .is_ok()
    );
    let before = g.area.items.get(seat.item).unwrap().location;
    assert!(g.area.player.enter(
        seat.id,
        my_summer_slop::vehicle::VehiclePose {
            position: [10., 0.25, 0.],
            rotation: [0., 0., 0., 1.]
        },
        &mut g.physics
    ));
    assert_eq!(
        g.area
            .interaction
            .parts
            .can_remove(seat.item, &g.area.player, &g.area.items),
        Err(InteractionBlock::Unavailable)
    );
    assert_eq!(g.area.items.get(seat.item).unwrap().location, before);
}
#[test]
#[ignore = "requires installed source export"]
fn door_detach_resets_its_source_usable_and_keeps_window_state() {
    let r = installed();
    let mut g = GarageRuntime::new(&r).unwrap();
    let door = part(&r, "frdoor");
    let p = r.parts.iter().find(|p| p.item == door).unwrap();
    let usable = p.usable.unwrap();
    let window = r
        .mechanisms
        .iter()
        .find(|m| m.parent_item == Some(door) && m.slide.is_some())
        .unwrap()
        .item;
    let view = g.area.player.view_pose(&g.physics);
    g.area.interaction.mechanisms.step(
        usable,
        FrameInput {
            held: keys::USE,
            mouse: [50., 0.],
            ..Default::default()
        },
        view,
        0.02,
    );
    g.area.interaction.mechanisms.step(
        window,
        FrameInput {
            held: keys::USE,
            mouse: [0., 100.],
            ..Default::default()
        },
        view,
        0.02,
    );
    g.area
        .interaction
        .sync_parts(&mut g.area.items, &mut g.physics)
        .unwrap();
    let progress = g.area.interaction.mechanisms.value(window, false);
    assert!(progress.unwrap() > 0.);
    let driven = r
        .mechanisms
        .iter()
        .find(|m| m.item == usable)
        .unwrap()
        .driven_nodes[0];
    let before = g.area.interaction.mechanisms.node_pose(driven).unwrap();
    detach(&mut g, door);
    assert_eq!(
        g.area.interaction.mechanisms.rotation_degrees(usable),
        Some([0.; 2])
    );
    assert_eq!(g.area.interaction.mechanisms.value(window, false), progress);
    let after = g.area.interaction.mechanisms.node_pose(driven).unwrap();
    let dot: f32 = before
        .rotation
        .iter()
        .zip(after.rotation)
        .map(|(a, b)| a * b)
        .sum();
    assert!((dot.abs() - 1.).abs() < 1e-4, "{before:?} {after:?}");
}
#[test]
#[ignore = "requires installed source export"]
fn detached_lamps_turn_off_and_compatible_remount_restores_pose() {
    let r = installed();
    let mut g = GarageRuntime::new(&r).unwrap();
    let lamps: Vec<_> = r
        .parts
        .iter()
        .filter(|p| p.part_types.iter().any(|t| t == "roundheadlight"))
        .map(|p| p.item)
        .collect();
    g.lighting.controls.headlights = 1;
    for _ in 0..20 {
        g.lighting
            .step(&g.area.interaction.mechanisms, &g.area.items, 0.02);
    }
    let group = r
        .lighting
        .groups
        .iter()
        .position(|lamp| lamp.owner == Some(lamps[0]))
        .unwrap();
    assert!(g.lighting.output(group, 1) > 0.);
    detach(&mut g, lamps[0]);
    g.lighting
        .step(&g.area.interaction.mechanisms, &g.area.items, 0.02);
    assert_eq!(g.lighting.output(group, 1), 0.);
    let slot = r
        .part_slots
        .iter()
        .find(|s| s.part == Some(lamps[1]))
        .unwrap();
    detach(&mut g, lamps[1]);
    g.area
        .interaction
        .parts
        .install(lamps[0], slot.id, &mut g.area.items, &mut g.physics)
        .unwrap();
    g.area
        .interaction
        .parts
        .mount_mechanism(lamps[0], &mut g.area.interaction.mechanisms);
    g.area
        .interaction
        .sync_parts(&mut g.area.items, &mut g.physics)
        .unwrap();
    let anchor = r
        .anchors
        .iter()
        .find(|a| a.id == slot.mount_anchor)
        .unwrap();
    let expected = g
        .area
        .interaction
        .mechanisms
        .node_pose(anchor.node)
        .unwrap();
    let actual = g.area.items.get(lamps[0]).unwrap().pose;
    assert!(
        (Vector::from_array(actual.position) - Vector::from_array(expected.position)).length()
            < 1e-4
    );
    for _ in 0..20 {
        g.lighting
            .step(&g.area.interaction.mechanisms, &g.area.items, 0.02);
    }
    assert!(g.lighting.output(group, 1) > 0.);
}
#[test]
#[ignore = "requires installed source export"]
fn missing_door_does_not_power_the_cabin_lamp() {
    let r = installed();
    let mut g = GarageRuntime::new(&r).unwrap();
    let door = part(&r, "frdoor");
    let usable = r
        .parts
        .iter()
        .find(|p| p.item == door)
        .unwrap()
        .usable
        .unwrap();
    let view = g.area.player.view_pose(&g.physics);
    g.area.interaction.mechanisms.step(
        usable,
        FrameInput {
            held: keys::USE,
            mouse: [100., 100.],
            ..Default::default()
        },
        view,
        0.02,
    );
    if g.area.interaction.mechanisms.value(usable, false).unwrap() < 0.02 {
        g.area.interaction.mechanisms.step(
            usable,
            FrameInput {
                held: keys::USE,
                mouse: [-200., -200.],
                ..Default::default()
            },
            view,
            0.02,
        );
    }
    for _ in 0..20 {
        g.lighting
            .step(&g.area.interaction.mechanisms, &g.area.items, 0.02);
    }
    assert!(g.lighting.controls.interior_light);
    detach(&mut g, door);
    for _ in 0..20 {
        g.lighting
            .step(&g.area.interaction.mechanisms, &g.area.items, 0.02);
    }
    assert!(!g.lighting.controls.interior_light);
}
#[test]
#[ignore = "requires installed source export"]
fn failed_carry_acquisition_keeps_part_mounted() {
    let r = installed();
    let mut g = GarageRuntime::new(&r).unwrap();
    let item = part(&r, "coolanttank");
    let before = g.area.items.get(item).unwrap().clone();
    let source = r.parts.iter().find(|p| p.item == item).unwrap();
    let handle = g
        .physics
        .rapier()
        .colliders
        .iter()
        .find(|(_, c)| (c.user_data >> 64) as u32 == source.colliders[0].0)
        .unwrap()
        .0;
    let collider_parent = g.physics.rapier().colliders[handle].parent();
    let action = g.area.interaction.step(
        FrameInput {
            held: keys::GRAB,
            pressed: keys::GRAB,
            ..Default::default()
        },
        ResolvedAim {
            grab_target: Some(AimTarget {
                collider: handle,
                item,
                point: [f32::NAN, 0., 0.],
                distance: 1.,
                slot: None,
            }),
            ..Default::default()
        },
        &mut g.area.player,
        &mut g.area.items,
        &mut g.physics,
        0.02,
    );
    let after = g.area.items.get(item).unwrap();
    assert!(action.is_none());
    assert!(g.area.interaction.held().is_none());
    assert!(g.area.interaction.parts.installed(item));
    assert_eq!(after.location, before.location);
    assert_eq!(after.pose, before.pose);
    assert_eq!(after.fluid, before.fluid);
    assert_eq!(after.condition, before.condition);
    assert_eq!(
        g.physics.rapier().colliders[handle].parent(),
        collider_parent
    );
}

#[test]
#[ignore = "requires installed source export"]
fn transferred_hubcap_blocks_only_its_current_wheel() {
    let r = installed();
    let mut g = GarageRuntime::new(&r).unwrap();
    let source = PartSlotId(65545);
    let spare = PartSlotId(74070);
    let cover = g.area.interaction.parts.slot_item(source).unwrap();
    let rim = r
        .wheel_parts
        .iter()
        .find(|p| p.cover_slot == Some(source))
        .unwrap()
        .item;
    let spare_rim = r
        .wheel_parts
        .iter()
        .find(|p| p.cover_slot == Some(spare))
        .unwrap()
        .item;
    let slot = g.wheels.slot_for_item(rim).unwrap();
    detach(&mut g, cover);
    g.area
        .interaction
        .parts
        .install(cover, spare, &mut g.area.items, &mut g.physics)
        .unwrap();
    g.area
        .interaction
        .parts
        .mount_mechanism(cover, &mut g.area.interaction.mechanisms);
    g.area
        .interaction
        .sync_parts(&mut g.area.items, &mut g.physics)
        .unwrap();
    assert_eq!(g.area.interaction.parts.slot_item(source), None);
    g.wheels
        .remove(slot, &mut g.area.items, &mut g.physics)
        .unwrap();
    assert!(g.wheels.can_install(rim, slot, &g.area.items).is_ok());
    assert!(
        g.wheels
            .can_install(spare_rim, slot, &g.area.items)
            .is_err()
    );
    g.wheels
        .install(rim, slot, &mut g.area.items, &mut g.physics)
        .unwrap();
    let replacement = g
        .area
        .interaction
        .parts
        .slot_item(PartSlotId(66804))
        .unwrap();
    detach(&mut g, replacement);
    g.area
        .interaction
        .parts
        .install(replacement, source, &mut g.area.items, &mut g.physics)
        .unwrap();
    g.area
        .interaction
        .parts
        .mount_mechanism(replacement, &mut g.area.interaction.mechanisms);
    g.area
        .interaction
        .sync_parts(&mut g.area.items, &mut g.physics)
        .unwrap();
    assert!(
        g.wheels
            .remove(slot, &mut g.area.items, &mut g.physics)
            .is_err()
    );
    detach(&mut g, replacement);
    g.wheels
        .remove(slot, &mut g.area.items, &mut g.physics)
        .unwrap();
}

#[test]
#[ignore = "requires installed source export"]
fn loose_mass_tracks_nested_attachment_and_fluid_changes() {
    fn mass(g: &mut GarageRuntime, item: ItemId) -> f32 {
        let body = g.area.items.get(item).unwrap().body.unwrap();
        let world = g.physics.rapier_mut();
        world.bodies[body].recompute_mass_properties_from_colliders(&world.colliders);
        world.bodies[body].mass()
    }
    let r = installed();
    let mut g = GarageRuntime::new(&r).unwrap();
    let door = part(&r, "frdoor");
    let mirror = part(&r, "rightmirror");
    let slot = r
        .part_slots
        .iter()
        .find(|s| s.part == Some(mirror))
        .unwrap()
        .id;
    detach(&mut g, door);
    let initial = mass(&mut g, door);
    detach(&mut g, mirror);
    let door_mass = mass(&mut g, door);
    let mirror_mass = mass(&mut g, mirror);
    assert!(mirror_mass > 0.);
    assert!((initial - door_mass - mirror_mass).abs() < 0.001);
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
    assert!((mass(&mut g, door) - initial).abs() < 0.001);
    let radiator = part(&r, "coolanttank");
    detach(&mut g, radiator);
    let before = mass(&mut g, radiator);
    g.area
        .items
        .get_mut(radiator)
        .unwrap()
        .fluid
        .as_mut()
        .unwrap()
        .mixture
        .quantities_liters[3] += 1.;
    g.area
        .interaction
        .parts
        .sync(&mut g.area.items, &mut g.physics)
        .unwrap();
    assert!((mass(&mut g, radiator) - before - 1.).abs() < 0.001);
}

#[test]
#[ignore = "requires installed source export"]
fn detached_door_children_survive_rebase_and_recovery() {
    let r = installed();
    let mut g = GarageRuntime::new(&r).unwrap();
    let door = part(&r, "frdoor");
    let mirror = part(&r, "rightmirror");
    let window = r
        .mechanisms
        .iter()
        .find(|m| m.source.path_id == 70647)
        .unwrap()
        .item;
    let radiator = part(&r, "coolanttank");
    g.area.interaction.mechanisms.step(
        window,
        FrameInput {
            held: keys::USE,
            mouse: [0., 100.],
            ..Default::default()
        },
        g.area.player.view_pose(&g.physics),
        0.02,
    );
    g.area
        .interaction
        .sync_parts(&mut g.area.items, &mut g.physics)
        .unwrap();
    let window_value = g.area.interaction.mechanisms.value(window, false);
    assert!(window_value.unwrap() > 0.);
    detach(&mut g, door);
    detach(&mut g, radiator);
    g.area.items.get_mut(mirror).unwrap().condition = 0.37;
    g.area
        .items
        .get_mut(radiator)
        .unwrap()
        .fluid
        .as_mut()
        .unwrap()
        .mixture
        .quantities_liters[3] = 1.7;
    g.area
        .interaction
        .sync_parts(&mut g.area.items, &mut g.physics)
        .unwrap();
    let before = [door, window, mirror, radiator].map(|id| g.area.items.get(id).unwrap().clone());
    let source_handles: Vec<_> = r
        .parts
        .iter()
        .map(|p| g.area.interaction.parts.loose_body(p.item).unwrap())
        .collect();
    let shift = [1280., 0., -1280.];
    g.physics.rebase(shift);
    g.stream.origin.shift(shift);
    g.area.sync_origin(g.stream.origin.offset);
    let radiator_slot = r
        .part_slots
        .iter()
        .find(|s| s.part == Some(radiator))
        .unwrap();
    let anchor = r
        .anchors
        .iter()
        .find(|a| a.id == radiator_slot.mount_anchor)
        .unwrap();
    let expected_mount = Vector::from_array(
        g.area
            .interaction
            .mechanisms
            .node_pose(anchor.node)
            .unwrap()
            .position,
    );
    g.area
        .interaction
        .parts
        .install(
            radiator,
            radiator_slot.id,
            &mut g.area.items,
            &mut g.physics,
        )
        .unwrap();
    let actual_mount = Vector::from_array(g.area.items.get(radiator).unwrap().pose.position);
    assert!(
        (actual_mount - expected_mount).length() < 1e-3,
        "radiator mount used an old anchor: {actual_mount:?} vs {expected_mount:?}"
    );
    g.area
        .interaction
        .parts
        .mount_mechanism(radiator, &mut g.area.interaction.mechanisms);
    let door_node = r.parts.iter().find(|p| p.item == door).unwrap().node;
    let rebased_door = Vector::from_array(
        g.area
            .interaction
            .mechanisms
            .node_pose(door_node)
            .unwrap()
            .position,
    );
    assert!(
        (rebased_door - (Vector::from_array(before[0].pose.position) - Vector::from_array(shift)))
            .length()
            < 1e-3
    );
    g.area
        .interaction
        .sync_parts(&mut g.area.items, &mut g.physics)
        .unwrap();
    detach(&mut g, radiator);
    // An immediate child transaction must use the rebased slot anchor.
    detach(&mut g, mirror);
    let expected = Vector::from_array(before[2].pose.position) - Vector::from_array(shift);
    let actual = Vector::from_array(g.area.items.get(mirror).unwrap().pose.position);
    assert!(
        (actual - expected).length() < 1e-3,
        "mirror detach used an old anchor: {actual:?} vs {expected:?}"
    );
    let mirror_slot = r
        .part_slots
        .iter()
        .find(|s| s.part == Some(mirror))
        .unwrap()
        .id;
    g.area
        .interaction
        .parts
        .install(mirror, mirror_slot, &mut g.area.items, &mut g.physics)
        .unwrap();
    g.area
        .interaction
        .parts
        .mount_mechanism(mirror, &mut g.area.interaction.mechanisms);
    g.area
        .interaction
        .sync_parts(&mut g.area.items, &mut g.physics)
        .unwrap();
    // Keep the detached assembly with the player through recovery.
    let point = g.area.items.get(door).unwrap().pose.position;
    g.area
        .interaction
        .pickup_item(
            door,
            point,
            &g.area.player,
            &mut g.area.items,
            &mut g.physics,
        )
        .unwrap();
    g.recover(&r);
    assert_eq!(
        g.area.interaction.mechanisms.value(window, false),
        window_value
    );
    for (id, old) in [door, window, mirror, radiator].into_iter().zip(before) {
        let state = g.area.items.get(id).unwrap();
        assert_eq!(state.location, old.location);
        assert_eq!(state.condition, old.condition);
        assert_eq!(state.fluid, old.fluid);
        if id == window || id == mirror {
            assert_eq!(state.body, g.area.items.get(door).unwrap().body);
            let node = r.items.iter().find(|i| i.id == id).unwrap().node;
            let mechanism_pose = g.area.interaction.mechanisms.node_pose(node).unwrap();
            assert!(
                (Vector::from_array(mechanism_pose.position)
                    - Vector::from_array(state.pose.position))
                .length()
                    < 1e-3
            );
        }
        let rendered = g.area.item_render_poses.iter().find(|p| p.0 == id).unwrap();
        assert_eq!(rendered.1, state.pose);
        assert_eq!(rendered.2, state.pose);
    }
    for (p, handle) in r.parts.iter().zip(source_handles) {
        assert_eq!(g.area.interaction.parts.loose_body(p.item), Some(handle));
        assert!(g.physics.rapier().bodies.contains(handle));
    }
    let body = g.area.items.get(door).unwrap().body.unwrap();
    let door_inverse = g.physics.rapier().bodies[body].position().inverse();
    for source in r.parts.iter().filter(|p| [door, mirror].contains(&p.item)) {
        for id in &source.colliders {
            let collider = g
                .physics
                .rapier()
                .colliders
                .iter()
                .find(|(_, c)| (c.user_data >> 64) as u32 == id.0)
                .unwrap()
                .1;
            assert_eq!(collider.parent(), Some(body));
            assert!(collider.is_enabled());
        }
    }
    for id in [window, mirror] {
        let item = g.area.items.get(id).unwrap();
        assert!(
            (door_inverse * my_summer_slop::vehicle::to_rapier(item.pose))
                .translation
                .distance(my_summer_slop::vehicle::to_rapier(item.local_pose).translation)
                < 1e-3
        );
    }
    let radiator_body = g.area.items.get(radiator).unwrap().body.unwrap();
    let world = g.physics.rapier_mut();
    world.bodies[radiator_body].recompute_mass_properties_from_colliders(&world.colliders);
    let mass = world.bodies[radiator_body].mass();
    g.area
        .items
        .get_mut(radiator)
        .unwrap()
        .fluid
        .as_mut()
        .unwrap()
        .mixture
        .quantities_liters[3] += 0.3;
    g.area
        .interaction
        .parts
        .sync(&mut g.area.items, &mut g.physics)
        .unwrap();
    let world = g.physics.rapier_mut();
    world.bodies[radiator_body].recompute_mass_properties_from_colliders(&world.colliders);
    assert!((world.bodies[radiator_body].mass() - mass - 0.3).abs() < 1e-3);
}

#[test]
#[ignore = "requires installed source export"]
fn restart_clears_constraints_and_restores_source_parts() {
    let r = installed();
    let mut g = GarageRuntime::new(&r).unwrap();
    let initial: Vec<_> = g
        .area
        .items
        .iter()
        .filter(|i| i.id.0 < 0x80000000)
        .cloned()
        .collect();
    let counts = (g.physics.rapier().bodies.len(), g.physics.collider_count());
    let controls: Vec<_> = r
        .mechanisms
        .iter()
        .map(|m| (m.item, g.area.interaction.mechanisms.value(m.item, false)))
        .collect();
    for _ in 0..3 {
        let door = part(&r, "frdoor");
        detach(&mut g, door);
        let point = g.area.items.get(door).unwrap().pose.position;
        g.area
            .interaction
            .pickup_item(
                door,
                point,
                &g.area.player,
                &mut g.area.items,
                &mut g.physics,
            )
            .unwrap();
        assert!(!g.physics.rapier().impulse_joints.is_empty());
        for i in g.area.items.iter_mut() {
            i.condition = 0.1;
            if let Some(f) = &mut i.fluid {
                f.mixture.quantities_liters = [0.; 6];
            }
        }
        let seat = &r.seats[0];
        assert!(g.area.player.enter(
            seat.id,
            g.area.items.get(seat.item).unwrap().pose,
            &mut g.physics
        ));
        g.restart(&r).unwrap();
        assert_eq!(
            (g.physics.rapier().bodies.len(), g.physics.collider_count()),
            counts
        );
        assert!(g.physics.rapier().impulse_joints.is_empty());
        assert_eq!(g.area.player.seat(), None);
        assert_eq!(g.area.interaction.held(), None);
        assert_eq!(g.area.interaction.throw_charge(), 0.);
        assert!(!g.area.interaction.push_gripping());
        for old in &initial {
            let item = g.area.items.get(old.id).unwrap();
            assert_eq!(item.location, old.location);
            assert_eq!(item.fluid, old.fluid);
            assert_eq!(item.condition, old.condition);
        }
        for &(id, value) in &controls {
            assert_eq!(g.area.interaction.mechanisms.value(id, false), value);
        }
        for slot in &r.part_slots {
            assert_eq!(g.area.interaction.parts.slot_item(slot.id), slot.part);
        }
    }
}
