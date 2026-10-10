//! `roadGenScript.PlaceOneRoad`: bone rotation, skinning, and the pole.
//! Inputs use Unity space. Outputs use game space relative to the segment start.

use rapier3d::glamx::{EulerRot, Mat4, Quat, Vec3};

use super::generation::DotNetRandom;
use crate::reference::world::{PoleShape, RoadBiomeReference, RoadReference};

/// Game space mirrors Unity Z.
const MIRROR: Mat4 = Mat4::from_cols_array(&[
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
]);

fn to_game(matrix: Mat4) -> Mat4 {
    MIRROR * matrix * MIRROR
}

/// Converts a column-major Unity matrix to game space.
pub fn mirror(matrix: [f32; 16]) -> [f32; 16] {
    to_game(Mat4::from_cols_array(&matrix)).to_cols_array()
}

#[derive(Clone, Debug, PartialEq)]
pub enum PoleCollider {
    Capsule { a: Vec3, b: Vec3, radius: f32 },
    Box { matrix: Mat4, half_size: Vec3 },
}

/// Placed road geometry. Positions are game meters from the segment start.
pub struct RoadMesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<[u32; 3]>,
    pub biome: usize,
    /// Game transform of the pole root after `RandomPoleRot`.
    pub pole: Mat4,
    pub colliders: Vec<PoleCollider>,
}

/// Bone transforms after `PlaceOneRoad` rotates the rest pose.
fn bones(road: &RoadReference, biome: &RoadBiomeReference, rotations: &[[f32; 2]]) -> Vec<Mat4> {
    let armature = Mat4::from_cols_array(&biome.armature);
    let (_, mut parent, mut position) = armature.to_scale_rotation_translation();
    let mut joints = Vec::with_capacity(road.segment.bind_poses.len());
    for (index, rotation) in rotations.iter().enumerate() {
        if index > 0 {
            position += parent * Vec3::new(0.0, road.bone_length, 0.0);
        }
        // `Transform.Rotate` in world space applies the new rotation before the current one.
        let pitch = Quat::from_axis_angle(parent * Vec3::X, rotation[0].to_radians());
        let yaw = Quat::from_axis_angle(Vec3::Y, rotation[1].to_radians());
        parent = (yaw * pitch * parent).normalize();
        joints.push(Mat4::from_rotation_translation(parent, position));
    }
    joints.resize(road.segment.bind_poses.len(), Mat4::IDENTITY);
    for bone in &biome.static_bones {
        joints[bone.joint as usize] = armature * Mat4::from_cols_array(&bone.matrix);
    }
    joints
}

/// `roadscript.RandomPoleRot` in degrees.
fn pole_rotation(biome: &RoadBiomeReference, index: usize, seed: i32) -> Quat {
    let pole = &biome.pole;
    let mut random = DotNetRandom::new(seed.wrapping_add(index as i32));
    let roll = random.next_double() as f32 * (pole.chances[0] + pole.chances[1]);
    let angles = if roll < pole.chances[0] {
        pole.angles[0]
    } else if roll - pole.chances[0] < pole.chances[1] {
        pole.angles[1]
    } else {
        return Quat::IDENTITY;
    };
    let [x, y, z] = angles.map(|a| (random.range(-a, a) as f32).to_radians());
    // Unity Euler order: Z, then X, then Y.
    Quat::from_euler(EulerRot::YXZ, y, x, z)
}

fn pole_collider(matrix: Mat4, shape: &PoleShape) -> PoleCollider {
    let (scale, rotation, _) = matrix.to_scale_rotation_translation();
    match *shape {
        PoleShape::Capsule {
            center,
            radius,
            height,
            direction,
        } => {
            let axis = direction as usize;
            let mut offset = Vec3::ZERO;
            offset[axis] = (height * 0.5 - radius).max(0.0);
            let center = Vec3::from_array(center);
            let game = MIRROR * matrix;
            PoleCollider::Capsule {
                a: game.transform_point3(center + offset),
                b: game.transform_point3(center - offset),
                radius: radius
                    * (0..3)
                        .filter(|i| *i != axis)
                        .map(|i| scale[i].abs())
                        .fold(0.0, f32::max),
            }
        }
        PoleShape::Box { center, size } => {
            let center = matrix.transform_point3(Vec3::from_array(center));
            PoleCollider::Box {
                matrix: to_game(Mat4::from_rotation_translation(rotation, center)),
                half_size: Vec3::from_array(size) * scale.abs() * 0.5,
            }
        }
    }
}

/// Skinned road and pole for one segment. `BakeMesh` gives the same collider vertices.
pub fn place(
    road: &RoadReference,
    biome: usize,
    rotations: &[[f32; 2]],
    index: usize,
    seed: i32,
) -> RoadMesh {
    let reference = &road.biomes[biome];
    let mesh = &road.segment;
    let joints = bones(road, reference, rotations);
    let skins: Vec<Mat4> = joints
        .iter()
        .zip(&mesh.bind_poses)
        .map(|(joint, bind)| *joint * Mat4::from_cols_array(bind))
        .collect();
    let mut positions = Vec::with_capacity(mesh.positions.len());
    let mut normals = Vec::with_capacity(mesh.positions.len());
    for ((point, normal), joint) in mesh.positions.iter().zip(&mesh.normals).zip(&mesh.joints) {
        let skin = MIRROR * skins[*joint as usize];
        positions.push(skin.transform_point3(Vec3::from_array(*point)).to_array());
        normals.push(
            skin.transform_vector3(Vec3::from_array(*normal))
                .normalize_or_zero()
                .to_array(),
        );
    }
    let pole = &reference.pole;
    let root = joints[0]
        * Mat4::from_cols_array(&pole.matrix)
        * Mat4::from_quat(pole_rotation(reference, index, seed));
    RoadMesh {
        positions,
        normals,
        uvs: mesh.uvs.clone(),
        // The Z mirror reverses winding.
        indices: mesh.triangles.iter().map(|t| [t[0], t[2], t[1]]).collect(),
        biome,
        pole: to_game(root),
        colliders: pole
            .colliders
            .iter()
            .map(|c| pole_collider(root * Mat4::from_cols_array(&c.matrix), &c.shape))
            .collect(),
    }
}
