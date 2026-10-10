use my_summer_slop::simulation::{FrameInput, keys};

#[test]
fn catch_up_steps_consume_toggle_once_and_keep_held_input() {
    let mut pending = FrameInput::default();
    pending.merge(FrameInput {
        held: keys::IGNITION,
        pressed: keys::IGNITION | keys::SPACE,
        mouse: [4.0, -3.0],
        ..Default::default()
    });
    let mut toggles = 0;
    let mut look = [0.0; 2];
    for _ in 0..6 {
        let input = pending.take_step();
        toggles += (input.pressed & keys::SPACE != 0) as usize;
        assert_ne!(input.held & keys::IGNITION, 0);
        for (total, delta) in look.iter_mut().zip(input.mouse) {
            *total += delta;
        }
    }
    assert_eq!(toggles, 1);
    assert_eq!(look, [4.0, -3.0]);
}

#[test]
fn frames_without_physics_keep_edges_and_accumulate_look() {
    let mut pending = FrameInput::default();
    pending.merge(FrameInput {
        held: keys::USE,
        pressed: keys::USE,
        mouse: [2.0, 1.0],
        ..Default::default()
    });
    pending.merge(FrameInput {
        released: keys::USE,
        mouse: [3.0, -1.0],
        ..Default::default()
    });
    let input = pending.take_step();
    assert_eq!(input.held, 0);
    assert_eq!(input.pressed, keys::USE);
    assert_eq!(input.released, keys::USE);
    assert_eq!(input.mouse, [5.0, 0.0]);
    assert_eq!(pending.take_step(), FrameInput::default());
}

#[test]
fn pause_clear_discards_held_keys_edges_and_look() {
    let mut pending = FrameInput {
        held: keys::FORWARD | keys::ROTATE | keys::TRANSLATE | keys::RECENTER,
        pressed: keys::GRAB,
        released: keys::SPACE,
        mouse: [12.0, 15.0],
        wheel: 2.0,
    };
    pending.clear();
    assert_eq!(pending.take_step(), FrameInput::default());
}

#[test]
fn carry_input_preserves_modes_and_consumes_recenter_once() {
    let mut pending = FrameInput {
        held: keys::ROTATE | keys::TRANSLATE | keys::RECENTER,
        pressed: keys::RECENTER,
        mouse: [2.0, -3.0],
        ..Default::default()
    };
    let first = pending.take_step();
    let second = pending.take_step();
    assert_eq!(first.pressed, keys::RECENTER);
    assert_eq!(second.pressed, 0);
    assert_eq!(second.held, first.held);
    assert_eq!(second.mouse, [0.0; 2]);
    let bridge = include_str!("../web/main.js");
    assert!(bridge.contains("Mouse1: 4096"));
    assert!(bridge.contains("Mouse2: 16384"));
    assert!(bridge.contains("Tab: 32768"));
    assert_eq!(
        bridge
            .matches("event.button >= 0 && event.button <= 2")
            .count(),
        2
    );
}

#[test]
fn f_release_edge_runs_once_across_catch_up_steps() {
    let mut pending = FrameInput::default();
    pending.merge(FrameInput {
        held: keys::GRAB,
        pressed: keys::GRAB,
        ..Default::default()
    });
    pending.merge(FrameInput {
        released: keys::GRAB,
        ..Default::default()
    });
    let first = pending.take_step();
    assert_eq!(first.pressed, keys::GRAB);
    assert_eq!(first.released, keys::GRAB);
    assert_eq!(first.held, 0);
    for _ in 0..5 {
        assert_eq!(pending.take_step(), FrameInput::default());
    }
}
