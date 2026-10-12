use rapier3d::math::{Rotation, Vector};

use crate::reference::{MirrorRecord, MirrorRulesReference};
use crate::vehicle::VehiclePose;

/// Positions and axes use runtime coordinates.
/// Form axes from the plane world rotation, without hierarchy scale.
#[derive(Clone, Copy, Debug, Default)]
pub struct MirrorInput {
    pub plane_position: [f32; 3],
    /// Plane world rotation times runtime +Y.
    pub plane_normal: [f32; 3],
    /// Plane world rotation times runtime -Z, used as LookAt up.
    pub camera_up: [f32; 3],
    pub camera_position: [f32; 3],
    pub normal_view_visible: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct MirrorFrame {
    pub player_position: [f32; 3],
    pub view_head_position: [f32; 3],
    pub source_enabled: bool,
    pub pointer_locked: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct MirrorOutput {
    pub capture: bool,
    pub rank: usize,
    pub pose: VehiclePose,
    pub fov_degrees: f32,
    pub capture_near_plane: f32,
    pub capture_far_plane: f32,
    pub next_far_plane: f32,
}

impl MirrorOutput {
    /// Invalid source clipping disables drawing without changing retained source state.
    pub fn has_valid_clip_range(&self) -> bool {
        self.capture_near_plane.is_finite()
            && self.capture_far_plane.is_finite()
            && self.capture_near_plane > 0.0
            && self.capture_far_plane > self.capture_near_plane
    }
}

pub struct MirrorState {
    inputs: Vec<MirrorInput>,
    candidates: Vec<usize>,
    distances: Vec<f32>,
    outputs: Vec<MirrorOutput>,
}

impl MirrorState {
    pub fn new(reference: &MirrorRecord) -> Self {
        let mut state = Self {
            inputs: Vec::new(),
            candidates: Vec::new(),
            distances: Vec::new(),
            outputs: Vec::new(),
        };
        state.reset(reference);
        state
    }

    pub fn reset(&mut self, reference: &MirrorRecord) {
        let count = reference.mirrors.len();
        self.inputs.resize(count, MirrorInput::default());
        self.candidates.resize(count, 0);
        for (index, slot) in self.candidates.iter_mut().enumerate() {
            *slot = index;
        }
        self.distances.resize(count, f32::INFINITY);
        self.outputs.clear();
        self.outputs
            .extend(reference.mirrors.iter().map(|mirror| MirrorOutput {
                capture: false,
                rank: usize::MAX,
                pose: VehiclePose {
                    position: [0.0; 3],
                    rotation: [0.0, 0.0, 0.0, 1.0],
                },
                fov_degrees: mirror.serialized_fov_degrees,
                capture_near_plane: mirror.near_plane,
                capture_far_plane: mirror.initial_far_plane,
                next_far_plane: mirror.initial_far_plane,
            }));
    }

    pub fn outputs(&self) -> &[MirrorOutput] {
        &self.outputs
    }

    pub fn update(
        &mut self,
        rules: &MirrorRulesReference,
        frame: MirrorFrame,
        inputs: &[MirrorInput],
    ) {
        for output in &mut self.outputs {
            output.capture = false;
            output.rank = usize::MAX;
        }
        if !rules.enabled || !frame.source_enabled || !frame.pointer_locked {
            return;
        }
        let count = self.outputs.len().min(inputs.len());
        self.inputs[..count].copy_from_slice(&inputs[..count]);
        let player = vector(frame.player_position);
        let head = vector(frame.view_head_position);
        if !finite(&player) || !finite(&head) {
            return;
        }

        for index in 0..self.outputs.len() {
            self.distances[index] = if index < count {
                let plane = vector(self.inputs[index].plane_position);
                let distance = (plane - player).length();
                if distance.is_finite() {
                    distance
                } else {
                    f32::INFINITY
                }
            } else {
                f32::INFINITY
            };
        }

        // Source OrderBy is stable over its current list, including prior frame order.
        for rank in 1..self.candidates.len() {
            let index = self.candidates[rank];
            let distance = self.distances[index];
            let mut place = rank;
            while place > 0 && distance < self.distances[self.candidates[place - 1]] {
                self.candidates[place] = self.candidates[place - 1];
                place -= 1;
            }
            self.candidates[place] = index;
        }

        for rank in 0..self.candidates.len() {
            let index = self.candidates[rank];
            let output = &mut self.outputs[index];
            output.rank = rank;
            if rank >= rules.count_limit || index >= count || !self.distances[index].is_finite() {
                continue;
            }
            let input = self.inputs[index];
            let plane = vector(input.plane_position);
            let camera = vector(input.camera_position);
            // The producer supplies converted positions and rotation-only axes.
            let normal = vector(input.plane_normal);
            let up = vector(input.camera_up);
            let normal_length = normal.length();
            if !finite(&camera) || !finite(&normal) || !finite(&up) || normal_length <= f32::EPSILON
            {
                continue;
            }
            let normal = normal / normal_length;
            let view = plane - head;
            let reflected = view - normal * (2.0 * view.dot(normal));
            let target = plane + reflected;
            let direction = target - camera;
            let direction_length = direction.length();
            let up_length = up.length();
            if !finite(&direction)
                || direction_length <= f32::EPSILON
                || !up_length.is_finite()
                || up_length <= f32::EPSILON
                || direction.cross(up).length_squared() <= f32::EPSILON
            {
                continue;
            }
            let camera_distance = (camera - head).length();
            if !camera_distance.is_finite() {
                continue;
            }
            let camera_factor = inverse_lerp_reverse(rules.distance_bounds_meters, camera_distance);
            let distance_factor =
                inverse_lerp_reverse(rules.distance_bounds_meters, self.distances[index]);
            let fov = rules.fov_bounds_degrees[0]
                + (rules.fov_bounds_degrees[1] - rules.fov_bounds_degrees[0]) * camera_factor;
            if !fov.is_finite() {
                continue;
            }
            let forward = direction / direction_length;
            let up = up - forward * up.dot(forward);
            let up = up.normalize();
            let aim = Rotation::from_rotation_arc(Vector::NEG_Z, forward);
            let aimed_up = aim * Vector::Y;
            let roll = forward.dot(aimed_up.cross(up)).atan2(aimed_up.dot(up));
            let quaternion = (Rotation::from_axis_angle(forward, roll) * aim).normalize();
            let pose = VehiclePose {
                position: input.camera_position,
                rotation: [quaternion.x, quaternion.y, quaternion.z, quaternion.w],
            };
            if !pose.rotation.iter().all(|value| value.is_finite()) {
                continue;
            }
            output.pose = pose;
            output.fov_degrees = fov;
            if distance_factor > 0.0 && normal.dot(reflected) > 0.0 && input.normal_view_visible {
                output.capture = true;
                output.capture_far_plane = output.next_far_plane;
                output.next_far_plane = rules.draw_distance_meters * distance_factor;
            }
        }
    }
}

fn vector(value: [f32; 3]) -> Vector {
    Vector::new(value[0], value[1], value[2])
}

fn finite(value: &Vector) -> bool {
    value.x.is_finite() && value.y.is_finite() && value.z.is_finite()
}

fn inverse_lerp_reverse(bounds: [f32; 2], distance: f32) -> f32 {
    ((bounds[1] - distance) / (bounds[1] - bounds[0])).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reference::ReferenceManifest;

    #[test]
    fn updates_keep_storage_capacity() {
        let reference =
            ReferenceManifest::from_json(include_bytes!("../tests/common/reference.json")).unwrap();
        let mut state = MirrorState::new(&reference.mirrors);
        let capacities = (
            state.inputs.capacity(),
            state.candidates.capacity(),
            state.distances.capacity(),
            state.outputs.capacity(),
        );
        let inputs = vec![
            MirrorInput {
                plane_position: [0.0, 0.0, 2.0],
                plane_normal: [0.0, 1.0, 0.0],
                camera_up: [0.0, 0.0, -1.0],
                camera_position: [0.0; 3],
                normal_view_visible: true,
            };
            state.outputs.len()
        ];
        for _ in 0..128 {
            state.update(
                &reference.mirrors.rules,
                MirrorFrame {
                    player_position: [0.0; 3],
                    view_head_position: [0.0, 2.0, 0.0],
                    source_enabled: true,
                    pointer_locked: true,
                },
                &inputs,
            );
        }
        assert_eq!(
            capacities,
            (
                state.inputs.capacity(),
                state.candidates.capacity(),
                state.distances.capacity(),
                state.outputs.capacity(),
            )
        );
    }
}
