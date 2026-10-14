mod generated;
pub use generated::{GeneratedWorldMaterial, generated_mesh};

use crate::reference::{
    self as source, MaterialRecord, RenderingRecord, ShaderFamily, TextureColorSpace, TextureRecord,
};
use bevy::asset::{AssetId, LoadState, RenderAssetUsages, UntypedAssetId};
use bevy::gltf::{GltfExtras, GltfMaterialExtras};
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
    factors: Vec4,
    scales: Vec4,
    flags: UVec4,
    banks: [UVec4; 3],
    uv: [Vec4; 9],
    sampling: [Vec4; 9],
    defaults: [Vec4; 9],
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct SurfaceKey {
    cull: u8,
    z_write: bool,
    z_test: u8,
    src: u8,
    dst: u8,
    reflected: bool,
    standard: bool,
    unlit: bool,
    transparent: bool,
    glass: bool,
    glass_shader: Option<Handle<Shader>>,
}
impl From<&SourceSurface> for SurfaceKey {
    fn from(value: &SourceSurface) -> Self {
        value.state.clone()
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
    record: TextureRecord,
    levels: Vec<Handle<Image>>,
    image: Option<Handle<Image>>,
    label: String,
}
#[derive(Resource)]
pub struct SourceMaterials {
    pub standard_records: HashMap<AssetId<StandardMaterial>, (source::AssetId, u32)>,
    records: Vec<MaterialRecord>,
    textures: Vec<SourceTexture>,
    texture_indices: HashMap<String, usize>,
    pub handles: HashMap<(source::AssetId, u32, bool, u32), Handle<SourceMaterial>>,
    shaders: [Handle<Shader>; 3],
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
#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
pub(crate) struct MaterialPrewarmAudit {
    pub(crate) state: u8,
    pub(crate) sealed: bool,
    pub(crate) request_count: u32,
    pub(crate) missing_mesh_count: u32,
    pub(crate) missing_material_count: u32,
    pub(crate) first_missing_mesh_request: Option<u32>,
    pub(crate) first_missing_material_request: Option<u32>,
    pub(crate) warmed_queued: bool,
    pub(crate) warmed_id_count: u32,
    pub(crate) ready_pipeline_count: u32,
    pub(crate) view_key_count: u32,
    pub(crate) cookies_present: bool,
}
impl MaterialLoading {
    pub fn is_ready(&self) -> bool {
        self.0.lock().unwrap().state == RenderingState::Ready
    }
    pub fn source_shadow_capacity(&self) -> Option<usize> {
        let data = self.0.lock().unwrap();
        data.sealed.then_some(data.shadow_capacity)
    }
    pub fn transparent_pass_capacity(&self) -> Option<usize> {
        let data = self.0.lock().unwrap();
        data.sealed.then_some(data.transparent_pass_capacity)
    }

