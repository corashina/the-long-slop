use the_long_slop::{
    reference::ReferenceManifest,
    simulation::{DriveInput, FIXED_DT},
    starting_area::GarageRuntime,
    vehicle::{DriveTorques, PhysicsWorld, VehiclePose},
};

fn drive(physics: &mut PhysicsWorld, seconds: f32, input: DriveInput, motor_nm: f32) {
    for _ in 0..(seconds / FIXED_DT) as usize {
        physics.apply_vehicle_forces(
            0,
            input,
            DriveTorques {
                motor_nm,
                engine_brake_nm: 0.0,
            },
        );
        physics.step_physics();
    }
}

#[test]
#[ignore = "requires the installed source export"]
fn installed_lada_launches_and_brakes() {
    let bytes = std::fs::read("web/assets/reference/manifest.json").unwrap();
    let reference = ReferenceManifest::from_runtime_json(&bytes).unwrap();
    let mut run =
        GarageRuntime::new(&reference, the_long_slop::world::GameMode::default()).unwrap();
    let physics = &mut run.physics;
    // Flat ground away from the garage.
    let start = physics.snapshot(0).pose.position;
    let flat = [start[0] + 1000.0, start[1], start[2]];
    physics.insert_box([flat[0], flat[1] - 0.5, flat[2]], [300.0, 0.5, 300.0]);
    physics.reset(
        0,
        VehiclePose {
            position: [flat[0], flat[1] + 1.0, flat[2]],
            rotation: [0.0, 0.0, 0.0, 1.0],
        },
    );
    drive(physics, 2.0, DriveInput::default(), 0.0);
    assert!(physics.snapshot(0).wheels_contact.iter().all(|c| *c));
    // First gear near peak engine torque.
    let throttle = DriveInput {
        throttle: 1.0,
        ..Default::default()
    };
    drive(physics, 3.0, throttle, 1600.0);
    let speed = physics.snapshot(0).speed_mps;
    assert!(speed > 4.0, "launch speed {speed}");
    assert!(physics.driven_wheel_rpm(0) > 0.0);
    let brake = DriveInput {
        brake: 1.0,
        ..Default::default()
    };
    drive(physics, 4.0, brake, 0.0);
    assert!(physics.snapshot(0).speed_mps.abs() < 0.5);
    // A parked car holds against a walking player's push.
    let parked = DriveInput {
        handbrake: 1.0,
        ..Default::default()
    };
    drive(physics, 2.0, parked, 0.0);
    let start = physics.snapshot(0).pose.position;
    for _ in 0..(2.0 / FIXED_DT) as usize {
        physics.edit_body(physics.chassis_handle(0), |body| {
            let point = body.translation();
            body.apply_impulse_at_point(rapier3d::math::Vector::X * 60.0, point, true);
        });
        drive(physics, FIXED_DT, parked, 0.0);
    }
    let end = physics.snapshot(0).pose.position;
    let moved = ((end[0] - start[0]).powi(2) + (end[2] - start[2]).powi(2)).sqrt();
    assert!(moved < 0.05, "parked car moved {moved}");
}
