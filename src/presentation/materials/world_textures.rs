//! Admitted source texture arrays. Each layer retains all authored mip levels.
use crate::reference::{LightingTextureRecord, ReferenceError, SourceId, WorldSourceMaterial};
use bevy::asset::AssetId;
use bevy::{
    asset::{LoadState, RenderAssetUsages},
    image::{ImageFilterMode, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
    render::render_resource::*,
};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

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
struct Layer {
    source: SourceId,
    path: String,
    hash: String,
    load: Option<Handle<Image>>,
    staging_id: Option<AssetId<Image>>,
}
struct TextureArray {
    failed: bool,
    key: ArrayKey,
    layers: Vec<Layer>,
    image: Option<Handle<Image>>,
    users: usize,
    bytes: usize,
    cpu_bytes: usize,
    cpu_live: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct TextureBinding {
    pub array: usize,
    pub layer: u32,
    pub scale: [f32; 2],
    pub offset: [f32; 2],
}
struct MaterialTextures {
    arrays: Vec<usize>,
    slots: HashMap<String, TextureBinding>,
}
#[derive(Default)]
pub struct WorldTextures {
    uploaded: Arc<Mutex<HashSet<AssetId<Image>>>>,
    arrays: Vec<Option<TextureArray>>,
    materials: HashMap<SourceId, MaterialTextures>,
    /// Reserved decoded image bytes. Includes the array and its source staging images.
    cpu_reserved: usize,
    gpu_reserved: usize,
}
impl WorldTextures {
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
                    array.layers.capacity() * size_of::<Layer>()
                        + array
                            .layers
                            .iter()
                            .map(|layer| {
                                layer.source.file.capacity()
                                    + layer.path.capacity()
                                    + layer.hash.capacity()
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
        arrays
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
        let mut staging_headers = 0usize;
        for ((key, layers), existing) in groups.iter().zip(&existing) {
            if existing.is_none() {
                staging_headers += layers.len() * 16;
                let bytes = key
                    .layer_bytes()?
                    .checked_mul(layers.len())
                    .ok_or_else(|| ReferenceError("Source texture array size overflow".into()))?;
                added = added
                    .checked_add(bytes)
                    .ok_or_else(|| ReferenceError("Source texture allocation overflow".into()))?;
            }
        }
        // Reserve both decoded buffers, SMIP headers, and new binding metadata.
        let metadata = groups
            .iter()
            .zip(&existing)
            .filter(|(_, old)| old.is_none())
            .map(|((_, layers), _)| {
                std::mem::size_of::<TextureArray>() * 4
                    + layers
                        .iter()
                        .map(|layer| {
                            std::mem::size_of::<Layer>()
                                + layer.source.file.len()
                                + layer.mip_data.path.len()
                                + layer.mip_data.sha256.len()
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
        let cpu = added
            .checked_mul(2)
            .and_then(|v| v.checked_add(staging_headers))
            .and_then(|v| v.checked_add(self.cpu_reserved));
        let gpu = added.checked_add(self.gpu_reserved);
        if cpu.is_none_or(|v| {
            v.saturating_add(self.metadata_bytes())
                .saturating_add(metadata)
                > cpu_available
        }) || gpu.is_none_or(|v| v > gpu_available)
        {
            return Err(ReferenceError(format!(
                "Source texture budget {}:{}",
                material.source.file, material.source.path_id
            )));
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
                let cpu_bytes = bytes * 2 + records.len() * 16;
                let layers = records
                    .into_iter()
                    .map(|record| Layer {
                        source: record.source.clone(),
                        path: record.mip_data.path.clone(),
                        hash: record.mip_data.sha256.clone(),
                        load: None,
                        staging_id: None,
                    })
                    .collect();
                let array = Some(TextureArray {
                    failed: false,
                    key,
                    layers,
                    image: None,
                    users: 1,
                    bytes,
                    cpu_bytes,
                    cpu_live: true,
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
    /// Assemble one completed array per update. Never generate missing mip pixels.
    pub fn prepare(
        &mut self,
        server: &AssetServer,
        images: &mut Assets<Image>,
        new_jobs: usize,
        mut staging_available: usize,
    ) -> Result<(), ReferenceError> {
        {
            let uploaded = self.uploaded.lock().unwrap();
            for array in self.arrays.iter_mut().flatten() {
                if array.cpu_live
                    && array
                        .layers
                        .iter()
                        .all(|layer| layer.staging_id.is_none_or(|id| !images.contains(id)))
                    && array
                        .image
                        .as_ref()
                        .is_some_and(|image| uploaded.contains(&image.id()))
                {
                    self.cpu_reserved -= array.cpu_bytes;
                    array.cpu_live = false;
                }
            }
        }
        let mut failure = None;
        for array in self.arrays.iter_mut().flatten() {
            for layer in &mut array.layers {
                if let Some(handle) = &layer.load {
                    if let LoadState::Failed(error) = server.load_state(handle.id()) {
                        if !array.failed {
                            failure = Some(ReferenceError(format!(
                                "Source texture {}:{}: {error}",
                                layer.source.file, layer.source.path_id
                            )));
                        }
                        array.failed = true;
                        images.remove(handle.id());
                        layer.load = None;
                    }
                }
            }
        }
        let retired: Vec<_> = self
            .materials
            .iter()
            .filter(|(_, m)| {
                m.arrays
                    .iter()
                    .any(|&i| self.arrays[i].as_ref().is_some_and(|a| a.failed))
            })
            .map(|(id, _)| id.clone())
            .collect();
        for material in retired {
            if self.retirable(&material, images, server) {
                self.release(&material, images, server)?;
            }
        }
        let mut pending = self
            .arrays
            .iter()
            .flatten()
            .flat_map(|a| &a.layers)
            .filter_map(|l| l.load.as_ref())
            .filter(|h| !images.contains(h.id()))
            .count();
        let job_limit = pending + new_jobs;
        for array in self.arrays.iter_mut().flatten() {
            if array.failed || array.image.is_some() {
                continue;
            }
            let key = array.key;
            for layer in &mut array.layers {
                let staging = key.layer_bytes()? + 16;
                if staging > 32 * 1024 * 1024 {
                    return Err(ReferenceError(
                        "Source mip payload exceeds shared staging cap".into(),
                    ));
                }
                if layer.load.is_none() && pending < job_limit && staging <= staging_available {
                    staging_available -= staging;
                    layer.load=Some(server.load_builder().with_settings(move |settings: &mut crate::presentation::textures::SourceImageSettings| {
                        settings.srgb=key.srgb;settings.filter=key.filter;
                        settings.wrap_u=key.wrap_u;settings.wrap_v=key.wrap_v;
                        settings.main_world_only=true;settings.decoded_bytes=key.layer_bytes().expect("admitted layer size");
                    }).load(layer.path.clone()));
                    pending += 1;
                }
            }
            if !array
                .layers
                .iter()
                .all(|l| l.load.as_ref().is_some_and(|h| images.contains(h.id())))
            {
                continue;
            }
            let key = array.key;
            let layer_bytes = key.layer_bytes()?;
            // Validate all layers before removing any source image.
            for layer in &array.layers {
                let image = images.get(layer.load.as_ref().unwrap()).unwrap();
                if image.width() != key.width
                    || image.height() != key.height
                    || image.texture_descriptor.mip_level_count != key.mips
                    || image.data.as_ref().map(Vec::len) != Some(layer_bytes)
                    || image.texture_descriptor.format
                        != if key.srgb {
                            TextureFormat::Rgba8UnormSrgb
                        } else {
                            TextureFormat::Rgba8Unorm
                        }
                {
                    return Err(ReferenceError(format!(
                        "Invalid source array layer {}:{}",
                        layer.source.file, layer.source.path_id
                    )));
                }
            }
            let mut data = Vec::with_capacity(array.bytes);
            for layer in &mut array.layers {
                // The loading handles are unique by source path. Another admitted array can
                // share a source layer, so retain its asset until all loading handles drop.
                let image = images.get(layer.load.as_ref().unwrap()).unwrap();
                data.extend_from_slice(image.data.as_ref().unwrap());
            }
            let mut image = Image::new_uninit(
                Extent3d {
                    width: key.width,
                    height: key.height,
                    depth_or_array_layers: array.layers.len() as u32,
                },
                TextureDimension::D2,
                if key.srgb {
                    TextureFormat::Rgba8UnormSrgb
                } else {
                    TextureFormat::Rgba8Unorm
                },
                RenderAssetUsages::RENDER_WORLD,
            );
            image.data = Some(data);
            image.data_order = TextureDataOrder::LayerMajor;
            image.texture_descriptor.mip_level_count = key.mips;
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
                address_mode_u: super::wrap(key.wrap_u),
                address_mode_v: super::wrap(key.wrap_v),
                mag_filter: filter,
                min_filter: filter,
                mipmap_filter: if key.filter == 2 || key.aniso > 1 {
                    ImageFilterMode::Linear
                } else {
                    ImageFilterMode::Nearest
                },
                anisotropy_clamp: u16::try_from(key.aniso.max(1)).map_err(|_| {
                    ReferenceError("Source anisotropy exceeds GPU sampler range".into())
                })?,
                lod_max_clamp: (key.mips - 1) as f32,
                ..default()
            });
            array.image = Some(images.add(image));
            // Release staging only after the render-world upload receipt.
            for layer in &mut array.layers {
                layer.staging_id = layer.load.as_ref().map(Handle::id);
                layer.load = None;
            }
            return failure.map_or(Ok(()), Err);
        }
        failure.map_or(Ok(()), Err)
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
    pub fn staging_bytes(&self, images: &Assets<Image>) -> usize {
        self.arrays
            .iter()
            .flatten()
            .map(|array| {
                let bytes = array.key.layer_bytes().expect("admitted source dimensions") + 16;
                array
                    .layers
                    .iter()
                    .filter(|layer| {
                        layer
                            .load
                            .as_ref()
                            .is_some_and(|h| !images.contains(h.id()))
                    })
                    .count()
                    * bytes
            })
            .sum()
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
            m.arrays
                .iter()
                .all(|&i| self.arrays[i].as_ref().is_some_and(|a| a.image.is_some()))
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
                        || a.layers.iter().all(|l| {
                            l.load.as_ref().is_none_or(|h| {
                                images.contains(h.id())
                                    || matches!(server.load_state(h.id()), LoadState::Failed(_))
                            })
                        })
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
            if let Some(image) = array.image {
                self.uploaded.lock().unwrap().remove(&image.id());
                images.remove(image.id());
            }
            if array.cpu_live {
                self.cpu_reserved -= array.cpu_bytes;
            }
            for layer in array.layers {
                if let Some(image) = layer.load {
                    images.remove(image.id());
                }
            }
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
