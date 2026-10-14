//! Source camera stack. GPU tokens are acknowledged after the render queue submission.
use super::lighting_scene::PROBE_COUNT;
use super::surface_views::{MIRROR_START, VIEW_COUNT};
use super::{
    frame_copy::FrameCopy,
    lighting_assets::LightingRenderAssets,
    materials::RenderingState,
    motion_draw::{MotionDrawView, MotionObjects, MotionPipelines},
    motion_vectors::SourceMotionPass,
    probes::{ProbeFilter, ProbePublication},
    source_lighting::{SourceExecutionGate, SourceLightingStatus, SourceViewCamera},
    source_sky::SkyPass,
    surface_graph::{SourceSurface, SourceSurfaceStage},
    surface_views::{SourceSurfaceView, SurfaceResources},
    view_frame::{SourceSubmissionAcks, SourceViewFrame},
};
use crate::reference::reflections::ProbeJob;
use bevy::camera::{RenderTarget, visibility::RenderLayers};
use bevy::core_pipeline::core_3d::Transparent3d;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::render::camera::CameraRenderGraph;
use bevy::render::{
    Render, RenderApp, RenderSystems,
    extract_component::ExtractComponent,
    render_phase::{DrawFunctions, PhaseItem, SortedPhaseItem, ViewSortedRenderPhases},
    render_resource::*,
    renderer::{
        RenderContext, RenderDevice, RenderGraph, RenderGraphSystems, RenderQueue, ViewQuery,
    },
    sync_world::MainEntity,
    view::{ExtractedView, ViewTarget},
};
use bevy::{ecs as bevy_ecs, prelude::*};
use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicU32, Ordering},
};

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SourceSkySurface;
#[derive(Component, Clone, Copy)]
pub struct SourceSkyCamera;
impl bevy::render::sync_component::SyncComponent for SourceSkyCamera {
    type Target = Self;
}
impl ExtractComponent for SourceSkyCamera {
    type QueryData = &'static Self;
    type QueryFilter = ();
    type Out = Self;
    fn extract_component(_: bevy::ecs::query::QueryItem<'_, '_, Self::QueryData>) -> Option<Self> {
        Some(Self)
    }
}
#[derive(Component, Clone, Copy)]
pub struct SourceSkyExtra(pub u32);
impl bevy::render::sync_component::SyncComponent for SourceSkyExtra {
    type Target = Self;
}
impl ExtractComponent for SourceSkyExtra {
    type QueryData = &'static Self;
    type QueryFilter = ();
    type Out = Self;
    fn extract_component(
        item: bevy::ecs::query::QueryItem<'_, '_, Self::QueryData>,
    ) -> Option<Self> {
        Some(*item)
    }
}
#[derive(Resource, Default)]
struct SkyExtras(bevy::ecs::entity::EntityHashMap<(Entity, u32)>);
#[derive(Default, serde::Serialize)]
struct FrameProgress {
    scene: u32,
    extent: [u32; 2],
    first_token: u32,
    first_jitter: [f32; 2],
    jitter: [f32; 2],
    frame: u32,
    mask: u16,
    sky: bool,
    presented: bool,
    token: u32,
    validation: u32,
    validated: bool,
    views: [ViewAudit; VIEW_COUNT],
    cpu_boundary: CpuBoundary,
    gpu_boundary: GpuBoundary,
    probes: [ProbeAudit; PROBE_COUNT],
    motion: MotionAudit,
    effects: super::source_effects::EffectsAudit,
}
#[derive(Resource, Clone, Default)]
pub struct SourceGraphProgress(Arc<Mutex<FrameProgress>>);
impl SourceGraphProgress {
    pub(crate) fn frame_submission(&self) -> (u32, u32, bool) {
        let state = self.0.lock().unwrap();
        (state.frame, state.token, state.presented)
    }
    pub(crate) fn publish_effects(&self, audit: super::source_effects::EffectsAudit) {
        self.0.lock().unwrap().effects = audit;
    }
    pub(crate) fn readiness(&self, scene: u32) -> [bool; 5] {
        let s = self.0.lock().unwrap();
        if s.scene != scene {
            return [false; 5];
        }
        let gpu = &s.gpu_boundary;
        let complete = s.presented
            && s.sky
            && s.mask == 7
            && s.token != 0
            && s.motion.frame == s.frame
            && s.motion.submitted_token == s.token
            && s.motion.validated
            && s.effects.complete(s.frame, s.token)
            && s.validated;
        [
            gpu.source_targets
                && gpu.lighting_textures != 0
                && gpu.lighting_gpu_textures == gpu.lighting_textures,
            gpu.source_surface_ready
                && gpu.source_camera_ready
                && gpu.source_sky_ready
                && gpu.source_cloud_ready,
            gpu.source_caster_ready,
            s.effects.pipelines_ready(),
            complete,
        ]
    }
    pub fn probe_readiness(&self) -> (bool, bool) {
        let state = self.0.lock().unwrap();
        (
            state.probes.iter().all(|probe| probe.pipelines_ready),
            state
                .probes
                .iter()
                .all(|probe| !probe.enabled || probe.published),
        )
    }
    pub fn validated(&self) -> bool {
        self.0.lock().unwrap().validated
    }
}
#[derive(Resource)]
pub(crate) struct FrameResources {
    colors: Arc<super::render_targets::SourceColorTargets>,
    _sky_depth: Texture,
    sky_depth: TextureView,
    display: FrameCopy,
    display_format: TextureFormat,
}

#[derive(Clone, Copy, Default, serde::Serialize)]
struct ProbeAudit {
    enabled: bool,
    checks: u32,
    distance_squared: f32,
    token: u32,
    job: u8,
    face_mask: u8,
    mip_mask: u8,
    pipelines_ready: bool,
    filter_validation: u32,
    publication_validation: u32,
    published: bool,
    generation: u32,
    submitted_token: u32,
    published_token: u32,
}
#[derive(Resource)]
pub(crate) struct ProbeFrame {
    scene: u32,
    probes: [ProbeState; PROBE_COUNT],
}
struct ProbeState {
    filter: ProbeFilter,
    publication: ProbePublication,
    encoded_token: Option<u32>,
    publication_encoded: Option<u32>,
    filter_validation: u32,
    publication_validation: u32,
    publication_token: u32,
    submitted_token: u32,
    published_token: u32,
    sky_frames: [AtomicU32; 6],
}
impl ProbeState {
    fn accepts(&self, token: u32) -> bool {
        token != 0
            && token > self.submitted_token
            && !self.publication.waiting()
            && self.publication_validation == 0
    }
}

#[derive(Clone, Copy, Default, serde::Serialize)]
struct MotionAudit {
    first_token: u32,
    first_draws: [u32; 3],
    first_camera_registered: bool,
    pipelines_ready: bool,
    objects_ready: bool,
    frame: u32,
    camera_token: u32,
    object_token: u32,
    submitted_token: u32,
    registered_objects: u32,
    allocated_slots: u32,
    reused_slots: u32,
    draws: [u32; 3],
    extent: [u32; 2],
    validation: u32,
    validated: bool,
    resize_count: u32,
    rebase_count: u32,
}
#[derive(Resource)]
pub(crate) struct MotionFrame {
    motion_revision: u32,
    scene: u32,
    first_token: u32,
    first_draws: [u32; 3],
    first_camera_registered: bool,
    pass: Option<SourceMotionPass>,
    targets: Arc<super::render_targets::SourceViewTargets>,
    origin: [f64; 3],
    camera_frame: u32,
    camera_token: u32,
    submitted_token: u32,
    validation: u32,
    resize_count: u32,
    rebase_count: u32,
}

impl MotionFrame {
    pub(crate) fn encoded(&self, frame: u32, token: u32) -> bool {
        self.camera_frame == frame && self.camera_token == token
    }
}

