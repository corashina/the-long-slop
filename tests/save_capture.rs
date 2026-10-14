mod common;
use my_summer_slop::lighting::{LightingState, VehicleLightSample};
use my_summer_slop::reference::*;
use my_summer_slop::save::*;
use my_summer_slop::simulation::engine::EngineInput;
use my_summer_slop::starting_area::GarageRuntime;
use my_summer_slop::vehicle::VehiclePose;
use rapier3d::math::Vector;
use serde_json::json;

fn stamp(r: &ReferenceManifest) -> SourceStamp {
    SourceStamp {
        app_id: r.app_id,
        build_id: r.build_id,
        manifest_sha256: "a".repeat(64),
        source_hashes: r
            .source_hashes
            .iter()
            .map(|s| my_summer_slop::save::SourceHash {
                file: s.file.clone(),
                sha256: s.sha256.clone(),
            })
            .collect(),
    }
}
fn fixture() -> ReferenceManifest {
    let mut r = common::reference();
    let engine = r
        .items
        .iter()
        .find(|i| i.id == r.vehicles[0].engine.item)
        .unwrap()
        .clone();
    let mut node = r
        .nodes
        .iter()
        .find(|n| n.id == engine.node)
        .unwrap()
        .clone();
    node.id = NodeId(190);
    node.source.path_id = 90090;
    node.parent = Some(r.vehicles[0].root);
    node.transform.translation = [2., 1., 0.];
    r.nodes.push(node);
    let mut body = r
        .bodies
        .iter()
        .find(|b| b.id == r.vehicles[0].body)
        .unwrap()
        .clone();
    body.id = BodyId(90);
    body.source.path_id = 90091;
    body.node = NodeId(190);
    body.parent_body = Some(r.vehicles[0].body);
    body.pose.translation = [2., 1., 0.];
    r.bodies.push(body);
    let item = r.items.iter_mut().find(|i| i.id == engine.id).unwrap();
    item.node = NodeId(190);
    item.body = BodyId(90);
    r.parts.push(serde_json::from_value(json!({
        "scan_radius": 1., "usable": null, "item": engine.id, "source": {"file":"fixture", "path_id":90001},
        "node":190, "main_types":["lada"], "part_types":["engine"], "cover_slot":null,
        "local_rotation_after_mount":[0.,0.,0.,1.], "colliders":[], "linear_damping":0., "angular_damping":0.,
        "enable_if_mounted":[], "enable_if_dismounted":[], "disable_if_mounted":[], "disable_if_dismounted":[]
    })).unwrap());
    r.part_slots.push(serde_json::from_value(json!({
        "id":1, "source":{"file":"fixture","path_id":90002}, "node":190,
        "owner":null, "part":engine.id, "mount_anchor":r.anchors[0].id, "detach_anchor":null,
        "fixed_detach":false, "detach_position":[0.,0.,0.], "detach_rotation":[0.,0.,0.,1.],
        "main_types":["lada"], "part_types":["engine"], "cant_craft":false, "cant_uncraft":false,
        "disable_colliders_on_craft":false, "colliders":[]
    })).unwrap());
    let child_id = ItemId(5);
    let mut child_node = r
        .nodes
        .iter()
        .find(|n| n.id == r.vehicles[0].root)
        .unwrap()
        .clone();
    child_node.id = NodeId(191);
    child_node.source.path_id = 90092;
    child_node.parent = Some(NodeId(190));
    child_node.transform.translation = [0.25, 0.5, 0.75];
    r.nodes.push(child_node);
    let child = r.items.iter_mut().find(|i| i.id == child_id).unwrap();
    child.node = NodeId(191);
    child.body = BodyId(90);
    child.attached_to = Some(engine.id);
    r.openings[0].valve = 0.37;
    r.vehicles[0].engine.start_chance = 0.5;
    r
}
#[test]
fn capture_covers_drive_parts_fluids_and_carry() {
    let r = fixture();
    let source = stamp(&r);
    let mut g = GarageRuntime::new(&r).unwrap();
    for attempt_step in 0..120 {
        let fluids = g.area.items.vehicle_fluids(0).unwrap();
        g.vehicles[0].engine_output = g.vehicles[0].engine.step(
            EngineInput {
                ignition_held: true,
                ignition_pressed: attempt_step == 0,
                ..Default::default()
            },
            fluids,
            &r.vehicles[0].engine,
            0.37,
            0.02,
        );
    }
    for _ in 0..2 {
        g.vehicles[0].engine_output = g.vehicles[0].engine.step(
            EngineInput::default(),
            g.area.items.vehicle_fluids(0).unwrap(),
            &r.vehicles[0].engine,
            0.37,
            0.02,
        );
    }
    let mut restored_engine =
        my_summer_slop::simulation::engine::EngineState::new(&r.vehicles[0].engine);
    restored_engine.restore_save(&g.vehicles[0].engine.capture_save());
    assert_eq!(g.vehicles[0].engine.running(), restored_engine.running());
    let mut original_engine = g.vehicles[0].engine.clone();
    let tanks = [
        r.vehicles[0].engine.fuel_tank,
        r.vehicles[0].engine.oil_tank,
        r.vehicles[0].engine.coolant_tank,
    ]
    .map(|id| {
        g.area
            .items
            .get(id)
            .unwrap()
            .fluid
            .as_ref()
            .unwrap()
            .clone()
    });
    let mut original_tanks = tanks.clone();
    let mut restored_tanks = tanks;
    let next = |engine: &mut my_summer_slop::simulation::engine::EngineState,
                tanks: &mut [my_summer_slop::fluids::FluidContainer; 3]| {
        let [fuel, sump, coolant] = tanks;
        engine.step(
            EngineInput::default(),
            my_summer_slop::fluids::VehicleFluids {
                fuel,
                sump,
                coolant,
            },
            &r.vehicles[0].engine,
            0.37,
            0.02,
        )
    };
    let original_output = next(&mut original_engine, &mut original_tanks);
    let restored_output = next(&mut restored_engine, &mut restored_tanks);
    assert_eq!(original_output.running, restored_output.running);
    assert_eq!(
        original_engine.capture_save(),
        restored_engine.capture_save()
    );
    assert_eq!(original_tanks, restored_tanks);
    let chassis = g.physics.chassis_handle(0);
    g.physics.reset(
        0,
        VehiclePose {
            position: [0., 1.1, 0.],
            rotation: [0., 0., 0., 1.],
        },
    );
    for _ in 0..120 {
        g.physics
            .apply_vehicle_forces(0, Default::default(), Default::default());
        g.physics.step_physics();
    }

    g.physics.rapier_mut().bodies[chassis].set_linvel(Vector::new(3., 0., -7.), true);
    g.physics.rapier_mut().bodies[chassis].set_angvel(Vector::new(0., 0.2, 0.), true);
    g.physics
        .apply_vehicle_forces(0, Default::default(), Default::default());
    assert!(g.physics.driven_wheel_rpm(0).abs() > 0.);
    g.wheels.sync(&mut g.area.items, &mut g.physics).unwrap();
    let ids = IdentityMap::new(&r).unwrap();
    let rotating = g.wheels.capture_save(&ids, &g.physics).unwrap().0;
    assert!(rotating.iter().any(|w| w.rpm != 0.));
    let removed_slot = ids.wheel_slots[&rotating
        .iter()
        .find(|w| w.previous_rotation != 0.)
        .unwrap()
        .slot];
    let removed_key = SourceKey::from(
        &r.wheel_slots
            .iter()
            .find(|w| w.id == removed_slot)
            .unwrap()
            .source,
    );
    let baseline = rotating
        .iter()
        .find(|w| w.slot == removed_key)
        .unwrap()
        .previous_rotation;
    assert_ne!(baseline, 0.);
    g.wheels
        .remove(removed_slot, &mut g.area.items, &mut g.physics)
        .unwrap();
    g.wheels.sync(&mut g.area.items, &mut g.physics).unwrap();
    g.vehicles[0].handbrake = false;
    g.vehicles[0].trip_m = 1256.75;
    g.attempts = 9007199254740993;
    let seat = &r.seats[0];
    assert!(g.area.player.enter(
        seat.id,
        VehiclePose {
            position: [0., 1., 0.],
            rotation: [0., 0., 0., 1.]
        },
        &mut g.physics
    ));
    let engine = r.vehicles[0].engine.item;
    g.area
        .interaction
        .parts
        .remove(engine, &mut g.area.items, &mut g.physics)
        .unwrap();
    g.area.interaction.parts.detach_mechanism(
        engine,
        &mut g.area.interaction.mechanisms,
        &mut g.physics,
    );
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
    g.area.items.get_mut(engine).unwrap().temperature = Some(g.vehicles[0].engine.temperature);
    let fuel = g
        .area
        .items
        .get_mut(r.vehicles[0].engine.fuel_tank)
        .unwrap();
    fuel.condition = 0.42;
    fuel.fluid.as_mut().unwrap().mixture.quantities_liters = [1., 0.2, 0.3, 0.4, 0.5, 0.6];
    let saved = g
        .capture_save(&r, &source, CameraSave::Chase, "2026-10-12T10:00:00Z")
        .unwrap();
    saved.validate(&r, &source).unwrap();
    assert_eq!(saved.vehicles[0].trip_m, g.vehicles[0].trip_m);
    assert_eq!(saved.progress.start_attempts, "9007199254740993");
    assert!(!saved.vehicles[0].handbrake);
    assert_eq!(saved.view.camera, CameraSave::Chase);
    assert_eq!(
        saved.player.seat.as_ref().unwrap().source,
        SourceKey::from(&seat.source)
    );
    assert_eq!(
        saved.vehicles[0].body.linear_velocity,
        g.physics.snapshot(0).linear_velocity
    );
    assert!(saved.vehicles[0].body.linear_velocity[2].abs() > 6.);
    assert_eq!(
        saved.vehicles[0].body.angular_velocity,
        g.physics.snapshot(0).angular_velocity
    );
    assert!(saved.vehicles[0].engine.elapsed > 0.);
    assert!(saved.vehicles[0].engine.last_start_chance_time > 0.);
    assert_eq!(saved.vehicles[0].engine.random_start, 0.37);
    let empty = saved
        .attachments
        .wheels
        .iter()
        .find(|w| w.slot == removed_key)
        .unwrap();
    assert!(empty.rim.is_none());
    assert_eq!(empty.previous_rotation, baseline);
    assert_ne!(empty.rotation, baseline);
    assert!(saved.attachments.wheels.iter().any(|w| w.rpm != 0.));
    let child = saved
        .items
        .iter()
        .find(|i| i.instance == ids.item_instance(ItemId(5)).unwrap())
        .unwrap();
    assert_eq!(
        child.location,
        ItemLocationSave::Attached {
            owner: ids.item_instance(engine).unwrap()
        }
    );
    assert_eq!(
        saved.vehicles[0].engine,
        g.vehicles[0].engine.capture_save()
    );
    assert!(saved.attachments.parts[0].part.is_none());
    let held = saved.player.carry.as_ref().unwrap();
    assert_eq!(
        held.item,
        InstanceId::initial(&r.items.iter().find(|i| i.id == engine).unwrap().source)
    );
    assert_ne!(held.offset, [0.; 3]);
    assert_eq!(held.local_grip.rotation, [0., 0., 0., 1.]);
    let fuel = saved
        .items
        .iter()
        .find(|i| {
            i.instance
                == InstanceId::initial(
                    &r.items
                        .iter()
                        .find(|i| i.id == r.vehicles[0].engine.fuel_tank)
                        .unwrap()
                        .source,
                )
        })
        .unwrap();
    assert_eq!(fuel.condition, 0.42);
    assert_eq!(fuel.fluids, Some([1., 0.2, 0.3, 0.4, 0.5, 0.6]));
    assert_eq!(saved.mechanisms.openings.len(), r.openings.len());
    assert_eq!(saved.mechanisms.openings[0].valve, 0.37);
    assert_eq!(saved.items.len(), r.items.len());
    assert!(!saved.world.records.is_empty());
    for WorldRecord::Pump { instance, valve } in &saved.world.records {
        assert_eq!(instance.station().unwrap().0, g.stream.seed);
        assert_eq!(*valve, 1.);
    }
    for o in &saved.mechanisms.openings {
        let id = IdentityMap::new(&r).unwrap().openings[&o.source];
        assert_eq!(o.valve, g.area.interaction.opening_valve(id));
    }
    assert_eq!(
        saved
            .bodies
            .iter()
            .filter(|b| matches!(b.identity, BodyIdentity::Assembly { .. }))
            .count(),
        2
    );
    assert!(
        saved
            .items
            .windows(2)
            .all(|w| w[0].instance < w[1].instance)
    );
}
#[test]
fn capture_is_absolute_after_rebase() {
    let mut r = fixture();
    let mut shared = r
        .bodies
        .iter()
        .find(|b| b.id == r.vehicles[0].body)
        .unwrap()
        .clone();
    shared.id = BodyId(192);
    shared.source.path_id = 90192;
    shared.parent_body = None;
    shared.dynamic = true;
    shared.pose.translation = [16., 2., -32.];
    shared.node = r.items.iter().find(|i| i.id == ItemId(18)).unwrap().node;
    r.bodies.push(shared);
    for item in &mut r.items {
        if item.id == ItemId(18) || item.id == ItemId(19) {
            item.body = BodyId(192);
        }
    }
    r.items
        .iter_mut()
        .find(|i| i.id == ItemId(19))
        .unwrap()
        .attached_to = Some(ItemId(18));
    let source = stamp(&r);
    let mut g = GarageRuntime::new(&r).unwrap();
    let handle = g.area.bodies[&BodyId(192)];
    g.physics.rapier_mut().bodies[handle].set_linvel(Vector::new(1., 2., 3.), true);
    g.physics.rapier_mut().bodies[handle].set_angvel(Vector::new(0.1, 0.2, 0.3), true);
    let before = g
        .capture_save(&r, &source, CameraSave::FirstPerson, "2026-10-12T10:00:00Z")
        .unwrap();
    let shared_key = SourceKey::from(
        &r.bodies
            .iter()
            .find(|b| b.id == BodyId(192))
            .unwrap()
            .source,
    );
    let shared = before
        .bodies
        .iter()
        .filter(|b| {
            b.identity
                == BodyIdentity::Source {
                    source: shared_key.clone(),
                }
        })
        .collect::<Vec<_>>();
    assert_eq!(shared.len(), 1);
    assert_eq!(shared[0].linear_velocity, [1., 2., 3.]);
    assert_eq!(shared[0].angular_velocity, [0.1, 0.2, 0.3]);
    let shift = [2048., 0., -4096.];
    g.physics.rebase(shift);
    g.stream.origin.shift(shift);
    g.area.sync_origin(g.stream.origin.offset);
    let after = g
        .capture_save(&r, &source, CameraSave::FirstPerson, "2026-10-12T10:00:00Z")
        .unwrap();
    assert_eq!(before.player.pose.position, after.player.pose.position);
    for item in &before.items {
        assert_eq!(
            item.pose.position,
            after
                .items
                .iter()
                .find(|i| i.instance == item.instance)
                .unwrap()
                .pose
                .position
        );
    }
    assert_eq!(before.vehicles[0].body.pose, after.vehicles[0].body.pose);
    assert_eq!(before.bodies, after.bodies);

    let mut seated = GarageRuntime::new(&r).unwrap();
    assert!(seated.area.player.enter(
        r.seats[0].id,
        VehiclePose {
            position: [0.03, 1., 0.],
            rotation: [0., 0., 0., 1.]
        },
        &mut seated.physics,
    ));
    let seated_before = seated
        .capture_save(&r, &source, CameraSave::FirstPerson, "2026-10-12T10:00:00Z")
        .unwrap();
    assert_eq!(seated_before.player.pose.position[0], f64::from(0.03_f32));
    let seated_shift = [2048., 0., 0.];
    seated.physics.rebase(seated_shift);
    seated.stream.origin.shift(seated_shift);
    seated.area.sync_origin(seated.stream.origin.offset);
    let seated_after = seated
        .capture_save(&r, &source, CameraSave::FirstPerson, "2026-10-12T10:00:00Z")
        .unwrap();
    assert_eq!(
        seated_before.vehicles[0].body.pose,
        seated_after.vehicles[0].body.pose
    );
    assert_eq!(seated_before.player.pose, seated_after.player.pose);
}
#[test]
fn cycle_wrap_and_fade_capture_preserve_next_update() {
    let r = common::reference();
    let ids = IdentityMap::new(&r).unwrap();
    let mut original = LightingState::new(&r.lighting);
    let duration = f64::from(r.lighting.cycle.day_seconds + r.lighting.cycle.night_seconds);
    original.cycle.advance(
        &r.lighting.cycle,
        my_summer_slop::lighting::CycleAdvance {
            simulation_delta: (duration + 0.125) as f32,
            cycle_extra: 0.,
            cloud_scale: 1.,
            sleep_progress: None,
        },
    );
    original.cycle_headlights(&r.lighting, 0);
    for c in &r.lighting.controllers {
        if c.kind == LightControllerKind::Switch || c.kind == LightControllerKind::Cabin {
            original.use_control(&r.lighting, c.id);
        }
    }
    assert_eq!(original.cycle.capture_save().cycle_seconds, 0.);
    assert!(original.cycle.capture_save().evaluated_cycle_seconds > duration);
    let vehicle_lights: Vec<_> = (0..r.vehicles.len())
        .map(|vehicle| VehicleLightSample {
            electronics_output: r
                .lighting
                .controllers
                .iter()
                .find(|c| c.kind == LightControllerKind::Rear && c.vehicle == Some(vehicle))
                .unwrap()
                .electronics_output,
            ..Default::default()
        })
        .collect();
    let headlights: Vec<_> = (0..r.vehicles.len())
        .map(|vehicle| original.headlights(vehicle))
        .collect();
    let lights = |s: &LightingState| {
        s.light_outputs()
            .iter()
            .map(|o| (o.id, o.enabled, o.intensity))
            .collect::<Vec<_>>()
    };
    let materials = |s: &LightingState| {
        s.material_outputs()
            .iter()
            .map(|o| (o.renderer, o.asset, o.material_index))
            .collect::<Vec<_>>()
    };
    let visible = |s: &LightingState| {
        s.visibility_outputs()
            .iter()
            .map(|o| (o.node, o.visible))
            .collect::<Vec<_>>()
    };
    for (stage, dt) in [0.001, 0.004].into_iter().enumerate() {
        original.advance_fades(&r.lighting, dt);
        let saved = original.capture_save(&r.lighting, &ids).unwrap();
        if stage == 0 {
            assert!(
                saved
                    .controllers
                    .iter()
                    .any(|c| c.current_state != c.refreshed_state)
            );
        }
        assert!(
            saved
                .controllers
                .iter()
                .any(|c| matches!(c.switch_motion, Some(SwitchMotionSave::Moving { .. })))
        );
        let mut restored = LightingState::new(&r.lighting);
        restored
            .restore_save(
                &r.lighting,
                &ids,
                &saved,
                &vec![true; r.lighting.controllers.len()],
                &vehicle_lights,
                &headlights,
            )
            .unwrap();
        assert_eq!(original.cycle.output(), restored.cycle.output());
        assert_eq!(saved, restored.capture_save(&r.lighting, &ids).unwrap());
        assert_eq!(lights(&original), lights(&restored));
        assert_eq!(materials(&original), materials(&restored));
        assert_eq!(visible(&original), visible(&restored));
        original.cycle.advance(
            &r.lighting.cycle,
            my_summer_slop::lighting::CycleAdvance {
                simulation_delta: 0.02,
                cycle_extra: 0.,
                cloud_scale: 1.,
                sleep_progress: None,
            },
        );
        restored.cycle.advance(
            &r.lighting.cycle,
            my_summer_slop::lighting::CycleAdvance {
                simulation_delta: 0.02,
                cycle_extra: 0.,
                cloud_scale: 1.,
                sleep_progress: None,
            },
        );
        original.advance_fades(&r.lighting, 0.02);
        restored.advance_fades(&r.lighting, 0.02);
        assert_eq!(original.cycle.output(), restored.cycle.output());
        assert_eq!(
            original.capture_save(&r.lighting, &ids).unwrap(),
            restored.capture_save(&r.lighting, &ids).unwrap()
        );
        assert_eq!(lights(&original), lights(&restored));
        assert_eq!(materials(&original), materials(&restored));
        assert_eq!(visible(&original), visible(&restored));
    }
}
#[test]
#[ignore = "requires installed source export"]
fn installed_source_capture() {
    let r =
        ReferenceManifest::from_json(&std::fs::read("web/assets/reference/manifest.json").unwrap())
            .unwrap();
    let source = stamp(&r);
    let g = GarageRuntime::new(&r).unwrap();
    let saved = g
        .capture_save(&r, &source, CameraSave::FirstPerson, "2026-10-12T10:00:00Z")
        .unwrap();
    assert_eq!(saved.items.len(), r.items.len());
    assert_eq!(saved.attachments.parts.len(), r.part_slots.len());
    assert_eq!(saved.attachments.wheels.len(), r.wheel_slots.len());
    let decoded = SaveSnapshot::from_json(&serde_json::to_vec(&saved).unwrap()).unwrap();
    for (a, b) in decoded.items.iter().zip(&saved.items) {
        assert!(
            a == b,
            "item {:?}: decoded pose {:?}; original pose {:?}",
            a.instance,
            a.pose,
            b.pose
        );
    }
    assert!(
        decoded == saved,
        "save JSON changed a scalar outside the item inventory"
    );
    saved.validate(&r, &source).unwrap();
}
