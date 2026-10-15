#![allow(dead_code)]

use my_summer_slop::reference::{ColliderGeometry, ColliderId, ReferenceManifest};
use my_summer_slop::vehicle::{PhysicsWorld, VehiclePose};

pub fn reference() -> ReferenceManifest {
    let mut reference = ReferenceManifest::from_json(include_bytes!("reference.json"))
        .expect("valid native fixture");
    separate_tool_owners(&mut reference);
    reference.colliders[0].geometry = ColliderGeometry::Box {
        center: [0.0; 3],
        half_extents: [0.76, 0.24, 1.55],
        rotation: [0.0, 0.0, 0.0, 1.0],
    };
    let mut cabin = reference.colliders[0].clone();
    cabin.id = ColliderId(
        reference
            .colliders
            .iter()
            .map(|collider| collider.id.0)
            .max()
            .unwrap()
            + 1,
    );
    cabin.source.path_id += 1;
    cabin.geometry = ColliderGeometry::Box {
        center: [0.0, 0.57, 0.2],
        half_extents: [0.65, 0.36, 0.72],
        rotation: [0.0, 0.0, 0.0, 1.0],
    };
    reference.vehicles[0].colliders.push(cabin.id);
    reference
        .action_bindings
        .push(my_summer_slop::reference::ActionBinding {
            collider: cabin.id,
            use_owner: None,
            pickup_owner: None,
            removal_owner: None,
            push_owner: None,
        });
    reference.colliders.push(cabin);
    reference
}

pub fn world() -> PhysicsWorld {
    let reference = reference();
    let mut poses = my_summer_slop::starting_area::initial_vehicle_poses(&reference)
        .expect("valid fixture vehicle poses");
    poses[0] = VehiclePose {
        position: [0.0, 1.1, 0.0],
        rotation: [0.0, 0.0, 0.0, 1.0],
    };
    PhysicsWorld::new(&reference, &poses).expect("valid fixture physics")
}

pub fn resolved(
    target: Option<my_summer_slop::interaction::AimTarget>,
) -> my_summer_slop::interaction::ResolvedAim {
    my_summer_slop::interaction::ResolvedAim {
        use_target: target,
        grab_target: target,
        installation_target: target,
        filling_target: target,
        focus_target: target,
        ..Default::default()
    }
}

pub fn focus(
    interaction: &my_summer_slop::interaction::InteractionState,
    physics: &PhysicsWorld,
    view: VehiclePose,
) -> Option<my_summer_slop::interaction::AimTarget> {
    interaction
        .resolve(
            physics,
            view,
            &my_summer_slop::interaction::items::ItemStore::new(&reference()).unwrap(),
        )
        .focus_target
}

