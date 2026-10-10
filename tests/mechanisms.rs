mod common;
use my_summer_slop::interaction::mechanisms::MechanismState;
use my_summer_slop::reference::*;
use my_summer_slop::simulation::{FrameInput, keys};
use my_summer_slop::vehicle::{VehiclePose, to_rapier};
use rapier3d::math::{Rotation, Vector};

fn view() -> VehiclePose {
    VehiclePose {
        position: [0.0; 3],
        rotation: [0.0, 0.0, 0.0, 1.0],
    }
}
fn fixture() -> ReferenceManifest {
    let mut r = common::reference();
    let mut body = r.bodies[1].clone();
    body.id = BodyId(90);
    body.parent_body = Some(r.vehicle.body);
    r.bodies.push(body);
    common::add_rotation(&mut r, 1, 90, [-30.0, 40.0]);
    let root = r.mechanisms[0].driven_nodes[0];
    let rotation = r.mechanisms[0].rotation.as_mut().unwrap();
    rotation.limits_degrees[1] = [-20.0, 25.0];
    rotation.targets[0].mapping[1] = [true, false, false];
    for (id, pos) in [
        (200, [0.0, 0.0, 0.0]),
        (201, [0.0, -1.0, 0.0]),
        (202, [0.0, 0.0, 0.0]),
        (203, [0.3, 0.0, 0.0]),
        (204, [0.3, 0.0, 0.0]),
    ] {
        let mut n = r.nodes.last().unwrap().clone();
        n.id = NodeId(id);
        n.parent = Some(root);
        n.transform.translation = pos;
        n.transform.rotation = view().rotation;
        n.transform.scale = [1.0; 3];
        r.nodes.push(n);
    }
    let mut window = r.mechanisms[0].clone();
    window.id = MechanismId(2);
    window.item = ItemId(2);
    window.parent_item = Some(ItemId(1));
    window.rotation = None;
    window.driven_nodes = vec![NodeId(202), NodeId(203)];
    window.slide = Some(SlideReference {
        initial_value: 0.0,
        speed: 0.1,
        sensitivity: 0.02,
        axes: [false, false, false, false, true, true],
        multiplier: [1.0, 1.0, 10.0],
        scroll_without_use: true,
        animation_source: None,
        paths: vec![SlidePath {
            node: NodeId(202),
            points: vec![
                SlidePoint {
                    node: NodeId(200),
                    value: 0.0,
                },
                SlidePoint {
                    node: NodeId(201),
                    value: 1.0,
                },
            ],
            rotate: true,
        }],
        rotations: vec![SlideRotation {
            node: NodeId(203),
            reference: NodeId(204),
            axis: [-1.0, 0.0, 0.0],
            angle_degrees: -810.0,
        }],
    });
    r.mechanisms[0].child = Some(window.item);
    r.mechanisms.push(window);
    r
}
#[test]
fn mechanism_limits_move_visual_and_collision_pose() {
    let r = fixture();
    let mut state = MechanismState::new(&r).unwrap();
    let body = r.mechanisms[0].driven_nodes[0];
    state.step(
        ItemId(1),
        FrameInput {
            held: keys::USE,
            mouse: [-100.0, -100.0],
            ..Default::default()
        },
        view(),
        0.016,
    );
    assert_eq!(state.rotation_degrees(ItemId(1)), Some([40.0, 25.0]));
    let expected = Rotation::from_rotation_y(-40.0f32.to_radians())
        * Rotation::from_rotation_x(-25.0f32.to_radians());
    assert!(
        to_rapier(state.node_pose(body).unwrap())
            .rotation
            .dot(expected)
            .abs()
            > 0.99999
    );
    assert_eq!(
        to_rapier(state.node_pose(body).unwrap()),
        state.body_pose(BodyId(90)).unwrap()
    );
    state.step(
        ItemId(2),
        FrameInput {
            held: keys::USE,
            mouse: [0.0, 125.0],
            ..Default::default()
        },
        view(),
        0.016,
    );
    assert!(
        (state.value(ItemId(2), false).unwrap() - 0.5).abs() < 1e-6,
        "source window opens with downward browser motion"
    );
    let parent = to_rapier(state.node_pose(body).unwrap());
    let glass = to_rapier(state.node_pose(NodeId(202)).unwrap());
    assert!(
        (parent.inverse() * glass)
            .translation
            .abs_diff_eq(Vector::new(0.0, -0.5, 0.0), 1e-5)
    );
    let handle = to_rapier(state.node_pose(NodeId(203)).unwrap());
    assert!(
        (parent.inverse() * handle)
            .rotation
            .dot(Rotation::from_rotation_x(405.0f32.to_radians()))
            .abs()
            > 0.99999
    );
    assert!(
        (state.value(ItemId(1), true).unwrap() - state.value(ItemId(1), false).unwrap() - 0.5)
            .abs()
            < 1e-6
    );
    for (mouse, expected) in [(10000.0, 1.0), (-10000.0, 0.0)] {
        state.step(
            ItemId(2),
            FrameInput {
                held: keys::USE,
                mouse: [0.0, mouse],
                ..Default::default()
            },
            view(),
            0.016,
        );
        assert_eq!(state.value(ItemId(2), false), Some(expected));
        let glass = to_rapier(state.node_pose(NodeId(202)).unwrap());
        assert!(
            (parent.inverse() * glass)
                .translation
                .abs_diff_eq(Vector::new(0.0, -expected, 0.0), 1e-5)
        );
    }
    state.step(
        ItemId(2),
        FrameInput {
            wheel: -1.0,
            ..Default::default()
        },
        view(),
        0.016,
    );
    assert!(state.value(ItemId(2), false).unwrap() > 0.0);
    state.cancel_input();
    let before = state.value(ItemId(2), false);
    state.step(ItemId(2), FrameInput::default(), view(), 0.016);
    assert_eq!(state.value(ItemId(2), false), before);
}
#[test]
fn mechanism_rules_preserve_child_values_and_reject_disabled_input() {
    let mut r = fixture();
    r.mechanisms[1].disable_on_uncraft = true;
    let mut state = MechanismState::new(&r).unwrap();
    state.set_mounted(ItemId(1), false);
    assert!(!state.contains(ItemId(2)));
    state.step(
        ItemId(2),
        FrameInput {
            held: keys::USE,
            mouse: [0.0, 100.0],
            ..Default::default()
        },
        view(),
        0.016,
    );
    assert_eq!(state.value(ItemId(2), false), Some(0.0));
    state.set_mounted(ItemId(1), true);
    assert!(state.contains(ItemId(2)));
}

