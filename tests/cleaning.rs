mod common;
use my_summer_slop::{
    conditions::ConditionStore,
    interaction::{
        cleaning::{CleaningTools, ToolFrame},
        items::ItemStore,
        parts::PartAttachments,
    },
    reference::*,
    vehicle::{PhysicsWorld, VehiclePose},
};
use rapier3d::prelude::*;
fn setup() -> (
    ReferenceManifest,
    CleaningTools,
    ConditionStore,
    ItemStore,
    PhysicsWorld,
    PartAttachments,
) {
    let mut r = common::reference();
    let tool_item = r.conditions.brushes[0].item;
    r.conditions.brushes[0].model = Some(r.items.iter().find(|i| i.id == tool_item).unwrap().node);
    let target = r.conditions.surfaces[0].id;
    r.conditions.surfaces[0].startup.state = 3;
    r.conditions.surfaces[0].permission_index = 0;
    r.conditions.repair_tables[0] = [[true; 7]; 5];
    for row in &mut r.conditions.repair_tables[0] {
        row[3] = false;
    }
    let collider = r.colliders[0].id;
    r.conditions
        .collider_bindings
        .push(ConditionColliderBinding {
            collider,
            source: r.colliders[0].source.clone(),
            ancestor_condition: Some(target),
            pickup_condition: None,
            forwarded_condition: None,
            installed_slot_source: None,
            installed_part_slot: None,
            installed_wheel_slot: None,
            installed_tire_slot: None,
            wash: None,
        });
    let poses = my_summer_slop::starting_area::initial_vehicle_poses(&r).unwrap();
    let mut p = PhysicsWorld::new(&r, &poses).unwrap();
    p.rapier_mut().colliders.insert(
        ColliderBuilder::cuboid(2., 2., 0.1)
            .translation(rapier3d::math::Vector::new(1000., 1000., 998.9))
            .user_data(u128::from(collider.0) << 64),
    );
    p.refresh_queries();
    let tools = CleaningTools::new(&r).unwrap();
    let conditions = ConditionStore::new(&r).unwrap();
    let items = ItemStore::new(&r).unwrap();
    let parts = PartAttachments::new(&r).unwrap();
    (r, tools, conditions, items, p, parts)
}
fn frame(r: &ReferenceManifest, x: f32) -> ToolFrame {
    ToolFrame {
        held: Some(r.conditions.brushes[0].item),
        lmb: true,
        active: true,
        dt: 0.1,
        view: VehiclePose {
            position: [1000. + x, 1000., 1000.],
            rotation: [0., 0., 0., 1.],
        },
        origin: [0.; 3],
    }
}
#[test]
fn stationary_contact_does_not_repair() {
    let (r, mut t, mut c, i, p, a) = setup();
    for _ in 0..30 {
        let mut f = frame(&r, 0.);
        f.dt = 0.02;
        assert!(t.advance(f, &r, &mut c, &i, &p, &a));
    }
    assert_eq!(c.get(r.conditions.surfaces[0].id).repair_progress, 0.);
}
#[test]
fn contact_reset_does_not_wear_tool() {
    let (r, mut t, mut c, i, p, a) = setup();
    let id = r.conditions.brushes[0].own_condition;
    let before = c.get(id).state;
    for x in [0., 0.5, 1.] {
        t.reset_contact();
        t.advance(frame(&r, x), &r, &mut c, &i, &p, &a);
    }
    assert_eq!(c.get(id).state, before);
    assert_eq!(c.get(r.conditions.surfaces[0].id).repair_progress, 0.);
}
#[test]
fn held_tool_reserves_lmb_on_invalid_target() {
    let (r, mut t, mut c, i, p, a) = setup();
    assert!(t.advance(frame(&r, 10.), &r, &mut c, &i, &p, &a));
    assert!(!t.signals()[0].doing_work);
}
#[test]
fn blocked_ray_cannot_repair_far_surface() {
    let (r, mut t, mut c, i, mut p, a) = setup();
    p.rapier_mut().colliders.insert(
        ColliderBuilder::cuboid(2., 2., 0.1)
            .translation(rapier3d::math::Vector::new(1000., 1000., 999.5)),
    );
    p.refresh_queries();
    for x in [0., 0.1, 0.2, 0.3] {
        t.advance(frame(&r, x), &r, &mut c, &i, &p, &a);
    }
    assert_eq!(c.get(r.conditions.surfaces[0].id).repair_progress, 0.);
}

