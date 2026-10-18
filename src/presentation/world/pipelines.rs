//! Compile admitted material variants before collision and visual publication.
use crate::world::renderer_pending::{
    PIPELINE_BATCH_KEYS, PIPELINE_DIAGNOSTIC_BYTES, PipelineAccounting, PipelineBufferGrowth,
    PipelineContinuation, PipelineOperation, pipeline_diagnostic, pipeline_pair_ready,
    write_pipeline_diagnostic,
};
use crate::world::upload_receipt::UploadReceipt;
use crate::{
    presentation::materials::RenderingState, reference::ReferenceError, world::WorldStream,
};
use bevy::render::renderer::RenderQueue;
use bevy::{
    asset::{AssetId, UntypedAssetId},
    ecs as bevy_ecs,
    material::key::{ErasedMaterialPipelineKey, ErasedMeshPipelineKey},
    pbr::{
        MeshPipelineKey, PreparedMaterial, ViewKeyCache, alpha_mode_pipeline_key, base_specialize,
    },
    prelude::*,
    render as bevy_render,
    render::{
        Render, RenderApp, RenderSystems,
        erased_render_asset::ErasedRenderAssets,
        extract_resource::{ExtractResource, ExtractResourcePlugin},
        mesh::RenderMesh,
        render_asset::RenderAssets,
        render_resource::*,
    },
    shader::ShaderCacheError,
};
use std::sync::{Arc, Mutex};

