//! Camera-dependent Grass work uses the world publication and allocation gates.
use super::pipelines::{PipelineTicket, WorldPipelines};
use crate::{
    presentation::materials::{
        RenderingState,
        grass::{
            GrassPatch, GrassSettings, GrassView,
            domain::GrassDomain,
            expansion::GrassExpansion,
            material::{BLADE_ATTRIBUTE, GrassMaterial},
        },
    },
    reference::ReferenceError,
    world::WorldStream,
};
use bevy::{
    asset::RenderAssetUsages,
    ecs as bevy_ecs,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};
use std::sync::Arc;
struct Geometry {
    mesh: Handle<Mesh>,
    material: Handle<GrassMaterial>,
    patches: Vec<u32>,
    cpu: usize,
    gpu: usize,
    _ticket: PipelineTicket,
}
enum Pending {
    Queued,
    Expand(GrassExpansion),
    Upload {
        mesh: Handle<Mesh>,
        material: Handle<GrassMaterial>,
        view: GrassView,
        patches: Vec<u32>,
        ticket: Option<PipelineTicket>,
    },
}
struct Instance {
    continuation: Option<crate::world::renderer_pending::PipelineContinuation>,
    upload: Option<Arc<crate::presentation::fixed_assets::MeshUpload>>,
    retiring: bool,
    entity: Entity,
    parent: Entity,
    last_started: u64,
    session: u64,
    settings: GrassSettings,
    patches: Arc<[GrassPatch]>,
    authored_mesh: Handle<Mesh>,
    material: Handle<GrassMaterial>,
    shadows: bool,
    shadow_entity: Option<Entity>,
    bounds: bevy::camera::primitives::Aabb,
    latest: GrassView,
    accepted_view: Option<GrassView>,
    accepted: Option<Geometry>,
    pending: Option<Pending>,
    pending_cpu: usize,
    pending_gpu: usize,
    metadata: usize,
    ready: bool,
}
#[derive(Resource, Default)]
pub struct GrassJobs {
    instances: Vec<Instance>,
    domains: Option<Arc<[GrassDomain; 4]>>,
    revision: u64,
}
fn allocation(
    stream: &mut WorldStream,
    cpu: usize,
    gpu: usize,
    jobs: i32,
    pending: i32,
) -> Result<(), ReferenceError> {
    let mut next = stream.presentation_allocation();
    next.active_cpu += cpu;
    next.active_gpu += gpu;
    next.jobs = next
        .jobs
        .checked_add_signed(jobs as isize)
        .expect("Grass job ownership");
    next.pending = next
        .pending
        .checked_add_signed(pending as isize)
        .expect("Grass pending ownership");
    stream.set_presentation_allocation(next)
}
fn pending_allocation(
    stream: &mut WorldStream,
    cpu: usize,
    gpu: usize,
    jobs: usize,
) -> Result<(), ReferenceError> {
    let mut next = stream.presentation_allocation();
    next.active_cpu += cpu;
    next.active_gpu += gpu;
    next.jobs += jobs;
    next.staging_bytes += cpu;
    stream.set_presentation_allocation(next)
}
impl GrassJobs {
    pub fn release_all(
        &mut self,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<GrassMaterial>,
    ) {
        for instance in self.instances.drain(..) {
            if let Some(accepted) = instance.accepted {
                meshes.remove(accepted.mesh.id());
                materials.remove(accepted.material.id());
            }
            if let Some(Pending::Upload { mesh, material, .. }) = instance.pending {
                meshes.remove(mesh.id());
                materials.remove(material.id());
            }
        }
        self.domains = None;
    }
    pub(crate) fn drained(&self) -> bool {
        self.instances
            .iter()
            .all(|i| i.continuation.as_ref().is_none_or(|c| c.drained()))
    }
    pub fn settled(&self) -> bool {
        self.instances
            .iter()
            .all(|i| i.ready && i.pending.is_none())
    }

