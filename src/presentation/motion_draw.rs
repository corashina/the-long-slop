//! Rigid motion reuses each prepared opaque instance and its original draw ID.
use super::{
    lighting_scene::SourceWorldOrigin,
    materials::{GeneratedWorldMaterial, MaterialLoading, RenderingState, SourceMaterial},
    motion_vectors::{MotionMode, MotionModelHistory, MotionObjectParams, SourceMotionPass},
    renderers::SourceRenderer,
    source_lighting::{SetSourceViewCamera, SourceLightingStatus},
    view_frame::SourceViewFrame,
};
use bevy::core_pipeline::core_3d::{AlphaMask3d, Opaque3d};
use bevy::ecs as bevy_ecs;
use bevy::ecs::{
    query::ROQueryItem,
    system::{SystemParamItem, lifetimeless::SRes},
};
use bevy::pbr::{DrawMesh, RenderMeshInstances, SetMeshBindGroup};
use bevy::prelude::*;
use bevy::render::{
    Extract, ExtractSchedule, Render, RenderApp, RenderSystems,
    render_phase::*,
    render_resource::*,
    renderer::{RenderContext, RenderDevice, RenderQueue},
    view::ExtractedView,
};
use bevy::shader::ShaderDefVal;
use std::{
    collections::HashMap,
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
};