fn arc_bytes<T>() -> usize {
    std::alloc::Layout::new::<[usize; 2]>()
        .extend(std::alloc::Layout::new::<T>())
        .unwrap()
        .0
        .pad_to_align()
        .size()
}
struct PipelineJob {
    physical: UploadReceipt,
    session: u64,
    mesh: AssetId<Mesh>,
    motion_mesh: AssetId<Mesh>,
    material: UntypedAssetId,
    material_type: &'static str,
    shadows: bool,
    state: RenderingState,
    waiting: &'static str,
    dispatched: bool,
    spill: bool,
    ids: Vec<CachedRenderPipelineId>,
}
#[derive(Clone)]
pub struct PipelineTicket(Arc<Mutex<PipelineJob>>);
impl PipelineTicket {
    pub fn set_motion_mesh(&self, mesh: &Handle<Mesh>) {
        let mut job = self.0.lock().unwrap();
        assert!(!job.dispatched, "Motion mesh changed after specialization");
        job.motion_mesh = mesh.id();
    }
    pub fn retry(slot: &mut Option<Self>) {
        if slot.as_ref().is_some_and(|ticket| {
            let job = ticket.0.lock().unwrap();
            job.spill && job.physical.completed()
        }) {
            *slot = None;
        }
    }
    pub fn state(&self) -> RenderingState {
        let job = self.0.lock().unwrap();
        if !pipeline_pair_ready(&job.physical, job.spill) {
            RenderingState::Loading
        } else {
            job.state.clone()
        }
    }
}
type PipelineBatch = crate::world::renderer_pending::PipelineBatch<Mutex<PipelineJob>>;
#[derive(Default)]
struct PipelineBank {
    pairs: Vec<PipelineTicket>,
    batches: Vec<PipelineBatch>,
}
#[derive(Resource, Clone, ExtractResource, Default)]
pub struct WorldPipelines {
    jobs: Option<Arc<Mutex<PipelineBank>>>,
    accounting: PipelineAccounting,
    cache_reserved: bool,
    session: u64,
}
#[derive(Resource, Clone, ExtractResource, Default)]
pub struct CandidatePipelines(
    pub Option<WorldPipelines>,
    pub Option<crate::presentation::gpu_prepare::GpuPreparation>,
);
#[derive(Resource, Default)]
pub struct PipelineIsolation(pub std::collections::HashSet<CachedRenderPipelineId>);
pub struct WorldPipelinePlugin;
impl Plugin for WorldPipelinePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CandidatePipelines>()
            .add_plugins(ExtractResourcePlugin::<CandidatePipelines>::default())
            .init_resource::<WorldPipelines>()
            .add_plugins(ExtractResourcePlugin::<WorldPipelines>::default());
        app.sub_app_mut(RenderApp)
            .init_resource::<PipelineIsolation>()
            .init_resource::<super::super::pipeline_ownership::PipelineOwners>()
            .add_systems(
                Render,
                super::super::pipeline_ownership::prepare
                    .before(RenderSystems::Render)
                    .after(compile),
            )
            .add_systems(Render, compile.after(RenderSystems::PrepareBindGroups));
    }
}
impl WorldPipelines {
    pub(crate) fn startup_audit(&self) -> serde_json::Value {
        let Some(bank) = &self.jobs else {
            return serde_json::json!({"retained":0,"physical_batches":0});
        };
        let bank = bank.lock().unwrap();
        let mut requests = std::collections::HashSet::new();
        let mut compiled = std::collections::HashSet::new();
        let mut ready = 0;
        for ticket in &bank.pairs {
            let job = ticket.0.lock().unwrap();
            requests.insert((job.mesh, job.material, job.shadows));
            compiled.extend(job.ids.iter().copied());
            ready += usize::from(
                job.state == RenderingState::Ready && pipeline_pair_ready(&job.physical, job.spill),
            );
        }
        serde_json::json!({
            "retained": bank.pairs.len(), "unique_requests":requests.len(),
            "unique_compiled_pipelines":compiled.len(), "ready":ready,
            "physical_batches":bank.batches.len(),
            "incomplete_batches":bank.batches.iter().filter(|b|!b.operation.physical.completed()).count(),
            "batch_pairs":bank.batches.iter().map(|b|b.work.pairs).sum::<usize>(),
            "batch_base_keys":bank.batches.iter().map(|b|b.work.base_keys).sum::<usize>(),
            "batch_base_requests":bank.batches.iter().map(|b|b.dispatched_keys).sum::<usize>(),
            "pending": bank.pairs.iter().filter_map(|ticket| {
                let job = ticket.0.lock().unwrap();
                (job.state != RenderingState::Ready || !pipeline_pair_ready(&job.physical,job.spill)).then(||serde_json::json!({
                    "material":job.material_type,"shadows":job.shadows,"state":format!("{:?}",job.state),
                    "waiting":job.waiting,"pipelines":job.ids.len(),"retry":job.spill,"stopped":job.physical.stopped(),
                }))
            }).collect::<Vec<_>>(),
        })
    }
    pub fn reserve_cache(&mut self, stream: &mut WorldStream) -> Result<(), ReferenceError> {
        if !self.cache_reserved {
            let mut allocation = stream.presentation_allocation();
            allocation.prototype_cpu += super::super::pipeline_ownership::CACHE_BYTES;
            stream.set_presentation_allocation(allocation)?;
            self.cache_reserved = true;
        }
        Ok(())
    }
    pub fn owns(&self, id: CachedRenderPipelineId) -> bool {
        self.jobs.as_ref().is_some_and(|bank| {
            bank.lock()
                .unwrap()
                .pairs
                .iter()
                .any(|ticket| ticket.0.lock().unwrap().ids.contains(&id))
        })
    }
    pub fn settled(&self) -> Result<bool, ReferenceError> {
        let Some(bank) = &self.jobs else {
            return Ok(true);
        };
        for ticket in &bank.lock().unwrap().pairs {
            match ticket.state() {
                RenderingState::Loading => return Ok(false),
                RenderingState::Failed(e) => return Err(ReferenceError(e)),
                RenderingState::Ready => {}
            }
        }
        Ok(true)
    }

