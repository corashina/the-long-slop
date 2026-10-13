use my_summer_slop::reference::view_math::source_culling_view;
use rapier3d::na::{Matrix4, UnitQuaternion, Vector3, Vector4};

#[test]
fn source_culling_inverse_and_positions_share_source_world() {
    let position = Vector3::new(17.0_f32, -3.25, -22.5);
    let world = Matrix4::new_translation(&position)
        * UnitQuaternion::from_euler_angles(0.31, -0.72, 0.16).to_homogeneous();
    let target_view = world.try_inverse().unwrap();
    let columns =
        std::array::from_fn(|column| std::array::from_fn(|row| target_view[(row, column)]));
    let source_columns = source_culling_view(columns);
    let words: Vec<_> = source_columns.into_iter().flatten().collect();
    let source_view = Matrix4::from_column_slice(&words);
    let source_world = source_view.try_inverse().unwrap();
    let reflection = Matrix4::from_diagonal(&Vector4::new(1., 1., -1., 1.));
    assert!((source_view * source_world - Matrix4::identity()).norm() < 0.0001);
    let source_camera = source_world.column(3);
    let expected_camera = reflection * world.column(3);
    assert!((source_camera - expected_camera).norm() < 0.0001);
    assert!((source_camera.z - 22.5).abs() < 0.0001);
    let target_point = Vector4::new(2.5, 1.25, -8., 1.);
    let source_point = reflection * target_point;
    assert!((source_view * source_point - target_view * target_point).norm() < 0.0001);
    // This checks the distinct view-space conversion used by the motion producer.
    let motion_world = (reflection * target_view).try_inverse().unwrap();
    assert!((motion_world.column(3) - world.column(3)).norm() < 0.0001);
    assert!((motion_world.column(3) - source_camera).norm() > 40.);
}
