//! The seated start car driver gets out in the garage.
#[test]
#[ignore = "requires the installed source export"]
fn start_car_driver_exits_in_garage() {
    let (reference, mut run) = super::garage_controls::load_garage();
    assert!(super::drive_out::enter(&mut run, &reference));
    let player = &mut run.area.player;
    let mut exits = Vec::new();
    for step in 0..24 {
        let yaw = step as f32 * 15.;
        let head = player.view(&run.physics).head_pose;
        let rotation =
            rapier3d::math::Rotation::from_array(player.body_pose(&run.physics).rotation)
                * rapier3d::math::Rotation::from_rotation_y(-yaw.to_radians());
        let target = rapier3d::math::Vector::from_array(head.position)
            + rotation * -rapier3d::math::Vector::Z;
        player.look_at(target.to_array(), &run.physics);
        if player.can_exit(&run.physics) {
            exits.push(yaw);
        } else if step == 6 || step == 18 {
            eprintln!("yaw {yaw}:\n{}", player.exit_trace(&run.physics));
        }
    }
    assert!(!exits.is_empty(), "no clear exit");
    assert!(player.try_exit(&mut run.physics));
}
