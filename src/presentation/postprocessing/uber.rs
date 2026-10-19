//! Camera-local Uber output and persistent noise/exposure bindings.
use super::{dither::DitherSequence, pipeline_failure};
use crate::reference::postprocessing::PostprocessingRecord;
use bevy::prelude::*;
use bevy::render::{
    render_resource::{binding_types::*, *},
    renderer::{RenderDevice, RenderQueue},
};
#[derive(Clone, Copy, ShaderType)]
struct Params {
    extent: Vec4,
    noise: Vec4,
    bloom: Vec4,
    tint: Vec4,
    dirt: Vec4,
    grading: Vec4,
}
pub struct MainInputs<'a> {
    pub bloom: &'a TextureView,
    pub exposure: [&'a TextureView; 2],
    pub dirt: &'a TextureView,
    pub dirt_extent: [u32; 2],
    pub lut: &'a TextureView,
    pub sample_scale: f32,
}
pub struct UberPass {
    uniform: UniformBuffer<Params>,
    bindings: Vec<BindGroup>,
    pipeline: CachedRenderPipelineId,
    sequence: DitherSequence,
    pending: Option<DitherSequence>,
    extent: [u32; 2],
    main: bool,
}
impl UberPass {
    pub fn new(
        device: &RenderDevice,
        queue: &RenderQueue,
        cache: &PipelineCache,
        shader: Handle<Shader>,
        source: &TextureView,
        noise: [&TextureView; 64],
        extent: [u32; 2],
        record: &PostprocessingRecord,
        main: Option<MainInputs<'_>>,
        previous: Option<Self>,
    ) -> Result<Self, String> {
        if extent.contains(&0) {
            return Err("Source Uber requires a positive extent".into());
        }
        let mut entries = BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                uniform_buffer::<Params>(false),
                texture_2d(TextureSampleType::Float { filterable: true }),
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                sampler(SamplerBindingType::NonFiltering),
            ),
        )
        .to_vec();
        if main.is_some() {
            entries.extend(
                BindGroupLayoutEntries::with_indices(
                    ShaderStages::FRAGMENT,
                    (
                        (5, texture_2d(TextureSampleType::Float { filterable: true })),
                        (
                            6,
                            texture_2d(TextureSampleType::Float { filterable: false }),
                        ),
                        (7, texture_2d(TextureSampleType::Float { filterable: true })),
                        (8, texture_3d(TextureSampleType::Float { filterable: true })),
                        (9, sampler(SamplerBindingType::Filtering)),
                    ),
                )
                .to_vec(),
            );
        }
        let layout = BindGroupLayoutDescriptor::new("source_uber", &entries);
        let pipeline = previous.as_ref().map_or_else(
            || {
                cache.queue_render_pipeline(RenderPipelineDescriptor {
                    label: Some("source_uber".into()),
                    layout: vec![layout.clone()],
                    vertex: VertexState {
                        shader: shader.clone(),
                        entry_point: Some("vertex".into()),
                        ..default()
                    },
                    fragment: Some(FragmentState {
                        shader,
                        entry_point: Some("uber".into()),
                        shader_defs: if main.is_some() {
                            vec!["SOURCE_UBER_MAIN".into()]
                        } else {
                            vec![]
                        },
                        targets: vec![Some(ColorTargetState {
                            format: TextureFormat::Rgba16Float,
                            blend: None,
                            write_mask: ColorWrites::ALL,
                        })],
                        ..default()
                    }),
                    multisample: MultisampleState {
                        count: crate::presentation::render_targets::MSAA_SAMPLES,
                        ..default()
                    },
                    ..default()
                })
            },
            |old| old.pipeline,
        );
        let mut params = Params {
            extent: Vec4::new(
                extent[0] as f32,
                extent[1] as f32,
                1. / extent[0] as f32,
                1. / extent[1] as f32,
            ),
            noise: Vec4::ZERO,
            bloom: Vec4::ZERO,
            tint: Vec4::ONE,
            dirt: Vec4::new(1., 1., 0., 0.),
            grading: Vec4::new(1. / 33., 32., record.grading.post_exposure.exp2(), 0.),
        };
        if let Some(input) = main.as_ref() {
            if input.dirt_extent.contains(&0) {
                return Err("Source bloom dirt extent is invalid".into());
            }
            params.bloom = Vec4::new(
                input.sample_scale,
                (record.bloom.intensity / 10.).exp2() - 1.,
                record.bloom.dirt_intensity,
                0.,
            );
            let c = record.bloom.color;
            params.tint = Vec4::from_array([
                super::bloom::gamma_to_linear(c[0]),
                super::bloom::gamma_to_linear(c[1]),
                super::bloom::gamma_to_linear(c[2]),
                c[3],
            ]);
            let image_aspect = input.dirt_extent[0] as f32 / input.dirt_extent[1] as f32;
            let screen_aspect = extent[0] as f32 / extent[1] as f32;
            if image_aspect > screen_aspect {
                params.dirt.x = screen_aspect / image_aspect;
                params.dirt.z = (1. - params.dirt.x) / 2.;
            } else {
                params.dirt.y = image_aspect / screen_aspect;
                params.dirt.w = (1. - params.dirt.y) / 2.;
            }
        }
        let mut uniform = UniformBuffer::from(params);
        uniform.write_buffer(device, queue);
        let linear = device.create_sampler(&SamplerDescriptor {
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            ..default()
        });
        let point = device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            ..default()
        });
        let repeat = device.create_sampler(&SamplerDescriptor {
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            ..default()
        });
        let bindings = (0..if main.is_some() { 128 } else { 64 })
            .map(|i| {
                let mut entries = BindGroupEntries::sequential((
                    uniform.binding().unwrap(),
                    source,
                    noise[i % 64],
                    &linear,
                    &point,
                ))
                .to_vec();
                if let Some(input) = main.as_ref() {
                    entries.extend(
                        BindGroupEntries::with_indices((
                            (5, input.bloom),
                            (6, input.exposure[i / 64]),
                            (7, input.dirt),
                            (8, input.lut),
                            (9, &repeat),
                        ))
                        .to_vec(),
                    );
                }
                device.create_bind_group(
                    "source_uber",
                    &cache.get_bind_group_layout(&layout),
                    &entries,
                )
            })
            .collect();
        Ok(Self {
            uniform,
            bindings,
            pipeline,
            sequence: previous.map_or_else(DitherSequence::default, |old| old.sequence),
            pending: None,
            extent,
            main: main.is_some(),
        })
    }
    pub fn recreate(&mut self) {
        self.sequence = DitherSequence::default();
        self.pending = None;
    }
    pub fn noise_index(&self) -> u32 {
        self.sequence.noise_index()
    }
    pub fn completed(&mut self) {
        if let Some(sequence) = self.pending.take() {
            self.sequence = sequence;
        }
    }
    pub fn ready(&self, cache: &PipelineCache) -> bool {
        cache.get_render_pipeline(self.pipeline).is_some()
    }
    pub fn failure(&self, cache: &PipelineCache) -> Option<String> {
        pipeline_failure(cache, &[self.pipeline])
    }
    pub fn encode(
        &mut self,
        encoder: &mut CommandEncoder,
        device: &RenderDevice,
        queue: &RenderQueue,
        cache: &PipelineCache,
        output: &TextureView,
        resolve: &TextureView,
        exposure_index: usize,
    ) -> bool {
        if !self.ready(cache) || exposure_index > usize::from(self.main) {
            return false;
        }
        let mut sequence = self.sequence.clone();
        let (index, noise) = sequence.render(self.extent);
        self.pending = Some(sequence);
        self.uniform.get_mut().noise = Vec4::from_array(noise);
        self.uniform.write_buffer(device, queue);
        let colors = [Some(RenderPassColorAttachment {
            view: output,
            depth_slice: None,
            resolve_target: Some(resolve),
            ops: Operations {
                load: LoadOp::Load,
                store: StoreOp::Store,
            },
        })];
        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("source_uber"),
            color_attachments: &colors,
            ..default()
        });
        pass.set_pipeline(cache.get_render_pipeline(self.pipeline).unwrap());
        pass.set_bind_group(0, &self.bindings[exposure_index * 64 + index], &[]);
        pass.draw(0..3, 0..1);
        true
    }
}
