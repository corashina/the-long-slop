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
fn generated_quality_round_trip_keeps_imported_semantic_owner() {
    use my_summer_slop::world::{AbsolutePose, GeneratedId, WorldLayer};
    let r = common::reference();
    let mut items = ItemStore::new(&r).unwrap();
    let prototype = &r.world.prototypes[0];
    let key = GeneratedId {
        layer: WorldLayer::Prop,
        generator: prototype.source.clone(),
        unit: [0, 0],
        ordinal: 0,
        children: Vec::new(),
    };
    let pose = AbsolutePose {
        position: [10., 2., 3.],
        rotation: [0., 0., 0., 1.],
    };
    let id = items.insert_generated(key, prototype, pose).unwrap();
    let mut saved = items.generated_snapshot(id, None).unwrap();
    saved.condition = 0.25;
    items.restore_generated(id, &saved).unwrap();
    assert_eq!(items.generated_snapshot(id, None).unwrap().condition, 0.25);
    let imported = r.conditions.surfaces.iter().find_map(|s| s.item).unwrap();
    assert_eq!(items.get(imported).unwrap().generated_condition, None);
    items.restore_generated(imported, &saved).unwrap();
    assert_eq!(items.get(imported).unwrap().generated_condition, None);
    let conditions = ConditionStore::new(&r).unwrap();
    assert_eq!(conditions.item_quality(imported), Some(1.));
}
