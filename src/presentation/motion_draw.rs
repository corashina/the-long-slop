//! Rigid motion reuses each prepared opaque instance and its original draw ID.
use super::{
    lighting::ExtractedLighting,
    lighting_scene::SourceWorldOrigin,
    materials::{MaterialLoading, RenderingState},
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
use bevy::pbr::{RenderMeshInstances, SetMeshBindGroup};
use bevy::prelude::*;
use bevy::{
    asset::AssetId,
    render::{
        mesh::{RenderMesh, RenderMeshBufferInfo, allocator::MeshAllocator},
        render_asset::RenderAssets,
    },
};
#[derive(Component, Clone)]
pub struct AuthoredMotionMesh(pub Handle<Mesh>);
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
    mesh: AssetId<Mesh>,
    mode: MotionMode,
    camera_visible: bool,
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
    master: Res<super::lighting::SourceLighting>,
    mut output: ResMut<MotionScene>,
    status: Res<SourceLightingStatus>,
    models: Query<
        (
            Entity,
            &GlobalTransform,
            &SourceRenderer,
            &Mesh3d,
            Option<&AuthoredMotionMesh>,
        ),
        (
            With<SourceRenderer>,
            Without<super::materials::SourceMaterialWarmup>,
            Without<super::save::HiddenWorld>,
        ),
    >,
) {
    output.frame = 0;
    output.rows.clear();
    if !master.materials_prepared
        || master.warming
        || frame.views[1].is_none()
        || frame.main_token == 0
        || shadow.capacity() == 0
    {
        return;
    }
    if output.rows.capacity() < shadow.capacity() {
        let count = shadow.capacity().saturating_sub(output.rows.len());
        output.rows.reserve_exact(count);
    }
    for (entity, model, source, mesh, authored) in &models {
        let mode = match source.0.motion_vectors {
            0 => MotionMode::Camera,
            1 => MotionMode::Object,
            2 => MotionMode::ForceNoMotion,
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
            mesh: authored.map_or(mesh.0.id(), |m| m.0.id()),
            mode,
            camera_visible: source.0.cast_shadows != 3,
        });
    }
    output.frame = frame.frame;
    output.token = frame.main_token;
    output.origin = origin.0;
}
fn extract_models(input: Extract<Res<MotionScene>>, mut output: ResMut<MotionScene>) {
    if output.rows.capacity() < input.rows.capacity() {
        let count = input.rows.capacity().saturating_sub(output.rows.len());
        output.rows.reserve_exact(count);
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
    entity: Option<Entity>,
    mesh: AssetId<Mesh>,
    history: Mutex<MotionModelHistory>,
    uniform: UniformBuffer<MotionObjectParams>,
    binding: BindGroup,
    eligible: bool,
    frame: u32,
}
#[derive(Resource, Default)]
pub struct MotionObjects {
    motion_revision: u32,
    scene: u32,
    slots: Vec<MotionObject>,
    lookup: bevy::ecs::entity::EntityHashMap<usize>,
    reused_slots: u32,
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
        for &index in self.lookup.values() {
            self.slots[index].history.lock().unwrap().completed(token);
        }
        true
    }
    pub fn count(&self) -> usize {
        self.lookup.len()
    }
    pub fn allocated_slots(&self) -> u32 {
        self.slots.len() as u32
    }
    pub fn reused_slots(&self) -> u32 {
        self.reused_slots
    }
    fn get(&self, entity: Entity) -> Option<&MotionObject> {
        self.lookup.get(&entity).map(|&index| &self.slots[index])
    }
    fn sync_membership(
        &mut self,
        rows: &[ModelSnapshot],
        frame: u32,
        token: u32,
    ) -> Result<(), &'static str> {
        if rows.len() > self.slots.len() {
            return Err("Source motion object capacity was exceeded");
        }
        for slot in &mut self.slots {
            slot.frame = 0;
        }
        let mut changed = rows.len() != self.lookup.len();
        // Protect every retained slot before assigning any new entity.
        for row in rows {
            if let Some(&index) = self.lookup.get(&row.entity) {
                self.slots[index].frame = frame;
            } else {
                changed = true;
            }
        }
        if !changed {
            return Ok(());
        }
        let mut free = 0;
        for row in rows {
            if self.lookup.contains_key(&row.entity) {
                continue;
            }
            while free < self.slots.len() && self.slots[free].frame != 0 {
                free += 1;
            }
            let slot = self
                .slots
                .get_mut(free)
                .ok_or("Source motion object capacity was exceeded")?;
            self.reused_slots += u32::from(slot.entity.is_some());
            slot.entity = Some(row.entity);
            slot.frame = frame;
            slot.eligible = false;
            *slot.history.get_mut().unwrap() = MotionModelHistory::new(row.model, token);
        }
        // Clearing restores the preallocated hash table after streaming removals.
        self.lookup.clear();
        for (index, slot) in self.slots.iter().enumerate() {
            if slot.frame == frame {
                self.lookup.insert(slot.entity.unwrap(), index);
            }
        }
        Ok(())
    }
    pub fn logical_uniform_bytes(&self) -> u64 {
        self.slots.len() as u64 * 144
    }
}
// Runtime render poses subtract this same target-space shift on all three axes.
fn target_rebase_delta(current: [f64; 3], previous: [f64; 3]) -> Vec3 {
    Vec3::from_array(std::array::from_fn(|i| (current[i] - previous[i]) as f32))
}
fn prepare_models(
    scene: Res<MotionScene>,
    frame: Res<SourceViewFrame>,
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
    if objects.slots.len() < scene.rows.capacity() {
        let capacity = scene.rows.capacity() - objects.slots.len();
        objects.slots.reserve_exact(capacity);
        objects.lookup.reserve(capacity);
        let layout = cache.get_bind_group_layout(&SourceMotionPass::object_layout());
        for _ in 0..capacity {
            let history = MotionModelHistory::new(Mat4::IDENTITY, 0);
            let mut uniform = UniformBuffer::from(history.params(0, MotionMode::Object));
            uniform.write_buffer(&device, &queue);
            let binding = device.create_bind_group(
                "source_motion_object",
                &layout,
                &BindGroupEntries::single(uniform.binding().unwrap()),
            );
            objects.slots.push(MotionObject {
                entity: None,
                mesh: AssetId::default(),
                history: Mutex::new(history),
                uniform,
                binding,
                eligible: false,
                frame: 0,
            });
        }
    }
    let recreate = objects.scene != frame.scene || objects.motion_revision != frame.motion_revision;
    objects.motion_revision = frame.motion_revision;
    objects.scene = frame.scene;
    if recreate {
        objects.origin = None;
        objects.completed.store(0, Ordering::Release);
    }
    let delta = objects
        .origin
        .map_or(Vec3::ZERO, |old| target_rebase_delta(scene.origin, old));
    if delta != Vec3::ZERO {
        for object in &mut objects.slots {
            object.history.get_mut().unwrap().rebase(delta);
        }
    }
    objects.origin = Some(scene.origin);
    if scene.rows.iter().any(|row| !row.model.is_finite()) {
        *status.0.lock().unwrap() = RenderingState::Failed("Source motion model is invalid".into());
        return;
    }
    if let Err(error) = objects.sync_membership(&scene.rows, scene.frame, scene.token) {
        *status.0.lock().unwrap() = RenderingState::Failed(error.into());
        return;
    }
    for row in &scene.rows {
        let index = objects.lookup[&row.entity];
        let object = &mut objects.slots[index];
        let history = object.history.get_mut().unwrap();
        if recreate {
            *history = MotionModelHistory::new(row.model, scene.token);
        }
        object.mesh = row.mesh;
        history.update(scene.token, row.model);
        object.eligible = row.camera_visible && history.eligible(scene.token, row.mode);
        if object.eligible {
            object.uniform.set(history.params(scene.token, row.mode));
            object.uniform.write_buffer(&device, &queue);
        }
    }
    objects.token = scene.token;
    objects.frame = scene.frame;
}

