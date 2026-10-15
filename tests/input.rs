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
fn lifecycle_boundary_discards_the_sampled_frame() {
    let app = include_str!("../src/app.rs");
    let boundary = app
        .find("if actions & 22 != 0")
        .expect("lifecycle actions must discard the frame sampled before input clear");
    let merge = app.find("drive.input.merge(frame)").unwrap();
    assert!(boundary < merge);
    assert!(app[boundary..merge].contains("return;"));
}

#[test]
fn driving_is_equal_at_different_render_rates() {
    use my_summer_slop::reference::SeatId;
    use my_summer_slop::simulation::StepClock;
    use my_summer_slop::simulation::input::{DrivingControls, KeyboardDriving};
    let mut expected = Vec::new();
    for fps in [30, 60, 120] {
        let mut clock = StepClock::default();
        let mut input = KeyboardDriving::new(1.25, 5.0);
        let mut samples = Vec::new();
        for _ in 0..fps / 2 {
            clock.accept_frame(1.0 / fps as f64);
            while clock.take_step(1.) {
                let held = if samples.len() < 6 {
                    keys::LEFT | keys::FORWARD | keys::BACK | keys::CLUTCH
                } else {
                    keys::LEFT
                };
                samples.push(input.step(Some(SeatId(1)), held));
            }
        }
        assert_eq!(samples.len(), 30);
        assert!((samples[29].steer - 0.625).abs() < 1e-5);
        assert_eq!(
            samples[29],
            DrivingControls {
                steer: samples[29].steer,
                ..Default::default()
            }
        );
        if expected.is_empty() {
            expected = samples;
        } else {
            assert_eq!(samples, expected);
        }
    }
}

mod common;
