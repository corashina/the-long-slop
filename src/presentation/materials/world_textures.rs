//! Admitted source texture arrays. Each layer retains all authored mip levels.
use crate::presentation::source_images::wrap;
use crate::reference::{LightingTextureRecord, ReferenceError, SourceId, WorldSourceMaterial};
use bevy::asset::AssetId;
use bevy::{
    asset::{LoadState, RenderAssetUsages},
    image::{ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
    render::{render_asset::RenderAssets, render_resource::*, texture::GpuImage},
};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

pub(super) fn bind_arrays(
    result: &mut UnpreparedBindGroup,
    arrays: &[Handle<Image>],
    images: &RenderAssets<GpuImage>,
) -> Result<(), AsBindGroupError> {
    for (index, handle) in arrays.iter().enumerate() {
        let image = images
            .get(handle.id())
            .ok_or(AsBindGroupError::RetryNextUpdate)?;
        result.bindings.0.extend([
            (
                1 + index as u32 * 2,
                OwnedBindingResource::TextureView(
                    TextureViewDimension::D2Array,
                    image.texture_view.clone(),
                ),
            ),
            (
                2 + index as u32 * 2,
                OwnedBindingResource::Sampler(SamplerBindingType::Filtering, image.sampler.clone()),
            ),
        ]);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ArrayKey {
    width: u32,
    height: u32,
    srgb: bool,
    mips: u32,
    filter: u8,
    wrap_u: u8,
    wrap_v: u8,
    aniso: u32,
    bias: u32,
}
impl From<&LightingTextureRecord> for ArrayKey {
    fn from(t: &LightingTextureRecord) -> Self {
        Self {
            width: t.width,
            height: t.height,
            srgb: t.color_space != 0,
            mips: t.mip_count,
            filter: t.filter_mode,
            wrap_u: t.wrap_u,
            wrap_v: t.wrap_v,
            aniso: t.aniso,
            bias: t.mip_bias.to_bits(),
        }
    }
}
impl ArrayKey {
    fn layer_bytes(self) -> Result<usize, ReferenceError> {
        if self.width == 0
            || self.height == 0
            || self.mips == 0
            || self.mips > 32 - self.width.max(self.height).leading_zeros()
        {
            return Err(ReferenceError("Invalid source texture dimensions".into()));
        }
        let bytes: u64 = (0..self.mips)
            .map(|level| {
                u64::from((self.width >> level).max(1))
                    * u64::from((self.height >> level).max(1))
                    * 4
            })
            .sum();
        usize::try_from(bytes).map_err(|_| ReferenceError("Source texture size overflow".into()))
    }
}
#[derive(Clone)]
struct Layer {
    source: SourceId,
    path: String,
    hash: String,
    chunks: Arc<[crate::reference::LightingTextureChunk]>,
    load: Option<Handle<Image>>,
}
#[derive(Clone)]
struct TextureArray {
    owner: Arc<()>,
    failed: bool,
    key: ArrayKey,
    layers: Vec<Layer>,
    image: Option<Handle<Image>>,
    users: usize,
    bytes: usize,
    cpu_bytes: usize,
    cpu_live: bool,
    next_layer: usize,
    next_chunk: usize,
    pending_upload: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct TextureBinding {
    pub array: usize,
    pub layer: u32,
    pub scale: [f32; 2],
    pub offset: [f32; 2],
}
#[derive(Clone)]
struct MaterialTextures {
    arrays: Vec<usize>,
    slots: HashMap<String, TextureBinding>,
}
#[derive(Default, Clone)]
pub struct WorldTextures {
    uploaded: Arc<Mutex<HashSet<AssetId<Image>>>>,
    uploads: Arc<Mutex<super::world_texture_upload::UploadQueue>>,
    arrays: Vec<Option<TextureArray>>,
    materials: HashMap<SourceId, MaterialTextures>,
    /// Live source layer and bounded queue upload overlap.
    cpu_reserved: usize,
    gpu_reserved: usize,
}
impl WorldTextures {
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn startup_audit(
        &self,
        images: &Assets<Image>,
        server: &AssetServer,
    ) -> serde_json::Value {
        let queue = self.uploads.lock().unwrap();
        serde_json::json!({
            "arrays": self.arrays.iter().flatten().map(|a| {
                let layer = a.layers.get(a.next_layer);
                serde_json::json!({
                    "layers": a.layers.len(), "next_layer": a.next_layer,
                    "next_chunk": a.next_chunk, "cpu_live": a.cpu_live,
                    "pending_upload": a.pending_upload, "failed": a.failed,
                    "path": layer.and_then(|l| l.chunks.get(a.next_chunk)).map(|c| &c.data.path),
                    "loaded": layer.and_then(|l| l.load.as_ref()).map(|h| images.contains(h.id())),
                    "load_state": layer.and_then(|l| l.load.as_ref()).map(|h| format!("{:?}", server.load_state(h.id()))),
                })
            }).collect::<Vec<_>>(),
            "completed": queue.completed.is_some(),
            "job": queue.job.as_ref().map(|j| serde_json::json!({
                "finished": j.finished, "submitted": j.submitted,
                "callback_registered": j.callback_registered,
                "completion": j.completion.load(std::sync::atomic::Ordering::Acquire),
            })),
        })
    }

    pub(crate) fn owns_image(&self, id: AssetId<Image>) -> bool {
        self.arrays.iter().flatten().any(|a| {
            Arc::strong_count(&a.owner) == 1 && a.image.as_ref().is_some_and(|h| h.id() == id)
        })
    }

    pub fn drain_ready(&self, server: &AssetServer, images: &Assets<Image>) -> bool {
        self.uploads.lock().unwrap().job.is_none()
            && self.arrays.iter().flatten().all(|a| {
                a.layers.iter().all(|l| {
                    l.load.as_ref().is_none_or(|h| {
                        images.contains(h.id())
                            || matches!(server.load_state(h.id()), LoadState::Failed(_))
                    })
                })
            })
    }
    pub fn release_all(&mut self, images: &mut Assets<Image>) {
        for a in self.arrays.drain(..).flatten() {
            if Arc::strong_count(&a.owner) == 1 {
                if let Some(h) = a.image {
                    self.uploaded.lock().unwrap().remove(&h.id());
                    images.remove(h.id());
                }
            }
        }
        self.materials.clear();
        self.cpu_reserved = 0;
        self.gpu_reserved = 0;
    }

    pub fn settled(&self) -> bool {
        self.arrays.iter().flatten().all(|a| {
            !a.failed && a.image.is_some() && !a.pending_upload && a.next_layer == a.layers.len()
        }) && self.uploads.lock().unwrap().job.is_none()
    }
    pub fn fork(&self) -> Self {
        let mut copy = self.clone();
        copy.uploads = Arc::default();
        copy
    }
    pub fn shared_bytes(&self) -> (usize, usize) {
        self.shared_bytes_if(true)
    }
    pub fn fork_shared_bytes(&self) -> (usize, usize) {
        self.shared_bytes_if(false)
    }
    fn shared_bytes_if(&self, shared: bool) -> (usize, usize) {
        self.arrays
            .iter()
            .flatten()
            .filter(|a| !shared || Arc::strong_count(&a.owner) > 1)
            .fold((0, 0), |n, a| {
                (
                    n.0 + (if a.cpu_live { a.cpu_bytes } else { 0 })
                        + a.layers
                            .iter()
                            .filter(|l| !shared || Arc::strong_count(&l.chunks) > 1)
                            .map(|l| {
                                l.chunks.len()
                                    * std::mem::size_of::<crate::reference::LightingTextureChunk>()
                                    + l.chunks
                                        .iter()
                                        .map(|c| {
                                            c.data.path.capacity()
                                                + c.data.sha256.capacity()
                                                + c.data.source.file.capacity()
                                        })
                                        .sum::<usize>()
                            })
                            .sum::<usize>(),
                    n.1 + a.bytes,
                )
            })
    }

    pub(crate) fn measurement_images(&self) -> impl Iterator<Item = AssetId<Image>> + '_ {
        self.arrays.iter().flatten().flat_map(|a| {
            a.image.iter().map(Handle::id).chain(
                a.layers
                    .iter()
                    .filter_map(|l| l.load.as_ref().map(Handle::id)),
            )
        })
    }

    pub(super) fn upload_queue(&self) -> &Arc<Mutex<super::world_texture_upload::UploadQueue>> {
        &self.uploads
    }

    pub fn upload_receipts(&self) -> Arc<Mutex<HashSet<AssetId<Image>>>> {
        Arc::clone(&self.uploaded)
    }
    pub fn bytes(&self) -> (usize, usize) {
        (self.cpu_reserved + self.metadata_bytes(), self.gpu_reserved)
    }

    fn metadata_bytes(&self) -> usize {
        use std::mem::size_of;
        let arrays = self.arrays.capacity() * size_of::<Option<TextureArray>>()
            + self
                .arrays
                .iter()
                .flatten()
                .map(|array| {
                    4096 + array.layers.capacity() * size_of::<Layer>()
                        + array
                            .layers
                            .iter()
                            .map(|layer| {
                                layer.source.file.capacity()
                                    + layer.path.capacity()
                                    + layer.hash.capacity()
                                    + layer.chunks.len()
                                        * size_of::<crate::reference::LightingTextureChunk>()
                                    + layer
                                        .chunks
                                        .iter()
                                        .map(|c| {
                                            c.data.path.capacity()
                                                + c.data.sha256.capacity()
                                                + c.data.source.file.capacity()
                                        })
                                        .sum::<usize>()
                            })
                            .sum::<usize>()
                })
                .sum::<usize>();
        let materials = map_bytes(
            self.materials.capacity(),
            size_of::<(SourceId, MaterialTextures)>(),
        ) + self
            .materials
            .iter()
            .map(|(id, material)| {
                id.file.capacity()
                    + material.arrays.capacity() * size_of::<usize>()
                    + map_bytes(
                        material.slots.capacity(),
                        size_of::<(String, TextureBinding)>(),
                    )
                    + material.slots.keys().map(String::capacity).sum::<usize>()
            })
            .sum::<usize>();
        size_of::<super::world_texture_upload::UploadQueue>()
            + 4 * size_of::<usize>()
            + size_of::<Mutex<super::world_texture_upload::UploadQueue>>()
            + arrays
            + materials
            + map_bytes(
                self.uploaded.lock().unwrap().capacity(),
                size_of::<AssetId<Image>>(),
            )
    }

    /// Reserve the complete material before starting any asynchronous load.
    pub fn admit(
        &mut self,
        material: &WorldSourceMaterial,
        textures: &[LightingTextureRecord],
        cpu_available: usize,
        gpu_available: usize,
    ) -> Result<(), ReferenceError> {
        if self.materials.contains_key(&material.source) {
            return Ok(());
        }
        let mut groups: Vec<(ArrayKey, Vec<&LightingTextureRecord>)> = Vec::new();
        for link in &material.textures {
            let record = textures
                .iter()
                .find(|t| t.id == link.texture)
                .ok_or_else(|| {
                    ReferenceError(format!("Missing source texture {}", link.texture))
                })?;
            validate_chunks(record)?;
            let key = ArrayKey::from(record);
            let index = groups
                .iter()
                .position(|(old, _)| *old == key)
                .unwrap_or_else(|| {
                    groups.push((key, Vec::new()));
                    groups.len() - 1
                });
            if !groups[index]
                .1
                .iter()
                .any(|old| old.source == record.source)
            {
                groups[index].1.push(record);
            }
        }
        // Source identity, mip hash and exact sampler state define sharing.
        for (_, layers) in &mut groups {
            layers.sort_by(|a, b| {
                (&a.source.file, a.source.path_id).cmp(&(&b.source.file, b.source.path_id))
            });
        }
        let existing: Vec<_> = groups
            .iter()
            .map(|(key, layers)| {
                self.arrays.iter().position(|array| {
                    array.as_ref().is_some_and(|array| {
                        array.key == *key
                            && array.layers.len() == layers.len()
                            && array.layers.iter().zip(layers).all(|(a, b)| {
                                a.source == b.source
                                    && a.hash == b.mip_data.sha256
                                    && a.path == b.mip_data.path
                            })
                    })
                })
            })
            .collect();
        let mut added = 0usize;

        for ((key, layers), existing) in groups.iter().zip(&existing) {
            if existing.is_none() {
                let bytes = key
                    .layer_bytes()?
                    .checked_mul(layers.len())
                    .ok_or_else(|| ReferenceError("Source texture array size overflow".into()))?;
                added = added
                    .checked_add(bytes)
                    .ok_or_else(|| ReferenceError("Source texture allocation overflow".into()))?;
            }
        }
        // Reserve binding metadata. Layer buffers are admitted when loading starts.
        let metadata = groups
            .iter()
            .zip(&existing)
            .filter(|(_, old)| old.is_none())
            .map(|((_, layers), _)| {
                4096 + std::mem::size_of::<TextureArray>() * 4
                    + layers
                        .iter()
                        .map(|layer| {
                            std::mem::size_of::<Layer>()
                                + layer.source.file.len()
                                + layer.mip_data.path.len()
                                + layer.mip_data.sha256.len()
                                + layer.chunks.len()
                                    * std::mem::size_of::<crate::reference::LightingTextureChunk>()
                                + layer
                                    .chunks
                                    .iter()
                                    .map(|c| {
                                        c.data.path.len()
                                            + c.data.sha256.len()
                                            + c.data.source.file.len()
                                    })
                                    .sum::<usize>()
                        })
                        .sum::<usize>()
            })
            .sum::<usize>()
            + map_bytes(
                (self.materials.len() + 1).next_power_of_two() * 2,
                std::mem::size_of::<(SourceId, MaterialTextures)>(),
            )
            + map_bytes(
                material.textures.len().next_power_of_two() * 2,
                std::mem::size_of::<(String, TextureBinding)>(),
            )
            + material
                .textures
                .iter()
                .map(|slot| slot.property.len())
                .sum::<usize>()
            + material.source.file.len()
            + groups.len() * std::mem::size_of::<usize>();
        let cpu = Some(self.cpu_reserved);
        let gpu = added.checked_add(self.gpu_reserved);
        if cpu.is_none_or(|v| {
            v.saturating_add(self.metadata_bytes())
                .saturating_add(metadata)
                > cpu_available
        }) || gpu.is_none_or(|v| v > gpu_available)
        {
            // The material stays unadmitted. Callers wait until retirement frees budget.
            return Ok(());
        }
        self.cpu_reserved = cpu.unwrap();
        self.gpu_reserved = gpu.unwrap();
        let mut indices = Vec::with_capacity(groups.len());
        for ((key, records), existing) in groups.into_iter().zip(existing) {
            let index = if let Some(index) = existing {
                self.arrays[index].as_mut().unwrap().users += 1;
                index
            } else {
                let bytes = key.layer_bytes()? * records.len();
                // Fetch buffer, reader, shared asset, upload payload, and GPU upload copy.
                let cpu_bytes = 5 * (super::world_texture_upload::UPLOAD_BYTES + 16);
                let layers = records
                    .into_iter()
                    .map(|record| Layer {
                        source: record.source.clone(),
                        path: record.mip_data.path.clone(),
                        hash: record.mip_data.sha256.clone(),
                        chunks: record.chunks.clone().into(),
                        load: None,
                    })
                    .collect();
                let array = Some(TextureArray {
                    owner: Arc::new(()),
                    failed: false,
                    key,
                    layers,
                    image: None,
                    users: 1,
                    bytes,
                    cpu_bytes,
                    cpu_live: false,
                    next_layer: 0,
                    next_chunk: 0,
                    pending_upload: false,
                });
                if let Some(index) = self.arrays.iter().position(Option::is_none) {
                    self.arrays[index] = array;
                    index
                } else {
                    self.arrays.push(array);
                    self.arrays.len() - 1
                }
            };
            indices.push(index);
        }
        let mut slots = HashMap::with_capacity(material.textures.len());
        for link in &material.textures {
            let record = textures.iter().find(|t| t.id == link.texture).unwrap();
            let (array, layer) = indices
                .iter()
                .find_map(|&index| {
                    self.arrays[index]
                        .as_ref()
                        .unwrap()
                        .layers
                        .iter()
                        .position(|l| l.source == record.source)
                        .map(|layer| (index, layer))
                })
                .unwrap();
            slots.insert(
                link.property.clone(),
                TextureBinding {
                    array,
                    layer: layer as u32,
                    scale: link.scale,
                    offset: link.offset,
                },
            );
        }
        self.materials.insert(
            material.source.clone(),
            MaterialTextures {
                arrays: indices,
                slots,
            },
        );
        Ok(())
    }
    /// Admit one source layer, then upload at most four MiB per render update.
    pub fn prepare(
        &mut self,
        server: &AssetServer,
        images: &mut Assets<Image>,
        new_jobs: usize,
        staging_available: usize,
        cpu_available: usize,
    ) -> Result<(), ReferenceError> {
        let result = self.prepare_inner(server, images, new_jobs, staging_available, cpu_available);
        loop {
            let failed = self
                .materials
                .iter()
                .find(|(id, material)| {
                    material
                        .arrays
                        .iter()
                        .any(|&i| self.arrays[i].as_ref().is_some_and(|a| a.failed))
                        && self.retirable(id, images, server)
                })
                .map(|(id, _)| id.clone());
            let Some(id) = failed else {
                break;
            };
            self.release(&id, images, server)?;
        }
        result
    }
    fn prepare_inner(
        &mut self,
        server: &AssetServer,
        images: &mut Assets<Image>,
        new_jobs: usize,
        staging_available: usize,
        cpu_available: usize,
    ) -> Result<(), ReferenceError> {
        if let Some((image, layer, chunk)) = self.uploads.lock().unwrap().completed.take() {
            if let Some(array) = self
                .arrays
                .iter_mut()
                .flatten()
                .find(|a| a.image.as_ref().is_some_and(|h| h.id() == image))
            {
                if array.pending_upload
                    && array.next_layer == layer as usize
                    && array.next_chunk == chunk as usize
                {
                    array.pending_upload = false;
                    // Drop only this consumer. AssetServer owns shared chunk retirement.
                    array.layers[array.next_layer].load = None;
                    array.next_chunk += 1;
                    if array.next_chunk == array.layers[array.next_layer].chunks.len() {
                        array.next_layer += 1;
                        array.next_chunk = 0;
                    }
                    self.cpu_reserved -= array.cpu_bytes;
                    array.cpu_live = false;
                }
            }
        }
        // Several arrays fetch and decode chunks at once. The upload slot stays serial.
        let (mut jobs, mut staging) = (new_jobs, staging_available);
        for index in 0..self.arrays.len() {
            let Some(array) = self.arrays[index].as_ref() else {
                continue;
            };
            if array.failed || array.next_layer == array.layers.len() {
                continue;
            }
            let loading = array.cpu_live;
            self.step(index, server, images, jobs, staging, cpu_available)?;
            let array = self.arrays[index].as_ref().unwrap();
            if !loading && array.cpu_live {
                jobs -= 1;
                staging -= array.cpu_bytes;
            }
        }
        Ok(())
    }
    fn step(
        &mut self,
        index: usize,
        server: &AssetServer,
        images: &mut Assets<Image>,
        new_jobs: usize,
        staging_available: usize,
        cpu_available: usize,
    ) -> Result<(), ReferenceError> {
        let array = self.arrays[index].as_mut().unwrap();
        if array.pending_upload {
            return Ok(());
        }
        if array.failed {
            return Ok(());
        }
        let key = array.key;
        let layer = &mut array.layers[array.next_layer];
        let chunk = &layer.chunks[array.next_chunk];
        let (width, rows, mip, row) = (chunk.width, chunk.rows, chunk.mip, chunk.row);
        if let Some(handle) = &layer.load {
            if let LoadState::Failed(error) = server.load_state(handle.id()) {
                array.failed = true;
                layer.load = None;
                self.cpu_reserved -= array.cpu_bytes;
                array.cpu_live = false;
                return Err(ReferenceError(format!(
                    "Source texture {}:{}: {error}",
                    layer.source.file, layer.source.path_id
                )));
            }
            let Some(image) = images.get(handle) else {
                return Ok(());
            };
            if image.width() != width
                || image.height() != rows
                || image.texture_descriptor.mip_level_count != 1
                || image.data.as_ref().map(Vec::len) != Some(width as usize * rows as usize * 4)
                || !matches!(
                    image.texture_descriptor.format,
                    TextureFormat::Rgba8Unorm | TextureFormat::Rgba8UnormSrgb
                )
            {
                array.failed = true;
                layer.load = None;
                return Err(ReferenceError("Invalid source array layer".into()));
            }
            let mut queue = self.uploads.lock().unwrap();
            // Another array owns the serial upload slot.
            if queue.job.is_some() || queue.completed.is_some() {
                return Ok(());
            }
            if array.image.is_none() {
                array.image = Some(images.add(array_image(key, array.layers.len())?));
            }
            // A chunk path may serve several arrays or an overlapping Restart.
            // Retain its strong handle through completion; never remove its shared asset.
            let data = images
                .get(array.layers[array.next_layer].load.as_ref().unwrap())
                .ok_or_else(|| ReferenceError("Source layer ownership disappeared".into()))?
                .data
                .as_ref()
                .unwrap()
                .clone();
            queue.job = Some(super::world_texture_upload::Upload {
                image: array.image.as_ref().unwrap().clone(),
                layer: array.next_layer as u32,
                width,
                height: rows,
                chunk: array.next_chunk as u32,
                data,
                mip,
                row,
                finished: false,
                submitted: false,
                callback_registered: false,
                completion: Arc::new(std::sync::atomic::AtomicBool::new(true)),
            });
            array.pending_upload = true;
            return Ok(());
        }
        if new_jobs == 0 {
            return Ok(());
        }
        if array.cpu_bytes > 32 * 1024 * 1024 {
            return Err(ReferenceError(
                "Source layer and upload overlap exceed shared staging cap".into(),
            ));
        }
        if array.cpu_bytes > staging_available
            || self
                .cpu_reserved
                .saturating_add(array.cpu_bytes)
                .saturating_add(self.metadata_bytes())
                > cpu_available
        {
            return Ok(());
        }
        let array = self.arrays[index].as_mut().unwrap();
        self.cpu_reserved += array.cpu_bytes;
        array.cpu_live = true;
        array.layers[array.next_layer].load = Some(
            server
                .load_builder()
                .with_settings(
                    move |settings: &mut crate::presentation::textures::SourceImageSettings| {
                        settings.srgb = key.srgb;
                        settings.filter = key.filter;
                        settings.wrap_u = key.wrap_u;
                        settings.wrap_v = key.wrap_v;
                        settings.main_world_only = true;
                        settings.decoded_bytes = width as usize * rows as usize * 4;
                    },
                )
                .load(
                    array.layers[array.next_layer].chunks[array.next_chunk]
                        .data
                        .path
                        .clone(),
                ),
        );
        Ok(())
    }
    pub fn jobs(&self, images: &Assets<Image>) -> usize {
        self.arrays
            .iter()
            .flatten()
            .flat_map(|a| &a.layers)
            .filter_map(|l| l.load.as_ref())
            .filter(|h| !images.contains(h.id()))
            .count()
    }
    pub fn staging_bytes(&self, _images: &Assets<Image>) -> usize {
        self.cpu_reserved
    }
    pub fn admitted(&self, material: &SourceId) -> bool {
        self.materials.contains_key(material)
    }
    pub fn sampling(&self, array: usize) -> Option<Vec4> {
        let key = self.arrays.get(array)?.as_ref()?.key;
        Some(Vec4::new(
            f32::from_bits(key.bias),
            (key.wrap_u == 3) as u8 as f32,
            (key.wrap_v == 3) as u8 as f32,
            (key.mips - 1) as f32,
        ))
    }
    pub fn binding(
        &self,
        material: &SourceId,
        property: &str,
    ) -> Option<(TextureBinding, &Handle<Image>)> {
        let binding = *self.materials.get(material)?.slots.get(property)?;
        Some((
            binding,
            self.arrays[binding.array].as_ref()?.image.as_ref()?,
        ))
    }
    pub fn ready(&self, material: &SourceId) -> bool {
        self.materials.get(material).is_some_and(|m| {
            m.arrays.iter().all(|&i| {
                self.arrays[i]
                    .as_ref()
                    .is_some_and(|a| a.next_layer == a.layers.len())
            })
        })
    }
    fn upload_in_flight(&self, array: &TextureArray) -> bool {
        self.uploads
            .lock()
            .unwrap()
            .job
            .as_ref()
            .is_some_and(|job| {
                array
                    .image
                    .as_ref()
                    .is_some_and(|image| image.id() == job.image.id())
                    && job.submitted
                    && !job.completion.load(std::sync::atomic::Ordering::Acquire)
            })
    }
    pub fn retirable(
        &self,
        material: &SourceId,
        images: &Assets<Image>,
        server: &AssetServer,
    ) -> bool {
        self.materials.get(material).is_none_or(|m| {
            m.arrays.iter().all(|&i| {
                self.arrays[i].as_ref().is_none_or(|a| {
                    a.users > 1
                        || (!self.upload_in_flight(a)
                            && a.layers.iter().all(|l| {
                                l.load.as_ref().is_none_or(|h| {
                                    images.contains(h.id())
                                        || matches!(server.load_state(h.id()), LoadState::Failed(_))
                                })
                            }))
                })
            })
        })
    }
    pub fn release(
        &mut self,
        material: &SourceId,
        images: &mut Assets<Image>,
        server: &AssetServer,
    ) -> Result<(), ReferenceError> {
        if !self.retirable(material, images, server) {
            return Err(ReferenceError(
                "Source texture retirement still has pending work".into(),
            ));
        }
        if self.materials.get(material).is_some_and(|binding| {
            binding.arrays.iter().any(|&index| {
                self.arrays[index].as_ref().is_some_and(|array| {
                    array.users == 1
                        && array.layers.iter().any(|layer| {
                            layer.load.as_ref().is_some_and(|handle| {
                                !images.contains(handle.id())
                                    && !matches!(
                                        server.load_state(handle.id()),
                                        LoadState::Failed(_)
                                    )
                            })
                        })
                })
            })
        }) {
            return Err(ReferenceError(
                "Source texture retirement still has a loader job".into(),
            ));
        }
        let Some(material) = self.materials.remove(material) else {
            return Ok(());
        };
        for index in material.arrays {
            let array = self.arrays[index].as_mut().unwrap();
            array.users -= 1;
            if array.users > 0 {
                continue;
            }
            let array = self.arrays[index].take().unwrap();
            if let Some(image) = array.image.filter(|_| Arc::strong_count(&array.owner) == 1) {
                let mut queue = self.uploads.lock().unwrap();
                if queue
                    .job
                    .as_ref()
                    .is_some_and(|job| job.image.id() == image.id())
                {
                    queue.job = None;
                }
                if queue
                    .completed
                    .as_ref()
                    .is_some_and(|(id, _, _)| *id == image.id())
                {
                    queue.completed = None;
                }
                self.uploaded.lock().unwrap().remove(&image.id());
                images.remove(image.id());
            }
            if array.cpu_live {
                self.cpu_reserved -= array.cpu_bytes;
            }
            // Dropping layer handles releases this owner without invalidating other users.
            drop(array.layers);
            self.gpu_reserved -= array.bytes;
        }
        Ok(())
    }
}

// Hash-table allocation upper bound from capacity, including control bytes.
fn map_bytes(capacity: usize, entry: usize) -> usize {
    if capacity == 0 {
        0
    } else {
        capacity
            .saturating_mul(2)
            .saturating_mul(entry + 1)
            .saturating_add(16)
    }
}

fn array_image(key: ArrayKey, layers: usize) -> Result<Image, ReferenceError> {
    let mut image = Image::new_uninit(
        Extent3d {
            width: key.width,
            height: key.height,
            depth_or_array_layers: layers as u32,
        },
        TextureDimension::D2,
        if key.srgb {
            TextureFormat::Rgba8UnormSrgb
        } else {
            TextureFormat::Rgba8Unorm
        },
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.mip_level_count = key.mips;
    image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST;
    image.texture_view_descriptor = Some(TextureViewDescriptor {
        dimension: Some(TextureViewDimension::D2Array),
        ..default()
    });
    let filter = if key.filter == 0 {
        ImageFilterMode::Nearest
    } else {
        ImageFilterMode::Linear
    };
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: wrap(key.wrap_u),
        address_mode_v: wrap(key.wrap_v),
        mag_filter: filter,
        min_filter: filter,
        mipmap_filter: if key.filter == 2 || key.aniso > 1 {
            ImageFilterMode::Linear
        } else {
            ImageFilterMode::Nearest
        },
        anisotropy_clamp: u16::try_from(key.aniso.max(1))
            .map_err(|_| ReferenceError("Source anisotropy exceeds GPU sampler range".into()))?,
        lod_max_clamp: (key.mips - 1) as f32,
        ..default()
    });
    Ok(image)
}

fn validate_chunks(texture: &LightingTextureRecord) -> Result<(), ReferenceError> {
    let mut index = 0;
    for mip in 0..texture.mip_count {
        let width = (texture.width >> mip).max(1);
        let height = (texture.height >> mip).max(1);
        let mut row = 0;
        while row < height {
            let chunk = texture
                .chunks
                .get(index)
                .ok_or_else(|| ReferenceError("Missing source texture chunk".into()))?;
            if chunk.mip != mip
                || chunk.row != row
                || chunk.width != width
                || chunk.rows == 0
                || chunk.rows > height - row
                || u64::from(width) * u64::from(chunk.rows) * 4
                    > super::world_texture_upload::UPLOAD_BYTES as u64
                || chunk.data.source != texture.source
            {
                return Err(ReferenceError(
                    "Invalid source texture chunk coverage".into(),
                ));
            }
            row += chunk.rows;
            index += 1;
        }
    }
    if index != texture.chunks.len() {
        return Err(ReferenceError("Extra source texture chunks".into()));
    }
    Ok(())
}