#[test]
fn moving_target_does_not_count_as_rubbing() {
    let (r, mut t, mut c, i, mut p, a) = setup();
    t.advance(frame(&r, 0.), &r, &mut c, &i, &p, &a);
    let handle = p
        .rapier()
        .colliders
        .iter()
        .find(|(_, c)| c.translation().x == 1000.)
        .unwrap()
        .0;
    for n in 1..10 {
        p.rapier_mut().colliders[handle].set_translation(rapier3d::math::Vector::new(
            1000. + n as f32,
            1000.,
            998.9,
        ));
        p.refresh_queries();
        t.advance(frame(&r, n as f32), &r, &mut c, &i, &p, &a);
        assert_eq!(t.signals()[0].force, 0.);
    }
    assert_eq!(c.get(r.conditions.surfaces[0].id).repair_progress, 0.);
}
#[test]
fn one_active_frame_has_one_tool_update() {
    let (r, mut t, mut c, i, mut p, a) = setup();
    t.advance(frame(&r, 0.), &r, &mut c, &i, &p, &a);
    t.advance(frame(&r, 0.2), &r, &mut c, &i, &p, &a);
    for _ in 0..5 {
        p.step_physics();
    }
    assert_eq!(c.get(r.conditions.surfaces[0].id).repair_progress, 0.);
    t.advance(frame(&r, 0.4), &r, &mut c, &i, &p, &a);
    assert!((c.get(r.conditions.surfaces[0].id).repair_progress - 0.05).abs() < 1e-6);
}
#[test]
fn wash_completion_is_strict_and_wears_only_on_completion() {
    let (mut r, _, _, i, p, a) = setup();
    let wash = r.conditions.washes[0].id;
    r.conditions.washes[0].active = true;
    r.conditions.washes[0].on = false;
    r.conditions.washes[0].repair_progress = 0.95;
    r.conditions.washes[0].never_repair = false;
    r.conditions.collider_bindings[0].wash = Some(wash);
    r.conditions.washes[0].node = r.colliders[0].node;
    r.conditions.washes[0].colliders.push(r.colliders[0].id);
    let permission = r.conditions.washes[0].permission_index as usize;
    r.conditions.clean_tables[permission][r.conditions.brushes[0].cleaning_type as usize] = true;
    let mut t = CleaningTools::new(&r).unwrap();
    let mut c = ConditionStore::new(&r).unwrap();
    let id = r.conditions.brushes[0].own_condition;
    let before = c.get(id).state;
    for x in [0., 0.2, 0.4] {
        t.advance(frame(&r, x), &r, &mut c, &i, &p, &a);
    }
    assert_eq!(c.wash(wash).repair_progress, 1.);
    assert!(c.wash(wash).visible());
    assert_eq!(c.get(id).state, before);
    t.advance(frame(&r, 0.6), &r, &mut c, &i, &p, &a);
    assert!(!c.wash(wash).visible());
    assert_eq!(c.wash(wash).repair_progress, 0.);
    assert_eq!(c.get(id).state, before + 1);
}
#[test]
fn worn_tool_stops_work_and_survives_save() {
    let (mut r, _, _, i, p, a) = setup();
    let id = r.conditions.brushes[0].own_condition;
    r.conditions
        .surfaces
        .iter_mut()
        .find(|s| s.id == id)
        .unwrap()
        .startup
        .state = 4;
    r.conditions.surfaces[0].startup.repair_progress = 0.95;
    let mut t = CleaningTools::new(&r).unwrap();
    let mut c = ConditionStore::new(&r).unwrap();
    for x in [0., 0.2, 0.4] {
        t.advance(frame(&r, x), &r, &mut c, &i, &p, &a);
    }
    assert_eq!(c.get(id).state, 5);
    assert!(t.get(r.conditions.brushes[0].item).unwrap().disabled);
    assert_eq!(t.prompt(r.conditions.brushes[0].item), None);
    assert_eq!(t.signals()[0].emission, [0.; 2]);
    let saved = t.capture_save(&r).unwrap();
    let mut restored = CleaningTools::new(&r).unwrap();
    restored.restore_save(&r, &saved).unwrap();
    assert!(restored.get(r.conditions.brushes[0].item).unwrap().disabled);
    assert!(!restored.signals()[0].playing);
}
#[test]
fn pause_release_and_origin_change_reset_contact() {
    let (r, mut t, mut c, i, p, a) = setup();
    t.advance(frame(&r, 0.), &r, &mut c, &i, &p, &a);
    t.advance(frame(&r, 0.2), &r, &mut c, &i, &p, &a);
    let mut paused = frame(&r, 0.4);
    paused.active = false;
    t.advance(paused, &r, &mut c, &i, &p, &a);
    t.advance(frame(&r, 0.4), &r, &mut c, &i, &p, &a);
    assert_eq!(c.get(r.conditions.surfaces[0].id).repair_progress, 0.);
    let mut shifted = frame(&r, 0.6);
    shifted.origin = [10., 0., 0.];
    t.advance(shifted, &r, &mut c, &i, &p, &a);
    assert_eq!(c.get(r.conditions.surfaces[0].id).repair_progress, 0.);
    let mut released = frame(&r, 0.8);
    released.lmb = false;
    t.advance(released, &r, &mut c, &i, &p, &a);
    assert_eq!(t.signals()[0].force, 0.);
    assert!(!t.signals()[0].playing);
}

