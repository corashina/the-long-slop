mod common;
use my_summer_slop::simulation::{FrameInput, keys};
use my_summer_slop::starting_area::GarageRuntime;

fn initial() -> (my_summer_slop::reference::ReferenceManifest, GarageRuntime) {
    let mut reference = common::reference();
    let item = reference.items[0].id;
    let node = reference.items[0].node;
    let mut body = reference.bodies[1].clone();
    body.id = my_summer_slop::reference::BodyId(90);
    body.node = node;
    body.mass_kg = 7.0;
    body.pose.translation = [10.0, 2.0, -1.0];
    reference.bodies.push(body);
    reference.items[0].body = my_summer_slop::reference::BodyId(90);
    reference.items[0].attached_to = None;
    reference
        .nodes
        .iter_mut()
        .find(|record| record.id == node)
        .unwrap()
        .transform
        .translation = [10.0, 2.0, -1.0];
    let mut collider = reference.colliders[0].clone();
    collider.id = my_summer_slop::reference::ColliderId(90);
    collider.body = my_summer_slop::reference::BodyId(90);
    collider.node = node;
    collider.geometry = my_summer_slop::reference::ColliderGeometry::Sphere {
        center: [0.0; 3],
        radius: 0.2,
    };
    reference.colliders.push(collider);
    reference.area.player_spawn.translation = [10.0, 1.0, 0.0];
    assert_eq!(reference.items[0].id, item);
    let runtime = GarageRuntime::new(&reference).expect("garage state");
    (reference, runtime)
}
fn fluids(runtime: &GarageRuntime) -> Vec<(u32, [f32; 6])> {
    runtime
        .area
        .items
        .iter()
        .filter(|item| item.id.0 < 0x80000000)
        .filter_map(|item| {
            item.fluid
                .as_ref()
                .map(|fluid| (item.id.0, fluid.mixture.quantities_liters))
        })
        .collect()
}
#[test]
fn pause_freezes_complete_state() {
    let (reference, mut runtime) = initial();
    runtime.started = true;
    runtime.paused = true;
    let pose = runtime.physics.snapshot().pose;
    let quantities = fluids(&runtime);
    let engine = runtime.engine.clone();
    let hinges: Vec<_> = reference
        .hinges
        .iter()
        .map(|hinge| runtime.area.interaction.hinge_angle(hinge.item))
        .collect();
    let input = FrameInput {
        held: keys::FORWARD | keys::IGNITION | keys::USE | keys::FILL,
        pressed: keys::SPACE | keys::IGNITION,
        mouse: [10.0; 2],
        ..FrameInput::default()
    };
    for _ in 0..60 {
        runtime.step(&reference, input);
    }
    assert_eq!(runtime.physics.snapshot().pose, pose);
    assert_eq!(fluids(&runtime), quantities);
    assert_eq!(runtime.engine.rpm, engine.rpm);
    assert_eq!(runtime.engine.temperature, engine.temperature);
    assert_eq!(runtime.engine.ignition, engine.ignition);
    assert_eq!(runtime.trip_m, 0.0);
    for (hinge, angle) in reference.hinges.iter().zip(hinges) {
        assert_eq!(runtime.area.interaction.hinge_angle(hinge.item), angle);
    }
}
#[test]
fn restart_after_carry_detach_rebase() {
    let (reference, mut runtime) = initial();
    let count = (
        runtime.physics.rapier().bodies.len(),
        runtime.physics.collider_count(),
    );
    let ids: Vec<_> = runtime.area.items.iter().map(|item| item.id).collect();
    let quantities = fluids(&runtime);
    for _ in 0..3 {
        runtime.area.interaction.step(
            FrameInput {
                held: keys::GRAB,
                pressed: keys::GRAB,
                ..FrameInput::default()
            },
            Some(my_summer_slop::interaction::AimTarget {
                item: reference.items[0].id,
                distance: 1.0,
                point: [10.0, 2.0, -1.0],
                slot: None,
                part_slot: false,
            }),
            &mut runtime.area.player,
            &mut runtime.area.items,
            &mut runtime.physics,
            1.0 / 60.0,
        );
        assert_eq!(runtime.area.interaction.held(), Some(reference.items[0].id));
        assert!(!runtime.physics.rapier().impulse_joints.is_empty());
        let removed = runtime
            .wheels
            .remove(
                reference.wheel_slots[0].id,
                &mut runtime.area.items,
                &mut runtime.physics,
            )
            .unwrap();
        assert_eq!(
            runtime.area.items.get(removed).unwrap().location,
            my_summer_slop::interaction::items::ItemLocation::World
        );
        runtime.physics.rebase([1280.0, 0.0, -1280.0]);
        runtime.stream.origin.shift([1280.0, 0.0, -1280.0]);
        runtime.area.sync_origin(runtime.stream.origin.offset);
        for item in runtime.area.items.iter_mut() {
            if let Some(fluid) = item.fluid.as_mut() {
                fluid.mixture.quantities_liters = [0.0; 6];
            }
        }
        runtime.trip_m = 100.0;
        runtime.handbrake = false;
        runtime.restart(&reference).unwrap();
        assert_eq!(
            (
                runtime.physics.rapier().bodies.len(),
                runtime.physics.collider_count()
            ),
            count
        );
        assert_eq!(
            runtime
                .area
                .items
                .iter()
                .map(|item| item.id)
                .collect::<Vec<_>>(),
            ids
        );
        assert_eq!(fluids(&runtime), quantities);
        assert_eq!(runtime.stream.origin.offset, [0.0; 3]);
        assert_eq!(runtime.trip_m, 0.0);
        assert!(runtime.handbrake);
        assert!(!runtime.engine.ignition);
        assert!(runtime.physics.rapier().impulse_joints.is_empty());
        assert_eq!(runtime.area.interaction.held(), None);
    }
}
#[test]
fn recovery_preserves_state() {
    let (reference, mut runtime) = initial();
    runtime.trip_m = 256.0;
    for item in runtime.area.items.iter_mut() {
        item.condition = 4 - item.condition;
    }
    runtime
        .area
        .items
        .get_mut(reference.engine.fuel_tank)
        .unwrap()
        .fluid
        .as_mut()
        .unwrap()
        .mixture
        .quantities_liters = [0.0; 6];
    let quantities = fluids(&runtime);
    // Station supplies stream with the road. Recovery may load another one.
    let parts = |runtime: &GarageRuntime| {
        runtime
            .area
            .items
            .iter()
            .filter(|item| item.id.0 < 0x8000_0000)
            .map(|item| (item.id, item.location, item.condition))
            .collect::<Vec<_>>()
    };
    let before = parts(&runtime);
    let body_count = runtime.physics.rapier().bodies.len();
    runtime.recover(&reference);
    assert_eq!(fluids(&runtime), quantities);
    assert_eq!(parts(&runtime), before);
    assert_eq!(runtime.trip_m, 256.0);
    assert_eq!(runtime.physics.rapier().bodies.len(), body_count);
}

