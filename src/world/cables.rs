//! Source power cable math: rope aim and pole selection.
use crate::reference::SourceId;
use rapier3d::math::{Rotation, Vector};

/// Source `roadGenScript.CheckCableConnections` range in meters.
pub(crate) const CABLE_RANGE: f64 = 200.;

/// Local pose of one rope node.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NodePose {
    pub node: SourceId,
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
}

/// Source `ropeScript.Connect`: LookAt the target, localScale z = distance / 100.
/// Port forward is -Z and up is +Y. Returns the local rotation and scale z.
pub(crate) fn aim(rope: [f64; 3], parent_rotation: Rotation, target: [f64; 3]) -> ([f32; 4], f32) {
    let delta = [0, 1, 2].map(|i| target[i] - rope[i]);
    let distance = delta.iter().map(|d| d * d).sum::<f64>().sqrt();
    if distance == 0. {
        return (Rotation::IDENTITY.to_array(), 0.);
    }
    let forward = Vector::from_array(delta.map(|d| (d / distance) as f32));
    let right = forward.cross(Vector::Y);
    // Straight up or down has no right axis. Use the shortest arc.
    let look = if right.length_squared() < 1e-12 {
        Rotation::from_rotation_arc(Vector::NEG_Z, forward)
    } else {
        let right = right.normalize();
        Rotation::from_rotation_axes(right, right.cross(forward), -forward)
    };
    let local = (parent_rotation.inverse() * look).normalize();
    (local.to_array(), (distance / 100.) as f32)
}

/// Source `roadGenScript.CheckCableConnections`.
/// Keep the pole with the nearest target 0 under 200 m. Then pick its nearest target.
pub(crate) fn nearest_target<'a>(
    rope0: [f64; 3],
    poles: impl Iterator<Item = (u32, &'a [[f64; 3]])>,
) -> Option<(u32, usize)> {
    let distance = |target: &[f64; 3]| {
        (0..3)
            .map(|i| (target[i] - rope0[i]).powi(2))
            .sum::<f64>()
            .sqrt()
    };
    let mut best: Option<(u32, &[[f64; 3]], f64)> = None;
    for (pole, targets) in poles {
        let Some(first) = targets.first() else {
            continue;
        };
        let d = distance(first);
        if d < best.map_or(CABLE_RANGE, |(_, _, best)| best) {
            best = Some((pole, targets, d));
        }
    }
    let (pole, targets, _) = best?;
    let mut index = 0;
    for (i, target) in targets.iter().enumerate().skip(1) {
        if distance(target) < distance(&targets[index]) {
            index = i;
        }
    }
    Some((pole, index))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world_forward(parent: Rotation, local: [f32; 4]) -> Vector {
        (parent * Rotation::from_array(local) * Vector::NEG_Z).normalize()
    }

    #[test]
    fn aim_points_forward_at_target() {
        let (rotation, scale) = aim([0.; 3], Rotation::IDENTITY, [3., 4., -12.]);
        assert!((scale - 0.13).abs() < 1e-6);
        let expected = Vector::new(3., 4., -12.) / 13.;
        assert!((world_forward(Rotation::IDENTITY, rotation) - expected).length() < 1e-5);
    }

    #[test]
    fn aim_removes_parent_rotation() {
        let parent = Rotation::from_rotation_y(std::f32::consts::FRAC_PI_2);
        let (rotation, _) = aim([1., 2., 3.], parent, [4., 6., -9.]);
        let expected = Vector::new(3., 4., -12.) / 13.;
        assert!((world_forward(parent, rotation) - expected).length() < 1e-5);
    }

    #[test]
    fn nearest_target_follows_source_rule() {
        let far = [[201., 0., 0.]];
        let near = [[199., 0., 0.]];
        assert_eq!(nearest_target([0.; 3], [(1, &far[..])].into_iter()), None);
        assert_eq!(
            nearest_target([0.; 3], [(1, &near[..])].into_iter()),
            Some((1, 0))
        );
        let first = [[50., 0., 0.]];
        let second = [[20., 0., 0.], [10., 0., 0.], [15., 0., 0.]];
        let poles = [(7, &first[..]), (8, &second[..])];
        assert_eq!(nearest_target([0.; 3], poles.into_iter()), Some((8, 1)));
    }
}
