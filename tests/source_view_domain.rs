use rapier3d::na::{Matrix4, UnitQuaternion, Vector3, Vector4};
use the_long_slop::reference::view_math::source_culling_view;

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
}

#[test]
fn candidate_views_use_private_pose_and_origin() {
    use the_long_slop::{
        reference::{shadow_candidates::DirectionalHull, shadow_planes::implicit_cull_planes},
        world::{RequiredView, WorldBounds},
    };
    let view = |world: Matrix4<f32>, origin| {
        let runtime_view = world.try_inverse().unwrap();
        let source = source_culling_view(std::array::from_fn(|c| {
            std::array::from_fn(|r| runtime_view[(r, c)])
        }));
        let source = Matrix4::from_column_slice(&source.into_iter().flatten().collect::<Vec<_>>());
        let projection =
            rapier3d::na::Perspective3::new(1., 60_f32.to_radians(), 0.1, 100.).to_homogeneous();
        let clip = projection * source;
        let inverse = source.try_inverse().unwrap();
        let planes = implicit_cull_planes(
            clip.as_slice().try_into().unwrap(),
            inverse.as_slice().try_into().unwrap(),
            0.1,
            100.,
            30.,
            0.,
        );
        RequiredView {
            camera: planes.camera,
            shadow: DirectionalHull::new(planes.shadow, [1., 0., 0.]).unwrap(),
            mask: 1,
            light_mask: 0,
            origin,
        }
    };
    let live_pose = Matrix4::new_translation(&Vector3::new(0., 1., 0.));
    let candidate_pose = Matrix4::new_translation(&Vector3::new(20., 1., 4.))
        * UnitQuaternion::from_euler_angles(0., std::f32::consts::FRAC_PI_2, 0.).to_homogeneous();
    let live = view(live_pose, [0.; 3]);
    let candidate = view(candidate_pose, [1000., 0., 3000.]);
    let bounds = |x, z| WorldBounds {
        min: [x - 0.1, 0.9, z - 0.1],
        max: [x + 0.1, 1.1, z + 0.1],
    };
    let live_id = bounds(0., -10.);
    let candidate_id = bounds(1010., 3004.);
    assert!(live.includes(live_id, 1));
    assert!(candidate.includes(candidate_id, 1));
    assert!(!live.includes(candidate_id, 1));
    assert!(!candidate.includes(live_id, 1));
    assert_eq!(live.camera, view(live_pose, [0.; 3]).camera);
}
