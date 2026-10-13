//! Dynamic primary materials share the accepted source material cache.
use super::*;
use crate::{
    reference::{AssetId as SourceAssetId, ReferenceError},
    world::WorldStream,
};
const ENCODED_JOB_BYTES: usize = 32 * 1024 * 1024;
#[derive(Default)]
pub struct PrimaryOwnership {
    assets: HashMap<SourceAssetId, Vec<String>>,
    borrowed_records: std::collections::HashSet<SourceAssetId>,
    cpu: usize,
    gpu: usize,
    jobs: usize,
}
fn pixels(texture: &TextureRecord) -> usize {
    texture
        .mips
        .iter()
        .map(|m| m.width as usize * m.height as usize * 4)
        .sum()
}
fn texture_metadata(texture: &TextureRecord) -> usize {
    use std::mem::size_of;
    size_of::<TextureRecord>()
        + texture.slot.capacity()
        + texture.source.file.capacity()
        + texture.file.capacity()
        + texture.mips.capacity() * size_of::<source::TextureMip>()
        + texture
            .mips
            .iter()
            .map(|m| m.file.capacity())
            .sum::<usize>()
}
fn record_bytes(record: &MaterialRecord) -> usize {
    record.retained_cpu_bytes()
}

impl SourceMaterials {
    pub fn begin_world_session(&mut self) {
        // Resources stay owned. The new stream receives their full charges on preparation.
        self.world_primary.cpu = 0;
        self.world_primary.gpu = 0;
        self.world_primary.jobs = 0;
        self.world_deferred = DeferredAllocation::default();
    }