pub struct SourceFramePlugin;
impl Plugin for SourceFramePlugin {
    fn build(&self, app: &mut App) {
        let progress = SourceGraphProgress::default();
        let status = app.world().resource::<SourceLightingStatus>().0.clone();
        let gate = app.world().resource::<SourceExecutionGate>().clone();
        let _ = SOURCE_DIAGNOSTICS.set(SourceDiagnostics {
            progress: progress.0.clone(),
            loading: Mutex::new(LoadingProgress::default()),
            status,
            gate,
            mirrors: app
                .world()
                .resource::<super::mirrors::MirrorGpuStatus>()
                .clone(),
        });
        app.insert_resource(progress.clone())
            .add_plugins((
                bevy::render::extract_component::ExtractComponentPlugin::<SourceSkyCamera>::default(
                ),
                bevy::render::extract_component::ExtractComponentPlugin::<SourceSkyExtra>::default(
                ),
            ))
            .add_systems(
                Update,
                bind_sky_extras.after(super::materials::bind_materials),
            );
        app.add_systems(
            Update,
            cpu_boundaries.after(super::materials::bind_authored_materials),
        );
        super::view_frame::install(app);
        super::shadow_queue::install(app);
        super::caster_material::install(app);
        super::shadow_collector::install(app);
        super::motion_draw::install(app);
        super::source_effects::install(app);
        let render = app.sub_app_mut(RenderApp);
        render
            .insert_resource(progress)
            .init_resource::<SkyExtras>()
            .add_systems(Render, gpu_boundaries.in_set(RenderSystems::Cleanup))
            .add_systems(
                Render,
                (
                    update_sky_extras,
                    prepare_frame,
                    prepare_probe,
                    prepare_motion,
                )
                    .after(RenderSystems::PrepareResources)
                    .before(RenderSystems::Render),
            )
            .add_systems(
                Render,
                sort_sky
                    .after(RenderSystems::PhaseSort)
                    .before(super::transparent::prepare_pass_order),
            )
            .add_schedule(Schedule::new(SourceSkySurface))
            .add_systems(SourceSkySurface, render_sky)
            .add_systems(
                SourceSurface,
                (
                    clear_capture
                        .before(SourceSurfaceStage::CameraDepth)
                        .run_if(surface_ready)
                        .run_if(super::shadow_queue::graph_ready),
                    render_capture_sky
                        .in_set(SourceSurfaceStage::AfterOpaque)
                        .run_if(surface_ready)
                        .run_if(super::shadow_queue::collector_encoded),
                    super::shadow_atlas::render_atlas
                        .in_set(SourceSurfaceStage::ShadowAtlas)
                        .run_if(surface_ready)
                        .run_if(super::shadow_queue::graph_ready),
                    (encode_motion_camera, encode_motion_objects)
                        .chain()
                        .in_set(SourceSurfaceStage::Motion)
                        .run_if(surface_ready)
                        .run_if(super::shadow_queue::collector_encoded),
                    finish_surface
                        .in_set(SourceSurfaceStage::AfterTransparent)
                        .run_if(surface_ready),
                ),
            )
            .add_systems(RenderGraph, begin_frame.in_set(RenderGraphSystems::Begin))
            .add_systems(
                RenderGraph,
                filter_probe
                    .after(RenderGraphSystems::Render)
                    .before(RenderGraphSystems::Submit),
            )
            .add_systems(
                RenderGraph,
                (submitted, submit_motion, super::source_effects::submitted)
                    .chain()
                    .in_set(RenderGraphSystems::Finish),
            );
    }
}

pub fn spawn_cameras(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    reference: &crate::reference::ReferenceManifest,
    pose: Transform,
) {
    let record = &reference.render_lighting;
    let sky = &record.cameras[0];
    commands.spawn((
        SourceSkyCamera,
        Camera3d::default(),
        bevy::light::cluster::ClusterConfig::None,
        bevy::camera::Hdr,
        Camera {
            order: sky.order as isize,
            ..default()
        },
        CameraRenderGraph::new(SourceSkySurface),
        RenderLayers::layer(16),
        Projection::Perspective(PerspectiveProjection {
            fov: sky.field_of_view.to_radians(),
            near: sky.near,
            far: sky.far,
            ..default()
        }),
        Transform::from_rotation(pose.rotation),
        Msaa::Sample8,
        bevy::core_pipeline::tonemapping::Tonemapping::None,
    ));
    let mut image = Image::new_fill(
        Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0; 8],
        TextureFormat::Rgba16Float,
        bevy::asset::RenderAssetUsages::default(),
    );
    image.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT;
    let capture = images.add(image);
    for index in 0..MIRROR_START {
        let (near, far, fov, mask, order, samples) = if index < 3 {
            let c = &record.cameras[index + 1];
            (
                c.near,
                c.far,
                c.field_of_view,
                c.mask,
                c.order as isize,
                Msaa::Sample8,
            )
        } else {
            let p = &record.probes[0];
            (p.near, p.far, 90., p.mask, 10 + index as isize, Msaa::Off)
        };
        let mut entity = commands.spawn((
            Camera3d::default(),
            bevy::light::cluster::ClusterConfig::Single,
            bevy::camera::Hdr,
            Camera { order, ..default() },
            CameraRenderGraph::new(SourceSurface),
            (0..32)
                .filter(|layer| mask & (1u32 << layer) != 0)
                .collect::<RenderLayers>(),
            Projection::Perspective(PerspectiveProjection {
                fov: fov.to_radians(),
                near,
                far,
                ..default()
            }),
            pose,
            samples,
            bevy::core_pipeline::tonemapping::Tonemapping::None,
            SourceSurfaceView {
                index,
                near,
                far,
                nonjittered_clip_from_world: Mat4::IDENTITY,
                shadow_fade: Vec4::ZERO,
                shadow_fade_center: Vec4::ZERO,
            },
        ));
        if index == 1 {
            entity.insert(super::camera::DriveCamera);
        }
        if index >= 3 {
            entity.insert(RenderTarget::Image(capture.clone().into()));
        }
    }
}

