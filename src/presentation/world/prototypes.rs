//! Selected prototype ownership. Reservations precede every accessor or scene allocation.
use super::{
    preflight::{
        SourcePlan, SourcePlanLoader, load_plan,
        selected::{IoLedger, Reservation, SelectedMetadata, control_bytes, retained_plan_bytes},
    },
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

pub type LoadedPrototype =
    super::preflight::selected::LoadedPrototype<Handle<SourcePlan>, Handle<SourceGltf>>;
type State = SelectedMetadata<Handle<SourcePlan>, Handle<SourceGltf>>;
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
    io: IoLedger,
    control_cpu: usize,
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
            let terminal = match &e.state {
                State::PlanLoading(job) => super::preflight::plan_terminal(server, plans, job),
                State::Loading { asset, .. } => {
                    assets.contains(asset.id())
                        || matches!(server.load_state(asset.id()), LoadState::Failed(_))
                }
                _ => true,
            };
            if !e.state.can_retire(terminal) {
                return false;
            }
        }
        true
    }
    pub fn release_all(
        self,
        assets: &mut Assets<SourceGltf>,
        _plans: &mut Assets<SourcePlan>,
        meshes: &mut Assets<Mesh>,
        skins: &mut Assets<SkinnedMeshInverseBindposes>,
    ) {
        for e in self.entries {
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
        let session = stream.frame().request.session;
        let control_cpu = control_bytes::<Entry>(
            self.entries.len(),
            self.entries.iter().map(|e| e.source.file.len()).sum(),
            self.entries.len(),
            0,
        );
        let mut allocation = stream.presentation_allocation();
        allocation.prototype_cpu += control_cpu;
        stream.set_presentation_allocation(allocation)?;
        let mut copy = Self {
            entries: Vec::with_capacity(self.entries.len()),
            retired_sources: Vec::with_capacity(self.entries.len()),
            control_cpu,
            accounted_session: session,
            ..default()
        };
        for e in &self.entries {
            let State::Ready(_) = &e.state else {
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
                state: e.state.fork_ready().unwrap(),
            });
        }
        copy.account(stream)?;
        Ok(copy)
    }

    pub fn request(
        &mut self,
        stream: &mut WorldStream,
        source: &SourceId,
    ) -> Result<(), ReferenceError> {
        let session = stream.frame().request.session;
        self.account(stream)?;
        if self.entries.iter().any(|e| e.source == *source) {
            return Ok(());
        }
        if stream.metrics().pending_work >= 32 {
            return Ok(());
        }
        if self.entries.len() >= 8192 {
            return Err(ReferenceError("Source prototype owner limit".into()));
        }
        // Reserve container storage before extending the selected owner list.
        let retired_capacity = self.entries.len() + self.retired_sources.len() + 1;
        let extra = usize::from(self.entries.len() == self.entries.capacity())
            * std::mem::size_of::<Entry>()
            + source.file.len()
            + retired_capacity.saturating_sub(self.retired_sources.capacity())
                * std::mem::size_of::<SourceId>();
        let mut allocation = stream.presentation_allocation();
        allocation.prototype_cpu += extra;
        allocation.pending += 1;
        stream.set_presentation_allocation(allocation)?;
        self.io.pending += 1;
        self.next_request = self.next_request.wrapping_add(1);
        self.control_cpu += extra;
        self.entries.reserve_exact(1);
        if self.retired_sources.capacity() < retired_capacity {
            self.retired_sources
                .reserve_exact(retired_capacity - self.retired_sources.len());
        }
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
    pub fn plan(&self, session: u64, source: &SourceId) -> Option<&PrototypePlan> {
        self.entries
            .iter()
            .find(|e| e.session == session && e.source == *source && !e.retired)
            .and_then(|e| e.state.plan())
    }
    pub fn admit_payload(
        &mut self,
        stream: &mut WorldStream,
        reference: &ReferenceManifest,
        server: &AssetServer,
        source: &SourceId,
    ) -> Result<bool, ReferenceError> {
        self.account(stream)?;
        let session = stream.frame().request.session;
        let Some(entry) = self
            .entries
            .iter_mut()
            .find(|e| e.session == session && e.source == *source && !e.retired)
        else {
            return Ok(false);
        };
        let State::Planned(plan) = &entry.state else {
            return Ok(matches!(
                entry.state,
                State::Loading { .. } | State::Ready(_)
            ));
        };
        let prototype = reference
            .world
            .prototypes
            .iter()
            .find(|p| p.source == *source)
            .ok_or_else(|| ReferenceError("Missing checked source prototype".into()))?;
        let record = reference
            .assets
            .iter()
            .find(|a| a.id == prototype.asset)
            .ok_or_else(|| ReferenceError("Missing prototype asset".into()))?;
        let reservation = entry.state.payload_reservation()?;
        if !reservation.fits(
            &stream.metrics(),
            stream.available_external_jobs(),
            stream.available_dependency_staging_bytes(),
            entry.cpu,
            entry.gpu,
            false,
        ) {
            return Ok(false);
        }
        let settings = SourceGltfSettings {
            json_bytes: plan.json_bytes,
            buffer_bytes: plan.buffer_bytes,
            mesh_bytes: plan.mesh_bytes,
        };
        let allocation = stream.presentation_allocation();
        stream.set_presentation_allocation(reservation.allocation(allocation, false))?;
        if let Err(error) = stream.resize_prototype(source, reservation.cpu, reservation.gpu) {
            stream.set_presentation_allocation(allocation)?;
            return Err(error);
        }
        let asset = server
            .load_builder()
            .with_settings(move |target: &mut SourceGltfSettings| {
                *target = SourceGltfSettings {
                    json_bytes: settings.json_bytes,
                    buffer_bytes: settings.buffer_bytes,
                    mesh_bytes: settings.mesh_bytes,
                };
            })
            .load::<SourceGltf>(record.path.clone());
        entry.state.start_payload(asset);
        entry.cpu = reservation.cpu;
        entry.gpu = reservation.gpu;
        entry.staging = reservation.staging;
        self.io.add(IoLedger {
            jobs: 1,
            pending: 1,
            staging: reservation.staging,
        });
        Ok(true)
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
    pub fn retire_unused(
        &mut self,
        stream: &WorldStream,
        reference: &ReferenceManifest,
        instances: &super::instances::SourceInstances,
    ) {
        let frame = stream.frame();
        for entry in &mut self.entries {
            if matches!(&entry.state, State::Ready(loaded) if Arc::strong_count(loaded)>1)
                || reference.world.road.prototypes.contains(&entry.source)
                || instances.uses_prototype(&entry.source)
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
            self.io = IoLedger::default();
            self.control_cpu = 0;
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
        let mut io = IoLedger::default();
        for entry in &self.entries {
            io.add(entry.state.io(entry.staging));
        }
        let control_cpu = control_bytes::<Entry>(
            self.entries.capacity(),
            self.entries.iter().map(|e| e.source.file.capacity()).sum(),
            self.retired_sources.capacity(),
            self.retired_sources.iter().map(|s| s.file.capacity()).sum(),
        );
        let mut allocation = stream.presentation_allocation();
        allocation.prototype_cpu = allocation
            .prototype_cpu
            .saturating_sub(self.stale_cpu + self.control_cpu)
            + stale_cpu
            + control_cpu;
        allocation.prototype_gpu =
            allocation.prototype_gpu.saturating_sub(self.stale_gpu) + stale_gpu;
        allocation = io.replace(self.io, allocation);
        stream.set_presentation_allocation(allocation)?;
        self.stale_cpu = stale_cpu;
        self.stale_gpu = stale_gpu;
        self.io = io;
        self.control_cpu = control_cpu;
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
                State::Queued | State::Planned(_) => {
                    if entry.retired {
                        completed = Some(index);
                        break;
                    }
                }
                State::PlanLoading(job) => {
                    let result = super::preflight::poll_plan(server, plans, job);
                    if let Err(error) = result {
                        if entry.retired {
                            completed = Some(index);
                            break;
                        }
                        entry.retired = true;
                        return Err(error);
                    }
                    let Some(plan) = result.unwrap() else {
                        continue;
                    };
                    if entry.retired {
                        completed = Some(index);
                        break;
                    }
                    entry.state.complete_metadata(plan);
                    let plan = entry.state.plan().unwrap();
                    let prototype = reference
                        .world
                        .prototypes
                        .iter()
                        .find(|p| p.source == entry.source)
                        .ok_or_else(|| ReferenceError("Missing checked source prototype".into()))?;
                    entry.retired = true;
                    validate_plan(plan, prototype, reference)?;
                    let cpu = retained_plan_bytes(plan);
                    stream.resize_prototype(&entry.source, cpu, 0)?;
                    entry.cpu = cpu;
                    entry.gpu = 0;
                    entry.staging = 0;
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
                        + retained_plan_bytes(plan)
                        + State::ready_control_bytes()
                        + plan.scene_bytes
                        + loaded.meshes.len() * 4096;
                    let gpu = loaded.mesh_bytes + loaded.skin_bytes + loaded.meshes.len() * 8;
                    stream.resize_prototype(&entry.source, cpu, gpu)?;
                    entry.cpu = cpu;
                    entry.gpu = gpu;
                    entry.state.complete_payload();
                    entry.staging = 0;
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
            self.retired_sources.push(entry.source);
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
            let reservation = Reservation::metadata(&reference.file_catalog, &record.path)?;
            if !reservation.fits(
                &stream.metrics(),
                stream.available_external_jobs(),
                stream.available_dependency_staging_bytes(),
                entry.cpu,
                entry.gpu,
                true,
            ) || reservation.cpu > stream.prototype_headroom(&entry.source)
            {
                return Ok(());
            }
            let allocation = stream.presentation_allocation();
            stream.set_presentation_allocation(reservation.allocation(allocation, true))?;
            if stream
                .reserve_prototype(entry.source.clone(), reservation.cpu, 0)
                .is_err()
            {
                stream.set_presentation_allocation(allocation)?;
                return Ok(());
            }
            let handle = load_plan(
                server,
                &reference.file_catalog,
                &record.path,
                reservation.cpu,
                reservation.staging,
            )?;
            entry.cpu = reservation.cpu;
            entry.gpu = 0;
            entry.staging = reservation.staging;
            entry.state = State::PlanLoading(handle);
            self.io.jobs += 1;
            self.io.staging += reservation.staging;
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