pub fn add_rotation(reference: &mut ReferenceManifest, item: u32, body: u32, limits: [f32; 2]) {
    use my_summer_slop::reference::*;
    let body_index = reference
        .bodies
        .iter()
        .position(|b| b.id == BodyId(body))
        .unwrap();
    let record = reference.bodies[body_index].clone();
    let node = NodeId(10000 + body);
    let mut local = record.pose.clone();
    let parent = record.parent_body.map(|id| {
        let parent = reference.bodies.iter().find(|b| b.id == id).unwrap();
        let relative = my_summer_slop::vehicle::to_rapier(VehiclePose {
            position: parent.pose.translation,
            rotation: parent.pose.rotation,
        })
        .inverse()
            * my_summer_slop::vehicle::to_rapier(VehiclePose {
                position: record.pose.translation,
                rotation: record.pose.rotation,
            });
        local.translation = relative.translation.to_array();
        local.rotation = relative.rotation.to_array();
        parent.node
    });
    let original_node = record.node;
    let mut source_node = reference
        .nodes
        .iter()
        .find(|n| n.id == original_node)
        .unwrap()
        .clone();
    source_node.id = node;
    source_node.parent = parent;
    source_node.transform = local;
    source_node.source.path_id += i64::from(node.0);
    reference.nodes.push(source_node);
    reference.bodies[body_index].node = node;
    for c in &mut reference.colliders {
        if c.body == BodyId(body) && c.node == original_node {
            c.node = node;
        }
    }
    let owner = reference
        .items
        .iter_mut()
        .find(|i| i.id == ItemId(item))
        .unwrap();
    owner.body = BodyId(body);
    owner.node = node;
    for surface in &mut reference.conditions.surfaces {
        if surface.item == Some(ItemId(item)) {
            surface.node = node;
        }
    }
    reference.mechanisms.push(MechanismReference {
        id: MechanismId(item),
        source: reference.player.source.clone(),
        item: ItemId(item),
        parent_item: None,
        driven_nodes: vec![node],
        enabled: true,
        child: None,
        invert_child: false,
        invert_value: false,
        disable_on_craft: false,
        disable_on_uncraft: false,
        drag: MechanismDrag {
            mode: DragMode::Direct,
            node,
            node2: node,
            axis: [1.0, 0.0, 0.0],
            axes2: [[0.0; 3]; 2],
        },
        rotation: Some(RotationReference {
            targets: vec![RotationTarget {
                node,
                mapping: [[false, true, false], [false; 3]],
            }],
            sum_axes: true,
            limits_degrees: [limits, [0.0; 2]],
            initial_degrees: [0.0; 2],
            inverted: [false; 2],
            sensitivity: 1.0,
        }),
        slide: None,
        turn: None,
    });
}

pub fn detach(
    g: &mut my_summer_slop::starting_area::GarageRuntime,
    item: my_summer_slop::reference::ItemId,
) {
    g.area
        .interaction
        .parts
        .can_remove(item, &g.area.player, &g.area.items)
        .unwrap();
    g.area
        .interaction
        .parts
        .remove(item, &mut g.area.items, &mut g.physics)
        .unwrap();
    g.area.interaction.parts.detach_mechanism(
        item,
        &mut g.area.interaction.mechanisms,
        &mut g.physics,
    );
    g.area
        .interaction
        .sync_parts(&mut g.area.items, &mut g.physics)
        .unwrap();
}

pub fn set_condition(
    area: &mut my_summer_slop::starting_area::StartingArea,
    reference: &ReferenceManifest,
    item: my_summer_slop::reference::ItemId,
    state: u8,
) {
    let owner = reference
        .conditions
        .surfaces
        .iter()
        .find(|s| s.item == Some(item))
        .expect("source condition owner");
    let (mut surfaces, washes) = area.conditions.capture_save(reference).unwrap();
    surfaces
        .iter_mut()
        .find(|s| s.source == my_summer_slop::save::SourceKey::from(&owner.source))
        .unwrap()
        .state = state;
    area.conditions
        .restore_save(
            reference,
            &my_summer_slop::save::ConditionSnapshot {
                surfaces,
                washes,
                tools: Vec::new(),
            },
        )
        .unwrap();
}

/// Add a semantic condition owner for an existing test item.
pub fn add_condition_owner(
    reference: &mut ReferenceManifest,
    item: my_summer_slop::reference::ItemId,
) -> my_summer_slop::reference::ConditionId {
    use my_summer_slop::reference::ConditionId;
    assert!(
        reference
            .conditions
            .surfaces
            .iter()
            .all(|s| s.item != Some(item))
    );
    let mut owner = reference.conditions.surfaces[0].clone();
    owner.id = ConditionId(
        reference
            .conditions
            .surfaces
            .iter()
            .map(|s| s.id.0)
            .max()
            .unwrap()
            + 1,
    );
    owner.source.file = "condition-test-fixture".into();
    owner.source.path_id = i64::from(owner.id.0);
    owner.item = Some(item);
    owner.node = reference.items.iter().find(|i| i.id == item).unwrap().node;
    owner.children.clear();
    owner.renderers.clear();
    owner.objects.clear();
    owner.startup.state = 0;
    let id = owner.id;
    reference.conditions.surfaces.push(owner);
    id
}

