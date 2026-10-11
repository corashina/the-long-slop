//! Road and terrain generator records. Values use Unity space and column-major matrices.

use super::{ReferenceError, SourceHash, SourceId, require};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldAsset {
    pub path: String,
    pub sha256: String,
    pub dependencies: Vec<SourceHash>,
}

/// Near `TerrainSetting` inputs at the default draw distance.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainReference {
    pub resolution: usize,
    pub tile_size: f32,
    pub generate_distance: f32,
    pub remove_distance: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlatZoneReference {
    pub center: [f32; 3],
    pub inner_sqr: f32,
    pub outer_sqr: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoadNoise {
    pub amplitude: f32,
    pub frequency: f32,
    pub octaves: u32,
    pub persistence: f32,
    pub lacunarity: f32,
}

/// Source road mesh. Each vertex follows one bone with full weight.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoadMeshReference {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub triangles: Vec<[u32; 3]>,
    pub joints: Vec<u16>,
    pub bind_poses: Vec<[f32; 16]>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StaticBoneReference {
    pub joint: u16,
    pub matrix: [f32; 16],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PoleShape {
    Capsule {
        center: [f32; 3],
        radius: f32,
        height: f32,
        direction: u8,
    },
    Box {
        center: [f32; 3],
        size: [f32; 3],
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PoleRenderer {
    pub matrix: [f32; 16],
    pub mesh: u32,
    pub material: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PoleCollider {
    pub matrix: [f32; 16],
    pub shape: PoleShape,
}

/// Power pole on the first road bone. `roadscript.RandomPoleRot` tilts it.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PoleReference {
    pub matrix: [f32; 16],
    pub chances: [f32; 2],
    pub angles: [[i32; 3]; 2],
    pub renderers: Vec<PoleRenderer>,
    pub colliders: Vec<PoleCollider>,
}

/// One road prefab. The biome index selects it.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoadBiomeReference {
    pub source: SourceId,
    pub material: u32,
    pub mesh: SourceId,
    pub armature: [f32; 16],
    pub static_bones: Vec<StaticBoneReference>,
    pub pole: PoleReference,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoadReference {
    pub source: SourceId,
    pub length: f32,
    pub width: f32,
    pub scale: f32,
    pub noises: Vec<RoadNoise>,
    pub segment_length: f32,
    pub bone_length: f32,
    pub bone_count: usize,
    pub max_pitch_degrees: f32,
    pub max_yaw_degrees: f32,
    pub above_terrain: f32,
    pub start_straight: usize,
    pub straight_modifier: f32,
    pub point_range: f32,
    pub generate_distance: f32,
    pub segment: RoadMeshReference,
    pub biomes: Vec<RoadBiomeReference>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldReference {
    pub source: SourceId,
    pub graphics_source: SourceId,
    pub asset: WorldAsset,
    pub seed: i32,
    pub edge_of_world: f32,
    pub edge_gradient: f32,
    pub terrain: TerrainReference,
    pub terrain_material: u32,
    pub flat_zones: Vec<FlatZoneReference>,
    pub height_anchor: [f32; 3],
    pub road: RoadReference,
}

fn finite(values: &[f32]) -> bool {
    values.iter().all(|v| v.is_finite())
}

impl WorldReference {
    pub fn validate(&self) -> Result<(), ReferenceError> {
        let road = &self.road;
        let mesh = &road.segment;
        require(
            self.asset.path.ends_with(".gltf") && super::hash(&self.asset.sha256),
            "world asset",
        )?;
        let terrain = &self.terrain;
        require(
            terrain.resolution >= 4
                && terrain.tile_size > 0.0
                && terrain.generate_distance > 0.0
                && terrain.remove_distance >= terrain.generate_distance,
            "terrain",
        )?;
        require(
            self.edge_of_world > self.edge_gradient && self.edge_gradient > 0.0,
            "edge of world",
        )?;
        require(
            self.flat_zones.iter().all(|zone| {
                finite(&zone.center) && zone.inner_sqr > 0.0 && zone.outer_sqr > zone.inner_sqr
            }) && finite(&self.height_anchor),
            "flat zones",
        )?;
        require(
            road.bone_count >= 2
                && road.bone_length > 0.0
                && (road.bone_length * road.bone_count as f32 - road.segment_length).abs() < 1e-3
                && road.length > 0.0
                && road.scale > 0.0
                && road.width > 0.0
                && road.point_range > 0.0
                && road.generate_distance > 0.0
                && !road.noises.is_empty()
                && road
                    .noises
                    .iter()
                    .all(|n| n.octaves > 0 && n.frequency > 0.0),
            "road generator",
        )?;
        let joints = mesh.bind_poses.len();
        require(
            joints >= road.bone_count
                && !mesh.positions.is_empty()
                && mesh.normals.len() == mesh.positions.len()
                && mesh.uvs.len() == mesh.positions.len()
                && mesh.joints.len() == mesh.positions.len()
                && mesh.joints.iter().all(|j| (*j as usize) < joints)
                && mesh
                    .triangles
                    .iter()
                    .flatten()
                    .all(|i| (*i as usize) < mesh.positions.len())
                && mesh.bind_poses.iter().all(|m| finite(m)),
            "road mesh",
        )?;
        // `GetBiome` returns desert, winter, or grass.
        require(road.biomes.len() == 3, "road biomes")?;
        for biome in &road.biomes {
            let pole = &biome.pole;
            require(
                biome.static_bones.len() + road.bone_count == joints
                    && biome
                        .static_bones
                        .iter()
                        .all(|b| (b.joint as usize) >= road.bone_count && finite(&b.matrix))
                    && finite(&biome.armature)
                    && finite(&pole.matrix)
                    && pole.chances.iter().all(|c| *c >= 0.0)
                    && pole.colliders.iter().all(|c| match c.shape {
                        PoleShape::Capsule {
                            radius,
                            height,
                            direction,
                            ..
                        } => radius > 0.0 && height > 0.0 && direction < 3,
                        PoleShape::Box { size, .. } => size.iter().all(|s| *s > 0.0),
                    }),
                "road biome",
            )?;
        }
        Ok(())
    }
}