#[derive(Clone, Copy)]
struct ModelSnapshot {
    entity: Entity,
    model: Mat4,
    mode: MotionMode,
}
#[derive(Resource, Default)]
pub struct MotionScene {
    rows: Vec<ModelSnapshot>,
    frame: u32,
    token: u32,
    origin: [f64; 3],
}
fn snapshot_models(
    frame: Res<SourceViewFrame>,
    origin: Res<SourceWorldOrigin>,
    shadow: Res<super::shadow_scene::ShadowScene>,
    mut output: ResMut<MotionScene>,
    status: Res<SourceLightingStatus>,
    models: Query<
        (
            Entity,
            &GlobalTransform,
            Option<&SourceRenderer>,
            Option<&GeneratedWorldMaterial>,
        ),
        (
            With<MeshMaterial3d<SourceMaterial>>,
            Without<super::materials::SourceMaterialWarmup>,
        ),
    >,
) {
    output.frame = 0;
    output.rows.clear();
    if frame.views[1].is_none() || frame.main_token == 0 || shadow.capacity() == 0 {
        return;
    }
    if output.rows.capacity() == 0 {
        output.rows.reserve_exact(shadow.capacity());
    }
    for (entity, model, source, generated) in &models {
        let mode = match source.map(|source| source.0.motion_vectors) {
            Some(0) => MotionMode::Camera,
            Some(1) => MotionMode::Object,
            Some(2) => MotionMode::ForceNoMotion,
            None if generated.is_some() => MotionMode::Object,
            _ => {
                *status.0.lock().unwrap() =
                    RenderingState::Failed("Source motion renderer metadata is missing".into());
                return;
            }
        };
        if output.rows.len() == output.rows.capacity() {
            *status.0.lock().unwrap() =
                RenderingState::Failed("Source motion scene capacity was exceeded".into());
            return;
        }
        output.rows.push(ModelSnapshot {
            entity,
            model: model.to_matrix(),
            mode,
        });
    }
    output.frame = frame.frame;
    output.token = frame.main_token;
    output.origin = origin.0;
}
fn extract_models(input: Extract<Res<MotionScene>>, mut output: ResMut<MotionScene>) {
    if output.rows.capacity() == 0 {
        output.rows.reserve_exact(input.rows.capacity());
    }
    assert!(
        input.rows.len() <= output.rows.capacity(),
        "Source motion extraction capacity changed"
    );
    output.rows.clear();
    output.rows.extend_from_slice(&input.rows);
    output.frame = input.frame;
    output.token = input.token;
    output.origin = input.origin;
}
struct MotionObject {
    history: Mutex<MotionModelHistory>,
    uniform: UniformBuffer<MotionObjectParams>,
    binding: BindGroup,
    eligible: bool,
    frame: u32,
}
#[derive(Resource, Default)]
pub struct MotionObjects {
    objects: bevy::ecs::entity::EntityHashMap<MotionObject>,
    pub frame: u32,
    token: u32,
    origin: Option<[f64; 3]>,
    completed: AtomicU32,
}
impl MotionObjects {
    pub fn matches(&self, frame: u32, token: u32) -> bool {
        frame != 0 && token != 0 && self.frame == frame && self.token == token
    }
    pub fn can_complete(&self, token: u32) -> bool {
        token != 0 && token == self.token && token > self.completed.load(Ordering::Acquire)
    }
    pub fn origin(&self) -> Option<[f64; 3]> {
        self.origin
    }
    pub fn completed(&self, token: u32) -> bool {
        let old = self.completed.load(Ordering::Acquire);
        if token == 0 || token != self.token || token <= old {
            return false;
        }
        if self
            .completed
            .compare_exchange(old, token, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return false;
        }
        for object in self.objects.values() {
            object.history.lock().unwrap().completed(token);
        }
        true
    }
    pub fn count(&self) -> usize {
        self.objects.len()
    }
    pub fn logical_uniform_bytes(&self) -> u64 {
        self.objects.len() as u64 * 144
    }
}
// Runtime render poses subtract this same target-space shift on all three axes.
fn target_rebase_delta(current: [f64; 3], previous: [f64; 3]) -> Vec3 {
    Vec3::from_array(std::array::from_fn(|i| (current[i] - previous[i]) as f32))
}
fn prepare_models(
    scene: Res<MotionScene>,
    mut objects: ResMut<MotionObjects>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    cache: Res<PipelineCache>,
    status: Res<SourceLightingStatus>,
) {
    objects.frame = 0;
    if scene.frame == 0 {
        return;
    }
    if objects.objects.capacity() == 0 {
        objects.objects.reserve(scene.rows.capacity());
    }
    for object in objects.objects.values_mut() {
        object.frame = 0;
    }
    for row in &scene.rows {
        if let Some(object) = objects.objects.get_mut(&row.entity) {
            object.frame = scene.frame;
        }
    }
    objects
        .objects
        .retain(|_, object| object.frame == scene.frame);
    let delta = objects
        .origin
        .map_or(Vec3::ZERO, |old| target_rebase_delta(scene.origin, old));
    if delta != Vec3::ZERO {
        for object in objects.objects.values_mut() {
            object.history.get_mut().unwrap().rebase(delta);
        }
    }
    objects.origin = Some(scene.origin);
    for row in &scene.rows {
        if !row.model.is_finite() {
            *status.0.lock().unwrap() =
                RenderingState::Failed("Source motion model is invalid".into());
            return;
        }
        if let Some(object) = objects.objects.get_mut(&row.entity) {
            let history = object.history.get_mut().unwrap();
            history.update(scene.token, row.model);
            object.eligible = history.eligible(scene.token, row.mode);
            object.frame = scene.frame;
            if object.eligible {
                object.uniform.set(history.params(scene.token, row.mode));
                object.uniform.write_buffer(&device, &queue);
            }
        } else {
            if objects.objects.len() == objects.objects.capacity() {
                *status.0.lock().unwrap() =
                    RenderingState::Failed("Source motion object capacity was exceeded".into());
                return;
            }
            let history = MotionModelHistory::new(row.model, scene.token);
            let mut uniform = UniformBuffer::from(history.params(scene.token, row.mode));
            uniform.write_buffer(&device, &queue);
            let binding = device.create_bind_group(
                "source_motion_object",
                &cache.get_bind_group_layout(&SourceMotionPass::object_layout()),
                &BindGroupEntries::single(uniform.binding().unwrap()),
            );
            objects.objects.insert(
                row.entity,
                MotionObject {
                    eligible: history.eligible(scene.token, row.mode),
                    history: Mutex::new(history),
                    uniform,
                    binding,
                    frame: scene.frame,
                },
            );
        }
    }
    objects.token = scene.token;
    objects.frame = scene.frame;
}