#[test]
fn cleaned_and_loaded_wash_colliders_stop_blocking_queries() {
    let (mut r, _, _, i, mut p, a) = setup();
    let wash = r.conditions.washes[0].id;
    r.conditions.washes[0].node = r.colliders[0].node;
    r.conditions.washes[0].colliders = vec![r.colliders[0].id];
    r.conditions.collider_bindings[0].wash = Some(wash);
    r.conditions.washes[0].active = true;
    let handle = p
        .rapier()
        .colliders
        .iter()
        .find(|(_, c)| c.translation().x == 1000.)
        .unwrap()
        .0;
    let mut t = CleaningTools::new(&r).unwrap();
    t.bind_collider(r.colliders[0].id, handle);
    let mut c = ConditionStore::new(&r).unwrap();
    t.sync_wash_colliders(&c, &mut p);
    let ray = Ray::new(
        rapier3d::math::Vector::new(1000., 1000., 1000.),
        -rapier3d::math::Vector::Z,
    );
    let enabled = |_: ColliderHandle, c: &Collider| c.is_enabled();
    assert!(
        p.queries(QueryFilter::default().predicate(&enabled))
            .cast_ray(&ray, 2., true)
            .is_some()
    );
    c.washes[0].active = false;
    c.washes[0].on = false;
    c.washes[0].destroyed = r.conditions.washes[0].destroy_on_repair;
    t.sync_wash_colliders(&c, &mut p);
    assert!(
        p.queries(QueryFilter::default().predicate(&enabled))
            .cast_ray(&ray, 2., true)
            .is_none()
    );
    let (surfaces, washes) = c.capture_save(&r).unwrap();
    let saved = my_summer_slop::save::ConditionSnapshot {
        surfaces,
        washes,
        tools: t.capture_save(&r).unwrap(),
    };
    let mut restored = ConditionStore::new(&r).unwrap();
    restored.restore_save(&r, &saved).unwrap();
    p.rapier_mut().colliders[handle].set_enabled(true);
    p.refresh_queries();
    t.sync_wash_colliders(&restored, &mut p);
    assert!(
        p.queries(QueryFilter::default().predicate(&enabled))
            .cast_ray(&ray, 2., true)
            .is_none()
    );
    r.colliders[0].enabled = false;
    let mut authored_disabled = CleaningTools::new(&r).unwrap();
    authored_disabled.bind_collider(r.colliders[0].id, handle);
    c.washes[0].active = true;
    authored_disabled.sync_wash_colliders(&c, &mut p);
    assert!(!p.rapier().colliders[handle].is_enabled());
    let _ = (i, a);
}

