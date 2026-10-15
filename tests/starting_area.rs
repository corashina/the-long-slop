mod common;
use common::aim_at;
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
    let pose = runtime.physics.snapshot(0).pose;
    let quantities = fluids(&runtime);
    let engine = runtime.vehicles[0].engine.clone();
    let hinges: Vec<_> = reference
        .mechanisms
        .iter()
        .map(|hinge| runtime.area.interaction.mechanisms.value(hinge.item, false))
        .collect();
    let input = FrameInput {
        held: keys::FORWARD | keys::IGNITION | keys::USE | keys::PRIMARY,
        pressed: keys::SPACE | keys::IGNITION,
        mouse: [10.0; 2],
        ..FrameInput::default()
    };
    for _ in 0..60 {
        runtime.step(&reference, input);
    }
    assert_eq!(runtime.physics.snapshot(0).pose, pose);
    assert_eq!(fluids(&runtime), quantities);
    assert_eq!(runtime.vehicles[0].engine.rpm, engine.rpm);
    assert_eq!(runtime.vehicles[0].engine.temperature, engine.temperature);
    assert_eq!(runtime.vehicles[0].engine.ignition, engine.ignition);
    assert!(runtime.vehicles[0].trip_m < 0.1);
    for (hinge, angle) in reference.mechanisms.iter().zip(hinges) {
        assert_eq!(
            runtime.area.interaction.mechanisms.value(hinge.item, false),
            angle
        );
    }
}
#[test]
fn new_game_after_carry_detach_rebase() {
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
            common::resolved(Some(my_summer_slop::interaction::AimTarget {
                collider: rapier3d::prelude::ColliderHandle::invalid(),
                item: reference.items[0].id,
                distance: 1.0,
                point: [10.0, 2.0, -1.0],
                slot: None,
            })),
            &mut runtime.area.player,
            &mut runtime.area.items,
            &mut runtime.physics,
            1.0 / 60.0,
            Default::default(),
            1.,
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
        runtime.vehicles[0].trip_m = 100.0;
        runtime.vehicles[0].handbrake = false;
        runtime = GarageRuntime::new(&reference).unwrap();
        runtime.started = true;

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
        assert!(runtime.vehicles[0].trip_m < 0.1);
        assert!(runtime.vehicles[0].handbrake);
        assert!(!runtime.vehicles[0].engine.ignition);
        assert!(runtime.physics.rapier().impulse_joints.is_empty());
        assert_eq!(runtime.area.interaction.held(), None);
    }
}
#[test]
fn recovery_preserves_state() {
    let (reference, mut runtime) = initial();
    runtime.vehicles[0].trip_m = 256.0;
    let (mut surfaces, washes) = runtime.area.conditions.capture_save(&reference).unwrap();
    for surface in &mut surfaces {
        surface.state = 2;
    }
    runtime
        .area
        .conditions
        .restore_save(
            &reference,
            &my_summer_slop::save::ConditionSnapshot {
                surfaces,
                washes,
                tools: Vec::new(),
            },
        )
        .unwrap();
    let condition_state = runtime.area.conditions.capture_save(&reference).unwrap();
    runtime
        .area
        .items
        .get_mut(reference.vehicles[0].engine.fuel_tank)
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
        .map(|item| (item.id, item.location))
        .collect();
    let body_count = runtime.physics.rapier().bodies.len();
    runtime.recover(&reference);
    assert_eq!(
        runtime.area.conditions.capture_save(&reference).unwrap(),
        condition_state
    );
    assert_eq!(fluids(&runtime), quantities);
    assert_eq!(
        runtime
            .area
            .items
            .iter()
            .map(|item| (item.id, item.location))
            .collect::<Vec<_>>(),
        parts
    );
    assert_eq!(runtime.vehicles[0].trip_m, 256.0);
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
        let vehicle = my_summer_slop::starting_area::body_vehicle(&reference, collider.body);
        if let Some(vehicle) = vehicle
            && collider.body != reference.vehicles[vehicle].body
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
                Some(runtime.physics.chassis_handle(vehicle)),
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
        common::resolved(Some(faucet_target)),
        &mut runtime.area.player,
        &mut runtime.area.items,
        &mut runtime.physics,
        1.0 / 60.0,
        Default::default(),
        1.,
    );
    assert_eq!(runtime.area.interaction.opening_valve(faucet.id), 1.0);
    let opening = reference
        .openings
        .iter()
        .find(|opening| opening.tank == reference.vehicles[0].engine.fuel_tank)
        .unwrap();
    let cap = aim_at(&mut runtime, &reference, opening.cap);
    runtime.area.interaction.step(
        FrameInput {
            pressed: keys::USE,
            ..FrameInput::default()
        },
        common::resolved(Some(cap)),
        &mut runtime.area.player,
        &mut runtime.area.items,
        &mut runtime.physics,
        1.0 / 60.0,
        Default::default(),
        1.,
    );
    assert!(runtime.area.interaction.opening_is_open(opening.id));
    let can = reference
        .tanks
        .iter()
        .find(|tank| {
            tank.quantities_liters[0] > 0.1
                && my_summer_slop::starting_area::body_vehicle(
                    &reference,
                    reference
                        .items
                        .iter()
                        .find(|item| item.id == tank.item)
                        .unwrap()
                        .body,
                )
                .is_none()
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
        common::resolved(Some(can_target)),
        &mut runtime.area.player,
        &mut runtime.area.items,
        &mut runtime.physics,
        1.0 / 60.0,
        Default::default(),
        1.,
    );
    assert_eq!(runtime.area.interaction.held(), Some(can));
    let cap = aim_at(&mut runtime, &reference, opening.cap);
    let fuel_before = runtime
        .area
        .items
        .get(reference.vehicles[0].engine.fuel_tank)
        .unwrap()
        .fluid
        .as_ref()
        .unwrap()
        .mixture
        .total();
    let fill_action = runtime.area.interaction.step(
        FrameInput {
            held: keys::PRIMARY,
            ..FrameInput::default()
        },
        common::resolved(Some(cap)),
        &mut runtime.area.player,
        &mut runtime.area.items,
        &mut runtime.physics,
        1.0 / 60.0,
        Default::default(),
        1.,
    );
    assert!(matches!(
        fill_action,
        Some(my_summer_slop::interaction::InteractionAction::Fill { .. })
    ));
    let fuel_after = runtime
        .area
        .items
        .get(reference.vehicles[0].engine.fuel_tank)
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
        runtime.vehicles[0].engine_output.running,
        "engine did not start: {:?}",
        runtime.vehicles[0].engine_output
    );
    let before = runtime.physics.snapshot(0).pose.position;
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
    let after = runtime.physics.snapshot(0).pose.position;
    let travel = ((after[0] - before[0]).powi(2) + (after[2] - before[2]).powi(2)).sqrt();
    assert!(travel > 0.2, "engine force did not move the car: {travel}");
    println!(
        "{} source bodies, {} colliders, fuel transfer {}, engine {} rpm, car moved {} m",
        reference.bodies.len(),
        runtime.physics.collider_count(),
        fuel_after - fuel_before,
        runtime.vehicles[0].engine_output.rpm,
        travel
    );
}

