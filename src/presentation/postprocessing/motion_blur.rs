//! Installed seven-draw motion blur. Targets and bindings persist until physical resize.
use bevy::prelude::*;
use bevy::render::{
    render_resource::{binding_types::*, *},
    renderer::{RenderAdapter, RenderDevice, RenderQueue},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MotionLayout {
    pub extents: [[u32; 2]; 6],
    pub radius: u32,
    pub tile_size: u32,
    pub tile_loop: u32,
}
impl MotionLayout {
    pub fn new(extent: [u32; 2], limit: u32) -> Result<Self, String> {
        if extent.iter().any(|&n| n == 0 || n > limit) {
            return Err("Source motion blur physical extent is invalid".into());
        }
        let radius = ((5.0_f32 * extent[1] as f32) / 100.0) as u32;
        // Installed integer division truncates towards zero, including radius0.
        let tile_size = (((radius as i32 - 1) / 8 + 1) * 8) as u32;
        let extents = [
            extent,
            extent.map(|n| n / 2),
            extent.map(|n| n / 4),
            extent.map(|n| n / 8),
            extent.map(|n| n / tile_size),
            extent.map(|n| n / tile_size),
        ];
        if extents.iter().flatten().any(|&n| n == 0) {
            return Err("Source motion blur floor division produced an empty target".into());
        }
        Ok(Self {
            extents,
            radius,
            tile_size,
            tile_loop: tile_size / 8,
        })
    }
    pub fn logical_bytes(&self) -> u64 {
        self.extents
            .iter()
            .map(|e| u64::from(e[0]) * u64::from(e[1]) * 4)
            .sum::<u64>()
            + u64::from(self.extents[0][0]) * u64::from(self.extents[0][1]) * 8
    }
}
#[derive(Clone, Copy, ShaderType)]
struct MotionParams {
    output_size: Vec2,
    screen_size: Vec2,
    settings: Vec4,
    depth: Vec4,
    control: UVec4,
}
struct Target {
    _texture: Texture,
    view: TextureView,
}
impl Target {
    fn new(device: &RenderDevice, extent: [u32; 2], format: TextureFormat) -> Self {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("source_motion_blur"),
            size: Extent3d {
                width: extent[0],
                height: extent[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&default());
        Self {
            _texture: texture,
            view,
        }
    }
}
pub struct MotionBlurPass {
    pub layout: MotionLayout,
    targets: [Target; 6],
    output: Target,
    uniforms: [UniformBuffer<MotionParams>; 8],
    bindings: [BindGroup; 8],
    pipelines: [CachedRenderPipelineId; 7],
    reset: bool,
}
impl MotionBlurPass {
    pub fn new(
        adapter: &RenderAdapter,
        device: &RenderDevice,
        queue: &RenderQueue,
        cache: &PipelineCache,
        shader: Handle<Shader>,
        source: &TextureView,
        depth: &TextureView,
        motion: &TextureView,
        extent: [u32; 2],
        previous: Option<&Self>,
    ) -> Result<Self, String> {
        let layout = MotionLayout::new(extent, device.limits().max_texture_dimension_2d)?;
        for format in [
            TextureFormat::Rgb10a2Unorm,
            TextureFormat::Rg16Float,
            TextureFormat::Rgba16Float,
        ] {
            if !adapter
                .get_texture_format_features(format)
                .allowed_usages
                .contains(TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT)
            {
                return Err(format!(
                    "Source motion blur requires sampled render attachment {format:?}"
                ));
            }
        }
        let targets = std::array::from_fn(|i| {
            Target::new(
                device,
                layout.extents[i],
                if i == 0 {
                    TextureFormat::Rgb10a2Unorm
                } else {
                    TextureFormat::Rg16Float
                },
            )
        });
        let output = Target::new(device, extent, TextureFormat::Rgba16Float);
        let descriptor = BindGroupLayoutDescriptor::new(
            "source_motion_blur",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    uniform_buffer::<MotionParams>(false),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_2d(TextureSampleType::Float { filterable: false }),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::NonFiltering),
                    sampler(SamplerBindingType::Filtering),
                ),
            ),
        );
        let pipelines = previous.map_or_else(
            || {
                std::array::from_fn(|i| {
                    let entry = [
                        "velocity",
                        "tile_decode",
                        "tile_reduce",
                        "tile_variable",
                        "neighbor",
                        "reconstruct",
                        "reset_copy",
                    ][i];
                    cache.queue_render_pipeline(RenderPipelineDescriptor {
                        label: Some(format!("source_motion_{entry}").into()),
                        layout: vec![descriptor.clone()],
                        vertex: VertexState {
                            shader: shader.clone(),
                            entry_point: Some("vertex".into()),
                            ..default()
                        },
                        fragment: Some(FragmentState {
                            shader: shader.clone(),
                            entry_point: Some(entry.into()),
                            targets: vec![Some(ColorTargetState {
                                format: if i == 0 {
                                    TextureFormat::Rgb10a2Unorm
                                } else if i < 5 {
                                    TextureFormat::Rg16Float
                                } else {
                                    TextureFormat::Rgba16Float
                                },
                                blend: None,
                                write_mask: ColorWrites::ALL,
                            })],
                            ..default()
                        }),
                        ..default()
                    })
                })
            },
            |p| p.pipelines,
        );
        let uniforms = std::array::from_fn(|i| {
            let out = if i < 6 { layout.extents[i] } else { extent };
            let mut buffer = UniformBuffer::from(MotionParams {
                output_size: Vec2::new(out[0] as f32, out[1] as f32),
                screen_size: Vec2::new(extent[0] as f32, extent[1] as f32),
                settings: Vec4::new(
                    layout.radius as f32,
                    1.0 / layout.radius as f32,
                    5.0 / 360.0,
                    2.0,
                ),
                depth: Vec4::new(0., 1., (layout.tile_loop as f32 - 1.) * -0.5, 0.),
                control: UVec4::new(layout.tile_loop, 0, 0, 0),
            });
            buffer.write_buffer(device, queue);
            buffer
        });
        let point = device.create_sampler(&SamplerDescriptor {
            label: Some("source_motion_point_clamp"),
            ..default()
        });
        let linear = device.create_sampler(&SamplerDescriptor {
            label: Some("source_motion_linear_clamp"),
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            ..default()
        });
        let bindings = std::array::from_fn(|i| {
            device.create_bind_group(
                "source_motion_blur",
                &cache.get_bind_group_layout(&descriptor),
                &BindGroupEntries::sequential((
                    uniforms[i].binding().unwrap(),
                    if (1..6).contains(&i) {
                        &targets[i - 1].view
                    } else {
                        source
                    },
                    if i == 6 { &targets[0].view } else { source },
                    if i == 6 { &targets[5].view } else { source },
                    depth,
                    motion,
                    &point,
                    &linear,
                )),
            )
        });
        Ok(Self {
            layout,
            targets,
            output,
            uniforms,
            bindings,
            pipelines,
            reset: previous.is_none_or(|p| p.reset),
        })
    }
    pub fn output(&self) -> &TextureView {
        &self.output.view
    }
    pub fn reset_history(&mut self) {
        self.reset = true;
    }
    pub fn failure(&self, cache: &PipelineCache) -> Option<String> {
        super::pipeline_failure(cache, &self.pipelines)
    }
    pub fn completed(&mut self) {
        self.reset = false;
    }
    pub fn ready(&self, cache: &PipelineCache) -> bool {
        self.pipelines
            .iter()
            .all(|id| cache.get_render_pipeline(*id).is_some())
    }
    pub fn update(
        &mut self,
        device: &RenderDevice,
        queue: &RenderQueue,
        near: f32,
        far: f32,
        shutter: f32,
    ) {
        for uniform in &mut self.uniforms {
            let value = uniform.get_mut();
            value.settings.z = shutter / 360.0;
            value.depth.x = far / near - 1.0;
            uniform.write_buffer(device, queue);
        }
    }
    /// A reset performs the installed copy only. Resize preserves the existing reset state.
    pub fn encode(&mut self, encoder: &mut CommandEncoder, cache: &PipelineCache) -> bool {
        if !self.ready(cache) {
            return false;
        }
        if self.reset {
            draw(
                encoder,
                cache.get_render_pipeline(self.pipelines[6]).unwrap(),
                &self.bindings[7],
                &self.output.view,
            );
            return true;
        }
        for (i, pipeline) in [0, 1, 2, 2, 3, 4, 5].into_iter().enumerate() {
            draw(
                encoder,
                cache.get_render_pipeline(self.pipelines[pipeline]).unwrap(),
                &self.bindings[i],
                if i < 6 {
                    &self.targets[i].view
                } else {
                    &self.output.view
                },
            );
        }
        true
    }
}
fn draw(
    encoder: &mut CommandEncoder,
    pipeline: &RenderPipeline,
    binding: &BindGroup,
    output: &TextureView,
) {
    let colors = [Some(RenderPassColorAttachment {
        view: output,
        depth_slice: None,
        resolve_target: None,
        ops: Operations {
            load: LoadOp::Clear(LinearRgba::NONE.into()),
            store: StoreOp::Store,
        },
    })];
    let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
        label: Some("source_motion_blur"),
        color_attachments: &colors,
        ..default()
    });
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, binding, &[]);
    pass.draw(0..3, 0..1);
}
