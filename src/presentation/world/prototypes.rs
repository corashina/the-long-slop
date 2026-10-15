//! Selected prototype ownership. Reservations precede every accessor or scene allocation.
use super::{
    preflight::{SourcePlan, SourcePlanLoader, load_plan},
    prototype_loader::{SourceGltf, SourceGltfLoader, SourceGltfSettings},
};
use crate::{
    reference::{ReferenceError, ReferenceManifest, SourceId, prototype_plan::PrototypePlan},
    world::WorldStream,
};
use bevy::{
    asset::LoadState, ecs as bevy_ecs, mesh::skinning::SkinnedMeshInverseBindposes, prelude::*,
};
use std::sync::Arc;

pub struct LoadedPrototype {
    pub plan: PrototypePlan,
    pub asset: Handle<SourceGltf>,
}
enum State {
    Queued,
    PlanLoading(Handle<SourcePlan>),
    Loading {
        plan: PrototypePlan,
        asset: Handle<SourceGltf>,
    },
    Ready(Arc<LoadedPrototype>),
}
struct Entry {
    source: SourceId,
    session: u64,
    request: u64,
    retired: bool,
    cpu: usize,
    gpu: usize,
    staging: usize,
    state: State,
}
#[derive(Resource, Default)]
pub struct SourcePrototypes {
    entries: Vec<Entry>,
    next_request: u64,
    accounted_session: u64,
    stale_cpu: usize,
    stale_gpu: usize,
    retired_sources: Vec<SourceId>,
    accounted_jobs: usize,
    accounted_pending: usize,
    accounted_staging: usize,
}
pub struct SourcePrototypePlugin;
impl Plugin for SourcePrototypePlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<SourceGltf>()
            .init_asset_loader::<SourceGltfLoader>()
            .init_asset::<SourcePlan>()
            .init_asset_loader::<SourcePlanLoader>()
            .init_resource::<SourcePrototypes>();
    }
}
impl SourcePrototypes {
    pub(crate) fn owns_mesh(
        &self,
        id: bevy::asset::AssetId<Mesh>,
        assets: &Assets<SourceGltf>,
    ) -> bool {
        self.entries.iter().any(|entry| {
            let handle = match &entry.state {
                State::Loading { asset, .. } => asset,
                State::Ready(loaded) if Arc::strong_count(loaded) == 1 => &loaded.asset,
                _ => return false,
            };
            assets
                .get(handle)
                .is_some_and(|asset| asset.meshes.iter().any(|mesh| mesh.id() == id))
        })
    }
    pub fn cancel(&mut self) {
        for e in &mut self.entries {
            e.retired = true;
        }
    }
    pub fn drain_ready(
        &mut self,
        server: &AssetServer,
        assets: &Assets<SourceGltf>,
        plans: &Assets<SourcePlan>,
    ) -> bool {
        for e in &mut self.entries {
            match &mut e.state {
                State::PlanLoading(job) => {
                    if !plans.contains(job.id())
                        && !matches!(server.load_state(job.id()), LoadState::Failed(_))
                    {
                        return false;
                    }
                }
                State::Loading { asset, .. } => {
                    if !assets.contains(asset.id())
                        && !matches!(server.load_state(asset.id()), LoadState::Failed(_))
                    {
                        return false;
                    }
                }
                _ => {}
            }
        }
        true
    }
    pub fn release_all(
        self,
        assets: &mut Assets<SourceGltf>,
        plans: &mut Assets<SourcePlan>,
        meshes: &mut Assets<Mesh>,
        skins: &mut Assets<SkinnedMeshInverseBindposes>,
    ) {
        for e in self.entries {
            if let State::PlanLoading(handle) = &e.state {
                plans.remove(handle.id());
            }
            let asset = match e.state {
                State::Ready(loaded) if Arc::strong_count(&loaded) == 1 => {
                    Some(loaded.asset.clone())
                }
                State::Loading { asset, .. } => Some(asset),
                _ => None,
            };
            if let Some(asset) = asset.and_then(|h| assets.remove(h.id())) {
                for m in asset.meshes {
                    meshes.remove(m.id());
                }
                for s in asset.inverse_bindposes {
                    skins.remove(s.id());
                }
            }
        }
    }

