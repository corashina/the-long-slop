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
    entity: Entity,
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
    pub(crate) fn startup_audit(&self) -> serde_json::Value {
        serde_json::json!({
            "idle": self.instances.iter().filter(|i| !i.ready && i.pending.is_none()).count(),
            "queued": self.instances.iter().filter(|i| matches!(i.pending, Some(Pending::Queued))).count(),
            "expanding": self.instances.iter().filter(|i| matches!(i.pending, Some(Pending::Expand(_)))).count(),
            "uploading": self.instances.iter().filter(|i| matches!(i.pending, Some(Pending::Upload { .. }))).count(),
        })
    }
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
        stream: &mut WorldStream,
        entity: Entity,
        view: GrassView,
    ) -> Result<(), ReferenceError> {
        if let Some(instance) = self.instances.iter_mut().find(|i| i.entity == entity) {
            instance.latest = view;
            if instance.pending.is_none()
                && instance.accepted_view != Some(view)
                && stream.pending_room(2)
            {
                allocation(stream, 0, 0, 0, 1)?;
                instance.pending = Some(Pending::Queued);
            }
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
        patches: Arc<[GrassPatch]>,
        authored_mesh: Handle<Mesh>,
        settings: GrassSettings,
        material: Handle<GrassMaterial>,
        view: GrassView,
        shadows: bool,
        shadow_entity: Option<Entity>,
        bounds: bevy::camera::primitives::Aabb,
    ) -> Result<(), ReferenceError> {
        if let Some(instance) = self.instances.iter_mut().find(|i| i.entity == entity) {
            instance.latest = view;
            if instance.pending.is_none()
                && instance.accepted_view != Some(view)
                && stream.pending_room(2)
            {
                allocation(stream, 0, 0, 0, 1)?;
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
        // A full queue leaves the instance idle. `request_view` queues it on a later frame.
        let queued = stream.pending_room(2);
        allocation(stream, metadata, 0, 0, i32::from(queued))?;
        self.instances.push(Instance {
            entity,
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
    ) -> Result<(), ReferenceError> {
        for instance in &mut self.instances {
            if instance.session != stream.frame().request.session {
                continue;
            }
            if writers_ready
                && matches!(instance.pending, Some(Pending::Queued))
                && stream.available_external_jobs() > 0
            {
                self.revision = self.revision.wrapping_add(1);
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

                    let mut mesh = Mesh::new(
                        PrimitiveTopology::TriangleList,
                        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
                    );
                    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, job.output.anchors);
                    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, job.output.uvs);
                    mesh.insert_attribute(BLADE_ATTRIBUTE, job.output.blades);
                    mesh.insert_indices(Indices::U32(job.output.indices));
                    let mut material = materials
                        .get(&instance.material)
                        .ok_or_else(|| ReferenceError("Missing Grass material owner".into()))?
                        .clone();
                    material.uniform.camera = job.view.position.extend(job.view.screen_height);
                    instance.pending = Some(Pending::Upload {
                        mesh: meshes.add(mesh),
                        material: materials.add(material),
                        view: job.view,
                        patches: job.output.patches,
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
                    *ticket = pipelines.request(stream, mesh, material, instance.shadows)?;
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
                    if !writers_ready || !stream.take_presentation_publication() {
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
                    allocation(stream, 0, 0, 0, -1)?;
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
    ) -> Result<(), ReferenceError> {
        let Some(index) = self.instances.iter().position(|i| i.entity == entity) else {
            return Ok(());
        };
        let instance = self.instances.swap_remove(index);
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
        next.pending = next
            .pending
            .saturating_sub(usize::from(instance.pending.is_some()));
        next.jobs = next.jobs.saturating_sub(usize::from(matches!(
            instance.pending,
            Some(Pending::Expand(_))
        )));
        stream.set_presentation_allocation(next)
    }
}