    fn primary_bytes(&self) -> (usize, usize) {
        let cpu = self
            .textures
            .iter()
            .filter(|t| !t.eager)
            .map(|t| {
                pixels(&t.record) * 2
                    + texture_metadata(&t.record)
                    + t.label.capacity()
                    + t.levels.capacity() * std::mem::size_of::<Handle<Image>>()
            })
            .sum::<usize>()
            + self
                .records
                .iter()
                .filter(|r| {
                    self.world_primary.assets.contains_key(&r.asset)
                        && !self.world_primary.borrowed_records.contains(&r.asset)
                })
                .map(record_bytes)
                .sum::<usize>();
        let gpu = self
            .textures
            .iter()
            .filter(|t| !t.eager)
            .map(|t| pixels(&t.record))
            .sum();
        (cpu, gpu)
    }
    fn account_primary(
        &mut self,
        stream: &mut WorldStream,
        cpu: usize,
        gpu: usize,
        jobs: usize,
    ) -> Result<(), ReferenceError> {
        let mut allocation = stream.presentation_allocation();
        allocation.prototype_cpu = allocation
            .prototype_cpu
            .saturating_sub(self.world_primary.cpu)
            + cpu;
        allocation.prototype_gpu = allocation
            .prototype_gpu
            .saturating_sub(self.world_primary.gpu)
            + gpu;
        allocation.jobs = allocation.jobs.saturating_sub(self.world_primary.jobs) + jobs;
        allocation.staging_bytes = allocation
            .staging_bytes
            .saturating_sub(self.world_primary.jobs * ENCODED_JOB_BYTES)
            + jobs * ENCODED_JOB_BYTES;
        stream.set_presentation_allocation(allocation)?;
        self.world_primary.cpu = cpu;
        self.world_primary.gpu = gpu;
        self.world_primary.jobs = jobs;
        Ok(())
    }
    pub fn admit_world_asset(
        &mut self,
        asset: SourceAssetId,
        reference: &ReferenceManifest,
        stream: &mut WorldStream,
    ) -> Result<(), ReferenceError> {
        if self.world_primary.assets.contains_key(&asset) {
            return Ok(());
        }
        let borrowed = self.records.iter().any(|r| r.asset == asset);
        let records: Vec<_> = reference
            .rendering
            .materials
            .iter()
            .filter(|r| r.asset == asset)
            .collect();
        let mut paths = Vec::new();
        let mut added_cpu = 0usize;
        let mut added_gpu = 0usize;
        for record in &records {
            added_cpu += record_bytes(record);
            for texture in &record.properties.textures {
                if paths.contains(&texture.file) {
                    continue;
                }
                paths.push(texture.file.clone());
                if let Some(&index) = self.texture_indices.get(&texture.file) {
                    let retained = &self.textures[index].record;
                    let a = &retained.sampler;
                    let b = &texture.sampler;
                    if retained.source != texture.source
                        || retained.color_space != texture.color_space
                        || retained.encoding != texture.encoding
                        || a.filter != b.filter
                        || a.wrap_u != b.wrap_u
                        || a.wrap_v != b.wrap_v
                        || a.aniso != b.aniso
                        || a.mip_bias.to_bits() != b.mip_bias.to_bits()
                        || a.mip_count != b.mip_count
                        || retained.mips.len() != texture.mips.len()
                        || !retained.mips.iter().zip(&texture.mips).all(|(a, b)| {
                            a.file == b.file && a.width == b.width && a.height == b.height
                        })
                    {
                        return Err(ReferenceError(
                            "Conflicting source texture identity or sampler".into(),
                        ));
                    }
                }
                if !self.texture_indices.contains_key(&texture.file) {
                    let bytes = pixels(texture);
                    added_gpu += bytes;
                    added_cpu += bytes * 2
                        + std::mem::size_of::<SourceTexture>()
                        + texture.file.len() * 2
                        + texture.source.file.len()
                        + texture.mips.len() * std::mem::size_of::<source::TextureMip>()
                        + texture.mips.iter().map(|m| m.file.len()).sum::<usize>()
                        + 128;
                }
            }
        }
        self.account_primary(
            stream,
            self.world_primary.cpu + added_cpu,
            self.world_primary.gpu + added_gpu,
            self.world_primary.jobs,
        )?;
        for record in records {
            for texture in &record.properties.textures {
                if !self.texture_indices.contains_key(&texture.file) {
                    self.texture_indices
                        .insert(texture.file.clone(), self.textures.len());
                    self.textures.push(SourceTexture {
                        failed: false,
                        eager: false,
                        users: 0,
                        record: texture.clone(),
                        levels: Vec::with_capacity(texture.mips.len()),
                        image: None,
                        label: format!(
                            "source material {}:{} {}",
                            record.source.file, record.source.path_id, texture.slot
                        ),
                    });
                }
            }
            if !borrowed {
                self.records.push(record.clone());
            }
        }
        for path in &paths {
            self.textures[self.texture_indices[path]].users += 1;
        }
        if borrowed {
            self.world_primary.borrowed_records.insert(asset);
        }
        self.world_primary.assets.insert(asset, paths);
        Ok(())
    }
    pub fn world_asset_ready(&self, asset: SourceAssetId) -> bool {
        self.world_primary.assets.get(&asset).is_some_and(|paths| {
            paths
                .iter()
                .all(|path| self.textures[self.texture_indices[path]].image.is_some())
        })
    }
    pub fn prepare_world_primary(
        &mut self,
        stream: &mut WorldStream,
        server: &AssetServer,
        images: &mut Assets<Image>,
        materials: &mut Assets<SourceMaterial>,
    ) -> Result<(), ReferenceError> {
        let mut failure = None;
        for texture in self
            .textures
            .iter_mut()
            .filter(|t| !t.eager && t.image.is_none())
        {
            for handle in &texture.levels {
                if let LoadState::Failed(error) = server.load_state(handle.id()) {
                    if !texture.failed {
                        failure = Some(ReferenceError(format!("{}: {error}", texture.label)));
                    }
                    texture.failed = true;
                }
            }
            if texture.failed {
                texture.levels.retain(|h| {
                    if matches!(server.load_state(h.id()), LoadState::Failed(_)) {
                        images.remove(h.id());
                        false
                    } else {
                        true
                    }
                });
            }
        }
        let retired: Vec<_> = self
            .world_primary
            .assets
            .iter()
            .filter(|(_, paths)| {
                paths
                    .iter()
                    .any(|p| self.textures[self.texture_indices[p]].failed)
            })
            .map(|(id, _)| *id)
            .collect();
        for asset in retired {
            if self.world_asset_retirable(asset, images, server) {
                self.release_world_asset(asset, images, materials, server)?;
            }
        }
        let jobs = self
            .textures
            .iter()
            .filter(|t| !t.eager && t.image.is_none())
            .flat_map(|t| &t.levels)
            .filter(|h| !images.contains(h.id()))
            .count();
        let (cpu, gpu) = self.primary_bytes();
        self.account_primary(stream, cpu + jobs * ENCODED_JOB_BYTES, gpu, jobs)?;
        for index in 0..self.textures.len() {
            if stream.available_external_jobs() == 0
                || stream.available_staging_bytes() < ENCODED_JOB_BYTES
            {
                break;
            }
            let texture = &self.textures[index];
            if texture.failed
                || texture.eager
                || texture.image.is_some()
                || texture.levels.len() == texture.record.mips.len()
            {
                continue;
            }
            self.account_primary(
                stream,
                self.world_primary.cpu + ENCODED_JOB_BYTES,
                gpu,
                self.world_primary.jobs + 1,
            )?;
            let texture = &mut self.textures[index];
            let mip = &texture.record.mips[texture.levels.len()];
            let srgb = texture.record.color_space == TextureColorSpace::Srgb;
            texture.levels.push(
                server
                    .load_builder()
                    .with_settings(move |settings: &mut ImageLoaderSettings| {
                        settings.is_srgb = srgb;
                        settings.asset_usage = RenderAssetUsages::MAIN_WORLD;
                    })
                    .load(mip.file.clone()),
            );
        }
        let assembled = assemble_textures(&mut self.textures, server, images, false);
        if let Ok(bytes) = &assembled {
            self.texture_bytes += bytes;
        }
        let jobs = self
            .textures
            .iter()
            .filter(|t| !t.eager && t.image.is_none())
            .flat_map(|t| &t.levels)
            .filter(|h| !images.contains(h.id()))
            .count();
        let (cpu, gpu) = self.primary_bytes();
        self.account_primary(stream, cpu + jobs * ENCODED_JOB_BYTES, gpu, jobs)?;
        if let Some(error) = failure {
            return Err(error);
        }
        assembled.map(|_| ())
    }
    pub fn world_asset_retirable(
        &self,
        asset: SourceAssetId,
        images: &Assets<Image>,
        server: &AssetServer,
    ) -> bool {
        self.world_primary.assets.get(&asset).is_none_or(|paths| {
            paths.iter().all(|path| {
                let t = &self.textures[self.texture_indices[path]];
                t.eager
                    || t.image.is_some()
                    || t.levels.iter().all(|h| {
                        images.contains(h.id())
                            || matches!(server.load_state(h.id()), LoadState::Failed(_))
                    })
            })
        })
    }
    pub fn release_world_asset(
        &mut self,
        asset: SourceAssetId,
        images: &mut Assets<Image>,
        materials: &mut Assets<SourceMaterial>,
        server: &AssetServer,
    ) -> Result<(), ReferenceError> {
        let Some(_paths) = self.world_primary.assets.get(&asset) else {
            return Ok(());
        };
        if !self.world_asset_retirable(asset, images, server) {
            return Err(ReferenceError(
                "Source primary texture retirement still has a loader job".into(),
            ));
        }
        let paths = self.world_primary.assets.remove(&asset).unwrap();
        for path in paths {
            self.textures[self.texture_indices[&path]].users -= 1;
        }
        // Startup overrides can borrow one of the five imported material assets.
        // Their cache handles and records remain owned by the imported scene.
        if !self.world_primary.borrowed_records.remove(&asset) {
            self.handles.retain(|key, handle| {
                if key.0 == asset {
                    materials.remove(handle.id());
                    false
                } else {
                    true
                }
            });
            self.records.retain(|r| r.asset != asset);
        }
        self.textures.retain(|texture| {
            if !texture.eager && texture.users == 0 {
                for level in &texture.levels {
                    images.remove(level.id());
                }
                if let Some(image) = &texture.image {
                    images.remove(image.id());
                }
                false
            } else {
                true
            }
        });
        self.texture_indices.clear();
        for (index, texture) in self.textures.iter().enumerate() {
            self.texture_indices
                .insert(texture.record.file.clone(), index);
        }
        Ok(())
    }
}

