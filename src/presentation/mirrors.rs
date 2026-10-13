//! Retained source mirror cameras and per-view surface bindings.
use super::{
    camera::DriveCamera,
    materials::{SourceMaterial, SourceMaterials, authored_material, make_mirror_material},
    reference::ReferenceVisuals,
};
use crate::{
    app::{Drive, RuntimeReference},
    mirrors::{MirrorFrame, MirrorInput, MirrorState},
};
use bevy::asset::{AssetId, RenderAssetUsages, UntypedAssetId};
use bevy::camera::{CameraProjection, RenderTarget, visibility::RenderLayers};
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::math::Vec3A;
use bevy::render::{
    Render, RenderApp, RenderSystems,
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    extract_resource::{ExtractResource, ExtractResourcePlugin},
    render_resource::*,
};
use bevy::{ecs as bevy_ecs, prelude::*, render as bevy_render};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum MirrorViewRole {
    Normal,
    Capture(usize),
}

impl bevy::render::sync_component::SyncComponent for MirrorViewRole {
    type Target = Self;
}
impl ExtractComponent for MirrorViewRole {
    type QueryData = &'static Self;
    type QueryFilter = ();
    type Out = Self;
    fn extract_component(
        item: bevy::ecs::query::QueryItem<'_, '_, Self::QueryData>,
    ) -> Option<Self> {
        Some(*item)
    }
}

pub fn mirror_material_for_view(
    role: MirrorViewRole,
    owner: usize,
    live: AssetId<SourceMaterial>,
    disabled: AssetId<SourceMaterial>,
) -> AssetId<SourceMaterial> {
    if role == MirrorViewRole::Capture(owner) {
        disabled
    } else {
        live
    }
}

#[derive(Clone, Debug)]
pub(crate) struct MirrorProjection {
    pub(crate) perspective: PerspectiveProjection,
}
impl CameraProjection for MirrorProjection {
    fn get_clip_from_view(&self) -> Mat4 {
        // Reverse-Z finite perspective. Preserve the native far value outside this projection.
        Mat4::perspective_rh(
            self.perspective.fov,
            1.,
            self.perspective.far,
            self.perspective.near,
        )
    }
    fn get_clip_from_view_for_sub(&self, _: &bevy::camera::SubCameraView) -> Mat4 {
        self.get_clip_from_view()
    }
    fn update(&mut self, _: f32, _: f32) {}
    fn far(&self) -> f32 {
        self.perspective.far
    }
    fn get_frustum_corners(&self, near: f32, far: f32) -> [Vec3A; 8] {
        self.perspective.get_frustum_corners(near, far)
    }
}

struct MirrorBinding {
    plane: Entity,
    camera_node: Entity,
    screen: Entity,
    camera: Entity,
    target: Handle<Image>,
    live: Handle<SourceMaterial>,
    disabled: Handle<SourceMaterial>,
}
#[derive(Resource)]
pub struct SourceMirrors {
    instance: u64,
    bindings: Vec<MirrorBinding>,
    normal_camera: Entity,
    native: MirrorState,
    inputs: Vec<MirrorInput>,
}
#[derive(Resource, Clone, Default, ExtractResource)]
pub(crate) struct MirrorSurfaces {
    instance: u64,
    entries: Arc<Vec<(Entity, AssetId<SourceMaterial>, AssetId<SourceMaterial>)>>,
}

#[derive(Resource, Clone, Default, ExtractResource)]
struct MirrorWarmup {
    instance: u64,
    pairs: Arc<Vec<(AssetId<Mesh>, UntypedAssetId)>>,
    owners: [(
        AssetId<Mesh>,
        AssetId<SourceMaterial>,
        AssetId<SourceMaterial>,
    ); 3],
}

fn collect_mirror_warmup(world: &mut World) {
    let Some(mirrors) = world.get_resource::<SourceMirrors>() else {
        return;
    };
    if world
        .get_resource::<MirrorWarmup>()
        .is_some_and(|w| w.instance == mirrors.instance && !w.pairs.is_empty())
    {
        return;
    }
    let lighting = world.resource::<super::lighting::SourceLighting>();
    if !lighting.materials_prepared
        || !lighting.sky.bound()
        || world.resource::<SourceMaterials>().state != super::materials::RenderingState::Ready
    {
        return;
    }
    let instance = mirrors.instance;
    let owners = std::array::from_fn(|i| {
        let b = &mirrors.bindings[i];
        (
            world.get::<Mesh3d>(b.screen).unwrap().0.id(),
            b.live.id(),
            b.disabled.id(),
        )
    });
    let mut pairs = Vec::new();
    let mut scene_meshes = HashSet::new();
    super::materials::mirror_warmup_pairs(world, &mut pairs);
    super::plates::plate_warmup_pairs(world, &mut pairs);
    for (mesh, source, standard, sky) in world
        .query_filtered::<(
            &Mesh3d,
            Option<&MeshMaterial3d<SourceMaterial>>,
            Option<&MeshMaterial3d<StandardMaterial>>,
            Option<&MeshMaterial3d<super::sky::SkyMaterial>>,
        ), Without<super::materials::SourceMaterialWarmup>>()
        .iter(world)
    {
        scene_meshes.insert(mesh.0.id());
        if let Some(material) = source {
            pairs.push((mesh.0.id(), material.id().untyped()));
        }
        if let Some(material) = standard {
            pairs.push((mesh.0.id(), material.id().untyped()));
        }
        if let Some(material) = sky {
            pairs.push((mesh.0.id(), material.id().untyped()));
            for owner in 0..3 {
                pairs.push((
                    mesh.0.id(),
                    world
                        .resource::<super::sky::SkyVariants>()
                        .resolve(MirrorViewRole::Capture(owner), material.id().untyped()),
                ));
            }
        }
    }
    // Save replacement and streaming remove terrain meshes from the previous scene.
    // Retain every material variant only while its mesh belongs to this scene.
    let previous_count = pairs.len();
    pairs.retain(|(mesh, _)| scene_meshes.contains(mesh));
    world
        .resource::<MirrorGpuStatus>()
        .0
        .lock()
        .unwrap()
        .obsolete_pairs_removed = (previous_count - pairs.len()) as u32;
    let mut seen = HashSet::new();
    pairs.retain(|pair| seen.insert(*pair));
    world.insert_resource(MirrorWarmup {
        instance,
        pairs: Arc::new(pairs),
        owners,
    });
}

