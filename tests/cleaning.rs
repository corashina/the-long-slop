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