    pub fn settled(&self) -> bool {
        self.entries
            .iter()
            .all(|e| matches!(e.state, State::Ready(_)))
    }
    pub fn fork_shared_bytes(&self) -> (usize, usize) {
        self.entries
            .iter()
            .fold((0, 0), |n, e| (n.0 + e.cpu, n.1 + e.gpu))
    }
    pub fn shared_bytes(&self) -> (usize, usize) {
        self.entries
            .iter()
            .filter(|e| matches!(&e.state, State::Ready(loaded) if Arc::strong_count(loaded)>1))
            .fold((0, 0), |n, e| (n.0 + e.cpu, n.1 + e.gpu))
    }
    pub fn fork(&self, stream: &mut WorldStream) -> Result<Self, ReferenceError> {
        if !self.settled() {
            return Err(ReferenceError("Source prototype loads are pending".into()));
        }
        let mut allocation = stream.presentation_allocation();
        allocation.prototype_cpu += self.entries.len() * std::mem::size_of::<Entry>()
            + self
                .entries
                .iter()
                .map(|e| e.source.file.len())
                .sum::<usize>();
        stream.set_presentation_allocation(allocation)?;
        let mut copy = Self {
            entries: Vec::with_capacity(self.entries.len()),
            ..default()
        };
        let session = stream.frame().request.session;
        for e in &self.entries {
            let State::Ready(loaded) = &e.state else {
                unreachable!()
            };
            stream.reserve_prototype(e.source.clone(), e.cpu, e.gpu)?;
            stream.bind_prototype(&e.source)?;
            copy.entries.push(Entry {
                source: e.source.clone(),
                session,
                request: e.request,
                retired: false,
                cpu: e.cpu,
                gpu: e.gpu,
                staging: 0,
                state: State::Ready(Arc::clone(loaded)),
            });
        }
        Ok(copy)
    }