pub fn sync_stack_cameras(
    main: Query<&Transform, (With<super::camera::DriveCamera>, Without<SourceSkyCamera>)>,
    mut others: Query<
        (
            &mut Transform,
            Option<&SourceSurfaceView>,
            Has<SourceSkyCamera>,
        ),
        Without<super::camera::DriveCamera>,
    >,
) {
    let Ok(main) = main.single() else { return };
    for (mut transform, source, sky) in &mut others {
        if sky {
            *transform = Transform::from_rotation(main.rotation);
        } else if source.is_some_and(|v| v.index < 3) {
            *transform = *main;
        }
    }
}
fn bind_sky_extras(
    mut commands: Commands,
    assets: Res<Assets<super::sky::SkyMaterial>>,
    meshes: Query<(Entity, &MeshMaterial3d<super::sky::SkyMaterial>), Without<SourceSkyExtra>>,
) {
    for (entity, material) in &meshes {
        let Some(material) = assets.get(&material.0) else {
            continue;
        };
        match material.settings.kind.x {
            3 | 4 => {
                commands.entity(entity).insert((
                    SourceSkyExtra(material.settings.kind.x),
                    RenderLayers::layer(16),
                ));
            }
            _ => {
                commands.entity(entity).despawn();
            }
        }
    }
}
fn update_sky_extras(
    changed: Query<
        (Entity, &MainEntity, &SourceSkyExtra),
        Or<(Changed<SourceSkyExtra>, Changed<MainEntity>)>,
    >,
    mut removed: RemovedComponents<SourceSkyExtra>,
    mut extras: ResMut<SkyExtras>,
) {
    for entity in removed.read() {
        extras.0.retain(|_, (render, _)| *render != entity);
    }
    for (render, main, extra) in &changed {
        extras.0.insert(main.id(), (render, extra.0));
    }
}
fn sort_sky(
    views: Query<
        (&ExtractedView, Option<&super::mirrors::MirrorViewRole>),
        Or<(With<SourceSkyCamera>, With<super::mirrors::MirrorViewRole>)>,
    >,
    mut phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
) {
    for (view, role) in &views {
        if matches!(role, Some(super::mirrors::MirrorViewRole::Normal)) {
            continue;
        }
        if let Some(phase) = phases.get_mut(&view.retained_view_entity) {
            Transparent3d::sort(&mut phase.items);
        }
    }
}
fn prepare_frame(
    mut commands: Commands,
    resources: Option<Res<SurfaceResources>>,
    assets: Option<Res<LightingRenderAssets>>,
    existing: Option<ResMut<FrameResources>>,
    mut sky: Option<ResMut<SkyPass>>,
    mut clouds: Option<ResMut<super::clouds::CloudPass>>,
    scene: Res<super::lighting_scene::SourceSceneLighting>,
    views: Query<(&SourceSurfaceView, &ViewTarget)>,
    sky_views: Query<&ExtractedView, With<SourceSkyCamera>>,
    frame: Res<SourceViewFrame>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    cache: Res<PipelineCache>,
    status: Res<SourceLightingStatus>,
) {
    let (Some(resources), Some(assets), Some(scene)) = (resources, assets, scene.0) else {
        return;
    };
    let (Some(sky_clip), Some(skybox_clip)) =
        (frame.sky_clip_from_world, frame.skybox_clip_from_world)
    else {
        return;
    };
    if let Some(sky) = sky.as_mut() {
        if sky_views.single().is_ok() {
            sky.write_live(
                0,
                &scene,
                Vec3::ZERO,
                assets.record.cameras[0].far,
                skybox_clip,
                &device,
                &queue,
            );
        }
    }
    if let (Some(sky), Some(ProbeJob::Face(face))) = (sky.as_mut(), frame.probe_job) {
        if let Some(snapshot) = frame.views[3 + usize::from(face)] {
            let projection =
                Mat4::from_cols_array_2d(&crate::reference::sky::source_sky_gpu_projection(
                    snapshot.implicit_projection.to_cols_array_2d(),
                    snapshot.near,
                ));
            sky.write_live(
                1 + usize::from(face),
                &scene,
                scene.probe_position[frame.probe_index],
                snapshot.far,
                projection * snapshot.view_from_world,
                &device,
                &queue,
            );
        }
    }
    if let Some(clouds) = clouds.as_mut() {
        if sky_views.single().is_ok() {
            clouds.write_live(0, Vec3::ZERO, sky_clip, &scene, &device, &queue);
        }
    }
    for owner in 0..3 {
        let Some(snapshot) = frame.views[MIRROR_START + owner] else {
            continue;
        };
        let position = snapshot.view_from_world.inverse().w_axis.truncate();
        let mut sky_projection = crate::reference::sky::source_sky_gpu_projection(
            snapshot.implicit_projection.to_cols_array_2d(),
            snapshot.near,
        );
        // MirrorProjection preserves its native upright render texture projection.
        for column in &mut sky_projection {
            column[1] = -column[1];
        }
        if let Some(sky) = sky.as_mut() {
            sky.write_live(
                7 + owner,
                &scene,
                position,
                snapshot.far,
                Mat4::from_cols_array_2d(&sky_projection) * snapshot.view_from_world,
                &device,
                &queue,
            );
        }
        if let Some(clouds) = clouds.as_mut() {
            clouds.write_live(
                owner + 1,
                position,
                snapshot.raster_projection * snapshot.view_from_world,
                &scene,
                &device,
                &queue,
            );
        }
    }
    let Some((_, target)) = views.iter().find(|(source, _)| source.index == 2) else {
        return;
    };
    let Some(format) = target.out_texture_view_format() else {
        return;
    };
    if !format.is_srgb() {
        *status.0.lock().unwrap() =
            RenderingState::Failed("Source final display requires an sRGB view".into());
        return;
    }
    let colors = resources.targets.stack.clone();
    if let Some(mut existing) = existing {
        if !Arc::ptr_eq(&existing.colors, &colors) {
            existing
                .display
                .rebind(&device, &cache, colors.resolved.sampled.as_ref().unwrap());
            let texture = sky_depth(&device, colors.width, colors.height);
            existing.sky_depth = texture.create_view(&TextureViewDescriptor::default());
            existing._sky_depth = texture;
            existing.colors = colors;
        }
        if existing.display_format != format {
            *status.0.lock().unwrap() =
                RenderingState::Failed("Source display format changed".into());
        }
        if let Some(error) = existing.display.failure(&cache) {
            *status.0.lock().unwrap() = RenderingState::Failed(error);
        }
        return;
    }
    let texture = sky_depth(&device, colors.width, colors.height);
    let depth = texture.create_view(&TextureViewDescriptor::default());
    let display = FrameCopy::new(
        &device,
        &cache,
        assets.shaders.copy.clone(),
        colors.resolved.sampled.as_ref().unwrap(),
        format,
        1,
    );
    commands.insert_resource(FrameResources {
        colors,
        _sky_depth: texture,
        sky_depth: depth,
        display,
        display_format: format,
    });
}
fn sky_depth(device: &RenderDevice, width: u32, height: u32) -> Texture {
    device.create_texture(&TextureDescriptor {
        label: Some("source_sky_depth"),
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 8,
        dimension: TextureDimension::D2,
        format: super::render_targets::DEPTH_FORMAT,
        usage: TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    })
}
pub(crate) fn surface_ready(
    view: ViewQuery<&SourceSurfaceView>,
    frame: Res<SourceViewFrame>,
    progress: Res<SourceGraphProgress>,
    camera: Res<SourceViewCamera>,
    resources: Option<Res<FrameResources>>,
    probe: Option<Res<ProbeFrame>>,
    cache: Res<PipelineCache>,
    effects: Option<Res<super::source_effects::SourceEffects>>,
) -> bool {
    let index = view.into_inner().index;
    let state = progress.0.lock().unwrap();
    index < VIEW_COUNT
        && frame.views[index].is_some()
        && camera.ready()
        && resources.is_some_and(|r| r.display.ready(&cache))
        && effects.is_some_and(|effects| effects.ready(&cache))
        && state.sky
        && (index < 3
            || index >= MIRROR_START
            || (state.presented
                && frame.probe_job == Some(ProbeJob::Face((index - 3) as u8))
                && probe.is_some_and(|p| {
                    p.probes[frame.probe_index].accepts(frame.probe_token)
                        && p.probes[frame.probe_index].filter.ready(&cache)
                })))
}
fn begin_frame(frame: Res<SourceViewFrame>, progress: Res<SourceGraphProgress>) {
    let mut state = progress.0.lock().unwrap();
    if state.scene != frame.scene {
        let cpu = state.cpu_boundary;
        let gpu = state.gpu_boundary;
        *state = FrameProgress {
            scene: frame.scene,
            cpu_boundary: cpu,
            gpu_boundary: gpu,
            ..default()
        };
    }
    if let Some(view) = frame.views[1] {
        let extent = view.extent.to_array();
        if state.extent != extent {
            state.extent = extent;
            state.validation = 0;
            state.validated = false;
        }
    }
    state.jitter = frame.jitter.map_or([0.; 2], |j| j.pixels);
    state.frame = frame.frame;
    state.mask = 0;
    state.sky = false;
    state.presented = false;
}
fn draw_extras(
    kind: u32,
    phase: &bevy::render::render_phase::SortedRenderPhase<Transparent3d>,
    world: &World,
    view: Entity,
    context: &mut RenderContext,
    targets: &super::render_targets::SourceColorTargets,
    depth: &TextureView,
    extras: &SkyExtras,
) -> Result<(), String> {
    let colors = [Some(RenderPassColorAttachment {
        view: targets.attachment(),
        depth_slice: None,
        resolve_target: targets.resolve_target().map(|v| &**v),
        ops: Operations {
            load: LoadOp::Load,
            store: StoreOp::Store,
        },
    })];
    let mut pass = context.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("source_sky_extras"),
        color_attachments: &colors,
        depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
            view: depth,
            depth_ops: Some(Operations {
                load: LoadOp::Load,
                store: StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        ..default()
    });
    let mut functions = world.resource::<DrawFunctions<Transparent3d>>().write();
    functions.prepare(world);
    for item in phase.items.values() {
        if extras
            .0
            .get(&item.main_entity().id())
            .is_some_and(|(_, k)| *k == kind)
        {
            functions
                .get_mut(item.draw_function())
                .ok_or("Source sky draw function missing")?
                .draw(world, &mut pass, view, item)
                .map_err(|e| format!("Source sky extra: {e:?}"))?;
        }
    }
    Ok(())
}
fn render_sky(
    world: &World,
    view: ViewQuery<&ExtractedView>,
    resources: Option<Res<FrameResources>>,
    sky: Option<Res<SkyPass>>,
    clouds: Option<Res<super::clouds::CloudPass>>,
    extras: Res<SkyExtras>,
    phases: Res<ViewSortedRenderPhases<Transparent3d>>,
    allocator: Res<bevy::render::mesh::allocator::MeshAllocator>,
    cache: Res<PipelineCache>,
    frame: Res<SourceViewFrame>,
    loading: Res<super::materials::MaterialLoading>,
    status: Res<SourceLightingStatus>,
    progress: Res<SourceGraphProgress>,
    mut context: RenderContext,
) {
    let entity = view.entity();
    let view = view.into_inner();
    if frame.sky_clip_from_world.is_none()
        || frame.views[..MIRROR_START].iter().any(Option::is_none)
    {
        return;
    }
    let (Some(resources), Some(sky), Some(clouds)) = (resources, sky, clouds) else {
        return;
    };
    if !sky.ready(&cache)
        || !clouds.ready(&cache)
        || !resources.display.ready(&cache)
        || !loading.is_ready()
    {
        return;
    }
    let Some(phase) = phases.get(&view.retained_view_entity) else {
        return;
    };
    let colors = [Some(RenderPassColorAttachment {
        view: resources.colors.attachment(),
        depth_slice: None,
        resolve_target: resources.colors.resolve_target().map(|v| &**v),
        ops: Operations {
            load: LoadOp::Clear(LinearRgba::BLACK.into()),
            store: StoreOp::Store,
        },
    })];
    drop(
        context
            .command_encoder()
            .begin_render_pass(&RenderPassDescriptor {
                label: Some("source_stack_clear"),
                color_attachments: &colors,
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &resources.sky_depth,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(0.),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: Some(Operations {
                        load: LoadOp::Clear(0),
                        store: StoreOp::Store,
                    }),
                }),
                ..default()
            }),
    );
    if !sky.encode(
        0,
        context.command_encoder(),
        &cache,
        resources.colors.attachment(),
        resources.colors.resolve_target(),
        &resources.sky_depth,
    ) {
        return;
    }
    if let Err(error) = draw_extras(
        3,
        phase,
        world,
        entity,
        &mut context,
        &resources.colors,
        &resources.sky_depth,
        &extras,
    ) {
        *status.0.lock().unwrap() = RenderingState::Failed(error);
        return;
    }
    if !clouds.encode(
        0,
        context.command_encoder(),
        &cache,
        &allocator,
        resources.colors.attachment(),
        resources.colors.resolve_target(),
        &resources.sky_depth,
    ) {
        return;
    }
    if let Err(error) = draw_extras(
        4,
        phase,
        world,
        entity,
        &mut context,
        &resources.colors,
        &resources.sky_depth,
        &extras,
    ) {
        *status.0.lock().unwrap() = RenderingState::Failed(error);
        return;
    }
    progress.0.lock().unwrap().sky = true;
}
fn finish_surface(
    view: ViewQuery<(
        &SourceSurfaceView,
        &ViewTarget,
        &super::shadow_collector::CollectorView,
        &super::source_lighting::SourceViewLighting,
        &super::shadow_phase::ShadowDrawState,
        &super::camera_depth::CameraDepthState,
        &super::surface_graph::SurfaceViewTargets,
        &super::stained_glass::SourceGlassView,
        Option<&super::mirrors::MirrorInstance>,
    )>,
    frame: Res<SourceViewFrame>,
    progress: Res<SourceGraphProgress>,
    resources: Res<FrameResources>,
    surfaces: Res<SurfaceResources>,
    mut probe: Option<ResMut<ProbeFrame>>,
    cache: Res<PipelineCache>,
    effects: Res<super::source_effects::SourceEffects>,
    mirror_outputs: Res<super::mirrors::MirrorOutputs>,
    mirror_status: Res<super::mirrors::MirrorGpuStatus>,
    status: Res<SourceLightingStatus>,
    mut context: RenderContext,
) {
    let (source, target, collector, lighting, shadow, depth, targets, glass, mirror_instance) =
        view.into_inner();
    if collector.encoded_frame() != frame.frame || glass.audit().frame != frame.frame {
        return;
    }
    if matches!(*status.0.lock().unwrap(), RenderingState::Failed(_)) {
        return;
    }
    let mut state = progress.0.lock().unwrap();
    let (shadow_epoch, shadow_attempts, shadow_issued) = shadow.counts();
    state.views[source.index] = ViewAudit {
        frame: frame.frame,
        glass: glass.audit(),
        camera_depth: depth.counts(),
        surface: lighting.draws(),
        shadow_epoch,
        shadow_attempts,
        shadow_issued,
        collector_frame: collector.encoded_frame(),
        atlas_size: targets.0.atlas_size,
        physical_extent: [targets.0.colors.width, targets.0.colors.height],
    };
    if source.index >= MIRROR_START {
        let owner = source.index - MIRROR_START;
        let Some(instance) = mirror_instance else {
            return;
        };
        if !mirror_status.sky_ready(instance.0, owner, frame.frame) {
            return;
        }
        let Some(output) = target.out_texture_color_attachment(None) else {
            return;
        };
        if mirror_outputs.0[owner]
            .as_ref()
            .is_some_and(|copy| copy.encode(context.command_encoder(), &cache, output.view, None))
        {
            mirror_status.encoded(instance.0, owner, frame.frame);
        }
        return;
    }
    if source.index >= 3 {
        let Some(probes) = probe.as_mut() else { return };
        let probe = &mut probes.probes[frame.probe_index];
        let face = (source.index - 3) as u8;
        if probe.sky_frames[usize::from(face)].load(Ordering::Acquire) != frame.frame {
            return;
        }
        if !probe.filter.capture_face(
            context.command_encoder(),
            &cache,
            &surfaces.probes[frame.probe_index],
            usize::from(face),
        ) {
            return;
        }
        let job = ProbeJob::Face(face);
        if let Err(error) = probe.publication.encoded(frame.probe_token, job) {
            *status.0.lock().unwrap() = RenderingState::Failed(error.into());
            return;
        }
        probe.encoded_token = Some(frame.probe_token);
        return;
    }
    if !effects.encoded(source.index, frame.frame, frame.main_token) {
        return;
    }
    state.mask |= 1 << source.index;
    if source.index == 2 && state.mask == 7 && state.sky {
        let Some(output) = target.out_texture_color_attachment(None) else {
            return;
        };
        if resources
            .display
            .encode(context.command_encoder(), &cache, output.view, None)
        {
            state.presented = true;
            state.token = frame.main_token;
        }
    }
}
fn submitted(
    progress: Res<SourceGraphProgress>,
    frame: Res<SourceViewFrame>,
    mut probe: Option<ResMut<ProbeFrame>>,
    cache: Res<PipelineCache>,
    acks: Res<SourceSubmissionAcks>,
    gate: Res<SourceExecutionGate>,
    queue: Res<RenderQueue>,
    status: Res<SourceLightingStatus>,
) {
    if matches!(*status.0.lock().unwrap(), RenderingState::Failed(_)) {
        return;
    }
    let mut state = progress.0.lock().unwrap();
    if state.presented {
        if state.first_token == 0 {
            state.first_token = state.token;
            state.first_jitter = state.jitter;
        }
        acks.main_submitted(state.token);
        if state.validation == 0 {
            state.validation = gate.watch_submission(&queue);
        }
        state.validated = gate.submission_validated(state.validation);
    }
    let Some(probes) = probe.as_mut() else { return };
    if probes.scene != frame.scene {
        return;
    }
    for (index, probe) in probes.probes.iter_mut().enumerate() {
        let result = (|| -> Result<(), &'static str> {
            if let Some(token) = probe.encoded_token.take() {
                let advance = probe.publication.submitted(token)?;
                probe.submitted_token = token;
                if advance {
                    acks.probe_submitted(token);
                } else {
                    probe.filter_validation = gate.watch_submission(&queue);
                }
            }
            if let Some(token) = probe.publication_encoded.take() {
                probe.publication_token = token;
                probe.publication_validation = gate.watch_submission(&queue);
            }
            if gate.submission_validated(probe.publication_validation) {
                let token = probe.publication_token;
                probe.publication.publication_submitted(token)?;
                acks.probe_publication_submitted(token);
                probe.published_token = token;
                probe.publication_validation = 0;
                probe.publication_token = 0;
            }
            Ok(())
        })();
        if let Err(error) = result {
            *status.0.lock().unwrap() = RenderingState::Failed(error.into());
            return;
        }
        state.probes[index] = ProbeAudit {
            enabled: frame.probe_enabled[index],
            checks: frame.probe_checks,
            distance_squared: frame.probe_distance_squared[index],
            token: frame.probe_token,
            job: match if index == frame.probe_index {
                frame.probe_job
            } else {
                None
            } {
                Some(ProbeJob::Face(face)) => face + 1,
                Some(ProbeJob::FirstMip) => 7,
                Some(ProbeJob::RemainingMipsAndPublish) => 8,
                None => 0,
            },
            face_mask: probe.publication.face_mask(),
            mip_mask: probe.publication.mip_mask(),
            pipelines_ready: probe.filter.ready(&cache),
            filter_validation: probe.filter_validation,
            publication_validation: probe.publication_validation,
            published: probe.publication.published(),
            generation: probe.publication.generation(),
            submitted_token: probe.submitted_token,
            published_token: probe.published_token,
        };
    }
}