#[test]
fn relative_two_axis_controls_keep_separate_source_pivots() {
    let mut r = fixture();
    let root = r.mechanisms[0].driven_nodes[0];
    r.mechanisms.truncate(1);
    r.mechanisms[0].child = None;
    r.mechanisms[0].driven_nodes.push(NodeId(202));
    r.mechanisms[0].drag.mode = DragMode::RelativeTwo;
    r.mechanisms[0].drag.axes2 = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
    let rotation = r.mechanisms[0].rotation.as_mut().unwrap();
    rotation.sum_axes = false;
    rotation.targets[0].mapping = [[false; 3], [true, false, false]];
    rotation.targets.push(RotationTarget {
        node: NodeId(202),
        mapping: [[false, true, false], [false; 3]],
    });
    let mut state = MechanismState::new(&r).unwrap();
    state.step(
        ItemId(1),
        FrameInput {
            held: keys::USE,
            mouse: [-15.0, -10.0],
            ..Default::default()
        },
        view(),
        0.016,
    );
    assert_eq!(state.rotation_degrees(ItemId(1)), Some([15.0, 10.0]));
    let parent = to_rapier(state.node_pose(root).unwrap());
    let child = to_rapier(state.node_pose(NodeId(202)).unwrap());
    assert!(
        parent
            .rotation
            .dot(Rotation::from_rotation_x(-10.0f32.to_radians()))
            .abs()
            > 0.99999
    );
    assert!(
        (parent.inverse() * child)
            .rotation
            .dot(Rotation::from_rotation_y(-15.0f32.to_radians()))
            .abs()
            > 0.99999
    );
}