/// Run with locally exported installed-build assets.
#[test]
#[ignore = "Requires exported Steam assets"]
fn production_garage_bindings_fill_start_and_move() {
    let bytes = std::fs::read("web/assets/reference/manifest.json").expect("exported manifest");
    let reference = my_summer_slop::reference::ReferenceManifest::from_json(&bytes).unwrap();
    verify_source_passenger_controls(&reference);
    let mut runtime = GarageRuntime::new(&reference).unwrap();
    assert_eq!(runtime.area.bodies.len(), reference.bodies.len());
    for collider in &reference.colliders {
        if collider.body != reference.vehicle.body
            && my_summer_slop::starting_area::belongs_to_vehicle(&reference, collider.body)
        {
            let shape = runtime
                .physics
                .rapier()
                .colliders
                .iter()
                .find(|(_, shape)| ((shape.user_data >> 64) as u32) == collider.id.0)
                .unwrap()
                .1;
            assert_eq!(
                shape.parent(),
                Some(runtime.physics.chassis_handle()),
                "car hinge shape must use source compound body"
            );
        }
    }
    assert_eq!(
        runtime.area.items.len(),
        reference.items.len()
            + runtime
                .area
                .items
                .iter()
                .filter(|item| item.id.0 >= 0x80000000)
                .count()
    );
    let faucet = reference
        .openings
        .iter()
        .find(|opening| opening.can_tune)
        .expect("source faucet");
    let faucet_target = aim_at(&mut runtime, &reference, faucet.cap);
    assert_eq!(runtime.area.interaction.opening_valve(faucet.id), 0.0);
    runtime.area.interaction.step(
        FrameInput {
            held: keys::USE,
            mouse: [0.0, 500.0],
            ..FrameInput::default()
        },
        Some(faucet_target),
        &mut runtime.area.player,
        &mut runtime.area.items,
        &mut runtime.physics,
        1.0 / 60.0,
    );
    assert_eq!(runtime.area.interaction.opening_valve(faucet.id), 1.0);
    let opening = reference
        .openings
        .iter()
        .find(|opening| opening.tank == reference.engine.fuel_tank)
        .unwrap();
    let cap = aim_at(&mut runtime, &reference, opening.cap);
    runtime.area.interaction.step(
        FrameInput {
            pressed: keys::USE,
            ..FrameInput::default()
        },
        Some(cap),
        &mut runtime.area.player,
        &mut runtime.area.items,
        &mut runtime.physics,
        1.0 / 60.0,
    );
    assert!(runtime.area.interaction.opening_is_open(opening.id));
    let can = reference
        .tanks
        .iter()
        .find(|tank| {
            tank.item != reference.engine.fuel_tank
                && tank.quantities_liters[0] > 0.1
                && reference
                    .items
                    .iter()
                    .find(|item| item.id == tank.item)
                    .unwrap()
                    .body
                    != reference.vehicle.body
        })
        .unwrap()
        .item;
    // Stage the source can near the opening. Browser checks cover the walking route.
    let mut staged = runtime.area.items.get(can).unwrap().pose;
    let view = my_summer_slop::vehicle::to_rapier(runtime.area.player.view_pose(&runtime.physics));
    staged.position =
        (view.translation + view.rotation * rapier3d::math::Vector::new(0.0, 0.0, -0.6)).to_array();
    let can_body = runtime.area.items.get(can).unwrap().body.unwrap();
    runtime.physics.rapier_mut().bodies[can_body]
        .set_position(my_summer_slop::vehicle::to_rapier(staged), true);
    runtime.physics.refresh_queries();
    runtime.area.items.sync_poses(&runtime.physics);
    let can_target = aim_at(&mut runtime, &reference, can);
    runtime.area.interaction.step(
        FrameInput {
            pressed: keys::GRAB,
            ..FrameInput::default()
        },
        Some(can_target),
        &mut runtime.area.player,
        &mut runtime.area.items,
        &mut runtime.physics,
        1.0 / 60.0,
    );
    assert_eq!(runtime.area.interaction.held(), Some(can));
    let cap = aim_at(&mut runtime, &reference, opening.cap);
    let fuel_before = runtime
        .area
        .items
        .get(reference.engine.fuel_tank)
        .unwrap()
        .fluid
        .as_ref()
        .unwrap()
        .mixture
        .total();
    let fill_action = runtime.area.interaction.step(
        FrameInput {
            held: keys::FILL,
            ..FrameInput::default()
        },
        Some(cap),
        &mut runtime.area.player,
        &mut runtime.area.items,
        &mut runtime.physics,
        1.0 / 60.0,
    );
    assert!(matches!(
        fill_action,
        Some(my_summer_slop::interaction::InteractionAction::Fill { .. })
    ));
    let fuel_after = runtime
        .area
        .items
        .get(reference.engine.fuel_tank)
        .unwrap()
        .fluid
        .as_ref()
        .unwrap()
        .mixture
        .total();
    assert!(fuel_after > fuel_before, "aimed fill did not transfer fuel");
    runtime
        .area
        .interaction
        .drop_held(&mut runtime.area.items, &mut runtime.physics);
    enter_source_driver(&mut runtime, &reference);
    runtime.started = true;
    for step in 0..240 {
        runtime.step(
            &reference,
            FrameInput {
                held: keys::IGNITION,
                pressed: if step == 0 { keys::IGNITION } else { 0 },
                ..FrameInput::default()
            },
        );
    }
    assert!(
        runtime.engine_output.running,
        "engine did not start: {:?}",
        runtime.engine_output
    );
    let before = runtime.physics.snapshot().pose.position;
    for step in 0..180 {
        runtime.step(
            &reference,
            FrameInput {
                held: keys::FORWARD,
                pressed: if step == 0 { keys::SPACE } else { 0 },
                released: if step == 0 { keys::IGNITION } else { 0 },
                ..FrameInput::default()
            },
        );
    }
    let after = runtime.physics.snapshot().pose.position;
    let travel = ((after[0] - before[0]).powi(2) + (after[2] - before[2]).powi(2)).sqrt();
    assert!(travel > 0.2, "engine force did not move the car: {travel}");
    println!(
        "{} source bodies, {} colliders, fuel transfer {}, engine {} rpm, car moved {} m",
        reference.bodies.len(),
        runtime.physics.collider_count(),
        fuel_after - fuel_before,
        runtime.engine_output.rpm,
        travel
    );
}

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
        .find(|(_, collider)| {
            collider.user_data as u32 == item.0 + 1
                && collider.user_data & my_summer_slop::interaction::parts::PART_SLOT_FLAG == 0
        })
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
        if let Some(target) = runtime.area.interaction.aim(
            &runtime.physics,
            runtime.area.player.view_pose(&runtime.physics),
            reference.player.aim_reach,
        ) && target.item == item
        {
            return target;
        }
    }
    panic!("cannot aim at source item {}", item.0);
}

