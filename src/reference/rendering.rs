//! Source material bindings and authored texture levels.

use super::{AssetId, AssetRecord, ReferenceError, SourceId, finite, require};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenderingRecord {
    pub materials: Vec<MaterialRecord>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShaderFamily {
    Standard,
    UnlitColor,
    StainedGlass,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialRecord {
    pub asset: AssetId,
    pub gltf_material: u32,
    pub source: SourceId,
    pub shader_family: ShaderFamily,
    pub color: [f32; 4],
    pub condition: Option<MaterialCondition>,
    pub keywords: Vec<String>,
    pub state: MaterialState,
    pub properties: ShaderProperties,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialCondition {
    pub source: SourceId,
    pub state: u8,
    pub quality: f32,
    pub range: Option<[u8; 2]>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlphaMode {
    Opaque,
    Mask,
    Blend,
    Premultiply,
    Refract,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialState {
    pub alpha_mode: AlphaMode,
    pub cull: u8,
    pub z_write: bool,
    pub z_test: u8,
    pub src_blend: u8,
    pub dst_blend: u8,
    pub render_queue: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShaderProperties {
    pub floats: BTreeMap<String, f32>,
    pub colors: BTreeMap<String, [f32; 4]>,
    pub textures: Vec<TextureRecord>,
    pub defaults: BTreeMap<String, TextureDefault>,
    pub exclusions: Vec<TextureExclusion>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextureDefault {
    White,
    Grey,
    Black,
    Bump,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextureExclusion {
    pub slot: String,
    pub source: SourceId,
    pub reason: ExclusionReason,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExclusionReason {
    LiveMirror,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextureColorSpace {
    Srgb,
    Linear,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextureEncoding {
    Rgba,
    Rgb,
    Ag,
    Rg,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextureRecord {
    pub slot: String,
    pub source: SourceId,
    pub file: String,
    pub color_space: TextureColorSpace,
    pub encoding: TextureEncoding,
    pub source_encoding: TextureEncoding,
    pub uv_channel: u8,
    pub scale: [f32; 2],
    pub offset: [f32; 2],
    pub sampler: SamplerRecord,
    pub mips: Vec<TextureMip>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SamplerRecord {
    pub filter: u8,
    pub wrap_u: u8,
    pub wrap_v: u8,
    pub aniso: u8,
    pub mip_bias: f32,
    pub mip_count: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextureMip {
    pub file: String,
    pub width: u32,
    pub height: u32,
}

fn source(id: &SourceId) -> bool {
    !id.file.is_empty() && id.path_id != 0
}
fn safe_path(path: &str) -> bool {
    path.starts_with("reference/")
        && !path.contains(['\\', ':', '?', '#', '\0'])
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}
fn known_slot(slot: &str) -> bool {
    matches!(
        slot,
        "_MainTex"
            | "_BumpMap"
            | "_DetailAlbedoMap"
            | "_DetailMask"
            | "_DetailNormalMap"
            | "_MetallicGlossMap"
            | "_OcclusionMap"
            | "_EmissionMap"
            | "_ParallaxMap"
    )
}

impl RenderingRecord {
    pub fn validate(&self, assets: &[AssetRecord]) -> Result<(), ReferenceError> {
        let mut bindings = HashSet::new();
        for material in &self.materials {
            let label = format!(
                "material {}:{}",
                material.source.file, material.source.path_id
            );
            let asset = assets
                .iter()
                .find(|asset| asset.id == material.asset)
                .ok_or_else(|| ReferenceError(format!("{label}: missing asset")))?;
            require(
                bindings.insert((material.asset, material.gltf_material)),
                &format!("{label}: duplicate binding"),
            )?;
            require(source(&material.source) && finite(&material.color), &label)?;
            if let Some(condition) = &material.condition {
                require(
                    source(&condition.source)
                        && condition.state <= 4
                        && condition.quality.is_finite()
                        && (condition.quality - (1. - f32::from(condition.state) / 4.)).abs()
                            < 0.0001
                        && condition.range.is_none_or(|[a, b]| a <= b && b <= 4),
                    &format!("{label}: condition"),
                )?;
            }
            let state = &material.state;
            require(
                state.cull <= 2
                    && state.z_test <= 8
                    && state.src_blend <= 10
                    && state.dst_blend <= 10
                    && (-1..=5000).contains(&state.render_queue),
                &format!("{label}: state"),
            )?;
            require(
                material
                    .properties
                    .floats
                    .values()
                    .all(|value| value.is_finite())
                    && material
                        .properties
                        .colors
                        .values()
                        .all(|value| finite(value)),
                &format!("{label}: properties"),
            )?;
            let keywords: HashSet<_> = material.keywords.iter().map(String::as_str).collect();
            require(
                keywords.len() == material.keywords.len(),
                &format!("{label}: duplicate keywords"),
            )?;
            require(
                material.properties.colors.contains_key("_Color"),
                &format!("{label}: _Color"),
            )?;
            let mut active = HashSet::new();
            match material.shader_family {
                ShaderFamily::Standard => {
                    require(
                        keywords.iter().all(|key| {
                            matches!(
                                *key,
                                "_NORMALMAP"
                                    | "_DETAIL_MULX2"
                                    | "_METALLICGLOSSMAP"
                                    | "_EMISSION"
                                    | "_PARALLAXMAP"
                                    | "_SMOOTHNESS_TEXTURE_ALBEDO_CHANNEL_A"
                                    | "_SPECULARHIGHLIGHTS_OFF"
                                    | "_GLOSSYREFLECTIONS_OFF"
                                    | "_ALPHATEST_ON"
                                    | "_ALPHABLEND_ON"
                                    | "_ALPHAPREMULTIPLY_ON"
                            )
                        }),
                        &format!("{label}: keyword"),
                    )?;
                    for field in [
                        "_Cutoff",
                        "_Glossiness",
                        "_GlossMapScale",
                        "_Metallic",
                        "_BumpScale",
                        "_OcclusionStrength",
                        "_DetailNormalMapScale",
                        "_UVSec",
                        "_SrcBlend",
                        "_DstBlend",
                        "_ZWrite",
                    ] {
                        require(
                            material.properties.floats.contains_key(field),
                            &format!("{label}: {field}"),
                        )?;
                    }
                    if keywords.contains("_EMISSION") {
                        require(
                            material.properties.colors.contains_key("_EmissionColor"),
                            &format!("{label}: _EmissionColor"),
                        )?;
                    }
                    if keywords.contains("_PARALLAXMAP") {
                        require(
                            material.properties.floats.contains_key("_Parallax"),
                            &format!("{label}: _Parallax"),
                        )?;
                    }
                    let alpha = if keywords.contains("_ALPHAPREMULTIPLY_ON") {
                        AlphaMode::Premultiply
                    } else if keywords.contains("_ALPHABLEND_ON") {
                        AlphaMode::Blend
                    } else if keywords.contains("_ALPHATEST_ON") {
                        AlphaMode::Mask
                    } else {
                        AlphaMode::Opaque
                    };
                    require(state.alpha_mode == alpha, &format!("{label}: alpha mode"))?;
                    active.extend(["_MainTex", "_OcclusionMap"]);
                    if keywords.contains("_NORMALMAP") {
                        active.insert("_BumpMap");
                    }
                    if keywords.contains("_DETAIL_MULX2") {
                        active.extend(["_DetailAlbedoMap", "_DetailMask"]);
                        if keywords.contains("_NORMALMAP") {
                            active.insert("_DetailNormalMap");
                        }
                    }
                    for (keyword, slot) in [
                        ("_METALLICGLOSSMAP", "_MetallicGlossMap"),
                        ("_EMISSION", "_EmissionMap"),
                        ("_PARALLAXMAP", "_ParallaxMap"),
                    ] {
                        if keywords.contains(keyword) {
                            active.insert(slot);
                        }
                    }
                }
                ShaderFamily::StainedGlass => {
                    active.extend(["_MainTex", "_BumpMap"]);
                    require(
                        material.properties.floats.contains_key("_BumpAmt")
                            && state.alpha_mode == AlphaMode::Refract,
                        &label,
                    )?;
                }
                ShaderFamily::UnlitColor => {
                    require(state.alpha_mode == AlphaMode::Opaque, &label)?;
                }
            }
            let mut slots = HashSet::new();
            for texture in &material.properties.textures {
                let field = format!("{label}: {}", texture.slot);
                require(
                    known_slot(&texture.slot)
                        && active.contains(texture.slot.as_str())
                        && slots.insert(texture.slot.as_str()),
                    &field,
                )?;
                require(
                    source(&texture.source)
                        && safe_path(&texture.file)
                        && texture.uv_channel <= 1
                        && finite(&texture.scale)
                        && finite(&texture.offset),
                    &field,
                )?;
                let normal = matches!(texture.slot.as_str(), "_BumpMap" | "_DetailNormalMap");
                let color = matches!(
                    texture.slot.as_str(),
                    "_MainTex" | "_DetailAlbedoMap" | "_EmissionMap"
                );
                require(
                    texture.encoding
                        == if normal {
                            TextureEncoding::Rgb
                        } else {
                            TextureEncoding::Rgba
                        }
                        && texture.color_space
                            == if color {
                                TextureColorSpace::Srgb
                            } else {
                                TextureColorSpace::Linear
                            },
                    &field,
                )?;
                let sampler = &texture.sampler;
                require(
                    sampler.filter <= 2
                        && sampler.wrap_u <= 3
                        && sampler.wrap_v <= 3
                        && sampler.aniso <= 16
                        && sampler.mip_bias.is_finite()
                        && sampler.mip_count > 0
                        && sampler.mip_count as usize == texture.mips.len(),
                    &field,
                )?;
                require(
                    texture
                        .mips
                        .first()
                        .is_some_and(|mip| mip.file == texture.file),
                    &field,
                )?;
                let mut dimensions: Option<(u32, u32)> = None;
                let mut paths = HashSet::new();
                for mip in &texture.mips {
                    require(
                        safe_path(&mip.file)
                            && paths.insert(&mip.file)
                            && mip.width > 0
                            && mip.height > 0
                            && asset
                                .dependencies
                                .iter()
                                .any(|dependency| dependency.file == mip.file),
                        &field,
                    )?;
                    if let Some((width, height)) = dimensions {
                        require(
                            (width > 1 || height > 1)
                                && mip.width == (width / 2).max(1)
                                && mip.height == (height / 2).max(1),
                            &field,
                        )?;
                    }
                    dimensions = Some((mip.width, mip.height));
                }
            }
            for (slot, default) in &material.properties.defaults {
                require(
                    known_slot(slot)
                        && active.contains(slot.as_str())
                        && slots.insert(slot.as_str()),
                    &label,
                )?;
                let expected = match slot.as_str() {
                    "_BumpMap" | "_DetailNormalMap" => TextureDefault::Bump,
                    "_DetailAlbedoMap" => TextureDefault::Grey,
                    "_ParallaxMap" => TextureDefault::Black,
                    _ => TextureDefault::White,
                };
                require(*default == expected, &format!("{label}: {slot} default"))?;
            }
            require(
                slots == active,
                &format!("{label}: missing active texture/default"),
            )?;
            for exclusion in &material.properties.exclusions {
                require(
                    matches!(exclusion.slot.as_str(), "_MainTex" | "_EmissionMap")
                        && source(&exclusion.source)
                        && material.source.file == "sharedassets1.assets"
                        && material.source.path_id == 624,
                    &format!("{label}: live mirror exclusion"),
                )?;
            }
        }
        for asset in assets {
            let count = self
                .materials
                .iter()
                .filter(|record| record.asset == asset.id)
                .count();
            require(
                count > 0 && (0..count as u32).all(|index| bindings.contains(&(asset.id, index))),
                "missing glTF material binding",
            )?;
        }
        Ok(())
    }
}