    /// State: 0=Loading, 1=Ready, 2=Failed. Request indices start at zero.
    pub(crate) fn prewarm_audit(&self, world: &World) -> MaterialPrewarmAudit {
        let mut pending = self.0.lock().unwrap();
        let state = match pending.state {
            RenderingState::Loading => 0,
            RenderingState::Ready => 1,
            RenderingState::Failed(_) => 2,
        };
        // Requests are assigned as a complete Vec before sealing. Its retained
        // storage identifies that assignment without scanning request contents.
        let requests = (pending.requests.as_ptr() as usize, pending.requests.len());
        if state == 0 {
            pending.terminal_prewarm_audit = None;
        } else if let Some((previous_requests, audit)) = pending.terminal_prewarm_audit {
            if previous_requests == requests
                && audit.state == state
                && audit.sealed == pending.sealed
            {
                return audit;
            }
        }
        let meshes = world.get_resource::<RenderAssets<RenderMesh>>();
        let materials = world.get_resource::<ErasedRenderAssets<PreparedMaterial>>();
        let warmed = world.get_resource::<WarmedPipelines>();
        let cache = world.get_resource::<PipelineCache>();
        let mut audit = MaterialPrewarmAudit {
            state,
            sealed: pending.sealed,
            request_count: pending.requests.len() as u32,
            warmed_queued: warmed.is_some_and(|warmed| warmed.queued),
            warmed_id_count: warmed.map_or(0, |warmed| warmed.ids.len() as u32),
            ready_pipeline_count: warmed.map_or(0, |warmed| {
                warmed
                    .ids
                    .iter()
                    .filter(|(id, _)| {
                        cache.is_some_and(|cache| cache.get_render_pipeline(*id).is_some())
                    })
                    .count() as u32
            }),
            view_key_count: world
                .get_resource::<ViewKeyCache>()
                .map_or(0, |keys| keys.len() as u32),
            cookies_present: world
                .get_resource::<LightingGpu>()
                .is_some_and(|gpu| gpu.cookies.is_some()),
            ..Default::default()
        };
        for (index, request) in pending.requests.iter().enumerate() {
            if meshes.is_none_or(|meshes| meshes.get(request.mesh).is_none()) {
                audit.missing_mesh_count += 1;
                audit.first_missing_mesh_request.get_or_insert(index as u32);
            }
            if materials.is_none_or(|materials| materials.get(request.material).is_none()) {
                audit.missing_material_count += 1;
                audit
                    .first_missing_material_request
                    .get_or_insert(index as u32);
            }
        }
        if state != 0 {
            pending.terminal_prewarm_audit = Some((requests, audit));
        }
        audit
    }
}
#[derive(Component)]
pub struct SourceMaterialWarmup;

struct LoadingData {
    requests: Vec<PipelineRequest>,
    sealed: bool,
    state: RenderingState,
    shadow_capacity: usize,
    transparent_pass_capacity: usize,
    // A terminal prewarm boundary remains fixed until requests, seal, or state change.
    terminal_prewarm_audit: Option<((usize, usize), MaterialPrewarmAudit)>,
}
#[derive(Resource, Default)]
pub(crate) struct WarmedPipelines {
    pub(crate) ids: Vec<(CachedRenderPipelineId, String)>,
    queued: bool,
}

pub struct SourceMaterialPlugin;
impl Plugin for SourceMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractResourcePlugin::<MaterialLoading>::default(),
            bevy::render::extract_component::ExtractComponentPlugin::<
                super::transparent::SourceSortData,
            >::default(),
        ))
        .insert_resource(MaterialLoading(Arc::new(Mutex::new(LoadingData {
            requests: Vec::new(),
            sealed: false,
            state: RenderingState::Loading,
            shadow_capacity: 0,
            transparent_pass_capacity: 0,
            terminal_prewarm_audit: None,
        }))))
        .init_resource::<super::renderers::RendererMeshes>()
        .init_resource::<generated::GeneratedCasterMaterials>()
        .add_systems(
            Update,
            generated::prepare_local_casters.before(bind_materials),
        )
        .add_systems(
            Update,
            super::renderers::bind_renderer_meshes
                .before(bind_materials)
                .before(bind_authored_materials),
        )
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