#[test]
fn car_hinge_uses_chassis_compound_and_moves_its_shape() {
    use my_summer_slop::reference::{
        AnchorId, BodyId, ColliderId, HingeRecord, ItemId, ItemKind, NodeId, Transform,
    };
    use my_summer_slop::vehicle::collider_builder;
    let mut reference = common::reference();
    let source = reference.player.source.clone();
    let identity = Transform {
        translation: [0.0; 3],
        rotation: [0.0, 0.0, 0.0, 1.0],
        scale: [1.0; 3],
    };
    let mut node = reference.nodes[1].clone();
    node.id = NodeId(90);
    node.parent = Some(reference.vehicle.root);
    node.transform = Transform {
        translation: [0.82, 0.7, 0.0],
        ..identity
    };
    reference.nodes.push(node);
    let mut body = reference.bodies[1].clone();
    body.id = BodyId(90);
    body.node = NodeId(90);
    body.dynamic = false;
    body.parent_body = Some(reference.vehicle.body);
    body.mass_kg = 0.0;
    body.pose = Transform {
        translation: [0.82, 0.7, 0.0],
        ..identity
    };
    reference.bodies.push(body);
    let mut item = reference.items[0].clone();
    item.id = ItemId(90);
    item.node = NodeId(90);
    item.body = BodyId(90);
    item.kind = ItemKind::Door;
    item.attached_to = None;
    reference.items.push(item);
    let mut anchor = reference.anchors[0].clone();
    anchor.id = AnchorId(90);
    anchor.node = NodeId(90);
    anchor.body = BodyId(90);
    anchor.transform = identity;
    reference.anchors.push(anchor);
    reference.hinges.push(HingeRecord {
        item: ItemId(90),
        source,
        anchor: AnchorId(90),
        body: BodyId(90),
        axis: [0.0, 1.0, 0.0],
        closed_degrees: 0.0,
        min_degrees: 0.0,
        max_degrees: 90.0,
        drag_axis: [1.0, 0.0, 0.0],
        drag_input_axis: 0,
        drag_sensitivity: 1.0,
        input_inverted: [false, false],
        rotation_axes: [[0.0, 1.0, 0.0], [0.0; 3]],
        relative_drag: false,
        drag_anchor: AnchorId(90),
        motor_degrees_per_second: None,
    });
    let mut collider = reference.colliders[0].clone();
    collider.id = ColliderId(90);
    collider.node = NodeId(90);
    collider.body = BodyId(90);
    collider.sensor = false;
    collider.geometry = my_summer_slop::reference::ColliderGeometry::Box {
        center: [0.3, 0.0, 0.0],
        half_extents: [0.2, 0.5, 0.01],
        rotation: [0.0, 0.0, 0.0, 1.0],
    };
    let local = *collider_builder(&collider.geometry)
        .unwrap()
        .build()
        .position();
    reference.colliders.push(collider);
    let mut runtime = GarageRuntime::new(&reference).unwrap();
    let shape = runtime
        .physics
        .rapier()
        .colliders
        .iter()
        .find(|(_, shape)| ((shape.user_data >> 64) as u32) == 90)
        .unwrap()
        .0;
    assert_eq!(
        runtime.physics.rapier().colliders[shape].parent(),
        Some(runtime.physics.chassis_handle())
    );
    assert!(
        runtime.physics.rapier().bodies[runtime.area.bodies[&BodyId(90)]]
            .colliders()
            .is_empty()
    );
    let before = *runtime.physics.rapier().colliders[shape].position();
    runtime.area.interaction.step(
        FrameInput {
            held: keys::USE,
            pressed: keys::USE,
            mouse: [-45.0, 0.0],
            ..FrameInput::default()
        },
        Some(my_summer_slop::interaction::AimTarget {
            item: ItemId(90),
            distance: 1.0,
            point: [0.8, 0.7, 0.0],
            slot: None,
            part_slot: false,
        }),
        &mut runtime.area.player,
        &mut runtime.area.items,
        &mut runtime.physics,
        1.0 / 60.0,
    );
    assert_eq!(runtime.area.interaction.hinge_angle(ItemId(90)), Some(45.0));
    runtime.area.sync_compound(&mut runtime.physics);
    runtime.physics.refresh_queries();
    let expected =
        *runtime.physics.rapier().bodies[runtime.area.bodies[&BodyId(90)]].next_position() * local;
    let actual = *runtime.physics.rapier().colliders[shape].position();
    assert!((actual.translation - expected.translation).length() < 1.0e-4);
    assert!((actual.translation - before.translation).length() > 0.1);
    // Source vehicle geometry shares one body. Rapier cannot generate car self-contact pairs.
    runtime.physics.step_physics();
    for contact in runtime.physics.rapier().narrow_phase.contact_pairs() {
        let first = &runtime.physics.rapier().colliders[contact.collider1];
        let second = &runtime.physics.rapier().colliders[contact.collider2];
        assert!(
            !(contact.has_any_active_contact()
                && first.user_data & my_summer_slop::vehicle::VEHICLE_COLLIDER_FLAG != 0
                && second.user_data & my_summer_slop::vehicle::VEHICLE_COLLIDER_FLAG != 0)
        );
    }
}

