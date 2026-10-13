//! Installed Neutral grading LUT. Bake once into persistent 33-cubed RGBA16F storage.
use crate::reference::postprocessing::GradingSettings;
use bevy::prelude::*;
use bevy::render::{
    render_resource::{binding_types::*, *},
    renderer::{RenderAdapter, RenderDevice, RenderQueue},
};
pub const LUT_SIZE: u32 = 33;
pub const LUT_BYTES: u64 = 33 * 33 * 33 * 8;
pub const CURVE_BYTES: u64 = 128 * 2 * 8;
#[derive(Clone, Copy, ShaderType)]
struct Params {
    size: Vec4,
    balance: Vec4,
    filter: Vec4,
    hue_sat_con: Vec4,
    mixer_r: Vec4,
    mixer_g: Vec4,
    mixer_b: Vec4,
    lift: Vec4,
    inverse_gamma: Vec4,
    gain: Vec4,
    slice: UVec4,
}
fn color_balance(temperature: f32, tint: f32) -> [f32; 3] {
    let t = temperature / 60.;
    let tint = tint / 60.;
    let x = 0.3127099871635437
        - t * if t < 0. {
            0.10000000149011612
        } else {
            0.05000000074505806
        };
    let y =
        ((2.869999885559082 * x - (3. * x) * x) - 0.2750950753688812) + tint * 0.05000000074505806;
    let xx = x / y;
    let zz = ((1. - x) - y) / y;
    let l = (0.7328000068664551 * xx + 0.4296000003814697) - 0.1624000072479248 * zz;
    let m = (-0.7035999894142151 * xx + 1.6974999904632568) + 0.006099999882280827 * zz;
    let s = (0.003000000026077032 * xx + 0.01360000018030405) + 0.9833999872207642 * zz;
    [
        0.9492369890213013 / l,
        1.0354199409484863 / m,
        1.087280035018921 / s,
    ]
}
fn trackball(value: [f32; 4], scale: f32, kind: usize) -> [f32; 3] {
    let v = value.map(|x| x * scale);
    let lum = (v[0] * 0.2125999927520752 + v[1] * 0.7152000069618225) + v[2] * 0.0722000002861023;
    let offset = if kind == 0 { v[3] } else { v[3] + 1. };
    std::array::from_fn(|i| {
        let c = (v[i] - lum) + offset;
        if kind == 1 {
            1. / c.max(0.0010000000474974513)
        } else {
            c
        }
    })
}
impl Params {
    fn new(s: &GradingSettings) -> Self {
        let extend = |a: [f32; 3]| Vec3::from_array(a).extend(0.);
        Self {
            size: Vec4::new(33., 0.03125, 0., 0.),
            balance: extend(color_balance(s.temperature, s.tint)),
            filter: Vec4::from_array(s.color_filter),
            hue_sat_con: Vec4::new(
                s.hue_shift / 360.,
                s.saturation / 100. + 1.,
                s.contrast / 100. + 1.,
                0.,
            ),
            mixer_r: extend(s.channel_mixer[0].map(|x| x * (1. / 100.))),
            mixer_g: extend(s.channel_mixer[1].map(|x| x * (1. / 100.))),
            mixer_b: extend(s.channel_mixer[2].map(|x| x * (1. / 100.))),
            lift: extend(trackball(s.trackballs[0], 0.20000000298023224, 0)),
            inverse_gamma: extend(trackball(s.trackballs[1], 0.800000011920929, 1)),
            gain: extend(trackball(s.trackballs[2], 0.800000011920929, 2)),
            slice: UVec4::ZERO,
        }
    }
}
struct Binding {
    _uniform: UniformBuffer<Params>,
    group: BindGroup,
}
pub struct LutPass {
    _texture: Texture,
    view: TextureView,
    _curves: Texture,
    bindings: [Binding; 33],
    pipeline: CachedRenderPipelineId,
}
impl LutPass {
    pub fn new(
        adapter: &RenderAdapter,
        device: &RenderDevice,
        queue: &RenderQueue,
        cache: &PipelineCache,
        shader: Handle<Shader>,
        settings: &GradingSettings,
    ) -> Result<Self, String> {
        if settings.lut_size != LUT_SIZE {
            return Err("Source Neutral LUT requires 33 cubed storage".into());
        }
        let support = adapter.get_texture_format_features(TextureFormat::Rgba16Float);
        if !support
            .allowed_usages
            .contains(TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING)
            || !support
                .flags
                .contains(TextureFormatFeatureFlags::FILTERABLE)
        {
            return Err("Source Neutral LUT requires filterable RGBA16F render storage".into());
        }
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("source_neutral_lut"),
            size: Extent3d {
                width: LUT_SIZE,
                height: LUT_SIZE,
                depth_or_array_layers: LUT_SIZE,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D3,
            format: TextureFormat::Rgba16Float,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&default());
        let curves = device.create_texture(&TextureDescriptor {
            label: Some("source_grading_curves"),
            size: Extent3d {
                width: 128,
                height: 2,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba16Float,
            usage: TextureUsages::COPY_DST | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        // ColorGrading empty splines return m_ZeroValue=.5. HDR leaves the second row at zero.
        let mut pixels = [0u8; CURVE_BYTES as usize];
        for value in pixels[..1024].chunks_exact_mut(2) {
            value.copy_from_slice(&0x3800u16.to_le_bytes());
        }
        queue.write_texture(
            TexelCopyTextureInfo {
                texture: &curves,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            &pixels,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(1024),
                rows_per_image: Some(2),
            },
            Extent3d {
                width: 128,
                height: 2,
                depth_or_array_layers: 1,
            },
        );
        let curve_view = curves.create_view(&default());
        let curve_sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("source_grading_curve_linear_clamp"),
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            ..default()
        });
        let layout = BindGroupLayoutDescriptor::new(
            "source_neutral_lut",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    uniform_buffer::<Params>(false),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                ),
            ),
        );
        let pipeline = cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("source_neutral_lut".into()),
            layout: vec![layout.clone()],
            vertex: VertexState {
                shader: shader.clone(),
                entry_point: Some("vertex".into()),
                ..default()
            },
            fragment: Some(FragmentState {
                shader,
                entry_point: Some("bake".into()),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rgba16Float,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            ..default()
        });
        let params = Params::new(settings);
        let bindings = std::array::from_fn(|i| {
            let mut params = params;
            params.slice.x = i as u32;
            let mut uniform = UniformBuffer::from(params);
            uniform.write_buffer(device, queue);
            let group = device.create_bind_group(
                "source_neutral_lut",
                &cache.get_bind_group_layout(&layout),
                &BindGroupEntries::sequential((
                    uniform.binding().unwrap(),
                    &curve_view,
                    &curve_sampler,
                )),
            );
            Binding {
                _uniform: uniform,
                group,
            }
        });
        Ok(Self {
            _texture: texture,
            view,
            _curves: curves,
            bindings,
            pipeline,
        })
    }
    pub fn output(&self) -> &TextureView {
        &self.view
    }
    pub fn ready(&self, cache: &PipelineCache) -> bool {
        cache.get_render_pipeline(self.pipeline).is_some()
    }
    pub fn failure(&self, cache: &PipelineCache) -> Option<String> {
        match cache.get_render_pipeline_state(self.pipeline) {
            CachedPipelineState::Err(
                bevy::shader::ShaderCacheError::ShaderNotLoaded(_)
                | bevy::shader::ShaderCacheError::ShaderImportNotYetAvailable,
            ) => None,
            CachedPipelineState::Err(e) => Some(format!("Source Neutral LUT pipeline: {e}")),
            _ => None,
        }
    }
    pub fn encode(&self, encoder: &mut CommandEncoder, cache: &PipelineCache) -> bool {
        let Some(pipeline) = cache.get_render_pipeline(self.pipeline) else {
            return false;
        };
        for (slice, binding) in self.bindings.iter().enumerate() {
            let colors = [Some(RenderPassColorAttachment {
                view: &self.view,
                depth_slice: Some(slice as u32),
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(default()),
                    store: StoreOp::Store,
                },
            })];
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("source_neutral_lut_slice"),
                color_attachments: &colors,
                ..default()
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &binding.group, &[]);
            pass.draw(0..3, 0..1);
        }
        true
    }
}
