mod common;
use my_summer_slop::{
    reference::DrivingAssists,
    simulation::{
        drivetrain::{DrivetrainState, transmitted_torques},
        engine::EngineOutput,
        input::DrivingControls,
        keys,
    },
    starting_area::GarageRuntime,
};
const MANUAL: DrivingAssists = DrivingAssists {
    automatic_clutch: false,
    automatic_gears: false,
};
#[test]
fn manual_shift_requires_source_clutch_threshold() {
    let r = common::reference();
    for v in &r.vehicles {
        let mut d = DrivetrainState::new(v);
        assert_eq!(d.gear(), v.driving.initial_gear);
        for (clutch, want) in [(0.5, 1), (0.5001, 2)] {
            d.step(
                DrivingControls {
                    clutch,
                    ..Default::default()
                },
                keys::GEAR_1,
                MANUAL,
                Some(EngineOutput::default()),
                0.,
                0.,
                v,
                &r.driving,
                true,
            );
            assert_eq!(d.gear(), want);
        }
        d.step(
            DrivingControls {
                clutch: 1.,
                ..Default::default()
            },
            keys::GEAR_6,
            MANUAL,
            Some(EngineOutput::default()),
            0.,
            0.,
            v,
            &r.driving,
            true,
        );
        assert_eq!(d.gear(), 2);
    }
}
#[test]
fn reverse_neutral_and_clutch_route_signed_torque_once() {
    let r = common::reference();
    for v in &r.vehicles {
        let engine = EngineOutput {
            torque_nm: 50.,
            brake_torque_nm: 10.,
            ..Default::default()
        };
        let mut d = DrivetrainState::new(v);
        let reverse = d.step(
            DrivingControls {
                clutch: 0.75,
                ..Default::default()
            },
            keys::REVERSE,
            MANUAL,
            Some(engine),
            100.,
            0.,
            v,
            &r.driving,
            true,
        );
        assert_eq!(reverse.engagement, 0.25);
        assert_eq!(
            reverse.wheel_rpm,
            100. * v.final_drive_ratio * v.gear_ratios[0]
        );
        let torque = transmitted_torques(engine, v, reverse.gear);
        assert_eq!(
            torque.motor_nm,
            50. * v.gear_ratios[0] * v.final_drive_ratio
        );
        assert_eq!(
            torque.engine_brake_nm,
            10. * v.gear_ratios[0].abs() * v.final_drive_ratio
        );
        let neutral = d.step(
            DrivingControls {
                clutch: 1.,
                ..Default::default()
            },
            keys::NEUTRAL,
            MANUAL,
            Some(engine),
            100.,
            0.,
            v,
            &r.driving,
            true,
        );
        assert_eq!(neutral.engagement, 0.);
        assert_eq!(transmitted_torques(engine, v, neutral.gear).motor_nm, 0.);
    }
}
#[test]
fn assists_keep_simultaneous_pedals_and_source_direction_rules() {
    let r = common::reference();
    assert!(
        r.driving.default_assists.automatic_clutch && r.driving.default_assists.automatic_gears
    );
    for v in &r.vehicles {
        let mut d = DrivetrainState::new(v);
        let engine = EngineOutput {
            rpm: v.engine.auto_shift_clutch_rpm,
            running: true,
            ..Default::default()
        };
        let out = d.step(
            DrivingControls {
                throttle: 0.5,
                brake: 0.75,
                ..Default::default()
            },
            keys::REVERSE,
            r.driving.default_assists,
            Some(engine),
            100.,
            0.,
            v,
            &r.driving,
            true,
        );
        assert!(v.gear_ratios[out.gear] > 0.);
        assert_eq!((out.throttle, out.brake, out.engagement), (0.5, 0.75, 0.25));
        let out = d.step(
            DrivingControls {
                brake: 0.5,
                ..Default::default()
            },
            0,
            r.driving.default_assists,
            Some(engine),
            100.,
            0.,
            v,
            &r.driving,
            true,
        );
        assert!(v.gear_ratios[out.gear] < 0.);
        assert_eq!((out.throttle, out.brake), (0.5, 0.));
        let out = d.step(
            DrivingControls {
                brake: 0.5,
                ..Default::default()
            },
            0,
            MANUAL,
            Some(engine),
            -100.,
            0.,
            v,
            &r.driving,
            true,
        );
        assert_eq!((out.throttle, out.brake), (0., 0.5));
    }
}
#[test]
fn release_and_seat_exit_keep_selected_gear() {
    let r = common::reference();
    let mut game = GarageRuntime::new(&r).unwrap();
    game.started = true;
    game.paused = true;
    game.set_driving_assists(MANUAL).unwrap();
    for (i, v) in r.vehicles.iter().enumerate() {
        game.vehicles[i].drivetrain.step(
            DrivingControls {
                clutch: 1.,
                ..Default::default()
            },
            keys::GEAR_2,
            MANUAL,
            Some(EngineOutput::default()),
            0.,
            0.,
            v,
            &r.driving,
            true,
        );
    }
    for index in 0..r.vehicles.len() {
        let seat = r
            .seats
            .iter()
            .find(|s| s.driver && s.vehicle == index)
            .unwrap();
        let pose = my_summer_slop::vehicle::VehiclePose {
            position: [20., 2., 20.],
            rotation: [0., 0., 0., 1.],
        };
        assert!(game.area.player.enter(seat.id, pose, &mut game.physics));
        game.paused = false;
        game.step(&r, Default::default());
        assert_eq!(game.vehicles[index].drivetrain.gear(), 3);
        game.area.player.step(
            my_summer_slop::simulation::FrameInput {
                mouse: [90., 0.],
                ..Default::default()
            },
            &mut game.physics,
            my_summer_slop::simulation::FIXED_DT,
        );
        assert!(game.area.player.try_exit(
            &mut game.physics,
            &my_summer_slop::player::ExitReference::from_reference(&r.player, seat)
        ));
    }
    game.clear_input();
    game.paused = false;
    for _ in 0..10 {
        game.step(&r, Default::default());
    }
    assert!(game.vehicles.iter().all(|v| v.drivetrain.gear() == 3));
    game.paused = true;
    game.set_driving_assists(r.driving.default_assists).unwrap();
    game.paused = false;
    game.step(&r, Default::default());
    assert!(game.vehicles.iter().all(|v| v.drivetrain.gear() == 3));
}

