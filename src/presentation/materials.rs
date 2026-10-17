mod generated;

pub mod grass;
pub mod impostor;
pub mod lod;
pub mod primary;
pub mod specular;
pub mod terrain;
use crate::world::upload_receipt;
mod world_texture_upload;
pub mod world_textures;
use super::png_mip_chain::{SourcePngMipChain, SourcePngMipChainSettings};
use crate::reference::{
    self as source, AssetUsage, MaterialRecord, ReferenceManifest, ShaderFamily, TextureRecord,
};
use bevy::asset::{AssetId, LoadState, UntypedAssetId};
use bevy::gltf::{GltfExtras, GltfMaterialExtras};
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
    renderer::{RenderDevice, RenderQueue},
};
use bevy::shader::{ShaderCacheError, ShaderDefVal, ShaderRef};
use bevy::{
    asset as bevy_asset, ecs as bevy_ecs, image as bevy_image, reflect as bevy_reflect,
    render as bevy_render,
};
use std::{
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

#[derive(Clone)]
struct SourceTexture {
    owner: Arc<()>,
    upload: upload_receipt::UploadReceipt,
    failed: bool,
    persistent: bool,
    users: usize,
    record: Arc<TextureRecord>,
    chain: Option<Handle<SourcePngMipChain>>,
    input_staging: usize,
    pixel_bytes: usize,
    cpu_reservation: usize,
    image: Option<Handle<Image>>,
    label: String,
}
#[derive(Resource, Clone)]
pub struct SourceMaterials {
    pub preparation: super::gpu_prepare::GpuPreparation,
    pub world_textures: world_textures::WorldTextures,
    pub world_primary: primary::PrimaryOwnership,
    pub world_deferred: primary::DeferredAllocation,
    pub standard_records: HashMap<AssetId<StandardMaterial>, (source::AssetId, u32)>,
    records: Vec<Arc<MaterialRecord>>,
    sort_data: HashMap<(source::AssetId, u32), super::transparent::SourceSortData>,
    textures: Vec<SourceTexture>,
    texture_indices: HashMap<String, usize>,
    pub handles: HashMap<(source::AssetId, u32, bool, u32), Handle<SourceMaterial>>,
    pub(crate) shaders: [Handle<Shader>; 3],
    pub(crate) fixed_admitted: bool,
    pub state: RenderingState,
    pub bound: bool,
    pub texture_bytes: usize,
}
#[derive(Clone)]
struct PipelineRequest {
    mesh: AssetId<Mesh>,
    material: UntypedAssetId,
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

impl SourceMaterials {
    pub(crate) fn sort_data(
        &self,
        identity: (source::AssetId, u32),
        renderer: &source::render_lighting::RendererRecord,
        slot: u32,
    ) -> Result<Option<super::transparent::SourceSortData>, source::ReferenceError> {
        let Some(mut sort) = self.sort_data.get(&identity).copied() else {
            return Ok(None);
        };
        if slot > 0x7fff {
            return Err(source::ReferenceError(
                "Source transparent material slot is invalid".into(),
            ));
        }
        sort.source_material_index = (slot as u16) << 1;
        let layer = u32::from((i32::from(renderer.sorting_layer) + 0x8000) as u16);
        let order = u32::from((i32::from(renderer.sorting_order) + 0x8000) as u16);
        sort.layer_and_order = (layer << 16) | order;
        Ok(Some(sort))
    }
    pub(crate) fn owns_image(&self, id: AssetId<Image>) -> bool {
        self.textures.iter().any(|t| {
            !t.persistent
                && Arc::strong_count(&t.owner) == 1
                && t.image.as_ref().is_some_and(|h| h.id() == id)
        }) || self.world_textures.owns_image(id)
    }
}
pub struct SourceMaterialPlugin;
impl Plugin for SourceMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractResourcePlugin::<MaterialLoading>::default(),
            ExtractResourcePlugin::<RetainedColors>::default(),
            bevy::render::extract_component::ExtractComponentPlugin::<
                super::transparent::SourceSortData,
            >::default(),
            MaterialPlugin::<terrain::TerrainMaterial>::default(),
            MaterialPlugin::<specular::SpecularMaterial>::default(),
            MaterialPlugin::<impostor::ImpostorMaterial>::default(),
            MaterialPlugin::<lod::LodCasterMaterial>::default(),
            MaterialPlugin::<lod::SurfaceCasterMaterial>::default(),
            MaterialPlugin::<grass::material::GrassMaterial>::default(),
            grass::writers::GrassDisplacementPlugin,
            world_texture_upload::WorldTextureUploadPlugin,
            super::fixed_assets::FixedAssetsPlugin,
            super::candidate_upload::CandidateUploadPlugin,
        ))
        .insert_resource(MaterialLoading(Arc::new(Mutex::new(LoadingData {
            requests: Vec::new(),
            sealed: false,
            state: RenderingState::Loading,
            shadow_capacity: 0,
            transparent_pass_capacity: 0,
            terminal_prewarm_audit: None,
        }))))
        .init_resource::<RetainedColors>()
        .init_resource::<super::renderers::RendererMeshes>()
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
                bind_authored_materials
                    .after(bind_materials)
                    // Apply material commands before lighting removes its prototype.
                    .before(super::lighting::lighting_readiness)
                    .before(super::world::prepare_road_lamp),
            )
                .chain(),
        );
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render.add_systems(
                Render,
                (prepare_retained_casters, upload_retained_colors)
                    .chain()
                    .after(RenderSystems::PrepareBindGroups)
                    .before(prewarm_materials),
            );
            render
                .init_resource::<super::renderers::ShadowsOnly>()
                .add_systems(
                    bevy::render::ExtractSchedule,
                    super::renderers::extract_shadows_only,
                )
                .init_resource::<WarmedPipelines>()
                .add_systems(
                    Render,
                    prewarm_materials.after(RenderSystems::PrepareBindGroups),
                );
        }
    }
}