#[derive(Clone, Copy, Default, serde::Serialize)]
struct ViewAudit {
    frame: u32,
    glass: super::stained_glass::GlassAudit,
    camera_depth: [u32; 3],
    surface: [u32; 5],
    shadow_epoch: u32,
    shadow_attempts: [u32; 4],
    shadow_issued: [u32; 4],
    collector_frame: u32,
    atlas_size: u32,
    physical_extent: [u32; 2],
}
struct SourceDiagnostics {
    progress: Arc<Mutex<FrameProgress>>,
    loading: Mutex<LoadingProgress>,
    status: Arc<Mutex<RenderingState>>,
    gate: SourceExecutionGate,
    mirrors: super::mirrors::MirrorGpuStatus,
}
static SOURCE_DIAGNOSTICS: OnceLock<SourceDiagnostics> = OnceLock::new();

#[derive(Default)]
struct LoadingProgress {
    scene: u32,
    mirror_instance: u64,
    revision: u32,
    counts: [u64; 7],
    milestones: [u64; 5],
}

/// Poll bounded readiness work without serializing the full diagnostic audit.
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn source_loading_progress() -> u32 {
    let Some(audit) = SOURCE_DIAGNOSTICS.get() else {
        return 0;
    };
    let state = audit.progress.lock().unwrap();
    let cpu = &state.cpu_boundary;
    let gpu = &state.gpu_boundary;
    let prewarm = &gpu.prewarm;
    let (mirror_instance, mirror_bits) = audit.mirrors.loading_progress();
    let counts = [
        cpu.source_texture_bytes,
        cpu.lighting_texture_bytes,
        u64::from(prewarm.ready_pipeline_count),
        u64::from(if prewarm.sealed {
            prewarm
                .request_count
                .saturating_sub(prewarm.missing_mesh_count)
        } else {
            0
        }),
        u64::from(if prewarm.sealed {
            prewarm
                .request_count
                .saturating_sub(prewarm.missing_material_count)
        } else {
            0
        }),
        u64::from(gpu.source_lighting_bindings.min(VIEW_COUNT as u32)),
        u64::from(gpu.source_glass_bindings.min(VIEW_COUNT as u32)),
    ];
    let bits = |values: &[bool]| {
        values
            .iter()
            .enumerate()
            .fold(0u64, |mask, (i, value)| mask | (u64::from(*value) << i))
    };
    let probe_bits = |probe: &ProbeAudit| {
        // Publication proves every prerequisite. Polling can miss intermediate masks;
        // later capture cycles must not report those missed bits as new loading work.
        if probe.published {
            return 0x3f | (0x7f << 8) | (0xf << 16);
        }
        u64::from(probe.face_mask)
            | (u64::from(probe.mip_mask) << 8)
            | (u64::from(probe.pipelines_ready) << 16)
            | (u64::from(probe.filter_validation != 0) << 17)
            | (u64::from(probe.publication_validation != 0) << 18)
    };
    let milestones = [
        bits(&[
            cpu.lighting_assets_present,
            cpu.lighting_assets_loaded,
            cpu.source_materials_present,
            cpu.source_materials_bound,
            cpu.source_material_state == 1,
            cpu.material_loading_ready,
            cpu.requests_sealed,
            cpu.master_bound,
            cpu.master_materials_prepared,
            cpu.master_cookie_handles,
            cpu.visuals_bound,
        ]),
        bits(&[
            gpu.master_cookies,
            gpu.master_uniform,
            gpu.master_state == 1,
            gpu.lighting_render_assets,
            gpu.cloud_gpu_mesh,
            gpu.source_targets,
            gpu.frame_resources,
            gpu.frame_copy_ready,
            gpu.source_camera_ready,
            gpu.source_surface_ready,
            gpu.source_caster_ready,
            gpu.source_sky_ready,
            gpu.source_cloud_ready,
            prewarm.sealed,
            prewarm.warmed_queued,
            prewarm.state == 1,
            state.sky,
            state.presented,
            state.mask & 1 != 0,
            state.mask & 2 != 0,
            state.mask & 4 != 0,
            state.validated,
            state.motion.validated,
            state.effects.pipelines_ready(),
            state.effects.complete(state.frame, state.token),
        ]),
        probe_bits(&state.probes[0]),
        probe_bits(&state.probes[1]),
        mirror_bits,
    ];
    let mut progress = audit.loading.lock().unwrap();
    let mut changed = false;
    if progress.scene != state.scene || progress.mirror_instance != mirror_instance {
        progress.scene = state.scene;
        progress.mirror_instance = mirror_instance;
        progress.counts.fill(0);
        progress.milestones.fill(0);
        changed = true;
    }
    for (previous, value) in progress.counts.iter_mut().zip(counts) {
        if value > *previous {
            *previous = value;
            changed = true;
        }
    }
    for (previous, value) in progress.milestones.iter_mut().zip(milestones) {
        if value & !*previous != 0 {
            *previous |= value;
            changed = true;
        }
    }
    if changed {
        progress.revision = progress.revision.saturating_add(1);
    }
    progress.revision
}

