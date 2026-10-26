//! Computer screens: one CPU-drawn C64output image per computer, shown by connected TVs.
use crate::interaction::computer::screen;
use crate::reference::{ComputerReference, ItemId};
use crate::world::stream::content_computers::Computers;
use bevy::{
    asset::RenderAssetUsages,
    ecs as bevy_ecs,
    image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};

#[derive(Resource, Default)]
pub struct ComputerScreens {
    screens: Vec<(ItemId, Handle<Image>)>,
}

impl ComputerScreens {
    pub fn image(&self, item: ItemId) -> Option<&Handle<Image>> {
        self.screens
            .iter()
            .find(|(i, _)| *i == item)
            .map(|(_, h)| h)
    }
    /// Make one image per bound computer and redraw the changed screens. The images of
    /// removed computers drop.
    pub(crate) fn sync(
        &mut self,
        reference: &ComputerReference,
        computers: &mut Computers,
        images: &mut Assets<Image>,
    ) {
        self.screens
            .retain(|(item, _)| computers.computer(*item).is_some());
        computers.take_screens(|item, computer, changed| {
            let handle = match self.screens.iter().find(|(i, _)| *i == item) {
                Some(_) if !changed => return,
                Some((_, handle)) => handle.clone(),
                None => {
                    let handle = images.add(blank(reference));
                    self.screens.push((item, handle.clone()));
                    handle
                }
            };
            if let Some(mut image) = images.get_mut(&handle)
                && let Some(pixels) = image.data.as_mut()
            {
                screen::draw(reference, computer, pixels);
            }
        });
    }
}

/// C64output: point filter, clamp, no mips, non-sRGB.
fn blank(reference: &ComputerReference) -> Image {
    let [width, height] = reference.screen.size.map(u32::from);
    let mut image = Image::new_fill(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0; 4],
        TextureFormat::Rgba8Unorm,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::ClampToEdge,
        mag_filter: ImageFilterMode::Nearest,
        min_filter: ImageFilterMode::Nearest,
        mipmap_filter: ImageFilterMode::Nearest,
        ..default()
    });
    image
}