#[derive(Resource, Default)]
pub struct MotionPipelines {
    inspected: usize,
    isolated: std::collections::HashSet<CachedRenderPipelineId>,
    world_variants: HashMap<
        (
            CachedRenderPipelineId,
            bevy::mesh::MeshVertexBufferLayoutRef,
        ),
        CachedRenderPipelineId,
    >,
    variants: HashMap<CachedRenderPipelineId, CachedRenderPipelineId>,
}
impl MotionPipelines {
    pub fn revisit(&mut self, id: CachedRenderPipelineId) {
        self.inspected = self.inspected.min(id.id());
    }
    pub fn ready(&self, cache: &PipelineCache) -> bool {
        !self.variants.is_empty()
            && self
                .variants
                .iter()
                .filter(|(base, _)| !self.isolated.contains(base))
                .map(|(_, id)| id)
                .chain(
                    self.world_variants
                        .iter()
                        .filter(|((base, _), _)| !self.isolated.contains(base))
                        .map(|(_, id)| id),
                )
                .filter(|id| !cache.render_pipeline_is_held(**id))
                .all(|id| cache.get_render_pipeline(*id).is_some())
    }
    pub(crate) fn physically_settled(
        &self,
        base: CachedRenderPipelineId,
        cache: &PipelineCache,
    ) -> bool {
        self.variants
            .get(&base)
            .into_iter()
            .chain(
                self.world_variants
                    .iter()
                    .filter(|((owner, _), _)| *owner == base)
                    .map(|(_, id)| id),
            )
            .all(|id| match cache.get_render_pipeline_state(*id) {
                CachedPipelineState::Err(
                    bevy::shader::ShaderCacheError::ShaderNotLoaded(_)
                    | bevy::shader::ShaderCacheError::ShaderImportNotYetAvailable,
                ) => false,
                CachedPipelineState::Ok(_) | CachedPipelineState::Err(_) => true,
                _ => false,
            })
    }
    pub fn failure(&self, cache: &PipelineCache) -> Option<String> {
        self.variants
            .iter()
            .filter(|(base, _)| !self.isolated.contains(base))
            .map(|(_, id)| id)
            .chain(
                self.world_variants
                    .iter()
                    .filter(|((base, _), _)| !self.isolated.contains(base))
                    .map(|(_, id)| id),
            )
            .filter(|id| !cache.render_pipeline_is_held(**id))
            .find_map(|id| super::postprocessing::pipeline_failure(cache, &[*id]))
    }
}
/// Compile the source replacement motion pass against the retained authored stream.
pub fn prepare_world_motion(
    world: &mut World,
    base: CachedRenderPipelineId,
    mesh: AssetId<Mesh>,
    owner: Option<super::gpu_prepare::GpuPreparation>,
) -> Result<bool, String> {
    if owner.as_ref().is_some_and(|o| o.cancelled()) {
        return Ok(false);
    }
    let cache = world.resource::<PipelineCache>();
    let original = cache.get_render_pipeline_descriptor(base);
    let Some(fragment) = &original.fragment else {
        return Ok(true);
    };
    if !source_flag(&fragment.shader_defs, "SOURCE_WORLD") {
        if !source_flag(&fragment.shader_defs, "SOURCE_SURFACE") {
            return Ok(true);
        }
        let pipelines = world.resource::<MotionPipelines>();
        if pipelines.inspected <= base.id() {
            return Ok(false);
        }
        return match pipelines.variants.get(&base) {
            None => Ok(true),
            Some(id) => match super::postprocessing::pipeline_failure(cache, &[*id]) {
                Some(error) => Err(error),
                None => Ok(cache.get_render_pipeline(*id).is_some()),
            },
        };
    }
    let Some(mesh) = world.resource::<RenderAssets<RenderMesh>>().get(mesh) else {
        return Ok(false);
    };
    let key = (base, mesh.layout.clone());
    if !world
        .resource::<MotionPipelines>()
        .world_variants
        .contains_key(&key)
    {
        let owners = world.resource::<super::pipeline_ownership::PipelineOwners>();
        let _queue = if owner.is_some() {
            Some(super::pipeline_ownership::queue_scope(cache).map_err(str::to_owned)?)
        } else {
            None
        };
        let mut descriptor = original.clone();
        descriptor.label = Some("source_world_authored_motion".into());
        descriptor.layout[1] = SourceMotionPass::object_camera_layout();
        descriptor.layout[3] = SourceMotionPass::object_layout();
        descriptor.multisample = MultisampleState::default();
        descriptor.vertex.buffers = vec![
            mesh.layout
                .0
                .get_layout(&[Mesh::ATTRIBUTE_POSITION.at_shader_location(0)])
                .map_err(|e| e.to_string())?,
        ];
        let Some(shader) = world
            .resource::<super::fixed_assets::AdmittedShaders>()
            .get("shaders/source_motion_objects.wgsl")
            .cloned()
        else {
            return Ok(false);
        };
        descriptor.vertex.shader = shader.clone();
        descriptor.vertex.entry_point = Some("object_vertex".into());
        let fragment = descriptor.fragment.as_mut().unwrap();
        fragment.shader = shader;
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
            defs.retain(|d| !matches!(d,ShaderDefVal::Bool(name,_) if name=="SOURCE_WORLD"));
        }
        descriptor.primitive.cull_mode = Some(Face::Back);
        descriptor.depth_stencil = Some(DepthStencilState {
            format: TextureFormat::Depth24PlusStencil8,
            depth_write_enabled: Some(false),
            depth_compare: Some(CompareFunction::GreaterEqual),
            stencil: default(),
            bias: default(),
        });
        let id = cache.queue_render_pipeline(descriptor);
        if let Some(owner) = &owner {
            owners.claim(cache, id, owner);
        }
        world
            .resource_mut::<MotionPipelines>()
            .world_variants
            .insert(key.clone(), id);
    }
    let pipelines = world.resource::<MotionPipelines>();
    let Some(id) = pipelines.world_variants.get(&key) else {
        return Ok(true);
    };
    let id = *id;
    if let Some(owner) = &owner {
        world
            .resource::<super::pipeline_ownership::PipelineOwners>()
            .claim(world.resource::<PipelineCache>(), id, owner);
    } else {
        world.resource_scope(
            |world, owners: Mut<super::pipeline_ownership::PipelineOwners>| {
                owners.reclaim_active(&mut world.resource_mut::<PipelineCache>(), id)
            },
        );
    }
    let cache = world.resource::<PipelineCache>();
    if let Some(error) = super::postprocessing::pipeline_failure(cache, &[id]) {
        Err(error)
    } else {
        Ok(cache.get_render_pipeline(id).is_some())
    }
}