    pub fn request(
        &mut self,
        stream: &mut WorldStream,
        source: &SourceId,
    ) -> Result<(), ReferenceError> {
        let session = stream.frame().request.session;
        if self.entries.iter().any(|e| e.source == *source) {
            return Ok(());
        }
        if self.entries.len() >= 8192 {
            return Err(ReferenceError("Source prototype owner limit".into()));
        }
        let mut allocation = stream.presentation_allocation();
        allocation.pending += 1;
        stream.set_presentation_allocation(allocation)?;
        self.accounted_pending += 1;
        self.next_request = self.next_request.wrapping_add(1);
        self.entries.push(Entry {
            source: source.clone(),
            session,
            request: self.next_request,
            retired: false,
            cpu: 0,
            gpu: 0,
            staging: 0,
            state: State::Queued,
        });
        Ok(())
    }
    pub fn loaded(&self, session: u64, source: &SourceId) -> Option<&LoadedPrototype> {
        self.entries.iter().find_map(|entry| {
            if entry.session == session && entry.source == *source && !entry.retired {
                if let State::Ready(loaded) = &entry.state {
                    Some(loaded.as_ref())
                } else {
                    None
                }
            } else {
                None
            }
        })
    }
    /// Retirement keeps loader reservations until cancellation or loading has completed.
    pub fn retire(&mut self, source: &SourceId) {
        for entry in &mut self.entries {
            if entry.source == *source {
                entry.retired = true;
            }
        }
    }
    pub fn take_retired(&mut self) -> Option<SourceId> {
        self.retired_sources.pop()
    }
    pub fn references_material(&self, reference: &ReferenceManifest, source: &SourceId) -> bool {
        self.entries.iter().filter(|e| !e.retired).any(|e| {
            reference
                .world
                .prototypes
                .iter()
                .find(|p| p.source == e.source)
                .is_some_and(|p| p.deferred_materials.iter().any(|m| m.source == *source))
        })
    }
    pub fn references_asset(
        &self,
        reference: &ReferenceManifest,
        asset: crate::reference::AssetId,
    ) -> bool {
        self.entries.iter().filter(|e| !e.retired).any(|e| {
            reference
                .world
                .prototypes
                .iter()
                .find(|p| p.source == e.source)
                .is_some_and(|p| {
                    p.asset == asset
                        || p.nodes.iter().any(|n| {
                            super::preflight::initial_material(reference, &n.source)
                                .is_some_and(|m| m.0 == asset)
                        })
                })
        })
    }
    pub fn requested(&self, source: &SourceId) -> bool {
        self.entries
            .iter()
            .any(|e| !e.retired && e.source == *source)
    }
    pub fn retire_unused(&mut self, stream: &WorldStream, reference: &ReferenceManifest) {
        let frame = stream.frame();
        for entry in &mut self.entries {
            if matches!(&entry.state, State::Ready(loaded) if Arc::strong_count(loaded)>1)
                || reference.world.road.prototypes.contains(&entry.source)
            {
                continue;
            }
            if stream
                .pending_structures()
                .iter()
                .any(|p| p.prototype == entry.source)
                || frame
                    .structures
                    .iter()
                    .any(|s| s.placement.prototype == entry.source)
            {
                continue;
            }
            entry.retired = true;
        }
    }
    fn account(&mut self, stream: &mut WorldStream) -> Result<(), ReferenceError> {
        let session = stream.frame().request.session;
        if self.accounted_session != session {
            self.accounted_jobs = 0;
            self.accounted_pending = 0;
            self.accounted_staging = 0;
            self.stale_cpu = 0;
            self.stale_gpu = 0;
            self.accounted_session = session;
        }
        let stale_cpu = self
            .entries
            .iter()
            .filter(|e| e.session != session)
            .map(|e| e.cpu)
            .sum::<usize>();
        let stale_gpu = self
            .entries
            .iter()
            .filter(|e| e.session != session)
            .map(|e| e.gpu)
            .sum::<usize>();
        let jobs = self
            .entries
            .iter()
            .filter(|entry| matches!(entry.state, State::PlanLoading(_) | State::Loading { .. }))
            .count();
        let pending = self
            .entries
            .iter()
            .filter(|entry| !matches!(entry.state, State::Ready(_)))
            .count();
        let staging = self
            .entries
            .iter()
            .map(|entry| match &entry.state {
                State::PlanLoading(_) | State::Loading { .. } => entry.staging,
                _ => 0,
            })
            .sum::<usize>();
        let mut allocation = stream.presentation_allocation();
        allocation.prototype_cpu =
            allocation.prototype_cpu.saturating_sub(self.stale_cpu) + stale_cpu;
        allocation.prototype_gpu =
            allocation.prototype_gpu.saturating_sub(self.stale_gpu) + stale_gpu;
        allocation.staging_bytes = allocation
            .staging_bytes
            .saturating_sub(self.accounted_staging)
            + staging;
        allocation.dependency_staging_bytes = allocation
            .dependency_staging_bytes
            .saturating_sub(self.accounted_staging)
            + staging;
        allocation.jobs = allocation.jobs.saturating_sub(self.accounted_jobs) + jobs;
        allocation.pending = allocation.pending.saturating_sub(self.accounted_pending) + pending;
        stream.set_presentation_allocation(allocation)?;
        self.stale_cpu = stale_cpu;
        self.stale_gpu = stale_gpu;
        self.accounted_staging = staging;
        self.accounted_jobs = jobs;
        self.accounted_pending = pending;
        Ok(())
    }
    pub fn prepare(
        &mut self,
        stream: &mut WorldStream,
        reference: &ReferenceManifest,
        server: &AssetServer,
        assets: &mut Assets<SourceGltf>,
        plans: &mut Assets<SourcePlan>,
        meshes: &mut Assets<Mesh>,
        skins: &mut Assets<SkinnedMeshInverseBindposes>,
    ) -> Result<(), ReferenceError> {
        let session = stream.frame().request.session;
        for entry in &mut self.entries {
            if entry.session != session {
                entry.retired = true;
            }
        }
        let mut completed = None;
        for index in 0..self.entries.len() {
            let entry = &mut self.entries[index];
            match &mut entry.state {
                State::Queued => {
                    if entry.retired {
                        completed = Some(index);
                        break;
                    }
                }
                State::PlanLoading(job) => {
                    if let LoadState::Failed(error) = server.load_state(job.id()) {
                        if entry.retired {
                            completed = Some(index);
                            break;
                        }
                        entry.retired = true;
                        return Err(ReferenceError(format!("Source plan: {error}")));
                    }
                    let Some(SourcePlan(plan)) = plans.remove(job.id()) else {
                        continue;
                    };
                    if entry.retired {
                        completed = Some(index);
                        break;
                    }
                    // A consumed plan result cannot keep an unfinished job alive.
                    // On any admission error the next update retires this queued entry.
                    entry.state = State::Queued;
                    entry.retired = true;
                    let prototype = reference
                        .world
                        .prototypes
                        .iter()
                        .find(|p| p.source == entry.source)
                        .ok_or_else(|| ReferenceError("Missing checked source prototype".into()))?;
                    validate_plan(&plan, prototype, reference)?;
                    let record = reference
                        .assets
                        .iter()
                        .find(|a| a.id == prototype.asset)
                        .ok_or_else(|| ReferenceError("Missing prototype asset".into()))?;
                    let (cpu, gpu) = plan.reservation()?;
                    let cpu = cpu
                        .checked_add(plan.retained_bytes())
                        .ok_or_else(|| ReferenceError("Plan ownership overflow".into()))?;
                    stream.resize_prototype(&entry.source, cpu, gpu)?;
                    entry.cpu = cpu;
                    entry.gpu = gpu;
                    if 2 * (plan.json_bytes + plan.buffer_bytes) > entry.staging {
                        return Err(ReferenceError(
                            "Source prototype exceeds shared staging cap".into(),
                        ));
                    }
                    let settings = SourceGltfSettings {
                        json_bytes: plan.json_bytes,
                        buffer_bytes: plan.buffer_bytes,
                        mesh_bytes: plan.mesh_bytes,
                    };
                    let asset = server
                        .load_builder()
                        .with_settings(move |target: &mut SourceGltfSettings| {
                            *target = SourceGltfSettings {
                                json_bytes: settings.json_bytes,
                                buffer_bytes: settings.buffer_bytes,
                                mesh_bytes: settings.mesh_bytes,
                            }
                        })
                        .load::<SourceGltf>(record.path.clone());
                    entry.state = State::Loading { plan, asset };
                    entry.retired = false;
                }
                State::Loading { plan, asset } => {
                    if let LoadState::Failed(error) = server.load_state(asset.id()) {
                        if entry.retired {
                            completed = Some(index);
                            break;
                        }
                        entry.retired = true;
                        return Err(ReferenceError(format!(
                            "Source prototype {}:{}: {error}",
                            entry.source.file, entry.source.path_id
                        )));
                    }
                    let Some(loaded) = assets.get(asset) else {
                        continue;
                    };
                    if entry.retired {
                        completed = Some(index);
                        break;
                    }
                    if !loaded.meshes.iter().all(|mesh| meshes.contains(mesh.id()))
                        || !loaded
                            .inverse_bindposes
                            .iter()
                            .all(|skin| skins.contains(skin.id()))
                    {
                        continue;
                    }
                    entry.retired = true;
                    if loaded.primitives.len()
                        != plan
                            .nodes
                            .iter()
                            .map(|n| {
                                plan.primitives
                                    .iter()
                                    .filter(|p| Some(p.mesh) == n.mesh)
                                    .count()
                            })
                            .sum::<usize>()
                    {
                        return Err(ReferenceError(
                            "Source primitive count changed after preflight".into(),
                        ));
                    }
                    // Keep both main-world data and render extraction/upload copies charged.
                    let cpu = loaded.mesh_bytes * 2
                        + loaded.skin_bytes * 2
                        + plan.retained_bytes()
                        + plan.scene_bytes
                        + loaded.meshes.len() * 4096;
                    let gpu = loaded.mesh_bytes + loaded.skin_bytes + loaded.meshes.len() * 8;
                    stream.resize_prototype(&entry.source, cpu, gpu)?;
                    entry.cpu = cpu;
                    entry.gpu = gpu;
                    let State::Loading { plan, asset } =
                        std::mem::replace(&mut entry.state, State::Queued)
                    else {
                        unreachable!()
                    };
                    entry.state = State::Ready(Arc::new(LoadedPrototype { plan, asset }));
                    entry.retired = false;
                }
                State::Ready(_) => {
                    if entry.retired {
                        completed = Some(index);
                        break;
                    }
                }
            }
        }
        if let Some(index) = completed {
            let entry = &self.entries[index];
            // Active geometry must release its owners before prototype resources retire.
            stream.release_prototype(&entry.source)?;
            let entry = self.entries.remove(index);
            self.retired_sources.push(entry.source.clone());
            let asset = match entry.state {
                State::Ready(loaded) => {
                    if Arc::strong_count(&loaded) > 1 {
                        None
                    } else {
                        Some(loaded.asset.clone())
                    }
                }
                State::Loading { asset, .. } => Some(asset),
                _ => None,
            };
            if let Some(handle) = asset {
                if let Some(asset) = assets.remove(handle.id()) {
                    for mesh in asset.meshes {
                        meshes.remove(mesh.id());
                    }
                    for skin in asset.inverse_bindposes {
                        skins.remove(skin.id());
                    }
                }
            }
        }
        self.account(stream)?;
        if stream.available_external_jobs() == 0 {
            return Ok(());
        }
        if let Some(entry) = self
            .entries
            .iter_mut()
            .find(|entry| !entry.retired && matches!(entry.state, State::Queued))
        {
            let prototype = reference
                .world
                .prototypes
                .iter()
                .find(|p| p.source == entry.source)
                .ok_or_else(|| ReferenceError("Missing checked source prototype".into()))?;
            let record = reference
                .assets
                .iter()
                .find(|a| a.id == prototype.asset)
                .ok_or_else(|| ReferenceError("Missing prototype asset".into()))?;
            let input = reference.file_catalog.plan(&record.path)?;
            let plan_staging = (input.bytes as usize)
                .checked_mul(2)
                .ok_or_else(|| ReferenceError("Plan staging overflow".into()))?;
            let staging = reference
                .file_catalog
                .prototype_staging(record)?
                .max(plan_staging);
            if staging == 0 || staging > 32 * 1024 * 1024 {
                return Err(ReferenceError(
                    "Prototype input overlap exceeds staging cap".into(),
                ));
            }
            if staging > stream.available_dependency_staging_bytes() {
                return Ok(());
            }
            let budget = (input.bytes as usize)
                .checked_mul(18)
                .ok_or_else(|| ReferenceError("Plan metadata overflow".into()))?;
            if budget > stream.prototype_headroom(&entry.source) {
                return Err(ReferenceError(
                    "No source prototype metadata admission capacity".into(),
                ));
            }
            stream.reserve_prototype(entry.source.clone(), budget, 0)?;
            entry.cpu = budget;
            entry.gpu = 0;
            let mut allocation = stream.presentation_allocation();
            allocation.jobs += 1;
            allocation.staging_bytes += staging;
            allocation.dependency_staging_bytes += staging;
            stream.set_presentation_allocation(allocation)?;
            self.accounted_jobs += 1;
            self.accounted_staging += staging;
            entry.staging = staging;
            entry.state = State::PlanLoading(load_plan(
                server,
                &reference.file_catalog,
                &record.path,
                budget,
                staging,
            )?);
        }
        Ok(())
    }
}
fn validate_plan(
    plan: &PrototypePlan,
    prototype: &crate::reference::WorldPrototypeReference,
    reference: &ReferenceManifest,
) -> Result<(), ReferenceError> {
    for source in &prototype.nodes {
        let node = plan
            .nodes
            .get(source.gltf_node as usize)
            .ok_or_else(|| ReferenceError("Missing checked prototype node".into()))?;
        if node.index != source.gltf_node
            || node.extras.as_ref().and_then(|e| e.reference_id.as_ref()) != Some(&source.source)
        {
            return Err(ReferenceError(
                "Prototype node ownership differs from checked contract".into(),
            ));
        }
    }
    for primitive in &plan.primitives {
        let null = primitive.source_material.is_none()
            && (primitive.has_source_material || primitive.material.is_none());
        if null {
            let mut owners = plan.nodes.iter().filter(|n| n.mesh == Some(primitive.mesh));
            if !owners.clone().any(|_| true)
                || !owners.all(|node| {
                    node.extras
                        .as_ref()
                        .and_then(|e| e.reference_id.as_ref())
                        .and_then(|id| super::preflight::initial_material(reference, id))
                        .is_some()
                })
            {
                return Err(ReferenceError(
                    "Unresolved checked startup material slot".into(),
                ));
            }
            continue;
        }
        if let Some(source) = &primitive.source_material {
            if !prototype
                .deferred_materials
                .iter()
                .any(|m| m.source == *source)
            {
                return Err(ReferenceError(
                    "Unresolved deferred prototype material".into(),
                ));
            }
        } else if !primitive.has_source_material
            && primitive
                .material
                .as_ref()
                .is_none_or(|material| material.asset != prototype.asset)
        {
            return Err(ReferenceError(
                "Unresolved primary prototype material".into(),
            ));
        }
    }
    Ok(())
}
