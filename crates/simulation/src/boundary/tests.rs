use super::*;
use crate::reference::ItemId;
use glam::Quat;

#[test]
fn frame_round_trips() {
    let row = |x: f32| PoseRow {
        position: [x, 2., 3.],
        flags: PoseRow::VISIBLE,
        rotation: [0., 0., 0., 1.],
    };
    let frame = SimFrame {
        seq_consumed: 42,
        state: StateBlock {
            simulation_seconds: 12.5,
            origin: [1., 2., 3.],
            stream_origin: [4., 5., 6.],
            cycle_elapsed_seconds: 40.,
            session: 3,
            selection_revision: 9,
            alpha: 0.25,
            time_scale: 2.,
            flags: StateBlock::STARTED | StateBlock::SEATED,
            fov_degrees: 60.,
            stored_items: [0, 8, 0, 3],
            body_pose: row(7.),
            body_root: row(8.),
            joint_scale: 0.01,
            sky_anchor: [0., 300., 0.],
            ember_light: 8,
            dead_weight: 0.5,
            motion_revision: 2,
            far_plane: 1500.,
            sleep_camera: row(9.),
            seat_car: PoseRow::default(),
            prepared_roads: 12,
            pad: 0,
            look: LookBlock {
                yaw: 30.,
                gain: 1.,
                ..Default::default()
            },
        },
        hud: HudBlock {
            hp: 80.,
            slots: [0, 7, 0, 0],
            absolute: [1e6, 2., 3.],
            flags: HudBlock::CAR,
            ..HudBlock::default()
        },
        previous: vec![row(1.), row(4.)],
        current: vec![row(2.), row(5.)],
        items: vec![
            ItemRow::default(),
            ItemRow {
                scale: 0.5,
                bite: 2,
            },
        ],
        joints: vec![row(9.)],
        ragdoll: vec![row(10.)],
        cigarettes: vec![CigaretteRow {
            item: 7,
            blend: 0.5,
            color: [1., 0.5, 0., 1.],
            light_intensity: 2.,
        }],
        particle_groups: vec![ParticleGroupRow {
            kind: ParticleGroupRow::CAR,
            a: ParticleGroupRow::CAR_WHEEL,
            b: 2,
            c: 9,
            source: 17,
            start: 0,
            count: 1,
            fluid: 0,
        }],
        particles: vec![ParticleRow {
            position: [1., 2., 3.],
            size: 0.5,
            age: 1.,
            lifetime: 2.,
            ..Default::default()
        }],
        decals: vec![DecalRow {
            previous: row(4.),
            current: row(5.),
            kind: 3,
            scale: 0.25,
        }],
        planes: vec![PlaneRow {
            id: 3,
            prefab: 1,
            offset: [10., 0., 5.],
            heading: [0., 0., 0., 1.],
        }],
        conditions: vec![ConditionRow {
            id: 4,
            state: 2,
            state2: 1,
            state3: 0,
            repair_progress: -0.5,
            color: [0.2, 0.3, 0.4, 1.],
        }],
        washes: vec![1, 0],
        brush_poses: vec![PoseRow::default()],
        spray_colors: vec![[1., 0., 0., 1.]],
        audio: vec![
            AudioCall::Begin {
                session: 1,
                started: true,
                paused: false,
                seconds: 3.5,
            },
            AudioCall::Listener,
            AudioCall::Radio {
                emitter: 40,
                frame: crate::simulation::radio::RadioFrame {
                    power: true,
                    frequency: 98.5,
                    ..Default::default()
                },
            },
            AudioCall::End,
        ],
        budgets: vec![BudgetRow {
            used: 3.,
            limit: 8.,
        }],
        lighting: FrameLighting {
            cycle: crate::lighting::CycleOutput {
                progress: 0.3,
                grading_temperature: Some(5.),
                ..Default::default()
            },
            road_lamp: LightOutput {
                id: crate::reference::LightId(4),
                enabled: true,
                intensity: 1.5,
            },
        },
        lights: vec![LightOutput::default()],
        lamp_materials: Vec::new(),
        control_poses: Vec::new(),
        node_visibility: vec![crate::lighting::NodeVisibilityOutput {
            node: crate::reference::NodeId(2),
            visible: true,
        }],
        valves: vec![ValveRow {
            opening: 4,
            valve: 0.5,
        }],
        structures: vec![StructureRow {
            slot: 3,
            flags: StructureRow::READY | StructureRow::LAMP,
            lamp: [1., 0.5, 0., 1.],
            display_map: 0,
            poses: [0, 1],
            hidden: [0, 1],
            materials: [0, 1],
        }],
        structure_poses: vec![NodePoseRow {
            node: 2,
            kind: NodePoseRow::BODY,
            translation: [1., 2., 3.],
            rotation: [0., 0., 0., 1.],
            scale: [1.; 3],
        }],
        hidden_nodes: vec![5],
        node_materials: vec![MaterialRow { node: 1, source: 0 }],
        changes: vec![
            Change::Spawn {
                handle: Handle(1),
                key: PoseKey::Item(ItemId(7)),
                data: SpawnData::Spawned(crate::starting_area::spawned::SpawnedItem {
                    id: ItemId(7),
                    serial: 2,
                    prototype: 3,
                    pickup: 1,
                }),
            },
            Change::PlayerRig(vec![crate::reference::NodeId(5)]),
            Change::Retire(Handle(0)),
            Change::HomeCable(vec![RopeAim {
                node: crate::reference::NodeId(9),
                rotation: [0., 0., 0., 1.],
                scale_z: 0.4,
            }]),
            Change::FollowedCar(Some((
                crate::world::GeneratedId {
                    layer: crate::world::WorldLayer::Startup,
                    generator: crate::reference::SourceId {
                        file: "level0".into(),
                        path_id: 5,
                    },
                    unit: [1, -2],
                    ordinal: 4,
                    children: Vec::new(),
                },
                6,
            ))),
            Change::Feedback(Box::new(Feedback {
                name: "Jerry can".into(),
                amount: 4.5,
                quantities: [4.5, 0., 0., 0., 0., 0.],
                ..Feedback::default()
            })),
            Change::StructureRemove(3),
            Change::MaterialSource(crate::reference::SourceId {
                file: "level0".into(),
                path_id: 8,
            }),
            Change::Failure("stream".into()),
        ],
    };
    let mut bytes = Vec::new();
    encode(&frame, &mut bytes);
    let mut decoded = SimFrame::default();
    decode(&bytes, &mut decoded).unwrap();
    assert_eq!(decoded, frame);
    assert!(decode(&bytes[..20], &mut decoded).is_err());
}