fn source_flag(defs: &[ShaderDefVal], name: &str) -> bool {
    defs.iter()
        .any(|d| matches!(d,ShaderDefVal::Bool(key,true) if key==name))
}
fn prepare_pipelines(
    mut pipelines: ResMut<MotionPipelines>,
    shaders: Res<super::fixed_assets::AdmittedShaders>,
    cache: Res<PipelineCache>,
    loading: Res<MaterialLoading>,
    status: Res<SourceLightingStatus>,
    isolation: Res<super::world::pipelines::PipelineIsolation>,
    owners: Res<super::pipeline_ownership::PipelineOwners>,
) {
    let Some(shader) = shaders.get("shaders/source_motion_objects.wgsl") else {
        return;
    };
    pipelines.isolated.clone_from(&isolation.0);
    if matches!(*status.0.lock().unwrap(), RenderingState::Failed(_)) {
        return;
    }
    if pipelines.variants.capacity() == 0 {
        let Some(capacity) = loading.source_shadow_capacity() else {
            return;
        };
        pipelines
            .variants
            .reserve(capacity * super::surface_views::VIEW_COUNT);
    }
    let first = pipelines.inspected;
    for (index, pipeline) in cache.pipelines().enumerate().skip(first) {
        pipelines.inspected = index + 1;
        let base_id = CachedRenderPipelineId::new(index);
        if pipelines.variants.contains_key(&base_id)
            || owners.owner(base_id).is_some_and(|o| o.cancelled())
        {
            continue;
        }
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
            pipelines.variants.reserve(1);
        }
        let base_id = CachedRenderPipelineId::new(index);
        let _queue = if let Some(owner) = owners.owner(base_id) {
            match super::pipeline_ownership::queue_scope(&cache) {
                Ok(scope) => Some(scope),
                Err(error) => {
                    owner.fail(error.into());
                    continue;
                }
            }
        } else {
            None
        };
        let mut descriptor = base.clone();
        descriptor.label = Some("source_motion_object".into());
        descriptor.layout[0] = super::source_lighting::source_camera_layout();
        descriptor.layout[1] = SourceMotionPass::object_camera_layout();
        descriptor.layout[3] = SourceMotionPass::object_layout();
        descriptor.multisample = MultisampleState::default();
        descriptor.vertex.shader = shader.clone();
        descriptor.vertex.entry_point = Some("object_vertex".into());
        let fragment = descriptor.fragment.as_mut().unwrap();
        fragment.shader = shader.clone();
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
        owners.companion(&cache, base_id, id);
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
        SRes<RenderAssets<RenderMesh>>,
        SRes<ExtractedLighting>,
    );
    type ViewQuery = &'static MotionDrawView;
    type ItemQuery = ();
    fn render<'w>(
        item: &P,
        view: ROQueryItem<'w, '_, Self::ViewQuery>,
        _: Option<()>,
        (pipelines, cache, objects, meshes, lighting): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let objects = objects.into_inner();
        let Some(object) = objects.get(item.main_entity().id()) else {
            // Material warmups draw in the main view but have no motion model.
            if lighting.warming() {
                return RenderCommandResult::Skip;
            }
            return RenderCommandResult::Failure("Source motion object is missing");
        };
        if !object.eligible || object.frame != objects.frame {
            return RenderCommandResult::Skip;
        }
        let Some(mesh) = meshes.get(object.mesh) else {
            return RenderCommandResult::Skip;
        };
        let Some(id) = pipelines
            .world_variants
            .get(&(item.cached_pipeline(), mesh.layout.clone()))
            .or_else(|| pipelines.variants.get(&item.cached_pipeline()))
        else {
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
pub(crate) struct DrawAuthoredMotion;
impl<P: PhaseItem> RenderCommand<P> for DrawAuthoredMotion {
    type Param = (
        SRes<MotionObjects>,
        SRes<RenderAssets<RenderMesh>>,
        SRes<MeshAllocator>,
    );
    type ViewQuery = ();
    type ItemQuery = ();
    fn render<'w>(
        item: &P,
        _: (),
        _: Option<()>,
        (objects, meshes, allocator): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(object) = objects.into_inner().get(item.main_entity().id()) else {
            return RenderCommandResult::Failure("Source authored motion owner missing");
        };
        let Some(mesh) = meshes.into_inner().get(object.mesh) else {
            return RenderCommandResult::Skip;
        };
        let allocator = allocator.into_inner();
        let Some(vertex) = allocator.mesh_vertex_slice(&object.mesh) else {
            return RenderCommandResult::Skip;
        };
        pass.set_vertex_buffer(0, vertex.buffer.slice(..));
        match &mesh.buffer_info {
            RenderMeshBufferInfo::Indexed {
                index_format,
                count,
            } => {
                let Some(index) = allocator.mesh_index_slice(&object.mesh) else {
                    return RenderCommandResult::Skip;
                };
                pass.set_index_buffer(index.buffer.slice(..), *index_format);
                pass.draw_indexed(
                    index.range.start..index.range.start + *count,
                    vertex.range.start as i32,
                    0..1,
                );
            }
            RenderMeshBufferInfo::NonIndexed => pass.draw(vertex.range.clone(), 0..1),
        }
        RenderCommandResult::Success
    }
}
pub(crate) type DrawMotion = (
    SelectMotion,
    MotionMark<false>,
    SetSourceViewCamera,
    SetMeshBindGroup<2>,
    DrawAuthoredMotion,
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
    let warming = world.resource::<ExtractedLighting>().warming();
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
            let Some(object) = objects.get(main.id()) else {
                if warming {
                    slot += 1;
                    continue;
                }
                return Err(DrawError::RenderCommandFailure(
                    "Source motion batch model is missing",
                ));
            };
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
    app.init_resource::<MotionScene>().add_systems(
        PostUpdate,
        snapshot_models.after(super::shadow_scene::prepare_scene),
    );
    app.sub_app_mut(RenderApp)
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
