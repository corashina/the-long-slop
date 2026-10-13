pub mod grass;
pub mod impostor;
pub mod lod;
pub mod primary;
pub mod specular;
pub mod terrain;
mod world_texture_upload;
pub mod world_textures;
use crate::reference::{
    self as source, AssetUsage, MaterialRecord, ReferenceManifest, ShaderFamily, TextureColorSpace,
    TextureRecord,
};
use bevy::asset::{AssetId, LoadState, RenderAssetUsages};
use bevy::gltf::GltfMaterialExtras;
use bevy::image::{
    ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor,
};
use bevy::material::key::{ErasedMaterialPipelineKey, ErasedMeshPipelineKey};
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{
    MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline, MaterialPipeline,
    MaterialPipelineKey, MeshPipeline, MeshPipelineKey, PreparedMaterial, ViewKeyCache,
    alpha_mode_pipeline_key, base_specialize,
};
use bevy::prelude::*;
use bevy::render::{
    Render, RenderApp, RenderSystems,
    erased_render_asset::ErasedRenderAssets,
    extract_resource::{ExtractResource, ExtractResourcePlugin},
    mesh::RenderMesh,
    render_asset::RenderAssets,
    render_resource::*,
    renderer::RenderDevice,
};
use bevy::shader::{ShaderCacheError, ShaderDefVal, ShaderRef};
use bevy::{
    asset as bevy_asset, ecs as bevy_ecs, image as bevy_image, reflect as bevy_reflect,
    render as bevy_render,
};
use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

const SLOTS: [&str; 9] = source::MATERIAL_TEXTURE_SLOTS;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenderingState {
    Loading,
    Ready,
    Failed(String),
}

#[derive(Clone, Copy, Debug, ShaderType)]
pub struct SourceUniform {
    color: Vec4,
    emission: Vec4,
    specular: Vec4,
    factors: Vec4,
    scales: Vec4,
    flags: UVec4,
    banks: [UVec4; 3],
    layers: [UVec4; 3],
    uv: [Vec4; 9],
    sampling: [Vec4; 9],
    defaults: [Vec4; 9],
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct SurfaceKey {
    cull: u8,
    z_write: bool,
    z_test: u8,
    src: u8,
    dst: u8,
    reflected: bool,
}
impl From<&SourceSurface> for SurfaceKey {
    fn from(value: &SourceSurface) -> Self {
        value.state
    }
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
#[bind_group_data(SurfaceKey)]
pub struct SourceSurface {
    #[uniform(120)]
    data: SourceUniform,
    #[texture(121)]
    #[sampler(122)]
    bank6: Option<Handle<Image>>,
    state: SurfaceKey,
}
impl SourceSurface {
    pub fn set_color(&mut self, color: Color) {
        self.data.color = Vec4::from_array(color.to_linear().to_f32_array());
    }
}
fn blend_factor(value: u8) -> BlendFactor {
    match value {
        0 => BlendFactor::Zero,
        1 => BlendFactor::One,
        2 => BlendFactor::Dst,
        3 => BlendFactor::Src,
        4 => BlendFactor::OneMinusDst,
        5 => BlendFactor::SrcAlpha,
        6 => BlendFactor::OneMinusSrc,
        7 => BlendFactor::DstAlpha,
        8 => BlendFactor::OneMinusDstAlpha,
        9 => BlendFactor::SrcAlphaSaturated,
        _ => BlendFactor::OneMinusSrcAlpha,
    }
}

struct SourceTexture {
    failed: bool,
    eager: bool,
    users: usize,
    record: TextureRecord,
    levels: Vec<Handle<Image>>,
    image: Option<Handle<Image>>,
    label: String,
}
#[derive(Resource)]
pub struct SourceMaterials {
    pub world_textures: world_textures::WorldTextures,
    pub world_primary: primary::PrimaryOwnership,
    pub world_deferred: primary::DeferredAllocation,
    pub standard_records: HashMap<AssetId<StandardMaterial>, (source::AssetId, u32)>,
    records: Vec<MaterialRecord>,
    textures: Vec<SourceTexture>,
    texture_indices: HashMap<String, usize>,
    pub handles: HashMap<(source::AssetId, u32, bool, u32), Handle<SourceMaterial>>,
    shaders: [Handle<Shader>; 2],
    pub state: RenderingState,
    pub bound: bool,
    pub texture_bytes: usize,
}
#[derive(Clone)]
struct PipelineRequest {
    mesh: AssetId<Mesh>,
    material: AssetId<SourceMaterial>,
    label: String,
}
#[derive(Resource, Clone, ExtractResource)]
pub struct MaterialLoading(Arc<Mutex<LoadingData>>);
struct LoadingData {
    requests: Vec<PipelineRequest>,
    sealed: bool,
    state: RenderingState,
}
#[derive(Resource, Default)]
struct WarmedPipelines {
    ids: Vec<(CachedRenderPipelineId, String)>,
    queued: bool,
}

pub struct SourceMaterialPlugin;
impl Plugin for SourceMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractResourcePlugin::<MaterialLoading>::default(),
            MaterialPlugin::<terrain::TerrainMaterial>::default(),
            MaterialPlugin::<specular::SpecularMaterial>::default(),
            MaterialPlugin::<impostor::ImpostorMaterial>::default(),
            MaterialPlugin::<lod::LodCasterMaterial>::default(),
            MaterialPlugin::<grass::material::GrassMaterial>::default(),
            grass::writers::GrassDisplacementPlugin,
            world_texture_upload::WorldTextureUploadPlugin,
        ))
        .insert_resource(MaterialLoading(Arc::new(Mutex::new(LoadingData {
            requests: Vec::new(),
            sealed: false,
            state: RenderingState::Loading,
        }))))
        .add_systems(
            Update,
            (
                material_readiness,
                bind_authored_materials.after(bind_materials),
            )
                .chain(),
        );
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render.init_resource::<WarmedPipelines>().add_systems(
                Render,
                prewarm_materials.after(RenderSystems::PrepareBindGroups),
            );
        }
    }
}

