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
fn driving_ramps_are_independent_and_clear_at_input_boundaries() {
    use my_summer_slop::reference::SeatId;
    use my_summer_slop::simulation::input::{DrivingControls, KeyboardDriving};
    let mut input = KeyboardDriving::new(1.25, 5.0);
    let seat = Some(SeatId(1));
    let mut sample = DrivingControls::default();
    for _ in 0..6 {
        sample = input.step(seat, keys::FORWARD | keys::BACK | keys::CLUTCH | keys::LEFT);
    }
    assert!((sample.throttle - 0.5).abs() < 1e-5);
    assert!((sample.brake - 0.5).abs() < 1e-5);
    assert!((sample.clutch - 0.5).abs() < 1e-5);
    assert!((sample.steer - 0.125).abs() < 1e-5);
    sample = input.step(seat, keys::BACK);
    assert!((sample.throttle - 5.0 / 12.0).abs() < 1e-5);
    assert!((sample.brake - 7.0 / 12.0).abs() < 1e-5);
    assert!((sample.clutch - 5.0 / 12.0).abs() < 1e-5);
    assert!((sample.steer - 1.25 / 12.0).abs() < 1e-5);
    sample = input.step(Some(SeatId(2)), keys::RIGHT | keys::CLUTCH);
    assert!((sample.steer + 1.25 / 60.0).abs() < 1e-5);
    assert_eq!(sample.throttle, 0.0);
    assert_eq!(sample.brake, 0.0);
    assert!((sample.clutch - 5.0 / 60.0).abs() < 1e-5);
    assert_eq!(input.step(None, u32::MAX), DrivingControls::default());
    assert_eq!(input.step(seat, 0), DrivingControls::default());
    input.step(seat, u32::MAX);
    input.clear();
    assert_eq!(input.step(seat, 0), DrivingControls::default());
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

#[test]
fn driving_edges_survive_no_step_and_run_once_during_catch_up() {
    use my_summer_slop::simulation::StepClock;
    let actions = keys::CLUTCH
        | keys::SHIFT_UP
        | keys::SHIFT_DOWN
        | keys::REVERSE
        | keys::NEUTRAL
        | keys::GEAR_1
        | keys::GEAR_2
        | keys::GEAR_3
        | keys::GEAR_4
        | keys::GEAR_5
        | keys::GEAR_6;
    assert_eq!(keys::GEAR_6, 2147483648);
    let mut pending = FrameInput::default();
    let mut clock = StepClock::default();
    pending.merge(FrameInput {
        held: actions,
        pressed: actions,
        ..Default::default()
    });
    clock.accept_frame(1.0 / 120.0);
    assert!(!clock.take_step(1.));
    pending.merge(FrameInput {
        released: actions,
        ..Default::default()
    });
    clock.accept_frame(5.0 / 60.0);
    let mut steps = 0;
    while clock.take_step(1.) {
        let sample = pending.take_step();
        assert_eq!(sample.held, 0);
        assert_eq!(sample.pressed, if steps == 0 { actions } else { 0 });
        assert_eq!(sample.released, if steps == 0 { actions } else { 0 });
        steps += 1;
    }
    assert_eq!(steps, 5);
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
    assert!(!runtime.lighting.high_beam(0));
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
    assert!(!runtime.lighting.high_beam(0));
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
    assert!(!runtime.lighting.high_beam(0));
    input.merge(FrameInput {
        held: keys::LIGHTS,
        pressed: keys::LIGHTS,
        ..Default::default()
    });
    runtime.step(&r, input.take_step());
    assert!(runtime.lighting.high_beam(0));
}

#[test]
fn fixed_steps_advance_cycle_and_fades_once_and_pause_freezes_both() {
    use my_summer_slop::starting_area::GarageRuntime;
    let mut r = common::reference();
    for c in &mut r.lighting.controllers {
        if c.kind == my_summer_slop::reference::LightControllerKind::Headlight {
            c.energy = 1.;
        }
    }
    let mut runtime = GarageRuntime::new(&r).unwrap();
    runtime.started = true;
    runtime.lighting.cycle_headlights(&r.lighting, 0);
    let low_enabled = |runtime: &GarageRuntime| {
        runtime
            .lighting
            .light_outputs()
            .iter()
            .find(|o| o.id.0 == 13)
            .unwrap()
            .enabled
    };
    for _ in 0..6 {
        runtime.step(&r, FrameInput::default());
    }
    let elapsed = runtime.lighting.cycle.elapsed_seconds();
    assert!((elapsed - 6. * f64::from(my_summer_slop::simulation::FIXED_DT)).abs() < 1e-9);
    assert!(!low_enabled(&runtime));
    runtime.paused = true;
    for _ in 0..6 {
        runtime.step(&r, FrameInput::default());
    }
    assert_eq!(runtime.lighting.cycle.elapsed_seconds(), elapsed);
    assert!(!low_enabled(&runtime));
    runtime.paused = false;
    runtime.step(&r, FrameInput::default());
    assert!(!low_enabled(&runtime));
    runtime.step(&r, FrameInput::default());
    assert!(low_enabled(&runtime));
}

#[test]
#[ignore = "Requires exported Steam assets"]
fn exported_driver_lights_use_real_edges() {
    use my_summer_slop::{reference::ReferenceManifest, starting_area::GarageRuntime};
    let mut r =
        ReferenceManifest::from_json(&std::fs::read("web/assets/reference/manifest.json").unwrap())
            .unwrap();
    let tilt = (-85.0_f32).to_radians();
    let head = r.player.standing_height * 0.5;
    r.area.player_spawn.translation = [
        37.8,
        1.85 + head - head * tilt.cos(),
        -69.3 - head * tilt.sin(),
    ];
    r.area.player_spawn.rotation = [(tilt / 2.).sin(), 0., 0., (tilt / 2.).cos()];
    let mut runtime = GarageRuntime::new(&r).unwrap();
    runtime.started = true;
    let mut input = FrameInput {
        pressed: keys::USE,
        released: keys::USE,
        ..Default::default()
    };
    for _ in 0..6 {
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
    }
    assert!(runtime.lighting.high_beam(0));
    for _ in 0..18 {
        runtime.step(&r, FrameInput::default());
    }
    let rlight = runtime
        .lighting
        .light_outputs()
        .iter()
        .find(|o| o.id.0 == 14)
        .unwrap();
    assert!(rlight.enabled && rlight.intensity > 0.);
}

#[test]
#[ignore = "Requires exported Steam assets"]
fn exported_low_startup_keeps_source_outputs_through_pause() {
    use my_summer_slop::{
        reference::{LightControllerKind, ReferenceManifest},
        starting_area::GarageRuntime,
    };
    let mut r =
        ReferenceManifest::from_json(&std::fs::read("web/assets/reference/manifest.json").unwrap())
            .unwrap();
    r.lighting.cycle.startup_offset_seconds = 1348.;
    for c in &mut r.lighting.controllers {
        if c.kind == LightControllerKind::Headlight {
            c.current_state = 1;
            c.requested_state = 1;
            c.energy = 1.;
        }
    }
    let mut runtime = GarageRuntime::new(&r).unwrap();
    runtime.started = true;
    for _ in 0..132 {
        runtime.step(&r, FrameInput::default());
    }
    for c in r
        .lighting
        .controllers
        .iter()
        .filter(|c| c.kind == LightControllerKind::Headlight)
    {
        for id in &c.states[1].lights {
            let output = runtime
                .lighting
                .light_outputs()
                .iter()
                .find(|o| o.id == *id)
                .unwrap();
            assert!(output.enabled);
            assert_eq!(
                output.intensity,
                c.states[1].intensity * c.electronics_output
            );
        }
        for id in &c.states[2].lights {
            assert!(
                !runtime
                    .lighting
                    .light_outputs()
                    .iter()
                    .find(|o| o.id == *id)
                    .unwrap()
                    .enabled
            );
        }
        for renderer in &c.states[1].renderers {
            let material = c.states[1].on_material.as_ref().unwrap();
            let output = runtime
                .lighting
                .material_outputs()
                .iter()
                .find(|o| o.renderer == *renderer)
                .unwrap();
            assert_eq!(
                (output.asset, output.material_index),
                (material.asset, material.gltf_material)
            );
        }
    }
    assert!(!runtime.lighting.high_beam(0));
    let lights = runtime
        .lighting
        .light_outputs()
        .iter()
        .map(|o| (o.id, o.enabled, o.intensity))
        .collect::<Vec<_>>();
    let materials = runtime
        .lighting
        .material_outputs()
        .iter()
        .map(|o| (o.renderer, o.asset, o.material_index))
        .collect::<Vec<_>>();
    runtime.paused = true;
    runtime.step(
        &r,
        FrameInput {
            pressed: keys::LIGHTS,
            ..Default::default()
        },
    );
    assert_eq!(
        lights,
        runtime
            .lighting
            .light_outputs()
            .iter()
            .map(|o| (o.id, o.enabled, o.intensity))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        materials,
        runtime
            .lighting
            .material_outputs()
            .iter()
            .map(|o| (o.renderer, o.asset, o.material_index))
            .collect::<Vec<_>>()
    );
    assert!(!runtime.lighting.high_beam(0));
}

#[test]
fn car_and_headlight_electronics_keep_their_source_owners() {
    use my_summer_slop::{reference::LightControllerKind, starting_area::GarageRuntime};
    let mut r = common::reference();
    let mut part = 0;
    for c in &mut r.lighting.controllers {
        if c.kind == LightControllerKind::Rear {
            c.electronics_output = 0.2;
        } else if c.kind == LightControllerKind::Headlight {
            c.current_state = 2;
            c.requested_state = 2;
            c.energy = 1.;
            c.electronics_output = if part == 0 { 0.75 } else { 0.5 };
            part += 1;
        }
    }
    let mut runtime = GarageRuntime::new(&r).unwrap();
    runtime.started = true;
    runtime.vehicles[0].engine.ignition = true;
    runtime.step(&r, FrameInput::default());
    assert!(!runtime.lighting.high_beam(0));
    for c in r
        .lighting
        .controllers
        .iter()
        .filter(|c| c.kind == LightControllerKind::Headlight)
    {
        for id in &c.states[2].lights {
            let output = runtime
                .lighting
                .light_outputs()
                .iter()
                .find(|o| o.id == *id)
                .unwrap();
            assert_eq!(
                output.intensity,
                c.states[2].intensity * c.electronics_output
            );
        }
    }
    for id in 9..=12 {
        assert!(
            !runtime
                .lighting
                .light_outputs()
                .iter()
                .find(|o| o.id.0 == id)
                .unwrap()
                .enabled
        );
    }
}

#[test]
#[ignore = "Requires exported Steam assets"]
fn exported_brake_lights_sample_pedal_before_physics() {
    use my_summer_slop::{reference::ReferenceManifest, starting_area::GarageRuntime};
    let mut r =
        ReferenceManifest::from_json(&std::fs::read("web/assets/reference/manifest.json").unwrap())
            .unwrap();
    let tilt = (-85.0_f32).to_radians();
    let head = r.player.standing_height * 0.5;
    r.area.player_spawn.translation = [
        37.8,
        1.85 + head - head * tilt.cos(),
        -69.3 - head * tilt.sin(),
    ];
    r.area.player_spawn.rotation = [(tilt / 2.).sin(), 0., 0., (tilt / 2.).cos()];
    let mut runtime = GarageRuntime::new(&r).unwrap();
    runtime.started = true;
    runtime.vehicles[0].engine.ignition = true;
    runtime.step(
        &r,
        FrameInput {
            pressed: keys::USE,
            released: keys::USE,
            ..Default::default()
        },
    );
    assert!(runtime.area.player.driving());
    let chassis = runtime.physics.chassis_handle(0);
    let velocity = runtime.physics.rapier().bodies[chassis].position().rotation
        * -rapier3d::math::Vector::Z
        * 2.;
    runtime.physics.rapier_mut().bodies[chassis].set_linvel(velocity, true);
    runtime.step(
        &r,
        FrameInput {
            held: keys::BACK,
            ..Default::default()
        },
    );
    for id in 11..=12 {
        assert!(
            runtime
                .lighting
                .light_outputs()
                .iter()
                .find(|o| o.id.0 == id)
                .unwrap()
                .enabled
        );
    }
    for id in 9..=10 {
        assert!(
            !runtime
                .lighting
                .light_outputs()
                .iter()
                .find(|o| o.id.0 == id)
                .unwrap()
                .enabled
        );
    }
    runtime.physics.rapier_mut().bodies[chassis].set_linvel(velocity, true);
    runtime.step(&r, FrameInput::default());
    for id in 11..=12 {
        assert!(
            !runtime
                .lighting
                .light_outputs()
                .iter()
                .find(|o| o.id.0 == id)
                .unwrap()
                .enabled
        );
    }
}

#[test]
#[ignore = "Requires exported Steam assets"]
fn exported_reverse_lights_follow_selected_reverse_gear() {
    use my_summer_slop::{reference::ReferenceManifest, starting_area::GarageRuntime};
    let mut r =
        ReferenceManifest::from_json(&std::fs::read("web/assets/reference/manifest.json").unwrap())
            .unwrap();
    let tilt = (-85.0_f32).to_radians();
    let head = r.player.standing_height * 0.5;
    r.area.player_spawn.translation = [
        37.8,
        1.85 + head - head * tilt.cos(),
        -69.3 - head * tilt.sin(),
    ];
    r.area.player_spawn.rotation = [(tilt / 2.).sin(), 0., 0., (tilt / 2.).cos()];
    let mut runtime = GarageRuntime::new(&r).unwrap();
    runtime.started = true;
    runtime.vehicles[0].engine.ignition = true;
    runtime.step(
        &r,
        FrameInput {
            pressed: keys::USE,
            released: keys::USE,
            ..Default::default()
        },
    );
    assert!(runtime.area.player.driving());
    // Match the source automatic reverse request after its starter and wheel updates.
    let mut selected = false;
    for _ in 0..180 {
        runtime.step(
            &r,
            FrameInput {
                held: keys::BACK,
                ..Default::default()
            },
        );
        if runtime
            .lighting
            .light_outputs()
            .iter()
            .filter(|o| o.id.0 == 9 || o.id.0 == 10)
            .all(|o| o.enabled)
        {
            selected = true;
            break;
        }
    }
    assert!(selected, "source automatic reverse was not selected");
    for id in 11..=12 {
        assert!(
            !runtime
                .lighting
                .light_outputs()
                .iter()
                .find(|o| o.id.0 == id)
                .unwrap()
                .enabled
        );
    }
}

#[test]
fn driving2_uses_car_controls_without_throttle() {
    use my_summer_slop::{starting_area::GarageRuntime, vehicle::VehiclePose};
    let mut game = common::reference();
    let seat = game.seats.iter_mut().find(|s| s.driver).unwrap();
    seat.driver = false;
    seat.driving2 = true;
    let id = seat.id;
    let car = game
        .items
        .iter()
        .find(|i| i.node == game.vehicles[0].root)
        .unwrap()
        .id;
    game.items
        .iter_mut()
        .find(|i| i.id == game.vehicles[0].engine.item)
        .unwrap()
        .attached_to = Some(car);
    let mut run = GarageRuntime::new(&game).unwrap();
    run.started = true;
    assert!(run.area.player.enter(
        id,
        VehiclePose {
            position: [20., 2., 20.],
            rotation: [0., 0., 0., 1.]
        },
        &mut run.physics
    ));
    assert!(!run.area.player.driving());
    assert!(run.area.player.driving2());
    run.step(
        &game,
        FrameInput {
            held: keys::HORN | keys::FORWARD,
            pressed: keys::SPACE,
            ..Default::default()
        },
    );
    let state = run.audio.as_ref().unwrap();
    assert!(
        state
            .signals
            .iter()
            .find(|s| s.id == state.reference.horn.voice)
            .unwrap()
            .playing
    );
    assert!(!run.vehicles[0].engine_output.starter);
    assert!(!run.vehicles[0].handbrake, "driving2 must allow handbrake");
    assert!(
        !run.vehicles[0].engine_output.ignition,
        "driving2 forward must not trigger automatic throttle ignition"
    );
    run.step(
        &game,
        FrameInput {
            held: keys::IGNITION,
            pressed: keys::IGNITION,
            ..Default::default()
        },
    );
    run.step(
        &game,
        FrameInput {
            released: keys::IGNITION,
            ..Default::default()
        },
    );
    assert!(
        run.vehicles[0].engine_output.ignition,
        "driving2 must allow ignition"
    );
}
#[test]
#[ignore = "requires installed source export"]
fn installed_driving2_matches_source_seats() {
    use my_summer_slop::reference::ReferenceManifest;
    let game =
        ReferenceManifest::from_json(&std::fs::read("web/assets/reference/manifest.json").unwrap())
            .unwrap();
    for seat in &game.seats {
        let record: serde_json::Value = serde_json::from_slice(
            &std::fs::read(format!(
                ".cache/reference-library/records/{}/{}.json",
                seat.source.file, seat.source.path_id
            ))
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            seat.driving2,
            record["data"]["driverSeat1"].as_u64().unwrap() != 0
        );
    }
    assert!(game.seats.iter().any(|s| s.driving2 && !s.driver));
}

#[test]
fn automatic_pedal_threshold_and_pause_reset_apply_to_both_cars() {
    use my_summer_slop::starting_area::GarageRuntime;
    let mut reference = common::reference();
    for vehicle in &mut reference.vehicles {
        vehicle.dont_ignite_with_throttle_in_automatic = false;
    }
    for index in 0..reference.vehicles.len() {
        let seat = reference
            .seats
            .iter()
            .find(|s| s.driver && s.vehicle == index)
            .unwrap()
            .id;
        let mut runtime = GarageRuntime::new(&reference).unwrap();
        runtime.started = true;
        let pose = runtime.physics.snapshot(index).pose;
        assert!(runtime.area.player.enter(seat, pose, &mut runtime.physics));
        for sample in 1..=6 {
            runtime.step(
                &reference,
                FrameInput {
                    held: keys::FORWARD | keys::BACK | keys::CLUTCH,
                    ..Default::default()
                },
            );
            assert_eq!(runtime.attempts, if sample <= 3 { 0 } else { 1 });
        }
        runtime.paused = true;
        runtime.step(&reference, FrameInput::default());
        runtime.paused = false;
        for sample in 1..=4 {
            runtime.step(
                &reference,
                FrameInput {
                    held: keys::FORWARD,
                    ..Default::default()
                },
            );
            assert_eq!(runtime.attempts, if sample <= 3 { 1 } else { 2 });
        }
    }
}