#[test]
fn part_remove_reinstall_does_not_override_a_chassis_wash() {
    use my_summer_slop::{
        interaction::items::ItemLocation, save::ConditionSnapshot, starting_area::GarageRuntime,
    };
    let mut r = common::reference();
    let mut collider = r.colliders[0].clone();
    collider.id = ColliderId(r.colliders.iter().map(|c| c.id.0).max().unwrap() + 1);
    collider.source = SourceId {
        file: "cleaning-wash-fixture".into(),
        path_id: 1,
    };
    collider.sensor = true;
    collider.enabled = true;
    let id = collider.id;
    r.vehicles[0].colliders.push(id);
    r.conditions.washes[0].node = collider.node;
    r.conditions.washes[0].colliders = vec![id];
    r.conditions.washes[0].active = true;
    let wash = r.conditions.washes[0].id;
    r.conditions
        .collider_bindings
        .push(ConditionColliderBinding {
            collider: id,
            source: collider.source.clone(),
            ancestor_condition: None,
            pickup_condition: None,
            forwarded_condition: None,
            installed_slot_source: None,
            installed_part_slot: None,
            installed_wheel_slot: None,
            installed_tire_slot: None,
            wash: Some(wash),
        });
    r.colliders.push(collider);
    let mut g = GarageRuntime::new(&r).unwrap();
    let handle = g
        .physics
        .rapier()
        .colliders
        .iter()
        .find(|(_, c)| ColliderId((c.user_data >> 64) as u32) == id)
        .unwrap()
        .0;
    let (surfaces, washes) = g.area.conditions.capture_save(&r).unwrap();
    let active = ConditionSnapshot {
        surfaces,
        washes,
        tools: g.area.interaction.cleaning.capture_save(&r).unwrap(),
    };
    assert!(g.physics.rapier().colliders[handle].is_enabled());
    g.area.conditions.washes[0].active = false;
    g.area
        .interaction
        .cleaning
        .sync_wash_colliders(&g.area.conditions, &mut g.physics);
    assert!(!g.physics.rapier().colliders[handle].is_enabled());
    let part = r
        .parts
        .iter()
        .find(|p| {
            g.area
                .interaction
                .parts
                .can_remove(p.item, &g.area.player, &g.area.items)
                .is_ok()
        })
        .unwrap()
        .item;
    let ItemLocation::PartSlot(slot) = g.area.items.get(part).unwrap().location else {
        panic!("mounted part")
    };
    let body = g.area.interaction.parts.loose_body(part).unwrap();
    g.area
        .interaction
        .parts
        .remove(part, &mut g.area.items, &mut g.physics)
        .unwrap();
    g.area.interaction.parts.detach_mechanism(
        part,
        &mut g.area.interaction.mechanisms,
        &mut g.physics,
    );
    g.area
        .interaction
        .sync_parts(&mut g.area.items, &mut g.physics)
        .unwrap();
    assert!(g.physics.rapier().bodies[body].is_enabled());
    assert!(!g.physics.rapier().colliders[handle].is_enabled());
    g.physics.step_physics();
    assert!(!g.physics.rapier().colliders[handle].is_enabled());
    g.area
        .interaction
        .parts
        .install(part, slot, &mut g.area.items, &mut g.physics)
        .unwrap();
    g.area
        .interaction
        .parts
        .mount_mechanism(part, &mut g.area.interaction.mechanisms);
    g.area
        .interaction
        .sync_parts(&mut g.area.items, &mut g.physics)
        .unwrap();
    assert!(!g.physics.rapier().bodies[body].is_enabled());
    assert!(!g.physics.rapier().colliders[handle].is_enabled());
    g.physics.step_physics();
    assert!(!g.physics.rapier().colliders[handle].is_enabled());
    g.area.conditions.restore_save(&r, &active).unwrap();
    g.area
        .interaction
        .cleaning
        .sync_wash_colliders(&g.area.conditions, &mut g.physics);
    assert!(g.physics.rapier().colliders[handle].is_enabled());
    assert!(!g.physics.rapier().bodies[body].is_enabled());
}
fn spray_setup() -> (
    ReferenceManifest,
    CleaningTools,
    ConditionStore,
    ItemStore,
    PhysicsWorld,
    PartAttachments,
) {
    let (mut r, _, _, mut items, physics, parts) = setup();
    let spray = &mut r.conditions.sprays[0];
    spray.startup_quantity = 100.;
    spray.startup_valve = 0.;
    spray.startup_color = [1., 0., 0., 1.];
    items.get_mut(spray.item).unwrap().pose = VehiclePose {
        position: [1000., 1000., 1001.],
        rotation: [0., 0., 0., 1.],
    };
    r.conditions.surfaces[0].startup.color = [0., 0., 0., 1.];
    for row in &mut r.conditions.repair_tables[0] {
        row[0] = true;
    }
    let tools = CleaningTools::new(&r).unwrap();
    let conditions = ConditionStore::new(&r).unwrap();
    (r, tools, conditions, items, physics, parts)
}
fn spray_frame(r: &ReferenceManifest) -> ToolFrame {
    let mut f = frame(r, 10.);
    f.held = Some(r.conditions.sprays[0].item);
    f
}
#[test]
fn spray_consumes_paint_on_miss() {
    let (r, mut t, mut c, mut i, p, a) = spray_setup();
    i.get_mut(r.conditions.sprays[0].item)
        .unwrap()
        .pose
        .position[0] += 10.;
    t.advance(spray_frame(&r), &r, &mut c, &i, &p, &a);
    let state = t.get(r.conditions.sprays[0].item).unwrap();
    assert!((state.quantity.unwrap() - 99.55).abs() < 1e-5);
    assert_eq!(state.valve, 1.);
}
#[test]
fn spray_blends_by_source_distance() {
    let (r, mut t, mut c, i, p, a) = spray_setup();
    t.advance(spray_frame(&r), &r, &mut c, &i, &p, &a);
    assert!((c.get(r.conditions.surfaces[0].id).color[0] - 0.0125).abs() < 1e-5);
}
#[test]
fn empty_spray_stops_color_sound_and_particles() {
    let (r, mut t, mut c, i, p, a) = spray_setup();
    t.states.last_mut().unwrap().quantity = Some(0.);
    t.advance(spray_frame(&r), &r, &mut c, &i, &p, &a);
    let s = t.signals().last().unwrap();
    assert!(!s.playing);
    assert_eq!(s.emission, [0.; 2]);
    assert_eq!(c.get(r.conditions.surfaces[0].id).color, [0., 0., 0., 1.]);
}
#[test]
fn unsupported_surface_still_consumes_paint() {
    let (mut r, _, _, i, p, a) = spray_setup();
    for row in &mut r.conditions.repair_tables[0] {
        row[0] = false;
    }
    let mut t = CleaningTools::new(&r).unwrap();
    let mut c = ConditionStore::new(&r).unwrap();
    t.advance(spray_frame(&r), &r, &mut c, &i, &p, &a);
    assert!(
        (t.get(r.conditions.sprays[0].item)
            .unwrap()
            .quantity
            .unwrap()
            - 99.55)
            .abs()
            < 1e-5
    );
    assert_eq!(c.get(r.conditions.surfaces[0].id).color, [0., 0., 0., 1.]);
}
#[test]
fn spray_release_closes_valve() {
    let (r, mut t, mut c, i, p, a) = spray_setup();
    t.advance(spray_frame(&r), &r, &mut c, &i, &p, &a);
    let quantity = t.states.last().unwrap().quantity;
    let mut f = spray_frame(&r);
    f.lmb = false;
    f.dt = 0.05;
    t.advance(f, &r, &mut c, &i, &p, &a);
    assert_eq!(t.states.last().unwrap().valve, 0.5);
    assert_eq!(t.states.last().unwrap().quantity, quantity);
    assert_eq!(t.signals().last().unwrap().voice_gain, 0.5);
}
#[test]
fn saved_spray_keeps_color_and_quantity() {
    let (r, mut t, _, _, _, _) = spray_setup();
    let color = r.conditions.sprays[0]
        .color_palettes
        .values()
        .next()
        .unwrap()[0];
    let state = t.states.last_mut().unwrap();
    state.quantity = Some(37.5);
    state.color = Some(color);
    let saved = t.capture_save(&r).unwrap();
    let mut restored = CleaningTools::new(&r).unwrap();
    restored.restore_save(&r, &saved).unwrap();
    let state = restored.get(r.conditions.sprays[0].item).unwrap();
    assert_eq!(state.quantity, Some(37.5));
    assert_eq!(state.color, Some(color));
    assert_eq!(restored.signals().last().unwrap().color, color);
}
#[test]
fn last_paint_shot_changes_color_but_stops_effects() {
    let (r, mut t, mut c, i, p, a) = spray_setup();
    t.states.last_mut().unwrap().quantity = Some(0.01);
    t.advance(spray_frame(&r), &r, &mut c, &i, &p, &a);
    assert_eq!(t.states.last().unwrap().quantity, Some(0.));
    assert!(c.get(r.conditions.surfaces[0].id).color[0] > 0.);
    assert!(!t.signals().last().unwrap().playing);
    assert_eq!(t.signals().last().unwrap().emission, [0.; 2]);
    assert!(t.spray_particles(0, 0).is_empty());
}
#[test]
fn solid_and_source_trigger_hits_block_spray() {
    for sensor in [false, true] {
        let (r, mut t, mut c, i, mut p, a) = spray_setup();
        p.rapier_mut().colliders.insert(
            ColliderBuilder::cuboid(2., 2., 0.1)
                .translation(rapier3d::math::Vector::new(1000., 1000., 1000.5))
                .sensor(sensor),
        );
        p.refresh_queries();
        t.advance(spray_frame(&r), &r, &mut c, &i, &p, &a);
        assert_eq!(c.get(r.conditions.surfaces[0].id).color, [0., 0., 0., 1.]);
        assert!((t.states.last().unwrap().quantity.unwrap() - 99.55).abs() < 1e-5);
    }
}
#[test]
fn spray_pause_freezes_valve_and_drop_releases_without_loss() {
    let (r, mut t, mut c, i, p, a) = spray_setup();
    t.advance(spray_frame(&r), &r, &mut c, &i, &p, &a);
    let quantity = t.states.last().unwrap().quantity;
    let mut f = spray_frame(&r);
    f.active = false;
    f.dt = 10.;
    t.advance(f, &r, &mut c, &i, &p, &a);
    assert_eq!(t.states.last().unwrap().valve, 1.);
    assert_eq!(t.states.last().unwrap().quantity, quantity);
    assert!(!t.signals().last().unwrap().playing);
    f.active = true;
    f.held = None;
    f.dt = 0.05;
    t.advance(f, &r, &mut c, &i, &p, &a);
    assert_eq!(t.states.last().unwrap().valve, 0.5);
    assert_eq!(t.states.last().unwrap().quantity, quantity);
    assert!(t.signals().last().unwrap().playing);
}
#[test]
fn spray_uses_pickup_condition_only_without_direct_ancestor() {
    let (mut r, _, _, i, p, a) = spray_setup();
    let binding = r.conditions.collider_bindings.last_mut().unwrap();
    binding.pickup_condition = binding.ancestor_condition.take();
    let mut t = CleaningTools::new(&r).unwrap();
    let mut c = ConditionStore::new(&r).unwrap();
    t.advance(spray_frame(&r), &r, &mut c, &i, &p, &a);
    assert!((c.get(r.conditions.surfaces[0].id).color[0] - 0.0125).abs() < 1e-5);
}
#[test]
fn source_spray_color_reaches_particles_and_signal() {
    let (r, mut t, mut c, i, p, a) = spray_setup();
    t.advance(spray_frame(&r), &r, &mut c, &i, &p, &a);
    assert_eq!(t.signals().last().unwrap().color, [1., 0., 0., 1.]);
    assert!(!t.spray_particles(0, 0).is_empty());
    for particle in t.spray_particles(0, 0) {
        assert_eq!(particle.color[1], 0.);
        assert_eq!(particle.color[2], 0.);
    }
}