pub fn load_materials(
    commands: &mut Commands,
    server: &AssetServer,
    reference: &ReferenceManifest,
) {
    let eager_assets: HashSet<_> = reference
        .assets
        .iter()
        .filter(|asset| asset.usage != AssetUsage::WorldPrototype)
        .map(|asset| asset.id)
        .collect();
    let records: Vec<_> = reference
        .rendering
        .materials
        .iter()
        .filter(|record| eager_assets.contains(&record.asset))
        .cloned()
        .collect();
    let mut textures = Vec::new();
    let mut texture_indices = HashMap::new();
    for record in &records {
        for texture in &record.properties.textures {
            if texture_indices.contains_key(&texture.file) {
                continue;
            }
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
            texture_indices.insert(texture.file.clone(), textures.len());
            textures.push(SourceTexture {
                failed: false,
                eager: true,
                users: 0,
                record: texture.clone(),
                levels,
                image: None,
                label: format!(
                    "material {}:{} {}",
                    record.source.file, record.source.path_id, texture.slot
                ),
            });
        }
    }
    commands.insert_resource(SourceMaterials {
        world_textures: world_textures::WorldTextures::default(),
        world_primary: primary::PrimaryOwnership::default(),
        world_deferred: primary::DeferredAllocation::default(),
        standard_records: HashMap::new(),
        records,
        textures,
        texture_indices,
        handles: HashMap::new(),
        shaders: [
            server.load("shaders/source_surface.wgsl"),
            server.load("shaders/source_mesh.wgsl"),
        ],
        state: RenderingState::Loading,
        bound: false,
        texture_bytes: 0,
    });
}

pub fn material_readiness(
    server: Res<AssetServer>,
    mut source: ResMut<SourceMaterials>,
    mut images: ResMut<Assets<Image>>,
    loading: Res<MaterialLoading>,
) {
    if source.state != RenderingState::Loading {
        return;
    }
    for shader in &source.shaders {
        if let LoadState::Failed(error) = server.load_state(shader.id()) {
            source.state = RenderingState::Failed(format!("Source material shader: {error}"));
            return;
        }
    }
    match assemble_textures(&mut source.textures, &server, &mut images, true) {
        Ok(bytes) => source.texture_bytes += bytes,
        Err(error) => {
            source.state = RenderingState::Failed(error.0);
            return;
        }
    }
    if source.bound {
        source.state = loading.0.lock().unwrap().state.clone();
    }
}
fn assemble_textures(
    textures: &mut [SourceTexture],
    server: &AssetServer,
    images: &mut Assets<Image>,
    eager: bool,
) -> Result<usize, source::ReferenceError> {
    for texture in textures
        .iter_mut()
        .filter(|texture| texture.eager == eager && texture.image.is_none() && !texture.failed)
    {
        for (level, handle) in texture.levels.iter().enumerate() {
            if let LoadState::Failed(error) = server.load_state(handle.id()) {
                // Dynamic owners drain terminal loads in prepare_world_primary before reporting.
                if !eager {
                    continue;
                }
                return Err(source::ReferenceError(format!(
                    "{} mip {level}: {error}",
                    texture.label
                )));
            }
        }
        if texture.levels.len() != texture.record.mips.len()
            || !texture
                .levels
                .iter()
                .all(|handle| images.contains(handle.id()))
        {
            continue;
        }
        let byte_count = texture
            .record
            .mips
            .iter()
            .map(|mip| mip.width as usize * mip.height as usize * 4)
            .sum();
        // Validate every mip before removing any admitted source image.
        for (handle, mip) in texture.levels.iter().zip(&texture.record.mips) {
            let image = images.get(handle.id()).unwrap();
            let expected = mip.width as usize * mip.height as usize * 4;
            if image.width() != mip.width
                || image.height() != mip.height
                || image.data.as_ref().map(Vec::len) != Some(expected)
            {
                return Err(source::ReferenceError(format!(
                    "{}: invalid decoded mip {}",
                    texture.label, mip.file
                )));
            }
        }
        let mut bytes = Vec::with_capacity(byte_count);
        let mut assembled = None;
        for handle in &texture.levels {
            let mut image = images.remove(handle.id()).unwrap();
            bytes.append(image.data.as_mut().unwrap());
            if assembled.is_none() {
                assembled = Some(image);
            }
        }
        let mut image =
            assembled.ok_or_else(|| source::ReferenceError("Missing authored mip chain".into()))?;
        image.data = Some(bytes);
        image.texture_descriptor.mip_level_count = texture.record.sampler.mip_count;
        image.asset_usage = RenderAssetUsages::RENDER_WORLD;
        let settings = &texture.record.sampler;
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
        texture.image = Some(images.add(image));
        texture.levels.clear();
        return Ok(byte_count);
    }
    Ok(0)
}
fn wrap(value: u8) -> ImageAddressMode {
    match value {
        0 => ImageAddressMode::Repeat,
        2 => ImageAddressMode::MirrorRepeat,
        _ => ImageAddressMode::ClampToEdge,
    }
}

#[derive(serde::Deserialize)]
struct MaterialIdentity {
    asset: source::AssetId,
    gltf_material: u32,
}
pub fn bind_authored_materials(
    mut commands: Commands,
    mut source: ResMut<SourceMaterials>,
    mut materials: ResMut<Assets<SourceMaterial>>,
    visuals: Res<super::reference::ReferenceVisuals>,
    primitives: Query<
        (
            Entity,
            &GltfMaterialExtras,
            &Mesh3d,
            &GlobalTransform,
            Option<&bevy::camera::visibility::RenderLayers>,
        ),
        Without<super::sky::SkyMesh>,
    >,
    loading: Res<MaterialLoading>,
) {
    if source.bound
        || source.state != RenderingState::Loading
        || !visuals.bound
        || source
            .textures
            .iter()
            .any(|texture| texture.image.is_none())
    {
        return;
    }
    let mut shared = HashMap::<String, Handle<SourceMaterial>>::new();
    let mut requests = Vec::new();
    let mut variants = HashSet::new();
    for (entity, extras, mesh, transform, layers) in &primitives {
        let mask = layers.map_or(1, |layers| {
            layers.iter().fold(0u32, |bits, layer| {
                bits | 1u32.checked_shl(layer as u32).unwrap_or(0)
            })
        });
        let identity: MaterialIdentity = match serde_json::from_str(&extras.value) {
            Ok(value) => value,
            Err(error) => {
                source.state = RenderingState::Failed(format!("Source material identity: {error}"));
                return;
            }
        };
        let Some(record) = source.records.iter().find(|record| {
            record.asset == identity.asset && record.gltf_material == identity.gltf_material
        }) else {
            source.state = RenderingState::Failed(format!(
                "Missing material {}:{}",
                identity.asset.0, identity.gltf_material
            ));
            return;
        };
        let reflected = transform.affine().matrix3.determinant() < 0.;
        let key = (identity.asset, identity.gltf_material, reflected, mask);
        let label = format!(
            "material {}:{} (asset {}, index {})",
            record.source.file, record.source.path_id, identity.asset.0, identity.gltf_material
        );
        let handle = if let Some(handle) = source.handles.get(&key) {
            handle.clone()
        } else {
            let mut identity = serde_json::to_value(record).expect("validated record");
            identity.as_object_mut().unwrap().remove("asset");
            identity.as_object_mut().unwrap().remove("gltf_material");
            identity["reflected"] = serde_json::json!(reflected);
            identity["receiver_mask"] = serde_json::json!(mask);
            let identity = identity.to_string();
            let handle = shared
                .entry(identity)
                .or_insert_with(|| materials.add(make_material(record, reflected, mask, &source)))
                .clone();
            source.handles.insert(key, handle.clone());
            handle
        };
        commands
            .entity(entity)
            .remove::<MeshMaterial3d<StandardMaterial>>()
            .insert(MeshMaterial3d(handle.clone()));
        if variants.insert((mesh.0.id(), handle.id())) {
            requests.push(PipelineRequest {
                mesh: mesh.0.id(),
                material: handle.id(),
                label,
            });
        }
    }
    if requests.is_empty() {
        source.state = RenderingState::Failed("No source material primitives".into());
        return;
    }
    info!(
        "Source materials: {} records, {} handles, {} textures, {} mip bytes, {} mesh/material pairs",
        source.records.len(),
        shared.len(),
        source.textures.len(),
        source.texture_bytes,
        requests.len()
    );
    source.bound = true;
    let mut pending = loading.0.lock().unwrap();
    pending.requests = requests;
    pending.sealed = true;
}