/// Browser graphics failures use the same terminal state as native device errors.
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn source_render_failure(message: &str) {
    let Some(renderer) = SOURCE_DIAGNOSTICS.get() else {
        return;
    };
    let mut state = renderer.status.lock().unwrap();
    if !matches!(*state, RenderingState::Failed(_)) {
        *state = RenderingState::Failed(message.to_owned());
    }
}

/// Serialize only on explicit diagnostic calls. Render frames copy fixed-size counters.
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn source_render_audit() -> String {
    let Some(audit) = SOURCE_DIAGNOSTICS.get() else {
        return "null".into();
    };
    let progress = audit.progress.lock().unwrap();
    let state = audit.status.lock().unwrap();
    let state = match &*state {
        RenderingState::Loading => "Loading".to_owned(),
        RenderingState::Ready => "Ready".to_owned(),
        RenderingState::Failed(error) => format!("Failed: {error}"),
    };
    serde_json::to_string(&serde_json::json!({
        "state": state, "graph": &*progress,
        "gpu_validated_generation": audit.gate.validated_generation(),
        "mirrors": audit.mirrors.audit(),
    }))
    .expect("source diagnostic values serialize")
}

#[derive(Clone, Copy, Default, serde::Serialize)]
struct CpuBoundary {
    lighting_assets_present: bool,
    lighting_assets_loaded: bool,
    lighting_texture_bytes: u64,
    source_materials_present: bool,
    source_materials_bound: bool,
    source_material_state: u8,
    source_texture_bytes: u64,
    material_loading_ready: bool,
    requests_sealed: bool,
    shadow_capacity: u32,
    transparent_capacity: u32,
    master_bound: bool,
    master_materials_prepared: bool,
    master_cookie_handles: bool,
    visuals_bound: bool,
}
#[derive(Clone, Copy, Default, serde::Serialize)]
struct GpuBoundary {
    asset_audit_refreshes: u32,
    view_keys: u32,
    gpu_meshes: u32,
    prepared_materials: u32,
    source_prepared_materials: u32,
    master_cookies: bool,
    master_uniform: bool,
    master_state: u8,
    lighting_render_assets: bool,
    lighting_textures: u32,
    lighting_gpu_textures: u32,
    cloud_gpu_mesh: bool,
    source_targets: bool,
    frame_resources: bool,
    frame_copy_ready: bool,
    source_camera_ready: bool,
    source_surface_ready: bool,
    source_surface_pipelines: u32,
    source_add_pipelines: u32,
    source_caster_ready: bool,
    source_sky_ready: bool,
    source_cloud_ready: bool,
    source_views: u32,
    source_lighting_bindings: u32,
    source_glass_bindings: u32,
    opaque_phases: u32,
    masked_phases: u32,
    transparent_phases: u32,
    #[cfg(feature = "save-fixture")]
    transparent_storage: [[u32; 5]; VIEW_COUNT],
    #[cfg(feature = "save-fixture")]
    sort_index: [u32; 2],
    shadow_phases: u32,
    shadow_scene_frame: u32,
    shadow_queue_frame: u32,
    shadow_ready_mask: u16,
    snapshot_frame: u32,
    snapshot_views: u16,
    sky_snapshot: bool,
    prewarm: super::materials::MaterialPrewarmAudit,
}
fn state_code(state: &RenderingState) -> u8 {
    match state {
        RenderingState::Loading => 0,
        RenderingState::Ready => 1,
        RenderingState::Failed(_) => 2,
    }
}
fn cpu_boundaries(
    progress: Res<SourceGraphProgress>,
    assets: Option<Res<super::lighting_assets::LightingAssets>>,
    materials: Option<Res<super::materials::SourceMaterials>>,
    loading: Option<Res<super::materials::MaterialLoading>>,
    master: Option<Res<super::lighting::SourceLighting>>,
    visuals: Option<Res<super::reference::ReferenceVisuals>>,
) {
    let mut boundary = CpuBoundary::default();
    if let Some(assets) = assets {
        boundary.lighting_assets_present = true;
        boundary.lighting_assets_loaded = assets.loaded;
        boundary.lighting_texture_bytes = assets.texture_bytes as u64;
    }
    if let Some(materials) = materials {
        boundary.source_materials_present = true;
        boundary.source_materials_bound = materials.bound;
        boundary.source_material_state = state_code(&materials.state);
        boundary.source_texture_bytes = materials.texture_bytes as u64;
    }
    if let Some(loading) = loading {
        boundary.material_loading_ready = loading.is_ready();
        boundary.requests_sealed = loading.source_shadow_capacity().is_some();
        boundary.shadow_capacity = loading.source_shadow_capacity().unwrap_or(0) as u32;
        boundary.transparent_capacity = loading.transparent_pass_capacity().unwrap_or(0) as u32;
    }
    if let Some(master) = master {
        boundary.master_bound = master.bound;
        boundary.master_materials_prepared = master.materials_prepared;
        boundary.master_cookie_handles = master.cookie_handles.is_some();
    }
    boundary.visuals_bound = visuals.is_some_and(|v| v.bound);
    progress.0.lock().unwrap().cpu_boundary = boundary;
}
#[derive(Default)]
struct AssetAuditCache {
    stamps: Option<[Option<u32>; 4]>,
    counts: [u32; 5],
    cloud: bool,
    refreshes: u32,
}
fn resource_stamp<T: Resource>(world: &World) -> Option<u32> {
    world
        .get_resource_ref::<T>()
        .map(|resource| resource.last_changed().get())
}
fn gpu_boundaries(
    world: &World,
    mut audit: Local<AssetAuditCache>,
    views: Query<(
        &SourceSurfaceView,
        Option<&super::source_lighting::SourceViewLighting>,
        Option<&super::stained_glass::SourceGlassView>,
    )>,
) {
    use bevy::core_pipeline::core_3d::{AlphaMask3d, Opaque3d};
    use bevy::pbr::{PreparedMaterial, ViewKeyCache};
    use bevy::render::mesh::RenderMesh;
    use bevy::render::render_phase::ViewBinnedRenderPhases;
    use bevy::render::texture::GpuImage;
    use bevy::render::{erased_render_asset::ErasedRenderAssets, render_asset::RenderAssets};
    let mut boundary = GpuBoundary::default();
    if let Some(keys) = world.get_resource::<ViewKeyCache>() {
        boundary.view_keys = keys.len() as u32;
    }
    let stamps = [
        resource_stamp::<RenderAssets<RenderMesh>>(world),
        resource_stamp::<ErasedRenderAssets<PreparedMaterial>>(world),
        resource_stamp::<LightingRenderAssets>(world),
        resource_stamp::<RenderAssets<GpuImage>>(world),
    ];
    if audit.stamps != Some(stamps) {
        audit.stamps = Some(stamps);
        audit.counts = [0; 5];
        audit.cloud = false;
        audit.refreshes += 1;
        if let Some(meshes) = world.get_resource::<RenderAssets<RenderMesh>>() {
            audit.counts[0] = meshes.iter().count() as u32;
        }
        if let Some(materials) = world.get_resource::<ErasedRenderAssets<PreparedMaterial>>() {
            audit.counts[1] = materials.iter().count() as u32;
            audit.counts[2] = materials
                .iter()
                .filter(|(id, _)| {
                    id.type_id() == std::any::TypeId::of::<super::materials::SourceMaterial>()
                })
                .count() as u32;
        }
        if let Some(assets) = world.get_resource::<LightingRenderAssets>() {
            audit.counts[3] = assets.textures.len() as u32;
            if let Some(images) = world.get_resource::<RenderAssets<GpuImage>>() {
                audit.counts[4] = assets
                    .textures
                    .values()
                    .filter(|image| images.get(image.id()).is_some())
                    .count() as u32;
            }
            audit.cloud = world
                .get_resource::<RenderAssets<RenderMesh>>()
                .is_some_and(|meshes| meshes.get(assets.cloud_mesh.id()).is_some());
        }
    }
    boundary.gpu_meshes = audit.counts[0];
    boundary.prepared_materials = audit.counts[1];
    boundary.source_prepared_materials = audit.counts[2];
    boundary.lighting_textures = audit.counts[3];
    boundary.lighting_gpu_textures = audit.counts[4];
    boundary.cloud_gpu_mesh = audit.cloud;
    boundary.asset_audit_refreshes = audit.refreshes;
    boundary.lighting_render_assets = stamps[2].is_some();
    if let Some(master) = world.get_resource::<super::lighting::LightingGpu>() {
        boundary.master_cookies = master.cookies.is_some();
        boundary.master_uniform = master.uniform.buffer().is_some();
        boundary.master_state = match &*master.status.lock().unwrap() {
            super::lighting::RenderingState::Loading => 0,
            super::lighting::RenderingState::Ready => 1,
            super::lighting::RenderingState::Failed(_) => 2,
        };
    }
    let cache = world.resource::<PipelineCache>();
    boundary.source_targets = world.contains_resource::<SurfaceResources>();
    if let Some(resources) = world.get_resource::<FrameResources>() {
        boundary.frame_resources = true;
        boundary.frame_copy_ready = resources.display.ready(cache);
    }
    boundary.source_camera_ready = world
        .get_resource::<SourceViewCamera>()
        .is_some_and(|c| c.ready());
    if let Some(surface) = world.get_resource::<super::surface_pass::SurfacePasses>() {
        boundary.source_surface_ready = surface.ready(cache);
        let (base, add) = surface.counts();
        boundary.source_surface_pipelines = base as u32;
        boundary.source_add_pipelines = add as u32;
    }
    boundary.source_caster_ready = world
        .get_resource::<super::caster::CasterPasses>()
        .is_some_and(|p| p.ready(cache));
    boundary.source_sky_ready = world
        .get_resource::<SkyPass>()
        .is_some_and(|p| p.ready(cache));
    boundary.source_cloud_ready = world
        .get_resource::<super::clouds::CloudPass>()
        .is_some_and(|p| p.ready(cache));
    #[cfg(feature = "save-fixture")]
    if let Some(surfaces) = world.get_resource::<SurfaceResources>() {
        boundary.transparent_storage = std::array::from_fn(|i| surfaces.transparent[i].audit());
    }
    for (_, lighting, glass) in &views {
        boundary.source_views += 1;
        boundary.source_lighting_bindings += u32::from(lighting.is_some());
        boundary.source_glass_bindings += u32::from(glass.is_some());
    }
    boundary.opaque_phases = world
        .get_resource::<ViewBinnedRenderPhases<Opaque3d>>()
        .map_or(0, |p| p.len() as u32);
    boundary.masked_phases = world
        .get_resource::<ViewBinnedRenderPhases<AlphaMask3d>>()
        .map_or(0, |p| p.len() as u32);
    #[cfg(feature = "save-fixture")]
    if let Some(index) = world.get_resource::<super::transparent::SourceSortIndex>() {
        boundary.sort_index = index.audit(world);
    }
    boundary.transparent_phases = world
        .get_resource::<ViewSortedRenderPhases<Transparent3d>>()
        .map_or(0, |p| p.len() as u32);
    boundary.shadow_phases = world
        .get_resource::<ViewSortedRenderPhases<super::shadow_phase::SourceShadow>>()
        .map_or(0, |p| p.len() as u32);
    boundary.shadow_scene_frame = world
        .get_resource::<super::shadow_scene::ShadowScene>()
        .map_or(0, |s| s.frame);
    if let Some(queues) = world.get_resource::<super::shadow_queue::ShadowQueues>() {
        boundary.shadow_queue_frame = queues.frame;
        for (index, ready) in queues.ready.iter().enumerate() {
            if *ready {
                boundary.shadow_ready_mask |= 1 << index;
            }
        }
    }
    if let Some(frame) = world.get_resource::<SourceViewFrame>() {
        boundary.snapshot_frame = frame.frame;
        boundary.sky_snapshot = frame.sky_clip_from_world.is_some();
        for (index, view) in frame.views.iter().enumerate() {
            if view.is_some() {
                boundary.snapshot_views |= 1 << index;
            }
        }
    }
    if let Some(loading) = world.get_resource::<super::materials::MaterialLoading>() {
        boundary.prewarm = loading.prewarm_audit(world);
    }
    world
        .resource::<SourceGraphProgress>()
        .0
        .lock()
        .unwrap()
        .gpu_boundary = boundary;
}

