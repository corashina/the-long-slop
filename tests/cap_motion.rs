#[path = "../src/presentation/cap_motion.rs"]
mod cap_motion;

use bevy::prelude::Transform;
use cap_motion::cap_motion;

#[test]
fn f15_cap_moves_over_source_do_move_frames() {
    let closed = Transform::from_xyz(0., 0., 0.);
    let open = Transform::from_xyz(0., 1., 0.);
    let mut motion = None;
    // Start frame: DoMove lerps with capLerp = 0.
    let mut pose = cap_motion(closed, open, &mut motion, 0.05);
    assert_eq!((pose, motion), (closed, Some(0.)));
    pose = cap_motion(pose, open, &mut motion, 0.05);
    assert!((pose.translation.y - 0.5).abs() < 1e-6);
    pose = cap_motion(pose, open, &mut motion, 0.05);
    assert_eq!((pose, motion), (open, None));
    assert_eq!(cap_motion(open, open, &mut motion, 0.05), open);
    assert_eq!(motion, None);
}
