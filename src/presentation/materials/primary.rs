//! Dynamic primary materials share the accepted source material cache.
use super::*;
use crate::{
    reference::{AssetId as SourceAssetId, ReferenceError},
    world::WorldStream,
};
pub(super) fn png_staging(mip: &source::TextureMip) -> Result<usize, ReferenceError> {
    let encoded = crate::presentation::world::preflight::checked_input_bytes(&mip.file)?;
    // Input copies plus decoder buffer growth, chunk input, scanlines, and inflate state.
    crate::reference::fixed_reservation::png_staging(encoded, mip.width)
}
#[derive(Default, Clone)]
pub struct PrimaryOwnership {
    pub(super) assets: HashMap<SourceAssetId, Vec<String>>,
    owners: HashMap<SourceAssetId, Arc<()>>,
    pub(super) borrowed_records: std::collections::HashSet<SourceAssetId>,
    cpu: usize,
    gpu: usize,
    jobs: usize,
    pending: usize,
    staging: usize,
}
fn pixels(texture: &TextureRecord) -> Result<usize, ReferenceError> {
    crate::reference::fixed_reservation::png_pixels(&texture.mips)
}
fn texture_cpu_reservation(texture: &SourceTexture) -> usize {
    texture.cpu_reservation
}
pub(super) fn pending_texture_cpu(texture: &SourceTexture) -> usize {
    texture.upload.cpu_bytes(
        texture_cpu_reservation(texture),
        !texture.levels.is_empty() || texture.image.is_some(),
    )
}
pub(super) fn texture_metadata(texture: &TextureRecord) -> usize {
    use std::mem::size_of;
    4096 + size_of::<TextureRecord>()
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
pub(super) fn record_bytes(record: &MaterialRecord) -> usize {
    record.retained_cpu_bytes()
}

impl SourceMaterials {
    pub fn begin_world_session(&mut self) {
        // Resources stay owned. The new stream receives their full charges on preparation.
        self.world_primary.cpu = 0;
        self.world_primary.gpu = 0;
        self.world_primary.jobs = 0;
        self.world_primary.pending = 0;
        self.world_primary.staging = 0;
        self.world_deferred = DeferredAllocation::default();
    }

    fn container_bytes(&self) -> usize {
        use std::mem::size_of;
        self.sort_data.capacity()
            * (size_of::<(
                (SourceAssetId, u32),
                super::super::transparent::SourceSortData,
            )>() + 1)
            + self.records.capacity() * size_of::<Arc<MaterialRecord>>()
            + self.textures.capacity() * size_of::<SourceTexture>()
            + self
                .textures
                .iter()
                .map(|t| {
                    t.label.capacity()
                        + t.levels.capacity() * size_of::<Handle<Image>>()
                        + t.input_staging.capacity() * size_of::<usize>()
                })
                .sum::<usize>()
            + self.texture_indices.capacity() * (size_of::<(String, usize)>() + 1)
            + self
                .texture_indices
                .keys()
                .map(String::capacity)
                .sum::<usize>()
            + self.handles.capacity()
                * (size_of::<((SourceAssetId, u32, bool, u32), Handle<SourceMaterial>)>() + 1)
            + self.standard_records.capacity()
                * (size_of::<(AssetId<StandardMaterial>, (SourceAssetId, u32))>() + 1)
            + self.world_primary.assets.capacity() * (size_of::<(SourceAssetId, Vec<String>)>() + 1)
            + self
                .world_primary
                .assets
                .values()
                .map(|paths| {
                    paths.capacity() * size_of::<String>()
                        + paths.iter().map(String::capacity).sum::<usize>()
                })
                .sum::<usize>()
            + self.world_primary.owners.capacity() * (size_of::<(SourceAssetId, Arc<()>)>() + 1)
            + self.world_primary.borrowed_records.capacity() * (size_of::<SourceAssetId>() + 1)
    }
    pub(super) fn reserve_fork(&self, stream: &mut WorldStream) -> Result<(), ReferenceError> {
        let (cpu, gpu) = self.primary_bytes();
        let (deferred_cpu, deferred_gpu) = self.world_textures.bytes();
        let mut allocation = stream.presentation_allocation();
        allocation.prototype_cpu += cpu + deferred_cpu;
        allocation.prototype_gpu += gpu + deferred_gpu;
        stream.set_presentation_allocation(allocation)
    }
    pub(super) fn account_fork(&mut self, _stream: &mut WorldStream) -> Result<(), ReferenceError> {
        let (cpu, gpu) = self.primary_bytes();
        self.world_primary.cpu = cpu;
        self.world_primary.gpu = gpu;
        let (cpu, gpu) = self.world_textures.bytes();
        self.world_deferred.cpu = cpu;
        self.world_deferred.gpu = gpu;
        Ok(())
    }
    fn primary_bytes(&self) -> (usize, usize) {
        let cpu = self
            .textures
            .iter()
            .map(|t| pending_texture_cpu(t) + texture_metadata(&t.record))
            .sum::<usize>()
            + self
                .records
                .iter()
                .map(|record| record_bytes(record))
                .sum::<usize>();
        let gpu = self
            .textures
            .iter()
            .filter(|t| !t.failed || t.image.is_some())
            .map(|t| t.pixel_bytes)
            .sum();
        (cpu + self.container_bytes(), gpu)
    }
    fn account_primary(
        &mut self,
        stream: &mut WorldStream,
        cpu: usize,
        gpu: usize,
        jobs: usize,
        pending: usize,
        staging: usize,
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
        allocation.pending = allocation
            .pending
            .saturating_sub(self.world_primary.pending)
            + pending;
        allocation.staging_bytes = allocation
            .staging_bytes
            .saturating_sub(self.world_primary.staging)
            + staging;
        allocation.dependency_staging_bytes = allocation
            .dependency_staging_bytes
            .saturating_sub(self.world_primary.staging)
            + staging;
        stream.set_presentation_allocation(allocation)?;
        self.world_primary.cpu = cpu;
        self.world_primary.gpu = gpu;
        self.world_primary.jobs = jobs;
        self.world_primary.pending = pending;
        self.world_primary.staging = staging;
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
                    let bytes = pixels(texture)?;
                    added_gpu += bytes;
                    added_cpu += 4096
                        + std::mem::size_of::<SourceTexture>()
                        + texture.file.len() * 2
                        + texture.source.file.len()
                        + texture.mips.len()
                            * (std::mem::size_of::<source::TextureMip>()
                                + std::mem::size_of::<usize>())
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
            self.world_primary.pending,
            self.world_primary.staging,
        )?;
        for record in records {
            for texture in &record.properties.textures {
                if !self.texture_indices.contains_key(&texture.file) {
                    let input_staging = texture
                        .mips
                        .iter()
                        .map(png_staging)
                        .collect::<Result<Vec<_>, _>>()?;
                    let pixel_bytes = pixels(texture)?;
                    let cpu_reservation = crate::reference::fixed_reservation::png_cpu(
                        pixel_bytes,
                        input_staging.iter().copied().max().unwrap_or(0),
                    )?;
                    self.texture_indices
                        .insert(texture.file.clone(), self.textures.len());
                    self.textures.push(SourceTexture {
                        owner: Arc::new(()),
                        upload: default(),
                        failed: false,
                        persistent: false,
                        users: 0,
                        record: Arc::new(texture.clone()),
                        levels: Vec::with_capacity(texture.mips.len()),
                        input_staging,
                        pixel_bytes,
                        cpu_reservation,
                        image: None,
                        label: format!(
                            "source material {}:{} {}",
                            record.source.file, record.source.path_id, texture.slot
                        ),
                    });
                }
            }
            if !borrowed {
                self.records.push(Arc::new(record.clone()));
            }
        }
        for path in &paths {
            self.textures[self.texture_indices[path]].users += 1;
        }
        if borrowed {
            self.world_primary.borrowed_records.insert(asset);
        }
        self.world_primary.assets.insert(asset, paths);
        self.world_primary.owners.insert(asset, Arc::new(()));
        Ok(())
    }
    pub fn world_asset_ready(&self, asset: SourceAssetId) -> bool {
        self.world_primary.assets.get(&asset).is_some_and(|paths| {
            paths.iter().all(|path| {
                let texture = &self.textures[self.texture_indices[path]];
                texture.image.is_some() && pending_texture_cpu(texture) == 0
            })
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
        for texture in self.textures.iter_mut().filter(|t| t.image.is_none()) {
            for handle in &texture.levels {
                if let LoadState::Failed(error) = server.load_state(handle.id()) {
                    if !texture.failed {
                        failure = Some(ReferenceError(format!("{}: {error}", texture.label)));
                    }
                    texture.failed = true;
                }
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
        // Failed fixed chains retain their MAIN_WORLD mip handles and conservative CPU charge.
        // Dropping handles alone cannot prove that Bevy has removed their pixel assets.
        let mut jobs = 0;
        let mut staging = 0;
        for texture in self.textures.iter().filter(|t| t.image.is_none()) {
            for (handle, bytes) in texture.levels.iter().zip(&texture.input_staging) {
                if !images.contains(handle.id())
                    && !matches!(server.load_state(handle.id()), LoadState::Failed(_))
                {
                    jobs += 1;
                    staging += bytes;
                }
            }
        }
        let (cpu, gpu) = self.primary_bytes();
        self.account_primary(stream, cpu, gpu, jobs, self.pending_primary(), staging)?;
        for index in 0..self.textures.len() {
            if stream.available_external_jobs() == 0 {
                break;
            }
            let texture = &self.textures[index];
            if texture.failed
                || (texture.persistent && !self.fixed_admitted)
                || texture.image.is_some()
                || texture.levels.len() == texture.record.mips.len()
            {
                continue;
            }
            if texture.levels.is_empty() && stream.metrics().pending_work >= 32 {
                continue;
            }
            let first = texture.levels.is_empty();
            let staging = texture.input_staging[texture.levels.len()];
            if staging > 32 * 1024 * 1024 {
                return Err(ReferenceError(
                    "PNG input overlap exceeds staging cap".into(),
                ));
            }
            if staging > stream.available_dependency_staging_bytes() {
                continue;
            }
            let decoded = if texture.levels.is_empty() {
                texture_cpu_reservation(texture)
            } else {
                0
            };
            if decoded > stream.available_prototype_cpu_bytes()
                && (self.textures.iter().any(|t| pending_texture_cpu(t) != 0)
                    || self.world_primary.jobs < stream.presentation_allocation().jobs)
            {
                // Let admitted decodes and GPU uploads drain before starting another texture.
                continue;
            }
            self.account_primary(
                stream,
                self.world_primary.cpu + decoded,
                gpu,
                self.world_primary.jobs + 1,
                self.pending_primary() + usize::from(first),
                self.world_primary.staging + staging,
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
        let assembled = assemble_textures(&mut self.textures, server, images);
        if let Ok(bytes) = &assembled {
            self.texture_bytes += bytes;
        }
        let mut jobs = 0;
        let mut staging = 0;
        for texture in self.textures.iter().filter(|t| t.image.is_none()) {
            for (handle, bytes) in texture.levels.iter().zip(&texture.input_staging) {
                if !images.contains(handle.id())
                    && !matches!(server.load_state(handle.id()), LoadState::Failed(_))
                {
                    jobs += 1;
                    staging += bytes;
                }
            }
        }
        let (cpu, gpu) = self.primary_bytes();
        self.account_primary(stream, cpu, gpu, jobs, self.pending_primary(), staging)?;
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
                if t.image.is_some() {
                    pending_texture_cpu(t) == 0
                } else {
                    t.levels.iter().all(|h| {
                        images.contains(h.id())
                            || matches!(server.load_state(h.id()), LoadState::Failed(_))
                    })
                }
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
        let unique = self
            .world_primary
            .owners
            .remove(&asset)
            .is_none_or(|owner| Arc::strong_count(&owner) == 1);
        let paths = self.world_primary.assets.remove(&asset).unwrap();
        for path in paths {
            self.textures[self.texture_indices[&path]].users -= 1;
        }
        // Startup overrides can borrow one of the five imported material assets.
        // Their cache handles and records remain owned by the imported scene.
        if !self.world_primary.borrowed_records.remove(&asset) {
            self.handles.retain(|key, handle| {
                if key.0 == asset {
                    if unique {
                        materials.remove(handle.id());
                    }
                    false
                } else {
                    true
                }
            });
            self.records.retain(|r| r.asset != asset);
        }
        self.textures.retain(|texture| {
            if !texture.persistent && texture.users == 0 {
                if Arc::strong_count(&texture.owner) == 1 {
                    for level in &texture.levels {
                        images.remove(level.id());
                    }
                    if let Some(image) = &texture.image {
                        images.remove(image.id());
                    }
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

#[derive(Default, Clone)]
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
        allocation.dependency_staging_bytes = allocation
            .dependency_staging_bytes
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
            stream.available_prototype_cpu_bytes() + cpu,
            stream.available_prototype_gpu_bytes() + gpu,
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
        let staging_available = stream.available_dependency_staging_bytes();
        allocation.staging_bytes += staging_available;
        allocation.dependency_staging_bytes += staging_available;
        stream.set_presentation_allocation(allocation)?;
        self.world_deferred.jobs += free;
        self.world_deferred.staging += staging_available;
        let available_cpu = stream.available_prototype_cpu_bytes() + self.world_textures.bytes().0;
        let result =
            self.world_textures
                .prepare(server, images, free, staging_available, available_cpu);
        let jobs = self.world_textures.jobs(images);
        self.account_deferred(stream, jobs, self.world_textures.staging_bytes(images))?;
        result
    }
}

impl PrimaryOwnership {
    pub fn asset_ids(&self) -> impl Iterator<Item = SourceAssetId> + '_ {
        self.assets.keys().copied()
    }
}

impl SourceMaterials {
    pub(super) fn select_lighting_textures(
        &mut self,
        reference: &ReferenceManifest,
    ) -> Result<(), ReferenceError> {
        for t in &reference.render_lighting.textures {
            let file = &t.mips[0].file;
            if let Some(&index) = self.texture_indices.get(file) {
                let old = &self.textures[index].record;
                if old.source != t.source
                    || old.color_space != t.color_space
                    || old.sampler.filter != t.sampler.filter
                    || old.sampler.wrap_u != t.sampler.wrap_u
                    || old.sampler.wrap_v != t.sampler.wrap_v
                    || old.sampler.aniso != t.sampler.aniso
                    || old.sampler.mip_bias.to_bits() != t.sampler.mip_bias.to_bits()
                    || old.sampler.mip_count != t.sampler.mip_count
                    || old.mips.len() != t.mips.len()
                    || !old.mips.iter().zip(&t.mips).all(|(a, b)| {
                        a.file == b.file && a.width == b.width && a.height == b.height
                    })
                {
                    return Err(ReferenceError(
                        "Conflicting persistent lighting sampler".into(),
                    ));
                }
                self.textures[index].persistent = true;
                continue;
            }
            let record = TextureRecord {
                slot: String::new(),
                source: t.source.clone(),
                file: file.clone(),
                color_space: t.color_space,
                encoding: source::TextureEncoding::Rgba,
                source_encoding: source::TextureEncoding::Rgba,
                uv_channel: 0,
                scale: [1., 1.],
                offset: [0., 0.],
                sampler: t.sampler.clone(),
                mips: t.mips.clone(),
            };
            let input_staging = t
                .mips
                .iter()
                .map(png_staging)
                .collect::<Result<Vec<_>, _>>()?;
            let pixel_bytes = pixels(&record)?;
            let cpu_reservation = crate::reference::fixed_reservation::png_cpu(
                pixel_bytes,
                input_staging.iter().copied().max().unwrap_or(0),
            )?;
            self.texture_indices
                .insert(file.clone(), self.textures.len());
            self.textures.push(SourceTexture {
                owner: Arc::new(()),
                upload: default(),
                failed: false,
                persistent: true,
                users: 0,
                record: Arc::new(record),
                levels: Vec::with_capacity(t.mips.len()),
                input_staging,
                pixel_bytes,
                cpu_reservation,
                image: None,
                label: file.clone(),
            });
        }
        Ok(())
    }
    pub(crate) fn cancel_fixed(&mut self) {
        self.fixed_admitted = false;
        for texture in self
            .textures
            .iter_mut()
            .filter(|t| t.persistent && t.image.is_none())
        {
            texture.failed = true;
        }
    }
    fn pending_primary(&self) -> usize {
        self.textures
            .iter()
            .filter(|t| pending_texture_cpu(t) > 0)
            .count()
    }
    pub(crate) fn fixed_textures_ready(&self) -> bool {
        self.fixed_admitted
            && self
                .textures
                .iter()
                .filter(|t| t.persistent)
                .all(|t| t.image.is_some() && t.upload.completed())
    }
}
