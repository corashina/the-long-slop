//! Startup-only loading for the source lighting resources.
use super::materials::RenderingState;
use super::source_lighting::{SourceLighting, SourceLightingStatus};
use crate::reference::{SourceId, TextureColorSpace, lighting_scene::LightingRecord};
use bevy::asset::{
    AssetLoader, LoadContext, LoadState, RecursiveDependencyLoadState, RenderAssetUsages,
    io::Reader,
};
use bevy::gltf::{GltfAssetLabel, GltfLoaderSettings, convert_coordinates::GltfConvertCoordinates};
use bevy::image::{
    ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor,
};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::{asset as bevy_asset, ecs as bevy_ecs, reflect as bevy_reflect};
use std::{collections::HashMap, sync::Arc};

#[derive(Asset, TypePath)]
pub struct CasterDitherBytes(Vec<u8>);
#[derive(Default, TypePath)]
pub struct CasterDitherLoader;
impl AssetLoader for CasterDitherLoader {
    type Asset = CasterDitherBytes;
    type Settings = ();
    type Error = std::io::Error;
    async fn load(
        &self,
        reader: &mut dyn Reader,
        _: &(),
        _: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        if bytes.len() != 256 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Source caster dither requires 256 bytes",
            ));
        }
        Ok(CasterDitherBytes(bytes))
    }
    fn extensions(&self) -> &[&str] {
        &["bin"]
    }
}

struct LoadingTexture {
    levels: Vec<Handle<Image>>,
    image: Option<Handle<Image>>,
}
#[derive(Resource)]
pub struct LightingAssets {
    pub record: Arc<LightingRecord>,
    textures: Vec<LoadingTexture>,
    texture_indices: HashMap<SourceId, usize>,
    pub cloud_mesh: Handle<Mesh>,
    dither_bytes: Handle<CasterDitherBytes>,
    pub caster_dither: Option<Handle<Image>>,
    pub shaders: LightingShaders,
    pub loaded: bool,
    pub texture_bytes: usize,
}
impl LightingAssets {
    pub fn texture(&self, source: &SourceId) -> Option<&Handle<Image>> {
        self.texture_indices
            .get(source)
            .and_then(|index| self.textures[*index].image.as_ref())
    }
}

pub fn load_lighting(commands: &mut Commands, server: &AssetServer, record: &LightingRecord) {
    let mut textures = Vec::with_capacity(record.textures.len());
    let mut texture_indices = HashMap::with_capacity(record.textures.len());
    for texture in &record.textures {
        let srgb = texture.color_space == TextureColorSpace::Srgb;
        let levels = texture
            .mips
            .iter()
            .map(|mip| {
                server
                    .load_builder()
                    .with_settings(move |settings: &mut ImageLoaderSettings| {
                        settings.is_srgb = srgb;
                        settings.asset_usage = RenderAssetUsages::MAIN_WORLD;
                    })
                    .load(mip.file.clone())
            })
            .collect();
        texture_indices.insert(texture.source.clone(), textures.len());
        textures.push(LoadingTexture {
            levels,
            image: None,
        });
    }
    let cloud_mesh = server
        .load_builder()
        .with_settings(|settings: &mut GltfLoaderSettings| {
            settings.convert_coordinates = Some(GltfConvertCoordinates {
                rotate_scene_entity: false,
                rotate_meshes: false,
            });
        })
        .load(
            GltfAssetLabel::Primitive {
                mesh: 0,
                primitive: 0,
            }
            .from_asset(record.sky.cloud_mesh.file.clone()),
        );
    commands.insert_resource(LightingAssets {
        record: Arc::new(record.clone()),
        textures,
        texture_indices,
        cloud_mesh,
        dither_bytes: server.load(record.caster_dither.file.clone()),
        caster_dither: None,
        shaders: LightingShaders {
            lighting: server.load("shaders/source_view_lighting.wgsl"),
            cloud: server.load("shaders/source_cloud.wgsl"),
            sky: server.load("shaders/source_render_sky.wgsl"),
            float_store: server.load("shaders/source_float_store.wgsl"),
            glass: server.load("shaders/source_glass.wgsl"),
            copy: server.load("shaders/source_copy.wgsl"),
            histogram: server.load("shaders/source_histogram.wgsl"),
            exposure: server.load("shaders/source_exposure.wgsl"),
            probe: server.load("shaders/source_probe.wgsl"),
            taa: server.load("shaders/source_taa.wgsl"),
            caster: server.load("shaders/source_caster.wgsl"),
            caster_math: server.load("shaders/source_caster_math.wgsl"),
        },
        loaded: false,
        texture_bytes: 0,
    });
}

