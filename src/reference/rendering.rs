//! Shared source resource links used by the lighting export.
use super::{AssetId, SourceId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceReference {
    pub source: SourceId,
    pub path: String,
    pub sha256: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShaderReference {
    pub compiled_data: ResourceReference,
    pub source: SourceId,
    pub path: String,
    pub sha256: String,
    pub name: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextureRecord {
    pub source_data: ResourceReference,
    pub id: u32,
    pub source: SourceId,
    pub path: String,
    pub sha256: String,
    pub width: u32,
    pub height: u32,
    pub source_format: u32,
    pub sample_channel: String,
    pub color_space: u8,
    pub filter_mode: u8,
    pub wrap_u: u8,
    pub wrap_v: u8,
    pub mip_count: u32,
    pub aniso: u32,
    pub mip_bias: f32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextureLink {
    pub property: String,
    pub texture: u32,
    pub scale: [f32; 2],
    pub offset: [f32; 2],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkyMaterialReference {
    pub source: SourceId,
    pub shader: ShaderReference,
    pub source_data: ResourceReference,
    pub floats: BTreeMap<String, f32>,
    pub colors: BTreeMap<String, [f32; 4]>,
    pub textures: Vec<TextureLink>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialLink {
    pub asset: AssetId,
    pub gltf_material: u32,
    pub source: SourceId,
    pub condition_key: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialBindingReference {
    pub asset: AssetId,
    pub gltf_material: u32,
    pub source: SourceId,
}