    pub(crate) fn measurement_meshes(
        &self,
    ) -> impl Iterator<Item = (bevy::asset::AssetId<Mesh>, bool)> + '_ {
        self.instances.iter().flat_map(|i| {
            i.accepted
                .iter()
                .map(|g| (g.mesh.id(), false))
                .chain(i.pending.iter().filter_map(|p| match p {
                    Pending::Upload { mesh, .. } => Some((mesh.id(), true)),
                    _ => None,
                }))
        })
    }

    pub fn release_domains(&mut self) {
        if self.instances.is_empty() {
            self.domains = None;
        }
    }

    /// Source patch identities for the admitted canonical triangle expansion.
    pub fn patch_ids(&self, entity: Entity) -> Option<&[u32]> {
        self.instances
            .iter()
            .find(|i| i.entity == entity)
            .and_then(|i| i.accepted.as_ref())
            .map(|g| g.patches.as_slice())
    }

    pub fn request_view(
        &mut self,
        _stream: &mut WorldStream,
        entity: Entity,
        view: GrassView,
    ) -> Result<(), ReferenceError> {
        if let Some(instance) = self.instances.iter_mut().find(|i| i.entity == entity) {
            instance.latest = view;
        }
        Ok(())
    }
    pub fn set_fade(&self, entity: Entity, fade: f32, materials: &mut Assets<GrassMaterial>) {
        if let Some(instance) = self.instances.iter().find(|i| i.entity == entity) {
            if let Some(accepted) = &instance.accepted {
                if let Some(mut material) = materials.get_mut(&accepted.material) {
                    if material.uniform.lod.x != fade {
                        material.uniform.lod.x = fade;
                    }
                }
            }
            if let Some(Pending::Upload { material, .. }) = &instance.pending {
                if let Some(mut material) = materials.get_mut(material) {
                    if material.uniform.lod.x != fade {
                        material.uniform.lod.x = fade;
                    }
                }
            }
        }
    }
    pub fn ready(&self, entity: Entity) -> bool {
        self.instances.iter().any(|i| i.entity == entity && i.ready)
    }
    pub fn request(
        &mut self,
        stream: &mut WorldStream,
        entity: Entity,
        parent: Entity,
        patches: Arc<[GrassPatch]>,
        authored_mesh: Handle<Mesh>,
        settings: GrassSettings,
        material: Handle<GrassMaterial>,
        view: GrassView,
        shadows: bool,
        shadow_entity: Option<Entity>,
        bounds: bevy::camera::primitives::Aabb,
    ) -> Result<(), ReferenceError> {
        let group_idle = !self
            .instances
            .iter()
            .any(|i| i.parent == parent && i.pending.is_some());
        if let Some(instance) = self.instances.iter_mut().find(|i| i.entity == entity) {
            instance.latest = view;
            if group_idle
                && instance.pending.is_none()
                && instance.accepted_view != Some(view)
                && stream.can_request_independent(1)
            {
                instance.continuation = Some(stream.reserve_grass_renderer(entity.to_bits(), 0)?);
                instance.pending = Some(Pending::Queued);
            }
            return Ok(());
        }
        if self.instances.len() >= 8192 {
            return Err(ReferenceError("Grass instance count cap".into()));
        }
        if self.domains.is_none() {
            // Fixed selected domains have 304 points and 510 triangles in total.
            let bytes = std::mem::size_of::<[GrassDomain; 4]>() + 304 * 12 + 510 * 6;
            let mut next = stream.presentation_allocation();
            next.prototype_cpu += bytes;
            stream.set_presentation_allocation(next)?;
            self.domains = Some(Arc::new([
                GrassDomain::new(2)?,
                GrassDomain::new(4)?,
                GrassDomain::new(8)?,
                GrassDomain::new(16)?,
            ]));
        }
        let metadata = std::mem::size_of::<Instance>();
        let continuation = if group_idle && stream.can_request_independent(1) {
            Some(stream.reserve_grass_renderer(entity.to_bits(), metadata)?)
        } else {
            allocation(stream, metadata, 0, 0, 0)?;
            None
        };
        let queued = continuation.is_some();
        self.instances.push(Instance {
            continuation,
            upload: None,
            retiring: false,
            entity,
            parent,
            last_started: 0,
            session: stream.frame().request.session,
            settings,
            patches,
            authored_mesh,
            material,
            shadows,
            shadow_entity,
            bounds,
            latest: view,
            accepted_view: None,
            accepted: None,
            pending: queued.then_some(Pending::Queued),
            pending_cpu: 0,
            pending_gpu: 0,
            metadata,
            ready: false,
        });
        Ok(())
    }
    pub fn prepare(
        &mut self,
        stream: &mut WorldStream,
        writers_ready: bool,
        commands: &mut Commands,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<GrassMaterial>,
        pipelines: &mut WorldPipelines,
        fixed: &crate::presentation::fixed_assets::SourceFixedAssets,
    ) -> Result<(), ReferenceError> {
        for index in 0..self.instances.len() {
            let needs_child = self.instances[index].pending.is_none()
                && self.instances[index].accepted_view != Some(self.instances[index].latest);
            let parent = self.instances[index].parent;
            let group_idle = needs_child
                && !self.instances.iter().enumerate().any(|(other, i)| {
                    i.parent == parent
                        && (i.pending.is_some()
                            || (i.accepted_view != Some(i.latest)
                                && (i.last_started, other)
                                    < (self.instances[index].last_started, index)))
                });
            let instance = &mut self.instances[index];
            if instance.retiring || instance.session != stream.frame().request.session {
                continue;
            }
            if group_idle
                && instance.pending.is_none()
                && instance.accepted_view != Some(instance.latest)
                && stream.can_request_independent(1)
            {
                instance.continuation =
                    Some(stream.reserve_grass_renderer(instance.entity.to_bits(), 0)?);
                instance.pending = Some(Pending::Queued);
            }
            if writers_ready
                && matches!(instance.pending, Some(Pending::Queued))
                && stream.available_external_jobs() > 0
            {
                self.revision = self.revision.wrapping_add(1);
                instance.last_started = self.revision;
                let mut cpu = 0;
                let job = GrassExpansion::new(
                    instance.session,
                    self.revision,
                    instance.latest,
                    instance.settings,
                    Arc::clone(&instance.patches),
                    Arc::clone(self.domains.as_ref().unwrap()),
                    |bytes, _| {
                        pending_allocation(stream, bytes, 0, 1)?;
                        cpu = bytes;
                        Ok(())
                    },
                )?;
                instance.pending_cpu = cpu;
                instance.pending = Some(Pending::Expand(job));
            }
            if let Some(Pending::Expand(job)) = &mut instance.pending {
                let mut added = (0, 0);
                let complete = job.prepare(|cpu, gpu| {
                    pending_allocation(stream, cpu * 2, gpu, 0)?;
                    added = (cpu * 2, gpu);
                    Ok(())
                })?;
                instance.pending_cpu += added.0;
                instance.pending_gpu += added.1;
                if complete {
                    pending_allocation(stream, 4096, 8, 0)?;
                    instance.pending_cpu += 4096;
                    instance.pending_gpu += 8;
                    allocation(stream, 0, 0, -1, 0)?;
                    let Some(Pending::Expand(job)) = instance.pending.take() else {
                        unreachable!()
                    };

                    let (view, output, scratch) = job.finish();
                    let mut next = stream.presentation_allocation();
                    next.active_cpu = next.active_cpu.checked_sub(scratch).unwrap();
                    next.staging_bytes = next.staging_bytes.checked_sub(scratch).unwrap();
                    stream.set_presentation_allocation(next)?;
                    instance.pending_cpu = instance.pending_cpu.checked_sub(scratch).unwrap();
                    let mut mesh = Mesh::new(
                        PrimitiveTopology::TriangleList,
                        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
                    );
                    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, output.anchors);
                    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, output.uvs);
                    mesh.insert_attribute(BLADE_ATTRIBUTE, output.blades);
                    mesh.insert_indices(Indices::U32(output.indices));
                    let mut material = materials
                        .get(&instance.material)
                        .ok_or_else(|| ReferenceError("Missing Grass material owner".into()))?
                        .clone();
                    material.uniform.camera = view.position.extend(view.screen_height);
                    let mesh = meshes.add(mesh);
                    let upload = fixed.watch_meshes(std::iter::once(mesh.clone()));
                    instance
                        .continuation
                        .as_mut()
                        .unwrap()
                        .watch_upload(upload.receipt.clone());
                    instance.upload = Some(upload);
                    instance.pending = Some(Pending::Upload {
                        mesh,
                        material: materials.add(material),
                        view,
                        patches: output.patches,
                        ticket: None,
                    });
                }
            }
            if let Some(Pending::Upload {
                mesh,
                material,
                ticket,
                ..
            }) = &mut instance.pending
            {
                if ticket.is_none() {
                    *ticket = pipelines.request_continuation(
                        stream,
                        instance.continuation.as_mut().unwrap(),
                        mesh,
                        material,
                        instance.shadows,
                    )?;
                    if let Some(ticket) = ticket {
                        ticket.set_motion_mesh(&instance.authored_mesh);
                    }
                }
                if let Some(ticket) = ticket {
                    match ticket.state() {
                        RenderingState::Failed(error) => return Err(ReferenceError(error)),
                        RenderingState::Loading => continue,
                        RenderingState::Ready => {}
                    }
                    if !instance.continuation.as_ref().unwrap().drained()
                        || !writers_ready
                        || !stream.take_presentation_publication()
                    {
                        continue;
                    }
                    let Some(Pending::Upload {
                        mesh,
                        material,
                        view,
                        patches,
                        ticket,
                    }) = instance.pending.take()
                    else {
                        unreachable!()
                    };
                    if let Some(old) = instance.accepted.take() {
                        meshes.remove(old.mesh.id());
                        materials.remove(old.material.id());
                        let mut next = stream.presentation_allocation();
                        next.active_cpu = next.active_cpu.saturating_sub(old.cpu);
                        next.active_gpu = next.active_gpu.saturating_sub(old.gpu);
                        stream.set_presentation_allocation(next)?;
                    }
                    commands.entity(instance.entity).insert((
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(material.clone()),
                        instance.bounds,
                    ));
                    if let Some(entity) = instance.shadow_entity {
                        commands.entity(entity).insert((
                            Mesh3d(mesh.clone()),
                            MeshMaterial3d(material.clone()),
                            instance.bounds,
                        ));
                    }
                    instance.accepted_view = Some(view);
                    instance.accepted = Some(Geometry {
                        mesh,
                        material,
                        patches,
                        cpu: instance.pending_cpu,
                        gpu: instance.pending_gpu,
                        _ticket: ticket.unwrap(),
                    });
                    let mut next = stream.presentation_allocation();
                    next.staging_bytes = next.staging_bytes.saturating_sub(instance.pending_cpu);
                    stream.set_presentation_allocation(next)?;
                    instance.pending_cpu = 0;
                    instance.pending_gpu = 0;
                    instance.ready = true;
                    stream.release_grass_renderer(
                        instance.entity.to_bits(),
                        instance.continuation.as_mut().unwrap(),
                    )?;
                    instance.continuation = None;
                    instance.upload = None;
                }
            }
        }
        Ok(())
    }
    pub fn retire(
        &mut self,
        stream: &mut WorldStream,
        entity: Entity,
        meshes: &mut Assets<Mesh>,
        materials: &mut Assets<GrassMaterial>,
    ) -> Result<bool, ReferenceError> {
        let Some(index) = self.instances.iter().position(|i| i.entity == entity) else {
            return Ok(true);
        };
        self.instances[index].retiring = true;
        if self.instances[index]
            .continuation
            .as_ref()
            .is_some_and(|c| !c.drained())
        {
            return Ok(false);
        }
        let mut instance = self.instances.swap_remove(index);
        let mut cpu = instance.metadata + instance.pending_cpu;
        let mut gpu = instance.pending_gpu;
        if let Some(old) = instance.accepted {
            cpu += old.cpu;
            gpu += old.gpu;
            meshes.remove(old.mesh.id());
            materials.remove(old.material.id());
        }
        if let Some(Pending::Upload { mesh, material, .. }) = &instance.pending {
            meshes.remove(mesh.id());
            materials.remove(material.id());
        }
        let mut next = stream.presentation_allocation();
        next.active_cpu = next.active_cpu.saturating_sub(cpu);
        next.staging_bytes = next.staging_bytes.saturating_sub(instance.pending_cpu);
        next.active_gpu = next.active_gpu.saturating_sub(gpu);
        next.jobs = next.jobs.saturating_sub(usize::from(matches!(
            instance.pending,
            Some(Pending::Expand(_))
        )));
        stream.set_presentation_allocation(next)?;
        if let Some(claim) = instance.continuation.as_mut() {
            stream.release_grass_renderer(instance.entity.to_bits(), claim)?;
        }
        Ok(true)
    }
}