fn resolve_mirror_material(
    role: MirrorViewRole,
    entity: MainEntity,
    current: UntypedAssetId,
    surfaces: &MirrorSurfaces,
) -> UntypedAssetId {
    for (owner, entry) in surfaces.entries.iter().enumerate() {
        if MainEntity::from(entry.0) == entity {
            return if current == entry.2.untyped() {
                current
            } else {
                mirror_material_for_view(role, owner, entry.1, entry.2).untyped()
            };
        }
    }
    current
}

#[derive(Component, Clone, Copy)]
pub(crate) struct MirrorInstance(pub(crate) u64);
impl bevy::render::sync_component::SyncComponent for MirrorInstance {
    type Target = Self;
}
impl ExtractComponent for MirrorInstance {
    type QueryData = &'static Self;
    type QueryFilter = ();
    type Out = Self;
    fn extract_component(
        item: bevy::ecs::query::QueryItem<'_, '_, Self::QueryData>,
    ) -> Option<Self> {
        Some(*item)
    }
}

pub fn reset_mirrors(world: &mut World) {
    let instance = {
        let mut status = world.resource::<MirrorGpuStatus>().0.lock().unwrap();
        let instance = status
            .instance
            .checked_add(1)
            .expect("mirror instance overflow");
        *status = MirrorGpuData {
            instance,
            ..default()
        };
        instance
    };
    world.insert_resource(MirrorSurfaces {
        instance,
        ..default()
    });
    if let Some(mirrors) = world.remove_resource::<SourceMirrors>() {
        debug_assert!(mirrors.instance < instance);
        for binding in mirrors.bindings {
            world.despawn(binding.camera);
            if let Ok(mut screen) = world.get_entity_mut(binding.screen) {
                screen.remove::<MeshMaterial3d<SourceMaterial>>();
            }
            world
                .resource_mut::<Assets<Image>>()
                .remove(binding.target.id());
            world
                .resource_mut::<Assets<SourceMaterial>>()
                .remove(binding.live.id());
            world
                .resource_mut::<Assets<SourceMaterial>>()
                .remove(binding.disabled.id());
            super::materials::release_mirror_warmup(
                world,
                binding.live.id(),
                binding.disabled.id(),
            );
        }
    }
    if let Some(mut lighting) = world.get_resource_mut::<super::lighting::SourceLighting>() {
        lighting.sky.reset_capture_views();
    }
}

pub struct SourceMirrorPlugin;
impl Plugin for SourceMirrorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MirrorSurfaces>()
            .init_resource::<MirrorGpuStatus>()
            .init_resource::<MirrorWarmup>()
            .add_plugins((
                ExtractComponentPlugin::<MirrorViewRole>::default(),
                ExtractComponentPlugin::<MirrorInstance>::default(),
                ExtractResourcePlugin::<MirrorSurfaces>::default(),
                ExtractResourcePlugin::<MirrorWarmup>::default(),
                ExtractResourcePlugin::<MirrorGpuStatus>::default(),
            ))
            .add_systems(
                Update,
                bind_mirrors.after(super::materials::bind_authored_materials),
            )
            .add_systems(
                Update,
                collect_mirror_warmup
                    .after(bind_mirrors)
                    .after(super::materials::prepare_lamp_materials),
            )
            .add_systems(
                PostUpdate,
                sync_mirror_views
                    .after(bevy::transform::TransformSystems::Propagate)
                    .before(bevy::camera::CameraUpdateSystems)
                    .before(bevy::camera::visibility::VisibilitySystems::UpdateFrusta),
            );
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render
                .init_resource::<CapturePipelines>()
                .init_resource::<MirrorOutputs>()
                .add_render_command::<Opaque3d, DrawMirrorMaterial>()
                .add_render_command::<AlphaMask3d, DrawMirrorMaterial>()
                .add_render_command::<Transparent3d, DrawMirrorMaterial>()
                .add_render_command::<Transmissive3d, DrawMirrorMaterial>()
                .add_systems(
                    Render,
                    prepare_capture_outputs
                        .after(super::surface_views::prepare_views)
                        .in_set(RenderSystems::PrepareResources),
                )
                .add_systems(
                    Render,
                    prepare_capture_pipelines
                        .after(RenderSystems::Specialize)
                        .before(RenderSystems::QueueMeshes),
                )
                .add_systems(Render, finish_capture_frame.after(RenderSystems::Render))
                .add_systems(
                    Render,
                    queue_capture_meshes
                        .after(bevy::pbr::queue_material_meshes)
                        .in_set(RenderSystems::QueueMeshes),
                );
        }
    }
}