fn open_source_hinge(
    runtime: &mut GarageRuntime,
    reference: &my_summer_slop::reference::ReferenceManifest,
    source: i64,
) {
    let hinge = reference
        .mechanisms
        .iter()
        .find(|hinge| hinge.source.path_id == source)
        .unwrap();
    runtime.started = true;
    runtime.step(reference, FrameInput::default());
    aim_at(runtime, reference, hinge.item);
    let span = 1.0;
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
            if (runtime
                .area
                .interaction
                .mechanisms
                .value(hinge.item, false)
                .unwrap())
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
    assert!(passenger.driving2);
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
    assert!(runtime.area.player.driving2());
    runtime.step(
        reference,
        FrameInput {
            held: keys::FORWARD,
            ..FrameInput::default()
        },
    );
    assert!(!runtime.vehicles[0].engine_output.ignition);
    assert!(!runtime.vehicles[0].engine_output.starter);
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
    assert!(runtime.vehicles[0].engine_output.ignition);
    runtime.step(reference, FrameInput::default());
    assert!(!runtime.vehicles[0].engine_output.starter);
    assert!(!runtime.vehicles[0].handbrake);
    println!(
        "source passenger seat entered through aim; ignition and handbrake worked without throttle"
    );
}

fn close_source_hinge(
    runtime: &mut GarageRuntime,
    reference: &my_summer_slop::reference::ReferenceManifest,
    source: i64,
) {
    let hinge = reference
        .mechanisms
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
            if (runtime
                .area
                .interaction
                .mechanisms
                .value(hinge.item, false)
                .unwrap())
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
    let target = common::focus(
        &runtime.area.interaction,
        &runtime.physics,
        runtime.area.player.view_pose(&runtime.physics),
    );
    assert_eq!(
        target.map(|target| target.item),
        Some(item),
        "seated door control must use real aim"
    );
}

