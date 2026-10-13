//! Selected Standard (Specular setup) with authored mip arrays and explicit reflectance.
use super::{SourceExtension, SourceSurface, SourceUniform, world_textures::WorldTextures};
use crate::reference::{ReferenceError, SourceId, WorldSourceMaterial};
use bevy::{
    asset as bevy_asset,
    ecs::system::{SystemParamItem, lifetimeless::SRes},
    mesh::MeshVertexBufferLayoutRef,
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    reflect as bevy_reflect,
    render::{
        render_asset::RenderAssets, render_resource::*, renderer::RenderDevice, texture::GpuImage,
    },
    shader::{ShaderDefVal, ShaderRef},
};
#[derive(Asset, TypePath, Clone, Debug)]
pub struct SpecularMaterial {
    pub lod_fade: f32,
    pub source: SourceId,
    base: StandardMaterial,
    uniform: SourceUniform,
    arrays: [Handle<Image>; 4],
    array_count: u8,
    receiver_mask: u32,
    reflected: bool,
    uploaded:
        std::sync::Arc<std::sync::Mutex<std::collections::HashSet<bevy::asset::AssetId<Image>>>>,
}
impl SpecularMaterial {
    pub fn from_source(
        record: &WorldSourceMaterial,
        textures: &WorldTextures,
        mask: u32,
        reflected: bool,
    ) -> Result<Self, ReferenceError> {
        if record.shader.name != "Standard (Specular setup)"
            || record.valid_keywords.len() != 2
            || !record.valid_keywords.iter().any(|x| x == "_EMISSION")
            || !record.valid_keywords.iter().any(|x| x == "_NORMALMAP")
        {
            return Err(ReferenceError("Unsupported source specular variant".into()));
        }
        let number = |name: &str| {
            record
                .floats
                .get(name)
                .copied()
                .ok_or_else(|| ReferenceError(format!("Specular material missing {name}")))
        };
        for (name, expected) in [
            ("_Mode", 0.),
            ("_ZWrite", 1.),
            ("_SrcBlend", 1.),
            ("_DstBlend", 0.),
            ("_SpecularHighlights", 1.),
            ("_GlossyReflections", 1.),
        ] {
            if number(name)? != expected {
                return Err(ReferenceError(format!(
                    "Unsupported source specular state {name}"
                )));
            }
        }
        let color = |name: &str| {
            record
                .colors
                .get(name)
                .copied()
                .map(|v| {
                    Vec4::from_array(
                        Color::srgba(v[0], v[1], v[2], v[3])
                            .to_linear()
                            .to_f32_array(),
                    )
                })
                .ok_or_else(|| ReferenceError(format!("Specular material missing {name}")))
        };
        let mut uniform = SourceSurface::default().data;
        uniform.color = color("_Color")?;
        uniform.emission = color("_EmissionColor")?;
        // _SpecColor is a linear reflectance value in the checked native constant buffer.
        let specular = record
            .colors
            .get("_SpecColor")
            .ok_or_else(|| ReferenceError("Missing _SpecColor".into()))?;
        uniform.specular = Vec4::new(specular[0], specular[1], specular[2], 1.);
        uniform.factors = Vec4::new(
            0.,
            number("_Glossiness")?,
            number("_GlossMapScale")?,
            number("_Cutoff")?,
        );
        uniform.scales = Vec4::new(number("_BumpScale")?, 0., number("_OcclusionStrength")?, 0.);
        uniform.flags = UVec4::new((1 << 0) | (1 << 3), 0, 0, 0);
        let uv = *record
            .texture_coordinates
            .get("_MainTex")
            .ok_or_else(|| ReferenceError("Missing specular _MainTex_ST".into()))?;
        uniform.uv = [Vec4::from_array(uv); 9];
        uniform.defaults[1] = Vec4::new(0.5, 0.5, 1., 1.);
        // Checked shader's null emission default is white; its source emission color is zero.
        if uniform.emission.truncate() != Vec3::ZERO {
            return Err(ReferenceError(
                "Unresolved nonzero null specular emission".into(),
            ));
        }
        let mut banks: Vec<(usize, Handle<Image>)> = Vec::with_capacity(4);
        for (slot, name) in [(0, "_MainTex"), (1, "_BumpMap"), (3, "_OcclusionMap")] {
            let (binding, image) = textures
                .binding(&record.source, name)
                .ok_or_else(|| ReferenceError(format!("Specular texture not ready: {name}")))?;
            let bank = match banks.iter().position(|(id, _)| *id == binding.array) {
                Some(index) => index,
                None => {
                    banks.push((binding.array, image.clone()));
                    banks.len() - 1
                }
            };
            if bank >= 4 {
                return Err(ReferenceError(
                    "Specular material exceeds four exact arrays".into(),
                ));
            }
            uniform.banks[slot / 4][slot % 4] = bank as u32;
            uniform.layers[slot / 4][slot % 4] = binding.layer;
            uniform.flags.y |= 1 << slot;
            let sampling = textures
                .sampling(binding.array)
                .ok_or_else(|| ReferenceError("Missing source specular sampler".into()))?;
            uniform.sampling[slot] = Vec4::new(0., sampling.x, sampling.y, sampling.z);
        }
        let array_count = banks.len() as u8;
        let arrays = std::array::from_fn(|i| banks.get(i).unwrap_or(&banks[0]).1.clone());
        Ok(Self {
            lod_fade: 1.,
            source: record.source.clone(),
            base: StandardMaterial::default(),
            uniform,
            arrays,
            array_count,
            receiver_mask: mask,
            reflected,
            uploaded: textures.upload_receipts(),
        })
    }
}
impl Material for SpecularMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/source_mesh.wgsl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "shaders/source_surface.wgsl".into()
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
    fn specialize(
        pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        StandardMaterial::specialize(
            pipeline,
            descriptor,
            layout,
            MaterialPipelineKey::<StandardMaterial> {
                mesh_key: key.mesh_key,
                bind_group_data: StandardMaterial::default().bind_group_data(),
            },
        )?;
        if key.bind_group_data {
            descriptor.primitive.front_face = FrontFace::Cw;
        }
        if let Some(fragment) = &mut descriptor.fragment {
            fragment.shader_defs.extend([
                ShaderDefVal::Bool("SOURCE_AUTHORED".into(), true),
                ShaderDefVal::Bool("WORLD_TEXTURE_ARRAYS".into(), true),
            ]);
        }
        Ok(())
    }
}
impl AsBindGroup for SpecularMaterial {
    type Data = bool;
    type Param = (
        <SourceExtension as AsBindGroup>::Param,
        SRes<RenderAssets<GpuImage>>,
        <StandardMaterial as AsBindGroup>::Param,
    );
    fn label() -> &'static str {
        "source specular surface"
    }
    fn bind_group_data(&self) -> bool {
        self.reflected
    }
    fn unprepared_bind_group(
        &self,
        layout: &BindGroupLayout,
        device: &RenderDevice,
        params: &mut SystemParamItem<Self::Param>,
        _: bool,
    ) -> Result<UnpreparedBindGroup, AsBindGroupError> {
        let extension = SourceExtension {
            lod_fade: self.lod_fade,
            receiver_mask: self.receiver_mask,
            surface: None,
        };
        let mut result = extension.unprepared_bind_group(layout, device, &mut params.0, true)?;
        result.bindings.0.retain(|(index, _)| *index <= 113);
        let base = self
            .base
            .unprepared_bind_group(layout, device, &mut params.2, true)?;
        result
            .bindings
            .0
            .extend(base.bindings.0.into_iter().filter(|(index, _)| *index == 0));
        let mut data = encase::UniformBuffer::new(Vec::new());
        data.write(&self.uniform)
            .expect("specular surface uniform layout");
        result.bindings.0.push((
            120,
            OwnedBindingResource::Buffer(device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("specular surface parameters"),
                contents: data.as_ref(),
                usage: BufferUsages::UNIFORM,
            })),
        ));
        for (index, handle) in self.arrays.iter().enumerate() {
            let image = params
                .1
                .get(handle.id())
                .ok_or(AsBindGroupError::RetryNextUpdate)?;
            result.bindings.0.push((
                1 + index as u32 * 2,
                OwnedBindingResource::TextureView(
                    TextureViewDimension::D2Array,
                    image.texture_view.clone(),
                ),
            ));
            result.bindings.0.push((
                2 + index as u32 * 2,
                OwnedBindingResource::Sampler(SamplerBindingType::Filtering, image.sampler.clone()),
            ));
        }
        self.uploaded.lock().unwrap().extend(
            self.arrays[..self.array_count as usize]
                .iter()
                .map(Handle::id),
        );
        Ok(result)
    }
    fn bind_group_layout_entries(device: &RenderDevice, force: bool) -> Vec<BindGroupLayoutEntry> {
        use binding_types::*;
        let mut result = SourceExtension::bind_group_layout_entries(device, force);
        result.retain(|entry| entry.binding <= 113);
        result.extend(
            StandardMaterial::bind_group_layout_entries(device, true)
                .into_iter()
                .filter(|entry| entry.binding == 0),
        );
        result.extend(
            BindGroupLayoutEntries::with_indices(
                ShaderStages::FRAGMENT,
                ((120, uniform_buffer::<SourceUniform>(false)),),
            )
            .to_vec(),
        );
        result.extend(
            BindGroupLayoutEntries::sequential(
                ShaderStages::VERTEX_FRAGMENT,
                (
                    texture_2d_array(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                    texture_2d_array(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                    texture_2d_array(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                    texture_2d_array(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                ),
            )
            .to_vec()
            .into_iter()
            .enumerate()
            .map(|(index, mut entry)| {
                entry.binding = 1 + index as u32;
                entry
            }),
        );
        result
    }
}