#[test]
fn command_round_trips() {
    let mut command = SimCommand {
        seq: 9,
        delta_seconds: 1. / 60.,
        ..Default::default()
    };
    command.input.pressed = 5;
    command.input.mouse = [0.5, -1.];
    command.input.carry.owner = Some(ItemId(3));
    command.input.typing.text[..2].copy_from_slice(b"hi");
    command.input.typing.len = 2;
    command.input.gamepad.axes[31] = 0.75;
    command.actions = SimCommand::PAUSE;
    command.assists = Some(crate::reference::DrivingAssists {
        automatic_clutch: true,
        automatic_gears: false,
    });
    let mut bytes = Vec::new();
    encode_command(&command, &mut bytes);
    assert_eq!(decode_command(&bytes).unwrap(), command);
}

#[test]
fn command_merge_keeps_edges_and_sums_deltas() {
    let mut first = SimCommand {
        seq: 1,
        delta_seconds: 0.25,
        actions: SimCommand::PAUSE,
        ..Default::default()
    };
    first.input.held = 1;
    first.input.pressed = 1;
    first.input.mouse = [1., 2.];
    let mut second = SimCommand {
        seq: 2,
        delta_seconds: 0.5,
        ready: true,
        ..Default::default()
    };
    second.input.held = 2;
    second.input.released = 1;
    second.input.mouse = [0.5, -1.];
    first.merge(second.clone());
    assert_eq!(
        (first.seq, first.delta_seconds, first.ready),
        (2, 0.75, true)
    );
    assert_eq!(first.actions, SimCommand::PAUSE);
    assert_eq!(
        (first.input.held, first.input.pressed, first.input.released),
        (2, 1, 1)
    );
    assert_eq!(first.input.mouse, [1.5, 1.]);
    second.hold = true;
    first.merge(second);
    assert!(first.hold);
    assert_eq!(first.input, FrameInput::default());
}

#[test]
fn handle_is_reused_only_after_release() {
    let mut handles = Handles::default();
    let (a, new) = handles.assign(PoseKey::Item(ItemId(1)));
    assert!(new);
    assert_eq!(handles.assign(PoseKey::Item(ItemId(1))), (a, false));
    handles.retire(a);
    let (b, _) = handles.assign(PoseKey::Item(ItemId(2)));
    assert_ne!(a, b);
    handles.release(a);
    let (c, _) = handles.assign(PoseKey::Item(ItemId(3)));
    assert_eq!(c, a);
    assert_eq!(handles.len(), 2);
}

fn look_frame(yaw: f32, pitch: f32, seq_consumed: u64) -> SimFrame {
    let mut frame = SimFrame {
        seq_consumed,
        ..Default::default()
    };
    frame.state.alpha = 1.;
    frame.state.look = LookBlock {
        position: [[0., 1., 0.], [1., 2., 3.]],
        parent: [[0., 0., 0., 1.], Quat::from_rotation_y(0.5).to_array()],
        camera: [0., 0., 0., 1.],
        yaw,
        pitch,
        gain: 2.,
        yaw_limits: [-90., 90.],
        pitch_limits: [-60., 60.],
        pad: 0,
    };
    frame
}

#[test]
fn mouse_look_adds_unconsumed_mouse_within_limits() {
    let mut look = MouseLook::default();
    look.send(1, [5., 0.]);
    look.send(2, [1., 40.]);
    let pose = look.camera(&look_frame(10., 0., 1));
    let expected = Quat::from_rotation_y(0.5)
        * Quat::from_rotation_y(-12f32.to_radians())
        * Quat::from_rotation_x(-60f32.to_radians());
    assert!(Quat::from_array(pose.rotation).abs_diff_eq(expected, 1e-5));
    assert_eq!(pose.position, [1., 2., 3.]);
}

#[test]
fn mouse_look_matches_the_frame_after_consume() {
    let mut look = MouseLook::default();
    look.send(1, [3., -2.]);
    let before = look.camera(&look_frame(10., 5., 0));
    // The step applies the mouse with the same gain.
    let after = look.camera(&look_frame(16., 1., 1));
    assert!(Quat::from_array(before.rotation).abs_diff_eq(Quat::from_array(after.rotation), 1e-5));
    assert_eq!(look.camera(&look_frame(16., 1., 1)), after);
}
