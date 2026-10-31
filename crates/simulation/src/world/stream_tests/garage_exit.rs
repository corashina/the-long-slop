//! The seated start car driver gets out in the garage.
use super::drive_out::{frame, frame_with, settle};
use crate::simulation::{FrameInput, keys};
use rapier3d::math::{Rotation, Vector};

/// Looks at `yaw` degrees in the seat frame and `pitch` degrees up.
fn look(run: &mut crate::starting_area::GarageRuntime, yaw: f32, pitch: f32) {
    let player = &mut run.area.player;
    let head = player.view(&run.physics).head_pose;
    let rotation = Rotation::from_array(player.body_pose(&run.physics).rotation)
        * Rotation::from_rotation_y(-yaw.to_radians())
        * Rotation::from_rotation_x(pitch.to_radians());
    let target = Vector::from_array(head.position) + rotation * -Vector::Z;
    player.look_at(target.to_array(), &run.physics);
}

#[test]
#[ignore = "requires the installed source export"]
fn start_car_driver_exits_in_garage() {
    let (reference, mut run) = super::garage_controls::load_garage();
    settle(&mut run, &reference, 3.);
    let mut sandbox = crate::sandbox::Sandbox::default();
    crate::sandbox::run(
        &mut run,
        &reference,
        r#"[{"action":"seat","seat":"driver"}]"#,
        &mut sandbox,
    );
    eprintln!("seat: {:?}", sandbox.replies);
    assert!(run.area.player.seated());
    settle(&mut run, &reference, 2.);
    let mut exits = Vec::new();
    for step in 0..24 {
        let yaw = step as f32 * 15.;
        look(&mut run, yaw, 0.);
        frame(&mut run, &reference);
        let player = &run.area.player;
        if player.can_exit(&run.physics) {
            exits.push(yaw);
        } else if step % 6 == 0 {
            eprintln!("yaw {yaw}:\n{}", player.exit_trace(&run.physics));
        }
    }
    eprintln!("exits at yaw {exits:?} with the doors closed");
    // Source CanGetOut casts toward the view on useLayer. A closed door stops the search.
    for pitch in [0., -20., -40.] {
        look(&mut run, 270., pitch);
        frame(&mut run, &reference);
        eprintln!(
            "door press at pitch {pitch}:\n{}",
            run.area.player.exit_trace(&run.physics)
        );
        frame_with(
            &mut run,
            &reference,
            FrameInput {
                held: keys::USE,
                pressed: keys::USE,
                ..FrameInput::default()
            },
        );
        settle(&mut run, &reference, 1.);
        look(&mut run, 270., 0.);
        frame(&mut run, &reference);
        if run.area.player.can_exit(&run.physics) {
            eprintln!("exit clear after the press at pitch {pitch}");
            exits.push(270.);
            break;
        }
    }
    eprintln!("after door: {}", run.area.player.exit_trace(&run.physics));
    sandbox.replies.clear();
    crate::sandbox::run(
        &mut run,
        &reference,
        r#"[{"action":"stand"}]"#,
        &mut sandbox,
    );
    eprintln!("stand: {:?}", sandbox.replies);
    assert!(!exits.is_empty(), "no clear exit");
    assert!(!run.area.player.seated());
}