#[test]
fn dry_recovery_keeps_a_carried_can_reachable() {
    let (reference, mut runtime) = initial();
    let can = reference.items[0].id;
    runtime.area.interaction.step(
        FrameInput {
            held: keys::GRAB,
            pressed: keys::GRAB,
            ..FrameInput::default()
        },
        Some(my_summer_slop::interaction::AimTarget {
            item: can,
            distance: 1.0,
            point: [10.0, 2.0, -1.0],
            slot: None,
            part_slot: false,
        }),
        &mut runtime.area.player,
        &mut runtime.area.items,
        &mut runtime.physics,
        1.0 / 60.0,
    );
    assert_eq!(runtime.area.interaction.held(), Some(can));
    runtime
        .area
        .items
        .get_mut(reference.engine.fuel_tank)
        .unwrap()
        .fluid
        .as_mut()
        .unwrap()
        .mixture
        .quantities_liters = [0.0; 6];
    let quantities = fluids(&runtime);
    runtime.trip_m = 256.0;
    runtime.recover(&reference);
    let view = runtime.area.player.view_pose(&runtime.physics);
    let can_pose = runtime.area.items.get(can).unwrap().pose;
    let distance = view
        .position
        .iter()
        .zip(can_pose.position)
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f32>()
        .sqrt();
    assert!(
        distance < reference.player.aim_reach,
        "carried can was left behind: {distance}"
    );
    assert_eq!(fluids(&runtime), quantities);
    assert_eq!(runtime.trip_m, 256.0);
    assert_eq!(runtime.area.interaction.held(), None);
    assert!(runtime.physics.rapier().impulse_joints.is_empty());
}

#[test]
#[ignore = "Requires exported Steam assets"]
fn production_dry_recovery_keeps_source_can_and_cargo() {
    let bytes = std::fs::read("web/assets/reference/manifest.json").unwrap();
    let reference = my_summer_slop::reference::ReferenceManifest::from_json(&bytes).unwrap();
    let mut runtime = GarageRuntime::new(&reference).unwrap();
    let can = reference
        .tanks
        .iter()
        .find(|tank| {
            tank.role == my_summer_slop::reference::TankRole::Portable
                && tank.quantities_liters[0] > 0.1
        })
        .unwrap()
        .item;
    let target = aim_at(&mut runtime, &reference, can);
    runtime.area.interaction.step(
        FrameInput {
            pressed: keys::GRAB,
            ..FrameInput::default()
        },
        Some(target),
        &mut runtime.area.player,
        &mut runtime.area.items,
        &mut runtime.physics,
        1.0 / 60.0,
    );
    assert_eq!(runtime.area.interaction.held(), Some(can));
    runtime
        .area
        .items
        .get_mut(reference.engine.fuel_tank)
        .unwrap()
        .fluid
        .as_mut()
        .unwrap()
        .mixture
        .quantities_liters = [0.0; 6];
    let before = fluids(&runtime);
    let origin = runtime.stream.origin.offset;
    runtime.recover(&reference);
    assert_eq!(fluids(&runtime), before);
    assert_eq!(runtime.stream.origin.offset, origin);
    let view = runtime.area.player.view_pose(&runtime.physics);
    let can_pose = runtime.area.items.get(can).unwrap().pose;
    assert!(
        view.position
            .iter()
            .zip(can_pose.position)
            .map(|(a, b)| (a - b) * (a - b))
            .sum::<f32>()
            .sqrt()
            < reference.player.aim_reach
    );
    let car = runtime
        .stream
        .origin
        .absolute(runtime.physics.snapshot().pose.position);
    let station = runtime.stream.nearest_station(car);
    assert!(
        ((car[0] - station.point[0]).powi(2) + (car[2] - station.point[2]).powi(2)).sqrt() < 0.01
    );
    // Stage the same source can as trunk cargo. Recovery must preserve its car-relative pose.
    let body = runtime.area.items.get(can).unwrap().body.unwrap();
    let mut cargo = my_summer_slop::vehicle::to_rapier(runtime.physics.snapshot().pose);
    cargo.translation += cargo.rotation * rapier3d::math::Vector::new(0.0, 0.5, 0.0);
    runtime.physics.rapier_mut().bodies[body].set_position(cargo, true);
    runtime.physics.refresh_queries();
    runtime.area.items.sync_poses(&runtime.physics);
    let old_car = my_summer_slop::vehicle::to_rapier(runtime.physics.snapshot().pose);
    let relative = old_car.inverse() * *runtime.physics.rapier().bodies[body].position();
    let next = my_summer_slop::vehicle::VehiclePose {
        position: [
            old_car.translation.x + 20.0,
            old_car.translation.y,
            old_car.translation.z - 1400.0,
        ],
        rotation: old_car.rotation.to_array(),
    };
    runtime.physics.reset(next);
    runtime.physics.rapier_mut().bodies[body]
        .set_position(my_summer_slop::vehicle::to_rapier(next) * relative, true);
    runtime.recover(&reference);
    let expected = my_summer_slop::vehicle::to_rapier(runtime.physics.snapshot().pose) * relative;
    let actual = *runtime.physics.rapier().bodies[body].position();
    assert!(
        (actual.translation - expected.translation).length() < 0.001,
        "stored can was left behind"
    );
    assert_eq!(fluids(&runtime), before);
    assert_eq!(
        runtime.physics.rapier().bodies[body].linvel(),
        rapier3d::math::Vector::ZERO
    );
    assert_eq!(
        runtime.physics.rapier().bodies[body].angvel(),
        rapier3d::math::Vector::ZERO
    );
}