#[derive(Resource)]
struct MotionShader(Handle<Shader>);
#[derive(Resource, Default)]
pub struct MotionPipelines {
    inspected: usize,
    variants: HashMap<CachedRenderPipelineId, CachedRenderPipelineId>,
}
impl MotionPipelines {
    pub fn ready(&self, cache: &PipelineCache) -> bool {
        !self.variants.is_empty()
            && self
                .variants
                .values()
                .all(|id| cache.get_render_pipeline(*id).is_some())
    }
    pub fn failure(&self, cache: &PipelineCache) -> Option<String> {
        self.variants
            .values()
            .find_map(|id| super::postprocessing::pipeline_failure(cache, &[*id]))
    }
}
fn source_flag(defs: &[ShaderDefVal], name: &str) -> bool {
    defs.iter()
        .any(|d| matches!(d,ShaderDefVal::Bool(key,true) if key==name))
}
fn prepare_pipelines(
    mut pipelines: ResMut<MotionPipelines>,
    shader: Res<MotionShader>,
    cache: Res<PipelineCache>,
    loading: Res<MaterialLoading>,
    status: Res<SourceLightingStatus>,
) {
    if matches!(*status.0.lock().unwrap(), RenderingState::Failed(_)) {
        return;
    }
    if pipelines.variants.capacity() == 0 {
        let Some(capacity) = loading.source_shadow_capacity() else {
            return;
        };
        pipelines.variants.reserve(capacity * 9);
    }
    let first = pipelines.inspected;
    for (index, pipeline) in cache.pipelines().enumerate().skip(first) {
        pipelines.inspected = index + 1;
        let PipelineDescriptor::RenderPipelineDescriptor(base) = &pipeline.descriptor else {
            continue;
        };
        let Some(fragment) = &base.fragment else {
            continue;
        };
        if !source_flag(&fragment.shader_defs, "SOURCE_SURFACE")
            || source_flag(&fragment.shader_defs, "SOURCE_FORWARD_ADD")
            || source_flag(&fragment.shader_defs, "SOURCE_TRANSPARENT")
        {
            continue;
        }
        if pipelines.variants.len() == pipelines.variants.capacity() {
            *status.0.lock().unwrap() =
                RenderingState::Failed("Source motion pipeline capacity was exceeded".into());
            return;
        }
        let mut descriptor = base.clone();
        descriptor.label = Some("source_motion_object".into());
        descriptor.layout[0] = super::source_lighting::source_camera_layout();
        descriptor.layout[1] = SourceMotionPass::object_camera_layout();
        descriptor.layout[3] = SourceMotionPass::object_layout();
        descriptor.multisample = MultisampleState::default();
        descriptor.vertex.shader = shader.0.clone();
        descriptor.vertex.entry_point = Some("object_vertex".into());
        let fragment = descriptor.fragment.as_mut().unwrap();
        fragment.shader = shader.0.clone();
        fragment.entry_point = Some("object_fragment".into());
        fragment.targets = vec![Some(ColorTargetState {
            format: TextureFormat::Rg16Float,
            blend: None,
            write_mask: ColorWrites::ALL,
        })];
        for defs in [
            &mut descriptor.vertex.shader_defs,
            &mut fragment.shader_defs,
        ] {
            defs.retain(|v|!matches!(v,ShaderDefVal::Bool(key,_) if key=="SOURCE_SURFACE" || key=="SOURCE_FORWARD_ADD"));
        }
        descriptor.primitive.cull_mode = Some(Face::Back);
        descriptor.depth_stencil = Some(DepthStencilState {
            format: TextureFormat::Depth24PlusStencil8,
            depth_write_enabled: Some(false),
            depth_compare: Some(CompareFunction::GreaterEqual),
            stencil: default(),
            bias: default(),
        });
        let id = cache.queue_render_pipeline(*descriptor);
        pipelines
            .variants
            .insert(CachedRenderPipelineId::new(index), id);
    }
    if let Some(error) = pipelines.failure(&cache) {
        *status.0.lock().unwrap() = RenderingState::Failed(error);
    }
}