pub fn bind_mirrors(world: &mut World) {
    if let Some(mirrors) = world.get_resource::<SourceMirrors>() {
        let visuals = world.resource::<ReferenceVisuals>();
        let destroyed = !visuals.bound
            || mirrors.bindings.iter().any(|binding| {
                world.get::<GlobalTransform>(binding.plane).is_none()
                    || world.get::<GlobalTransform>(binding.camera_node).is_none()
                    || world.get::<Mesh3d>(binding.screen).is_none()
                    || world.get::<Camera>(binding.camera).is_none()
            });
        if destroyed {
            reset_mirrors(world);
        } else {
            return;
        }
    }
    if !world.resource::<ReferenceVisuals>().bound
        || !world.resource::<SourceMaterials>().textures_ready()
        || !world.resource::<SourceMaterials>().bound
    {
        return;
    }
    let reference = world.resource::<RuntimeReference>().0.clone();
    let instance = world
        .resource::<MirrorGpuStatus>()
        .0
        .lock()
        .unwrap()
        .instance;
    let normal_camera = world
        .query_filtered::<Entity, With<DriveCamera>>()
        .single(world)
        .unwrap();
    // Validate the complete imported instance before allocating any owned resources.
    for record in &reference.mirrors.mirrors {
        let nodes = &world.resource::<ReferenceVisuals>().nodes;
        if [record.plane_node, record.camera_node, record.surface_node]
            .iter()
            .any(|node| {
                nodes
                    .get(node)
                    .is_none_or(|entity| world.get::<GlobalTransform>(*entity).is_none())
            })
        {
            return;
        }
        if world
            .get::<Children>(nodes[&record.surface_node])
            .is_none_or(|children| {
                !children
                    .iter()
                    .any(|entity| world.get::<Mesh3d>(entity).is_some())
            })
        {
            return;
        }
    }
    let mut bindings = Vec::with_capacity(3);
    let mut surfaces = Vec::with_capacity(3);
    for record in &reference.mirrors.mirrors {
        let nodes = &world.resource::<ReferenceVisuals>().nodes;
        let plane = nodes[&record.plane_node];
        let camera_node = nodes[&record.camera_node];
        let screen_node = nodes[&record.surface_node];
        let Some(screen) = world.get::<Children>(screen_node).and_then(|children| {
            children
                .iter()
                .find(|entity| world.get::<Mesh3d>(*entity).is_some())
        }) else {
            return;
        };
        assert!(
            world.get::<super::reference::BodyVisual>(plane).is_some(),
            "checked mirror plane body root"
        );
        assert!(
            world.get::<ChildOf>(plane).is_none(),
            "mirror body transform must be detached"
        );
        let reflected = world
            .get::<GlobalTransform>(screen)
            .is_some_and(|t| t.affine().matrix3.determinant() < 0.);
        let mask = 1;
        let mut image = Image::new_target_texture(256, 256, TextureFormat::Rgba8Unorm, None);
        image.asset_usage = RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD;
        image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
            mag_filter: ImageFilterMode::Linear,
            min_filter: ImageFilterMode::Linear,
            address_mode_u: ImageAddressMode::ClampToEdge,
            address_mode_v: ImageAddressMode::ClampToEdge,
            ..default()
        });
        let target = world.resource_mut::<Assets<Image>>().add(image);
        let source = world.resource::<SourceMaterials>();
        let material_record = reference
            .rendering
            .materials
            .iter()
            .find(|m| {
                m.asset == record.live_material.asset
                    && m.gltf_material == record.live_material.gltf_material
            })
            .expect("checked mirror material");
        let mut live =
            make_mirror_material(material_record, source, target.clone(), reflected, mask);
        let mut disabled = authored_material(
            (
                record.disabled_material.asset,
                record.disabled_material.gltf_material,
            ),
            reflected,
            mask,
            source,
        );
        live.mirror = Some((instance, bindings.len(), true));
        disabled.mirror = Some((instance, bindings.len(), false));
        let live = world.resource_mut::<Assets<SourceMaterial>>().add(live);
        let disabled = world.resource_mut::<Assets<SourceMaterial>>().add(disabled);
        world
            .entity_mut(screen)
            .insert(MeshMaterial3d(disabled.clone()));
        let camera = world
            .spawn((
                Camera3d::default(),
                bevy::light::cluster::ClusterConfig::Single,
                bevy::render::camera::CameraRenderGraph::new(super::surface_graph::SourceSurface),
                super::surface_views::SourceSurfaceView {
                    index: super::surface_views::MIRROR_START + bindings.len(),
                    near: record.near_plane,
                    far: record.initial_far_plane,
                    nonjittered_clip_from_world: Mat4::IDENTITY,
                    shadow_fade: Vec4::ZERO,
                    shadow_fade_center: Vec4::ZERO,
                },
                Camera {
                    order: -6 + bindings.len() as isize,
                    is_active: false,
                    clear_color: ClearColorConfig::Custom(Color::srgba(
                        record.clear_color[0],
                        record.clear_color[1],
                        record.clear_color[2],
                        record.clear_color[3],
                    )),
                    ..default()
                },
                RenderTarget::Image(target.clone().into()),
                MirrorViewRole::Capture(bindings.len()),
                MirrorInstance(instance),
                Projection::custom(MirrorProjection {
                    perspective: PerspectiveProjection {
                        fov: record.serialized_fov_degrees.to_radians(),
                        near: record.near_plane,
                        far: record.initial_far_plane,
                        aspect_ratio: 1.,
                        ..default()
                    },
                }),
                Transform::default(),
                Msaa::Off,
                (0..32)
                    .filter(|layer| record.culling_mask & (1u32 << layer) != 0)
                    .collect::<RenderLayers>(),
                bevy::core_pipeline::tonemapping::Tonemapping::None,
            ))
            .id();
        if record.hdr {
            world.entity_mut(camera).insert(bevy::camera::Hdr);
        }
        surfaces.push((screen, live.id(), disabled.id()));
        let mesh = world.get::<Mesh3d>(screen).unwrap().0.id();
        super::materials::warm_mirror_material(world, mesh, live.id());
        super::materials::warm_mirror_material(world, mesh, disabled.id());
        bindings.push(MirrorBinding {
            plane,
            camera_node,
            screen,
            camera,
            target,
            live,
            disabled,
        });
    }
    info!(
        "Bound source mirror targets: {:?}",
        bindings
            .iter()
            .map(|binding| binding.target.id())
            .collect::<Vec<_>>()
    );
    world.insert_resource(MirrorSurfaces {
        instance,
        entries: Arc::new(surfaces),
    });
    world.insert_resource(SourceMirrors {
        instance,
        normal_camera,
        inputs: vec![MirrorInput::default(); bindings.len()],
        native: MirrorState::new(&reference.mirrors),
        bindings,
    });
}