#[test]
fn walking_far_keeps_a_parked_road_car_supported() {
    let mut physics = common::world();
    let mut stream = my_summer_slop::world::WorldStream::new(&common::reference());
    let pose = stream.reset_pose(false, &mut physics);
    physics.reset(pose);
    let parked = |physics: &mut my_summer_slop::vehicle::PhysicsWorld, steps: usize| {
        for _ in 0..steps {
            physics.apply_vehicle_forces(
                my_summer_slop::simulation::DriveInput {
                    handbrake: true,
                    ..Default::default()
                },
                0.0,
            );
            physics.step_physics();
        }
    };
    parked(&mut physics, 180);
    let before = physics.snapshot().pose.position;
    let car = stream.origin.absolute(before);
    stream.sync_at([car[0] + 400.0, car[1], car[2] - 3000.0], &mut physics);
    parked(&mut physics, 120);
    let after = physics.snapshot().pose.position;
    assert!(
        after[1] > before[1] - 0.1,
        "parked car lost its ground when walking focus moved: {} -> {}",
        before[1],
        after[1]
    );
    assert!(
        physics
            .snapshot()
            .wheels_contact
            .iter()
            .all(|contact| *contact),
        "the car keeps its own terrain tiles"
    );
    stream.sync_at(stream.origin.absolute(after), &mut physics);
    let count = physics.collider_count();
    stream.sync_at([car[0] + 1.0, car[1], car[2]], &mut physics);
    assert_eq!(
        physics.collider_count(),
        count,
        "returning focus duplicated terrain support"
    );
}

#[test]
#[ignore = "Requires exported Steam assets"]
fn production_parked_garage_car_stays_stable() {
    let bytes = std::fs::read("web/assets/reference/manifest.json").unwrap();
    let reference = my_summer_slop::reference::ReferenceManifest::from_json(&bytes).unwrap();
    let mut runtime = GarageRuntime::new(&reference).unwrap();
    runtime.started = true;
    let initial = runtime.physics.snapshot();
    for _ in 0..300 {
        runtime.step(&reference, FrameInput::default());
    }
    let settled = runtime.physics.snapshot();
    let mut max_drift = 0.0_f32;
    let mut max_rotation = 0.0_f32;
    let mut unsupported_steps = 0;
    for step in 0..5400 {
        runtime.step(&reference, FrameInput::default());
        let state = runtime.physics.snapshot();
        if runtime.physics.rapier().bodies[runtime.physics.chassis_handle()].is_sleeping() {
            assert_eq!(
                state.linear_velocity, [0.0; 3],
                "sleeping car accumulated a linear impulse"
            );
            assert_eq!(
                state.angular_velocity, [0.0; 3],
                "sleeping car accumulated an angular impulse"
            );
        }
        let drift = ((state.pose.position[0] - settled.pose.position[0]).powi(2)
            + (state.pose.position[2] - settled.pose.position[2]).powi(2))
        .sqrt();
        let rotation = my_summer_slop::vehicle::to_rapier(settled.pose)
            .rotation
            .angle_between(my_summer_slop::vehicle::to_rapier(state.pose).rotation)
            .to_degrees();
        max_drift = max_drift.max(drift);
        max_rotation = max_rotation.max(rotation);
        if !state.wheels_contact.iter().all(|contact| *contact) {
            unsupported_steps += 1;
        }
        if (step + 1) % 900 == 0 {
            println!(
                "idle {}s pose {:?}, drift {drift}m, rotation {rotation}deg, contacts {:?}, velocity {:?}, trip {}m",
                (step + 1) / 60,
                state.pose.position,
                state.wheels_contact,
                state.linear_velocity,
                runtime.trip_m
            );
        }
    }
    let final_state = runtime.physics.snapshot();
    println!(
        "parked source initial {:?}, settled {:?}, final {:?}, maximum drift {max_drift}m, rotation {max_rotation}deg, unsupported {unsupported_steps}/5400",
        initial.pose.position, settled.pose.position, final_state.pose.position
    );
    assert!(runtime.handbrake);
    assert!(!runtime.engine_output.ignition && !runtime.engine_output.running);
    assert!(
        max_drift < 0.05,
        "parked source car moved {max_drift}m without input"
    );
    assert!(
        max_rotation < 1.0,
        "parked source car rotated {max_rotation}deg without input"
    );
    assert_eq!(unsupported_steps, 0, "parked source car lost wheel support");
}