pub fn lighting_asset_readiness(
    assets: Option<ResMut<LightingAssets>>,
    server: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut bytes: ResMut<Assets<CasterDitherBytes>>,
    mut lighting: ResMut<SourceLighting>,
    status: Res<SourceLightingStatus>,
) {
    let Some(mut assets) = assets else {
        return;
    };
    if assets.loaded || matches!(lighting.state, RenderingState::Failed(_)) {
        return;
    }
    if let Err(error) = prepare_assets(&mut assets, &server, &mut images, &mut bytes) {
        let failure = RenderingState::Failed(format!("Source lighting assets: {error}"));
        lighting.state = failure.clone();
        *status.0.lock().expect("source lighting status lock") = failure;
    }
}
fn prepare_assets(
    assets: &mut LightingAssets,
    server: &AssetServer,
    images: &mut Assets<Image>,
    bytes: &mut Assets<CasterDitherBytes>,
) -> Result<(), String> {
    for shader in assets.shaders.handles() {
        check_load(server, shader)?;
    }
    check_load(server, &assets.cloud_mesh)?;
    check_load(server, &assets.dither_bytes)?;
    for (texture, record) in assets.textures.iter_mut().zip(&assets.record.textures) {
        if texture.image.is_some() {
            continue;
        }
        for level in &texture.levels {
            check_load(server, level)?;
        }
        if !texture
            .levels
            .iter()
            .all(|level| images.contains(level.id()))
        {
            continue;
        }
        let (image, count) = super::source_images::assemble_mips(
            &texture.levels,
            &record.mips,
            &record.sampler,
            images,
        )?;
        texture.image = Some(images.add(image));
        texture.levels.clear();
        assets.texture_bytes += count;
    }
    if assets.caster_dither.is_none() {
        if let Some(bytes) = bytes.remove(assets.dither_bytes.id()) {
            // Native normalized A8 alpha is transported in target R8 red.
            let mut image = Image::new(
                Extent3d {
                    width: 4,
                    height: 4,
                    depth_or_array_layers: 16,
                },
                TextureDimension::D3,
                bytes.0,
                TextureFormat::R8Unorm,
                RenderAssetUsages::RENDER_WORLD,
            );
            image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                address_mode_u: ImageAddressMode::Repeat,
                address_mode_v: ImageAddressMode::Repeat,
                address_mode_w: ImageAddressMode::Repeat,
                mag_filter: ImageFilterMode::Nearest,
                min_filter: ImageFilterMode::Nearest,
                mipmap_filter: ImageFilterMode::Nearest,
                lod_max_clamp: 0.,
                ..default()
            });
            assets.caster_dither = Some(images.add(image));
            assets.texture_bytes += 256;
        }
    }
    assets.loaded = assets
        .textures
        .iter()
        .all(|texture| texture.image.is_some())
        && assets.caster_dither.is_some()
        && server.is_loaded_with_dependencies(assets.cloud_mesh.id())
        && assets
            .shaders
            .handles()
            .into_iter()
            .all(|shader| server.is_loaded_with_dependencies(shader.id()));
    // CPU loading alone never sets graph readiness or opens Start.
    Ok(())
}
fn check_load<A: Asset>(server: &AssetServer, handle: &Handle<A>) -> Result<(), String> {
    if let Some((load, _, recursive)) = server.get_load_states(handle.id()) {
        if let LoadState::Failed(error) = load {
            return Err(format!(
                "{}: {error}",
                server
                    .get_path(handle.id())
                    .map(|path| path.to_string())
                    .unwrap_or_else(|| format!("{:?}", handle.id()))
            ));
        }
        if let RecursiveDependencyLoadState::Failed(error) = recursive {
            return Err(format!(
                "{}: {error}",
                server
                    .get_path(handle.id())
                    .map(|path| path.to_string())
                    .unwrap_or_else(|| format!("{:?}", handle.id()))
            ));
        }
    }
    Ok(())
}

/// Handles and source settings cross worlds once. GPU preparation waits for their uploads.
#[derive(Resource)]
pub struct LightingRenderAssets {
    pub record: Arc<LightingRecord>,
    pub textures: HashMap<SourceId, Handle<Image>>,
    pub cloud_mesh: Handle<Mesh>,
    pub caster_dither: Handle<Image>,
    pub shaders: LightingShaders,
}
pub fn extract_lighting_assets(
    mut commands: Commands,
    assets: bevy::render::Extract<Option<Res<LightingAssets>>>,
    existing: Option<Res<LightingRenderAssets>>,
) {
    if existing.is_some() {
        return;
    }
    let Some(assets) = assets.as_ref() else {
        return;
    };
    if !assets.loaded {
        return;
    }
    commands.insert_resource(LightingRenderAssets {
        record: assets.record.clone(),
        textures: assets
            .texture_indices
            .iter()
            .map(|(source, index)| {
                (
                    source.clone(),
                    assets.textures[*index]
                        .image
                        .as_ref()
                        .expect("loaded lighting texture")
                        .clone(),
                )
            })
            .collect(),
        cloud_mesh: assets.cloud_mesh.clone(),
        caster_dither: assets
            .caster_dither
            .as_ref()
            .expect("loaded caster dither")
            .clone(),
        shaders: assets.shaders.clone(),
    });
}
#[derive(Clone)]
pub struct LightingShaders {
    pub lighting: Handle<Shader>,
    pub cloud: Handle<Shader>,
    pub sky: Handle<Shader>,
    pub float_store: Handle<Shader>,
    pub glass: Handle<Shader>,
    pub copy: Handle<Shader>,
    pub histogram: Handle<Shader>,
    pub exposure: Handle<Shader>,
    pub probe: Handle<Shader>,
    pub taa: Handle<Shader>,
    pub caster: Handle<Shader>,
    pub caster_math: Handle<Shader>,
}
impl LightingShaders {
    fn handles(&self) -> [&Handle<Shader>; 12] {
        [
            &self.lighting,
            &self.cloud,
            &self.sky,
            &self.float_store,
            &self.glass,
            &self.copy,
            &self.histogram,
            &self.exposure,
            &self.probe,
            &self.taa,
            &self.caster,
            &self.caster_math,
        ]
    }
}
