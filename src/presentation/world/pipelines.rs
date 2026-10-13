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
    material: UntypedAssetId,
    shadows: bool,
    state: RenderingState,
    ids: Vec<CachedRenderPipelineId>,
}
#[derive(Clone)]
pub struct PipelineTicket(Arc<Mutex<PipelineJob>>);
impl PipelineTicket {
    pub fn state(&self) -> RenderingState {
        self.0.lock().unwrap().state.clone()
    }
}
#[derive(Resource, Clone, ExtractResource, Default)]
pub struct WorldPipelines {
    jobs: Arc<Mutex<Vec<PipelineTicket>>>,
    accounted_jobs: usize,
    accounted_tickets: usize,
}
pub struct WorldPipelinePlugin;
impl Plugin for WorldPipelinePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldPipelines>()
            .add_plugins(ExtractResourcePlugin::<WorldPipelines>::default());
        app.sub_app_mut(RenderApp)
            .add_systems(Render, compile.after(RenderSystems::PrepareBindGroups));
    }
}
impl WorldPipelines {
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
        allocation.jobs += 1;
        allocation.pending += 1;
        // Two keys per view plus directional and spot shadow variants remain reserved.
        allocation.prototype_cpu +=
            std::mem::size_of::<PipelineJob>() + 32 * std::mem::size_of::<CachedRenderPipelineId>();
        stream.set_presentation_allocation(allocation)?;
        self.accounted_jobs += 1;
        self.accounted_tickets += 1;
        let ticket = PipelineTicket(Arc::new(Mutex::new(PipelineJob {
            session: stream.frame().request.session,
            mesh: mesh.id(),
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
    let tickets = input.jobs.lock().unwrap();
    for ticket in tickets.iter() {
        let mut job = ticket.0.lock().unwrap();
        if job.state != RenderingState::Loading {
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
                job.state =
                    RenderingState::Failed("Source material pipeline reservation exceeded".into());
                continue;
            }
            for (mesh_key, shadow) in keys {
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
                        if !job.ids.contains(&id) {
                            job.ids.push(id);
                        }
                    }
                    Err(error) => {
                        job.state =
                            RenderingState::Failed(format!("Source material pipeline: {error}"));
                        break;
                    }
                }
            }
        }
        if job.state != RenderingState::Loading {
            continue;
        }
        let cache = world.resource::<PipelineCache>();
        let mut ready = true;
        let mut failure = None;
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
        if let Some(error) = failure {
            job.state = RenderingState::Failed(error);
        } else if ready {
            job.state = RenderingState::Ready;
        }
    }
}
