//! Exact ownership for deferred GPU pipeline creation and retained cache metadata.
use super::gpu_prepare::GpuPreparation;
use crate::{
    reference::ReferenceError,
    world::pipeline_ownership::{
        HeldEntryAction, HeldEntryObservation, HeldEntryPhase, held_entry_step,
    },
};
use bevy::{
    ecs as bevy_ecs,
    prelude::*,
    render::{render_resource::*, renderer::RenderDevice},
};
use std::{
    collections::{HashMap, HashSet},
    sync::Mutex,
};
// Count and retained capacity are separate bounds. Every bank admits the persistent cache reserve.
pub const CACHE_SLOTS: usize = 2048;
pub const DESCRIPTOR_BYTES: usize = 64 * 1024;
pub const CACHE_BYTES: usize = 16 * 1024 * 1024;
struct Entry {
    base: CachedRenderPipelineId,
    owner: Option<GpuPreparation>,
    phase: HeldEntryPhase,
}
#[derive(Resource, Default)]
pub struct PipelineOwners {
    entries: Mutex<HashMap<CachedRenderPipelineId, Entry>>,
    isolated_bases: HashSet<CachedRenderPipelineId>,
}
impl PipelineOwners {
    pub(crate) fn isolated(&self, id: CachedRenderPipelineId) -> bool {
        self.entries.lock().unwrap().contains_key(&id)
    }
    pub(crate) fn base_isolated(&self, base: CachedRenderPipelineId) -> bool {
        self.isolated_bases.contains(&base)
    }
    fn owner(&self, base: CachedRenderPipelineId) -> Option<GpuPreparation> {
        self.entries
            .lock()
            .unwrap()
            .get(&base)
            .and_then(|e| e.owner.clone())
    }
    pub(crate) fn cancelled(&self, base: CachedRenderPipelineId) -> bool {
        self.owner(base).is_some_and(|owner| owner.cancelled())
    }
    pub(crate) fn fail(&self, base: CachedRenderPipelineId, message: String) -> bool {
        let Some(owner) = self.owner(base) else {
            return false;
        };
        owner.fail(message);
        true
    }
    fn claim(
        &self,
        cache: &PipelineCache,
        base: CachedRenderPipelineId,
        owner: &GpuPreparation,
    ) -> bool {
        if !cache.render_pipeline_is_held(base) {
            return false;
        }
        let mut entries = self.entries.lock().unwrap();
        let entry = entries.entry(base).or_insert_with(|| Entry {
            base,
            owner: None,
            phase: HeldEntryPhase::Held,
        });
        if entry.phase == HeldEntryPhase::Held
            && entry
                .owner
                .as_ref()
                .is_some_and(|old| old.same_owner(owner))
        {
            return false;
        }
        for (&id, entry) in entries.iter_mut() {
            if id != base && entry.base != base {
                continue;
            }
            if let Some(old) = entry.owner.take() {
                if entry.phase == HeldEntryPhase::Held {
                    old.release_pipeline();
                }
            }
            entry.phase = HeldEntryPhase::Held;
            owner.retain_pipeline();
            entry.owner = Some(owner.clone());
        }
        true
    }
    fn companion(
        &self,
        base: CachedRenderPipelineId,
        id: CachedRenderPipelineId,
        owner: GpuPreparation,
    ) {
        let mut entries = self.entries.lock().unwrap();
        let old = entries.insert(
            id,
            Entry {
                base,
                owner: Some(owner),
                phase: HeldEntryPhase::Held,
            },
        );
        entries[&id].owner.as_ref().unwrap().retain_pipeline();
        if let Some(old) = old {
            if old.phase == HeldEntryPhase::Held {
                old.owner.unwrap().release_pipeline();
            }
        }
    }
    pub(crate) fn queue_companion(
        &self,
        cache: &PipelineCache,
        base: CachedRenderPipelineId,
        build: impl FnOnce() -> Option<RenderPipelineDescriptor>,
    ) -> Result<Option<CachedRenderPipelineId>, ReferenceError> {
        let owner = self.owner(base);
        if owner.as_ref().is_some_and(|owner| owner.cancelled()) {
            return Ok(None);
        }
        let _queue = owner
            .as_ref()
            .map(|_| queue_scope(cache))
            .transpose()
            .map_err(|error| ReferenceError(error.into()))?;
        let Some(descriptor) = build() else {
            return Ok(None);
        };
        let id = cache.queue_render_pipeline(descriptor);
        if let Some(owner) = owner {
            cache.hold_render_pipeline(id);
            self.companion(base, id, owner);
        }
        Ok(Some(id))
    }
    fn reclaim_active(&self, cache: &mut PipelineCache, base: CachedRenderPipelineId) -> bool {
        let mut reclaimed = false;
        self.entries.lock().unwrap().retain(|id, entry| {
            if *id != base && entry.base != base {
                return true;
            }
            reclaimed = true;
            cache.release_held_render_pipeline(*id);
            if let Some(owner) = &entry.owner {
                if entry.phase == HeldEntryPhase::Held {
                    owner.release_pipeline();
                }
            }
            false
        });
        reclaimed
    }
}
pub(crate) enum PipelineUse<'a> {
    Active,
    Candidate(&'a GpuPreparation),
}
pub(crate) fn specialize_base(
    world: &mut World,
    usage: PipelineUse<'_>,
    specialize: impl FnOnce(&mut World) -> Result<CachedRenderPipelineId, ReferenceError>,
) -> Result<Option<CachedRenderPipelineId>, ReferenceError> {
    let owner = match usage {
        PipelineUse::Active => None,
        PipelineUse::Candidate(owner) => Some(owner),
    };
    if owner.is_some_and(|owner| owner.cancelled()) {
        return Ok(None);
    }
    let _queue = owner
        .map(|_| queue_scope(world.resource::<PipelineCache>()))
        .transpose()
        .map_err(|error| ReferenceError(error.into()))?;
    let id = specialize(world)?;
    let changed = if let Some(owner) = owner {
        let owners = world.resource::<PipelineOwners>();
        let changed = owners.claim(world.resource::<PipelineCache>(), id, owner);
        if owners.isolated(id) {
            world
                .resource_mut::<PipelineOwners>()
                .isolated_bases
                .insert(id);
        }
        changed
    } else {
        world.resource_scope(|world, owners: Mut<PipelineOwners>| {
            owners.reclaim_active(&mut world.resource_mut::<PipelineCache>(), id)
        })
    };
    if changed {
        world
            .resource_mut::<super::caster::CasterPasses>()
            .revisit(id);
        world
            .resource_mut::<super::surface_pass::SurfacePasses>()
            .revisit(id);
    }
    Ok(Some(id))
}
fn queue_scope(cache: &PipelineCache) -> Result<OwnedRenderPipelineQueue, &'static str> {
    cache.owned_render_pipeline_queue(CACHE_SLOTS, CACHE_BYTES, DESCRIPTOR_BYTES)
}
pub(crate) fn refresh_isolation(world: &mut World) {
    world.resource_scope(|world, mut owners: Mut<PipelineOwners>| {
        let active = world.resource::<super::world::pipelines::WorldPipelines>();
        owners.isolated_bases.retain(|id| !active.owns(*id));
    });
}
pub(crate) fn prepare(world: &mut World) {
    let device = world.resource::<RenderDevice>().clone();
    world.resource_scope(|world, owners: Mut<PipelineOwners>| {
        let mut entries = owners.entries.lock().unwrap();
        let mut cache = world.resource_mut::<PipelineCache>();
        entries.retain(|id, entry| {
            let Some(owner) = entry.owner.as_ref() else {
                return true;
            };
            let observation = if entry.phase == HeldEntryPhase::Retired {
                HeldEntryObservation {
                    cancelled: false,
                    isolated: false,
                    pipeline_ready: false,
                    preparation_valid: false,
                }
            } else {
                let cancelled = owner.cancelled();
                let isolated = !cancelled && owner.isolated();
                let pipeline_ready = !cancelled && cache.get_render_pipeline(*id).is_some();
                HeldEntryObservation {
                    cancelled,
                    isolated,
                    pipeline_ready,
                    preparation_valid: isolated && !pipeline_ready && owner.ready().is_ok(),
                }
            };
            match held_entry_step(&mut entry.phase, observation) {
                HeldEntryAction::Wait => true,
                HeldEntryAction::Retire => {
                    cache.retire_held_render_pipeline(*id);
                    owner.release_pipeline();
                    true
                }
                HeldEntryAction::Release => {
                    cache.release_held_render_pipeline(*id);
                    owner.release_pipeline();
                    false
                }
                HeldEntryAction::Prepare => {
                    let _scope = owner.scope(&device);
                    cache.prepare_held_render_pipeline(*id);
                    true
                }
            }
        });
    });
}