fn prepare_probe(
    mut commands: Commands,
    surfaces: Option<Res<SurfaceResources>>,
    assets: Option<Res<LightingRenderAssets>>,
    existing: Option<ResMut<ProbeFrame>>,
    frame: Res<SourceViewFrame>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    cache: Res<PipelineCache>,
    status: Res<SourceLightingStatus>,
) {
    if matches!(*status.0.lock().unwrap(), RenderingState::Failed(_)) {
        return;
    }
    if let Some(mut existing) = existing {
        if existing.scene != frame.scene {
            let Some(surfaces) = surfaces else {
                return;
            };
            existing.scene = frame.scene;
            for (index, probe) in existing.probes.iter_mut().enumerate() {
                surfaces.probes[index].clear_publication(&queue);
                probe.publication = ProbePublication::default();
                probe.encoded_token = None;
                probe.publication_encoded = None;
                probe.filter_validation = 0;
                probe.publication_validation = 0;
                probe.publication_token = 0;
                probe.submitted_token = 0;
                probe.published_token = 0;
                for stamp in &probe.sky_frames {
                    stamp.store(0, Ordering::Release);
                }
            }
        }
        for probe in &existing.probes {
            if let Some(error) = probe.filter.failure(&cache) {
                *status.0.lock().unwrap() = RenderingState::Failed(error);
            }
        }
        return;
    }
    let (Some(surfaces), Some(assets)) = (surfaces, assets) else {
        return;
    };
    let mut pipelines = None;
    let probes = std::array::from_fn(|index| {
        let filter = ProbeFilter::new(
            &device,
            &queue,
            &cache,
            assets.shaders.probe.clone(),
            &surfaces.probes[index],
            surfaces.targets.capture.resolved.sampled.as_ref().unwrap(),
            pipelines,
        );
        pipelines = Some(filter.pipelines());
        ProbeState {
            filter,
            publication: ProbePublication::default(),
            encoded_token: None,
            publication_encoded: None,
            filter_validation: 0,
            publication_validation: 0,
            publication_token: 0,
            submitted_token: 0,
            published_token: 0,
            sky_frames: std::array::from_fn(|_| AtomicU32::new(0)),
        }
    });
    commands.insert_resource(ProbeFrame {
        scene: frame.scene,
        probes,
    });
}

