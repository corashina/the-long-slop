//! Source clip sampling into converted parent-local transform parts.
use crate::reference::{MotionClip, MotionTrack};
use crate::simulation::dials::unity_euler;
use rapier3d::math::{Rotation, Vector};

/// One sampled transform part of a node, in converted parent-local space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LocalPart {
    Position(Vector),
    Rotation(Rotation),
    Scale(Vector),
}

/// Clip time for `elapsed` seconds of a looping clip.
pub fn loop_time(clip: &MotionClip, elapsed: f32) -> f32 {
    clip.start_time + elapsed.rem_euclid(clip.stop_time - clip.start_time)
}

/// Samples each track at clip `time` and gives its node and converted part.
pub fn sample_clip(clip: &MotionClip, time: f32, mut out: impl FnMut(u32, LocalPart)) {
    for track in &clip.tracks {
        let [x, y, z, w] = sample_track(track, time);
        let part = match track.attribute {
            // Source parent-local values: z negates.
            1 => LocalPart::Position(Vector::new(x, y, -z)),
            2 => LocalPart::Rotation(Rotation::from_xyzw(-x, -y, z, w).normalize()),
            3 => LocalPart::Scale(Vector::new(x, y, z)),
            _ => {
                let [x, y, z, w] = unity_euler([x, y, z]);
                LocalPart::Rotation(Rotation::from_xyzw(x, y, z, w))
            }
        };
        out(track.node, part);
    }
}

/// Unity streamed cubic keys hold polynomial coefficients from each key time.
/// Components past the track width stay 0.
pub(crate) fn sample_track(track: &MotionTrack, time: f32) -> [f32; 4] {
    let times = &track.times;
    let i = times.partition_point(|t| *t <= time).saturating_sub(1);
    let mut out = [0.; 4];
    let values = &track.values[i];
    out[..values.len()].copy_from_slice(values);
    if time <= times[0] || i == times.len() - 1 {
        return out;
    }
    let Some(coefficients) = track.coefficients.get(i) else {
        return out;
    };
    let elapsed = time - times[i];
    for (c, [a, b, d]) in coefficients.iter().enumerate() {
        if !track.out_tangents[i][c].is_infinite() {
            out[c] = ((a * elapsed + b) * elapsed + d) * elapsed + values[c];
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reference::{MotionReceipt, SourceId};

    fn track(attribute: u32, interpolation: &str, values: Vec<Vec<f32>>) -> MotionTrack {
        let n = values.len();
        let width = values[0].len();
        MotionTrack {
            node: 5,
            attribute,
            path_hash: 0,
            custom_type: 0,
            curve_offset: 0,
            storage: String::new(),
            interpolation: interpolation.into(),
            times: (0..n).map(|i| i as f32 * 0.5).collect(),
            values,
            in_tangents: vec![vec![0.; width]; n],
            out_tangents: vec![vec![0.; width]; n],
            coefficients: vec![vec![[1., 1., 1.]; width]; n - 1],
        }
    }
    fn clip(tracks: Vec<MotionTrack>) -> MotionClip {
        MotionClip {
            source: SourceId {
                file: "test".into(),
                path_id: 1,
            },
            source_tracks: MotionReceipt {
                bytes: 0,
                file: String::new(),
                sha256: String::new(),
            },
            name: "clip".into(),
            start_time: 0.25,
            stop_time: 1.25,
            sample_rate: 60.,
            r#loop: true,
            cycle_offset: 0.,
            tracks,
            events: Vec::new(),
        }
    }
    fn parts(c: &MotionClip, time: f32) -> Vec<(u32, LocalPart)> {
        let mut parts = Vec::new();
        sample_clip(c, time, |node, part| parts.push((node, part)));
        parts
    }

    #[test]
    fn constant_scale_track_gives_its_value() {
        let mut t = track(3, "CONSTANT", vec![vec![2., 3., 4.]]);
        t.coefficients.clear();
        assert_eq!(
            parts(&clip(vec![t]), 0.7),
            [(5, LocalPart::Scale(Vector::new(2., 3., 4.)))]
        );
    }

    #[test]
    fn cubic_quaternion_track_gives_the_converted_normalized_key() {
        let t = track(
            2,
            "CUBIC",
            vec![
                vec![0., 0., 0., 2.],
                vec![1., 2., 3., 4.],
                vec![0., 0., 0., 1.],
            ],
        );
        let [(5, LocalPart::Rotation(r))] = parts(&clip(vec![t]), 0.5)[..] else {
            panic!("one rotation part");
        };
        let expected = Rotation::from_xyzw(-1., -2., 3., 4.).normalize();
        assert!(r.abs_diff_eq(expected, 1e-6));
    }

    #[test]
    fn loop_time_wraps_into_the_clip() {
        let c = clip(Vec::new());
        assert!((loop_time(&c, 0.5) - 0.75).abs() < 1e-6);
        assert!((loop_time(&c, 2.5) - 0.75).abs() < 1e-6);
        assert!((loop_time(&c, -0.25) - 1.).abs() < 1e-6);
    }
}
