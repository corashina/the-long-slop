//! Source TAA history, including reset and history-preserving resize.
use crate::presentation::render_targets::HDR_FORMAT;
use crate::reference::postprocessing::TaaSettings;
use bevy::prelude::*;
use bevy::render::{
    render_resource::{binding_types::*, *},
    renderer::{RenderDevice, RenderQueue},
};
#[derive(Clone, Copy, ShaderType)]
struct TaaParams {
    extent: Vec4,
    jitter: Vec4,
    blend: Vec4,
}
pub struct TaaPass {
    _textures: [Texture; 2],
    views: [TextureView; 2],
    bindings: [BindGroup; 2],
    seed_bindings: Option<[BindGroup; 2]>,
    reset_bindings: [BindGroup; 2],
    reset: bool,
    uniform: UniformBuffer<TaaParams>,
    pipelines: [CachedRenderPipelineId; 2],
    next_output: usize,
    initialized: bool,
    pending: bool,
    pub logical_bytes: u64,
}
impl TaaPass {
    pub fn new(
        device: &RenderDevice,
        queue: &RenderQueue,
        cache: &PipelineCache,
        shader: Handle<Shader>,
        source: &TextureView,
        depth: &TextureView,
        motion: &TextureView,
        extent: [u32; 2],
        settings: TaaSettings,
        previous: Option<Self>,
    ) -> Result<Self, String> {
        let [width, height] = extent;
        if width == 0
            || height == 0
            || width.max(height) > device.limits().max_texture_dimension_2d
            || !settings.validate()
        {
            return Err("Invalid source TAA extent or settings".into());
        }
        let layout = BindGroupLayoutDescriptor::new(
            "source_taa",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    uniform_buffer::<TaaParams>(false),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_2d(TextureSampleType::Float { filterable: false }),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                    sampler(SamplerBindingType::NonFiltering),
                ),
            ),
        );
        let seed_layout = BindGroupLayoutDescriptor::new(
            "source_taa_seed",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::FRAGMENT,
                (
                    (0, uniform_buffer::<TaaParams>(false)),
                    (1, texture_2d(TextureSampleType::Float { filterable: true })),
                    (5, sampler(SamplerBindingType::Filtering)),
                ),
            ),
        );
        let pipelines = previous
            .as_ref()
            .map(|old| old.pipelines)
            .unwrap_or_else(|| {
                std::array::from_fn(|index| {
                    cache.queue_render_pipeline(RenderPipelineDescriptor {
                        label: Some(
                            if index == 0 {
                                "source_taa_seed"
                            } else {
                                "source_taa_resolve"
                            }
                            .into(),
                        ),
                        layout: vec![if index == 0 {
                            seed_layout.clone()
                        } else {
                            layout.clone()
                        }],
                        vertex: VertexState {
                            shader: shader.clone(),
                            entry_point: Some("vertex".into()),
                            ..default()
                        },
                        fragment: Some(FragmentState {
                            shader: shader.clone(),
                            entry_point: Some(if index == 0 { "seed" } else { "resolve" }.into()),
                            targets: vec![
                                Some(ColorTargetState {
                                    format: HDR_FORMAT,
                                    blend: None,
                                    write_mask: ColorWrites::ALL
                                });
                                if index == 0 { 1 } else { 2 }
                            ],
                            ..default()
                        }),
                        ..default()
                    })
                })
            });
        let textures = std::array::from_fn(|_| {
            device.create_texture(&TextureDescriptor {
                label: Some("source_taa_history"),
                size: Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: HDR_FORMAT,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            })
        });
        let views = std::array::from_fn(|index| textures[index].create_view(&default()));
        let linear = device.create_sampler(&SamplerDescriptor {
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..default()
        });
        let point = device.create_sampler(&SamplerDescriptor::default());
        let mut uniform = UniformBuffer::from(TaaParams {
            extent: Vec4::new(
                width as f32,
                height as f32,
                1. / width as f32,
                1. / height as f32,
            ),
            jitter: Vec4::ZERO,
            blend: Vec4::new(
                settings.stationary_blending,
                settings.motion_blending,
                6000.,
                settings.sharpness,
            ),
        });
        uniform.write_buffer(device, queue);
        let bindings = std::array::from_fn(|index| {
            device.create_bind_group(
                "source_taa",
                &cache.get_bind_group_layout(&layout),
                &BindGroupEntries::sequential((
                    uniform.binding().unwrap(),
                    source,
                    &views[1 - index],
                    depth,
                    motion,
                    &linear,
                    &point,
                )),
            )
        });
        let old = previous.as_ref().filter(|old| old.initialized);
        let seed_bindings = std::array::from_fn(|index| {
            device.create_bind_group(
                "source_taa_seed",
                &cache.get_bind_group_layout(&seed_layout),
                &BindGroupEntries::with_indices((
                    (0, uniform.binding().unwrap()),
                    (1, old.map_or(source, |old| &old.views[index])),
                    (5, &linear),
                )),
            )
        });
        let reset_bindings = std::array::from_fn(|_| {
            device.create_bind_group(
                "source_taa_reset",
                &cache.get_bind_group_layout(&seed_layout),
                &BindGroupEntries::with_indices((
                    (0, uniform.binding().unwrap()),
                    (1, source),
                    (5, &linear),
                )),
            )
        });
        let reset = previous.as_ref().is_some_and(|old| old.reset);
        Ok(Self {
            _textures: textures,
            views,
            bindings,
            seed_bindings: Some(seed_bindings),
            reset_bindings,
            reset,
            uniform,
            pipelines,
            next_output: old.map_or(0, |old| old.next_output),
            initialized: false,
            pending: false,
            logical_bytes: u64::from(width) * u64::from(height) * 16,
        })
    }
    pub fn recreate(&mut self) {
        self.reset_history();
        self.next_output = 0;
        self.initialized = false;
        self.seed_bindings = None;
    }
    pub fn history(&self) -> [u32; 2] {
        [self.next_output as u32, u32::from(self.initialized)]
    }
    pub fn reset_history(&mut self) {
        self.reset = true;
        self.pending = false;
    }
    pub fn completed(&mut self) {
        if self.pending {
            self.next_output = 1 - self.next_output;
            self.initialized = true;
            self.reset = false;
            self.seed_bindings = None;
            self.pending = false;
        }
    }
    pub fn failure(&self, cache: &PipelineCache) -> Option<String> {
        super::pipeline_failure(cache, &self.pipelines)
    }
    pub fn ready(&self, cache: &PipelineCache) -> bool {
        self.pipelines
            .iter()
            .all(|id| cache.get_render_pipeline(*id).is_some())
    }
    pub fn encode(
        &mut self,
        encoder: &mut CommandEncoder,
        device: &RenderDevice,
        queue: &RenderQueue,
        cache: &PipelineCache,
        output: &TextureView,
        jitter_uv: [f32; 2],
    ) -> Result<bool, String> {
        if !jitter_uv.iter().all(|v| v.is_finite()) {
            return Err("Invalid source TAA jitter".into());
        }
        if !self.ready(cache) {
            return Ok(false);
        }
        self.uniform.get_mut().jitter = Vec4::new(jitter_uv[0], jitter_uv[1], 0., 0.);
        self.uniform.write_buffer(device, queue);
        if self.reset {
            for (index, binding) in self.reset_bindings.iter().enumerate() {
                draw(
                    encoder,
                    cache.get_render_pipeline(self.pipelines[0]).unwrap(),
                    binding,
                    &[&self.views[index]],
                );
            }
        }
        if let Some(bindings) = self.seed_bindings.as_ref().filter(|_| !self.reset) {
            for (index, binding) in bindings.iter().enumerate() {
                draw(
                    encoder,
                    cache.get_render_pipeline(self.pipelines[0]).unwrap(),
                    binding,
                    &[&self.views[index]],
                );
            }
        }
        draw(
            encoder,
            cache.get_render_pipeline(self.pipelines[1]).unwrap(),
            &self.bindings[self.next_output],
            &[output, &self.views[self.next_output]],
        );
        self.pending = true;
        Ok(true)
    }
}
fn draw(
    encoder: &mut CommandEncoder,
    pipeline: &RenderPipeline,
    group: &BindGroup,
    outputs: &[&TextureView],
) {
    let attachments: [Option<RenderPassColorAttachment>; 2] = std::array::from_fn(|index| {
        outputs.get(index).map(|view| RenderPassColorAttachment {
            view,
            depth_slice: None,
            resolve_target: None,
            ops: Operations {
                load: LoadOp::Clear(LinearRgba::NONE.into()),
                store: StoreOp::Store,
            },
        })
    });
    let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
        label: Some("source_taa"),
        color_attachments: &attachments[..outputs.len()],
        ..default()
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, group, &[]);
    pass.draw(0..3, 0..1);
}