fn clear_capture(
    view: ViewQuery<(
        &SourceSurfaceView,
        &super::surface_graph::SurfaceViewTargets,
    )>,
    mut context: RenderContext,
) {
    let (source, targets) = view.into_inner();
    if source.index < 3 {
        return;
    }
    let colors = [Some(RenderPassColorAttachment {
        view: targets.0.colors.attachment(),
        depth_slice: None,
        resolve_target: None,
        ops: Operations {
            load: LoadOp::Clear(LinearRgba::BLACK.into()),
            store: StoreOp::Store,
        },
    })];
    drop(
        context
            .command_encoder()
            .begin_render_pass(&RenderPassDescriptor {
                label: Some("source_probe_face_clear"),
                color_attachments: &colors,
                ..default()
            }),
    );
}
fn render_capture_sky(
    world: &World,
    view: ViewQuery<(
        &ExtractedView,
        &SourceSurfaceView,
        &super::surface_graph::SurfaceViewTargets,
        Option<&super::mirrors::MirrorInstance>,
    )>,
    sky: Res<SkyPass>,
    probe: Option<Res<ProbeFrame>>,
    frame: Res<SourceViewFrame>,
    cache: Res<PipelineCache>,
    status: Res<SourceLightingStatus>,
    mirror_status: Res<super::mirrors::MirrorGpuStatus>,
    clouds: Res<super::clouds::CloudPass>,
    extras: Res<SkyExtras>,
    phases: Res<ViewSortedRenderPhases<Transparent3d>>,
    allocator: Res<bevy::render::mesh::allocator::MeshAllocator>,
    mut context: RenderContext,
) {
    let entity = view.entity();
    let (extracted, source, targets, instance) = view.into_inner();
    if source.index < 3 || matches!(*status.0.lock().unwrap(), RenderingState::Failed(_)) {
        return;
    }
    if sky.encode(
        source.index - 2,
        context.command_encoder(),
        &cache,
        targets.0.colors.attachment(),
        None,
        &targets.0.depth.attachment,
    ) {
        if source.index < MIRROR_START {
            if let Some(probe) = probe.as_ref() {
                probe.probes[frame.probe_index].sky_frames[source.index - 3]
                    .store(frame.frame, Ordering::Release);
            }
            return;
        }
        let owner = source.index - MIRROR_START;
        let Some(instance) = instance else { return };
        let Some(phase) = phases.get(&extracted.retained_view_entity) else {
            return;
        };
        if let Err(error) = draw_extras(
            3,
            phase,
            world,
            entity,
            &mut context,
            &targets.0.colors,
            &targets.0.depth.attachment,
            &extras,
        ) {
            *status.0.lock().unwrap() = RenderingState::Failed(error);
            return;
        }
        if !clouds.encode(
            owner + 1,
            context.command_encoder(),
            &cache,
            &allocator,
            targets.0.colors.attachment(),
            None,
            &targets.0.depth.attachment,
        ) {
            return;
        }
        if let Err(error) = draw_extras(
            4,
            phase,
            world,
            entity,
            &mut context,
            &targets.0.colors,
            &targets.0.depth.attachment,
            &extras,
        ) {
            *status.0.lock().unwrap() = RenderingState::Failed(error);
            return;
        }
        mirror_status.sky_encoded(instance.0, owner, frame.frame);
    }
}
fn filter_probe(
    mut probe: Option<ResMut<ProbeFrame>>,
    surfaces: Option<Res<SurfaceResources>>,
    frame: Res<SourceViewFrame>,
    progress: Res<SourceGraphProgress>,
    gate: Res<SourceExecutionGate>,
    cache: Res<PipelineCache>,
    status: Res<SourceLightingStatus>,
    mut context: RenderContext,
) {
    if matches!(*status.0.lock().unwrap(), RenderingState::Failed(_))
        || !progress.0.lock().unwrap().presented
    {
        return;
    }
    let (Some(probe), Some(surfaces)) = (probe.as_mut(), surfaces) else {
        return;
    };
    let probe = &mut probe.probes[frame.probe_index];
    if probe.publication_validation != 0 {
        return;
    }
    if probe.filter_validation != 0 {
        if !gate.submission_validated(probe.filter_validation) {
            return;
        }
        let Some(token) = probe.publication.validation_token() else {
            *status.0.lock().unwrap() =
                RenderingState::Failed("Source probe validation lacks its token".into());
            return;
        };
        let result = probe.publication.validated(token, true).and_then(|()| {
            probe.publication.encode_publication(
                context.command_encoder(),
                &surfaces.probes[frame.probe_index],
                &probe.filter,
                &cache,
            )
        });
        match result {
            Ok(token) => {
                probe.filter_validation = 0;
                probe.publication_encoded = Some(token);
            }
            Err(error) => *status.0.lock().unwrap() = RenderingState::Failed(error.into()),
        }
        return;
    }
    if !probe.accepts(frame.probe_token) || !probe.filter.ready(&cache) {
        return;
    }
    let Some(job) = frame.probe_job else { return };
    let encoded = match job {
        ProbeJob::FirstMip => probe.filter.first_mip(
            context.command_encoder(),
            &cache,
            &surfaces.probes[frame.probe_index],
        ),
        ProbeJob::RemainingMipsAndPublish => probe.filter.remaining_mips(
            context.command_encoder(),
            &cache,
            &surfaces.probes[frame.probe_index],
        ),
        ProbeJob::Face(_) => false,
    };
    if encoded {
        if let Err(error) = probe.publication.encoded(frame.probe_token, job) {
            *status.0.lock().unwrap() = RenderingState::Failed(error.into());
            return;
        }
        probe.encoded_token = Some(frame.probe_token);
    }
}