#[derive(Default)]
pub struct DeferredAllocation {
    staging: usize,
    cpu: usize,
    gpu: usize,
    jobs: usize,
}
impl SourceMaterials {
    fn account_deferred(
        &mut self,
        stream: &mut WorldStream,
        jobs: usize,
        staging: usize,
    ) -> Result<(), ReferenceError> {
        let (cpu, gpu) = self.world_textures.bytes();
        let mut allocation = stream.presentation_allocation();
        allocation.prototype_cpu = allocation
            .prototype_cpu
            .saturating_sub(self.world_deferred.cpu)
            + cpu;
        allocation.prototype_gpu = allocation
            .prototype_gpu
            .saturating_sub(self.world_deferred.gpu)
            + gpu;
        allocation.jobs = allocation.jobs.saturating_sub(self.world_deferred.jobs) + jobs;
        allocation.staging_bytes = allocation
            .staging_bytes
            .saturating_sub(self.world_deferred.staging)
            + staging;
        stream.set_presentation_allocation(allocation)?;
        self.world_deferred = DeferredAllocation {
            cpu,
            gpu,
            jobs,
            staging,
        };
        Ok(())
    }
    pub fn admit_world_material(
        &mut self,
        material: &source::WorldSourceMaterial,
        reference: &ReferenceManifest,
        stream: &mut WorldStream,
    ) -> Result<(), ReferenceError> {
        if self.world_textures.admitted(&material.source) {
            return Ok(());
        }
        let metrics = stream.metrics();
        let (cpu, gpu) = self.world_textures.bytes();
        self.world_textures.admit(
            material,
            if reference
                .world
                .terrain
                .materials
                .iter()
                .any(|m| m.source == material.source)
            {
                &reference.world.terrain.textures
            } else {
                &reference.world.material_textures
            },
            (256 * 1024 * 1024usize).saturating_sub(metrics.prototype_cpu_bytes) + cpu,
            (512 * 1024 * 1024usize).saturating_sub(metrics.prototype_gpu_bytes) + gpu,
        )?;
        self.account_deferred(
            stream,
            self.world_deferred.jobs,
            self.world_deferred.staging,
        )
    }
    pub fn prepare_world_deferred(
        &mut self,
        stream: &mut WorldStream,
        server: &AssetServer,
        images: &mut Assets<Image>,
    ) -> Result<(), ReferenceError> {
        let jobs = self.world_textures.jobs(images);
        self.account_deferred(stream, jobs, self.world_textures.staging_bytes(images))?;
        let free = stream.available_external_jobs();
        // Reserve shared work slots before scheduling source image loaders.
        let mut allocation = stream.presentation_allocation();
        allocation.jobs += free;
        let staging_available = stream.available_staging_bytes();
        allocation.staging_bytes += staging_available;
        stream.set_presentation_allocation(allocation)?;
        self.world_deferred.jobs += free;
        self.world_deferred.staging += staging_available;
        let available_cpu = (256 * 1024 * 1024usize)
            .saturating_sub(stream.metrics().prototype_cpu_bytes)
            + self.world_textures.bytes().0;
        let result =
            self.world_textures
                .prepare(server, images, free, staging_available, available_cpu);
        let jobs = self.world_textures.jobs(images);
        self.account_deferred(stream, jobs, self.world_textures.staging_bytes(images))?;
        result
    }
}