fn make_material(
    record: &MaterialRecord,
    reflected: bool,
    mask: u32,
    source: &SourceMaterials,
) -> SourceMaterial {
    if record.shader_family == ShaderFamily::AmplifyRope {
        // Retain the master rope surface. Source vertex sway remains deferred.
        return SourceMaterial {
            base: StandardMaterial {
                base_color: Color::WHITE,
                metallic: 0.,
                perceptual_roughness: 0.5,
                fog_enabled: false,
                ..default()
            },
            extension: SourceExtension {
                lod_fade: 1.,
                receiver_mask: mask,
                surface: None,
            },
        };
    }
    let floats = &record.properties.floats;
    let factor = |name: &str, default: f32| floats.get(name).copied().unwrap_or(default);
    let keyword = |name: &str| record.keywords.iter().any(|value| value == name);
    let mut flags = 0u32;
    for (bit, name) in [
        "_NORMALMAP",
        "_DETAIL_MULX2",
        "_METALLICGLOSSMAP",
        "_EMISSION",
        "_PARALLAXMAP",
        "_SMOOTHNESS_TEXTURE_ALBEDO_CHANNEL_A",
        "_SPECULARHIGHLIGHTS_OFF",
        "_GLOSSYREFLECTIONS_OFF",
        "_ALPHAPREMULTIPLY_ON",
        "_ALPHATEST_ON",
        "_ALPHABLEND_ON",
    ]
    .iter()
    .enumerate()
    {
        if keyword(name) {
            flags |= 1 << bit;
        }
    }
    let mut data = SourceUniform {
        color: Vec4::from_array(
            Color::srgba(
                record.color[0],
                record.color[1],
                record.color[2],
                record.color[3],
            )
            .to_linear()
            .to_f32_array(),
        ),
        emission: Vec4::ZERO,
        specular: Vec4::ZERO,
        factors: Vec4::new(
            factor("_Metallic", 0.),
            factor("_Glossiness", 0.5),
            factor("_GlossMapScale", 1.),
            factor("_Cutoff", 0.5),
        ),
        scales: Vec4::new(
            factor("_BumpScale", 1.),
            factor("_DetailNormalMapScale", 1.),
            factor("_OcclusionStrength", 1.),
            factor("_Parallax", 0.),
        ),
        flags: UVec4::new(
            flags,
            0,
            match record.shader_family {
                ShaderFamily::Standard => 0,
                ShaderFamily::UnlitColor => 1,
                ShaderFamily::StainedGlass => 2,
                ShaderFamily::AmplifyRope => unreachable!("rope uses its master surface"),
            },
            factor("_UVSec", 0.) as u32,
        ),
        banks: [UVec4::ZERO; 3],
        layers: [UVec4::ZERO; 3],
        uv: [Vec4::new(1., 1., 0., 0.); 9],
        sampling: [Vec4::ZERO; 9],
        defaults: [Vec4::ONE; 9],
    };
    if let Some(color) = record.properties.colors.get("_EmissionColor") {
        data.emission = Vec4::from_array(
            Color::srgba(color[0], color[1], color[2], color[3])
                .to_linear()
                .to_f32_array(),
        );
    }
    let mapping = record
        .texture_banks()
        .expect("validated texture bank count");
    let mut textures: [Option<Handle<Image>>; 7] = default();
    for (index, slot) in SLOTS.iter().enumerate() {
        if let Some(texture) = record
            .properties
            .textures
            .iter()
            .find(|entry| entry.slot == *slot)
        {
            let bank = usize::from(mapping[index].expect("mapped source texture"));
            data.banks[index / 4][index % 4] = bank as u32;
            textures[bank] = source.textures[source.texture_indices[&texture.file]]
                .image
                .clone();
            data.flags.y |= 1 << index;
            data.uv[index] = Vec4::new(
                texture.scale[0],
                texture.scale[1],
                texture.offset[0],
                texture.offset[1],
            );
            data.sampling[index] = Vec4::new(
                texture.uv_channel as f32,
                texture.sampler.mip_bias,
                (texture.sampler.wrap_u == 3) as u8 as f32,
                (texture.sampler.wrap_v == 3) as u8 as f32,
            );
        } else if let Some(default) = record.properties.defaults.get(*slot) {
            data.defaults[index] = match default {
                source::TextureDefault::White => Vec4::ONE,
                source::TextureDefault::Grey => Vec4::splat(0.5),
                source::TextureDefault::Black => Vec4::new(0., 0., 0., 1.),
                source::TextureDefault::Bump => Vec4::new(0.5, 0.5, 1., 1.),
            };
            if *default == source::TextureDefault::Grey && matches!(index, 0 | 4 | 6) {
                data.defaults[index] = Vec4::new(0.21404114, 0.21404114, 0.21404114, 0.5);
            }
        }
    }
    let alpha_mode = match record.state.alpha_mode {
        source::AlphaMode::Opaque => AlphaMode::Opaque,
        source::AlphaMode::Mask => AlphaMode::Mask(data.factors.w),
        source::AlphaMode::Blend | source::AlphaMode::Refract => AlphaMode::Blend,
        source::AlphaMode::Premultiply => AlphaMode::Premultiplied,
    };
    let main = record
        .properties
        .textures
        .iter()
        .find(|texture| texture.slot == "_MainTex");
    SourceMaterial {
        base: StandardMaterial {
            base_color_channel: main.map_or(bevy::mesh::UvChannel::Uv0, |texture| {
                if texture.uv_channel == 0 {
                    bevy::mesh::UvChannel::Uv0
                } else {
                    bevy::mesh::UvChannel::Uv1
                }
            }),
            uv_transform: main.map_or(bevy::math::Affine2::IDENTITY, |texture| {
                bevy::math::Affine2::from_scale_angle_translation(
                    Vec2::from_array(texture.scale),
                    0.,
                    Vec2::from_array(texture.offset),
                )
            }),
            base_color: Color::srgba(
                record.color[0],
                record.color[1],
                record.color[2],
                record.color[3],
            ),
            cull_mode: match record.state.cull {
                0 => None,
                1 => Some(Face::Front),
                _ => Some(Face::Back),
            },
            base_color_texture: textures[0].clone(),
            emissive_texture: textures[1].clone(),
            metallic_roughness_texture: textures[2].clone(),
            occlusion_texture: textures[3].clone(),
            normal_map_texture: textures[4].clone(),
            depth_map: textures[5].clone(),
            alpha_mode,
            unlit: record.shader_family == ShaderFamily::UnlitColor,
            ..default()
        },
        extension: SourceExtension {
            lod_fade: 1.,
            receiver_mask: mask,
            surface: Some(SourceSurface {
                data,
                bank6: textures[6].clone(),
                state: SurfaceKey {
                    cull: record.state.cull,
                    z_write: record.state.z_write,
                    z_test: record.state.z_test,
                    src: record.state.src_blend,
                    dst: record.state.dst_blend,
                    reflected,
                },
            }),
        },
    }
}

