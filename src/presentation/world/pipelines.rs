//! Compile admitted material variants before collision and visual publication.
use crate::{
    presentation::materials::RenderingState, reference::ReferenceError, world::WorldStream,
};
use bevy::{
    asset::{AssetId, UntypedAssetId},
    ecs as bevy_ecs,
    material::key::{ErasedMaterialPipelineKey, ErasedMeshPipelineKey},
    pbr::{
        LightKeyCache, MeshPipelineKey, PreparedMaterial, ViewKeyCache, alpha_mode_pipeline_key,
        base_specialize,
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
struct PipelineJob {
    session: u64,
    mesh: AssetId<Mesh>,
    motion_mesh: AssetId<Mesh>,
    material: UntypedAssetId,
    shadows: bool,
    state: RenderingState,
    ids: Vec<CachedRenderPipelineId>,
}
#[derive(Clone)]
pub struct PipelineTicket(Arc<Mutex<PipelineJob>>);
impl PipelineTicket {
    pub fn set_motion_mesh(&self, mesh: &Handle<Mesh>) {
        self.0.lock().unwrap().motion_mesh = mesh.id();
    }
    pub fn state(&self) -> RenderingState {
        self.0.lock().unwrap().state.clone()
    }
}
#[derive(Resource, Clone, ExtractResource, Default)]
pub struct WorldPipelines {
    jobs: Arc<Mutex<Vec<PipelineTicket>>>,
    accounted_jobs: usize,
    accounted_tickets: usize,
    cache_reserved: bool,
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
        self.jobs
            .lock()
            .unwrap()
            .iter()
            .any(|ticket| ticket.0.lock().unwrap().ids.contains(&id))
    }
    pub fn settled(&self) -> Result<bool, ReferenceError> {
        for ticket in self.jobs.lock().unwrap().iter() {
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
        if stream.available_external_jobs() == 0 {
            return Ok(None);
        }
        let mut allocation = stream.presentation_allocation();
        if !self.cache_reserved {
            allocation.prototype_cpu += super::super::pipeline_ownership::CACHE_BYTES;
        }
        allocation.jobs += 1;
        allocation.pending += 1;
        allocation.staging_bytes += super::super::pipeline_ownership::DESCRIPTOR_BYTES;
        // Two keys per view plus directional and spot shadow variants remain reserved.
        allocation.prototype_cpu +=
            std::mem::size_of::<PipelineJob>() + 32 * std::mem::size_of::<CachedRenderPipelineId>();
        stream.set_presentation_allocation(allocation)?;
        self.cache_reserved = true;
        self.accounted_jobs += 1;
        self.accounted_tickets += 1;
        let ticket = PipelineTicket(Arc::new(Mutex::new(PipelineJob {
            session: stream.frame().request.session,
            mesh: mesh.id(),
            motion_mesh: mesh.id(),
            material: material.id().untyped(),
            shadows,
            state: RenderingState::Loading,
            ids: Vec::with_capacity(32),
        })));
        self.jobs.lock().unwrap().push(ticket.clone());
        Ok(Some(ticket))
    }
    pub fn prepare(&mut self, stream: &mut WorldStream) -> Result<(), ReferenceError> {
        let session = stream.frame().request.session;
        let mut jobs = self.jobs.lock().unwrap();
        jobs.retain(|ticket| {
            let mut job = ticket.0.lock().unwrap();
            if job.session != session {
                job.state = RenderingState::Failed("Stale source material job".into());
            }
            job.state == RenderingState::Loading || Arc::strong_count(&ticket.0) > 1
        });
        let count = jobs
            .iter()
            .filter(|ticket| ticket.0.lock().unwrap().state == RenderingState::Loading)
            .count();
        let retained = jobs.len();
        drop(jobs);
        let mut allocation = stream.presentation_allocation();
        allocation.jobs = allocation.jobs.saturating_sub(self.accounted_jobs) + count;
        allocation.pending = allocation.pending.saturating_sub(self.accounted_jobs) + count;
        allocation.staging_bytes = allocation.staging_bytes.saturating_sub(
            self.accounted_jobs * super::super::pipeline_ownership::DESCRIPTOR_BYTES,
        ) + count * super::super::pipeline_ownership::DESCRIPTOR_BYTES;
        let unit =
            std::mem::size_of::<PipelineJob>() + 32 * std::mem::size_of::<CachedRenderPipelineId>();
        allocation.prototype_cpu = allocation
            .prototype_cpu
            .saturating_sub(self.accounted_tickets * unit)
            + retained * unit;
        stream.set_presentation_allocation(allocation)?;
        self.accounted_jobs = count;
        self.accounted_tickets = retained;
        Ok(())
    }
}
fn compile(world: &mut World) {
    let Some(input) = world.get_resource::<WorldPipelines>().cloned() else {
        return;
    };
    let hidden = world.resource::<CandidatePipelines>().0.clone();
    world
        .resource_mut::<PipelineIsolation>()
        .0
        .retain(|id| !input.owns(*id));
    for (bank, input) in std::iter::once(input).chain(hidden).enumerate() {
        let tickets = input.jobs.lock().unwrap();
        for ticket in tickets.iter() {
            let mut job = ticket.0.lock().unwrap();
            if job.state != RenderingState::Loading {
                continue;
            }
            if job.material.type_id()
                == std::any::TypeId::of::<crate::presentation::materials::SourceMaterial>()
                && !crate::presentation::caster_material::ready(world, job.material.typed())
            {
                continue;
            }
            if job.ids.is_empty() {
                let Some(mesh) = world.resource::<RenderAssets<RenderMesh>>().get(job.mesh) else {
                    continue;
                };
                let Some(material) = world
                    .resource::<ErasedRenderAssets<PreparedMaterial>>()
                    .get(job.material)
                else {
                    continue;
                };
                let layout = mesh.layout.clone();
                let properties = material.properties.clone();
                let mesh_bits = MeshPipelineKey::from_bits_retain(mesh.key_bits.bits());
                let view_keys = world
                    .resource::<ViewKeyCache>()
                    .values()
                    .copied()
                    .collect::<Vec<_>>();
                if view_keys.is_empty() {
                    continue;
                }
                let mut keys = Vec::with_capacity(32);
                for view_key in view_keys {
                    let bits: MeshPipelineKey = properties.mesh_pipeline_key_bits.downcast();
                    let key = view_key
                        | mesh_bits
                        | bits
                        | alpha_mode_pipeline_key(
                            properties.alpha_mode,
                            &Msaa::from_samples(view_key.msaa_samples()),
                        );
                    if !keys.contains(&(key, false)) {
                        keys.push((key, false));
                    }
                }
                if job.shadows && properties.shadows_enabled {
                    let light_keys = world.resource::<LightKeyCache>();
                    if light_keys.is_empty() {
                        continue;
                    }
                    for light_key in light_keys.values() {
                        let mut key = *light_key | mesh_bits;
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
                        if !keys.contains(&(key, true)) {
                            keys.push((key, true));
                        }
                    }
                }
                if keys.len() > 32 {
                    job.state = RenderingState::Failed(
                        "Source material pipeline reservation exceeded".into(),
                    );
                    continue;
                }
                for (mesh_key, shadow) in keys {
                    let owner = (bank != 0).then(|| {
                        world
                            .resource::<CandidatePipelines>()
                            .1
                            .as_ref()
                            .unwrap()
                            .clone()
                    });
                    if owner.as_ref().is_some_and(|o| o.cancelled()) {
                        break;
                    }
                    let _queue = if owner.is_some() {
                        match super::super::pipeline_ownership::queue_scope(
                            world.resource::<PipelineCache>(),
                        ) {
                            Ok(scope) => Some(scope),
                            Err(error) => {
                                job.state = RenderingState::Failed(error.into());
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
                            job.state = RenderingState::Failed(
                                "Source shadow material has no shader pass".into(),
                            );
                            break;
                        };
                        specialize(world, key, &layout, &properties)
                    } else {
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
                                    world.resource_mut::<crate::presentation::surface_pass::SurfacePasses>().revisit(id);
                                    world.resource_mut::<crate::presentation::motion_draw::MotionPipelines>().revisit(id);
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
                                    world.resource_mut::<crate::presentation::surface_pass::SurfacePasses>().revisit(id);
                                    world.resource_mut::<crate::presentation::motion_draw::MotionPipelines>().revisit(id);
                                }
                            }
                            if !job.ids.contains(&id) {
                                job.ids.push(id);
                            }
                        }
                        Err(error) => {
                            job.state = RenderingState::Failed(format!(
                                "Source material pipeline: {error}"
                            ));
                            break;
                        }
                    }
                }
            }
            if job.state != RenderingState::Loading {
                continue;
            }
            let published = world.resource::<PipelineCache>().pipelines().count();
            if job.ids.iter().any(|id| id.id() >= published) {
                continue;
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
            let mut motion_failure = None;
            for id in &job.ids {
                match crate::presentation::motion_draw::prepare_world_motion(
                    world,
                    *id,
                    job.motion_mesh,
                    candidate_owner.clone(),
                ) {
                    Ok(done) => ready &= done,
                    Err(error) => {
                        motion_failure = Some(error);
                        break;
                    }
                }
            }
            let cache = world.resource::<PipelineCache>();
            let mut failure = motion_failure;
            for id in &job.ids {
                match cache.get_render_pipeline_state(*id) {
                    CachedPipelineState::Ok(_) => {}
                    CachedPipelineState::Err(
                        ShaderCacheError::ShaderNotLoaded(_)
                        | ShaderCacheError::ShaderImportNotYetAvailable,
                    ) => ready = false,
                    CachedPipelineState::Err(error) => {
                        failure = Some(format!("Source material shader: {error}"));
                        break;
                    }
                    _ => ready = false,
                }
            }
            for id in &job.ids {
                if let Some(result) = world
                    .resource::<crate::presentation::caster::CasterPasses>()
                    .companion_state(*id, cache)
                {
                    match result {
                        Ok(value) => ready &= value,
                        Err(error) => {
                            failure = Some(error);
                            break;
                        }
                    }
                }
                if let Some(result) = world
                    .resource::<crate::presentation::surface_pass::SurfacePasses>()
                    .companion_state(*id, cache)
                {
                    match result {
                        Ok(value) => ready &= value,
                        Err(error) => {
                            failure = Some(error);
                            break;
                        }
                    }
                }
                let descriptor = cache.get_render_pipeline_descriptor(*id);
                if descriptor.fragment.as_ref().is_some_and(|fragment|fragment.shader_defs.iter().any(|def|matches!(def,bevy::shader::ShaderDefVal::Bool(name,true) if name=="SOURCE_SURFACE" || name=="SOURCE_WORLD"))) && !world.resource::<crate::presentation::caster::CasterPasses>().classified(*id) {ready=false;}
            }
            if let Some(error) = failure {
                job.state = RenderingState::Failed(error);
            } else if ready {
                job.state = RenderingState::Ready;
            }
        }
    }
}