pub fn sync_mirror_views(world: &mut World, windows: &mut QueryState<&'static Window>) {
    let Some(mut mirrors) = world.remove_resource::<SourceMirrors>() else {
        return;
    };
    let reference = world.resource::<RuntimeReference>().0.clone();
    let available = world
        .get::<Camera>(mirrors.normal_camera)
        .is_some_and(|camera| camera.is_active)
        && windows
            .iter(world)
            .any(|window| window.physical_width() > 0 && window.physical_height() > 0);
    let drive = world.resource::<Drive>();
    let started = drive.runtime.started;
    let playing = started && !drive.runtime.paused && available;
    let ready = world.resource::<MirrorGpuStatus>().ready();
    let warming = available && !ready;
    let player = drive
        .runtime
        .area
        .player
        .body_pose(&drive.runtime.physics)
        .position;
    let head = world
        .get::<GlobalTransform>(mirrors.normal_camera)
        .map(|pose| pose.translation().to_array())
        .unwrap_or(player);
    // Compute normal visibility from its current projection, before capture camera visibility runs.
    let frustum = world
        .get::<GlobalTransform>(mirrors.normal_camera)
        .zip(world.get::<Projection>(mirrors.normal_camera))
        .map(|(pose, projection)| projection.compute_frustum(pose));
    for (index, binding) in mirrors.bindings.iter().enumerate() {
        mirrors.inputs[index] = MirrorInput::default();
        let Some(plane) = world.get::<GlobalTransform>(binding.plane) else {
            continue;
        };
        let Some(camera) = world.get::<GlobalTransform>(binding.camera_node) else {
            continue;
        };
        // Physical mirror planes are detached body roots. Rotation excludes retained signed scale.
        let rotation = world.get::<Transform>(binding.plane).unwrap().rotation;
        let visible = world
            .get::<InheritedVisibility>(binding.screen)
            .is_some_and(|v| v.get())
            && frustum.as_ref().is_some_and(|frustum| {
                match (
                    world.get::<bevy::camera::primitives::Aabb>(binding.screen),
                    world.get::<GlobalTransform>(binding.screen),
                ) {
                    (Some(bounds), Some(pose)) => {
                        frustum.intersects_obb(bounds, &pose.affine(), true, true)
                    }
                    _ => false,
                }
            });
        mirrors.inputs[index] = MirrorInput {
            plane_position: plane.translation().to_array(),
            plane_normal: (rotation * Vec3::Y).to_array(),
            camera_up: (rotation * Vec3::NEG_Z).to_array(),
            camera_position: camera.translation().to_array(),
            normal_view_visible: visible,
        };
    }
    mirrors.native.update(
        &reference.mirrors.rules,
        MirrorFrame {
            player_position: player,
            view_head_position: head,
            source_enabled: true,
            pointer_locked: playing && ready,
        },
        &mirrors.inputs,
    );
    for (binding, output) in mirrors.bindings.iter().zip(mirrors.native.outputs()) {
        let pose = super::transform(output.pose);
        let mut camera = world.entity_mut(binding.camera);
        camera.insert((pose, GlobalTransform::from(pose)));
        let mut settings = camera.get_mut::<Camera>().unwrap();
        let valid_clip = output.has_valid_clip_range();
        settings.is_active = output.pose_valid && valid_clip && (output.capture || warming);
        settings.order = if output.capture {
            -6 + output.rank as isize
        } else {
            -6
        };
        if let Projection::Custom(projection) = &mut *camera.get_mut::<Projection>().unwrap() {
            let projection = projection.get_mut::<MirrorProjection>().unwrap();
            projection.perspective.fov = output.fov_degrees.to_radians();
            projection.perspective.near = output.capture_near_plane;
            if valid_clip {
                projection.perspective.far = output.capture_far_plane;
            }
        }
        let selected = if output.capture && valid_clip && ready {
            &binding.live
        } else {
            &binding.disabled
        };
        if world
            .get::<MeshMaterial3d<SourceMaterial>>(binding.screen)
            .is_some_and(|m| m.0.id() != selected.id())
        {
            world
                .entity_mut(binding.screen)
                .insert(MeshMaterial3d(selected.clone()));
        }
    }
    world.insert_resource(mirrors);
}

use bevy::core_pipeline::{
    core_3d::{
        AlphaMask3d, Opaque3d, Opaque3dBatchSetKey, Opaque3dBinKey, Transparent3d,
        TransparentSortingInfo3d,
    },
    prepass::{OpaqueNoLightmap3dBatchSetKey, OpaqueNoLightmap3dBinKey},
};
use bevy::ecs::system::{SystemParamItem, lifetimeless::SRes};
use bevy::material::{
    OpaqueRendererMethod, RenderPhaseType,
    key::{ErasedMaterialPipelineKey, ErasedMeshPipelineKey},
};
use bevy::pbr::*;
use bevy::render::{
    batching::gpu_preprocessing::{BatchedInstanceBuffers, GpuPreprocessingSupport},
    erased_render_asset::ErasedRenderAssets,
    mesh::{RenderMesh, allocator::MeshAllocator},
    render_asset::RenderAssets,
    render_phase::*,
    renderer::RenderDevice,
    sync_world::MainEntity,
    view::{ExtractedView, RenderVisibleEntities},
};

#[derive(Resource, Default)]
struct CapturePipelines {
    instance: u64,
    sealed: bool,
    sources: HashSet<CachedRenderPipelineId>,
    owner_sources: HashMap<(usize, CachedRenderPipelineId), CachedRenderPipelineId>,
}

