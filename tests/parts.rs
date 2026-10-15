mod common;
use my_summer_slop::interaction::{AimTarget, InteractionBlock, ResolvedAim};
use my_summer_slop::reference::{ItemId, PartSlotId, ReferenceManifest};
use my_summer_slop::simulation::{FrameInput, keys};
use my_summer_slop::starting_area::GarageRuntime;

fn installed() -> ReferenceManifest {
    ReferenceManifest::from_json(&std::fs::read("web/assets/reference/manifest.json").unwrap())
        .unwrap()
}
/// The loose spare rim in the garage.
fn spare_rim(r: &ReferenceManifest) -> ItemId {
    r.items
        .iter()
        .find(|i| i.source.path_id == 34665)
        .unwrap()
        .id
}
fn part(r: &ReferenceManifest, tag: &str) -> ItemId {
    r.parts
        .iter()
        .find(|p| p.part_types.iter().any(|t| t == tag))
        .unwrap()
        .item
}
use common::detach;
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
    let conditions_before = g.area.conditions.capture_save(&r).unwrap();
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
        Default::default(),
        1.,
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
        Default::default(),
        1.,
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
            Default::default(),
            1.,
        );
        assert_eq!(g.area.interaction.throw_charge(), 0.0);
        assert_eq!(g.area.interaction.held(), Some(engine));
    }
    let after = g.area.items.get(engine).unwrap();
    assert_eq!(g.area.interaction.held(), Some(engine));
    assert_eq!(after.location, before.location);
    assert_eq!(after.pose, before.pose);
    assert_eq!(after.fluid, before.fluid);
    assert_eq!(
        g.area.conditions.capture_save(&r).unwrap(),
        conditions_before
    );
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
        .get_mut(r.vehicles[0].engine.oil_tank)
        .unwrap()
        .fluid
        .as_mut()
        .unwrap()
        .mixture
        .quantities_liters[r.vehicles[0].engine.oil_fluid as usize] = 1.7;
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
    let chassis = g.physics.chassis_handle(0);
    let before_mass = g.physics.rapier().bodies[chassis].mass();
    let oil = g
        .area
        .items
        .get(r.vehicles[0].engine.oil_tank)
        .unwrap()
        .fluid
        .clone();
    let coolant = g.area.items.get(radiator).unwrap().fluid.clone();
    detach(&mut g, engine);
    detach(&mut g, radiator);
    g.wheels.sync(&mut g.area.items, &mut g.physics).unwrap();
    assert!(g.physics.rapier().bodies[chassis].mass() < before_mass - 3.8);
    assert!(!g.area.items.mounted(engine));
    assert!(!g.area.items.mounted(radiator));
    let output = g.vehicles[0].engine.step(
        my_summer_slop::simulation::engine::EngineInput {
            engine_present: false,
            radiator_present: false,
            ignition_held: true,
            throttle: 1.,
            ..Default::default()
        },
        g.area.items.vehicle_fluids(0).unwrap(),
        &r.vehicles[0].engine,
        0.,
        0.02,
    );
    assert!(!output.running);
    assert_eq!(output.torque_nm, 0.);
    assert_eq!(
        g.area
            .items
            .get(r.vehicles[0].engine.oil_tank)
            .unwrap()
            .fluid,
        oil
    );
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

fn fit_spare_hubcap(g: &mut GarageRuntime, r: &ReferenceManifest) -> (ItemId, ItemId, ItemId) {
    let rim = spare_rim(r);
    let cap = ItemId(44);
    let tire = r
        .items
        .iter()
        .find(|i| {
            i.attached_to == Some(rim)
                && matches!(i.kind, my_summer_slop::reference::ItemKind::Tire)
        })
        .unwrap()
        .id;
    detach(g, cap);
    let slot = r
        .wheel_parts
        .iter()
        .find(|p| p.item == rim)
        .unwrap()
        .cover_slot
        .unwrap();
    g.area
        .interaction
        .parts
        .install(cap, slot, &mut g.area.items, &mut g.physics)
        .unwrap();
    g.area
        .interaction
        .parts
        .mount_mechanism(cap, &mut g.area.interaction.mechanisms);
    g.area
        .interaction
        .sync_parts(&mut g.area.items, &mut g.physics)
        .unwrap();
    g.wheels.sync(&mut g.area.items, &mut g.physics).unwrap();
    (rim, tire, cap)
}
fn body_mass(g: &mut GarageRuntime, item: ItemId) -> f32 {
    let body = g.area.items.get(item).unwrap().body.unwrap();
    let world = g.physics.rapier_mut();
    world.bodies[body].recompute_mass_properties_from_colliders(&world.colliders);
    world.bodies[body].mass()
}
#[test]
#[ignore = "requires installed source export"]
fn covered_loose_wheel_mass_conserves_add_remove_and_transfer() {
    let r = installed();
    let mut g = GarageRuntime::new(&r).unwrap();
    let rim = spare_rim(&r);
    let before = body_mass(&mut g, rim);
    let (_, _, cap) = fit_spare_hubcap(&mut g, &r);
    let cap_mass = r.items.iter().find(|i| i.id == cap).unwrap().mass_kg;
    assert!((body_mass(&mut g, rim) - before - cap_mass).abs() < 1e-4);
    detach(&mut g, cap);
    g.wheels.sync(&mut g.area.items, &mut g.physics).unwrap();
    assert!((body_mass(&mut g, rim) - before).abs() < 1e-4);
    let detached = body_mass(&mut g, cap);
    assert!(
        (detached - cap_mass).abs() < 1e-4,
        "cap mass {detached}, expected {cap_mass}, body {:?}, location {:?}",
        g.area.items.get(cap).unwrap().body,
        g.area.items.get(cap).unwrap().location
    );
    let target = &r.wheel_slots[0];
    let old_cap = g.area.interaction.parts.covering_item(target.rim).unwrap();
    detach(&mut g, old_cap);
    g.wheels
        .remove(target.id, &mut g.area.items, &mut g.physics)
        .unwrap();
    g.area
        .interaction
        .sync_parts(&mut g.area.items, &mut g.physics)
        .unwrap();
    let target_before = body_mass(&mut g, target.rim);
    let slot = r
        .wheel_parts
        .iter()
        .find(|p| p.item == target.rim)
        .unwrap()
        .cover_slot
        .unwrap();
    g.area
        .interaction
        .parts
        .install(cap, slot, &mut g.area.items, &mut g.physics)
        .unwrap();
    g.area
        .interaction
        .parts
        .mount_mechanism(cap, &mut g.area.interaction.mechanisms);
    g.area
        .interaction
        .sync_parts(&mut g.area.items, &mut g.physics)
        .unwrap();
    g.wheels.sync(&mut g.area.items, &mut g.physics).unwrap();
    assert!((body_mass(&mut g, target.rim) - target_before - cap_mass).abs() < 1e-4);
    assert!((body_mass(&mut g, rim) - before).abs() < 1e-4);
}
