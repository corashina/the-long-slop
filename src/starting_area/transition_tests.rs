use super::*;

fn game(index: usize) -> (ReferenceManifest, GarageRuntime) {
    let reference =
        ReferenceManifest::from_json(include_bytes!("../../tests/common/reference.json")).unwrap();
    let mut game = GarageRuntime::new(&reference).unwrap();
    game.started = true;
    game.driving_assists = crate::reference::DrivingAssists {
        automatic_clutch: true,
        automatic_gears: false,
    };
    let seat = reference
        .seats
        .iter()
        .find(|s| s.driver && s.vehicle == index)
        .unwrap();
    // Isolate input routing from garage collision and door access.
    let target = game
        .physics
        .rapier()
        .colliders
        .iter()
        .find(|(_, c)| c.user_data as u32 == seat.item.0 + 1)
        .unwrap()
        .0;
    let center = game.physics.rapier().colliders[target]
        .compute_aabb()
        .center()
        .to_array();
    let old = game.area.player.body_handle();
    game.physics.rapier_mut().remove_body(old);
    for (handle, collider) in game.physics.rapier_mut().colliders.iter_mut() {
        collider.set_enabled(handle == target);
    }
    game.area.player = crate::player::PlayerState::new(
        &reference.player,
        VehiclePose {
            position: [center[0], center[1], center[2] + 1.],
            rotation: [0., 0., 0., 1.],
        },
        &mut game.physics,
    );
    game.area.player.configure_seats(&reference.seats);
    game.area
        .interaction
        .bind_player_body(game.area.player.body_handle());
    let view = game.area.player.view_pose(&game.physics);
    let delta = std::array::from_fn::<_, 3, _>(|i| center[i] - view.position[i]);
    game.area.player.step(
        FrameInput {
            mouse: [
                delta[0].atan2(-delta[2]).to_degrees(),
                (-delta[1])
                    .atan2((delta[0] * delta[0] + delta[2] * delta[2]).sqrt())
                    .to_degrees(),
            ],
            ..Default::default()
        },
        &mut game.physics,
        FIXED_DT,
    );
    game.physics.refresh_queries();
    (reference, game)
}

#[test]
fn seat_entry_clears_pending_shift_and_pedals_for_both_cars() {
    for index in 0..2 {
        let (reference, mut game) = game(index);
        let seat = reference
            .seats
            .iter()
            .find(|s| s.driver && s.vehicle == index)
            .unwrap();
        let pedals = keys::FORWARD | keys::BACK | keys::CLUTCH;
        game.step(
            &reference,
            FrameInput {
                held: pedals,
                pressed: keys::GEAR_2 | keys::IGNITION,
                released: keys::USE,
                ..Default::default()
            },
        );
        assert_eq!(
            game.area.player.seat(),
            Some(seat.id),
            "actual seat entry for car {index}"
        );
        assert_eq!(
            game.vehicles[index].drivetrain.gear(),
            reference.vehicles[index].driving.initial_gear,
            "pending entry shift for car {index}"
        );
        assert_eq!(game.attempts, 0, "entry ignition edge");
        let controls = game.driving_input.step(Some(seat.id), pedals);
        let increment = reference.driving.pedal_rate * FIXED_DT;
        assert_eq!(
            [controls.throttle, controls.brake, controls.clutch],
            [increment; 3],
            "entry pedals start from zero"
        );
    }
}

#[test]
fn seat_exit_cancels_short_ignition_hold_without_toggle_for_both_cars() {
    for index in 0..2 {
        let (reference, mut game) = game(index);
        game.step(
            &reference,
            FrameInput {
                released: keys::USE,
                ..Default::default()
            },
        );
        assert!(game.area.player.driving());
        game.step(
            &reference,
            FrameInput {
                pressed: keys::GEAR_2 | keys::IGNITION,
                held: keys::IGNITION | keys::FORWARD | keys::BACK | keys::CLUTCH,
                ..Default::default()
            },
        );
        assert_eq!(game.vehicles[index].drivetrain.gear(), 3);
        let ignition = game.vehicles[index].engine_output.ignition;
        assert!(game.vehicles[index].ignition_held);
        game.area.player.step(
            FrameInput {
                mouse: [720., 0.],
                ..Default::default()
            },
            &mut game.physics,
            FIXED_DT,
        );
        for (_, collider) in game.physics.rapier_mut().colliders.iter_mut() {
            collider.set_enabled(false);
        }
        game.physics.refresh_queries();
        game.step(
            &reference,
            FrameInput {
                pressed: keys::USE | keys::GEAR_1,
                held: keys::IGNITION | keys::FORWARD | keys::BACK | keys::CLUTCH,
                ..Default::default()
            },
        );
        assert_eq!(
            game.area.player.seat(),
            None,
            "actual seat exit for car {index}"
        );
        assert_eq!(
            game.vehicles[index].engine_output.ignition, ignition,
            "exit must not toggle ignition for car {index}"
        );
        assert!(!game.vehicles[index].ignition_held);
        assert!(!game.vehicles[index].engine_output.starter);
        assert_eq!(game.vehicles[index].drivetrain.gear(), 3);
        assert_eq!(game.driving_input.step(None, 0), Default::default());
        game.step(
            &reference,
            FrameInput {
                released: keys::IGNITION,
                ..Default::default()
            },
        );
        assert_eq!(game.vehicles[index].engine_output.ignition, ignition);
        assert_eq!(game.vehicles[index].drivetrain.gear(), 3);
    }
}

#[test]
fn removed_engine_retains_automatic_gear_for_both_cars() {
    for index in 0..2 {
        let (reference, mut game) = game(index);
        game.step(
            &reference,
            FrameInput {
                released: keys::USE,
                ..Default::default()
            },
        );
        assert!(game.area.player.driving());
        game.step(
            &reference,
            FrameInput {
                pressed: keys::GEAR_2,
                ..Default::default()
            },
        );
        let engine = reference.vehicles[index].engine.item;
        game.area
            .interaction
            .parts
            .remove(engine, &mut game.area.items, &mut game.physics)
            .unwrap();
        assert!(!game.area.items.mounted(engine));
        game.driving_assists = reference.driving.default_assists;
        for held in [keys::FORWARD, keys::BACK, 0] {
            game.step(
                &reference,
                FrameInput {
                    held,
                    ..Default::default()
                },
            );
            assert_eq!(game.vehicles[index].drivetrain.gear(), 3);
            assert_eq!(game.attempts, 0);
        }
    }
}