fn warm_capture_pipelines(
    world: &mut World,
    cache: &mut CapturePipelines,
    surfaces: &MirrorSurfaces,
) {
    if cache.sealed {
        return;
    }
    let status = world.resource::<MirrorGpuStatus>().clone();
    let warmup = world.resource::<MirrorWarmup>().clone();
    status.0.lock().unwrap().warmup_stage = 1;
    if warmup.instance != surfaces.instance || warmup.pairs.is_empty() {
        return;
    }
    let mut views = world.query::<(&ExtractedView, &MirrorViewRole, Option<&MirrorInstance>)>();
    let keys = world.resource::<ViewKeyCache>();
    let mut view_keys = Vec::new();
    let mut owners = [false; 3];
    status.0.lock().unwrap().warmup_stage = 2;
    for (view, role, stamp) in views.iter(world) {
        if let MirrorViewRole::Capture(owner) = role {
            if stamp.is_none_or(|s| s.0 != surfaces.instance) {
                continue;
            }
            owners[*owner] = true;
        }
        let Some(key) = keys.get(&view.retained_view_entity) else {
            return;
        };
        if !view_keys.contains(key) {
            view_keys.push(*key);
        }
    }
    if !owners.iter().all(|v| *v) {
        return;
    }
    status.0.lock().unwrap().warmup_stage = 3;
    let mut work = Vec::new();
    for (mesh_id, material_id) in warmup.pairs.iter() {
        let Some(mesh) = world.resource::<RenderAssets<RenderMesh>>().get(*mesh_id) else {
            return;
        };
        let Some(material) = world
            .resource::<ErasedRenderAssets<PreparedMaterial>>()
            .get(*material_id)
        else {
            status.0.lock().unwrap().warmup_stage = 4;
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
                *mesh_id,
                *material_id,
                *view_key,
                ErasedMaterialPipelineKey {
                    type_id: material_id.type_id(),
                    mesh_key: ErasedMeshPipelineKey::new(mesh_key),
                    material_key: material.properties.material_key.clone(),
                },
                mesh.layout.clone(),
                material.properties.clone(),
            ));
        }
    }
    let mut ids = HashMap::new();
    for (mesh, material, view, key, layout, properties) in work {
        match base_specialize(world, key, &layout, &properties) {
            Ok(id) => {
                cache.sources.insert(id);
                ids.insert((mesh, material, view), id);
            }
            Err(error) => {
                world
                    .resource::<MirrorGpuStatus>()
                    .0
                    .lock()
                    .unwrap()
                    .failure = Some(format!("Mirror loading specialization: {error}"));
                return;
            }
        }
    }
    for (owner, (mesh, live, disabled)) in warmup.owners.iter().enumerate() {
        for view in &view_keys {
            let disabled_id = ids[&(*mesh, disabled.untyped(), *view)];
            for material in [live, disabled] {
                cache.owner_sources.insert(
                    (owner, ids[&(*mesh, material.untyped(), *view)]),
                    disabled_id,
                );
            }
        }
    }
    cache.sealed = true;
    status.0.lock().unwrap().warmup_stage = 5;
    info!(
        "Mirror loading contract: {} mesh/material pairs, {} view keys, {} source pipelines",
        warmup.pairs.len(),
        view_keys.len(),
        cache.sources.len()
    );
}

fn prepare_capture_pipelines(
    world: &mut World,
    views: &mut QueryState<(
        &'static ExtractedView,
        &'static MirrorViewRole,
        &'static MirrorInstance,
    )>,
) {
    let mut cache = world.remove_resource::<CapturePipelines>().unwrap();
    let surfaces = world.resource::<MirrorSurfaces>().clone();
    if cache.instance != surfaces.instance {
        cache.instance = surfaces.instance;
        cache.sealed = false;
        cache.owner_sources.clear();
        cache.sources.clear();
    }
    warm_capture_pipelines(world, &mut cache, &surfaces);
    let mut capture_views = [None; 3];
    for (view, role, stamp) in views.iter(world) {
        if stamp.0 != surfaces.instance {
            continue;
        }
        if let MirrorViewRole::Capture(owner) = role {
            capture_views[*owner] = Some(view.retained_view_entity);
        }
    }
    world.resource_scope(|world, mut views: Mut<SpecializedMaterialPipelineCache>| {
        for (owner, view) in capture_views.into_iter().enumerate() {
            let Some(pipelines) = view.and_then(|view| views.get_mut(&view)) else {
                continue;
            };
            for (entity, original) in pipelines.iter_mut() {
                let source_id = if surfaces
                    .entries
                    .get(owner)
                    .is_some_and(|entry| MainEntity::from(entry.0) == *entity)
                {
                    cache.owner_sources.get(&(owner, *original)).copied()
                } else {
                    Some(*original)
                };
                if let Some(id) = source_id.filter(|id| cache.sources.contains(id)) {
                    *original = id;
                } else if cache.sealed && source_id.is_none_or(|id| !cache.sources.contains(&id)) {
                    let mut status = world.resource::<MirrorGpuStatus>().0.lock().unwrap();
                    if status.failure.is_none() {
                        status.failure = Some(format!(
                            "Mirror pipeline outside loading contract: {original:?}"
                        ));
                        status.ready = false;
                    }
                }
            }
        }
    });
    world.insert_resource(cache);
}