pub fn load_materials(commands: &mut Commands, server: &AssetServer, rendering: &RenderingRecord) {
    let mut textures = Vec::new();
    let mut texture_indices = HashMap::new();
    for record in &rendering.materials {
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
        standard_records: HashMap::new(),
        records: rendering.materials.clone(),
        textures,
        texture_indices,
        handles: HashMap::new(),
        shaders: [
            server.load("shaders/source_surface.wgsl"),
            server.load("shaders/source_mesh.wgsl"),
            server.load("shaders/source_glass.wgsl"),
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
    let mut added_bytes = 0;
    for texture in &mut source.textures {
        if texture.image.is_some() {
            continue;
        }
        for (level, handle) in texture.levels.iter().enumerate() {
            if let LoadState::Failed(error) = server.load_state(handle.id()) {
                source.state =
                    RenderingState::Failed(format!("{} mip {level}: {error}", texture.label));
                return;
            }
        }
        if !texture
            .levels
            .iter()
            .all(|handle| images.contains(handle.id()))
        {
            continue;
        }
        let byte_count: usize = texture
            .record
            .mips
            .iter()
            .map(|mip| mip.width as usize * mip.height as usize * 4)
            .sum();
        let mut bytes = Vec::with_capacity(byte_count);
        let mut assembled = None;
        for (handle, mip) in texture.levels.iter().zip(&texture.record.mips) {
            let mut image = images.remove(handle.id()).expect("checked source mip");
            let expected = mip.width as usize * mip.height as usize * 4;
            if image.width() != mip.width
                || image.height() != mip.height
                || image.data.as_ref().map(Vec::len) != Some(expected)
            {
                source.state = RenderingState::Failed(format!(
                    "{}: invalid decoded mip {}",
                    texture.label, mip.file
                ));
                return;
            }
            bytes.append(image.data.as_mut().expect("checked pixels"));
            if assembled.is_none() {
                assembled = Some(image);
            }
        }
        let mut image = assembled.expect("validated mip chain");
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
        added_bytes += byte_count;
    }
    source.texture_bytes += added_bytes;
    if source.bound {
        source.state = loading.0.lock().unwrap().state.clone();
    }
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
    renderer_meshes: Res<super::renderers::RendererMeshes>,
    generated: Query<(
        &GeneratedWorldMaterial,
        &Mesh3d,
        &MeshMaterial3d<SourceMaterial>,
    )>,
    master: Res<super::lighting::SourceLighting>,
    primitives: Query<
        (
            Entity,
            &GltfMaterialExtras,
            &Mesh3d,
            &GlobalTransform,
            Option<&bevy::camera::visibility::RenderLayers>,
            &GltfExtras,
            &super::renderers::SourceRenderer,
        ),
        (Without<super::sky::SkyMesh>, Without<SourceMaterialWarmup>),
    >,
    loading: Res<MaterialLoading>,
) {
    if source.bound
        || source.state != RenderingState::Loading
        || !visuals.bound
        || !renderer_meshes.bound
        || master.road_lamps.recipe.is_none()
        || !master.stations.ready()
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
    let mut transparent_pass_capacity = 0;
    let mut sort_identities = HashMap::<String, u32>::new();
    let mut shader_identities = HashMap::<String, i32>::new();
    for (entity, extras, mesh, transform, layers, primitive, renderer) in &primitives {
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
        if matches!(
            record.state.alpha_mode,
            source::AlphaMode::Blend | source::AlphaMode::Premultiply | source::AlphaMode::Refract
        ) {
            transparent_pass_capacity += 1 + usize::from(matches!(
                record.shader_family,
                ShaderFamily::Standard | ShaderFamily::StainedGlass
            ));
            #[derive(serde::Deserialize)]
            struct PrimitiveIdentity {
                submesh: u16,
            }
            let slot = match serde_json::from_str::<PrimitiveIdentity>(&primitive.value) {
                Ok(value) if value.submesh <= 0x7fff => value.submesh << 1,
                _ => {
                    source.state = RenderingState::Failed(
                        "Source transparent material slot is invalid".into(),
                    );
                    return;
                }
            };
            let mut identity = serde_json::to_value(record).expect("validated material");
            identity.as_object_mut().unwrap().remove("asset");
            identity.as_object_mut().unwrap().remove("gltf_material");
            let next = sort_identities.len() as u32 + 1;
            let runtime_material = *sort_identities.entry(identity.to_string()).or_insert(next);
            let next = shader_identities.len() as i32 + 1;
            let runtime_shader = *shader_identities
                .entry(format!("{:?}", record.shader_family))
                .or_insert(next);
            let layer = u32::from((i32::from(renderer.0.sorting_layer) + 0x8000) as u16);
            let order = u32::from((i32::from(renderer.0.sorting_order) + 0x8000) as u16);
            commands.entity(entity).insert((
                bevy::render::batching::NoAutomaticBatching,
                super::transparent::SourceSortData {
                    material_identity: runtime_material,
                    glass_source: if record.shader_family == ShaderFamily::StainedGlass {
                        record.source.path_id
                    } else {
                        0
                    },
                    material_key: crate::reference::sorting::material_sort_key(
                        runtime_material,
                        record.enable_instancing,
                        0,
                    ),
                    shader_identity: runtime_shader,
                    source_material_index: slot,
                    layer_and_order: (layer << 16) | order,
                    queue: if record.state.render_queue < 0 {
                        3000
                    } else {
                        record.state.render_queue as i16
                    },
                    additive: record.shader_family == ShaderFamily::Standard,
                    glass: record.shader_family == ShaderFamily::StainedGlass,
                },
            ));
        }
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
    for (identity, mesh, material) in &generated {
        if variants.insert((mesh.0.id(), material.0.id())) {
            requests.push(PipelineRequest {
                mesh: mesh.0.id(),
                material: material.0.id(),
                label: format!("generated world {identity:?}"),
            });
        }
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
    pending.shadow_capacity = primitives.iter().count()
        + super::world::source_shadow_capacity()
        + master.road_lamps.source_mesh_capacity()
        + master.stations.source_mesh_capacity();
    pending.transparent_pass_capacity = transparent_pass_capacity;
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
        return generated::from_base(
            StandardMaterial {
                base_color: Color::WHITE,
                metallic: 0.,
                perceptual_roughness: 0.5,
                fog_enabled: false,
                ..default()
            },
            mask,
            reflected,
        );
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
        factors: Vec4::new(
            factor("_Metallic", 0.),
            factor("_Glossiness", 0.5),
            factor("_GlossMapScale", 1.),
            factor("_Cutoff", 0.5),
        ),
        scales: Vec4::new(
            if record.shader_family == ShaderFamily::StainedGlass {
                factor("_BumpAmt", 0.)
            } else {
                factor("_BumpScale", 1.)
            },
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
        mirror: None,
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
            receiver_mask: mask,
            surface: SourceSurface {
                data,
                bank6: textures[6].clone(),
                state: SurfaceKey {
                    cull: record.state.cull,
                    z_write: record.state.z_write,
                    z_test: record.state.z_test,
                    src: record.state.src_blend,
                    dst: record.state.dst_blend,
                    reflected,
                    standard: record.shader_family == ShaderFamily::Standard,
                    unlit: record.shader_family == ShaderFamily::UnlitColor,
                    transparent: matches!(
                        record.state.alpha_mode,
                        source::AlphaMode::Blend
                            | source::AlphaMode::Premultiply
                            | source::AlphaMode::Refract
                    ),
                    glass: record.shader_family == ShaderFamily::StainedGlass,
                    glass_shader: (record.shader_family == ShaderFamily::StainedGlass)
                        .then(|| source.shaders[2].clone()),
                },
            },
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
            descriptor.layout[0] = super::source_lighting::source_camera_layout();
            descriptor.layout[1] = super::source_lighting::surface_lighting_layout();
            let limits = world.resource::<RenderDevice>().limits();
            // Source outputs occupy locations 0..11, with conditional gaps.
            let varying_locations = 12;
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
                factors: Vec4::ZERO,
                scales: Vec4::ZERO,
                flags: UVec4::ZERO,
                banks: [UVec4::ZERO; 3],
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
                standard: true,
                unlit: false,
                transparent: false,
                glass: false,
                glass_shader: None,
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
    pub(crate) mirror: Option<(u64, usize, bool)>,
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
        SRes<super::mirrors::MirrorGpuStatus>,
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
        if let Some((instance, owner, live)) = self.mirror {
            params.2.record(instance, owner, live, &prepared.bindings);
        }
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
                && name != "VISIBILITY_RANGE_DITHER"
                && name != "LOAD_PREPASS_NORMALS"
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

#[derive(Asset, TypePath, Clone, Debug, Default)]
pub struct SourceExtension {
    pub receiver_mask: u32,
    pub surface: SourceSurface,
}
impl SourceExtension {
    pub fn has_caster(&self) -> bool {
        self.surface.state.standard
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
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let state = key.bind_group_data;
        descriptor.layout[0] = super::source_lighting::source_camera_layout();
        descriptor.layout[1] = if state.glass {
            super::stained_glass::glass_view_layout()
        } else {
            super::source_lighting::surface_lighting_layout()
        };
        if state.glass {
            let shader = state
                .glass_shader
                .as_ref()
                .expect("source glass shader handle");
            descriptor.vertex.shader = shader.clone();
            if let Some(fragment) = &mut descriptor.fragment {
                fragment.shader = shader.clone();
            }
        }
        let mut attributes = Vec::new();
        for (location, attribute) in [
            Mesh::ATTRIBUTE_POSITION,
            Mesh::ATTRIBUTE_NORMAL,
            Mesh::ATTRIBUTE_UV_0,
            Mesh::ATTRIBUTE_UV_1,
            Mesh::ATTRIBUTE_TANGENT,
            Mesh::ATTRIBUTE_COLOR,
        ]
        .into_iter()
        .enumerate()
        {
            if layout.0.contains(attribute) {
                attributes.push(attribute.at_shader_location(location as u32));
            }
        }
        attributes.push(super::renderers::SOURCE_BOUNDS_CENTER.at_shader_location(8));
        attributes.push(super::renderers::SOURCE_BOUNDS_EXTENTS.at_shader_location(9));
        descriptor.vertex.buffers = vec![layout.0.get_layout(&attributes)?];
        for (name, value) in [
            ("SOURCE_STANDARD", state.standard),
            ("SOURCE_UNLIT", state.unlit),
            ("SOURCE_TRANSPARENT", state.transparent),
            ("SOURCE_OBJECT_REFLECTED", state.reflected),
        ] {
            if value || name == "SOURCE_OBJECT_REFLECTED" {
                descriptor
                    .vertex
                    .shader_defs
                    .push(ShaderDefVal::Bool(name.into(), value));
                if let Some(fragment) = &mut descriptor.fragment {
                    fragment
                        .shader_defs
                        .push(ShaderDefVal::Bool(name.into(), value));
                }
            }
        }

        descriptor.primitive.cull_mode = match state.cull {
            0 => None,
            1 => Some(Face::Front),
            _ => Some(Face::Back),
        };
        let view_inverted = key.mesh_key.contains(MeshPipelineKey::INVERT_CULLING);
        descriptor.primitive.front_face = if state.reflected ^ view_inverted {
            FrontFace::Cw
        } else {
            FrontFace::Ccw
        };
        if let Some(depth) = &mut descriptor.depth_stencil {
            depth.format = super::render_targets::DEPTH_FORMAT;
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
            if state.src == 1 && state.dst == 0 {
                fragment
                    .shader_defs
                    .push(ShaderDefVal::Bool("SOURCE_REPLACEMENT".into(), true));
            }
            for target in fragment.targets.iter_mut().flatten() {
                let component = BlendComponent {
                    src_factor: blend_factor(state.src),
                    dst_factor: blend_factor(state.dst),
                    operation: BlendOperation::Add,
                };
                target.blend = if state.src == 1 && state.dst == 0 {
                    None
                } else {
                    Some(BlendState {
                        color: component,
                        alpha: component,
                    })
                };
            }
            fragment
                .shader_defs
                .push(ShaderDefVal::Bool("SOURCE_SURFACE".into(), true));
        }
        Ok(())
    }
}
impl AsBindGroup for SourceExtension {
    type Data = SurfaceKey;
    type Param = (SRes<LightingGpu>, <SourceSurface as AsBindGroup>::Param);
    fn label() -> &'static str {
        "source lighting"
    }
    fn bind_group_data(&self) -> Self::Data {
        self.surface.state.clone()
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
                            contents: &[self.receiver_mask.to_le_bytes(), [0; 4], [0; 4], [0; 4]]
                                .concat(),
                            usage: BufferUsages::UNIFORM,
                        }),
                    ),
                ),
            ]),
        };
        let surface = &self.surface;
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
    renderer_meshes: Res<super::renderers::RendererMeshes>,
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
    if !visuals.bound || !renderer_meshes.bound || !authored.textures_ready() {
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
                    materials.add(generated::from_base(
                        base,
                        mask,
                        global.affine().matrix3.determinant() < 0.,
                    ))
                }
            })
            .clone();
        commands
            .entity(entity)
            .remove::<MeshMaterial3d<StandardMaterial>>()
            .insert((MeshMaterial3d(material), bevy::light::NotShadowCaster));
    }
}