fn enter_seat(runtime: &mut GarageRuntime, reference: &ReferenceManifestRecord, vehicle: usize) {
    let seat = reference
        .seats
        .iter()
        .find(|seat| seat.vehicle == vehicle && seat.driver)
        .unwrap();
    let anchor = reference
        .anchors
        .iter()
        .find(|anchor| anchor.id == seat.anchor)
        .unwrap();
    let pose = my_summer_slop::vehicle::VehiclePose {
        position: anchor.transform.translation,
        rotation: anchor.transform.rotation,
    };
    assert!(
        runtime
            .area
            .player
            .enter(seat.id, pose, &mut runtime.physics)
    );
    runtime.step(reference, FrameInput::default());
    assert_eq!(runtime.vehicle, vehicle);
    assert!(runtime.area.player.driving());
}

fn leave_seat(runtime: &mut GarageRuntime, reference: &ReferenceManifestRecord) {
    let seat = reference
        .seats
        .iter()
        .find(|seat| Some(seat.id) == runtime.area.player.seat())
        .unwrap();
    let limits = my_summer_slop::player::ExitReference::from_reference(&reference.player, seat);
    // Turn toward the open driver door until the source exit check passes.
    'look: for pitch in [0.0, 20.0, -20.0] {
        runtime.step(
            reference,
            FrameInput {
                mouse: [-360.0, pitch - runtime.area.player.pitch_degrees()],
                ..FrameInput::default()
            },
        );
        for _ in 0..72 {
            if runtime.area.player.can_exit(&runtime.physics, &limits) {
                break 'look;
            }
            runtime.step(
                reference,
                FrameInput {
                    mouse: [5.0, 0.0],
                    ..FrameInput::default()
                },
            );
        }
    }
    assert!(runtime.area.player.try_exit(&mut runtime.physics, &limits));
    runtime.step(reference, FrameInput::default());
    assert_eq!(runtime.area.player.seat(), None);
}

type ReferenceManifestRecord = my_summer_slop::reference::ReferenceManifest;