pub(crate) struct SetMirrorMaterial<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetMirrorMaterial<I> {
    type Param = (
        SRes<ErasedRenderAssets<PreparedMaterial>>,
        SRes<RenderMaterialInstances>,
        SRes<MaterialBindGroupAllocators>,
        SRes<MirrorSurfaces>,
        SRes<super::sky::SkyVariants>,
    );
    type ViewQuery = Option<&'static MirrorViewRole>;
    type ItemQuery = ();
    fn render<'w>(
        item: &P,
        role: Option<&'w MirrorViewRole>,
        _: Option<()>,
        (materials, instances, allocators, surfaces, sky): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(instance) = instances.into_inner().instances.get(&item.main_entity()) else {
            return RenderCommandResult::Skip;
        };
        let role = role.copied().unwrap_or(MirrorViewRole::Normal);
        let selected = resolve_mirror_material(
            role,
            item.main_entity(),
            instance.asset_id,
            surfaces.into_inner(),
        );
        let selected = sky.into_inner().resolve(role, selected);
        let Some(material) = materials.into_inner().get(selected) else {
            return RenderCommandResult::Skip;
        };
        let Some(allocator) = allocators.into_inner().get(&selected.type_id()) else {
            return RenderCommandResult::Skip;
        };
        let Some(group) = allocator.get(material.binding.group) else {
            return RenderCommandResult::Skip;
        };
        let Some(binding) = group.bind_group() else {
            return RenderCommandResult::Skip;
        };
        pass.set_bind_group(I, binding, &[]);
        RenderCommandResult::Success
    }
}
type DrawMirrorMaterial = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshViewBindingArrayBindGroup<1>,
    SetMeshBindGroup<2>,
    SetMirrorMaterial<3>,
    DrawMesh,
);

// Phase insertion follows the pinned Bevy 0.19.1 material queue.
fn queue_capture_meshes(
    render_materials: Res<ErasedRenderAssets<PreparedMaterial>>,
    render_mesh_instances: Res<RenderMeshInstances>,
    render_material_instances: Res<RenderMaterialInstances>,
    mesh_assets: Res<RenderAssets<RenderMesh>>,
    mesh_allocator: Res<MeshAllocator>,
    gpu_preprocessing_support: Res<GpuPreprocessingSupport>,
    maybe_batched_instance_buffers: Option<
        Res<BatchedInstanceBuffers<MeshUniform, MeshInputUniform>>,
    >,
    mut opaque_render_phases: ResMut<ViewBinnedRenderPhases<Opaque3d>>,
    mut alpha_mask_render_phases: ResMut<ViewBinnedRenderPhases<AlphaMask3d>>,
    mut transmissive_render_phases: ResMut<ViewSortedRenderPhases<Transmissive3d>>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
    views: Query<(
        &ExtractedView,
        &RenderVisibleEntities,
        &MirrorViewRole,
        &MirrorInstance,
    )>,
    specialized_material_pipeline_cache: ResMut<SpecializedMaterialPipelineCache>,
    (surfaces, sky, captures): (
        Res<MirrorSurfaces>,
        Res<super::sky::SkyVariants>,
        Res<CapturePipelines>,
    ),
    (draw_opaque, draw_mask, draw_transparent, draw_transmissive): (
        Res<DrawFunctions<Opaque3d>>,
        Res<DrawFunctions<AlphaMask3d>>,
        Res<DrawFunctions<Transparent3d>>,
        Res<DrawFunctions<Transmissive3d>>,
    ),
) {
    for (view, visible_entities, role, stamp) in &views {
        if stamp.0 != surfaces.instance {
            continue;
        }
        let MirrorViewRole::Capture(_) = role else {
            continue;
        };
        let (
            Some(opaque_phase),
            Some(alpha_mask_phase),
            Some(transmissive_phase),
            Some(transparent_phase),
        ) = (
            opaque_render_phases.get_mut(&view.retained_view_entity),
            alpha_mask_render_phases.get_mut(&view.retained_view_entity),
            transmissive_render_phases.get_mut(&view.retained_view_entity),
            transparent_render_phases.get_mut(&view.retained_view_entity),
        )
        else {
            continue;
        };

        let Some(view_specialized_material_pipeline_cache) =
            specialized_material_pipeline_cache.get(&view.retained_view_entity)
        else {
            continue;
        };

        let Some(render_visible_mesh_entities) = visible_entities.get::<Mesh3d>() else {
            continue;
        };

        for (_render_entity, visible_entity) in render_visible_mesh_entities
            .entities_cpu_culling
            .iter()
            .map(|(render, main)| (render, main))
            .chain(
                render_visible_mesh_entities
                    .entities_gpu_culling
                    .iter()
                    .map(|(main, render)| (render, main)),
            )
        {
            opaque_phase.remove(*visible_entity);
            alpha_mask_phase.remove(*visible_entity);
            transmissive_phase.remove(Entity::PLACEHOLDER, *visible_entity);
            transparent_phase.remove(Entity::PLACEHOLDER, *visible_entity);
            let Some(pipeline_id) = view_specialized_material_pipeline_cache
                .get(visible_entity)
                .copied()
            else {
                continue;
            };

            if !captures.sources.contains(&pipeline_id) {
                continue;
            }
            // Check for material instance, mesh, and material. If any of these
            // fail, it's probably because the relevant asset hasn't loaded yet.
            // Retry from the retained visible set on the next frame.
            let Some(material_instance) = render_material_instances.instances.get(visible_entity)
            else {
                continue;
            };
            let Some(mesh_instance) = render_mesh_instances.render_mesh_queue_data(*visible_entity)
            else {
                continue;
            };
            let selected = resolve_mirror_material(
                *role,
                *visible_entity,
                material_instance.asset_id,
                &surfaces,
            );
            let selected = sky.resolve(*role, selected);
            let Some(material) = render_materials.get(selected) else {
                continue;
            };

            // Fetch the slabs that this mesh resides in.
            let Some(mesh_slabs) = mesh_allocator.mesh_slabs(&mesh_instance.mesh_asset_id()) else {
                continue;
            };

            match material.properties.render_phase_type {
                RenderPhaseType::Transmissive => {
                    let draw_function = draw_transmissive.read().id::<DrawMirrorMaterial>();
                    transmissive_phase.add_retained(Transmissive3d {
                        sorting_info: TransparentSortingInfo3d::Sorted {
                            mesh_center: get_mesh_instance_world_from_local(
                                *visible_entity,
                                mesh_instance.current_uniform_index,
                                &render_mesh_instances,
                                maybe_batched_instance_buffers.as_deref(),
                            )
                            .transform_point3(
                                mesh_assets
                                    .get(mesh_instance.mesh_asset_id())
                                    .unwrap()
                                    .aabb_center,
                            ),
                            depth_bias: material.properties.depth_bias,
                        },
                        entity: (Entity::PLACEHOLDER, *visible_entity),
                        draw_function,
                        pipeline: pipeline_id,
                        batch_range: 0..1,
                        extra_index: PhaseItemExtraIndex::None,
                        indexed: mesh_slabs.index_slab_id.is_some(),
                        // Filled in later.
                        distance: 0.0,
                    });
                }
                RenderPhaseType::Opaque => {
                    if material.properties.render_method == OpaqueRendererMethod::Deferred {
                        // Even though we aren't going to insert the entity into
                        // a bin, we still want to update its cache entry. That
                        // way, we know we don't need to re-examine it in future
                        // frames.
                        opaque_phase.update_cache(*visible_entity, None);
                        continue;
                    }
                    let draw_function =
                        if selected.type_id() == std::any::TypeId::of::<SourceMaterial>() {
                            draw_opaque
                                .read()
                                .id::<super::source_lighting::DrawSourceMaterial>()
                        } else {
                            draw_opaque.read().id::<DrawMirrorMaterial>()
                        };
                    let batch_set_key = Opaque3dBatchSetKey {
                        pipeline: pipeline_id,
                        draw_function,
                        material_bind_group_index: Some(material.binding.group.0),
                        slabs: mesh_slabs,
                        lightmap_slab: mesh_instance
                            .shared
                            .lightmap_slab_index()
                            .map(|index| *index),
                    };
                    let bin_key = Opaque3dBinKey {
                        asset_id: mesh_instance.mesh_asset_id().into(),
                    };
                    opaque_phase.add(
                        batch_set_key,
                        bin_key,
                        (Entity::PLACEHOLDER, *visible_entity),
                        mesh_instance.current_uniform_index,
                        BinnedRenderPhaseType::mesh(
                            mesh_instance.should_batch(),
                            &gpu_preprocessing_support,
                        ),
                    );
                }
                // Alpha mask
                RenderPhaseType::AlphaMask => {
                    let draw_function =
                        if selected.type_id() == std::any::TypeId::of::<SourceMaterial>() {
                            draw_mask
                                .read()
                                .id::<super::source_lighting::DrawSourceMaterial>()
                        } else {
                            draw_mask.read().id::<DrawMirrorMaterial>()
                        };
                    let batch_set_key = OpaqueNoLightmap3dBatchSetKey {
                        draw_function,
                        pipeline: pipeline_id,
                        material_bind_group_index: Some(material.binding.group.0),
                        slabs: mesh_slabs,
                    };
                    let bin_key = OpaqueNoLightmap3dBinKey {
                        asset_id: mesh_instance.mesh_asset_id().into(),
                    };
                    alpha_mask_phase.add(
                        batch_set_key,
                        bin_key,
                        (Entity::PLACEHOLDER, *visible_entity),
                        mesh_instance.current_uniform_index,
                        BinnedRenderPhaseType::mesh(
                            mesh_instance.should_batch(),
                            &gpu_preprocessing_support,
                        ),
                    );
                }
                RenderPhaseType::Transparent => {
                    let draw_function = if selected.type_id()
                        == std::any::TypeId::of::<super::plates::PlateTextMaterial>()
                    {
                        draw_transparent
                            .read()
                            .id::<super::source_lighting::DrawSourcePlate>()
                    } else {
                        draw_transparent.read().id::<DrawMirrorMaterial>()
                    };
                    transparent_phase.add_retained(Transparent3d {
                        sorting_info: TransparentSortingInfo3d::Sorted {
                            mesh_center: get_mesh_instance_world_from_local(
                                *visible_entity,
                                mesh_instance.current_uniform_index,
                                &render_mesh_instances,
                                maybe_batched_instance_buffers.as_deref(),
                            )
                            .transform_point3(
                                mesh_assets
                                    .get(mesh_instance.mesh_asset_id())
                                    .unwrap()
                                    .aabb_center,
                            ),
                            depth_bias: material.properties.depth_bias,
                        },
                        entity: (Entity::PLACEHOLDER, *visible_entity),
                        draw_function,
                        pipeline: pipeline_id,
                        batch_range: 0..1,
                        extra_index: PhaseItemExtraIndex::None,
                        indexed: mesh_slabs.index_slab_id.is_some(),
                        // Filled in later.
                        distance: 0.0,
                    });
                }
            }
        }
    }
}

