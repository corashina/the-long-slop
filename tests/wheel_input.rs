use rapier3d::math::{Rotation, Vector};
use the_long_slop::{
    interaction::{
        AimTarget, InteractionAction, ResolvedAim, SlotTarget, WheelAction, feedback::ActionMask,
        items::ItemLocation,
    },
    reference::{ItemId, ReferenceManifest},
    simulation::{FIXED_DT, FrameInput, keys},
    starting_area::GarageRuntime,
    vehicle::VehiclePose,
};

fn input(
    run: &mut GarageRuntime,
    frame: FrameInput,
    aim: ResolvedAim,
) -> Option<InteractionAction> {
    run.area
        .interaction
        .step(
            frame,
            aim,
            &mut run.area.player,
            &mut run.area.items,
            &mut run.physics,
            FIXED_DT,
            run.survival.output(),
            1.,
        )
        .action
}

fn ray_aim(
    run: &GarageRuntime,
    sensor: rapier3d::prelude::ColliderHandle,
    accepts: impl Fn(&ResolvedAim) -> bool,
) -> ResolvedAim {
    let center = run.physics.rapier().colliders[sensor]
        .compute_aabb()
        .center();
    [
        Vector::X,
        -Vector::X,
        Vector::Y,
        -Vector::Y,
        Vector::Z,
        -Vector::Z,
    ]
    .into_iter()
    .find_map(|axis| {
        let view = VehiclePose {
            position: (center + axis * 1.25).to_array(),
            rotation: Rotation::from_rotation_arc(-Vector::Z, -axis).to_array(),
        };
        let aim = run
            .area
            .interaction
            .resolve(&run.physics, view, &run.area.items);
        accepts(&aim).then_some(aim)
    })
    .expect("source tire slot must resolve the expected action")
}