#[test]
#[ignore = "Requires exported Steam assets"]
fn production_garage_exits_and_reverse_to_road() {
    let bytes = std::fs::read("web/assets/reference/manifest.json").unwrap();
    let reference = my_summer_slop::reference::ReferenceManifest::from_json(&bytes).unwrap();
    let mut runtime = GarageRuntime::new(&reference).unwrap();
    runtime.started = true;
    for _ in 0..5700 {
        runtime.step(&reference, FrameInput::default());
    }
    assert!(
        runtime.physics.rapier().bodies[runtime.physics.chassis_handle()].is_sleeping(),
        "source car did not sleep during garage preparation"
    );
    for source in [64783, 64777] {
        let hinge = reference
            .hinges
            .iter()
            .find(|hinge| hinge.source.path_id == source)
            .expect("recovered garage exit leaf");
        open_source_hinge(&mut runtime, &reference, source);
        let body = runtime.area.bodies[&hinge.body];
        assert!(runtime.physics.rapier().bodies[body].is_kinematic());
        assert!(!runtime.physics.rapier().bodies[body].colliders().is_empty());
        println!(
            "garage leaf {source} opened to {} degrees",
            runtime.area.interaction.hinge_angle(hinge.item).unwrap()
        );
    }
    runtime.step(&reference, FrameInput::default());
    enter_source_driver(&mut runtime, &reference);
    for step in 0..30 {
        runtime.step(
            &reference,
            FrameInput {
                held: if step == 0 { keys::RIGHT } else { 0 },
                ..FrameInput::default()
            },
        );
        let state = runtime.physics.snapshot();
        assert!(
            state.wheels_contact.iter().all(|contact| *contact),
            "steering woke the resting car into the air at step {step}: {:?}, velocity {:?}",
            state.pose.position,
            state.linear_velocity
        );
    }
    for step in 0..240 {
        runtime.step(
            &reference,
            FrameInput {
                held: keys::IGNITION,
                pressed: if step == 0 { keys::IGNITION } else { 0 },
                ..FrameInput::default()
            },
        );
    }
    assert!(runtime.engine_output.running, "engine did not start");
    let start = runtime.physics.snapshot().pose.position;
    let mut crossed = false;
    let mut joined = false;
    for step in 0..1200 {
        runtime.step(
            &reference,
            FrameInput {
                held: keys::BACK,
                pressed: if step == 0 { keys::SPACE } else { 0 },
                released: if step == 0 { keys::IGNITION } else { 0 },
                ..FrameInput::default()
            },
        );
        let state = runtime.physics.snapshot();
        if state.pose.position[0] < 33.0 {
            crossed = true;
        }
        let absolute = runtime.stream.origin.absolute(state.pose.position);
        let (road, yaw) = runtime.stream.nearest_road(absolute);
        let lateral = ((absolute[0] - road[0]) * yaw.cos() as f64
            - (absolute[2] - road[2]) * yaw.sin() as f64)
            .abs();
        if lateral > 4.0 {
            continue;
        }
        assert!(
            state.pose.position.iter().all(|value| value.is_finite())
                && state.pose.position[1] > 0.0,
            "car left the terrain at the road join"
        );
        // The source terrain sits 0.1 meters below the road within 4.5 meters of it.
        // A wheel can leave that lip for a moment. All four must land within one second.
        let mut settled = None;
        for extra in 0..60 {
            if runtime
                .physics
                .snapshot()
                .wheels_contact
                .iter()
                .all(|contact| *contact)
            {
                settled = Some(extra);
                break;
            }
            runtime.step(
                &reference,
                FrameInput {
                    held: keys::BACK,
                    ..FrameInput::default()
                },
            );
        }
        let landed = runtime.physics.snapshot();
        assert!(
            settled.is_some(),
            "car lost wheel support at the road join: {:?}, contacts {:?}",
            landed.pose.position,
            landed.wheels_contact
        );
        println!(
            "source road joined after {} reverse steps at {:?}; road bone {:?}; all wheels down after {} more steps",
            step + 1,
            absolute,
            road,
            settled.unwrap()
        );
        joined = true;
        break;
    }
    let exit = runtime.physics.snapshot();
    assert!(
        crossed,
        "car did not reverse beyond garage exit plane x35.21: {start:?} -> {:?}, engine {:?}",
        exit.pose.position, runtime.engine_output
    );
    assert!(exit.pose.position[0] < 35.21);
    assert!(
        joined,
        "car did not join the road within 20 seconds: {:?}, engine {:?}",
        exit.pose.position, runtime.engine_output
    );
    for step in 0..180 {
        runtime.step(
            &reference,
            FrameInput {
                pressed: if step == 0 { keys::SPACE } else { 0 },
                ..FrameInput::default()
            },
        );
    }
    let parked = runtime.physics.snapshot();
    assert!(
        parked.wheels_contact.iter().all(|contact| *contact),
        "car lost terrain support after braking: {:?}",
        parked.pose.position
    );
    assert!(parked.pose.position[1] > 0.0, "car fell below the terrain");
    println!(
        "source Lada reversed {:?} -> {:?}; terrain contacts after braking {:?}",
        start, parked.pose.position, parked.wheels_contact
    );
}

fn open_source_hinge(
    runtime: &mut GarageRuntime,
    reference: &my_summer_slop::reference::ReferenceManifest,
    source: i64,
) {
    let hinge = reference
        .hinges
        .iter()
        .find(|hinge| hinge.source.path_id == source)
        .unwrap();
    runtime.started = true;
    runtime.step(reference, FrameInput::default());
    aim_at(runtime, reference, hinge.item);
    let span = (hinge.max_degrees - hinge.closed_degrees)
        .abs()
        .max((hinge.min_degrees - hinge.closed_degrees).abs());
    for mouse in [[0.0, 50.0], [0.0, -50.0], [50.0, 0.0], [-50.0, 0.0]] {
        for step in 0..180 {
            runtime.step(
                reference,
                FrameInput {
                    held: keys::USE,
                    pressed: if step == 0 { keys::USE } else { 0 },
                    mouse,
                    ..FrameInput::default()
                },
            );
            if (runtime.area.interaction.hinge_angle(hinge.item).unwrap() - hinge.closed_degrees)
                .abs()
                > span * 0.9
            {
                runtime.step(reference, FrameInput::default());
                return;
            }
        }
        runtime.step(reference, FrameInput::default());
        aim_at(runtime, reference, hinge.item);
    }
    panic!("source hinge {source} did not open");
}