#[derive(Component)]
pub struct MotionDrawView {
    pub copied_depth: TextureView,
    camera: BindGroup,
    active: AtomicBool,
    epoch: AtomicU32,
    attempts: AtomicU32,
    successes: AtomicU32,
    encoded: AtomicU32,
}
impl MotionDrawView {
    pub fn new(camera: BindGroup, copied_depth: TextureView) -> Self {
        Self {
            camera,
            copied_depth,
            active: AtomicBool::new(false),
            epoch: AtomicU32::new(0),
            attempts: AtomicU32::new(0),
            successes: AtomicU32::new(0),
            encoded: AtomicU32::new(0),
        }
    }
    pub fn active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }
    pub fn counts(&self) -> [u32; 3] {
        [
            self.epoch.load(Ordering::Relaxed),
            self.attempts.load(Ordering::Relaxed),
            self.successes.load(Ordering::Relaxed),
        ]
    }
    pub fn encoded_token(&self) -> u32 {
        self.encoded.load(Ordering::Acquire)
    }
    fn begin(&self) -> MotionScope<'_> {
        self.epoch.fetch_add(1, Ordering::Relaxed);
        self.attempts.store(0, Ordering::Relaxed);
        self.successes.store(0, Ordering::Relaxed);
        self.encoded.store(0, Ordering::Release);
        self.active.store(true, Ordering::Relaxed);
        MotionScope(self)
    }
}
struct MotionScope<'a>(&'a MotionDrawView);
impl Drop for MotionScope<'_> {
    fn drop(&mut self) {
        self.0.active.store(false, Ordering::Relaxed);
    }
}
pub(crate) struct SelectMotion;
impl<P: CachedRenderPipelinePhaseItem> RenderCommand<P> for SelectMotion {
    type Param = (
        SRes<MotionPipelines>,
        SRes<PipelineCache>,
        SRes<MotionObjects>,
    );
    type ViewQuery = &'static MotionDrawView;
    type ItemQuery = ();
    fn render<'w>(
        item: &P,
        view: ROQueryItem<'w, '_, Self::ViewQuery>,
        _: Option<()>,
        (pipelines, cache, objects): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let objects = objects.into_inner();
        let Some(object) = objects.objects.get(&item.main_entity().id()) else {
            return RenderCommandResult::Failure("Source motion object is missing");
        };
        if !object.eligible || object.frame != objects.frame {
            return RenderCommandResult::Skip;
        }
        let Some(id) = pipelines.variants.get(&item.cached_pipeline()) else {
            return RenderCommandResult::Failure("Source motion pipeline is missing");
        };
        let Some(pipeline) = cache.into_inner().get_render_pipeline(*id) else {
            return RenderCommandResult::Skip;
        };
        if item.batch_range().len() != 1 {
            return RenderCommandResult::Failure("Source motion requires one prepared instance");
        }
        pass.set_render_pipeline(pipeline);
        pass.set_bind_group(1, &view.camera, &[]);
        pass.set_bind_group(3, &object.binding, &[]);
        RenderCommandResult::Success
    }
}
pub(crate) struct MotionMark<const DONE: bool>;
impl<P: PhaseItem, const DONE: bool> RenderCommand<P> for MotionMark<DONE> {
    type Param = ();
    type ViewQuery = &'static MotionDrawView;
    type ItemQuery = ();
    fn render<'w>(
        _: &P,
        view: ROQueryItem<'w, '_, Self::ViewQuery>,
        _: Option<()>,
        _: (),
        _: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        if DONE {
            view.successes.fetch_add(1, Ordering::Relaxed);
        } else {
            view.attempts.fetch_add(1, Ordering::Relaxed);
        }
        RenderCommandResult::Success
    }
}
pub(crate) type DrawMotion = (
    SelectMotion,
    MotionMark<false>,
    SetSourceViewCamera,
    SetMeshBindGroup<2>,
    DrawMesh,
    MotionMark<true>,
);