    pub fn request<M: Material>(
        &mut self,
        stream: &mut WorldStream,
        mesh: &Handle<Mesh>,
        material: &Handle<M>,
        shadows: bool,
    ) -> Result<Option<PipelineTicket>, ReferenceError> {
        self.request_owned(stream, None, mesh, material, shadows)
    }
    pub(crate) fn request_continuation<M: Material>(
        &mut self,
        stream: &mut WorldStream,
        parent: &mut PipelineContinuation,
        mesh: &Handle<Mesh>,
        material: &Handle<M>,
        shadows: bool,
    ) -> Result<Option<PipelineTicket>, ReferenceError> {
        self.request_owned(stream, Some(parent), mesh, material, shadows)
    }
    fn request_owned<M: Material>(
        &mut self,
        stream: &mut WorldStream,
        mut parent: Option<&mut PipelineContinuation>,
        mesh: &Handle<Mesh>,
        material: &Handle<M>,
        shadows: bool,
    ) -> Result<Option<PipelineTicket>, ReferenceError> {
        if let Some(parent) = parent.as_ref() {
            parent.validate(stream)?;
        }
        // This local Arc adds no allocation. It keeps the bank lock order bank -> pair.
        let mut shared = self.jobs.clone();
        let mut guard = shared.as_ref().map(|bank| bank.lock().unwrap());
        let append = guard.as_ref().and_then(|bank| {
            parent.as_ref().and_then(|parent| {
                bank.batches.iter().position(|batch| {
                    batch.open
                        && batch.len < PIPELINE_BATCH_KEYS
                        && parent.shares(&batch.operation.physical)
                })
            })
        });
        if append.is_none() && parent.as_ref().is_some_and(|parent| !parent.drained()) {
            return Ok(None);
        }
        let (pair_len, pair_cap, batch_len, batch_cap) =
            guard.as_ref().map_or((0, 0, 0, 0), |bank| {
                (
                    bank.pairs.len(),
                    bank.pairs.capacity(),
                    bank.batches.len(),
                    bank.batches.capacity(),
                )
            });
        let pair_growth_plan =
            PipelineBufferGrowth::new(pair_cap, pair_len + 1, size_of::<PipelineTicket>());
        let batch_growth_plan = PipelineBufferGrowth::new(
            batch_cap,
            batch_len + usize::from(append.is_none()),
            size_of::<PipelineBatch>(),
        );
        let pair_capacity = pair_growth_plan.capacity;
        let batch_capacity = batch_growth_plan.capacity;
        let pair_growth = pair_growth_plan.admitted;
        let batch_growth = batch_growth_plan.admitted;
        let cpu = ticket_bytes()
            + pair_growth
            + batch_growth
            + if shared.is_none() {
                arc_bytes::<Mutex<PipelineBank>>()
            } else {
                0
            }
            + if self.cache_reserved {
                0
            } else {
                super::super::pipeline_ownership::CACHE_BYTES
            }
            + if append.is_none() {
                UploadReceipt::allocation_bytes()
            } else {
                0
            };
        let operation = if let Some(index) = append {
            let operation = &guard.as_ref().unwrap().batches[index].operation;
            operation.reserve_pair(stream, cpu)?;
            operation.clone()
        } else {
            let operation = match parent.as_mut() {
                Some(parent) => parent.start(
                    stream,
                    cpu,
                    super::super::pipeline_ownership::DESCRIPTOR_BYTES,
                )?,
                None => PipelineOperation::independent(
                    stream,
                    cpu,
                    super::super::pipeline_ownership::DESCRIPTOR_BYTES,
                )?,
            };
            let Some(operation) = operation else {
                return Ok(None);
            };
            self.accounting.admitted(&operation);
            operation
        };
        if shared.is_none() {
            drop(guard);
            shared = Some(Arc::new(Mutex::new(PipelineBank::default())));
            self.jobs = shared.clone();
            guard = Some(shared.as_ref().unwrap().lock().unwrap());
        }
        let bank = guard.as_mut().unwrap();
        let mut released = 0;
        if pair_growth != 0 {
            grow(&mut bank.pairs, pair_capacity);
            released += pair_growth_plan.released;
        }
        if batch_growth != 0 {
            grow(&mut bank.batches, batch_capacity);
            released += batch_growth_plan.released;
        }
        let ids = Vec::with_capacity(PIPELINE_BATCH_KEYS);
        assert_eq!(ids.capacity(), PIPELINE_BATCH_KEYS);
        let ticket = PipelineTicket(Arc::new(Mutex::new(PipelineJob {
            physical: operation.physical.clone(),
            session: stream.frame().request.session,
            mesh: mesh.id(),
            motion_mesh: mesh.id(),
            material: material.id().untyped(),
            material_type: std::any::type_name::<M>(),
            shadows,
            state: RenderingState::Loading,
            waiting: "queued",
            dispatched: false,
            spill: false,
            ids,
        })));
        let index = if let Some(index) = append {
            index
        } else {
            bank.batches.push(PipelineBatch::new(operation));
            bank.batches.len() - 1
        };
        let batch = &mut bank.batches[index];
        batch.push(&ticket.0);
        bank.pairs.push(ticket.clone());
        self.accounting.pair_admitted(ticket_bytes());
        self.session = stream.frame().request.session;
        self.cache_reserved = true;
        if released != 0 {
            let mut allocation = stream.presentation_allocation();
            allocation.prototype_cpu -= released;
            stream.set_presentation_allocation(allocation)?;
        }
        Ok(Some(ticket))
    }
    pub(crate) fn drained(&self) -> bool {
        self.jobs.as_ref().is_none_or(|bank| {
            bank.lock()
                .unwrap()
                .batches
                .iter()
                .all(|batch| batch.operation.physical.completed())
        })
    }
    pub fn prepare(&mut self, stream: &mut WorldStream) -> Result<(), ReferenceError> {
        self.session = stream.frame().request.session;
        let Some(bank) = &self.jobs else {
            return Ok(());
        };
        let mut bank = bank.lock().unwrap();
        for ticket in &bank.pairs {
            let mut job = ticket.0.lock().unwrap();
            if job.session != self.session {
                job.physical.stop();
                job.waiting = "stale session";
            }
        }
        // Drop batch Weak references before dropping the admitted pair control blocks.
        for batch in &mut bank.batches {
            batch.retire_pairs();
        }
        bank.batches.retain(PipelineBatch::retain);
        bank.pairs.retain(|ticket| {
            let job = ticket.0.lock().unwrap();
            !job.physical.completed()
                || Arc::strong_count(&ticket.0) > 1
                || Arc::weak_count(&ticket.0) != 0
        });
        let cpu = bank.pairs.len() * ticket_bytes()
            + bank.batches.len() * UploadReceipt::allocation_bytes();
        self.accounting.reconcile(
            stream,
            bank.batches.iter().map(|batch| batch.operation.clone()),
            cpu,
            super::super::pipeline_ownership::DESCRIPTOR_BYTES,
        )
    }
}
fn grow<T>(buffer: &mut Vec<T>, capacity: usize) {
    let mut next = Vec::with_capacity(capacity);
    assert_eq!(next.capacity(), capacity);
    next.append(buffer);
    *buffer = next;
}
fn ticket_bytes() -> usize {
    arc_bytes::<Mutex<PipelineJob>>()
        + PIPELINE_BATCH_KEYS * size_of::<CachedRenderPipelineId>()
        + PIPELINE_DIAGNOSTIC_BYTES
}

