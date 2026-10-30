//! The seated start car driver gets out in the garage.
use super::drive_out::{frame, settle};
use rapier3d::math::{Rotation, Vector};

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
        let player = &mut run.area.player;
        let head = player.view(&run.physics).head_pose;
        let rotation = Rotation::from_array(player.body_pose(&run.physics).rotation)
            * Rotation::from_rotation_y(-yaw.to_radians());
        let target = Vector::from_array(head.position) + rotation * -Vector::Z;
        player.look_at(target.to_array(), &run.physics);
        frame(&mut run, &reference);
        let player = &run.area.player;
        if player.can_exit(&run.physics) {
            exits.push(yaw);
        } else if step % 6 == 0 {
            eprintln!("yaw {yaw}:\n{}", player.exit_trace(&run.physics));
        }
    }
    eprintln!("exits at yaw {exits:?}");
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
