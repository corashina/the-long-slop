mod common;
use my_summer_slop::reference::{ConditionId, ItemId};

#[test]
fn missing_condition_owner_is_rejected() {
    let mut r = common::reference();
    let source = r.conditions.surfaces[0].source.clone();
    r.conditions.surfaces[0].item = Some(ItemId(u32::MAX));
    let error = r.validate().unwrap_err().to_string();
    assert!(error.contains(&format!("{}:{}", source.file, source.path_id)));
    assert!(error.contains("item"));
}

#[test]
fn child_cycle_is_rejected() {
    let mut r = common::reference();
    let id = r.conditions.surfaces[0].id;
    let source = r.conditions.surfaces[0].source.clone();
    r.conditions.surfaces[0].children.push(ConditionId(id.0));
    let error = r.validate().unwrap_err().to_string();
    assert!(error.contains(&format!("{}:{}", source.file, source.path_id)));
    assert!(error.contains("children"));
}

#[test]
fn invalid_variant_is_rejected() {
    let mut r = common::reference();
    let source = r.conditions.surfaces[0].source.clone();
    r.conditions.surfaces[0].startup.state = 6;
    let error = r.validate().unwrap_err().to_string();
    assert!(error.contains(&format!("{}:{}", source.file, source.path_id)));
    assert!(error.contains("startup.state"));
}

#[test]
fn invalid_decay_rate_is_rejected() {
    for value in [-0.01, f32::INFINITY, f32::NAN] {
        let mut r = common::reference();
        r.conditions.repair_decay_rate = value;
        assert!(r.validate().is_err());
    }
}

#[test]
fn invalid_material_and_mesh_bindings_are_rejected() {
    let mut r = common::reference();
    r.conditions.surfaces[0].renderers[0].states[0].materials[0].gltf_material = u32::MAX;
    assert!(
        r.validate()
            .unwrap_err()
            .to_string()
            .contains("renderers.states.materials")
    );
    let mut r = common::reference();
    r.conditions.surfaces[0].renderers[0].meshes[0].gltf_mesh = u32::MAX;
    assert!(
        r.validate()
            .unwrap_err()
            .to_string()
            .contains("renderers.meshes")
    );
}

#[test]
fn conflicting_installed_slot_kinds_are_rejected() {
    let mut r = common::reference();
    let collider = &r.colliders[0];
    r.conditions
        .collider_bindings
        .push(my_summer_slop::reference::ConditionColliderBinding {
            collider: collider.id,
            source: collider.source.clone(),
            ancestor_condition: None,
            pickup_condition: None,
            forwarded_condition: None,
            installed_slot_source: Some(r.part_slots[0].source.clone()),
            installed_part_slot: Some(r.part_slots[0].id),
            installed_wheel_slot: Some(r.wheel_slots[0].id),
            installed_tire_slot: None,
            wash: None,
        });
    assert!(
        r.validate()
            .unwrap_err()
            .to_string()
            .contains("installed_slot_source/kinds")
    );
}

#[test]
fn invalid_palette_is_rejected() {
    let mut r = common::reference();
    let source = r.conditions.surfaces[0].source.clone();
    r.conditions.surfaces[0].colors.push([f32::NAN, 0., 0., 1.]);
    let error = r.validate().unwrap_err().to_string();
    assert!(error.contains(&format!("{}:{}", source.file, source.path_id)));
    assert!(error.contains("colors"));
}

#[test]
fn invalid_spray_palette_index_and_empty_choices_are_rejected() {
    let mut r = common::reference();
    let particles = r.conditions.sprays[0].particles.clone();
    r.conditions.sprays.clear();
    r.conditions.brushes.clear();
    let item = r.items.iter().find(|item| item.pickup.is_some()).unwrap();
    let identity = &item.source;
    let tool: my_summer_slop::reference::SprayReference =
        serde_json::from_value(serde_json::json!({
            "source":identity,"item":item.id,"body":item.body,
            "pickup":item.pickup.as_ref().unwrap().source,"save":identity,
            "ammo":identity,"color_source":identity,"float_quantity":true,
            "hide_look_at_count":false,"hide_carried_count":true,"no_per_show":true,
            "startup_quantity":100.,"maximum_quantity":100.,"startup_color":[1.,0.,0.,1.],
            "authored_color_set":false,"startup_palette":"paint","startup_color_index":1,
            "color_types":["paint"],"colors":[],"color_palettes":{"paint":[[1.,0.,0.,1.]]},
            "nozzle":item.node,"renderer":identity,"renderer_node":item.node,
            "distance":3.,"mask":8552449,"color_lerp":0.25,"paint_loss":4.5,
            "valve_speed":10.,"startup_valve":0.,"rate":250.,"max_volume":1.,
            "text":0,"start_speed":1.,"voice_source":r.audio["voices"][0]["source"],"voice":r.audio["voices"][0]["id"],"particles":particles
        }))
        .unwrap();
    r.conditions.sprays.push(tool);
    let error = r.validate().unwrap_err().to_string();
    assert!(error.contains("color/palette/index"));
    r.conditions.sprays[0].startup_color_index = Some(0);
    r.validate().unwrap();
    r.conditions.sprays[0]
        .color_palettes
        .get_mut("paint")
        .unwrap()
        .clear();
    let error = r.validate().unwrap_err().to_string();
    assert!(error.contains("color/palette/index"));
    r.conditions.sprays[0]
        .color_palettes
        .get_mut("paint")
        .unwrap()
        .push([1., 0., 0., 1.]);
    let source = r.conditions.sprays[0].source.clone();
    r.conditions.sprays[0].save.path_id = 0;
    let error = r.validate().unwrap_err().to_string();
    assert!(error.contains(&format!("{}:{}", source.file, source.path_id)));
    assert!(error.contains("save"));
    r.conditions.sprays[0].save = source;
    r.conditions.sprays[0].voice = None;
    r.conditions.sprays[0].voice_source = None;
    assert!(r.validate().unwrap_err().to_string().contains("voice"));
}

#[test]
fn enabled_particle_motion_requires_checked_payload() {
    for field in [
        "VelocityModule.x",
        "VelocityModule.speedModifier",
        "InheritVelocityModule.m_Curve",
        "CollisionModule.m_Bounce",
    ] {
        let mut r = common::reference();
        r.conditions.sprays[0].particles[0].curves.remove(field);
        assert!(
            r.conditions
                .validate(&r)
                .unwrap_err()
                .to_string()
                .contains(field)
        );
    }
    for field in [
        "InheritVelocityModule.m_Mode",
        "CollisionModule.radiusScale",
        "CollisionModule.collidesWith.m_Bits",
    ] {
        let mut r = common::reference();
        r.conditions.sprays[0].particles[0].scalars.remove(field);
        assert!(
            r.conditions
                .validate(&r)
                .unwrap_err()
                .to_string()
                .contains(field)
        );
    }
    let mut r = common::reference();
    r.conditions.sprays[0].particles[0]
        .flags
        .remove("VelocityModule.inWorldSpace");
    assert!(r.conditions.validate(&r).is_err());
    let mut r = common::reference();
    r.conditions.sprays[0].particles[0]
        .curves
        .get_mut("VelocityModule.x")
        .unwrap()
        .scalar = f32::NAN;
    assert!(r.conditions.validate(&r).is_err());
}
