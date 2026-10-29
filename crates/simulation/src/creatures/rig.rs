//! The rabbit Animator: the default 1D blend tree, sampled into node frames under the prefab root.
//! The simulation and the renderer share this sampler.
use crate::animation::clip::{LocalPart, sample_clip};
use crate::player::body::ik::Frame;
use crate::reference::{BlendReference, NodeId, NodeRecord, ReferenceError};
use rapier3d::math::{Rotation, Vector};
use std::collections::HashMap;

pub struct Rig {
    /// Asset nodes, parents first. Index 0 is the prefab root.
    nodes: Vec<NodeId>,
    index: HashMap<NodeId, usize>,
    parent: Vec<Option<usize>>,
    /// Prefab parent-local frames.
    rest: Vec<Frame>,
    /// Clip lengths in seconds.
    lengths: Vec<f32>,
}

/// Sampled frames. `local` is parent-local; `model` is under the prefab root.
pub struct RigPose {
    pub local: Vec<Frame>,
    pub model: Vec<Frame>,
    clip: Vec<Frame>,
    weights: Vec<f32>,
}

impl Rig {
    pub fn new(
        nodes: &[NodeRecord],
        root: NodeId,
        blend: &BlendReference,
    ) -> Result<Self, ReferenceError> {
        let asset = nodes
            .iter()
            .find(|n| n.id == root)
            .ok_or_else(|| ReferenceError("Missing rabbit root".into()))?
            .asset;
        let records: HashMap<_, _> = nodes
            .iter()
            .filter(|n| n.asset == asset)
            .map(|n| (n.id, n))
            .collect();
        let depth = |mut id: NodeId| {
            let mut depth = 0;
            while id != root {
                id = records.get(&id)?.parent?;
                depth += 1;
            }
            Some(depth)
        };
        let mut ids: Vec<(usize, NodeId)> = records
            .keys()
            .filter_map(|id| Some((depth(*id)?, *id)))
            .collect();
        ids.sort_by_key(|(depth, id)| (*depth, id.0));
        let nodes: Vec<NodeId> = ids.into_iter().map(|(_, id)| id).collect();
        let index: HashMap<_, _> = nodes.iter().enumerate().map(|(i, id)| (*id, i)).collect();
        let parent = nodes
            .iter()
            .map(|id| records[id].parent.and_then(|p| index.get(&p).copied()))
            .collect();
        let rest = nodes
            .iter()
            .map(|id| {
                let t = &records[id].transform;
                Frame {
                    position: Vector::from_array(t.translation),
                    rotation: Rotation::from_array(t.rotation),
                    scale: Vector::from_array(t.scale),
                }
            })
            .collect();
        if blend
            .clips
            .iter()
            .flat_map(|c| &c.tracks)
            .any(|t| !index.contains_key(&NodeId(t.node)))
        {
            return Err(ReferenceError("Rabbit track outside the rig".into()));
        }
        Ok(Self {
            nodes,
            index,
            parent,
            rest,
            lengths: blend
                .clips
                .iter()
                .map(|c| c.stop_time - c.start_time)
                .collect(),
        })
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
    pub fn nodes(&self) -> &[NodeId] {
        &self.nodes
    }
    pub fn index(&self, node: NodeId) -> Option<usize> {
        self.index.get(&node).copied()
    }

    pub fn pose(&self) -> RigPose {
        RigPose {
            local: self.rest.clone(),
            model: self.rest.clone(),
            clip: self.rest.clone(),
            weights: vec![0.; self.lengths.len()],
        }
    }

    /// Seconds of one blend cycle at `value`: the weighted clip lengths.
    pub fn duration(&self, blend: &BlendReference, value: f32, pose: &mut RigPose) -> f32 {
        weights(&blend.thresholds, value, &mut pose.weights);
        pose.weights
            .iter()
            .zip(&self.lengths)
            .map(|(w, l)| w * l)
            .sum()
    }

    /// Samples the blend at `phase` (normalized cycle time) and `value`.
    /// The root frame has the rabbit `scale` and no offset.
    pub fn sample(
        &self,
        blend: &BlendReference,
        phase: f32,
        value: f32,
        scale: f32,
        pose: &mut RigPose,
    ) {
        weights(&blend.thresholds, value, &mut pose.weights);
        let mut first = true;
        for (i, clip) in blend.clips.iter().enumerate() {
            let w = pose.weights[i];
            if w <= 0. {
                continue;
            }
            pose.clip.copy_from_slice(&self.rest);
            let time = clip.start_time + phase.rem_euclid(1.) * self.lengths[i];
            sample_clip(clip, time, |node, part| {
                let frame = &mut pose.clip[self.index[&NodeId(node)]];
                match part {
                    LocalPart::Position(p) => frame.position = p,
                    LocalPart::Rotation(r) => frame.rotation = r,
                    LocalPart::Scale(s) => frame.scale = s,
                }
            });
            for (out, f) in pose.local.iter_mut().zip(&pose.clip) {
                if first {
                    *out = Frame {
                        position: f.position * w,
                        rotation: f.rotation * w,
                        scale: f.scale * w,
                    };
                } else {
                    let r = if out.rotation.dot(f.rotation) < 0. {
                        -f.rotation
                    } else {
                        f.rotation
                    };
                    out.position += f.position * w;
                    out.rotation = out.rotation + r * w;
                    out.scale += f.scale * w;
                }
            }
            first = false;
        }
        if first {
            pose.local.copy_from_slice(&self.rest);
        }
        for f in &mut pose.local {
            f.rotation = f.rotation.normalize();
        }
        pose.model[0] = Frame {
            scale: Vector::splat(scale),
            ..Frame::IDENTITY
        };
        for i in 1..self.nodes.len() {
            let parent = self.parent[i].expect("rig nodes below the root");
            pose.model[i] = pose.model[parent].mul(&pose.local[i]);
        }
        pose.local[0] = pose.model[0];
    }
}

/// Unity 1D blend weights: the two thresholds around `value` share the weight.
fn weights(thresholds: &[f32], value: f32, out: &mut [f32]) {
    out.fill(0.);
    let last = thresholds.len() - 1;
    if value <= thresholds[0] {
        out[0] = 1.;
        return;
    }
    if value >= thresholds[last] {
        out[last] = 1.;
        return;
    }
    let i = thresholds.partition_point(|t| *t <= value) - 1;
    let t = (value - thresholds[i]) / (thresholds[i + 1] - thresholds[i]);
    out[i] = 1. - t;
    out[i + 1] = t;
}

#[cfg(test)]
mod tests {
    use super::weights;

    #[test]
    fn blend_weights_split_between_thresholds() {
        let mut w = [0.; 3];
        weights(&[0., 1., 3.], 2., &mut w);
        assert_eq!(w, [0., 0.5, 0.5]);
        weights(&[0., 1., 3.], -1., &mut w);
        assert_eq!(w, [1., 0., 0.]);
        weights(&[0., 1., 3.], 5., &mut w);
        assert_eq!(w, [0., 0., 1.]);
    }
}
