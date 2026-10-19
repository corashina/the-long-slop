//! Source screen attenuation, stored in RGBA8 before each view's surface draws.
use super::{
    lighting_assets::LightingRenderAssets,
    lighting_scene::SourceSceneLighting,
    materials::RenderingState,
    render_targets::{COLLECTOR_FORMAT, SourceViewTargets},
    shadow_phase::ShadowDrawState,
    shadow_queue::ShadowQueues,
    shadow_scene::ShadowScene,
    source_lighting::SourceLightingStatus,
    surface_graph::{SourceSurface, SourceSurfaceStage, SurfaceViewTargets},
    surface_views::SourceSurfaceView,
    view_frame::SourceViewFrame,
};
use crate::reference::shadow_profile;
use bevy::render::{
    Render, RenderApp, RenderSystems,
    render_resource::{binding_types::*, *},
    renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery},
};
use bevy::{ecs as bevy_ecs, prelude::*};
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};
#[derive(Clone, Copy, ShaderType)]
struct CollectorUniform {
    inverse_projection: Mat4,
    world_from_view: Mat4,
    world_to_shadow: [Mat4; 4],
    spheres: [Vec4; 4],
    radii_squared: Vec4,
    projection: Vec4,
    atlas: Vec4,
}
#[derive(Component)]
pub struct CollectorView {
    targets: Arc<SourceViewTargets>,
    uniform: UniformBuffer<CollectorUniform>,
    key: super::shadow_preparation::CollectorKey,
    binding: BindGroup,
    pipeline: CachedRenderPipelineId,
    pub prepared_frame: u32,
    encoded_frame: AtomicU32,
}
impl CollectorView {
    pub fn ready(&self, cache: &PipelineCache) -> bool {
        cache.get_render_pipeline(self.pipeline).is_some()
    }
    pub fn encoded_frame(&self) -> u32 {
        self.encoded_frame.load(Ordering::Acquire)
    }
}
fn layout() -> BindGroupLayoutDescriptor {
    BindGroupLayoutDescriptor::new(
        "source_shadow_collector",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::VERTEX_FRAGMENT,
            (
                uniform_buffer::<CollectorUniform>(false),
                texture_2d(TextureSampleType::Float { filterable: false }),
                texture_depth_2d(),
                sampler(SamplerBindingType::Comparison),
            ),
        ),
    )
}
fn prepare(
    mut commands: Commands,
    scene: Res<ShadowScene>,
    frame: Res<SourceViewFrame>,
    live: Res<SourceSceneLighting>,
    assets: Option<Res<LightingRenderAssets>>,
    shaders: Res<super::fixed_assets::AdmittedShaders>,
    mut pipeline: Local<Option<CachedRenderPipelineId>>,
    mut views: Query<(
        Entity,
        &SourceSurfaceView,
        &SurfaceViewTargets,
        Option<&mut CollectorView>,
    )>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    cache: Res<PipelineCache>,
    status: Res<SourceLightingStatus>,
) {
    let mut timing = super::reflection_preparation::Timer::new(
        super::reflection_preparation::Stage::Collector,
        &frame,
    );
    let mut shadow_timing = shadow_profile::Timer::new(shadow_profile::Stage::Collector);
    let Some(shader) = shaders.get("shaders/source_shadow_collector.wgsl") else {
        return;
    };
    if scene.frame == 0
        || scene.frame != frame.frame
        || !frame.required_views_ready()
        || matches!(*status.0.lock().unwrap(), RenderingState::Failed(_))
    {
        return;
    }
    let (Some(assets), Some(live)) = (assets, live.0) else {
        return;
    };
    let light = &assets.record.lights[0];
    let rotation = live.sun_rotation;
    let h = |v: Vec3| Vec3::new(v.x, v.y, -v.z);
    let basis = [
        h(rotation * Vec3::X),
        h(rotation * Vec3::Y),
        h(rotation * -Vec3::Z),
    ];
    let id = *pipeline.get_or_insert_with(|| {
        cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("source_shadow_collector".into()),
            layout: vec![layout()],
            vertex: VertexState {
                shader: shader.clone(),
                entry_point: Some("vertex".into()),
                ..default()
            },
            fragment: Some(FragmentState {
                shader: shader.clone(),
                entry_point: Some("collect".into()),
                targets: vec![Some(ColorTargetState {
                    format: COLLECTOR_FORMAT,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            ..default()
        })
    });
    if let CachedPipelineState::Err(error) = cache.get_render_pipeline_state(id) {
        if !matches!(
            error,
            bevy::shader::ShaderCacheError::ShaderNotLoaded(_)
                | bevy::shader::ShaderCacheError::ShaderImportNotYetAvailable
        ) {
            *status.0.lock().unwrap() =
                RenderingState::Failed(format!("Source shadow collector pipeline: {error}"));
            return;
        }
    }
    for (entity, view, targets, mut previous) in &mut views {
        let Some(snapshot) = frame.views[view.index] else {
            continue;
        };
        timing.face(view.index);
        let key = super::shadow_preparation::CollectorKey {
            view: scene.view_revision(view.index, &frame),
            projection: snapshot.raster_projection.to_cols_array().map(f32::to_bits),
            view_from_world: snapshot.view_from_world.to_cols_array().map(f32::to_bits),
            near_far_strength: [snapshot.near, snapshot.far, light.shadows.strength]
                .map(f32::to_bits),
            atlas_size: targets.0.atlas_size,
            primary: view.index == 0,
        };
        if let Some(old) = previous.as_deref_mut() {
            if Arc::ptr_eq(&old.targets, &targets.0) && old.key == key {
                old.prepared_frame = frame.frame;
                shadow_timing.hit();
                continue;
            }
        }
        // Undo only the target clip conversion. Source shader60 then reconstructs
        // from its current projection; shader46 uses the implicit far ray.
        let mut p = snapshot.raster_projection.to_cols_array_2d();
        for column in &mut p {
            column[1] = -column[1];
            column[2] = column[3] - 2. * column[2];
        }
        let mut value = CollectorUniform {
            inverse_projection: Mat4::from_cols_array_2d(&p).inverse(),
            world_from_view: snapshot.view_from_world.inverse(),
            world_to_shadow: [Mat4::ZERO; 4],
            spheres: [Vec4::ZERO; 4],
            radii_squared: Vec4::ZERO,
            projection: Vec4::new(
                snapshot.far / snapshot.near - 1.,
                1.,
                1. - light.shadows.strength,
                if view.index == 0 { 1. } else { 0. },
            ),
            atlas: Vec4::new(
                1. / targets.0.atlas_size as f32,
                targets.0.atlas_size as f32,
                0.,
                0.,
            ),
        };
        if let Some(fit) = scene.fits[view.index] {
            value.atlas.z = 1.;
            for (index, cascade) in fit.cascades.iter().enumerate() {
                let clip = super::shadow_queue::cascade_projection(cascade, basis).0;
                // Clip to this top-left atlas tile. Projection flip and world conversion
                // are already in clip and must not be applied again.
                let offset = Vec2::new(
                    (index & 1) as f32 * 0.5 + 0.25,
                    (index >> 1) as f32 * 0.5 + 0.25,
                );
                let atlas = Mat4::from_cols(
                    Vec4::new(0.25, 0., 0., 0.),
                    Vec4::new(0., -0.25, 0., 0.),
                    Vec4::Z,
                    Vec4::new(offset.x, offset.y, 0., 1.),
                );
                value.world_to_shadow[index] = atlas * clip;
                value.spheres[index] = Vec4::new(
                    cascade.sphere_center[0],
                    cascade.sphere_center[1],
                    -cascade.sphere_center[2],
                    0.,
                );
                value.radii_squared[index] = cascade.sphere_radius * cascade.sphere_radius;
            }
        }
        if let Some(mut old) = previous {
            if Arc::ptr_eq(&old.targets, &targets.0) {
                let mut upload = shadow_profile::Timer::new(shadow_profile::Stage::Upload);
                let words = |v: &CollectorUniform| {
                    v.inverse_projection
                        .to_cols_array()
                        .into_iter()
                        .chain(v.world_from_view.to_cols_array())
                        .chain(
                            v.world_to_shadow
                                .into_iter()
                                .flat_map(|m| m.to_cols_array()),
                        )
                        .chain(v.spheres.into_iter().flat_map(|v| v.to_array()))
                        .chain(v.radii_squared.to_array())
                        .chain(v.projection.to_array())
                        .chain(v.atlas.to_array())
                };
                if super::shadow_preparation::same_floats(words(old.uniform.get()), words(&value)) {
                    upload.hit();
                } else {
                    old.uniform.set(value);
                    old.uniform.write_buffer(&device, &queue);
                    upload.upload(CollectorUniform::min_size().get());
                }
                old.key = key;
                old.prepared_frame = frame.frame;
                continue;
            }
        }
        let mut uniform = UniformBuffer::from(value);
        {
            let mut upload = shadow_profile::Timer::new(shadow_profile::Stage::Upload);
            uniform.write_buffer(&device, &queue);
            upload.upload(CollectorUniform::min_size().get());
        }
        let compare = device.create_sampler(&SamplerDescriptor {
            label: Some("source_shadow_linear_compare"),
            compare: Some(CompareFunction::GreaterEqual),
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            ..default()
        });
        let binding = device.create_bind_group(
            "source_shadow_collector",
            &cache.get_bind_group_layout(&layout()),
            &BindGroupEntries::sequential((
                uniform.binding().unwrap(),
                targets.0.depth.sampled.as_ref().unwrap(),
                targets.0.atlas.sampled.as_ref().unwrap(),
                &compare,
            )),
        );
        commands.entity(entity).insert(CollectorView {
            targets: targets.0.clone(),
            uniform,
            key,
            binding,
            pipeline: id,
            prepared_frame: frame.frame,
            encoded_frame: AtomicU32::new(0),
        });
    }
}
fn collect(
    view: ViewQuery<(&SourceSurfaceView, &CollectorView, &ShadowDrawState)>,
    frame: Res<SourceViewFrame>,
    queues: Res<ShadowQueues>,
    cache: Res<PipelineCache>,
    status: Res<SourceLightingStatus>,
    mut context: RenderContext,
) {
    let (view, collector, shadow) = view.into_inner();
    if !queues.ready[view.index]
        || queues.frame != frame.frame
        || collector.prepared_frame != frame.frame
        || shadow.encoded_frame() != frame.frame
        || matches!(*status.0.lock().unwrap(), RenderingState::Failed(_))
    {
        return;
    }
    let Some(pipeline) = cache.get_render_pipeline(collector.pipeline) else {
        return;
    };
    let colors = [Some(RenderPassColorAttachment {
        view: &collector.targets.collector.attachment,
        depth_slice: None,
        resolve_target: None,
        ops: Operations {
            load: LoadOp::Clear(LinearRgba::WHITE.into()),
            store: StoreOp::Store,
        },
    })];
    let mut pass = context.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("source_shadow_collector"),
        color_attachments: &colors,
        ..default()
    });
    pass.set_render_pipeline(pipeline);
    pass.set_bind_group(0, &collector.binding, &[]);
    pass.draw(0..3, 0..1);
    drop(pass);
    collector
        .encoded_frame
        .store(frame.frame, Ordering::Release);
}
pub fn install(app: &mut App) {
    app.sub_app_mut(RenderApp)
        .add_systems(Render, prepare.in_set(RenderSystems::PrepareBindGroups))
        .add_systems(SourceSurface, collect.in_set(SourceSurfaceStage::Collector));
}