fn prewarm_materials(world: &mut World) {
    let Some(loading) = world.get_resource::<MaterialLoading>().cloned() else {
        return;
    };
    let mut pending = loading.0.lock().unwrap();
    if !pending.sealed || pending.state != RenderingState::Loading {
        return;
    }
    if !world.resource::<WarmedPipelines>().queued {
        if pending.requests.iter().any(|request| {
            world
                .resource::<RenderAssets<RenderMesh>>()
                .get(request.mesh)
                .is_none()
                || world
                    .resource::<ErasedRenderAssets<PreparedMaterial>>()
                    .get(request.material)
                    .is_none()
        }) {
            return;
        }
        let view_keys: Vec<_> = world.resource::<ViewKeyCache>().values().copied().collect();
        if view_keys.is_empty() {
            return;
        }
        let mut work = Vec::new();
        for request in &pending.requests {
            let Some(mesh) = world
                .resource::<RenderAssets<RenderMesh>>()
                .get(request.mesh)
            else {
                return;
            };
            let Some(material) = world
                .resource::<ErasedRenderAssets<PreparedMaterial>>()
                .get(request.material)
            else {
                return;
            };
            for view_key in &view_keys {
                let bits: MeshPipelineKey = material.properties.mesh_pipeline_key_bits.downcast();
                let mesh_key = *view_key
                    | MeshPipelineKey::from_bits_retain(mesh.key_bits.bits())
                    | bits
                    | alpha_mode_pipeline_key(
                        material.properties.alpha_mode,
                        &Msaa::from_samples(view_key.msaa_samples()),
                    );
                work.push((
                    ErasedMaterialPipelineKey {
                        type_id: TypeId::of::<SourceMaterial>(),
                        mesh_key: ErasedMeshPipelineKey::new(mesh_key),
                        material_key: material.properties.material_key.clone(),
                    },
                    mesh.layout.clone(),
                    material.properties.clone(),
                    request.label.clone(),
                ));
            }
        }
        let mut ids = Vec::new();
        let mut seen = HashSet::new();
        for (key, layout, properties, label) in work {
            let mesh_key: MeshPipelineKey = key.mesh_key.downcast();
            let mut descriptor = match world
                .resource::<MeshPipeline>()
                .specialize(mesh_key, &layout)
            {
                Ok(descriptor) => descriptor,
                Err(error) => {
                    pending.state = RenderingState::Failed(format!("{label}: {error}"));
                    return;
                }
            };
            descriptor
                .layout
                .push(properties.material_layout.as_ref().unwrap().clone());
            let limits = world.resource::<RenderDevice>().limits();
            // Source vertex outputs occupy locations 0..8, with conditional gaps.
            let varying_locations = 9;
            if varying_locations > limits.max_inter_stage_shader_variables {
                pending.state = RenderingState::Failed(format!(
                    "{label}: source shader needs {varying_locations} varying locations; device limit is {}",
                    limits.max_inter_stage_shader_variables
                ));
                return;
            }
            let mut textures = 0;
            let mut samplers = 0;
            let mut uniforms = 0;
            for layout in &descriptor.layout {
                for entry in &layout.entries {
                    if entry.visibility.contains(ShaderStages::FRAGMENT) {
                        match entry.ty {
                            BindingType::Texture { .. } => {
                                textures += entry.count.map_or(1, |count| count.get())
                            }
                            BindingType::Sampler(_) => samplers += 1,
                            BindingType::Buffer {
                                ty: BufferBindingType::Uniform,
                                ..
                            } => uniforms += 1,
                            _ => {}
                        }
                    }
                }
            }
            if textures > limits.max_sampled_textures_per_shader_stage
                || samplers > limits.max_samplers_per_shader_stage
                || uniforms > limits.max_uniform_buffers_per_shader_stage
            {
                pending.state = RenderingState::Failed(format!(
                    "{label}: WebGL2 layout requires {textures} textures, {samplers} samplers, {uniforms} uniforms; limits are {}, {}, {}",
                    limits.max_sampled_textures_per_shader_stage,
                    limits.max_samplers_per_shader_stage,
                    limits.max_uniform_buffers_per_shader_stage
                ));
                return;
            }
            let id = match base_specialize(world, key, &layout, &properties) {
                Ok(id) => id,
                Err(error) => {
                    pending.state = RenderingState::Failed(format!("{label}: {error}"));
                    return;
                }
            };
            if seen.insert(id) {
                info!(
                    "Source pipeline: {textures} fragment textures, {samplers} samplers, {uniforms} uniforms, {varying_locations} varying locations (upper bound; device limit {})",
                    limits.max_inter_stage_shader_variables
                );
                ids.push((id, label));
            }
        }
        let mut warmed = world.resource_mut::<WarmedPipelines>();
        warmed.ids = ids;
        warmed.queued = true;
    }
    let cache = world.resource::<PipelineCache>();
    for (id, label) in &world.resource::<WarmedPipelines>().ids {
        match cache.get_render_pipeline_state(*id) {
            CachedPipelineState::Ok(_) => {}
            CachedPipelineState::Err(
                ShaderCacheError::ShaderNotLoaded(_)
                | ShaderCacheError::ShaderImportNotYetAvailable,
            ) => return,
            CachedPipelineState::Err(error) => {
                pending.state = RenderingState::Failed(format!("{label}: shader pipeline {error}"));
                return;
            }
            _ => return,
        }
    }
    info!(
        "Source material pipelines ready: {}",
        world.resource::<WarmedPipelines>().ids.len()
    );
    pending.state = RenderingState::Ready;
}

