//! The selected source dashboard high-beam channel.
use crate::{lighting::LampMaterialOutput, reference::HighBeamLampReference};
#[derive(Clone, Copy, Debug, Default)]
pub struct InstrumentSample {
    pub high_beam: bool,
}
pub fn update_high_beam(
    lamp: &HighBeamLampReference,
    sample: InstrumentSample,
) -> LampMaterialOutput {
    let value = sample.high_beam as u8 as f32;
    let on = value >= lamp.min_value && (value <= lamp.max_value || lamp.stay_on);
    let material = if on { &lamp.on } else { &lamp.off };
    LampMaterialOutput {
        renderer: lamp.node,
        asset: material.asset,
        material_index: material.gltf_material,
    }
}
