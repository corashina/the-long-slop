mod common;
use my_summer_slop::{
    reference::DrivingAssists,
    simulation::{
        drivetrain::{DrivetrainState, transmitted_torques},
        engine::EngineOutput,
        input::DrivingControls,
        keys,
    },
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
