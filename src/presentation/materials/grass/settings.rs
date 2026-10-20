use crate::reference::{ReferenceError, WorldSourceMaterial};
use grass_mesh::GrassSettings;

pub fn settings_from_source(
    material: &WorldSourceMaterial,
) -> Result<GrassSettings, ReferenceError> {
    let get = |name: &str| {
        material
            .floats
            .get(name)
            .copied()
            .ok_or_else(|| ReferenceError(format!("Grass material is missing {name}")))
    };
    let max_tessellation = get("_MaxTessellation")?;
    let lod_max = get("_LODMax")?;
    if !matches!(max_tessellation, 2.0 | 4.0)
        || !(1.0..=6.0).contains(&lod_max)
        || lod_max.fract() != 0.0
    {
        return Err(ReferenceError(
            "Unsupported checked Grass tessellation or blade segment range".into(),
        ));
    }
    Ok(GrassSettings {
        edge_length: get("_EdgeLength")?,
        max_tessellation: max_tessellation as u32,
        max_height: get("_MaxHeight00")?,
        fade_end: get("_GrassFadeEnd")?,
        lod_start: get("_LODStart")?,
        lod_end: get("_LODEnd")?,
        lod_max: lod_max as u32,
        smooth_domain: material
            .valid_keywords
            .iter()
            .any(|x| x == "GRASS_HEIGHT_SMOOTHING" || x == "GRASS_WIDTH_SMOOTHING"),
    })
}