impl Default for SourceSurface {
    fn default() -> Self {
        Self {
            data: SourceUniform {
                color: Vec4::ONE,
                emission: Vec4::ZERO,
                specular: Vec4::ZERO,
                factors: Vec4::ZERO,
                scales: Vec4::ZERO,
                flags: UVec4::ZERO,
                banks: [UVec4::ZERO; 3],
                layers: [UVec4::ZERO; 3],
                uv: [Vec4::ZERO; 9],
                sampling: [Vec4::ZERO; 9],
                defaults: [Vec4::ONE; 9],
            },
            bank6: None,
            state: SurfaceKey {
                cull: 2,
                z_write: true,
                z_test: 4,
                src: 1,
                dst: 0,
                reflected: false,
            },
        }
    }
}

// Shared receiver bindings. Meshes retain StandardMaterial surface and alpha data.
use super::lighting::LightingGpu;
use bevy::ecs::system::SystemParamItem;

/// Forward source surfaces need the six base maps, not Bevy transmission maps.
#[derive(Asset, TypePath, Clone, Debug, Default)]
pub struct SourceMaterial {
    pub base: StandardMaterial,
    pub extension: SourceExtension,
}
impl AsBindGroup for SourceMaterial {
    type Data = (
        <StandardMaterial as AsBindGroup>::Data,
        <SourceExtension as AsBindGroup>::Data,
    );
    type Param = (
        <StandardMaterial as AsBindGroup>::Param,
        <SourceExtension as AsBindGroup>::Param,
    );
    fn label() -> &'static str {
        "source material"
    }
    fn bind_group_data(&self) -> Self::Data {
        (
            self.base.bind_group_data(),
            self.extension.bind_group_data(),
        )
    }
    fn unprepared_bind_group(
        &self,
        layout: &BindGroupLayout,
        device: &RenderDevice,
        params: &mut SystemParamItem<Self::Param>,
        _: bool,
    ) -> Result<UnpreparedBindGroup, AsBindGroupError> {
        let mut prepared = self
            .base
            .unprepared_bind_group(layout, device, &mut params.0, true)?;
        prepared.bindings.0.retain(|(binding, _)| *binding <= 12);
        let extension =
            self.extension
                .unprepared_bind_group(layout, device, &mut params.1, true)?;
        prepared.bindings.0.extend(extension.bindings.0);
        Ok(prepared)
    }
    fn bind_group_layout_entries(device: &RenderDevice, _: bool) -> Vec<BindGroupLayoutEntry> {
        let mut entries = StandardMaterial::bind_group_layout_entries(device, true);
        entries.retain(|entry| entry.binding <= 12);
        entries.extend(SourceExtension::bind_group_layout_entries(device, true));
        entries
    }
}
impl Material for SourceMaterial {
    fn vertex_shader() -> ShaderRef {
        SourceExtension::vertex_shader()
    }
    fn fragment_shader() -> ShaderRef {
        SourceExtension::fragment_shader()
    }
    fn alpha_mode(&self) -> AlphaMode {
        self.base.alpha_mode
    }
    fn opaque_render_method(&self) -> bevy::material::OpaqueRendererMethod {
        self.base.opaque_render_method()
    }
    fn depth_bias(&self) -> f32 {
        self.base.depth_bias
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
    fn specialize(
        pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        StandardMaterial::specialize(
            pipeline,
            descriptor,
            layout,
            MaterialPipelineKey::<StandardMaterial> {
                mesh_key: key.mesh_key,
                bind_group_data: key.bind_group_data.0,
            },
        )?;
        // The generated material path also uses zero transmission. Remove the
        // matching shader feature so it cannot read the omitted bindings.
        let unused = |definition: &ShaderDefVal| {
            let name = match definition {
                ShaderDefVal::Bool(name, _)
                | ShaderDefVal::Int(name, _)
                | ShaderDefVal::UInt(name, _) => name,
            };
            name != "PBR_TRANSMISSION_TEXTURES_SUPPORTED"
                && name != "PBR_MULTI_LAYER_MATERIAL_TEXTURES_SUPPORTED"
                && name != "PBR_ANISOTROPY_TEXTURE_SUPPORTED"
                && name != "PBR_SPECULAR_TEXTURES_SUPPORTED"
        };
        descriptor.vertex.shader_defs.retain(unused);
        if let Some(fragment) = &mut descriptor.fragment {
            fragment.shader_defs.retain(unused);
        }
        SourceExtension::specialize(
            &MaterialExtensionPipeline {
                mesh_pipeline: pipeline.mesh_pipeline.clone(),
            },
            descriptor,
            layout,
            MaterialExtensionKey {
                mesh_key: key.mesh_key,
                bind_group_data: key.bind_group_data.1,
            },
        )
    }
}

#[derive(Asset, TypePath, Clone, Debug)]
pub struct SourceExtension {
    pub receiver_mask: u32,
    pub lod_fade: f32,
    pub surface: Option<SourceSurface>,
}
impl Default for SourceExtension {
    fn default() -> Self {
        Self {
            receiver_mask: 0,
            lod_fade: 1.,
            surface: None,
        }
    }
}
impl MaterialExtension for SourceExtension {
    fn vertex_shader() -> ShaderRef {
        "shaders/source_mesh.wgsl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "shaders/source_surface.wgsl".into()
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
    fn specialize(
        _: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _: &MeshVertexBufferLayoutRef,
        key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let Some(state) = key.bind_group_data else {
            return Ok(());
        };
        if let Some(fragment) = &mut descriptor.fragment {
            fragment
                .shader_defs
                .push(ShaderDefVal::Bool("SOURCE_AUTHORED".into(), true));
        }
        descriptor.primitive.cull_mode = match state.cull {
            0 => None,
            1 => Some(Face::Front),
            _ => Some(Face::Back),
        };
        if state.reflected {
            descriptor.primitive.front_face = FrontFace::Cw;
        }
        if let Some(depth) = &mut descriptor.depth_stencil {
            depth.depth_write_enabled = Some(state.z_write);
            // Source uses conventional depth. Bevy uses reverse depth.
            depth.depth_compare = Some(match state.z_test {
                0 | 8 => CompareFunction::Always,
                1 => CompareFunction::Never,
                2 => CompareFunction::Greater,
                3 => CompareFunction::Equal,
                4 => CompareFunction::GreaterEqual,
                5 => CompareFunction::Less,
                6 => CompareFunction::NotEqual,
                _ => CompareFunction::LessEqual,
            });
        }
        if let Some(fragment) = &mut descriptor.fragment {
            for target in fragment.targets.iter_mut().flatten() {
                let component = BlendComponent {
                    src_factor: blend_factor(state.src),
                    dst_factor: blend_factor(state.dst),
                    operation: BlendOperation::Add,
                };
                target.blend = Some(BlendState {
                    color: component,
                    alpha: component,
                });
            }
            fragment
                .shader_defs
                .push(ShaderDefVal::Bool("SOURCE_SURFACE".into(), true));
        }
        Ok(())
    }
}
impl AsBindGroup for SourceExtension {
    type Data = Option<SurfaceKey>;
    type Param = (SRes<LightingGpu>, <SourceSurface as AsBindGroup>::Param);
    fn label() -> &'static str {
        "source lighting"
    }
    fn bind_group_data(&self) -> Self::Data {
        self.surface.as_ref().map(|s| s.state)
    }
    fn unprepared_bind_group(
        &self,
        layout: &BindGroupLayout,
        device: &RenderDevice,
        params: &mut SystemParamItem<Self::Param>,
        _: bool,
    ) -> Result<UnpreparedBindGroup, AsBindGroupError> {
        let gpu = &mut params.0;
        let cookies = gpu
            .cookies
            .as_ref()
            .ok_or(AsBindGroupError::RetryNextUpdate)?;
        let mut result = UnpreparedBindGroup {
            bindings: BindingResources(vec![
                (
                    100,
                    OwnedBindingResource::Buffer(
                        gpu.uniform
                            .buffer()
                            .ok_or(AsBindGroupError::RetryNextUpdate)?
                            .clone(),
                    ),
                ),
                (
                    101,
                    OwnedBindingResource::TextureView(
                        TextureViewDimension::D2,
                        gpu.atlas_view.clone(),
                    ),
                ),
                (
                    102,
                    OwnedBindingResource::Sampler(
                        SamplerBindingType::Comparison,
                        gpu.shadow_sampler.clone(),
                    ),
                ),
                (
                    113,
                    OwnedBindingResource::TextureView(
                        TextureViewDimension::D2,
                        gpu.atlas_view.clone(),
                    ),
                ),
                (
                    112,
                    OwnedBindingResource::Sampler(
                        SamplerBindingType::Comparison,
                        gpu.hard_shadow_sampler.clone(),
                    ),
                ),
                (
                    103,
                    OwnedBindingResource::TextureView(
                        TextureViewDimension::D2,
                        cookies[2].0.clone(),
                    ),
                ),
                (
                    104,
                    OwnedBindingResource::Sampler(
                        SamplerBindingType::Filtering,
                        cookies[2].1.clone(),
                    ),
                ),
                (
                    107,
                    OwnedBindingResource::TextureView(
                        TextureViewDimension::D2,
                        cookies[1].0.clone(),
                    ),
                ),
                (
                    108,
                    OwnedBindingResource::Sampler(
                        SamplerBindingType::Filtering,
                        cookies[1].1.clone(),
                    ),
                ),
                (
                    109,
                    OwnedBindingResource::TextureView(
                        TextureViewDimension::D2,
                        cookies[0].0.clone(),
                    ),
                ),
                (
                    110,
                    OwnedBindingResource::Sampler(
                        SamplerBindingType::Filtering,
                        cookies[0].1.clone(),
                    ),
                ),
                (
                    111,
                    OwnedBindingResource::Sampler(
                        SamplerBindingType::Filtering,
                        gpu.cookie_sampler.clone(),
                    ),
                ),
                (
                    105,
                    OwnedBindingResource::TextureView(
                        TextureViewDimension::D2,
                        gpu.attenuation_view.clone(),
                    ),
                ),
                (
                    106,
                    OwnedBindingResource::Buffer(
                        device.create_buffer_with_data(&BufferInitDescriptor {
                            label: Some("source receiver mask"),
                            contents: &[
                                self.receiver_mask.to_le_bytes(),
                                self.lod_fade.to_le_bytes(),
                                [0; 4],
                                [0; 4],
                            ]
                            .concat(),
                            usage: BufferUsages::UNIFORM,
                        }),
                    ),
                ),
            ]),
        };
        let surface = self.surface.clone().unwrap_or_default();
        let extra = surface.unprepared_bind_group(layout, device, &mut params.1, false)?;
        result.bindings.0.extend(extra.bindings.0);
        Ok(result)
    }
    fn bind_group_layout_entries(device: &RenderDevice, force: bool) -> Vec<BindGroupLayoutEntry> {
        use binding_types::*;
        let mut entries = BindGroupLayoutEntries::with_indices(
            ShaderStages::FRAGMENT,
            (
                (
                    100,
                    uniform_buffer::<super::lighting::LightingUniform>(false),
                ),
                (101, texture_2d(TextureSampleType::Depth)),
                (102, sampler(SamplerBindingType::Comparison)),
                (
                    103,
                    texture_2d(TextureSampleType::Float { filterable: true }),
                ),
                (104, sampler(SamplerBindingType::Filtering)),
                (
                    105,
                    texture_2d(TextureSampleType::Float { filterable: true }),
                ),
                (106, uniform_buffer::<UVec4>(false)),
                (
                    107,
                    texture_2d(TextureSampleType::Float { filterable: true }),
                ),
                (108, sampler(SamplerBindingType::Filtering)),
                (
                    109,
                    texture_2d(TextureSampleType::Float { filterable: true }),
                ),
                (110, sampler(SamplerBindingType::Filtering)),
                (111, sampler(SamplerBindingType::Filtering)),
                (112, sampler(SamplerBindingType::Comparison)),
                (113, texture_2d(TextureSampleType::Depth)),
            ),
        )
        .to_vec();
        entries.extend(SourceSurface::bind_group_layout_entries(device, force));
        entries
    }
}
use bevy::ecs::system::lifetimeless::SRes;

#[derive(Component)]
pub struct SourceCaster;
#[derive(Component, Clone, Copy)]
pub struct SourceShadowMode(pub u8);

#[derive(Resource, Default)]
pub struct MaterialCache {
    pub variants: std::collections::HashMap<
        (AssetId<StandardMaterial>, bool, bool),
        Handle<StandardMaterial>,
    >,
    pub converted:
        std::collections::HashMap<(AssetId<StandardMaterial>, u32), Handle<SourceMaterial>>,
}
pub fn bind_materials(
    mut commands: Commands,
    visuals: Res<super::reference::ReferenceVisuals>,
    mut originals: ResMut<Assets<StandardMaterial>>,
    mut materials: ResMut<Assets<SourceMaterial>>,
    mut cache: ResMut<MaterialCache>,
    imported_nodes: Query<&bevy::gltf::GltfExtras>,
    authored: Res<SourceMaterials>,
    query: Query<
        (
            Entity,
            &Mesh3d,
            &MeshMaterial3d<StandardMaterial>,
            Option<&bevy::camera::visibility::RenderLayers>,
            Option<&SourceShadowMode>,
            Option<&ChildOf>,
            Option<&GltfMaterialExtras>,
            &GlobalTransform,
        ),
        (Without<SourceCaster>, Without<super::sky::SkyMesh>),
    >,
) {
    if !visuals.bound || !authored.textures_ready() {
        return;
    }
    for (entity, mesh, handle, layers, shadow_mode, parent, extras, global) in &query {
        if shadow_mode.is_none()
            && parent.is_some_and(|parent| imported_nodes.contains(parent.parent()))
        {
            continue;
        }
        let mask = layers.map_or(1, |layers| {
            layers.iter().fold(0, |mask, layer| {
                mask | 1u32.checked_shl(layer as u32).unwrap_or(0)
            })
        });
        let Some(mut base) = originals.get(&handle.0).cloned() else {
            continue;
        };
        if let Some(extras) = extras {
            let identity: MaterialIdentity =
                serde_json::from_str(&extras.value).expect("validated material identity");
            base = authored_caster_material((identity.asset, identity.gltf_material), &authored);
            let record = authored
                .records
                .iter()
                .find(|r| r.asset == identity.asset && r.gltf_material == identity.gltf_material)
                .expect("validated material");
            base.base_color = Color::srgba(
                record.color[0],
                record.color[1],
                record.color[2],
                record.color[3],
            );
            base.cull_mode = match record.state.cull {
                0 => None,
                1 => Some(Face::Front),
                _ => Some(Face::Back),
            };
            if global.affine().matrix3.determinant() < 0. {
                base.cull_mode = base.cull_mode.map(|face| match face {
                    Face::Front => Face::Back,
                    Face::Back => Face::Front,
                });
            }
            originals
                .insert(handle.0.id(), base.clone())
                .expect("loaded source caster material");
        }
        let mode = shadow_mode.map_or(1, |mode| mode.0);
        if mode != 0 {
            let caster_material = if mode == 2 {
                cache
                    .variants
                    .entry((handle.0.id(), false, true))
                    .or_insert_with(|| {
                        let mut material = base.clone();
                        material.cull_mode = None;
                        originals.add(material)
                    })
                    .clone()
            } else {
                handle.0.clone()
            };
            let layers = (0..32)
                .filter(|layer| mask & (1 << layer) != 0)
                .map(|layer| layer + 32)
                .collect::<bevy::camera::visibility::RenderLayers>();
            commands.spawn((
                SourceCaster,
                mesh.clone(),
                MeshMaterial3d(caster_material),
                layers,
                Transform::default(),
                Visibility::Inherited,
                ChildOf(entity),
            ));
        }
        if mode == 3 {
            commands
                .entity(entity)
                .insert(bevy::camera::visibility::RenderLayers::none());
        }
        let material = cache
            .converted
            .entry((handle.0.id(), mask))
            .or_insert_with(|| {
                if let Some(extras) = extras {
                    let identity: MaterialIdentity = serde_json::from_str(&extras.value)
                        .expect("validated source material identity");
                    materials.add(authored_material(
                        (identity.asset, identity.gltf_material),
                        global.affine().matrix3.determinant() < 0.,
                        mask,
                        &authored,
                    ))
                } else {
                    let mut base = base.clone();
                    base.fog_enabled = false;
                    materials.add(SourceMaterial {
                        base,
                        extension: SourceExtension {
                            lod_fade: 1.,
                            receiver_mask: mask,
                            surface: None,
                        },
                    })
                }
            })
            .clone();
        commands
            .entity(entity)
            .remove::<MeshMaterial3d<StandardMaterial>>()
            .insert((MeshMaterial3d(material), bevy::light::NotShadowCaster));
    }
}

pub struct LampReceiverChoice {
    entity: Entity,
    material: Handle<SourceMaterial>,
    casters: Vec<(Entity, Handle<StandardMaterial>)>,
}
pub struct LampMaterialChoice {
    key: (crate::reference::AssetId, u32),
    receivers: Vec<LampReceiverChoice>,
}

/// Prepare source selections and actual mesh pipeline variants during loading.
pub(crate) fn prepare_lamp_materials(
    mut commands: Commands,
    reference: Res<crate::app::RuntimeReference>,
    drive: Res<crate::app::Drive>,
    instruments: Res<super::instruments::InstrumentVisuals>,
    mut source: ResMut<super::lighting::SourceLighting>,
    mut originals: ResMut<Assets<StandardMaterial>>,
    mut materials: ResMut<Assets<SourceMaterial>>,
    mut cache: ResMut<MaterialCache>,
    authored_materials: Res<SourceMaterials>,
    receivers: Query<
        (
            &Mesh3d,
            &MeshMaterial3d<SourceMaterial>,
            &GlobalTransform,
            Option<&SourceShadowMode>,
            Option<&Children>,
        ),
        Without<SourceCaster>,
    >,
    casters: Query<(), With<SourceCaster>>,
) {
    if !authored_materials.textures_ready()
        || !source.bound
        || !instruments.prepared
        || source.materials_prepared
        || !source.materials_ready
        || source.road_lamps.recipe.is_none()
    {
        return;
    }
    if source.lamp_materials.iter().any(|target| {
        target
            .receivers
            .iter()
            .any(|entity| !receivers.contains(*entity))
    }) {
        return;
    }
    let source = &mut *source;
    let far = Transform::from_translation(Vec3::splat(1_000_000.));
    for target in &mut source.lamp_materials {
        let output = &drive.runtime.lighting.material_outputs()[target.output_index];
        let mut keys = Vec::new();
        for state in reference
            .0
            .lighting
            .controllers
            .iter()
            .flat_map(|c| &c.states)
        {
            if !state.renderers.contains(&output.renderer) {
                continue;
            }
            for material in [&state.on_material, &state.off_material]
                .into_iter()
                .flatten()
            {
                let key = (material.asset, material.gltf_material);
                if !keys.contains(&key) {
                    keys.push(key);
                }
            }
        }
        let dashboard = &reference.0.lighting.high_beam;
        if dashboard.node == output.renderer {
            for material in [&dashboard.on, &dashboard.off] {
                let key = (material.asset, material.gltf_material);
                if !keys.contains(&key) {
                    keys.push(key);
                }
            }
        }
        for key in keys {
            let authored = source
                .materials
                .iter()
                .find(|m| (m.asset, m.index) == key)
                .expect("validated lamp material");
            let authored_base = authored_caster_material(key, &authored_materials);
            originals
                .insert(authored.handle.id(), authored_base.clone())
                .expect("loaded lamp caster material");
            let mut choice = LampMaterialChoice {
                key,
                receivers: Vec::with_capacity(target.receivers.len()),
            };
            for &entity in &target.receivers {
                let (mesh, receiver, global, shadow_mode, children) =
                    receivers.get(entity).unwrap();
                let mask = materials.get(&receiver.0).unwrap().extension.receiver_mask;
                let inverted = global.affine().matrix3.determinant() < 0.;
                let mut base = authored_base.clone();
                if inverted {
                    base.cull_mode = base.cull_mode.map(|face| match face {
                        Face::Front => Face::Back,
                        Face::Back => Face::Front,
                    });
                }
                let surface = if inverted {
                    cache
                        .variants
                        .entry((authored.handle.id(), true, false))
                        .or_insert_with(|| originals.add(base.clone()))
                        .clone()
                } else {
                    authored.handle.clone()
                };
                let caster = if shadow_mode.is_some_and(|mode| mode.0 == 2) {
                    cache
                        .variants
                        .entry((authored.handle.id(), inverted, true))
                        .or_insert_with(|| {
                            let mut material = base.clone();
                            material.cull_mode = None;
                            originals.add(material)
                        })
                        .clone()
                } else {
                    surface.clone()
                };
                let material = cache
                    .converted
                    .entry((surface.id(), mask))
                    .or_insert_with(|| {
                        materials.add(authored_material(key, inverted, mask, &authored_materials))
                    })
                    .clone();
                let warm = commands
                    .spawn((
                        mesh.clone(),
                        MeshMaterial3d(material.clone()),
                        far,
                        Visibility::Visible,
                        bevy::light::NotShadowCaster,
                        bevy::camera::visibility::NoFrustumCulling,
                    ))
                    .id();
                source.warmups.push((warm, false));
                let mut binding = LampReceiverChoice {
                    entity,
                    material,
                    casters: Vec::new(),
                };
                if let Some(children) = children {
                    for child in children.iter().filter(|child| casters.contains(*child)) {
                        binding.casters.push((child, caster.clone()));
                    }
                }
                if !binding.casters.is_empty() {
                    let warm = commands
                        .spawn((
                            SourceCaster,
                            mesh.clone(),
                            MeshMaterial3d(caster),
                            far,
                            Visibility::Visible,
                            (32..64).collect::<bevy::camera::visibility::RenderLayers>(),
                            bevy::camera::visibility::NoFrustumCulling,
                        ))
                        .id();
                    source.warmups.push((warm, true));
                }
                choice.receivers.push(binding);
            }
            target.choices.push(choice);
        }
    }
    source.materials_prepared = true;
    info!(
        "Source lamp cache: {} targets, {} choices, {} materials, {} warmup meshes",
        source.lamp_materials.len(),
        source
            .lamp_materials
            .iter()
            .map(|t| t.choices.len())
            .sum::<usize>(),
        cache.converted.len(),
        source.warmups.len()
    );
}

/// Select cached handles. Gameplay does not allocate materials or mesh variants.
pub(crate) fn sync_lamp_materials(
    mut commands: Commands,
    drive: Res<crate::app::Drive>,
    mut source: ResMut<super::lighting::SourceLighting>,
) {
    if !source.materials_prepared {
        return;
    }
    for target in &mut source.lamp_materials {
        let output = &drive.runtime.lighting.material_outputs()[target.output_index];
        let key = (output.asset, output.material_index);
        if target.current == Some(key) {
            continue;
        }
        let choice = target
            .choices
            .iter()
            .find(|choice| choice.key == key)
            .expect("prepared source material choice");
        for receiver in &choice.receivers {
            commands
                .entity(receiver.entity)
                .insert(MeshMaterial3d(receiver.material.clone()));
            for (entity, material) in &receiver.casters {
                commands
                    .entity(*entity)
                    .insert(MeshMaterial3d(material.clone()));
            }
        }
        target.current = Some(key);
    }
}

pub fn authored_material(
    key: (source::AssetId, u32),
    reflected: bool,
    mask: u32,
    source: &SourceMaterials,
) -> SourceMaterial {
    let record = source
        .records
        .iter()
        .find(|r| (r.asset, r.gltf_material) == key)
        .expect("validated authored material choice");
    make_material(record, reflected, mask, source)
}
impl SourceMaterials {
    pub fn textures_ready(&self) -> bool {
        self.textures.iter().all(|t| t.image.is_some())
    }
}

pub fn authored_caster_material(
    key: (source::AssetId, u32),
    source: &SourceMaterials,
) -> StandardMaterial {
    let mut base = authored_material(key, false, 0, source).base;
    base.emissive_texture = None;
    base.metallic_roughness_texture = None;
    base.occlusion_texture = None;
    base.normal_map_texture = None;
    base.depth_map = None;
    base
}
