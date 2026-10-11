//! Original decoded source mip loader.
use bevy::{
    asset::{AssetLoader, LoadContext, RenderAssetUsages, io::Reader},
    image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
    reflect as bevy_reflect,
    render::render_resource::*,
};
#[derive(Default, TypePath)]
pub struct SourceImageLoader;
#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct SourceImageSettings {
    pub srgb: bool,
    pub filter: u8,
    pub wrap_u: u8,
    pub wrap_v: u8,
}
impl AssetLoader for SourceImageLoader {
    type Asset = Image;
    type Settings = SourceImageSettings;
    type Error = std::io::Error;
    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &Self::Settings,
        _: &mut LoadContext<'_>,
    ) -> Result<Image, Self::Error> {
        let mut data = Vec::new();
        reader.read_to_end(&mut data).await?;
        let invalid = || {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid source mip payload",
            )
        };
        if data.len() < 16 || &data[..4] != b"SMIP" {
            return Err(invalid());
        }
        let width = u32::from_le_bytes(data[4..8].try_into().unwrap());
        let height = u32::from_le_bytes(data[8..12].try_into().unwrap());
        let count = u32::from_le_bytes(data[12..16].try_into().unwrap());
        if width == 0 || height == 0 || count == 0 || count > 32 - width.max(height).leading_zeros()
        {
            return Err(invalid());
        }
        let bytes = (0..count)
            .map(|level| {
                u64::from((width >> level).max(1)) * u64::from((height >> level).max(1)) * 4
            })
            .sum::<u64>();
        if bytes + 16 != data.len() as u64 {
            return Err(invalid());
        }
        data.drain(..16);
        let mut image = Image::new_uninit(
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            if settings.srgb {
                TextureFormat::Rgba8UnormSrgb
            } else {
                TextureFormat::Rgba8Unorm
            },
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        image.texture_descriptor.mip_level_count = count;
        image.data = Some(data);
        let address = |value| match value {
            0 => ImageAddressMode::Repeat,
            2 => ImageAddressMode::MirrorRepeat,
            _ => ImageAddressMode::ClampToEdge,
        };
        let filter = if settings.filter == 0 {
            ImageFilterMode::Nearest
        } else {
            ImageFilterMode::Linear
        };
        image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
            mag_filter: filter,
            min_filter: filter,
            mipmap_filter: if settings.filter == 2 {
                ImageFilterMode::Linear
            } else {
                ImageFilterMode::Nearest
            },
            address_mode_u: address(settings.wrap_u),
            address_mode_v: address(settings.wrap_v),
            ..default()
        });
        Ok(image)
    }
    fn extensions(&self) -> &[&str] {
        &["smip"]
    }
}