#[test]
fn source_assist_thresholds_force_manual_and_unoccupied_rules() {
    let mut r = common::reference();
    for vehicle in &mut r.vehicles {
        vehicle.dont_ignite_with_throttle_in_automatic = false;
        let mut d = DrivetrainState::new(vehicle);
        for (throttle, ignite) in [(0.25, false), (0.2501, true)] {
            let out = d.step(
                DrivingControls {
                    throttle,
                    ..Default::default()
                },
                0,
                r.driving.default_assists,
                Some(EngineOutput::default()),
                100.,
                0.,
                vehicle,
                &r.driving,
                true,
            );
            assert_eq!(out.ignition_request, ignite);
            assert_eq!(out.engagement, 0.);
        }
        let auto_clutch = DrivingAssists {
            automatic_clutch: true,
            automatic_gears: false,
        };
        let out = d.step(
            DrivingControls::default(),
            keys::GEAR_2,
            auto_clutch,
            Some(EngineOutput::default()),
            -100.,
            0.,
            vehicle,
            &r.driving,
            true,
        );
        assert_eq!(out.gear, 3);
        assert_eq!(out.engagement, 1.);
        assert!(out.wheel_rpm < 0.);
        vehicle.driving.force_automatic = true;
        let out = d.step(
            DrivingControls {
                clutch: 1.,
                throttle: 1.,
                ..Default::default()
            },
            keys::REVERSE,
            r.driving.default_assists,
            Some(EngineOutput::default()),
            100.,
            0.,
            vehicle,
            &r.driving,
            false,
        );
        assert_eq!(out.gear, 3);
        assert_eq!(out.engagement, 1.);
        assert_eq!((out.throttle, out.brake), (0., 0.));
        r.driving.force_manual = true;
        let out = d.step(
            DrivingControls::default(),
            keys::REVERSE,
            auto_clutch,
            Some(EngineOutput::default()),
            100.,
            0.,
            vehicle,
            &r.driving,
            true,
        );
        assert_eq!(out.gear, 3);
        let out = d.step(
            DrivingControls {
                clutch: 0.5001,
                ..Default::default()
            },
            keys::REVERSE,
            auto_clutch,
            Some(EngineOutput::default()),
            100.,
            0.,
            vehicle,
            &r.driving,
            true,
        );
        assert_eq!(out.gear, 0);
        r.driving.force_manual = false;
    }
}
#[test]
fn paused_assist_changes_normalize_dependency_and_keep_gears() {
    let r = common::reference();
    let mut g = GarageRuntime::new(&r).unwrap();
    g.started = true;
    assert!(g.set_driving_assists(MANUAL).is_err());
    g.paused = true;
    g.set_driving_assists(DrivingAssists {
        automatic_clutch: false,
        automatic_gears: true,
    })
    .unwrap();
    assert_eq!(g.driving_assists, MANUAL);
    assert!(g.vehicles.iter().all(|v| v.drivetrain.gear() == 1));
}

#[test]
fn absent_engine_skips_automatic_rules_but_keeps_manual_selection() {
    let mut r = common::reference();
    for v in &mut r.vehicles {
        v.dont_ignite_with_throttle_in_automatic = false;
        let mut d = DrivetrainState::new(v);
        let manual = DrivingAssists {
            automatic_clutch: true,
            automatic_gears: false,
        };
        let out = d.step(
            DrivingControls::default(),
            keys::GEAR_2,
            manual,
            None,
            100.,
            0.,
            v,
            &r.driving,
            true,
        );
        assert_eq!(out.gear, 3, "manual selection without engine");
        for (throttle, brake) in [(1., 0.), (0., 1.), (0.5, 0.75), (0., 0.)] {
            let out = d.step(
                DrivingControls {
                    throttle,
                    brake,
                    clutch: 0.2,
                    ..Default::default()
                },
                0,
                r.driving.default_assists,
                None,
                100.,
                0.,
                v,
                &r.driving,
                true,
            );
            assert_eq!(out.gear, 3, "absent engine must retain selected gear");
            assert_eq!(
                (out.throttle, out.brake, out.engagement),
                (throttle, brake, 0.8)
            );
            assert!(!out.ignition_request);
        }
    }
}