// The keys live only on the render stack, then freeze into the dispatched IDs.
type PairKeys = crate::world::renderer_pending::PipelineKeys<(MeshPipelineKey, bool)>;
fn pair_keys(world: &World, job: &mut PipelineJob) -> Result<Option<PairKeys>, ReferenceError> {
    if job.material.type_id()
        == std::any::TypeId::of::<crate::presentation::materials::SourceMaterial>()
        && !crate::presentation::caster_material::ready(world, job.material.typed())
    {
        job.waiting = "caster material";
        return Ok(None);
    }
    let Some(mesh) = world.resource::<RenderAssets<RenderMesh>>().get(job.mesh) else {
        job.waiting = "mesh";
        return Ok(None);
    };
    let Some(material) = world
        .resource::<ErasedRenderAssets<PreparedMaterial>>()
        .get(job.material)
    else {
        job.waiting = "material";
        return Ok(None);
    };
    let views = world.resource::<ViewKeyCache>();
    if views.values().next().is_none() {
        job.waiting = "view keys";
        return Ok(None);
    }
    let properties = &material.properties;
    let mesh_bits = MeshPipelineKey::from_bits_retain(mesh.key_bits.bits());
    let mut keys = PairKeys::new();
    for view_key in views.values().copied() {
        let bits: MeshPipelineKey = properties.mesh_pipeline_key_bits.downcast();
        keys.push((
            view_key
                | mesh_bits
                | bits
                | alpha_mode_pipeline_key(
                    properties.alpha_mode,
                    &Msaa::from_samples(view_key.msaa_samples()),
                ),
            false,
        ))?;
    }
    if job.shadows && properties.shadows_enabled {
        for light_key in [
            MeshPipelineKey::DEPTH_PREPASS
                | MeshPipelineKey::VIEW_PROJECTION_ORTHOGRAPHIC
                | MeshPipelineKey::UNCLIPPED_DEPTH_ORTHO,
            MeshPipelineKey::DEPTH_PREPASS | MeshPipelineKey::VIEW_PROJECTION_PERSPECTIVE,
        ] {
            let mut key = light_key | mesh_bits;
            if properties.alpha_mode != AlphaMode::Opaque {
                key |= MeshPipelineKey::MAY_DISCARD;
            }
            if properties
                .get_shader(bevy::pbr::PrepassVertexShader)
                .is_some()
                || properties
                    .get_shader(bevy::pbr::PrepassFragmentShader)
                    .is_some()
            {
                key |= MeshPipelineKey::PREPASS_READS_MATERIAL;
            }
            keys.push((key, true))?;
        }
    }
    Ok(Some(keys))
}
fn compile(world: &mut World) {
    let Some(active) = world.get_resource::<WorldPipelines>().cloned() else {
        return;
    };
    let candidate = world.resource::<CandidatePipelines>().0.clone();
    world
        .resource_mut::<PipelineIsolation>()
        .0
        .retain(|id| !active.owns(*id));
    for (bank, input) in std::iter::once(active).chain(candidate).enumerate() {
        let Some(shared) = &input.jobs else {
            continue;
        };
        let mut shared = shared.lock().unwrap();
        let owner = (bank != 0).then(|| {
            world
                .resource::<CandidatePipelines>()
                .1
                .as_ref()
                .unwrap()
                .clone()
        });
        let candidate_cancelled = owner.as_ref().is_some_and(|owner| owner.cancelled());
        for batch in &mut shared.batches {
            if candidate_cancelled {
                batch.operation.physical.stop();
            }
            if batch.operation.physical.completed() {
                continue;
            }
            batch.open = false;
            let cancelled = candidate_cancelled || batch.operation.physical.stopped();
            if cancelled {
                batch.stopped = true;
            }
            for pair in &batch.pairs[..batch.next] {
                let pair = pair.upgrade().expect("admitted pair control");
                let mut job = pair.lock().unwrap();
                if cancelled {
                    job.waiting = "cancelled";
                }
                if !cancelled && job.state == RenderingState::Loading {
                    compile_pair(world, &mut job, bank, None);
                }
                if matches!(job.state, RenderingState::Failed(_)) {
                    batch.stopped = true;
                }
            }
            while batch.next < batch.len && !batch.stopped {
                let pair = batch.pairs[batch.next]
                    .upgrade()
                    .expect("admitted pair control");
                let mut job = pair.lock().unwrap();
                if matches!(job.state, RenderingState::Failed(_)) {
                    batch.stopped = true;
                    break;
                }
                match pair_keys(world, &mut job) {
                    Ok(Some(keys)) => match batch.select(Some(keys.len)) {
                        Ok(true) => {
                            job.dispatched = true;
                            batch.dispatched_keys +=
                                compile_pair(world, &mut job, bank, Some(&keys));
                            if job.physical.stopped()
                                || matches!(job.state, RenderingState::Failed(_))
                            {
                                batch.stopped = true;
                            }
                        }
                        Ok(false) => {
                            batch.stopped = true;
                        }
                        Err(error) => {
                            job.state = RenderingState::Failed(pipeline_diagnostic(format_args!(
                                "{}",
                                error.0
                            )));
                            batch.stopped = true;
                        }
                    },
                    Ok(None) => {
                        batch
                            .select(None)
                            .expect("waiting has no specialization error");
                        break;
                    }
                    Err(error) => {
                        job.state = RenderingState::Failed(pipeline_diagnostic(format_args!(
                            "{}",
                            error.0
                        )));
                        batch.stopped = true;
                    }
                }
            }
            if batch.stopped {
                for pair in &batch.pairs[batch.next..batch.len] {
                    let pair = pair.upgrade().expect("admitted pair control");
                    let mut job = pair.lock().unwrap();
                    if !matches!(job.state, RenderingState::Failed(_)) {
                        job.spill = true;
                        job.waiting = "batch retry";
                    }
                }
            }
            let cancelled = cancelled || batch.operation.physical.stopped();
            let settled = batch.pairs[..batch.next].iter().map(|pair| {
                let pair = pair.upgrade().expect("admitted pair control");
                let job = pair.lock().unwrap();
                (cancelled || job.state != RenderingState::Loading)
                    && physical_pipelines_settled(world, &job)
            });
            let candidate_drained = owner
                .as_ref()
                .is_none_or(|owner| !owner.cancelled() || owner.drained());
            if batch.register(settled.chain(std::iter::once(candidate_drained))) {
                let receipt = batch.operation.physical.clone();
                world
                    .resource::<RenderQueue>()
                    .on_submitted_work_done(move || receipt.complete());
            }
        }
    }
}
fn compile_pair(
    world: &mut World,
    job: &mut PipelineJob,
    bank: usize,
    keys: Option<&PairKeys>,
) -> usize {
    let mut dispatched = 0;
    if let Some(keys) = keys {
        let layout = world
            .resource::<RenderAssets<RenderMesh>>()
            .get(job.mesh)
            .unwrap()
            .layout
            .clone();
        let properties = world
            .resource::<ErasedRenderAssets<PreparedMaterial>>()
            .get(job.material)
            .unwrap()
            .properties
            .clone();
        for (mesh_key, shadow) in keys.iter() {
            let owner = (bank != 0).then(|| {
                world
                    .resource::<CandidatePipelines>()
                    .1
                    .as_ref()
                    .unwrap()
                    .clone()
            });
            if owner.as_ref().is_some_and(|o| o.cancelled()) {
                job.physical.stop();
                job.waiting = "cancelled";
                return dispatched;
            }
            let _queue = if owner.is_some() {
                match super::super::pipeline_ownership::queue_scope(
                    world.resource::<PipelineCache>(),
                ) {
                    Ok(scope) => Some(scope),
                    Err(error) => {
                        job.state =
                            RenderingState::Failed(pipeline_diagnostic(format_args!("{error}")));
                        break;
                    }
                }
            } else {
                None
            };
            let key = ErasedMaterialPipelineKey {
                type_id: job.material.type_id(),
                mesh_key: ErasedMeshPipelineKey::new(mesh_key),
                material_key: properties.material_key.clone(),
            };
            let result = if shadow {
                let Some(specialize) = properties.prepass_specialize else {
                    job.state = RenderingState::Failed(pipeline_diagnostic(format_args!(
                        "Source shadow material has no shader pass"
                    )));
                    break;
                };
                dispatched += 1;
                specialize(world, key, &layout, &properties)
            } else {
                dispatched += 1;
                base_specialize(world, key, &layout, &properties)
            };
            match result {
                Ok(id) => {
                    if let Some(owner) = &owner {
                        let claimed = world
                            .resource::<super::super::pipeline_ownership::PipelineOwners>()
                            .claim(world.resource::<PipelineCache>(), id, owner);
                        if claimed {
                            world
                                .resource_mut::<crate::presentation::caster::CasterPasses>()
                                .revisit(id);
                            world
                                .resource_mut::<crate::presentation::surface_pass::SurfacePasses>()
                                .revisit(id);
                            world
                                .resource_mut::<crate::presentation::motion_draw::MotionPipelines>()
                                .revisit(id);
                        }
                        if world
                            .resource::<super::super::pipeline_ownership::PipelineOwners>()
                            .isolated(id)
                        {
                            world.resource_mut::<PipelineIsolation>().0.insert(id);
                        }
                    } else {
                        let reclaimed = world.resource_scope(
                                    |world,
                                     owners: Mut<
                                        super::super::pipeline_ownership::PipelineOwners,
                                    >| {
                                        owners.reclaim_active(
                                            &mut world.resource_mut::<PipelineCache>(),
                                            id,
                                        )
                                    },
                                );
                        if reclaimed {
                            world
                                .resource_mut::<crate::presentation::caster::CasterPasses>()
                                .revisit(id);
                            world
                                .resource_mut::<crate::presentation::surface_pass::SurfacePasses>()
                                .revisit(id);
                            world
                                .resource_mut::<crate::presentation::motion_draw::MotionPipelines>()
                                .revisit(id);
                        }
                    }
                    if !job.ids.contains(&id) {
                        job.ids.push(id);
                    }
                }
                Err(error) => {
                    job.state = RenderingState::Failed(pipeline_diagnostic(format_args!(
                        "Source material pipeline: {error}"
                    )));
                    break;
                }
            }
        }
    }
    if job.state != RenderingState::Loading {
        return dispatched;
    }
    let published = world.resource::<PipelineCache>().pipelines().count();
    if job.ids.iter().any(|id| id.id() >= published) {
        job.waiting = "pipeline publication";
        return dispatched;
    }
    let candidate_owner = (bank != 0).then(|| {
        world
            .resource::<CandidatePipelines>()
            .1
            .as_ref()
            .unwrap()
            .clone()
    });
    let mut ready = true;
    let mut waiting = "motion pipeline";
    let mut failure = None;
    for id in &job.ids {
        match crate::presentation::motion_draw::prepare_world_motion(
            world,
            *id,
            job.motion_mesh,
            candidate_owner.clone(),
        ) {
            Ok(done) => ready &= done,
            Err(error) => {
                write_pipeline_diagnostic(&mut failure, format_args!("{error}"));
                break;
            }
        }
    }
    let cache = world.resource::<PipelineCache>();
    for id in &job.ids {
        match cache.get_render_pipeline_state(*id) {
            CachedPipelineState::Ok(_) => {}
            CachedPipelineState::Err(
                ShaderCacheError::ShaderNotLoaded(_)
                | ShaderCacheError::ShaderImportNotYetAvailable,
            ) => ready = false,
            CachedPipelineState::Err(error) => {
                write_pipeline_diagnostic(
                    &mut failure,
                    format_args!("Source material shader: {error}"),
                );
                break;
            }
            _ => {
                waiting = "shader";
                ready = false;
            }
        }
    }
    for id in &job.ids {
        if let Some(result) = world
            .resource::<crate::presentation::caster::CasterPasses>()
            .companion_state(*id, cache)
        {
            match result {
                Ok(value) => {
                    if !value {
                        waiting = "caster pipeline";
                    }
                    ready &= value;
                }
                Err(error) => {
                    write_pipeline_diagnostic(&mut failure, format_args!("{error}"));
                    break;
                }
            }
        }
        if let Some(result) = world
            .resource::<crate::presentation::surface_pass::SurfacePasses>()
            .companion_state(*id, cache)
        {
            match result {
                Ok(value) => {
                    if !value {
                        waiting = "surface pipeline";
                    }
                    ready &= value;
                }
                Err(error) => {
                    write_pipeline_diagnostic(&mut failure, format_args!("{error}"));
                    break;
                }
            }
        }
        let descriptor = cache.get_render_pipeline_descriptor(*id);
        if descriptor.fragment.as_ref().is_some_and(|fragment|fragment.shader_defs.iter().any(|def|matches!(def,bevy::shader::ShaderDefVal::Bool(name,true) if name=="SOURCE_SURFACE" || name=="SOURCE_WORLD"))) && !world.resource::<crate::presentation::caster::CasterPasses>().classified(*id) {waiting="caster classification";ready=false;}
    }
    job.waiting = waiting;
    if let Some(error) = failure {
        job.state = RenderingState::Failed(error);
    } else if ready {
        job.state = RenderingState::Ready;
    }
    dispatched
}

