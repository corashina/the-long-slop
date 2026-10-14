#![allow(dead_code)]

mod scenarios;
#[allow(unused_imports)]
pub use scenarios::*;

use my_summer_slop::reference::{ColliderGeometry, ColliderId, ReferenceManifest};
use my_summer_slop::vehicle::{PhysicsWorld, VehiclePose};

pub fn reference() -> ReferenceManifest {
    ReferenceManifest::from_json(include_bytes!("reference.json")).expect("valid source fixture")
}

pub fn physics_reference() -> ReferenceManifest {
    let mut reference = reference();
    for index in 0..reference.vehicles.len() {
        let vehicle = &reference.vehicles[index];
        let mut shape = reference
            .colliders
            .iter()
            .find(|c| c.id == vehicle.colliders[0])
            .unwrap()
            .clone();
        let id = ColliderId(60000 + index as u32 * 2);
        shape.id = id;
        shape.source = my_summer_slop::reference::SourceId {
            file: "test-physics".into(),
            path_id: i64::from(id.0),
        };
        shape.node = vehicle.root;
        shape.body = vehicle.body;
        shape.enabled = true;
        shape.sensor = false;
        shape.geometry = ColliderGeometry::Box {
            center: [0.; 3],
            half_extents: [0.76, 0.24, 1.55],
            rotation: [0., 0., 0., 1.],
        };
        let mut cabin = shape.clone();
        cabin.id = ColliderId(id.0 + 1);
        cabin.source.path_id += 1;
        cabin.geometry = ColliderGeometry::Box {
            center: [0., 0.57, 0.2],
            half_extents: [0.65, 0.36, 0.72],
            rotation: [0., 0., 0., 1.],
        };
        reference.vehicles[index].colliders = vec![id, cabin.id];
        reference.colliders.extend([shape, cabin]);
    }
    reference
}

pub fn world() -> PhysicsWorld {
    let reference = physics_reference();
    let poses = my_summer_slop::starting_area::initial_vehicle_poses(&reference)
        .expect("valid fixture startup poses");
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
    reference.mechanisms.push(MechanismReference {
        id: MechanismId(item),
        source: my_summer_slop::reference::SourceId {
            file: "test-mechanism".into(),
            path_id: i64::from(item),
        },
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

pub fn uncover_wheel(
    g: &mut my_summer_slop::starting_area::GarageRuntime,
    rim: my_summer_slop::reference::ItemId,
) {
    if let Some(cover) = g.area.interaction.parts.covering_item(rim) {
        detach(g, cover);
    }
}

pub fn saved_vehicle_index(
    save: &my_summer_slop::save::SaveSnapshot,
    r: &ReferenceManifest,
    index: usize,
) -> usize {
    let instance = my_summer_slop::save::InstanceId::initial(&r.vehicles[index].source);
    save.vehicles
        .iter()
        .position(|vehicle| vehicle.instance == instance)
        .unwrap()
}

pub fn assert_rotation_close(a: [f32; 4], b: [f32; 4]) {
    let same = a
        .iter()
        .zip(b)
        .all(|(a, b)| (*a - b).abs() <= 8. * f32::EPSILON);
    let opposite = a
        .iter()
        .zip(b)
        .all(|(a, b)| (*a + b).abs() <= 8. * f32::EPSILON);
    assert!(same || opposite, "rotation {a:?} != {b:?}");
}
pub fn assert_rebased_position(a: [f64; 3], b: [f64; 3], shift: [f32; 3]) {
    for axis in 0..3 {
        let scale = a[axis]
            .abs()
            .max(b[axis].abs())
            .max(f64::from(shift[axis]).abs())
            .max(1.);
        let tolerance = 4. * f64::from(f32::EPSILON) * scale;
        assert!(
            (a[axis] - b[axis]).abs() <= tolerance,
            "position {a:?} != {b:?}, bound {tolerance}"
        );
    }
}
pub fn assert_rebased_pose(
    a: my_summer_slop::save::AbsolutePose,
    b: my_summer_slop::save::AbsolutePose,
    shift: [f32; 3],
) {
    assert_rebased_position(a.position, b.position, shift);
    assert_rotation_close(a.rotation, b.rotation);
}