pub fn load_materials(
    commands: &mut Commands,
    _server: &AssetServer,
    reference: &ReferenceManifest,
) -> Result<(), source::ReferenceError> {
    let mut sort_data = HashMap::new();
    let mut sort_identities = HashMap::<String, u32>::new();
    let mut shader_identities = HashMap::<String, i32>::new();
    for record in &reference.rendering.materials {
        if !matches!(
            record.state.alpha_mode,
            source::AlphaMode::Blend | source::AlphaMode::Premultiply | source::AlphaMode::Refract
        ) {
            continue;
        }
        let mut identity = serde_json::to_value(record).expect("validated material");
        identity.as_object_mut().unwrap().remove("asset");
        identity.as_object_mut().unwrap().remove("gltf_material");
        let next = sort_identities.len() as u32 + 1;
        let runtime_material = *sort_identities.entry(identity.to_string()).or_insert(next);
        let next = shader_identities.len() as i32 + 1;
        let runtime_shader = *shader_identities
            .entry(format!("{:?}", record.shader_family))
            .or_insert(next);
        sort_data.insert(
            (record.asset, record.gltf_material),
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
                source_material_index: 0,
                layer_and_order: 0,
                queue: if record.state.render_queue < 0 {
                    3000
                } else {
                    record.state.render_queue as i16
                },
                additive: record.shader_family == ShaderFamily::Standard,
                glass: record.shader_family == ShaderFamily::StainedGlass,
            },
        );
    }
    let fixed_assets: HashSet<_> = reference
        .assets
        .iter()
        .filter(|asset| {
            matches!(
                asset.usage,
                AssetUsage::UniqueScene
                    | AssetUsage::RoadLampPrototype
                    | AssetUsage::SkyPrototype
                    | AssetUsage::FoodPrototype
            )
        })
        .map(|asset| asset.id)
        .collect();
    let records: Vec<_> = reference
        .rendering
        .materials
        .iter()
        .filter(|record| fixed_assets.contains(&record.asset))
        .map(|record| Arc::new(record.clone()))
        .collect();
    let mut textures = Vec::new();
    let mut texture_indices = HashMap::new();
    for record in &records {
        for texture in &record.properties.textures {
            if texture_indices.contains_key(&texture.file) {
                continue;
            }
            let input_staging = primary::png_chain_staging(
                &texture.mips,
                texture.sampler.mip_count,
                &reference.file_catalog,
            )?;
            let pixel_bytes = source::fixed_reservation::png_pixels(&texture.mips)?;
            let cpu_reservation = source::fixed_reservation::png_cpu(pixel_bytes, input_staging)?;
            texture_indices.insert(texture.file.clone(), textures.len());
            textures.push(SourceTexture {
                owner: Arc::new(()),
                upload: default(),
                failed: false,
                persistent: true,
                users: 0,
                record: Arc::new(texture.clone()),
                chain: None,
                input_staging,
                pixel_bytes,
                cpu_reservation,
                image: None,
                label: format!(
                    "material {}:{} {}",
                    record.source.file, record.source.path_id, texture.slot
                ),
            });
        }
    }
    for texture in &reference.plates.material.textures {
        if texture_indices.contains_key(&texture.file) {
            continue;
        }
        let input_staging = primary::png_chain_staging(
            &texture.mips,
            texture.sampler.mip_count,
            &reference.file_catalog,
        )?;
        let pixel_bytes = source::fixed_reservation::png_pixels(&texture.mips)?;
        let cpu_reservation = source::fixed_reservation::png_cpu(pixel_bytes, input_staging)?;

        texture_indices.insert(texture.file.clone(), textures.len());
        textures.push(SourceTexture {
            owner: Arc::new(()),
            upload: default(),
            failed: false,
            persistent: true,
            users: 0,
            record: Arc::new(texture.clone()),
            chain: None,
            input_staging,
            pixel_bytes,
            cpu_reservation,
            image: None,
            label: format!("source texture {}", texture.file),
        });
    }
    let mut source = SourceMaterials {
        preparation: default(),
        world_textures: world_textures::WorldTextures::default(),
        world_primary: primary::PrimaryOwnership::default(),
        world_deferred: primary::DeferredAllocation::default(),
        standard_records: HashMap::new(),
        sort_data,
        records,
        textures,
        texture_indices,
        handles: HashMap::new(),
        shaders: default(),
        fixed_admitted: false,
        state: RenderingState::Loading,
        bound: false,
        texture_bytes: 0,
    };
    source.select_lighting_textures(reference)?;
    commands.insert_resource(source);
    Ok(())
}

