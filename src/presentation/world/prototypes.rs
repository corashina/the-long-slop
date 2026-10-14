//! Selected prototype ownership. Reservations precede every accessor or scene allocation.
use super::{
    preflight::{PREFLIGHT_RESERVATION, PrototypePlan, PrototypePreflight},
    prototype_loader::{SourceGltf, SourceGltfLoader, SourceGltfSettings},
};
use crate::{
    reference::{ReferenceError, ReferenceManifest, SourceId},
    world::WorldStream,
};
use bevy::{
    asset::LoadState, ecs as bevy_ecs, mesh::skinning::SkinnedMeshInverseBindposes, prelude::*,
};

pub struct LoadedPrototype {
    pub plan: PrototypePlan,
    pub asset: Handle<SourceGltf>,
}
enum State {
    Queued,
    Preflight(PrototypePreflight),
    Loading {
        plan: PrototypePlan,
        asset: Handle<SourceGltf>,
    },
    Ready(LoadedPrototype),
}
struct Entry {
    source: SourceId,
    session: u64,
    request: u64,
    retired: bool,
    cpu: usize,
    gpu: usize,
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
            .init_resource::<SourcePrototypes>();
    }
}
impl SourcePrototypes {
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
            state: State::Queued,
        });
        Ok(())
    }
    pub fn loaded(&self, session: u64, source: &SourceId) -> Option<&LoadedPrototype> {
        self.entries.iter().find_map(|entry| {
            if entry.session == session && entry.source == *source && !entry.retired {
                if let State::Ready(loaded) = &entry.state {
                    Some(loaded)
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
                if let State::Preflight(job) = &entry.state {
                    job.cancel();
                }
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
            if reference.world.road.prototypes.contains(&entry.source) {
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
            if let State::Preflight(job) = &entry.state {
                job.cancel();
            }
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
            .filter(|entry| matches!(entry.state, State::Preflight(_) | State::Loading { .. }))
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
                State::Preflight(_) => 32 * 1024 * 1024,
                State::Loading { plan, .. } => 2 * (plan.json_bytes + plan.buffer_bytes),
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
        meshes: &mut Assets<Mesh>,
        skins: &mut Assets<SkinnedMeshInverseBindposes>,
    ) -> Result<(), ReferenceError> {
        let session = stream.frame().request.session;
        for entry in &mut self.entries {
            if entry.session != session {
                entry.retired = true;
                if let State::Preflight(job) = &entry.state {
                    job.cancel();
                }
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
                State::Preflight(job) => {
                    let Some(result) = job.take() else { continue };
                    if entry.retired {
                        completed = Some(index);
                        break;
                    }
                    // A consumed preflight result cannot keep an unfinished job alive.
                    // On any admission error the next update retires this queued entry.
                    entry.state = State::Queued;
                    entry.retired = true;
                    let plan = result?;
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
                    stream.resize_prototype(&entry.source, cpu, gpu)?;
                    entry.cpu = cpu;
                    entry.gpu = gpu;
                    if 2 * (plan.json_bytes + plan.buffer_bytes) > 32 * 1024 * 1024 {
                        return Err(ReferenceError(
                            "Source prototype exceeds shared staging cap".into(),
                        ));
                    }
                    let settings = SourceGltfSettings {
                        json_bytes: plan.json_bytes,
                        buffer_bytes: plan.buffer_bytes,
                        mesh_bytes: plan.mesh_bytes,
                        sha256: record.sha256.clone(),
                        dependencies: record.dependencies.clone(),
                    };
                    let asset = server
                        .load_builder()
                        .with_settings(move |target: &mut SourceGltfSettings| {
                            *target = SourceGltfSettings {
                                json_bytes: settings.json_bytes,
                                buffer_bytes: settings.buffer_bytes,
                                mesh_bytes: settings.mesh_bytes,
                                sha256: settings.sha256.clone(),
                                dependencies: settings.dependencies.clone(),
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
                    if loaded.primitives.len() != plan.primitives.len() {
                        return Err(ReferenceError(
                            "Source primitive count changed after preflight".into(),
                        ));
                    }
                    // Keep both main-world data and render extraction/upload copies charged.
                    let cpu = loaded.mesh_bytes * 2
                        + loaded.skin_bytes * 2
                        + plan.retained_bytes()
                        + plan.scene_bytes;
                    let gpu = loaded.mesh_bytes + loaded.skin_bytes;
                    stream.resize_prototype(&entry.source, cpu, gpu)?;
                    entry.cpu = cpu;
                    entry.gpu = gpu;
                    let State::Loading { plan, asset } =
                        std::mem::replace(&mut entry.state, State::Queued)
                    else {
                        unreachable!()
                    };
                    entry.state = State::Ready(LoadedPrototype { plan, asset });
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
                State::Ready(loaded) => Some(loaded.asset),
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
        if stream.available_external_jobs() == 0
            || stream.available_staging_bytes() < 32 * 1024 * 1024
        {
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
            let budget = PREFLIGHT_RESERVATION.min(stream.prototype_headroom(&entry.source));
            if budget < 18 {
                return Err(ReferenceError(
                    "No source prototype metadata admission capacity".into(),
                ));
            }
            stream.reserve_prototype(entry.source.clone(), budget, 0)?;
            entry.cpu = budget;
            entry.gpu = 0;
            let mut allocation = stream.presentation_allocation();
            allocation.jobs += 1;
            allocation.staging_bytes += 32 * 1024 * 1024;
            stream.set_presentation_allocation(allocation)?;
            self.accounted_jobs += 1;
            self.accounted_staging += 32 * 1024 * 1024;
            entry.state = State::Preflight(PrototypePreflight::start(
                entry.session,
                entry.request,
                record.path.clone(),
                record.sha256.clone(),
                budget,
            ));
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