#[test]
#[ignore = "requires build 11140890 export"]
fn installed_source_tire_input_order_and_rejection() {
    let reference = ReferenceManifest::from_runtime_json(
        &std::fs::read("web/assets/reference/manifest.json").unwrap(),
    )
    .unwrap();
    let mut run = GarageRuntime::new(&reference).unwrap();
    let slot = &reference.wheel_slots[0];
    let rim = slot.rim;
    let original_tire = slot.tire;
    let spare = reference
        .items
        .iter()
        .find(|item| {
            matches!(item.kind, the_long_slop::reference::ItemKind::Rim)
                && item.attached_to.is_none()
        })
        .unwrap()
        .id;
    let tire = reference
        .items
        .iter()
        .find(|item| {
            item.attached_to == Some(spare)
                && matches!(item.kind, the_long_slop::reference::ItemKind::Tire)
        })
        .unwrap()
        .id;
    let cover_slot = reference
        .wheel_parts
        .iter()
        .find(|p| p.item == rim)
        .unwrap()
        .cover_slot
        .unwrap();
    let cover = run.area.interaction.parts.slot_item(cover_slot).unwrap();
    let conditions =
        [rim, tire, original_tire, cover].map(|id| run.area.conditions.item_quality(id));
    let sensor_id = reference
        .action_bindings
        .iter()
        .find(|binding| {
            binding.pickup_owner == Some(rim)
                && binding.removal_owner == Some(original_tire)
                && reference
                    .colliders
                    .iter()
                    .any(|c| c.id == binding.collider && c.sensor)
        })
        .unwrap()
        .collider;
    let sensor = run
        .physics
        .rapier()
        .colliders
        .iter()
        .find(|(_, c)| (c.user_data >> 64) as u32 == sensor_id.0)
        .unwrap()
        .0;
    assert!(!run.physics.rapier().colliders[sensor].is_enabled());
    run.wheels
        .split(spare, &mut run.area.items, &mut run.physics)
        .unwrap();
    let point = run.area.items.get(tire).unwrap().pose.position;
    run.area
        .interaction
        .pickup_item(
            tire,
            point,
            &run.area.player,
            &mut run.area.items,
            &mut run.physics,
        )
        .unwrap();
    let held_body = run.area.items.get(tire).unwrap().body;
    let bodies = run.physics.rapier().bodies.len();
    let joints = run.physics.rapier().impulse_joints.len();
    let target = AimTarget {
        collider: sensor,
        item: rim,
        point,
        distance: 1.,
        slot: Some(SlotTarget::Tire(rim)),
    };
    let aim = ResolvedAim {
        installation_target: Some(target),
        ..ResolvedAim::default()
    };
    assert_eq!(
        run.area
            .interaction
            .available_actions(
                &aim,
                &run.area.player,
                &run.area.items,
                &run.physics,
                run.survival.output()
            )
            .actions
            .0
            & ActionMask::MOUNT,
        0
    );
    assert!(
        run.area
            .interaction
            .join_wheel(
                rim,
                tire,
                &mut run.wheels,
                &mut run.area.items,
                &mut run.physics
            )
            .is_err()
    );
    assert_eq!(run.area.interaction.held(), Some(tire));
    assert_eq!(held_body, run.area.items.get(tire).unwrap().body);
    assert_eq!(joints, run.physics.rapier().impulse_joints.len());
    assert_eq!(bodies, run.physics.rapier().bodies.len());

    run.area
        .interaction
        .parts
        .remove(cover, &mut run.area.items, &mut run.physics)
        .unwrap();
    run.wheels
        .remove(slot.id, &mut run.area.items, &mut run.physics)
        .unwrap();
    assert!(run.physics.rapier().colliders[sensor].is_enabled());
    assert!(
        run.area
            .interaction
            .join_wheel(
                rim,
                tire,
                &mut run.wheels,
                &mut run.area.items,
                &mut run.physics
            )
            .is_err()
    );
    assert_eq!(run.area.interaction.held(), Some(tire));
    assert_eq!(
        run.area.items.get(original_tire).unwrap().location,
        ItemLocation::Attached(rim)
    );
    assert_eq!(joints, run.physics.rapier().impulse_joints.len());
    assert!(
        run.area
            .interaction
            .join_wheel(
                ItemId(u32::MAX),
                tire,
                &mut run.wheels,
                &mut run.area.items,
                &mut run.physics
            )
            .is_err()
    );
    assert_eq!(run.area.interaction.held(), Some(tire));

    run.wheels
        .split(rim, &mut run.area.items, &mut run.physics)
        .unwrap();
    // A player moves the detached wheel away before working on it.
    let rim_body = run.area.items.get(rim).unwrap().body.unwrap();
    run.physics.edit_body(rim_body, |body| {
        body.set_translation(body.translation() + Vector::Y * 5., true)
    });
    run.area.items.sync_poses(&run.physics);
    run.area
        .interaction
        .sync_parts(&mut run.area.items, &mut run.physics)
        .unwrap();
    run.physics.refresh_queries();
    run.area
        .interaction
        .parts
        .install(cover, cover_slot, &mut run.area.items, &mut run.physics)
        .unwrap();
    run.wheels
        .sync(&mut run.area.items, &mut run.physics)
        .unwrap();
    assert!(
        run.wheels
            .can_install(rim, slot.id, &run.area.items)
            .is_err()
    );
    let aim = ray_aim(&run, sensor, |aim| {
        aim.installation_target
            .is_some_and(|t| t.slot == Some(SlotTarget::Tire(rim)))
    });
    let available = run.area.interaction.available_actions(
        &aim,
        &run.area.player,
        &run.area.items,
        &run.physics,
        run.survival.output(),
    );
    assert_ne!(available.actions.0 & ActionMask::MOUNT, 0);
    // Release the original pickup press before a later mount press.
    assert_eq!(
        input(
            &mut run,
            FrameInput {
                released: keys::GRAB,
                ..FrameInput::default()
            },
            aim
        ),
        None
    );
    assert_eq!(run.area.interaction.held(), Some(tire));
    assert_eq!(
        input(
            &mut run,
            FrameInput {
                pressed: keys::GRAB,
                held: keys::GRAB,
                ..FrameInput::default()
            },
            aim
        ),
        None
    );
    let action = input(
        &mut run,
        FrameInput {
            released: keys::GRAB,
            ..FrameInput::default()
        },
        aim,
    );
    assert_eq!(
        action,
        Some(InteractionAction::Wheel(WheelAction::Join { rim, tire }))
    );
    run.area
        .interaction
        .join_wheel(
            rim,
            tire,
            &mut run.wheels,
            &mut run.area.items,
            &mut run.physics,
        )
        .unwrap();
    assert_eq!(run.area.interaction.held(), None);
    let expected_mass: f32 = [rim, tire, cover]
        .iter()
        .map(|id| {
            reference
                .items
                .iter()
                .find(|item| item.id == *id)
                .unwrap()
                .mass_kg
        })
        .sum();
    let body = run.area.items.get(rim).unwrap().body.unwrap();
    assert!((run.physics.rapier().bodies[body].mass() - expected_mass).abs() < 0.01);
    assert_eq!(
        run.area.items.get(tire).unwrap().location,
        ItemLocation::Attached(rim)
    );
    assert_eq!(
        run.area.items.get(original_tire).unwrap().location,
        ItemLocation::World
    );

    // Resolve a real ray after the tire swap. The source slot binding names the old tire.
    let aim = ray_aim(&run, sensor, |aim| {
        aim.grab_target.is_some_and(|target| target.item == tire)
    });
    let available = run.area.interaction.available_actions(
        &aim,
        &run.area.player,
        &run.area.items,
        &run.physics,
        run.survival.output(),
    );
    assert_ne!(available.actions.0 & ActionMask::DISMOUNT, 0);
    let point = aim.grab_target.unwrap().point;
    let action = input(
        &mut run,
        FrameInput {
            pressed: keys::GRAB,
            held: keys::GRAB,
            ..FrameInput::default()
        },
        aim,
    );
    assert_eq!(
        action,
        Some(InteractionAction::Wheel(WheelAction::Split(rim)))
    );
    assert_eq!(
        run.area
            .interaction
            .split_wheel(
                rim,
                point,
                &run.area.player,
                &mut run.wheels,
                &mut run.area.items,
                &mut run.physics
            )
            .unwrap(),
        tire
    );
    assert_eq!(run.area.interaction.held(), Some(tire));
    assert_eq!(
        run.area.items.get(tire).unwrap().location,
        ItemLocation::Held
    );
    run.area
        .interaction
        .drop_held(&mut run.area.items, &mut run.physics);
    let point = run.area.items.get(rim).unwrap().pose.position;
    run.area
        .interaction
        .pickup_item(
            rim,
            point,
            &run.area.player,
            &mut run.area.items,
            &mut run.physics,
        )
        .unwrap();
    let joints = run.physics.rapier().impulse_joints.len();
    assert!(
        run.area
            .interaction
            .join_wheel(
                rim,
                original_tire,
                &mut run.wheels,
                &mut run.area.items,
                &mut run.physics
            )
            .is_err()
    );
    assert!(
        run.area
            .interaction
            .split_wheel(
                rim,
                point,
                &run.area.player,
                &mut run.wheels,
                &mut run.area.items,
                &mut run.physics
            )
            .is_err()
    );
    assert_eq!(run.area.interaction.held(), Some(rim));
    assert_eq!(
        run.area.items.get(original_tire).unwrap().location,
        ItemLocation::World
    );
    assert_eq!(joints, run.physics.rapier().impulse_joints.len());
    assert_eq!(
        conditions,
        [rim, tire, original_tire, cover].map(|id| run.area.conditions.item_quality(id))
    );
}