#[test]
fn source_non_wash_sensor_blocks_brush() {
    let (r, mut t, mut c, i, mut p, a) = setup();
    p.rapier_mut().colliders.insert(
        ColliderBuilder::cuboid(2., 2., 0.1)
            .translation(rapier3d::math::Vector::new(1000., 1000., 999.5))
            .sensor(true),
    );
    p.refresh_queries();
    for x in [0., 0.1, 0.2, 0.3] {
        t.advance(frame(&r, x), &r, &mut c, &i, &p, &a);
    }
    assert_eq!(c.get(r.conditions.surfaces[0].id).repair_progress, 0.);
    assert!(!t.signals()[0].doing_work);
}

#[test]
fn looked_at_can_count_has_a_separate_owner_from_held_tool() {
    use my_summer_slop::{interaction::AimTarget, starting_area::GarageRuntime};
    let mut r = common::reference();
    let mut other = r.conditions.sprays[0].clone();
    let item = r.items.iter().find(|i| i.pickup.is_some()).unwrap();
    other.source = SourceId {
        file: "spray-hud-fixture".into(),
        path_id: 1,
    };
    other.item = item.id;
    other.body = item.body;
    other.pickup = item.pickup.as_ref().unwrap().source.clone();
    other.nozzle = item.node;
    other.renderer_node = item.node;
    let other_item = other.item;
    r.conditions.sprays.push(other);
    for held in [r.conditions.brushes[0].item, r.conditions.sprays[0].item] {
        let mut g = GarageRuntime::new(&r).unwrap();
        g.area
            .interaction
            .pickup_item(
                held,
                [10., 2., -1.],
                &g.area.player,
                &mut g.area.items,
                &mut g.physics,
            )
            .unwrap();
        g.area
            .interaction
            .cleaning
            .states
            .iter_mut()
            .find(|t| t.item == other_item)
            .unwrap()
            .quantity = Some(37.5);
        let aim = common::resolved(Some(AimTarget {
            item: other_item,
            collider: g.physics.rapier().colliders.iter().next().unwrap().0,
            distance: 1.,
            point: [0.; 3],
            slot: None,
        }));
        let available = g.area.interaction.available_actions(
            &aim,
            &g.area.player,
            &g.area.items,
            &g.physics,
            Default::default(),
        );
        assert_eq!(available.item, Some(held));
        assert_eq!(
            g.area.interaction.paint_feedback(&aim, &r),
            Some((37.5, 100.))
        );
        let own = common::resolved(Some(AimTarget {
            item: held,
            collider: g.physics.rapier().colliders.iter().next().unwrap().0,
            distance: 1.,
            point: [0.; 3],
            slot: None,
        }));
        assert_eq!(g.area.interaction.paint_feedback(&own, &r), None);
    }
}

