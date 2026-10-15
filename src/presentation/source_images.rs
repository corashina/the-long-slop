//! Shared assembly of authored PNG mip levels. Runs only during asset loading.
use crate::reference::{SamplerRecord, TextureMip};
use bevy::{
    asset::RenderAssetUsages,
    image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
};

pub fn assemble_mips(
    levels: &[Handle<Image>],
    mips: &[TextureMip],
    settings: &SamplerRecord,
    images: &mut Assets<Image>,
) -> Result<(Image, usize), String> {
    if levels.len() != mips.len() || mips.is_empty() || mips.len() != settings.mip_count as usize {
        return Err("Invalid authored mip count".into());
    }
    let byte_count = mips
        .iter()
        .map(|mip| mip.width as usize * mip.height as usize * 4)
        .sum();
    for (handle, mip) in levels.iter().zip(mips) {
        let image = images
            .get(handle.id())
            .ok_or_else(|| format!("Missing decoded mip {}", mip.file))?;
        let expected = mip.width as usize * mip.height as usize * 4;
        if image.width() != mip.width
            || image.height() != mip.height
            || image.data.as_ref().map(Vec::len) != Some(expected)
        {
            return Err(format!("Invalid decoded mip {}", mip.file));
        }
    }
    let mut bytes = Vec::with_capacity(byte_count);
    let mut assembled = None;
    for (handle, mip) in levels.iter().zip(mips) {
        let mut image = images
            .remove(handle.id())
            .ok_or_else(|| format!("Missing decoded mip {}", mip.file))?;
        bytes.append(image.data.as_mut().expect("checked pixels"));
        if assembled.is_none() {
            assembled = Some(image);
        }
    }
    let mut image = assembled.expect("validated mip count");
    image.data = Some(bytes);
    image.texture_descriptor.mip_level_count = settings.mip_count;
    image.asset_usage = RenderAssetUsages::RENDER_WORLD;
    let filter = if settings.filter == 0 {
        ImageFilterMode::Nearest
    } else {
        ImageFilterMode::Linear
    };
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: wrap(settings.wrap_u),
        address_mode_v: wrap(settings.wrap_v),
        mag_filter: filter,
        min_filter: filter,
        mipmap_filter: if settings.filter == 2 || settings.aniso > 1 {
            ImageFilterMode::Linear
        } else {
            ImageFilterMode::Nearest
        },
        anisotropy_clamp: u16::from(settings.aniso.max(1)),
        lod_max_clamp: (settings.mip_count - 1) as f32,
        ..default()
    });
    Ok((image, byte_count))
}
pub(crate) fn wrap(value: u8) -> ImageAddressMode {
    match value {
        0 => ImageAddressMode::Repeat,
        2 => ImageAddressMode::MirrorRepeat,
        _ => ImageAddressMode::ClampToEdge,
    }
}
