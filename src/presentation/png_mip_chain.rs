//! Checked sequential PNG mip decoding. This asset never enters the render world.
use crate::reference::{TextureColorSpace, TextureRecord};
use bevy::tasks::futures_lite::AsyncReadExt;
use bevy::{asset as bevy_asset, reflect as bevy_reflect};
use bevy::{
    asset::{AssetLoader, LoadContext, RenderAssetUsages, io::Reader},
    image::{CompressedImageFormats, ImageSampler, ImageType},
    prelude::*,
};
use std::{
    io::{Error, ErrorKind},
    sync::Arc,
};

#[derive(Asset, TypePath)]
pub struct SourcePngMipChain(pub Image);
#[derive(Default, TypePath)]
pub struct SourcePngMipChainLoader;
#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct SourcePngMipChainSettings {
    pub record: Option<Arc<TextureRecord>>,
    pub encoded_bytes: Vec<usize>,
}
fn invalid(message: impl Into<String>) -> Error {
    Error::new(ErrorKind::InvalidData, message.into())
}
impl AssetLoader for SourcePngMipChainLoader {
    type Asset = SourcePngMipChain;
    type Settings = SourcePngMipChainSettings;
    type Error = Error;
    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &Self::Settings,
        context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Error> {
        let record = settings
            .record
            .as_ref()
            .ok_or_else(|| invalid("Missing admitted PNG chain"))?;
        if record.mips.is_empty()
            || record.mips.len() != record.sampler.mip_count as usize
            || settings.encoded_bytes.len() != record.mips.len()
            || context.path().path() != std::path::Path::new(&record.mips[0].file)
        {
            return Err(invalid("Invalid admitted PNG chain"));
        }
        let byte_count = crate::reference::fixed_reservation::png_pixels(&record.mips)
            .map_err(|e| invalid(e.to_string()))?;
        let mut bytes = Vec::new();
        let mut assembled = None;
        for (index, mip) in record.mips.iter().enumerate() {
            // The base reader already checked its receipt. Never fetch it twice.
            let input = if index == 0 {
                let mut input = vec![0; settings.encoded_bytes[index]];
                reader.read_exact(&mut input).await?;
                let mut extra = [0];
                if reader.read(&mut extra).await? != 0 {
                    return Err(invalid(format!("{}: PNG exceeds receipt", mip.file)));
                }
                input
            } else {
                context
                    .read_asset_bytes(mip.file.clone())
                    .await
                    .map_err(|e| invalid(format!("{}: {e}", mip.file)))?
            };
            if input.len() != settings.encoded_bytes[index] {
                return Err(invalid(format!(
                    "{}: PNG byte count differs from receipt",
                    mip.file
                )));
            }
            let srgb = record.color_space == TextureColorSpace::Srgb;
            let mut image = Image::from_buffer(
                &input,
                ImageType::Extension("png"),
                CompressedImageFormats::NONE,
                srgb,
                ImageSampler::Default,
                RenderAssetUsages::MAIN_WORLD,
            )
            .map_err(|e| invalid(format!("{}: {e}", mip.file)))?;
            use bevy::render::render_resource::TextureFormat;
            let format = if srgb {
                TextureFormat::Rgba8UnormSrgb
            } else {
                TextureFormat::Rgba8Unorm
            };
            if image.width() != mip.width
                || image.height() != mip.height
                || image.texture_descriptor.format != format
                || image.texture_descriptor.mip_level_count != 1
                || image.data.as_ref().map(Vec::len)
                    != Some(mip.width as usize * mip.height as usize * 4)
            {
                return Err(invalid(format!("Invalid decoded mip {}", mip.file)));
            }
            // Allocate after the base decoder drops its RGB conversion buffers.
            if index == 0 {
                bytes.reserve_exact(byte_count);
            }
            // Taking drops the decoded capacity; the retained base Image keeps no pixel allocation.
            bytes.extend_from_slice(&image.data.take().expect("checked mip bytes"));
            if index == 0 {
                assembled = Some(image);
            }
            // Input and decoded level drop before the next checked read.
        }
        let mut image = assembled.expect("checked mip count");
        image.data = Some(bytes);
        image.texture_descriptor.mip_level_count = record.sampler.mip_count;
        image.asset_usage = RenderAssetUsages::RENDER_WORLD;
        super::source_images::apply_sampler(&mut image, &record.sampler);
        Ok(SourcePngMipChain(image))
    }
}