// The checked fixture once used a rim and hubcap as tool owners.
// Synthetic tool identities keep the original part ownership intact.
fn separate_tool_owners(reference: &mut ReferenceManifest) {
    use my_summer_slop::reference::*;
    fn owner(
        r: &mut ReferenceManifest,
        original: ItemId,
        name: &str,
    ) -> (ItemId, BodyId, NodeId, SourceId) {
        let mut item = r.items.iter().find(|i| i.id == original).unwrap().clone();
        item.id = ItemId(r.items.iter().map(|i| i.id.0).max().unwrap() + 1);
        item.source = SourceId {
            file: "cleaning-tool-fixture".into(),
            path_id: i64::from(item.id.0),
        };
        let mut node = r.nodes.iter().find(|n| n.id == item.node).unwrap().clone();
        node.id = NodeId(r.nodes.iter().map(|n| n.id.0).max().unwrap() + 1);
        node.source = item.source.clone();
        node.gltf_node = r.nodes.iter().filter(|n| n.asset == node.asset).count() as u32;
        node.parent = None;
        node.name = name.into();
        node.transform = Transform {
            translation: [10000. + item.id.0 as f32, 100., 10000.],
            rotation: [0., 0., 0., 1.],
            scale: [1.; 3],
        };
        let mut body = r.bodies.iter().find(|b| b.id == item.body).unwrap().clone();
        body.id = BodyId(r.bodies.iter().map(|b| b.id.0).max().unwrap() + 1);
        body.source = item.source.clone();
        body.node = node.id;
        body.dynamic = true;
        body.mass_kg = 1.;
        body.parent_body = None;
        body.pose = node.transform.clone();
        item.node = node.id;
        item.body = body.id;
        item.kind = ItemKind::Decoration;
        item.name = name.into();
        item.attached_to = None;
        // These pickup values match checked brush72520 and spray72519 records.
        let pickup = item.pickup.as_mut().unwrap();
        pickup.source = item.source.clone();
        let result = (item.id, body.id, node.id, pickup.source.clone());
        r.nodes.push(node);
        r.bodies.push(body);
        r.items.push(item);
        result
    }
    for index in 0..reference.conditions.brushes.len() {
        let old = reference.conditions.brushes[index].item;
        let (item, body, node, pickup) = owner(reference, old, "Brush");
        let condition = add_condition_owner(reference, item);
        let tool = &mut reference.conditions.brushes[index];
        tool.item = item;
        tool.body = body;
        tool.pickup = pickup;
        tool.own_condition = condition;
        tool.model = Some(node);
        tool.model_equip = node;
        tool.model_default = node;
    }
    for index in 0..reference.conditions.sprays.len() {
        let old = reference.conditions.sprays[index].item;
        let (item, body, node, pickup) = owner(reference, old, "Spray");
        let tool = &mut reference.conditions.sprays[index];
        tool.item = item;
        tool.body = body;
        tool.pickup = pickup;
        tool.nozzle = node;
        tool.renderer_node = node;
    }
}

pub fn aim_at(
    runtime: &mut my_summer_slop::starting_area::GarageRuntime,
    reference: &my_summer_slop::reference::ReferenceManifest,
    item: my_summer_slop::reference::ItemId,
) -> my_summer_slop::interaction::AimTarget {
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
            my_summer_slop::simulation::FrameInput {
                mouse: [yaw, pitch],
                ..my_summer_slop::simulation::FrameInput::default()
            },
            &mut runtime.physics,
            1.0 / 60.0,
        );
        if let Some(target) = focus(
            &runtime.area.interaction,
            &runtime.physics,
            runtime.area.player.view_pose(&runtime.physics),
        ) && target.item == item
        {
            return target;
        }
    }
    panic!("cannot aim at source item {}", item.0);
}