fn enter_source_driver(
    runtime: &mut GarageRuntime,
    reference: &my_summer_slop::reference::ReferenceManifest,
) {
    open_source_hinge(runtime, reference, 76010);
    let driver = reference.seats.iter().find(|seat| seat.driver).unwrap();
    let target = aim_at(runtime, reference, driver.item);
    assert_eq!(target.item, driver.item);
    runtime.started = true;
    runtime.step(
        reference,
        FrameInput {
            held: keys::USE,
            pressed: keys::USE,
            ..FrameInput::default()
        },
    );
    runtime.step(
        reference,
        FrameInput {
            released: keys::USE,
            ..FrameInput::default()
        },
    );
    assert_eq!(
        runtime.area.player.seat(),
        Some(driver.id),
        "driver seat must enter through actual aim and USE release"
    );
    assert!(runtime.area.player.driving());
    close_source_hinge(runtime, reference, 76010);
}

fn verify_source_passenger_controls(reference: &my_summer_slop::reference::ReferenceManifest) {
    let mut runtime = GarageRuntime::new(reference).unwrap();
    open_source_hinge(&mut runtime, reference, 70795);
    let passenger = reference
        .seats
        .iter()
        .find(|seat| seat.source.path_id == 66466)
        .expect("front passenger seat");
    assert!(!passenger.driver);
    aim_at(&mut runtime, reference, passenger.item);
    runtime.step(
        reference,
        FrameInput {
            held: keys::USE,
            pressed: keys::USE,
            ..FrameInput::default()
        },
    );
    runtime.step(
        reference,
        FrameInput {
            released: keys::USE,
            ..FrameInput::default()
        },
    );
    assert_eq!(
        runtime.area.player.seat(),
        Some(passenger.id),
        "passenger entry must use actual source aim"
    );
    assert!(!runtime.area.player.driving());
    for step in 0..180 {
        runtime.step(
            reference,
            FrameInput {
                held: keys::IGNITION | keys::FORWARD,
                pressed: if step == 0 {
                    keys::IGNITION | keys::SPACE
                } else {
                    0
                },
                ..FrameInput::default()
            },
        );
    }
    runtime.step(
        reference,
        FrameInput {
            released: keys::IGNITION,
            ..FrameInput::default()
        },
    );
    assert!(!runtime.engine_output.ignition);
    assert!(!runtime.engine_output.running);
    assert!(runtime.handbrake);
    println!(
        "source passenger seat entered through aim; engine and handbrake controls remained disabled"
    );
}

fn close_source_hinge(
    runtime: &mut GarageRuntime,
    reference: &my_summer_slop::reference::ReferenceManifest,
    source: i64,
) {
    let hinge = reference
        .hinges
        .iter()
        .find(|hinge| hinge.source.path_id == source)
        .unwrap();
    runtime.step(reference, FrameInput::default());
    if runtime.area.player.seat().is_some() {
        aim_from_seat(runtime, reference, hinge.item);
    } else {
        aim_at(runtime, reference, hinge.item);
    }
    for mouse in [[0.0, -50.0], [0.0, 50.0], [-50.0, 0.0], [50.0, 0.0]] {
        for step in 0..180 {
            runtime.step(
                reference,
                FrameInput {
                    held: keys::USE,
                    pressed: if step == 0 { keys::USE } else { 0 },
                    mouse,
                    ..FrameInput::default()
                },
            );
            if (runtime.area.interaction.hinge_angle(hinge.item).unwrap() - hinge.closed_degrees)
                .abs()
                < 0.1
            {
                runtime.step(reference, FrameInput::default());
                return;
            }
        }
        runtime.step(reference, FrameInput::default());
        if runtime.area.player.seat().is_some() {
            aim_from_seat(runtime, reference, hinge.item);
        } else {
            aim_at(runtime, reference, hinge.item);
        }
    }
    panic!("source hinge {source} did not close through native controls");
}

fn aim_from_seat(
    runtime: &mut GarageRuntime,
    reference: &my_summer_slop::reference::ReferenceManifest,
    item: my_summer_slop::reference::ItemId,
) {
    assert!(runtime.area.player.seat().is_some());
    let center = runtime
        .physics
        .rapier()
        .colliders
        .iter()
        .find(|(_, collider)| collider.user_data as u32 == item.0 + 1)
        .map(|(_, collider)| collider.compute_aabb().center())
        .unwrap();
    let view = my_summer_slop::vehicle::to_rapier(runtime.area.player.view_pose(&runtime.physics));
    let body = my_summer_slop::vehicle::to_rapier(runtime.area.player.body_pose(&runtime.physics));
    let delta = body.rotation.inverse() * (center - view.translation);
    let yaw = delta.x.atan2(-delta.z).to_degrees();
    let pitch = (-delta.y)
        .atan2((delta.x * delta.x + delta.z * delta.z).sqrt())
        .to_degrees();
    runtime.step(
        reference,
        FrameInput {
            mouse: [
                yaw - runtime.area.player.yaw_degrees(),
                pitch - runtime.area.player.pitch_degrees(),
            ],
            ..FrameInput::default()
        },
    );
    let target = runtime.area.interaction.aim(
        &runtime.physics,
        runtime.area.player.view_pose(&runtime.physics),
        reference.player.aim_reach,
    );
    assert_eq!(
        target.map(|target| target.item),
        Some(item),
        "seated door control must use real aim"
    );
}
#[test]
fn hard_crash_knocks_wheels_off() {
    use my_summer_slop::interaction::items::ItemLocation;
    let (reference, mut runtime) = initial();
    runtime.started = true;
    let rims: Vec<_> = reference.wheel_slots.iter().map(|slot| slot.rim).collect();
    let mounted = |runtime: &GarageRuntime| {
        rims.iter()
            .filter(|id| {
                matches!(
                    runtime.area.items.get(**id).unwrap().location,
                    ItemLocation::WheelSlot(_)
                )
            })
            .count()
    };
    let chassis = runtime.physics.chassis_handle();
    let forward = |runtime: &mut GarageRuntime, kmh: f32| {
        let rotation = *runtime.physics.rapier().bodies[chassis].rotation();
        runtime.physics.rapier_mut().bodies[chassis]
            .set_linvel(rotation * -rapier3d::math::Vector::Z * (kmh / 3.6), true);
    };
    for _ in 0..60 {
        forward(&mut runtime, 5.0);
        runtime.step(&reference, FrameInput::default());
    }
    assert_eq!(mounted(&runtime), 4, "gentle driving keeps every wheel");
    forward(&mut runtime, 100.0);
    runtime.step(&reference, FrameInput::default());
    assert!(mounted(&runtime) < 4, "a 95 km/h jump knocks wheels off");
    let loose = rims
        .iter()
        .find(|id| runtime.area.items.get(**id).unwrap().location == ItemLocation::World)
        .unwrap();
    let body = runtime.area.items.get(*loose).unwrap().body.unwrap();
    assert!(runtime.physics.rapier().bodies[body].is_dynamic());
}