// A failure flag does not release descriptor staging or the counted owner.
fn physical_pipelines_settled(world: &World, job: &PipelineJob) -> bool {
    let cache = world.resource::<PipelineCache>();
    let published = cache.pipelines().count();
    job.ids.iter().all(|id| {
        id.id() < published
            && match cache.get_render_pipeline_state(*id) {
                CachedPipelineState::Err(
                    ShaderCacheError::ShaderNotLoaded(_)
                    | ShaderCacheError::ShaderImportNotYetAvailable,
                ) => false,
                CachedPipelineState::Ok(_) | CachedPipelineState::Err(_) => true,
                _ => false,
            }
            && world
                .resource::<crate::presentation::motion_draw::MotionPipelines>()
                .physically_settled(*id, cache)
            && world
                .resource::<crate::presentation::caster::CasterPasses>()
                .physically_settled(*id, cache)
            && world
                .resource::<crate::presentation::surface_pass::SurfacePasses>()
                .companion_state(*id, cache)
                .is_none_or(|r| !matches!(r, Ok(false)))
    })
}
pub(crate) fn physical_pipeline_state(
    state: &CachedPipelineState,
) -> crate::world::renderer_pending::PhysicalPipelineState {
    use crate::world::renderer_pending::PhysicalPipelineState;
    match state {
        CachedPipelineState::Queued => PhysicalPipelineState::Queued,
        CachedPipelineState::Creating(_) => PhysicalPipelineState::Compiling,
        CachedPipelineState::Ok(_) => PhysicalPipelineState::Ready,
        CachedPipelineState::Err(
            ShaderCacheError::ShaderNotLoaded(_) | ShaderCacheError::ShaderImportNotYetAvailable,
        ) => PhysicalPipelineState::Retryable,
        CachedPipelineState::Err(_) => PhysicalPipelineState::Failed,
    }
}