#[test]
fn painted_virtual_part_keeps_color_through_remove_and_reinstall() {
    use my_summer_slop::{interaction::items::ItemLocation, starting_area::GarageRuntime};
    let mut r = common::reference();
    let initial = GarageRuntime::new(&r).unwrap();
    let part = r
        .parts
        .iter()
        .find(|p| {
            initial
                .area
                .interaction
                .parts
                .can_remove(p.item, &initial.area.player, &initial.area.items)
                .is_ok()
        })
        .unwrap()
        .item;
    let ItemLocation::PartSlot(slot) = initial.area.items.get(part).unwrap().location else {
        panic!("mounted part")
    };
    let target = r
        .conditions
        .surfaces
        .iter()
        .find(|s| s.item == Some(part))
        .map(|s| s.id)
        .unwrap_or_else(|| common::add_condition_owner(&mut r, part));
    let surface = r
        .conditions
        .surfaces
        .iter_mut()
        .find(|s| s.id == target)
        .unwrap();
    surface.startup.color = [0., 0., 0., 1.];
    surface.permission_index = 0;
    for row in &mut r.conditions.repair_tables[0] {
        row[0] = true;
    }
    r.conditions.sprays[0].startup_color = [1., 0., 0., 1.];
    let collider = r.colliders[0].id;
    r.conditions
        .collider_bindings
        .push(ConditionColliderBinding {
            collider,
            source: r.colliders[0].source.clone(),
            ancestor_condition: None,
            pickup_condition: None,
            forwarded_condition: None,
            installed_slot_source: Some(
                r.part_slots
                    .iter()
                    .find(|s| s.id == slot)
                    .unwrap()
                    .source
                    .clone(),
            ),
            installed_part_slot: Some(slot),
            installed_wheel_slot: None,
            installed_tire_slot: None,
            wash: None,
        });
    let mut g = GarageRuntime::new(&r).unwrap();
    g.area
        .items
        .get_mut(r.conditions.sprays[0].item)
        .unwrap()
        .pose = VehiclePose {
        position: [1000., 1000., 1001.],
        rotation: [0., 0., 0., 1.],
    };
    g.physics.rapier_mut().colliders.insert(
        ColliderBuilder::cuboid(2., 2., 0.1)
            .translation(rapier3d::math::Vector::new(1000., 1000., 998.9))
            .user_data(u128::from(collider.0) << 64),
    );
    g.physics.refresh_queries();
    g.area.interaction.cleaning.advance(
        spray_frame(&r),
        &r,
        &mut g.area.conditions,
        &g.area.items,
        &g.physics,
        &g.area.interaction.parts,
    );
    let color = g.area.conditions.get(target).color;
    assert!((color[0] - 0.0125).abs() < 1e-5);
    common::detach(&mut g, part);
    g.area.interaction.cleaning.advance(
        spray_frame(&r),
        &r,
        &mut g.area.conditions,
        &g.area.items,
        &g.physics,
        &g.area.interaction.parts,
    );
    assert_eq!(g.area.conditions.get(target).color, color);
    g.area
        .interaction
        .parts
        .install(part, slot, &mut g.area.items, &mut g.physics)
        .unwrap();
    g.area
        .interaction
        .parts
        .mount_mechanism(part, &mut g.area.interaction.mechanisms);
    g.area
        .interaction
        .sync_parts(&mut g.area.items, &mut g.physics)
        .unwrap();
    assert_eq!(g.area.conditions.get(target).color, color);
    g.area.interaction.cleaning.advance(
        spray_frame(&r),
        &r,
        &mut g.area.conditions,
        &g.area.items,
        &g.physics,
        &g.area.interaction.parts,
    );
    assert!(g.area.conditions.get(target).color[0] > color[0]);
    let ancestor = r.conditions.surfaces[0].id;
    assert_ne!(ancestor, target);
    let before = g.area.conditions.get(ancestor).color;
    let part_color = g.area.conditions.get(target).color;
    r.conditions
        .collider_bindings
        .last_mut()
        .unwrap()
        .ancestor_condition = Some(ancestor);
    let mut tools = CleaningTools::new(&r).unwrap();
    tools.advance(
        spray_frame(&r),
        &r,
        &mut g.area.conditions,
        &g.area.items,
        &g.physics,
        &g.area.interaction.parts,
    );
    assert_ne!(g.area.conditions.get(ancestor).color, before);
    assert_eq!(g.area.conditions.get(target).color, part_color);
}

