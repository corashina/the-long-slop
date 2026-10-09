use desert_drive::simulation::{FrameInput, keys};

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
        held: keys::FORWARD,
        pressed: keys::GRAB,
        released: keys::SPACE,
        mouse: [12.0, 15.0],
        wheel: 2.0,
    };
    pending.clear();
    assert_eq!(pending.take_step(), FrameInput::default());
}