pub fn material_readiness(
    server: Res<AssetServer>,
    mut source: ResMut<SourceMaterials>,
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
    if source.bound {
        source.state = loading.0.lock().unwrap().state.clone();
    }
}
fn assemble_textures(
    textures: &mut [SourceTexture],
    chains: &mut Assets<SourcePngMipChain>,
    images: &mut Assets<Image>,
) -> usize {
    let mut bytes = 0;
    for texture in textures
        .iter_mut()
        .filter(|t| t.image.is_none() && !t.failed)
    {
        let Some(handle) = &texture.chain else {
            continue;
        };
        let Some(SourcePngMipChain(image)) = chains.remove(handle.id()) else {
            continue;
        };
        texture.image = Some(images.add(image));
        texture.chain = None;
        bytes += texture.pixel_bytes;
    }
    bytes
}
#[derive(serde::Deserialize)]
pub(crate) struct MaterialIdentity {
    pub asset: source::AssetId,
    pub gltf_material: u32,
}
pub(crate) fn bind_authored_materials(
    mut commands: Commands,
    mut source: ResMut<SourceMaterials>,
    mut materials: ResMut<Assets<SourceMaterial>>,
    visuals: Res<super::reference::ReferenceVisuals>,
    renderer_meshes: Res<super::renderers::RendererMeshes>,
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
            let sort = source
                .sort_data(
                    (identity.asset, identity.gltf_material),
                    &renderer.0,
                    u32::from(slot >> 1),
                )
                .expect("validated transparent slot")
                .expect("transparent material metadata");
            commands
                .entity(entity)
                .insert((bevy::render::batching::NoAutomaticBatching, sort));
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
                material: handle.id().untyped(),
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
    pending.requests.extend(requests);
    pending.shadow_capacity = primitives.iter().count();
    // The checked plate contract owns exactly two retained glyph meshes.
    pending.transparent_pass_capacity = transparent_pass_capacity + 2;
    pending.sealed = false;
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
        specular: Vec4::ZERO,
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
                ShaderFamily::PlayerModel => unreachable!("inactive player material"),
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
    record
        .validate_active_use()
        .expect("validated active material usage");
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
        retained_color: None,
        preparation: default(),
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
            lod_fade: 1.,
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
                        type_id: request.material.type_id(),
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
    pub(crate) retained_color: Option<RetainedColor>,
    pub preparation: super::gpu_prepare::GpuPreparation,
    pub(crate) mirror: Option<(u64, usize, bool)>,
    pub base: StandardMaterial,
    pub extension: SourceExtension,
}
impl SourceMaterial {
    pub(crate) fn caster_data(&self) -> SourceUniform {
        let mut data = self.extension.surface.data;
        if let Some(color) = &self.retained_color {
            data.color = color.0.lock().unwrap().linear;
        }
        data
    }
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
    fn as_bind_group(
        &self,
        descriptor: &BindGroupLayoutDescriptor,
        device: &RenderDevice,
        cache: &PipelineCache,
        param: &mut SystemParamItem<Self::Param>,
    ) -> Result<PreparedBindGroup, AsBindGroupError> {
        crate::presentation::gpu_prepare::bind_group(
            self,
            &self.preparation,
            descriptor,
            device,
            cache,
            param,
        )
    }
    fn unprepared_bind_group(
        &self,
        layout: &BindGroupLayout,
        device: &RenderDevice,
        params: &mut SystemParamItem<Self::Param>,
        direct: bool,
    ) -> Result<UnpreparedBindGroup, AsBindGroupError> {
        if self.preparation.isolated() && !direct {
            return Err(AsBindGroupError::CreateBindGroupDirectly);
        }
        let mut prepared = self
            .base
            .unprepared_bind_group(layout, device, &mut params.0, true)?;
        prepared.bindings.0.retain(|(binding, _)| *binding <= 12);
        let extension =
            self.extension
                .unprepared_bind_group(layout, device, &mut params.1, true)?;
        prepared.bindings.0.extend(extension.bindings.0);
        if let Some(color) = &self.retained_color {
            let mut retained = color.0.lock().unwrap();
            let buffer = retained
                .buffer
                .get_or_insert_with(|| {
                    let mut bytes = encase::UniformBuffer::new(Vec::new());
                    bytes.write(&self.extension.surface.data).unwrap();
                    device.create_buffer_with_data(&BufferInitDescriptor {
                        label: Some("retained condition receiver"),
                        contents: bytes.as_ref(),
                        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                    })
                })
                .clone();
            *prepared
                .bindings
                .0
                .iter_mut()
                .find(|(binding, _)| *binding == 120)
                .expect("source color binding") = (120, OwnedBindingResource::Buffer(buffer));
            retained.dirty = true;
        }
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

#[derive(Asset, TypePath, Clone, Debug)]
pub struct SourceExtension {
    pub lod_fade: f32,
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
impl Default for SourceExtension {
    fn default() -> Self {
        Self {
            receiver_mask: u32::MAX,
            lod_fade: 1.,
            surface: SourceSurface::default(),
        }
    }
}
impl AsBindGroup for SourceExtension {
    type Data = SurfaceKey;
    type Param = (
        <SourceReceiver as AsBindGroup>::Param,
        <SourceSurface as AsBindGroup>::Param,
    );
    fn label() -> &'static str {
        "source surface lighting"
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
        let mut result = SourceReceiver {
            receiver_mask: self.receiver_mask,
            lod_fade: self.lod_fade,
        }
        .unprepared_bind_group(layout, device, &mut params.0, false)?;
        result.bindings.0.extend(
            self.surface
                .unprepared_bind_group(layout, device, &mut params.1, false)?
                .bindings
                .0,
        );
        Ok(result)
    }
    fn bind_group_layout_entries(device: &RenderDevice, force: bool) -> Vec<BindGroupLayoutEntry> {
        let mut entries = SourceReceiver::bind_group_layout_entries(device, force);
        entries.extend(SourceSurface::bind_group_layout_entries(device, force));
        entries
    }
}
pub struct SourceReceiver {
    pub receiver_mask: u32,
    pub lod_fade: f32,
}
impl AsBindGroup for SourceReceiver {
    type Data = ();
    type Param = SRes<LightingGpu>;
    fn label() -> &'static str {
        "source lighting"
    }
    fn bind_group_data(&self) -> Self::Data {
        ()
    }
    fn unprepared_bind_group(
        &self,
        layout: &BindGroupLayout,
        device: &RenderDevice,
        params: &mut SystemParamItem<Self::Param>,
        _: bool,
    ) -> Result<UnpreparedBindGroup, AsBindGroupError> {
        let gpu = params;
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
                    114,
                    OwnedBindingResource::Buffer(
                        gpu.shadow_a
                            .buffer()
                            .ok_or(AsBindGroupError::RetryNextUpdate)?
                            .clone(),
                    ),
                ),
                (
                    115,
                    OwnedBindingResource::Buffer(
                        gpu.shadow_b
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
                        TextureViewDimension::D2Array,
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
                (
                    114,
                    uniform_buffer::<
                        super::lighting::ShadowUniform<{ super::lighting::SHADOW_BANK_SIZE }>,
                    >(false),
                ),
                (
                    115,
                    uniform_buffer::<
                        super::lighting::ShadowUniform<
                            { super::lighting::SHADOW_SLOTS - super::lighting::SHADOW_BANK_SIZE },
                        >,
                    >(false),
                ),
                (101, texture_2d(TextureSampleType::Depth)),
                (102, sampler(SamplerBindingType::Comparison)),
                (
                    103,
                    texture_2d_array(TextureSampleType::Float { filterable: true }),
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
    sort: Option<super::transparent::SourceSortData>,
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
            &super::renderers::SourceRenderer,
            &GltfExtras,
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
                let (mesh, receiver, global, shadow_mode, children, renderer, primitive) =
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
                let sort = match authored_materials.sort_data(
                    key,
                    &renderer.0,
                    serde_json::from_str::<serde_json::Value>(&primitive.value)
                        .expect("validated primitive")["submesh"]
                        .as_u64()
                        .expect("validated submesh") as u32,
                ) {
                    Ok(sort) => sort,
                    Err(error) => {
                        source.state = super::lighting::RenderingState::Failed(error.0);
                        return;
                    }
                };
                let mut warm = commands.spawn((
                    mesh.clone(),
                    MeshMaterial3d(material.clone()),
                    SourceMaterialWarmup,
                    renderer.clone(),
                    far,
                    Visibility::Visible,
                    bevy::light::NotShadowCaster,
                    bevy::camera::visibility::NoFrustumCulling,
                ));
                if let Some(sort) = sort {
                    warm.insert((sort, bevy::render::batching::NoAutomaticBatching));
                }
                source.warmups.push((warm.id(), false));
                let mut binding = LampReceiverChoice {
                    entity,
                    sort,
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
    receivers: Query<&MeshMaterial3d<SourceMaterial>>,
    casters: Query<&MeshMaterial3d<StandardMaterial>>,
) {
    if !source.materials_prepared {
        return;
    }
    for target in &mut source.lamp_materials {
        let output = &drive.runtime.lighting.material_outputs()[target.output_index];
        let key = (output.asset, output.material_index);
        let choice = target
            .choices
            .iter()
            .find(|choice| choice.key == key)
            .expect("prepared source material choice");
        if target.current == Some(key)
            && choice.receivers.iter().all(|r| {
                receivers.get(r.entity).is_ok_and(|m| m.0 == r.material)
                    && r.casters
                        .iter()
                        .all(|(entity, handle)| casters.get(*entity).is_ok_and(|m| &m.0 == handle))
            })
        {
            continue;
        }
        for receiver in &choice.receivers {
            commands
                .entity(receiver.entity)
                .insert(MeshMaterial3d(receiver.material.clone()));
            if let Some(sort) = receiver.sort {
                commands
                    .entity(receiver.entity)
                    .insert((sort, bevy::render::batching::NoAutomaticBatching));
            } else {
                commands
                    .entity(receiver.entity)
                    .remove::<super::transparent::SourceSortData>();
            }
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
    let mut material = make_material(record, reflected, mask, source);
    material.preparation = source.preparation.clone();
    material
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
        material: material.untyped(),
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
        .retain(|request| {
            request.material != live.untyped() && request.material != disabled.untyped()
        });
}

/// Retain all source choices, including hidden lamps, for capture loading.
pub(crate) fn mirror_warmup_pairs(world: &World, pairs: &mut Vec<(AssetId<Mesh>, UntypedAssetId)>) {
    let loading = world.resource::<MaterialLoading>().0.lock().unwrap();
    pairs.extend(loading.requests.iter().map(|r| (r.mesh, r.material)));
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

impl SourceMaterials {
    pub(crate) fn texture(&self, file: &str) -> Option<Handle<Image>> {
        self.texture_indices
            .get(file)
            .and_then(|i| self.textures[*i].image.clone())
    }
}
pub(crate) fn warm_retained_material(
    world: &World,
    mesh: AssetId<Mesh>,
    material: UntypedAssetId,
    label: String,
) {
    world
        .resource::<MaterialLoading>()
        .0
        .lock()
        .unwrap()
        .requests
        .push(PipelineRequest {
            mesh,
            material,
            label,
        });
}

pub(crate) fn seal_plate_materials(world: &World) {
    if world.resource::<SourceMaterials>().bound
        && world
            .resource::<super::conditions::ConditionVisuals>()
            .prepared
    {
        let warmup = world
            .iter_entities()
            .filter(|e| e.contains::<MeshMaterial3d<SourceMaterial>>())
            .count();
        let casters = world
            .iter_entities()
            .filter(|e| e.contains::<SourceCaster>())
            .count();
        let weight = world.resource::<SourceMaterials>().maximum_pass_weight();
        let mut pending = world.resource::<MaterialLoading>().0.lock().unwrap();
        pending.transparent_pass_capacity =
            pending.transparent_pass_capacity.max(warmup * weight + 2);
        pending.shadow_capacity = pending.shadow_capacity.max(casters);
        pending.sealed = true;
    }
}

pub(crate) fn set_source_color(material: &mut SourceMaterial, color: [f32; 4]) {
    let value = Color::srgba(color[0], color[1], color[2], color[3]);
    material.base.base_color = value;
    material.extension.surface.data.color = Vec4::from_array(value.to_linear().to_f32_array());
}
impl SourceMaterials {
    fn maximum_pass_weight(&self) -> usize {
        self.records
            .iter()
            .filter(|r| {
                matches!(
                    r.state.alpha_mode,
                    source::AlphaMode::Blend
                        | source::AlphaMode::Premultiply
                        | source::AlphaMode::Refract
                )
            })
            .map(|r| {
                1 + usize::from(matches!(
                    r.shader_family,
                    ShaderFamily::Standard | ShaderFamily::StainedGlass
                ))
            })
            .max()
            .unwrap_or(1)
    }
}

// The source shader reads color first in binding 120. Keep the original buffer and bind group.
#[derive(Clone, Debug)]
pub(crate) struct RetainedColor(Arc<Mutex<RetainedColorData>>);
#[derive(Debug)]
struct RetainedColorData {
    linear: Vec4,
    revision: u64,
    dirty: bool,
    buffer: Option<Buffer>,
    caster: Option<(AssetId<StandardMaterial>, StandardMaterial)>,
    caster_buffer: Option<Buffer>,
}
#[derive(Resource, Clone, Default, ExtractResource)]
struct RetainedColors(Arc<Mutex<Vec<RetainedColor>>>);
impl RetainedColor {
    pub(crate) fn revision_color(&self) -> (u64, Vec4) {
        let value = self.0.lock().unwrap();
        (value.revision, value.linear)
    }
}
impl SourceUniform {
    pub(crate) fn with_color(mut self, color: Vec4) -> Self {
        self.color = color;
        self
    }
}
pub(crate) fn retain_color(material: &mut SourceMaterial, world: &World) {
    let color = RetainedColor(Arc::new(Mutex::new(RetainedColorData {
        linear: material.extension.surface.data.color,
        revision: 0,
        dirty: true,
        buffer: None,
        caster: None,
        caster_buffer: None,
    })));
    world
        .resource::<RetainedColors>()
        .0
        .lock()
        .unwrap()
        .push(color.clone());
    material.retained_color = Some(color);
}
pub(crate) fn update_retained_color(material: &SourceMaterial, color: [f32; 4]) {
    let linear = Vec4::from_array(
        Color::srgba(color[0], color[1], color[2], color[3])
            .to_linear()
            .to_f32_array(),
    );
    let mut retained = material
        .retained_color
        .as_ref()
        .expect("retained source color")
        .0
        .lock()
        .unwrap();
    if retained.linear != linear {
        retained.linear = linear;
        retained.revision += 1;
        retained.dirty = true;
    }
}
pub(crate) fn link_retained_color(
    material: &Handle<SourceMaterial>,
    caster: &Handle<StandardMaterial>,
    world: &World,
) {
    let source = world
        .resource::<Assets<SourceMaterial>>()
        .get(material)
        .unwrap();
    let mut retained = source.retained_color.as_ref().unwrap().0.lock().unwrap();
    retained.caster = Some((
        caster.id(),
        world
            .resource::<Assets<StandardMaterial>>()
            .get(caster)
            .unwrap()
            .clone(),
    ));
}
fn prepare_retained_casters(
    colors: Res<RetainedColors>,
    device: Res<RenderDevice>,
    cache: Res<PipelineCache>,
    mut params: bevy::ecs::system::StaticSystemParam<<StandardMaterial as AsBindGroup>::Param>,
    mut prepared: ResMut<ErasedRenderAssets<PreparedMaterial>>,
    mut allocators: ResMut<bevy::pbr::MaterialBindGroupAllocators>,
) {
    for color in colors.0.lock().unwrap().iter() {
        let mut color = color.0.lock().unwrap();
        if color.caster_buffer.is_some() {
            continue;
        }
        let Some((id, source)) = &color.caster else {
            continue;
        };
        let Some(material) = prepared.get_mut(*id) else {
            continue;
        };
        assert!(
            !StandardMaterial::bindless_supported(&device),
            "retained source casters require the checked WebGL2 layout"
        );
        let descriptor = material
            .properties
            .material_layout
            .as_ref()
            .expect("standard material layout");
        let layout = cache.get_bind_group_layout(descriptor);
        let Ok(mut group) = source.unprepared_bind_group(&layout, &device, &mut params, true)
        else {
            continue;
        };
        let binding = group
            .bindings
            .0
            .iter_mut()
            .find(|(binding, _)| *binding == 0)
            .expect("standard uniform binding");
        let OwnedBindingResource::Data(data) = &binding.1 else {
            panic!("checked StandardMaterial binding 0 Data");
        };
        let buffer = device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("retained condition caster"),
            contents: &data.0,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        binding.1 = OwnedBindingResource::Buffer(buffer.clone());
        let entries: Vec<_> = group
            .bindings
            .0
            .iter()
            .map(|(binding, resource)| BindGroupEntry {
                binding: *binding,
                resource: resource.get_binding(),
            })
            .collect();
        let bind_group = device.create_bind_group("retained condition caster", &layout, &entries);
        let allocator = allocators
            .get_mut(&std::any::TypeId::of::<StandardMaterial>())
            .expect("standard allocator");
        let old = material.binding;
        material.binding = allocator.allocate_prepared(PreparedBindGroup {
            bindings: group.bindings,
            bind_group,
        });
        allocator.free(old);
        color.caster_buffer = Some(buffer);
        color.dirty = true;
    }
}
fn upload_retained_colors(colors: Res<RetainedColors>, queue: Res<RenderQueue>) {
    for color in colors.0.lock().unwrap().iter() {
        let mut color = color.0.lock().unwrap();
        if !color.dirty {
            continue;
        }
        let Some(buffer) = &color.buffer else {
            continue;
        };
        let mut bytes = [0u8; 16];
        for (slot, value) in bytes.chunks_exact_mut(4).zip(color.linear.to_array()) {
            slot.copy_from_slice(&value.to_le_bytes());
        }
        queue.write_buffer(buffer, 0, &bytes);
        if let Some(buffer) = &color.caster_buffer {
            queue.write_buffer(buffer, 0, &bytes);
        }
        color.dirty = false;
    }
}

impl SourceMaterials {
    pub(crate) fn measurement_images(&self) -> impl Iterator<Item = (AssetId<Image>, bool)> + '_ {
        self.textures
            .iter()
            .flat_map(|t| t.image.iter().map(move |h| (h.id(), true)))
            .chain(
                self.world_textures
                    .measurement_images()
                    .map(|id| (id, true)),
            )
    }
}

impl SourceMaterials {
    pub fn drain_ready(
        &mut self,
        server: &AssetServer,
        chains: &Assets<SourcePngMipChain>,
        batches: &mut Assets<crate::presentation::texture_batches::SourceTextureBatch>,
    ) -> bool {
        self.world_textures.drain_ready(server, batches)
            && self.textures.iter().all(|t| {
                (t.image.is_some() && primary::pending_texture_cpu(t) == 0)
                    || (t.image.is_none()
                        && t.chain.iter().all(|h| {
                            chains.contains(h.id())
                                || matches!(server.load_state(h.id()), LoadState::Failed(_))
                        }))
            })
    }
    pub fn release_all(
        &mut self,
        server: &AssetServer,
        images: &mut Assets<Image>,
        chains: &mut Assets<SourcePngMipChain>,
        materials: &mut Assets<SourceMaterial>,
    ) -> Result<(), source::ReferenceError> {
        let assets: Vec<_> = self.world_primary.asset_ids().collect();
        for asset in assets {
            self.release_world_asset(asset, images, chains, materials, server)?;
        }
        self.world_textures.release_all(images);
        Ok(())
    }
    pub fn settled(&self) -> bool {
        self.textures
            .iter()
            .all(|t| !t.failed && t.image.is_some() && primary::pending_texture_cpu(t) == 0)
            && self.world_textures.settled()
    }
    pub fn shared_bytes(&self) -> (usize, usize) {
        self.shared_bytes_if(true)
    }
    pub fn fork_shared_bytes(&self) -> (usize, usize) {
        self.shared_bytes_if(false)
    }
    fn shared_bytes_if(&self, shared: bool) -> (usize, usize) {
        let pixels: usize = self
            .textures
            .iter()
            .filter(|t| !shared || Arc::strong_count(&t.owner) > 1)
            .map(|t| t.pixel_bytes)
            .sum();
        let record_bytes: usize = self
            .records
            .iter()
            .filter(|r| !shared || Arc::strong_count(r) > 1)
            .map(|r| primary::record_bytes(r))
            .sum();
        let texture_metadata: usize = self
            .textures
            .iter()
            .filter(|t| !shared || Arc::strong_count(&t.record) > 1)
            .map(|t| primary::texture_metadata(&t.record))
            .sum();
        let (cpu, gpu) = if shared {
            self.world_textures.shared_bytes()
        } else {
            self.world_textures.fork_shared_bytes()
        };
        (
            cpu + self
                .textures
                .iter()
                .filter(|t| !shared || Arc::strong_count(&t.owner) > 1)
                .map(primary::pending_texture_cpu)
                .sum::<usize>()
                + record_bytes
                + texture_metadata,
            gpu + pixels,
        )
    }
    pub fn fork(
        &self,
        stream: &mut crate::world::WorldStream,
    ) -> Result<Self, source::ReferenceError> {
        let world_textures = self.world_textures.fork()?;
        self.reserve_fork(stream)?;
        let mut copy = self.clone();
        copy.preparation = super::gpu_prepare::GpuPreparation::candidate();
        copy.world_textures = world_textures;
        copy.begin_world_session();
        copy.account_fork(stream)?;
        Ok(copy)
    }
}
