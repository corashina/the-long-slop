use crate::reference::{
    self as source, MaterialRecord, RenderingRecord, ShaderFamily, TextureColorSpace, TextureRecord,
};
use bevy::asset::{AssetId, LoadState, RenderAssetUsages};
use bevy::gltf::GltfMaterialExtras;
use bevy::image::{
    ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor,
};
use bevy::material::key::{ErasedMaterialPipelineKey, ErasedMeshPipelineKey};
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{
    ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline,
    MeshPipeline, MeshPipelineKey, PreparedMaterial, ViewKeyCache, alpha_mode_pipeline_key,
    base_specialize,
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

const SLOTS: [&str; 9] = [
    "_MainTex",
    "_BumpMap",
    "_MetallicGlossMap",
    "_OcclusionMap",
    "_EmissionMap",
    "_ParallaxMap",
    "_DetailAlbedoMap",
    "_DetailMask",
    "_DetailNormalMap",
];
pub type SourceMaterial = ExtendedMaterial<StandardMaterial, SourceSurface>;

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
    #[uniform(100)]
    data: SourceUniform,
    #[texture(101)]
    #[sampler(102)]
    detail: Option<Handle<Image>>,
    #[texture(103)]
    #[sampler(104)]
    mask: Option<Handle<Image>>,
    #[texture(105)]
    #[sampler(106)]
    detail_normal: Option<Handle<Image>>,
    state: SurfaceKey,
}
impl MaterialExtension for SourceSurface {
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
        let state = key.bind_group_data;
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
    records: Vec<MaterialRecord>,
    textures: Vec<SourceTexture>,
    texture_indices: HashMap<String, usize>,
    pub handles: HashMap<(source::AssetId, u32, bool), Handle<SourceMaterial>>,
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
            MaterialPlugin::<SourceMaterial>::default(),
            ExtractResourcePlugin::<MaterialLoading>::default(),
        ))
        .insert_resource(MaterialLoading(Arc::new(Mutex::new(LoadingData {
            requests: Vec::new(),
            sealed: false,
            state: RenderingState::Loading,
        }))))
        .add_systems(Update, (material_readiness, bind_materials).chain());
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
        records: rendering.materials.clone(),
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
pub fn bind_materials(
    mut commands: Commands,
    mut source: ResMut<SourceMaterials>,
    mut materials: ResMut<Assets<SourceMaterial>>,
    visuals: Res<super::reference::ReferenceVisuals>,
    primitives: Query<(Entity, &GltfMaterialExtras, &Mesh3d, &GlobalTransform)>,
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
    for (entity, extras, mesh, transform) in &primitives {
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
        let key = (identity.asset, identity.gltf_material, reflected);
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
            let identity = identity.to_string();
            let handle = shared
                .entry(identity)
                .or_insert_with(|| materials.add(make_material(record, reflected, &source)))
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
    source: &SourceMaterials,
) -> SourceMaterial {
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
            },
            factor("_UVSec", 0.) as u32,
        ),
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
    let mut textures: [Option<Handle<Image>>; 9] = default();
    for (index, slot) in SLOTS.iter().enumerate() {
        if let Some(texture) = record
            .properties
            .textures
            .iter()
            .find(|entry| entry.slot == *slot)
        {
            textures[index] = source.textures[source.texture_indices[&texture.file]]
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
    SourceMaterial {
        base: StandardMaterial {
            base_color: Color::WHITE,
            base_color_texture: textures[0].clone(),
            normal_map_texture: textures[1].clone(),
            metallic_roughness_texture: textures[2].clone(),
            occlusion_texture: textures[3].clone(),
            emissive_texture: textures[4].clone(),
            depth_map: textures[5].clone(),
            alpha_mode,
            unlit: record.shader_family == ShaderFamily::UnlitColor,
            ..default()
        },
        extension: SourceSurface {
            data,
            detail: textures[6].clone(),
            mask: textures[7].clone(),
            detail_normal: textures[8].clone(),
            state: SurfaceKey {
                cull: record.state.cull,
                z_write: record.state.z_write,
                z_test: record.state.z_test,
                src: record.state.src_blend,
                dst: record.state.dst_blend,
                reflected,
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
