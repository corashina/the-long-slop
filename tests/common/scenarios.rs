use my_summer_slop::reference::*;

pub const CAN: ItemId = ItemId(50000);
pub const DESTINATION: ItemId = ItemId(50001);
pub const HINGE: ItemId = ItemId(50002);
pub const SUPPLY: ItemId = ItemId(50003);
pub const CHILD: ItemId = ItemId(50004);

pub fn item_mut(r: &mut ReferenceManifest, id: ItemId) -> &mut ItemRecord {
    r.items.iter_mut().find(|item| item.id == id).unwrap()
}
pub fn tank_mut(r: &mut ReferenceManifest, id: ItemId) -> &mut TankReference {
    r.tanks.iter_mut().find(|tank| tank.item == id).unwrap()
}
pub fn source_item(r: &ReferenceManifest, file: &str, path_id: i64) -> ItemId {
    r.items
        .iter()
        .find(|item| item.source.file == file && item.source.path_id == path_id)
        .unwrap()
        .id
}
pub fn car() -> ItemId {
    let r = super::reference();
    r.items
        .iter()
        .find(|item| item.node == r.vehicles[0].root)
        .unwrap()
        .id
}
pub fn driver() -> ItemId {
    super::reference()
        .seats
        .iter()
        .find(|seat| seat.driver && seat.vehicle == 0)
        .unwrap()
        .item
}
pub fn rim() -> ItemId {
    super::reference().wheel_slots[0].rim
}

pub fn containers() -> ReferenceManifest {
    let mut r = super::reference();
    for (id, name, position) in [
        (CAN, "Test carry can", [10., 2., -1.]),
        (DESTINATION, "Test destination", [10., 2., -2.]),
        (HINGE, "Test hinge", [10., 2., -3.]),
        (SUPPLY, "Test supply", [10., 2., -4.]),
        (CHILD, "Test child", [10., 2., -5.]),
    ] {
        let source = SourceId {
            file: "test-scenario".into(),
            path_id: i64::from(id.0),
        };
        let transform = Transform {
            translation: position,
            rotation: [0., 0., 0., 1.],
            scale: [1.; 3],
        };
        let node = NodeId(id.0);
        let body = BodyId(id.0);
        r.nodes.push(NodeRecord {
            id: node,
            source: source.clone(),
            name: name.into(),
            hierarchy_path: name.into(),
            asset: r.nodes[0].asset,
            gltf_node: id.0,
            parent: None,
            active: true,
            transform,
        });
        r.bodies.push(BodyRecord {
            id: body,
            source: source.clone(),
            node,
            dynamic: true,
            mass_kg: 7.,
            linear_damping: 0.,
            angular_damping: 0.,
            pose: transform,
            parent_body: None,
        });
        r.items.push(ItemRecord {
            id,
            source: source.clone(),
            node,
            body,
            name: name.into(),
            kind: ItemKind::Container,
            condition: 1.,
            mass_kg: 7.,
            attached_to: None,
            pickup: Some(PickupReference {
                source: source.clone(),
                cant_pickup: false,
                cant_pickup_while_mounted: false,
                cant_pickup_while_seated: false,
                no_rotation: false,
                move_with_rotation: false,
                use_recenter_position: false,
                recenter_position: [0.; 3],
                use_recenter_rotation: false,
                recenter_rotation: [0., 0., 0., 1.],
                recenter_distance: -0.5,
                can_inventory: false,
                can_hold_right_hand: false,
                disable_rh_fix: false,
                right_hand_position: [0.; 3],
                right_hand_rotation: [0., 0., 0., 1.],
                right_hand_grip: None,
                left_hand_grip: None,
                disable_when_stored: Some(source.clone()),
                keep_when_stored: None,
                inventory_icon: None,
            }),
        });
        let collider = ColliderId(id.0);
        r.colliders.push(ColliderRecord {
            id: collider,
            source: source.clone(),
            node,
            body,
            sensor: false,
            enabled: true,
            membership: 1,
            filter: u32::MAX,
            friction: 0.6,
            restitution: 0.,
            geometry: ColliderGeometry::Sphere {
                center: [0.; 3],
                radius: 0.2,
            },
        });
        r.action_bindings.push(ActionBinding {
            collider,
            use_owner: Some(id),
            pickup_owner: Some(id),
            removal_owner: None,
            push_owner: None,
        });
        if id == HINGE || id == CHILD {
            continue;
        }
        let opening_id = OpeningId(id.0);
        r.anchors.push(AnchorRecord {
            id: AnchorId(id.0),
            source: source.clone(),
            node,
            body,
            transform: Transform {
                translation: [0.; 3],
                rotation: [0., 0., 0., 1.],
                scale: [1.; 3],
            },
        });
        let mut opening = r.openings[0].clone();
        opening.id = opening_id;
        opening.source = source.clone();
        opening.tank = id;
        opening.cap = id;
        opening.anchor = AnchorId(id.0);
        opening.flow_liters_per_second = 0.5;
        opening.tilt_degrees = 100.;
        opening.angle_disabled = false;
        opening.can_toggle = true;
        opening.can_tune = false;
        opening.valve = 0.;
        opening.flow_radius = 0.02;
        opening.visuals = CapVisualReference {
            open_nodes: vec![],
            closed_nodes: vec![],
            cap_node: None,
            closed_node: None,
            open_node: None,
            euler_rotation: false,
            closed_euler_degrees: [0.; 3],
            open_euler_degrees: [0.; 3],
        };
        opening.presentation = FluidPresentation {
            has_name: false,
            name: String::new(),
            no_text: false,
        };
        for role in ["outlet", "splash"] {
            let mut particle = r
                .weather
                .particles
                .iter()
                .find(|p| p.role == format!("{role}_1"))
                .unwrap()
                .clone();
            particle.role = format!("{role}_{}", id.0);
            r.weather.particles.push(particle);
        }
        r.openings.push(opening);
        r.tanks.push(TankReference {
            item: id,
            source,
            capacity_liters: if id == SUPPLY { 1000. } else { 39. },
            quantities_liters: if id == DESTINATION {
                [0., 2.5, 0., 0., 0., 0.]
            } else {
                [0.5, 0., 0., 0., 0., 0.]
            },
            openings: vec![opening_id],
            no_drink: false,
            no_fill_this: false,
            no_fill_from: false,
            slow_oil: false,
            role: if id == SUPPLY {
                TankRole::Supply
            } else {
                TankRole::Portable
            },
        });
    }
    r
}
