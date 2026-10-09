mod common;
use desert_drive::simulation::{FrameInput, keys};
use desert_drive::starting_area::GarageRuntime;

fn initial() -> (desert_drive::reference::ReferenceManifest, GarageRuntime) {
    let mut reference = common::reference();
    let item = reference.items[0].id;
    let node = reference.items[0].node;
    let mut body = reference.bodies[1].clone();
    body.id = desert_drive::reference::BodyId(90);
    body.node = node;
    body.mass_kg = 7.0;
    body.pose.translation = [10.0, 2.0, -1.0];
    reference.bodies.push(body);
    reference.items[0].body = desert_drive::reference::BodyId(90);
    reference.items[0].attached_to = None;
    reference
        .nodes
        .iter_mut()
        .find(|record| record.id == node)
        .unwrap()
        .transform
        .translation = [10.0, 2.0, -1.0];
    let mut collider = reference.colliders[0].clone();
    collider.id = desert_drive::reference::ColliderId(90);
    collider.body = desert_drive::reference::BodyId(90);
    collider.node = node;
    collider.geometry = desert_drive::reference::ColliderGeometry::Sphere {
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
            Some(desert_drive::interaction::AimTarget {
                item: reference.items[0].id,
                distance: 1.0,
                point: [10.0, 2.0, -1.0],
                slot: None,
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
            desert_drive::interaction::items::ItemLocation::World
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
        item.condition *= 0.5;
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
    let parts: Vec<_> = runtime
        .area
        .items
        .iter()
        .map(|item| (item.id, item.location, item.condition))
        .collect();
    let body_count = runtime.physics.rapier().bodies.len();
    runtime.recover(&reference);
    assert_eq!(fluids(&runtime), quantities);
    assert_eq!(
        runtime
            .area
            .items
            .iter()
            .map(|item| (item.id, item.location, item.condition))
            .collect::<Vec<_>>(),
        parts
    );
    assert_eq!(runtime.trip_m, 256.0);
    assert_eq!(runtime.physics.rapier().bodies.len(), body_count);
}

/// Run with locally exported installed-build assets.
#[test]
#[ignore = "Requires exported Steam assets"]
fn production_garage_bindings_fill_start_and_move() {
    let bytes = std::fs::read("web/assets/reference/manifest.json").expect("exported manifest");
    let reference = desert_drive::reference::ReferenceManifest::from_json(&bytes).unwrap();
    let mut runtime = GarageRuntime::new(&reference).unwrap();
    assert_eq!(runtime.area.bodies.len(), reference.bodies.len());
    for collider in &reference.colliders {
        if collider.body != reference.vehicle.body
            && desert_drive::starting_area::belongs_to_vehicle(&reference, collider.body)
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
    let view = desert_drive::vehicle::to_rapier(runtime.area.player.view_pose(&runtime.physics));
    staged.position =
        (view.translation + view.rotation * rapier3d::math::Vector::new(0.0, 0.0, -0.6)).to_array();
    let can_body = runtime.area.items.get(can).unwrap().body.unwrap();
    runtime.physics.rapier_mut().bodies[can_body]
        .set_position(desert_drive::vehicle::to_rapier(staged), true);
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
        Some(desert_drive::interaction::InteractionAction::Fill { .. })
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
    let driver = reference.seats.iter().find(|seat| seat.driver).unwrap();
    let seat = reference
        .anchors
        .iter()
        .find(|anchor| anchor.id == driver.anchor)
        .unwrap();
    assert!(
        runtime.area.player.enter(
            driver.id,
            desert_drive::vehicle::VehiclePose {
                position: seat.transform.translation,
                rotation: seat.transform.rotation
            },
            &mut runtime.physics
        ),
        "driver seat is blocked"
    );
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
    reference: &desert_drive::reference::ReferenceManifest,
    item: desert_drive::reference::ItemId,
) -> desert_drive::interaction::AimTarget {
    let center = runtime
        .physics
        .rapier()
        .colliders
        .iter()
        .find(|(_, collider)| collider.user_data as u32 == item.0 + 1)
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
        let pose = desert_drive::vehicle::VehiclePose {
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
            .intersect_shape(desert_drive::vehicle::to_rapier(pose), standing.as_ref())
            .next()
            .is_some()
        {
            continue;
        }
        runtime.physics.rapier_mut().remove_body(old);
        runtime.area.player =
            desert_drive::player::PlayerState::new(&reference.player, pose, &mut runtime.physics);
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
    use desert_drive::reference::{
        AnchorId, BodyId, ColliderId, HingeRecord, ItemId, ItemKind, NodeId, Transform,
    };
    use desert_drive::vehicle::collider_builder;
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
    collider.geometry = desert_drive::reference::ColliderGeometry::Box {
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
        Some(desert_drive::interaction::AimTarget {
            item: ItemId(90),
            distance: 1.0,
            point: [0.8, 0.7, 0.0],
            slot: None,
        }),
        &mut runtime.area.player,
        &mut runtime.area.items,
        &mut runtime.physics,
        1.0 / 60.0,
    );
    assert_eq!(runtime.area.interaction.hinge_angle(ItemId(90)), Some(45.0));
    runtime.area.sync_compound(&mut runtime.physics, true);
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
                && first.user_data & desert_drive::vehicle::VEHICLE_COLLIDER_FLAG != 0
                && second.user_data & desert_drive::vehicle::VEHICLE_COLLIDER_FLAG != 0)
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
        Some(desert_drive::interaction::AimTarget {
            item: can,
            distance: 1.0,
            point: [10.0, 2.0, -1.0],
            slot: None,
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
    let reference = desert_drive::reference::ReferenceManifest::from_json(&bytes).unwrap();
    let mut runtime = GarageRuntime::new(&reference).unwrap();
    let can = reference
        .tanks
        .iter()
        .find(|tank| {
            tank.role == desert_drive::reference::TankRole::Portable
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
    let station = desert_drive::world::nearest_station(
        runtime.stream.seed,
        -runtime
            .stream
            .origin
            .absolute(runtime.physics.snapshot().pose.position)[2],
    );
    let car = runtime
        .stream
        .origin
        .absolute(runtime.physics.snapshot().pose.position);
    assert!(
        ((car[0] - station.absolute_point[0]).powi(2)
            + (car[2] - station.absolute_point[2]).powi(2))
        .sqrt()
            < 0.01
    );
    // Stage the same source can as trunk cargo. Recovery must preserve its car-relative pose.
    let body = runtime.area.items.get(can).unwrap().body.unwrap();
    let mut cargo = desert_drive::vehicle::to_rapier(runtime.physics.snapshot().pose);
    cargo.translation += cargo.rotation * rapier3d::math::Vector::new(0.0, 0.5, 0.0);
    runtime.physics.rapier_mut().bodies[body].set_position(cargo, true);
    runtime.physics.refresh_queries();
    runtime.area.items.sync_poses(&runtime.physics);
    let old_car = desert_drive::vehicle::to_rapier(runtime.physics.snapshot().pose);
    let relative = old_car.inverse() * *runtime.physics.rapier().bodies[body].position();
    let next = desert_drive::vehicle::VehiclePose {
        position: [
            old_car.translation.x + 20.0,
            old_car.translation.y,
            old_car.translation.z - 1400.0,
        ],
        rotation: old_car.rotation.to_array(),
    };
    runtime.physics.reset(next);
    runtime.physics.rapier_mut().bodies[body]
        .set_position(desert_drive::vehicle::to_rapier(next) * relative, true);
    runtime.recover(&reference);
    let expected = desert_drive::vehicle::to_rapier(runtime.physics.snapshot().pose) * relative;
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
    let mut stream = desert_drive::world::WorldStream::new(desert_drive::world::DEFAULT_SEED);
    let pose = stream.reset_pose(640.0, false, &mut physics);
    physics.reset(pose);
    for _ in 0..180 {
        physics.apply_vehicle_forces(
            desert_drive::simulation::DriveInput {
                handbrake: true,
                ..Default::default()
            },
            0.0,
        );
        physics.step_physics();
    }
    let before = physics.snapshot().pose.position;
    let distant = desert_drive::world::road_sample(stream.seed, 3000.0).center;
    stream.sync_at(distant, &mut physics);
    for _ in 0..120 {
        physics.apply_vehicle_forces(
            desert_drive::simulation::DriveInput {
                handbrake: true,
                ..Default::default()
            },
            0.0,
        );
        physics.step_physics();
    }
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
        "support halo must retain all four wheel contacts"
    );
    assert_eq!(stream.chunks().count(), 11);
    assert!(
        physics.collider_count() <= 66,
        "support halo exceeded collider bound"
    );
    assert_eq!(
        physics
            .rapier()
            .colliders
            .iter()
            .filter(|(_, collider)| collider.shape().as_trimesh().is_some())
            .count(),
        14
    );
    stream.sync_at(stream.origin.absolute(after), &mut physics);
    let count = physics.collider_count();
    assert!(
        count <= 63,
        "returning focus must remove the terrain-only support halo"
    );
    assert_eq!(
        physics
            .rapier()
            .colliders
            .iter()
            .filter(|(_, collider)| collider.shape().as_trimesh().is_some())
            .count(),
        11
    );
    stream.sync_at(stream.origin.absolute(after), &mut physics);
    assert_eq!(
        physics.collider_count(),
        count,
        "returning focus duplicated terrain support"
    );
}

#[test]
#[ignore = "Requires exported Steam assets"]
fn production_opens_both_garage_exit_leaves() {
    let bytes = std::fs::read("web/assets/reference/manifest.json").unwrap();
    let reference = desert_drive::reference::ReferenceManifest::from_json(&bytes).unwrap();
    let mut runtime = GarageRuntime::new(&reference).unwrap();
    runtime.started = true;
    for source in [64783, 64777] {
        let hinge = reference
            .hinges
            .iter()
            .find(|hinge| hinge.source.path_id == source)
            .expect("recovered garage exit leaf");
        runtime.step(&reference, FrameInput::default());
        aim_at(&mut runtime, &reference, hinge.item);
        let span = (hinge.max_degrees - hinge.closed_degrees)
            .abs()
            .max((hinge.min_degrees - hinge.closed_degrees).abs());
        let mut open = false;
        for mouse in [[0.0, 50.0], [0.0, -50.0], [50.0, 0.0], [-50.0, 0.0]] {
            for step in 0..180 {
                runtime.step(
                    &reference,
                    FrameInput {
                        held: keys::USE,
                        pressed: if step == 0 { keys::USE } else { 0 },
                        mouse,
                        ..FrameInput::default()
                    },
                );
                let angle = runtime.area.interaction.hinge_angle(hinge.item).unwrap();
                if (angle - hinge.closed_degrees).abs() >= span * 0.9 {
                    open = true;
                    break;
                }
            }
            if open {
                break;
            }
            runtime.step(&reference, FrameInput::default());
            aim_at(&mut runtime, &reference, hinge.item);
        }
        assert!(
            open,
            "garage exit leaf {source} did not open with its recovered controls"
        );
        let body = runtime.area.bodies[&hinge.body];
        assert!(runtime.physics.rapier().bodies[body].is_kinematic());
        assert!(!runtime.physics.rapier().bodies[body].colliders().is_empty());
        println!(
            "garage leaf {source} opened to {} degrees",
            runtime.area.interaction.hinge_angle(hinge.item).unwrap()
        );
    }
}