fn prepare_motion(
    mut commands: Commands,
    mut existing: Option<ResMut<MotionFrame>>,
    views: Query<(
        Entity,
        &SourceSurfaceView,
        &super::surface_graph::SurfaceViewTargets,
        Option<&MotionDrawView>,
    )>,
    frame: Res<SourceViewFrame>,
    objects: Res<MotionObjects>,
    assets: Option<Res<LightingRenderAssets>>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    cache: Res<PipelineCache>,
    status: Res<SourceLightingStatus>,
) {
    if matches!(*status.0.lock().unwrap(), RenderingState::Failed(_))
        || !objects.matches(frame.frame, frame.main_token)
    {
        return;
    }
    let (Some(snapshot), Some(assets), Some(origin)) =
        (frame.views[1].as_ref(), assets, objects.origin())
    else {
        return;
    };
    let Some((entity, _, targets, draw)) = views.iter().find(|(_, view, _, _)| view.index == 1)
    else {
        return;
    };
    let result = (|| -> Result<(), String> {
        if let Some(existing) = existing.as_mut() {
            if existing.scene != frame.scene || existing.motion_revision != frame.motion_revision {
                existing.motion_revision = frame.motion_revision;
                existing.scene = frame.scene;
                existing.first_token = 0;
                existing.first_draws = [0; 3];
                existing.first_camera_registered = false;
                existing.pass.as_mut().unwrap().recreate(snapshot);
                existing.origin = origin;
                existing.camera_frame = 0;
                existing.camera_token = 0;
                existing.submitted_token = 0;
                existing.validation = 0;
            }
            let delta = Vec3::from_array(std::array::from_fn(|i| {
                (origin[i] - existing.origin[i]) as f32
            }));
            if !delta.is_finite() {
                return Err("Source motion origin is invalid".into());
            }
            if delta != Vec3::ZERO {
                existing.pass.as_mut().unwrap().rebase(delta);
                existing.origin = origin;
                existing.rebase_count += 1;
            }
            if !Arc::ptr_eq(&existing.targets, &targets.0) {
                let previous = existing.pass.take();
                existing.pass = Some(SourceMotionPass::new(
                    &device,
                    &queue,
                    &cache,
                    assets.shaders.motion_camera.clone(),
                    targets.0.depth.sampled.as_ref().unwrap(),
                    snapshot,
                    previous,
                )?);
                existing.targets = targets.0.clone();
                existing.camera_frame = 0;
                existing.camera_token = 0;
                existing.validation = 0;
                existing.resize_count += 1;
                let pass = existing.pass.as_ref().unwrap();
                commands.entity(entity).insert(MotionDrawView::new(
                    pass.binding().clone(),
                    pass.copied_depth.clone(),
                ));
            } else if draw.is_none() {
                let pass = existing.pass.as_ref().unwrap();
                commands.entity(entity).insert(MotionDrawView::new(
                    pass.binding().clone(),
                    pass.copied_depth.clone(),
                ));
            }
            if let Some(error) = existing.pass.as_ref().unwrap().failure(&cache) {
                return Err(error);
            }
        } else {
            let pass = SourceMotionPass::new(
                &device,
                &queue,
                &cache,
                assets.shaders.motion_camera.clone(),
                targets.0.depth.sampled.as_ref().unwrap(),
                snapshot,
                None,
            )?;
            commands.entity(entity).insert(MotionDrawView::new(
                pass.binding().clone(),
                pass.copied_depth.clone(),
            ));
            commands.insert_resource(MotionFrame {
                motion_revision: frame.motion_revision,
                scene: frame.scene,
                first_token: 0,
                first_draws: [0; 3],
                first_camera_registered: false,
                pass: Some(pass),
                targets: targets.0.clone(),
                origin,
                camera_frame: 0,
                camera_token: 0,
                submitted_token: 0,
                validation: 0,
                resize_count: 0,
                rebase_count: 0,
            });
        }
        Ok(())
    })();
    if let Err(error) = result {
        *status.0.lock().unwrap() = RenderingState::Failed(error);
    }
}

fn encode_motion_camera(
    view: ViewQuery<(
        &SourceSurfaceView,
        &super::surface_graph::SurfaceViewTargets,
    )>,
    mut motion: Option<ResMut<MotionFrame>>,
    frame: Res<SourceViewFrame>,
    objects: Res<MotionObjects>,
    pipelines: Res<MotionPipelines>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    cache: Res<PipelineCache>,
    status: Res<SourceLightingStatus>,
    mut context: RenderContext,
) {
    let (source, targets) = view.into_inner();
    if source.index != 1
        || !objects.matches(frame.frame, frame.main_token)
        || !pipelines.ready(&cache)
        || matches!(*status.0.lock().unwrap(), RenderingState::Failed(_))
    {
        return;
    }
    let (Some(motion), Some(snapshot)) = (motion.as_mut(), frame.views[1].as_ref()) else {
        return;
    };
    if motion.first_token == 0 {
        motion.first_camera_registered = motion
            .pass
            .as_ref()
            .unwrap()
            .camera_history_matches(snapshot);
    }
    match motion.pass.as_mut().unwrap().encode_camera(
        context.command_encoder(),
        &device,
        &queue,
        &cache,
        &targets.0.motion.attachment,
        snapshot,
        frame.main_token,
    ) {
        Ok(true) => {
            motion.camera_frame = frame.frame;
            motion.camera_token = frame.main_token;
        }
        Ok(false) => {}
        Err(error) => *status.0.lock().unwrap() = RenderingState::Failed(error),
    }
}

fn encode_motion_objects(
    world: &World,
    view: ViewQuery<&SourceSurfaceView>,
    motion: Option<Res<MotionFrame>>,
    frame: Res<SourceViewFrame>,
    status: Res<SourceLightingStatus>,
    mut context: RenderContext,
) {
    let entity = view.entity();
    if view.into_inner().index != 1
        || matches!(*status.0.lock().unwrap(), RenderingState::Failed(_))
    {
        return;
    }
    let Some(motion) = motion else { return };
    if motion.camera_frame != frame.frame || motion.camera_token != frame.main_token {
        return;
    }
    if let Err(error) =
        super::motion_draw::render_objects(world, entity, frame.main_token, &mut context)
    {
        *status.0.lock().unwrap() = RenderingState::Failed(error);
    }
}

fn submit_motion(
    mut motion: Option<ResMut<MotionFrame>>,
    views: Query<(&SourceSurfaceView, &MotionDrawView)>,
    objects: Res<MotionObjects>,
    pipelines: Res<MotionPipelines>,
    frame: Res<SourceViewFrame>,
    progress: Res<SourceGraphProgress>,
    cache: Res<PipelineCache>,
    gate: Res<SourceExecutionGate>,
    queue: Res<RenderQueue>,
    status: Res<SourceLightingStatus>,
) {
    if matches!(*status.0.lock().unwrap(), RenderingState::Failed(_)) {
        return;
    }
    let Some(motion) = motion.as_mut() else {
        return;
    };
    let Some((_, draw)) = views.iter().find(|(view, _)| view.index == 1) else {
        return;
    };
    let mut state = progress.0.lock().unwrap();
    let token = frame.main_token;
    if state.presented
        && state.frame == frame.frame
        && state.token == token
        && motion.camera_frame == frame.frame
        && motion.camera_token == token
        && draw.encoded_token() == token
        && objects.matches(frame.frame, token)
        && token > motion.submitted_token
        && objects.can_complete(token)
    {
        if !motion.pass.as_mut().unwrap().completed(token) || !objects.completed(token) {
            *status.0.lock().unwrap() =
                RenderingState::Failed("Source motion history submission did not match".into());
            return;
        }
        if motion.first_token == 0 {
            motion.first_token = token;
            motion.first_draws = draw.counts();
        }
        motion.submitted_token = token;
        if motion.validation == 0 {
            motion.validation = gate.watch_submission(&queue);
        }
    }
    state.motion = MotionAudit {
        first_token: motion.first_token,
        first_draws: motion.first_draws,
        first_camera_registered: motion.first_camera_registered,
        pipelines_ready: pipelines.ready(&cache) && motion.pass.as_ref().unwrap().ready(&cache),
        objects_ready: objects.matches(frame.frame, token),
        frame: motion.camera_frame,
        camera_token: motion.camera_token,
        object_token: draw.encoded_token(),
        submitted_token: motion.submitted_token,
        registered_objects: objects.count() as u32,
        allocated_slots: objects.allocated_slots(),
        reused_slots: objects.reused_slots(),
        draws: draw.counts(),
        extent: motion.pass.as_ref().unwrap().extent.to_array(),
        validation: motion.validation,
        validated: gate.submission_validated(motion.validation),
        resize_count: motion.resize_count,
        rebase_count: motion.rebase_count,
    };
}