/// The Skoda drives on its own engine. The Lada keeps its garage start.
#[test]
#[ignore = "Requires exported Steam assets"]
fn production_skoda_drives_switches_recovers_and_rebases() {
    let bytes = std::fs::read("web/assets/reference/manifest.json").unwrap();
    let reference = ReferenceManifestRecord::from_json(&bytes).unwrap();
    assert_eq!(reference.vehicles.len(), 2);
    let mut runtime = GarageRuntime::new(&reference).unwrap();
    runtime.started = true;
    for _ in 0..120 {
        runtime.step(&reference, FrameInput::default());
    }
    let lada = runtime.physics.snapshot(0).pose.position;
    let skoda = runtime.physics.snapshot(1).pose.position;
    assert!((lada[0] - reference.area.car_pose.translation[0]).abs() < 0.2);
    assert!((lada[2] - reference.area.car_pose.translation[2]).abs() < 0.2);
    assert!((skoda[0] - 38.5).abs() < 0.5 && (skoda[2] + 58.5).abs() < 0.5);
    assert!(
        runtime
            .physics
            .snapshot(1)
            .wheels_contact
            .iter()
            .all(|c| *c)
    );

    // Switch cars through the open driver doors. The Skoda engine keeps running.
    open_source_hinge(&mut runtime, &reference, 69020);
    open_source_hinge(&mut runtime, &reference, 76010);
    enter_seat(&mut runtime, &reference, 1);
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
        runtime.vehicles[1].engine_output.running,
        "Skoda did not start"
    );
    leave_seat(&mut runtime, &reference);
    enter_seat(&mut runtime, &reference, 0);
    assert!(runtime.vehicles[1].engine_output.running);
    assert!(!runtime.vehicles[0].engine_output.running);
    leave_seat(&mut runtime, &reference);
    enter_seat(&mut runtime, &reference, 1);

    for step in 0..600 {
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
    let moved = runtime.physics.snapshot(1).pose.position;
    let distance = ((moved[0] - skoda[0]).powi(2) + (moved[2] - skoda[2]).powi(2)).sqrt();
    assert!(distance > 2.0, "Skoda moved {distance} m to {moved:?}");
    assert!(runtime.vehicles[1].trip_m > 1.0);
    assert!(runtime.vehicles[0].trip_m < 0.1);
    let parked = runtime.physics.snapshot(0).pose.position;
    assert!((parked[0] - lada[0]).abs() < 0.05 && (parked[2] - lada[2]).abs() < 0.05);
    for step in 0..240 {
        runtime.step(
            &reference,
            FrameInput {
                pressed: if step == 0 { keys::SPACE } else { 0 },
                ..FrameInput::default()
            },
        );
    }
    assert!(runtime.vehicles[1].handbrake);
    let stopped = runtime.physics.snapshot(1);
    assert!(
        stopped.speed_mps < 0.5,
        "Skoda still moves: {} m/s at {:?}, moved {distance} m, contacts {:?}",
        stopped.speed_mps,
        stopped.pose.position,
        stopped.wheels_contact
    );

    // Recovery moves only the seated car.
    runtime.recover(&reference);
    let lada_absolute = runtime
        .stream
        .origin
        .absolute(runtime.physics.snapshot(0).pose.position);
    let recovered = runtime.physics.snapshot(1).pose.position;
    assert!(
        recovered[0] < 30.0 || recovered[2] < -80.0,
        "Skoda stayed in the yard: {recovered:?}"
    );

    // The floating origin keeps both cars at their absolute positions.
    let far = my_summer_slop::vehicle::VehiclePose {
        position: [
            my_summer_slop::world::REBASE_LIMIT + 300.0,
            recovered[1] + 50.0,
            recovered[2],
        ],
        rotation: runtime.physics.snapshot(1).pose.rotation,
    };
    let rebases = runtime.rebases;
    runtime.physics.reset(1, far);
    let far_absolute = runtime.stream.origin.absolute(far.position);
    runtime.step(&reference, FrameInput::default());
    assert_eq!(runtime.rebases, rebases + 1);
    let lada_after = runtime.persistent_world.absolute_vehicle_position(
        &runtime.physics,
        &runtime.stream.origin,
        0,
    );
    let skoda_after = runtime
        .stream
        .origin
        .absolute(runtime.physics.snapshot(1).pose.position);
    for axis in [0, 2] {
        assert!((lada_after[axis] - lada_absolute[axis]).abs() < 0.05);
        assert!((skoda_after[axis] - far_absolute[axis]).abs() < 1.0);
    }

    // Restart restores both cars.
    runtime = GarageRuntime::new(&reference).unwrap();
    runtime.started = true;
    assert_eq!(runtime.vehicle, 0);
    let skoda_restart = runtime.physics.snapshot(1).pose.position;
    assert!((skoda_restart[0] - 38.5).abs() < 0.5 && (skoda_restart[2] + 58.5).abs() < 0.5);
    assert!(!runtime.vehicles[1].engine_output.running);
}
