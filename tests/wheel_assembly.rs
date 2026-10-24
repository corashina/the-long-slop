use the_long_slop::{
    interaction::items::ItemLocation,
    reference::{ItemId, ReferenceManifest},
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

fn mass(run: &GarageRuntime, item: ItemId) -> f32 {
    let body = run.area.items.get(item).unwrap().body.unwrap();
    run.physics.rapier().bodies[body].mass()
}

fn wheel_colliders(
    run: &GarageRuntime,
    reference: &ReferenceManifest,
    item: ItemId,
    enabled: bool,
) -> bool {
    let record = reference
        .wheel_parts
        .iter()
        .find(|part| part.item == item)
        .unwrap();
    let body = run.area.items.get(item).unwrap().body;
    record.colliders.iter().all(|id| {
        let collider = run
            .physics
            .rapier()
            .colliders
            .iter()
            .find(|(_, c)| (c.user_data >> 64) as u32 == id.0)
            .unwrap()
            .1;
        collider.parent() == body
            && collider.is_enabled() == enabled
            && collider.solver_groups() == the_long_slop::vehicle::solver_groups(enabled)
    })
}

fn shape_offsets(
    run: &GarageRuntime,
    reference: &ReferenceManifest,
    item: ItemId,
) -> Vec<(rapier3d::prelude::ColliderHandle, rapier3d::math::Pose)> {
    let inverse =
        the_long_slop::vehicle::to_rapier(run.area.items.get(item).unwrap().pose).inverse();
    reference
        .wheel_parts
        .iter()
        .find(|p| p.item == item)
        .unwrap()
        .colliders
        .iter()
        .map(|id| {
            let (handle, collider) = run
                .physics
                .rapier()
                .colliders
                .iter()
                .find(|(_, c)| (c.user_data >> 64) as u32 == id.0)
                .unwrap();
            (handle, inverse * *collider.position())
        })
        .collect()
}

fn assert_shape_poses(
    run: &GarageRuntime,
    item: ItemId,
    shapes: &[(rapier3d::prelude::ColliderHandle, rapier3d::math::Pose)],
) {
    let local = the_long_slop::vehicle::to_rapier(run.area.items.get(item).unwrap().local_pose);
    for (handle, offset) in shapes {
        let expected = local * *offset;
        let actual = run.physics.rapier().colliders[*handle]
            .position_wrt_parent()
            .unwrap();
        assert!((actual.translation - expected.translation).length() < 0.0001);
        assert!(actual.rotation.dot(expected.rotation).abs() > 0.9999);
    }
}

fn check(errors: &mut Vec<String>, valid: bool, step: &str) {
    if !valid {
        errors.push(step.into());
    }
}

#[test]
#[ignore = "requires build 11140890 export"]
fn installed_lada_g03_identity_mass_and_collision() {
    let bytes = std::fs::read("web/assets/reference/manifest.json").unwrap();
    let reference = ReferenceManifest::from_runtime_json(&bytes).unwrap();
    let vehicle = reference
        .vehicles
        .iter()
        .position(|v| v.source.path_id == 77839)
        .unwrap();
    let mut run = GarageRuntime::new(&reference).unwrap();
    let chassis = run.physics.chassis_handle(vehicle);
    let original_mass = run.physics.rapier().bodies[chassis].mass();
    let item_count = run.area.items.iter().count();
    let collider_count = run.physics.rapier().colliders.len();
    let mut errors = Vec::new();
    for id in reference.vehicles[vehicle].wheel_slots {
        let slot = reference.wheel_slots.iter().find(|s| s.id == id).unwrap();
        let rim = slot.rim;
        let tire = slot.tire;
        let cover_slot = reference
            .wheel_parts
            .iter()
            .find(|p| p.item == rim)
            .unwrap()
            .cover_slot
            .unwrap();
        let cover = run.area.interaction.parts.slot_item(cover_slot).unwrap();
        let rim_mass = reference
            .items
            .iter()
            .find(|i| i.id == rim)
            .unwrap()
            .mass_kg;
        let tire_mass = reference
            .items
            .iter()
            .find(|i| i.id == tire)
            .unwrap()
            .mass_kg;
        let cover_mass = reference
            .items
            .iter()
            .find(|i| i.id == cover)
            .unwrap()
            .mass_kg;
        let conditions = [rim, tire, cover].map(|item| run.area.conditions.item_quality(item));
        let mut bodies = None;
        for cycle in 0..3 {
            assert!(
                run.wheels
                    .remove(id, &mut run.area.items, &mut run.physics)
                    .is_err()
            );
            assert_eq!(
                run.area.items.get(rim).unwrap().location,
                ItemLocation::WheelSlot(id)
            );
            assert_eq!(
                run.area.items.get(tire).unwrap().location,
                ItemLocation::Attached(rim)
            );
            run.area
                .interaction
                .parts
                .remove(cover, &mut run.area.items, &mut run.physics)
                .unwrap();
            sync(&mut run);
            assert!(
                (run.physics.rapier().bodies[chassis].mass() - (original_mass - cover_mass)).abs()
                    < 0.01
            );
            assert_eq!(
                run.wheels
                    .remove(id, &mut run.area.items, &mut run.physics)
                    .unwrap(),
                rim
            );
            sync(&mut run);
            assert_eq!(
                run.area.items.get(rim).unwrap().location,
                ItemLocation::World
            );
            assert_eq!(
                run.area.items.get(tire).unwrap().location,
                ItemLocation::Attached(rim)
            );
            assert!((mass(&run, rim) - rim_mass - tire_mass).abs() < 0.01);
            assert!(wheel_colliders(&run, &reference, rim, true));
            assert!(wheel_colliders(&run, &reference, tire, true));
            let rim_shapes = shape_offsets(&run, &reference, rim);
            let tire_shapes = shape_offsets(&run, &reference, tire);
            assert!(
                (run.physics.rapier().bodies[chassis].mass()
                    - (original_mass - cover_mass - rim_mass - tire_mass))
                    .abs()
                    < 0.01
            );
            assert_eq!(
                run.wheels
                    .split(rim, &mut run.area.items, &mut run.physics)
                    .unwrap(),
                (rim, tire)
            );
            assert_eq!(
                run.area.items.get(tire).unwrap().location,
                ItemLocation::World
            );
            assert!((mass(&run, rim) - rim_mass).abs() < 0.01);
            assert!((mass(&run, tire) - tire_mass).abs() < 0.01);
            let before = [rim, tire].map(|item| run.area.items.get(item).unwrap().location);
            assert!(
                run.wheels
                    .assemble(tire, rim, &mut run.area.items, &mut run.physics)
                    .is_err()
            );
            assert_eq!(
                before,
                [rim, tire].map(|item| run.area.items.get(item).unwrap().location)
            );
            assert_eq!(
                run.wheels
                    .assemble(rim, tire, &mut run.area.items, &mut run.physics)
                    .unwrap(),
                rim
            );
            assert!(
                run.wheels
                    .assemble(rim, tire, &mut run.area.items, &mut run.physics)
                    .is_err()
            );
            run.wheels
                .install(rim, id, &mut run.area.items, &mut run.physics)
                .unwrap();
            sync(&mut run);
            assert!(wheel_colliders(&run, &reference, rim, false));
            assert!(wheel_colliders(&run, &reference, tire, false));
            assert_shape_poses(&run, rim, &rim_shapes);
            assert_shape_poses(&run, tire, &tire_shapes);
            run.area
                .interaction
                .parts
                .install(cover, cover_slot, &mut run.area.items, &mut run.physics)
                .unwrap();
            sync(&mut run);
            assert!((run.physics.rapier().bodies[chassis].mass() - original_mass).abs() < 0.01);
            // Source tire slots do not inherit the rim's hubcap removal restriction.
            run.wheels
                .split(rim, &mut run.area.items, &mut run.physics)
                .unwrap();
            assert!(
                (run.physics.rapier().bodies[chassis].mass() - (original_mass - tire_mass)).abs()
                    < 0.01
            );
            run.wheels
                .assemble(rim, tire, &mut run.area.items, &mut run.physics)
                .unwrap();
            sync(&mut run);
            assert_shape_poses(&run, tire, &tire_shapes);
            check(
                &mut errors,
                wheel_colliders(&run, &reference, tire, false),
                &format!(
                    "slot {} cycle {cycle}: tire collision enabled after mounted assembly",
                    id.0
                ),
            );
            assert_eq!(
                run.area.items.get(rim).unwrap().location,
                ItemLocation::WheelSlot(id)
            );
            assert_eq!(
                run.area.items.get(tire).unwrap().location,
                ItemLocation::Attached(rim)
            );
            assert_eq!(
                run.area.items.get(cover).unwrap().location,
                ItemLocation::PartSlot(cover_slot)
            );
            assert_eq!(
                conditions,
                [rim, tire, cover].map(|item| run.area.conditions.item_quality(item))
            );
            assert_eq!(item_count, run.area.items.iter().count());
            assert_eq!(collider_count, run.physics.rapier().colliders.len());
            assert!((run.physics.rapier().bodies[chassis].mass() - original_mass).abs() < 0.01);
            if let Some(count) = bodies {
                assert_eq!(count, run.physics.rapier().bodies.len());
            }
            bodies = Some(run.physics.rapier().bodies.len());
        }
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}