#[test]
#[ignore = "Requires exported Steam assets"]
fn production_parts_come_off_and_fit_again() {
    use my_summer_slop::interaction::{
        InteractionAction, items::ItemLocation, parts::PART_SLOT_FLAG,
    };
    use rapier3d::math::{Rotation, Vector};
    let bytes = std::fs::read("web/assets/reference/manifest.json").unwrap();
    let reference = my_summer_slop::reference::ReferenceManifest::from_json(&bytes).unwrap();
    let part = |name: &str| {
        reference
            .parts
            .iter()
            .map(|part| part.item)
            .find(|id| {
                reference
                    .items
                    .iter()
                    .any(|item| item.id == *id && item.name == name)
            })
            .unwrap()
    };
    let (door, mirror) = (part("FRDoor"), part("RightMirror"));
    let car = reference.vehicle.item;
    let grab = FrameInput {
        pressed: keys::GRAB,
        ..FrameInput::default()
    };
    let mut runtime = GarageRuntime::new(&reference).unwrap();
    runtime.started = true;
    runtime.step(&reference, FrameInput::default());
    let chassis = runtime.physics.chassis_handle();
    let mass = runtime.physics.rapier().bodies[chassis].mass();
    let shapes = |runtime: &GarageRuntime| {
        runtime
            .physics
            .rapier()
            .colliders
            .iter()
            .filter(|(_, collider)| {
                collider.user_data & PART_SLOT_FLAG == 0 && collider.user_data as u32 == door.0 + 1
            })
            .map(|(_, collider)| (collider.parent(), *collider.position_wrt_parent().unwrap()))
            .collect::<Vec<_>>()
    };
    let fitted = shapes(&runtime);

    // Grab removes an aimed door with its mirror. The door can then be carried.
    aim_at(&mut runtime, &reference, door);
    runtime.step(&reference, grab);
    let body = runtime.area.items.get(door).unwrap().body.unwrap();
    assert_eq!(
        runtime.area.items.get(door).unwrap().location,
        ItemLocation::World
    );
    assert_eq!(runtime.area.items.get(mirror).unwrap().body, Some(body));
    assert!(!runtime.area.items.mounted(car, mirror));
    assert!(
        shapes(&runtime)
            .iter()
            .all(|(parent, _)| *parent == Some(body))
    );
    assert!(runtime.physics.rapier().bodies[chassis].mass() < mass - 10.0);
    for _ in 0..30 {
        runtime.step(&reference, FrameInput::default());
    }
    aim_at(&mut runtime, &reference, door);
    runtime.step(&reference, grab);
    assert_eq!(runtime.area.interaction.held(), Some(door));

    // The carried part shows its empty slot. Grab on that slot fits it again.
    let slot = runtime
        .physics
        .rapier()
        .colliders
        .iter()
        .find(|(_, collider)| {
            collider.user_data & PART_SLOT_FLAG != 0 && collider.user_data as u32 == door.0 + 1
        })
        .map(|(_, collider)| *collider.position())
        .unwrap();
    let car_pose = *runtime.physics.rapier().bodies[chassis].position();
    let mut outward = slot.translation - car_pose.translation;
    outward.y = 0.0;
    let outward = outward.normalize();
    let view = my_summer_slop::vehicle::from_rapier(&rapier3d::math::Pose::from_parts(
        slot.translation + outward * 1.2,
        Rotation::from_rotation_arc(-Vector::Z, -outward),
    ));
    let target = runtime
        .area
        .interaction
        .aim(&runtime.physics, view, reference.player.aim_reach)
        .unwrap();
    assert!(target.part_slot && target.item == door);
    let action = runtime.area.interaction.step(
        grab,
        Some(target),
        &mut runtime.area.player,
        &mut runtime.area.items,
        &mut runtime.physics,
        1.0 / 60.0,
    );
    assert_eq!(action, Some(InteractionAction::PartFit(door)));
    runtime
        .area
        .interaction
        .drop_held(&mut runtime.area.items, &mut runtime.physics);
    runtime
        .parts
        .fit(door, &mut runtime.area.items, &mut runtime.physics)
        .unwrap();
    runtime.step(&reference, FrameInput::default());
    assert_eq!(
        runtime.area.items.get(door).unwrap().location,
        ItemLocation::Attached(car)
    );
    assert!(runtime.area.items.mounted(car, mirror));
    for ((parent, pose), (old_parent, old_pose)) in shapes(&runtime).iter().zip(&fitted) {
        assert_eq!(parent, old_parent);
        assert!((pose.translation - old_pose.translation).length() < 0.001);
    }
    assert!((runtime.physics.rapier().bodies[chassis].mass() - mass).abs() < 0.01);

    // A hard crash knocks parts other than wheels off.
    let mut runtime = GarageRuntime::new(&reference).unwrap();
    runtime.started = true;
    let rotation = *runtime.physics.rapier().bodies[chassis].rotation();
    runtime.physics.rapier_mut().bodies[chassis]
        .set_linvel(rotation * -Vector::Z * (150.0 / 3.6), true);
    runtime.step(&reference, FrameInput::default());
    assert!(reference.parts.iter().any(|part| {
        runtime.area.items.get(part.item).unwrap().location == ItemLocation::World
    }));
}
