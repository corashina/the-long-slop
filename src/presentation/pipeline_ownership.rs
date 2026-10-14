//! Exact ownership for deferred GPU pipeline creation and retained cache metadata.
use super::gpu_prepare::GpuPreparation;
use bevy::{
    ecs as bevy_ecs,
    prelude::*,
    render::{render_resource::*, renderer::RenderDevice},
};
use std::{collections::HashMap, sync::Mutex};
// Count and retained capacity are separate bounds. Every bank admits the persistent cache reserve.
pub const CACHE_SLOTS: usize = 2048;
pub const DESCRIPTOR_BYTES: usize = 64 * 1024;
pub const CACHE_BYTES: usize = 16 * 1024 * 1024;
struct Entry {
    base: CachedRenderPipelineId,
    owner: Option<GpuPreparation>,
    retired: bool,
}
#[derive(Resource, Default)]
pub struct PipelineOwners(Mutex<HashMap<CachedRenderPipelineId, Entry>>);
impl PipelineOwners {
    pub fn isolated(&self, id: CachedRenderPipelineId) -> bool {
        self.0.lock().unwrap().contains_key(&id)
    }
    pub fn owner(&self, base: CachedRenderPipelineId) -> Option<GpuPreparation> {
        self.0
            .lock()
            .unwrap()
            .get(&base)
            .and_then(|e| e.owner.clone())
    }
    pub fn claim(
        &self,
        cache: &PipelineCache,
        base: CachedRenderPipelineId,
        owner: &GpuPreparation,
    ) -> bool {
        if !cache.render_pipeline_is_held(base) {
            return false;
        }
        let mut entries = self.0.lock().unwrap();
        let entry = entries.entry(base).or_insert_with(|| Entry {
            base,
            owner: None,
            retired: false,
        });
        if !entry.retired
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
                if !entry.retired {
                    old.release_pipeline();
                }
            }
            entry.retired = false;
            owner.retain_pipeline();
            entry.owner = Some(owner.clone());
        }
        true
    }
    pub fn companion(
        &self,
        cache: &PipelineCache,
        base: CachedRenderPipelineId,
        id: CachedRenderPipelineId,
    ) {
        let mut entries = self.0.lock().unwrap();
        let Some(owner) = entries.get(&base).and_then(|e| e.owner.clone()) else {
            return;
        };
        cache.hold_render_pipeline(id);
        owner.retain_pipeline();
        let old = entries.insert(
            id,
            Entry {
                base,
                owner: Some(owner),
                retired: false,
            },
        );
        if let Some(old) = old {
            if !old.retired {
                old.owner.unwrap().release_pipeline();
            }
        }
    }
    pub fn reclaim_active(&self, cache: &mut PipelineCache, base: CachedRenderPipelineId) -> bool {
        let mut reclaimed = false;
        self.0.lock().unwrap().retain(|id, entry| {
            if *id != base && entry.base != base {
                return true;
            }
            reclaimed = true;
            cache.release_held_render_pipeline(*id);
            if let Some(owner) = &entry.owner {
                if !entry.retired {
                    owner.release_pipeline();
                }
            }
            false
        });
        reclaimed
    }
}
pub fn queue_scope(cache: &PipelineCache) -> Result<OwnedRenderPipelineQueue, &'static str> {
    cache.owned_render_pipeline_queue(CACHE_SLOTS, CACHE_BYTES, DESCRIPTOR_BYTES)
}
pub fn prepare(world: &mut World) {
    let device = world.resource::<RenderDevice>().clone();
    world.resource_scope(|world, owners: Mut<PipelineOwners>| {
        let mut entries = owners.0.lock().unwrap();
        let mut cache = world.resource_mut::<PipelineCache>();
        entries.retain(|id, entry| {
            if entry.retired {
                return true;
            }
            let Some(owner) = entry.owner.as_ref() else {
                return true;
            };
            if owner.cancelled() {
                if !entry.retired {
                    cache.retire_held_render_pipeline(*id);
                    owner.release_pipeline();
                    entry.retired = true;
                }
                return true;
            }
            if !owner.isolated() {
                // Commit can release only completed GPU objects.
                assert!(
                    cache.get_render_pipeline(*id).is_some(),
                    "Promoted pipeline is not ready"
                );
                cache.release_held_render_pipeline(*id);
                owner.release_pipeline();
                return false;
            }
            if cache.get_render_pipeline(*id).is_none() && owner.ready().is_ok() {
                let _scope = owner.scope(&device);
                cache.prepare_held_render_pipeline(*id);
            }
            true
        });
    });
}
