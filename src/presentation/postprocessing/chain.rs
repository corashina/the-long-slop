//! Camera-local source effect chains. Allocate only at startup and physical resize.
use super::{
    HistogramPasses,
    ao_composite::AoCompositePass,
    ao_depth::AoDepthPass,
    ao_render::AoRenderPass,
    ao_upsample::AoUpsamplePass,
    bloom::BloomPass,
    copy::CopyPass,
    exposure::ExposurePass,
    lut::{CURVE_BYTES, LUT_BYTES, LutPass},
    motion_blur::MotionBlurPass,
    taa::TaaPass,
    uber::{MainInputs, UberPass},
};
use crate::reference::postprocessing::PostprocessingRecord;
use bevy::prelude::*;
use bevy::render::{
    render_resource::*,
    renderer::{RenderAdapter, RenderDevice, RenderQueue},
};

#[derive(Clone)]
pub struct PostShaders {
    pub copy: Handle<Shader>,
    pub taa: Handle<Shader>,
    pub motion_blur: Handle<Shader>,
    pub histogram: Handle<Shader>,
    pub exposure: Handle<Shader>,
    pub bloom: Handle<Shader>,
    pub lut: Handle<Shader>,
    pub uber: Handle<Shader>,
    pub ao_depth: Handle<Shader>,
    pub ao_render: Handle<Shader>,
    pub ao_upsample: Handle<Shader>,
    pub ao_composite: Handle<Shader>,
}
pub struct PostTextures<'a> {
    pub noise: [&'a TextureView; 64],
    pub dirt: &'a TextureView,
    pub dirt_extent: [u32; 2],
}
#[derive(Clone, Copy)]
pub struct PostFrame {
    pub token: u32,
    pub jitter_uv: [f32; 2],
    pub near: f32,
    pub far: f32,
    pub shutter: f32,
    pub delta_seconds: f32,
}
struct ColorTarget {
    _texture: Texture,
    view: TextureView,
}
impl ColorTarget {
    fn new(device: &RenderDevice, extent: [u32; 2]) -> Self {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("source_effect_color"),
            size: Extent3d {
                width: extent[0],
                height: extent[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba16Float,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&default());
        Self {
            _texture: texture,
            view,
        }
    }
}
struct MainChain {
    _taa_output: ColorTarget,
    taa: TaaPass,
    motion: MotionBlurPass,
    histogram: HistogramPasses,
    exposure: ExposurePass,
    bloom: BloomPass,
    lut: LutPass,
    lut_baked: bool,
}
pub struct SourcePostChain {
    pub extent: [u32; 2],
    pub logical_bytes: u64,
    pub startup_count: u32,
    pub resize_count: u32,
    _initial: ColorTarget,
    copy: CopyPass,
    uber: UberPass,
    main: MainChain,
    stop_nan: bool,
    completed: u32,
    pending: Option<u32>,
}
impl SourcePostChain {
    pub fn new(
        adapter: &RenderAdapter,
        device: &RenderDevice,
        queue: &RenderQueue,
        cache: &PipelineCache,
        shaders: &PostShaders,
        textures: &PostTextures<'_>,
        source: &TextureView,
        raw_depth: &TextureView,
        motion: &TextureView,
        extent: [u32; 2],
        record: &PostprocessingRecord,
        stop_nan: bool,
        previous: Option<Self>,
    ) -> Result<Self, String> {
        if extent.contains(&0)
            || extent.into_iter().max().unwrap() > device.limits().max_texture_dimension_2d
        {
            return Err("Invalid source postprocessing extent".into());
        }
        let startup_count = previous.as_ref().map_or(1, |old| old.startup_count);
        let resize_count = previous.as_ref().map_or(0, |old| old.resize_count + 1);
        let completed = previous.as_ref().map_or(0, |old| old.completed);
        let initial = ColorTarget::new(device, extent);
        let (mut copy, old_main, old_uber) = match previous {
            Some(old) => (old.copy, Some(old.main), Some(old.uber)),
            None => (
                CopyPass::new(device, cache, shaders.copy.clone(), source),
                None,
                None,
            ),
        };
        copy.rebind(device, cache, source);
        let main = {
            let (old_taa, old_motion, old_histogram, old_exposure, old_bloom, old_lut, lut_baked) =
                match old_main {
                    Some(old) => (
                        Some(old.taa),
                        Some(old.motion),
                        Some(old.histogram),
                        Some(old.exposure),
                        Some(old.bloom),
                        Some(old.lut),
                        old.lut_baked,
                    ),
                    None => (None, None, None, None, None, None, false),
                };
            let taa_output = ColorTarget::new(device, extent);
            let taa = TaaPass::new(
                device,
                queue,
                cache,
                shaders.taa.clone(),
                &initial.view,
                raw_depth,
                motion,
                extent,
                record.taa,
                old_taa,
            )?;
            let motion = MotionBlurPass::new(
                adapter,
                device,
                queue,
                cache,
                shaders.motion_blur.clone(),
                &taa_output.view,
                raw_depth,
                motion,
                extent,
                old_motion.as_ref(),
            )?;
            let histogram = HistogramPasses::new(
                device,
                queue,
                cache,
                shaders.histogram.clone(),
                motion.output(),
                extent[0],
                extent[1],
                old_histogram.as_ref(),
            )?;
            let exposure = if let Some(mut old) = old_exposure {
                old.rebind_histogram(device, cache, histogram.output());
                old
            } else {
                ExposurePass::new(
                    device,
                    queue,
                    cache,
                    shaders.exposure.clone(),
                    histogram.output(),
                    record.exposure,
                )?
            };
            let bloom = BloomPass::new(
                adapter,
                device,
                queue,
                cache,
                shaders.bloom.clone(),
                motion.output(),
                exposure.views(),
                extent,
                &record.bloom,
                old_bloom.as_ref(),
            )?;
            let lut = if let Some(old) = old_lut {
                old
            } else {
                LutPass::new(
                    adapter,
                    device,
                    queue,
                    cache,
                    shaders.lut.clone(),
                    &record.grading,
                )?
            };
            MainChain {
                _taa_output: taa_output,
                taa,
                motion,
                histogram,
                exposure,
                bloom,
                lut,
                lut_baked,
            }
        };
        let inputs = MainInputs {
            bloom: main.bloom.output(),
            exposure: main.exposure.views(),
            dirt: textures.dirt,
            dirt_extent: textures.dirt_extent,
            lut: main.lut.output(),
            sample_scale: main.bloom.layout.sample_scale,
        };
        let uber = UberPass::new(
            device,
            queue,
            cache,
            shaders.uber.clone(),
            main.motion.output(),
            textures.noise,
            extent,
            record,
            inputs,
            old_uber,
        )?;
        let area = u64::from(extent[0]) * u64::from(extent[1]);
        let logical_bytes = area * 16
            + main.taa.logical_bytes
            + main.motion.layout.logical_bytes()
            + main.histogram.layout.logical_bytes()
            + 8
            + main.bloom.layout.logical_bytes()
            + LUT_BYTES
            + CURVE_BYTES;
        Ok(Self {
            extent,
            logical_bytes,
            startup_count,
            resize_count,
            _initial: initial,
            copy,
            uber,
            main,
            stop_nan,
            completed,
            pending: None,
        })
    }
    pub fn completed_token(&self) -> u32 {
        self.completed
    }
    pub fn pending_token(&self) -> Option<u32> {
        self.pending
    }
    pub fn ready(&self, cache: &PipelineCache) -> bool {
        self.copy.ready(cache)
            && self.uber.ready(cache)
            && self.main.taa.ready(cache)
            && self.main.motion.ready(cache)
            && self.main.histogram.ready(cache)
            && self.main.exposure.ready(cache)
            && self.main.bloom.ready(cache)
            && self.main.lut.ready(cache)
    }
    pub fn failure(&self, cache: &PipelineCache) -> Option<String> {
        self.copy
            .failure(cache)
            .or_else(|| self.uber.failure(cache))
            .or_else(|| self.main.taa.failure(cache))
            .or_else(|| self.main.motion.failure(cache))
            .or_else(|| self.main.histogram.failure(cache))
            .or_else(|| self.main.exposure.failure(cache))
            .or_else(|| self.main.bloom.failure(cache))
            .or_else(|| self.main.lut.failure(cache))
    }
    pub fn recreate(&mut self) {
        self.pending = None;
        self.completed = 0;
        self.uber.recreate();
        self.main.taa.recreate();
        self.main.exposure.recreate();
        self.main.motion.reset_history();
        self.main.lut_baked = false;
    }
    /// Noise index, TAA next/initialized, exposure output/initialized.
    pub fn history(&self) -> [u32; 5] {
        let taa = self.main.taa.history();
        let exposure = self.main.exposure.history();
        [
            self.uber.noise_index(),
            taa[0],
            taa[1],
            exposure[0],
            exposure[1],
        ]
    }
    pub fn reset_history(&mut self) {
        self.pending = None;
        self.main.taa.reset_history();
        self.main.motion.reset_history();
        self.main.exposure.reset_history();
    }
    /// Image histories remain intact during floating-origin transport.
    pub fn rebase(&mut self, _delta: Vec3) {}
    pub fn encode(
        &mut self,
        encoder: &mut CommandEncoder,
        device: &RenderDevice,
        queue: &RenderQueue,
        cache: &PipelineCache,
        frame: PostFrame,
        output: &TextureView,
        resolve: &TextureView,
    ) -> Result<bool, String> {
        if frame.token == 0 || frame.token <= self.completed {
            return Ok(false);
        }
        if !self.ready(cache) {
            return Ok(false);
        }
        if !frame.jitter_uv.iter().all(|v| v.is_finite())
            || !frame.near.is_finite()
            || !frame.far.is_finite()
            || frame.near <= 0.
            || frame.far <= frame.near
            || !frame.shutter.is_finite()
            || !(0. ..=360.).contains(&frame.shutter)
            || !frame.delta_seconds.is_finite()
            || frame.delta_seconds < 0.
        {
            return Err("Invalid source effect frame".into());
        }
        self.copy
            .encode(encoder, cache, &self._initial.view, self.stop_nan);
        let exposure_index = {
            let main = &mut self.main;
            main.taa.encode(
                encoder,
                device,
                queue,
                cache,
                &main._taa_output.view,
                frame.jitter_uv,
            )?;
            main.motion
                .update(device, queue, frame.near, frame.far, frame.shutter);
            main.motion.encode(encoder, cache);
            main.histogram.encode(encoder, cache);
            main.exposure
                .encode(encoder, device, queue, cache, frame.delta_seconds)?;
            let index = main
                .exposure
                .output_index()
                .ok_or("Source exposure output is missing")?;
            main.bloom.encode(encoder, cache, index);
            if !main.lut_baked {
                main.lut.encode(encoder, cache);
            }
            index
        };
        if !self.uber.encode(
            encoder,
            device,
            queue,
            cache,
            output,
            resolve,
            exposure_index,
        ) {
            return Err("Source Uber did not encode a ready pipeline".into());
        }
        self.pending = Some(frame.token);
        Ok(true)
    }
    pub fn completed(&mut self, token: u32) -> bool {
        if token <= self.completed || self.pending != Some(token) {
            return false;
        }
        self.main.taa.completed();
        self.main.motion.completed();
        self.main.exposure.completed();
        self.main.lut_baked = true;
        self.uber.completed();
        self.completed = token;
        self.pending = None;
        true
    }
}

pub struct SourceAoChain {
    pub extent: [u32; 2],
    pub logical_bytes: u64,
    pub startup_count: u32,
    pub resize_count: u32,
    depth: AoDepthPass,
    render: AoRenderPass,
    upsample: AoUpsamplePass,
    composite: AoCompositePass,
}
impl SourceAoChain {
    pub fn new(
        adapter: &RenderAdapter,
        device: &RenderDevice,
        queue: &RenderQueue,
        cache: &PipelineCache,
        shaders: &PostShaders,
        camera_depth: &TextureView,
        extent: [u32; 2],
        record: &PostprocessingRecord,
        previous: Option<Self>,
    ) -> Result<Self, String> {
        let depth = AoDepthPass::new(
            adapter,
            device,
            queue,
            cache,
            shaders.ao_depth.clone(),
            camera_depth,
            extent,
            previous.as_ref().map(|old| &old.depth),
        )?;
        let render = AoRenderPass::new(
            adapter,
            device,
            queue,
            cache,
            shaders.ao_render.clone(),
            &depth,
            record.occlusion.intensity,
            record.occlusion.thickness,
            previous.as_ref().map(|old| &old.render),
        )?;
        let upsample = AoUpsamplePass::new(
            device,
            queue,
            cache,
            shaders.ao_upsample.clone(),
            &depth,
            &render,
            &record.occlusion,
            previous.as_ref().map(|old| &old.upsample),
        )?;
        let composite = AoCompositePass::new(
            device,
            queue,
            cache,
            shaders.ao_composite.clone(),
            &depth.copy.view,
            &upsample.output[3].view,
            previous.as_ref().map(|old| &old.composite),
        );
        let logical_bytes = depth.logical_bytes + render.logical_bytes + upsample.logical_bytes;
        Ok(Self {
            extent,
            logical_bytes,
            startup_count: previous.as_ref().map_or(1, |old| old.startup_count),
            resize_count: previous.as_ref().map_or(0, |old| old.resize_count + 1),
            depth,
            render,
            upsample,
            composite,
        })
    }
    pub fn raw_depth(&self) -> &TextureView {
        &self.depth.copy.view
    }
    pub fn ready(&self, cache: &PipelineCache) -> bool {
        self.depth.ready(cache)
            && self.render.ready(cache)
            && self.upsample.ready(cache)
            && self.composite.ready(cache)
    }
    pub fn failure(&self, cache: &PipelineCache) -> Option<String> {
        self.depth
            .failure(cache)
            .or_else(|| self.render.failure(cache))
            .or_else(|| self.upsample.failure(cache))
            .or_else(|| self.composite.failure(cache))
    }
    pub fn encode(
        &mut self,
        encoder: &mut CommandEncoder,
        device: &RenderDevice,
        queue: &RenderQueue,
        cache: &PipelineCache,
        color: &TextureView,
        resolve: &TextureView,
        near: f32,
        far: f32,
        projection_00: f32,
        fog_density: f32,
    ) -> Result<bool, String> {
        if !self.ready(cache) {
            return Ok(false);
        }
        if !near.is_finite()
            || !far.is_finite()
            || near <= 0.
            || far <= near
            || !projection_00.is_finite()
            || projection_00 <= 0.
            || !fog_density.is_finite()
            || fog_density < 0.
        {
            return Err("Invalid source AO frame".into());
        }
        self.depth
            .encode(encoder, device, queue, cache, near, far)?;
        self.render
            .encode(encoder, device, queue, cache, projection_00)?;
        self.upsample.encode(encoder, cache);
        self.composite.encode(
            encoder,
            device,
            queue,
            cache,
            color,
            resolve,
            near,
            far,
            fog_density,
        )
    }
}