#[derive(Clone, Copy, Default)]
struct PreparedBanks {
    views: [Option<TextureViewId>; 16],
    target: Option<TextureViewId>,
    count: usize,
}
#[derive(Default)]
struct MirrorGpuData {
    instance: u64,
    banks: [[Option<PreparedBanks>; 2]; 3],
    submitted: [bool; 3],
    encoded: [u32; 3],
    sky_frames: [u32; 3],
    validation: [u32; 3],
    ready: bool,
    failure: Option<String>,
    warmup_stage: u8,
    capture_pipelines: u32,
    pipelines_pending: u32,
    obsolete_pairs_removed: u32,
}
#[derive(serde::Serialize)]
pub(crate) struct MirrorAudit {
    instance: u64,
    ready: bool,
    banks_mask: u8,
    warmup_stage: u8,
    capture_pipelines: u32,
    pipelines_pending: u32,
    obsolete_pairs_removed: u32,
    sky_frames: [u32; 3],
    encoded: [u32; 3],
    validation: [u32; 3],
    submitted: [bool; 3],
}
#[derive(Resource, Clone, Default, ExtractResource)]
pub struct MirrorGpuStatus(Arc<Mutex<MirrorGpuData>>);
impl MirrorGpuStatus {
    pub(crate) fn audit(&self) -> MirrorAudit {
        let status = self.0.lock().unwrap();
        let mut banks_mask = 0;
        for (owner, pair) in status.banks.iter().enumerate() {
            for (live, bank) in pair.iter().enumerate() {
                banks_mask |= u8::from(bank.is_some()) << (owner * 2 + live);
            }
        }
        MirrorAudit {
            instance: status.instance,
            ready: status.ready,
            banks_mask,
            warmup_stage: status.warmup_stage,
            capture_pipelines: status.capture_pipelines,
            pipelines_pending: status.pipelines_pending,
            obsolete_pairs_removed: status.obsolete_pairs_removed,
            sky_frames: status.sky_frames,
            encoded: status.encoded,
            validation: status.validation,
            submitted: status.submitted,
        }
    }
    pub(crate) fn sky_encoded(&self, instance: u64, owner: usize, frame: u32) {
        let mut status = self.0.lock().unwrap();
        if status.instance == instance {
            status.sky_frames[owner] = frame;
        }
    }
    pub(crate) fn sky_ready(&self, instance: u64, owner: usize, frame: u32) -> bool {
        let status = self.0.lock().unwrap();
        status.instance == instance && status.sky_frames[owner] == frame && frame != 0
    }
    pub(crate) fn encoded(&self, instance: u64, owner: usize, frame: u32) {
        let mut status = self.0.lock().unwrap();
        if status.instance == instance {
            status.encoded[owner] = frame;
        }
    }
    pub fn ready(&self) -> bool {
        self.0.lock().unwrap().ready
    }
    pub fn failure(&self) -> Option<String> {
        self.0.lock().unwrap().failure.clone()
    }
    pub fn record(&self, instance: u64, owner: usize, live: bool, bindings: &BindingResources) {
        let mut banks = PreparedBanks::default();
        for (binding, resource) in bindings.iter() {
            if let OwnedBindingResource::TextureView(_, view) = resource {
                banks.views[banks.count] = Some(view.id());
                banks.count += 1;
                if *binding == 1 && live {
                    banks.target = Some(view.id());
                }
            }
        }
        let mut status = self.0.lock().unwrap();
        if status.instance == instance {
            status.banks[owner][live as usize] = Some(banks);
        }
    }
}
fn finish_capture_frame(
    status: Res<MirrorGpuStatus>,
    captures: Res<CapturePipelines>,
    pipelines: Res<PipelineCache>,
    surfaces: Res<MirrorSurfaces>,
    frame: Res<super::view_frame::SourceViewFrame>,
    gate: Res<super::source_lighting::SourceExecutionGate>,
    queue: Res<bevy::render::renderer::RenderQueue>,
) {
    let mut status = status.0.lock().unwrap();
    if status.instance != surfaces.instance || status.failure.is_some() {
        return;
    }
    status.capture_pipelines = captures.sources.len() as u32;
    status.pipelines_pending = 0;
    if status
        .banks
        .iter()
        .any(|pair| pair.iter().any(Option::is_none))
        || !captures.sealed
        || captures.sources.is_empty()
    {
        return;
    }
    for id in &captures.sources {
        match pipelines.get_render_pipeline_state(*id) {
            CachedPipelineState::Ok(_) => {}
            CachedPipelineState::Err(error) => {
                status.failure = Some(format!("Mirror capture pipeline: {error}"));
                status.ready = false;
                return;
            }
            _ => {
                status.pipelines_pending += 1;
                return;
            }
        }
    }
    if status.ready {
        return;
    }
    for owner in 0..3 {
        let target = status.banks[owner][1].unwrap().target;
        for surface in 0..3 {
            let selected = status.banks[surface][(surface != owner) as usize].unwrap();
            if selected.views[..selected.count].contains(&target) {
                status.failure = Some(format!(
                    "Mirror {owner} writable target occurs in prepared surface {surface}"
                ));
                return;
            }
            if surface != owner
                && !selected.views[..selected.count]
                    .contains(&status.banks[surface][1].unwrap().target)
            {
                status.failure = Some("Other mirror target missing from prepared bindings".into());
                return;
            }
        }
        let live = status.banks[owner][1].unwrap();
        if target.is_none()
            || live.views[..live.count]
                .iter()
                .filter(|id| **id == target)
                .count()
                != 1
        {
            status.failure = Some("Mirror target must use one sampled bank and sampler".into());
            return;
        }
    }
    for index in 0..3 {
        if status.encoded[index] == frame.frame && frame.frame != 0 && status.validation[index] == 0
        {
            status.validation[index] = gate.watch_submission(&queue);
        }
        status.submitted[index] = gate.submission_validated(status.validation[index]);
    }
    if status.submitted.iter().all(|submitted| *submitted) {
        status.ready = true;
        info!(
            "Source mirrors ready: 3 submitted targets; complete prepared banks feedback-safe; {} retained capture pipelines",
            captures.sources.len()
        );
    }
}

#[derive(Resource, Default)]
pub(crate) struct MirrorOutputs(pub(crate) [Option<super::frame_copy::FrameCopy>; 3]);

fn prepare_capture_outputs(
    mut outputs: ResMut<MirrorOutputs>,
    targets: Option<Res<super::surface_views::SurfaceResources>>,
    assets: Option<Res<super::lighting_assets::LightingRenderAssets>>,
    device: Res<RenderDevice>,
    cache: Res<PipelineCache>,
    status: Res<MirrorGpuStatus>,
) {
    let (Some(targets), Some(assets)) = (targets, assets) else {
        return;
    };
    for (index, output) in outputs.0.iter_mut().enumerate() {
        if let Some(output) = output {
            if let Some(error) = output.failure(&cache) {
                status.0.lock().unwrap().failure = Some(error);
            }
            continue;
        }
        *output = Some(super::frame_copy::FrameCopy::new(
            &device,
            &cache,
            assets.shaders.copy.clone(),
            targets.targets.mirrors[index]
                .resolved
                .sampled
                .as_ref()
                .unwrap(),
            TextureFormat::Rgba8Unorm,
            1,
        ));
    }
}
