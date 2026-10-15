//! Source pole placement on the current procedural road.
use super::road_sample;
use crate::reference::RoadLampReference;
use rapier3d::math::{Rotation, Vector};
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoadLampId(pub i64);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoadLampPlacement {
    pub id: RoadLampId,
    pub absolute_point: [f64; 3],
    pub rotation: [f32; 4],
}

/// Each source index belongs to one half-open interval. The source starts at zero.
pub fn road_lamp_range(start_m: f64, end_m: f64, spacing_m: f64) -> Range<i64> {
    assert!(spacing_m.is_finite() && spacing_m > 0.);
    let start = (start_m / spacing_m).ceil().max(0.) as i64;
    let end = (end_m / spacing_m).ceil().max(0.) as i64;
    start..end.max(start)
}

pub fn road_lamp_placement(
    seed: u64,
    id: RoadLampId,
    reference: &RoadLampReference,
) -> RoadLampPlacement {
    let road = road_sample(seed, id.0 as f64 * reference.spacing_meters as f64);
    let yaw = (-road.tangent[0]).atan2(-road.tangent[2]);
    let pitch = road.tangent[1].asin();
    let rotation = Rotation::from_rotation_y(yaw) * Rotation::from_rotation_x(pitch);
    let offset = rotation * Vector::from_array(reference.road_pose.translation);
    let incoming = rotation * Rotation::from_array(reference.road_pose.rotation).normalize();
    RoadLampPlacement {
        id,
        absolute_point: std::array::from_fn(|axis| road.center[axis] + offset[axis] as f64),
        rotation: flatten_then_tilt(incoming, source_angles(seed, id, reference)).to_array(),
    }
}

fn flatten_then_tilt(incoming: Rotation, angles: [i32; 3]) -> Rotation {
    let forward = incoming * Vector::NEG_Z;
    let yaw = (-forward.x).atan2(-forward.z);
    let [x, y, z] = angles.map(|angle| (angle as f32).to_radians());
    // Converted Unity Euler applies Z, X, then Y. FStart runs before this local tilt.
    Rotation::from_rotation_y(yaw)
        * Rotation::from_rotation_y(-y)
        * Rotation::from_rotation_x(-x)
        * Rotation::from_rotation_z(z)
}

fn source_angles(seed: u64, id: RoadLampId, reference: &RoadLampReference) -> [i32; 3] {
    // Source seed and segment index are signed integers. Preserve their low bits and wrapping sum.
    let mut random = SourceRandom::new((seed as i32).wrapping_add(id.0 as i32));
    let [chance1, chance2] = reference.rotation_chances;
    let chance = random.sample() as f32 * (chance1 + chance2);
    let angles = if chance < chance1 {
        reference.rotation_angles[0]
    } else if chance - chance1 < chance2 {
        reference.rotation_angles[1]
    } else {
        return [0; 3];
    };
    angles.map(|limit| (random.sample() * (limit * 2) as f64) as i32 - limit)
}

/// Installed mscorlib seeded subtractive RNG. These ranges use its small-range Next path.
struct SourceRandom {
    values: [i32; 56],
    next: usize,
    paired: usize,
}
impl SourceRandom {
    fn new(seed: i32) -> Self {
        let mut values = [0_i32; 56];
        let magnitude = if seed == i32::MIN {
            i32::MAX
        } else {
            seed.abs()
        };
        let mut last = 161803398_i32.wrapping_sub(magnitude);
        values[55] = last;
        let mut value = 1_i32;
        let mut index = 0;
        for _ in 1..55 {
            index = (index + 21) % 55;
            values[index] = value;
            value = last.wrapping_sub(value);
            if value < 0 {
                value = value.wrapping_add(i32::MAX);
            }
            last = values[index];
        }
        for _ in 1..5 {
            for index in 1..56 {
                values[index] = values[index].wrapping_sub(values[1 + (index + 30) % 55]);
                if values[index] < 0 {
                    values[index] = values[index].wrapping_add(i32::MAX);
                }
            }
        }
        Self {
            values,
            next: 0,
            paired: 21,
        }
    }
    fn sample(&mut self) -> f64 {
        self.next = if self.next + 1 == 56 {
            1
        } else {
            self.next + 1
        };
        self.paired = if self.paired + 1 == 56 {
            1
        } else {
            self.paired + 1
        };
        let mut value = self.values[self.next].wrapping_sub(self.values[self.paired]);
        if value == i32::MAX {
            value -= 1;
        }
        if value < 0 {
            value = value.wrapping_add(i32::MAX);
        }
        self.values[self.next] = value;
        value as f64 * 4.656612875245797E-10
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn flatten_precedes_local_tilt() {
        let incoming = Rotation::from_xyzw(-0.16872216, -0.23081309, 0.13302687, 0.94897944);
        let actual = flatten_then_tilt(incoming, [15, 10, 15]).to_array();
        let expected = [-0.16586584, -0.3201835, 0.07734451, 0.92951006];
        for (a, b) in actual.into_iter().zip(expected) {
            assert!((a - b).abs() < 1e-6);
        }
        assert!(
            (flatten_then_tilt(incoming, [15, 10, 15]) * Vector::NEG_Z)
                .y
                .abs()
                > 0.25
        );
    }
}