pub(crate) trait MotionBatchItem: BinnedPhaseItem + CachedRenderPipelinePhaseItem {
    fn keys(&self) -> (Self::BatchSetKey, Self::BinKey);
}
impl MotionBatchItem for Opaque3d {
    fn keys(&self) -> (Self::BatchSetKey, Self::BinKey) {
        (self.batch_set_key.clone(), self.bin_key.clone())
    }
}
impl MotionBatchItem for AlphaMask3d {
    fn keys(&self) -> (Self::BatchSetKey, Self::BinKey) {
        (self.batch_set_key.clone(), self.bin_key.clone())
    }
}
pub(crate) fn split_draw<'w, P: MotionBatchItem>(
    world: &'w World,
    pass: &mut TrackedRenderPass<'w>,
    view: Entity,
    item: &P,
    draw: &mut dyn Draw<P>,
) -> Result<(), DrawError> {
    let instances = world.resource::<RenderMeshInstances>();
    if !matches!(instances, RenderMeshInstances::CpuBuilding(_)) {
        return Err(DrawError::RenderCommandFailure(
            "Source motion requires the WebGL CPU instance producer",
        ));
    }
    let extracted = world
        .get::<ExtractedView>(view)
        .ok_or(DrawError::InvalidViewQuery)?;
    let phase = world
        .resource::<ViewBinnedRenderPhases<P>>()
        .get(&extracted.retained_view_entity)
        .ok_or(DrawError::InvalidViewQuery)?;
    let keys = item.keys();
    let objects = world.resource::<MotionObjects>();
    if let Some(bin) = phase.batchable_meshes.get(&keys) {
        let start = bin.entities().get_index_of(&item.main_entity()).ok_or(
            DrawError::RenderCommandFailure("Source motion batch representative is missing"),
        )?;
        let mut slot = item.batch_range().start;
        for (main, _) in bin.entities().iter().skip(start) {
            if slot == item.batch_range().end {
                break;
            }
            if instances.mesh_asset_id(*main).is_none() {
                continue;
            }
            let object = objects
                .objects
                .get(&main.id())
                .ok_or(DrawError::RenderCommandFailure(
                    "Source motion batch model is missing",
                ))?;
            if object.eligible && object.frame == objects.frame {
                let single = P::new(
                    keys.0.clone(),
                    keys.1.clone(),
                    (Entity::PLACEHOLDER, *main),
                    slot..slot + 1,
                    item.extra_index(),
                );
                draw.draw(world, pass, view, &single)?;
            }
            slot += 1;
        }
        if slot != item.batch_range().end {
            return Err(DrawError::RenderCommandFailure(
                "Source motion batch range is incomplete",
            ));
        }
        Ok(())
    } else if phase.unbatchable_meshes.contains_key(&keys) {
        draw.draw(world, pass, view, item)
    } else {
        Err(DrawError::RenderCommandFailure(
            "Source motion prepared bin is unsupported",
        ))
    }
}

