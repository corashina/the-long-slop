//! Stable world values. These types contain no renderer or physics handles.
use crate::reference::{ItemId, SourceId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorldLayer {
    Road,
    Terrain,
    DistantTerrain,
    DistantTerrain2,
    Building,
    Prop,
    Mountain,
    Tower,
    DesertBuilding,
    Lamp,
    Nested,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GeneratedId {
    pub layer: WorldLayer,
    pub generator: SourceId,
    pub unit: [i64; 2],
    pub ordinal: u32,
    pub children: Vec<(SourceId, u32)>,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum ObjectKey {
    Imported(ItemId),
    Generated(GeneratedId),
}
impl ObjectKey {
    /// Stable 64-bit FNV-1a seed of the key. Each copy of one prototype gets its own seed.
    pub fn seed(&self) -> u64 {
        fn mix(hash: &mut u64, bytes: &[u8]) {
            for byte in bytes {
                *hash = (*hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3);
            }
        }
        fn source(hash: &mut u64, id: &SourceId) {
            mix(hash, id.file.as_bytes());
            mix(hash, &id.path_id.to_le_bytes());
        }
        let mut hash = 0xcbf29ce484222325;
        match self {
            Self::Imported(id) => mix(&mut hash, &id.0.to_le_bytes()),
            Self::Generated(id) => {
                mix(&mut hash, &[id.layer as u8]);
                source(&mut hash, &id.generator);
                mix(&mut hash, &id.unit[0].to_le_bytes());
                mix(&mut hash, &id.unit[1].to_le_bytes());
                mix(&mut hash, &id.ordinal.to_le_bytes());
                for (child, ordinal) in &id.children {
                    source(&mut hash, child);
                    mix(&mut hash, &ordinal.to_le_bytes());
                }
            }
        }
        hash
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AbsolutePose {
    pub position: [f64; 3],
    pub rotation: [f32; 4],
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldBounds {
    pub min: [f64; 3],
    pub max: [f64; 3],
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldSurface {
    pub origin: [f64; 3],
    pub vertices: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub tangents: Vec<[f32; 4]>,
    pub uv: Vec<[f32; 2]>,
    /// Each range retains one authored material slot.
    pub submeshes: Vec<std::ops::Range<usize>>,
    pub colors: Vec<[f32; 4]>,
    pub triangles: Vec<[u32; 3]>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Placement {
    pub id: GeneratedId,
    pub prototype: SourceId,
    pub pose: AbsolutePose,
    pub scale: [f32; 3],
    pub bounds: WorldBounds,
    pub object_seed: i32,
}
