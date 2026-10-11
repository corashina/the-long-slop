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
        held: keys::FORWARD,
        pressed: keys::GRAB,
        released: keys::SPACE,
        mouse: [12.0, 15.0],
        wheel: 2.0,
    };
    pending.clear();
    assert_eq!(pending.take_step(), FrameInput::default());
}

mod common;
#[test]
fn one_light_edge_survives_six_steps_hold_and_release() {
    use my_summer_slop::{reference::LightId, starting_area::GarageRuntime, vehicle::VehiclePose};
    let r = common::reference();
    let mut runtime = GarageRuntime::new(&r).unwrap();
    runtime.started = true;
    runtime.step(
        &r,
        FrameInput {
            pressed: keys::LIGHTS,
            ..Default::default()
        },
    );
    assert!(!runtime.lighting.high_beam());
    let seat = r.seats.iter().find(|s| s.driver).unwrap();
    assert!(runtime.area.player.enter(
        seat.id,
        VehiclePose {
            position: [20., 2., 20.],
            rotation: [0., 0., 0., 1.]
        },
        &mut runtime.physics
    ));
    let mut input = FrameInput {
        held: keys::LIGHTS,
        pressed: keys::LIGHTS,
        ..Default::default()
    };
    for _ in 0..6 {
        runtime.step(&r, input.take_step());
    }
    for _ in 0..3 {
        runtime.lighting.advance_fades(&r.lighting, 0.1);
    }
    assert!(
        runtime
            .lighting
            .light_outputs()
            .iter()
            .find(|l| l.id == LightId(13))
            .unwrap()
            .enabled
    );
    assert!(!runtime.lighting.high_beam());
    input.merge(FrameInput {
        held: keys::LIGHTS,
        ..Default::default()
    });
    runtime.step(&r, input.take_step());
    input.merge(FrameInput {
        released: keys::LIGHTS,
        ..Default::default()
    });
    runtime.step(&r, input.take_step());
    assert!(!runtime.lighting.high_beam());
    input.merge(FrameInput {
        held: keys::LIGHTS,
        pressed: keys::LIGHTS,
        ..Default::default()
    });
    runtime.step(&r, input.take_step());
    assert!(runtime.lighting.high_beam());
}

#[test]
fn frame_fade_uses_active_elapsed_once_after_catch_up() {
    use my_summer_slop::starting_area::GarageRuntime;
    let mut r = common::reference();
    for c in &mut r.lighting.controllers {
        if c.kind == my_summer_slop::reference::LightControllerKind::Headlight {
            c.energy = 1.;
        }
    }
    let mut runtime = GarageRuntime::new(&r).unwrap();
    runtime.started = true;
    runtime.lighting.cycle_headlights(&r.lighting);
    for _ in 0..6 {
        runtime.step(&r, FrameInput::default());
    }
    runtime.advance_lighting(&r, 0.5);
    assert_eq!(runtime.lighting.cycle.elapsed_seconds(), 0.5);
    runtime.advance_light_fades(&r, 0.1);
    assert!(
        !runtime
            .lighting
            .light_outputs()
            .iter()
            .find(|o| o.id.0 == 13)
            .unwrap()
            .enabled
    );
    runtime.paused = true;
    runtime.advance_light_fades(&r, 0.1);
    assert!(
        !runtime
            .lighting
            .light_outputs()
            .iter()
            .find(|o| o.id.0 == 13)
            .unwrap()
            .enabled
    );
    runtime.paused = false;
    runtime.advance_light_fades(&r, 0.1);
    assert!(
        !runtime
            .lighting
            .light_outputs()
            .iter()
            .find(|o| o.id.0 == 13)
            .unwrap()
            .enabled
    );
    runtime.advance_light_fades(&r, 0.1);
    assert!(
        runtime
            .lighting
            .light_outputs()
            .iter()
            .find(|o| o.id.0 == 13)
            .unwrap()
            .enabled
    );
}

#[test]
#[ignore = "Requires exported Steam assets"]
fn exported_driver_lights_use_real_edges() {
    use my_summer_slop::{reference::ReferenceManifest, starting_area::GarageRuntime};
    let mut r =
        ReferenceManifest::from_json(&std::fs::read("web/assets/reference/manifest.json").unwrap())
            .unwrap();
    r.area.player_spawn.translation = [37.8, 1.85, -69.3];
    r.area.player_spawn.rotation = [0., 0., 0., 1.];
    let mut runtime = GarageRuntime::new(&r).unwrap();
    runtime.started = true;
    for mut input in [
        FrameInput {
            mouse: [0., 85.],
            ..Default::default()
        },
        FrameInput {
            held: keys::USE,
            pressed: keys::USE,
            ..Default::default()
        },
        FrameInput {
            released: keys::USE,
            ..Default::default()
        },
    ] {
        runtime.step(&r, input.take_step());
    }
    assert!(runtime.area.player.driving());
    for _ in 0..2 {
        let mut input = FrameInput {
            held: keys::LIGHTS,
            pressed: keys::LIGHTS,
            ..Default::default()
        };
        for _ in 0..6 {
            runtime.step(&r, input.take_step());
        }
        runtime.advance_light_fades(&r, 0.1);
    }
    assert!(runtime.lighting.high_beam());
    for _ in 0..3 {
        runtime.advance_light_fades(&r, 0.1);
    }
    let rlight = runtime
        .lighting
        .light_outputs()
        .iter()
        .find(|o| o.id.0 == 14)
        .unwrap();
    assert!(rlight.enabled && rlight.intensity > 0.);
}