#[derive(Clone)]
pub struct LampReceiverChoice {
    entity: Entity,
    material: Handle<SourceMaterial>,
    casters: Vec<(Entity, Handle<StandardMaterial>)>,
}
#[derive(Clone)]
pub struct LampMaterialChoice {
    key: (crate::reference::AssetId, u32),
    receivers: Vec<LampReceiverChoice>,
}

impl LampMaterialChoice {
    pub(crate) fn validate_assets(
        &self,
        materials: &Assets<SourceMaterial>,
        casters: &Assets<StandardMaterial>,
    ) -> Result<(), crate::save::SaveError> {
        for receiver in &self.receivers {
            if !materials.contains(receiver.material.id()) {
                return Err(crate::save::SaveError::MissingAssets(
                    "lamp material choice".into(),
                ));
            }
            for (_, handle) in &receiver.casters {
                if !casters.contains(handle.id()) {
                    return Err(crate::save::SaveError::MissingAssets(
                        "lamp caster material choice".into(),
                    ));
                }
            }
        }
        Ok(())
    }
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
        || !source.stations.ready()
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
        for dashboard in &reference.0.lighting.high_beams {
            if dashboard.node == output.renderer {
                for material in [&dashboard.on, &dashboard.off] {
                    let key = (material.asset, material.gltf_material);
                    if !keys.contains(&key) {
                        keys.push(key);
                    }
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
                        SourceMaterialWarmup,
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

/// Both source slots use one texture bank and its single sampler.
pub fn make_mirror_material(
    record: &MaterialRecord,
    source: &SourceMaterials,
    target: Handle<Image>,
    reflected: bool,
    mask: u32,
) -> SourceMaterial {
    let mut material = make_material(record, reflected, mask, source);
    let surface = &mut material.extension.surface;
    for slot in [0usize, 4] {
        surface.data.flags.y |= 1 << slot;
        surface.data.banks[slot / 4][slot % 4] = 0;
    }
    material.base.base_color_texture = Some(target);
    material
}

pub(crate) fn warm_mirror_material(
    world: &World,
    mesh: AssetId<Mesh>,
    material: AssetId<SourceMaterial>,
) {
    let loading = world.resource::<MaterialLoading>();
    let mut pending = loading.0.lock().unwrap();
    pending.requests.push(PipelineRequest {
        mesh,
        material,
        label: "source mirror surface".into(),
    });
}

pub(crate) fn release_mirror_warmup(
    world: &World,
    live: AssetId<SourceMaterial>,
    disabled: AssetId<SourceMaterial>,
) {
    world
        .resource::<MaterialLoading>()
        .0
        .lock()
        .unwrap()
        .requests
        .retain(|request| request.material != live && request.material != disabled);
}

/// Retain all source choices, including hidden lamps, for capture loading.
pub(crate) fn mirror_warmup_pairs(world: &World, pairs: &mut Vec<(AssetId<Mesh>, UntypedAssetId)>) {
    let loading = world.resource::<MaterialLoading>().0.lock().unwrap();
    pairs.extend(
        loading
            .requests
            .iter()
            .map(|r| (r.mesh, r.material.untyped())),
    );
    for target in &world
        .resource::<super::lighting::SourceLighting>()
        .lamp_materials
    {
        for choice in &target.choices {
            for receiver in &choice.receivers {
                if let Some(mesh) = world.get::<Mesh3d>(receiver.entity) {
                    pairs.push((mesh.0.id(), receiver.material.id().untyped()));
                }
            }
        }
    }
}