/// Call after the camera quad copied depth. The source objects overwrite covered velocity samples.
pub fn render_objects(
    world: &World,
    entity: Entity,
    token: u32,
    context: &mut RenderContext,
) -> Result<bool, String> {
    let cache = world.resource::<PipelineCache>();
    let pipelines = world.resource::<MotionPipelines>();
    let objects = world.resource::<MotionObjects>();
    if !pipelines.ready(cache) || token == 0 || token != objects.token || objects.frame == 0 {
        return Ok(false);
    }
    let view = world
        .get::<ExtractedView>(entity)
        .ok_or("Source motion view is missing")?;
    let state = world
        .get::<MotionDrawView>(entity)
        .ok_or("Source motion draw state is missing")?;
    let targets = world
        .get::<super::surface_graph::SurfaceViewTargets>(entity)
        .ok_or("Source motion target is missing")?;
    let opaque = world
        .resource::<ViewBinnedRenderPhases<Opaque3d>>()
        .get(&view.retained_view_entity)
        .ok_or("Source motion opaque phase is missing")?;
    let masked = world
        .resource::<ViewBinnedRenderPhases<AlphaMask3d>>()
        .get(&view.retained_view_entity)
        .ok_or("Source motion masked phase is missing")?;
    let _scope = state.begin();
    let colors = [Some(RenderPassColorAttachment {
        view: &targets.0.motion.attachment,
        depth_slice: None,
        resolve_target: None,
        ops: Operations {
            load: LoadOp::Load,
            store: StoreOp::Store,
        },
    })];
    let mut pass = context.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("source_motion_objects"),
        color_attachments: &colors,
        depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
            view: &state.copied_depth,
            depth_ops: Some(Operations {
                load: LoadOp::Load,
                store: StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        ..default()
    });
    opaque
        .render(&mut pass, world, entity)
        .and_then(|_| masked.render(&mut pass, world, entity))
        .map_err(|e| format!("Source motion draw: {e:?}"))?;
    let [epoch, attempts, successes] = state.counts();
    if attempts != successes {
        return Err(format!(
            "Source motion skipped draw at epoch{epoch}: {successes}/{attempts}"
        ));
    }
    state.encoded.store(token, Ordering::Release);
    Ok(true)
}

pub fn install(app: &mut App) {
    let shader = app
        .world()
        .resource::<AssetServer>()
        .load("shaders/source_motion_objects.wgsl");
    app.init_resource::<MotionScene>().add_systems(
        PostUpdate,
        snapshot_models.after(super::shadow_scene::prepare_scene),
    );
    app.sub_app_mut(RenderApp)
        .insert_resource(MotionShader(shader))
        .init_resource::<MotionScene>()
        .init_resource::<MotionObjects>()
        .init_resource::<MotionPipelines>()
        .add_systems(ExtractSchedule, extract_models)
        .add_systems(
            Render,
            (prepare_models, prepare_pipelines).in_set(RenderSystems::PrepareResources),
        );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn physical_rebase_keeps_a_static_target_model_motionless() {
        let before = Mat4::from_translation(Vec3::new(12., 3., -20.));
        let previous_origin = [1000., 0., -1600.];
        let current_origin = [1400., 20., -2400.];
        // StartingArea::sync_origin and app::advance_physics subtract this raw XYZ shift.
        let physical_shift = Vec3::from_array(std::array::from_fn(|i| {
            (current_origin[i] - previous_origin[i]) as f32
        }));
        let shifted_model = Mat4::from_translation(-physical_shift) * before;
        let mut history = MotionModelHistory::new(before, 1);
        history.completed(1);
        history.rebase(target_rebase_delta(current_origin, previous_origin));
        history.update(2, shifted_model);
        let params = history.params(2, MotionMode::Object);
        assert_eq!(params.current_model, params.previous_model);
        assert!(!history.eligible(2, MotionMode::Object));
        let vp =
            Mat4::perspective_rh_gl(1., 1.5, 0.3, 100.) * Mat4::from_translation(physical_shift);
        let vertex = Vec4::new(0.2, 0.1, 0., 1.);
        let current_clip = vp * params.current_model * vertex;
        let previous_clip = vp * params.previous_model * vertex;
        assert_eq!(
            current_clip.truncate() / current_clip.w,
            previous_clip.truncate() / previous_clip.w
        );
    }
}