#[test]
fn spray_lifecycle_resets_emitter_motion_without_erasing_particles() {
    for reset in 0..4 {
        let (mut r, _, mut conditions, mut items, physics, parts) = spray_setup();
        let particle = &mut r.conditions.sprays[0].particles[0];
        particle.rate_over_time = 10.;
        particle
            .curves
            .get_mut("InitialModule.startSpeed")
            .unwrap()
            .scalar = 0.;
        particle
            .scalars
            .insert("ShapeModule.radius.value".into(), 0.);
        for name in ["VelocityModule.x", "VelocityModule.y", "VelocityModule.z"] {
            particle.curves.get_mut(name).unwrap().mode = 0;
            particle.curves.get_mut(name).unwrap().scalar = 0.;
        }
        let mut tools = CleaningTools::new(&r).unwrap();
        tools.advance(
            spray_frame(&r),
            &r,
            &mut conditions,
            &items,
            &physics,
            &parts,
        );
        let original = tools.spray_particles(0, 0)[0];
        match reset {
            0 => {
                let mut paused = spray_frame(&r);
                paused.active = false;
                tools.advance(paused, &r, &mut conditions, &items, &physics, &parts);
            }
            1 => tools.reset_contact(), // InteractionState calls this on drop.
            2 => {
                let save = tools.capture_save(&r).unwrap();
                tools.restore_save(&r, &save).unwrap();
            }
            _ => {} // Origin change resets the shared pool in the next frame.
        }
        assert_eq!(
            tools.spray_particles(0, 0)[0].absolute_position,
            original.absolute_position
        );
        let mut frame = spray_frame(&r);
        let item = items.get_mut(r.conditions.sprays[0].item).unwrap();
        item.pose.position[0] += 1.;
        if reset == 3 {
            frame.origin = [10., 0., 0.];
            item.pose.position[0] -= 10.;
        }
        tools.advance(frame, &r, &mut conditions, &items, &physics, &parts);
        assert_eq!(tools.spray_particles(0, 0).len(), 2);
        assert!((tools.spray_particles(0, 0)[1].absolute_position[0] - 1001.).abs() < 1e-5);
    }
}
