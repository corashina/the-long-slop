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
    uber::{MainInputs, UberPass},
};
use crate::reference::postprocessing::{
    DeadGrading, DeadProfile, GlassesProfile, GradingSettings, PostprocessingRecord, Tonemapper,
};
use bevy::prelude::*;
use bevy::render::{
    render_resource::*,
    renderer::{RenderAdapter, RenderDevice, RenderQueue},
};

#[derive(Clone)]
pub struct PostShaders {
    pub copy: Handle<Shader>,
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
    /// `BinocularVignette::mask`.
    pub vignette_mask: &'a TextureView,
}
#[derive(Clone, Copy)]
pub struct PostFrame {
    pub token: u32,
    pub delta_seconds: f32,
    /// Main-view grading temperature. None keeps the applied value.
    pub grading_temperature: Option<f32>,
    /// `DeadV` weight over the main grading, 0 to 1.
    pub dead_weight: f32,
    /// `PPVSunGlasses` lens: index in `clothing.glasses_profiles`. None is no lens grading.
    pub glasses: Option<u32>,
    /// The binocular volume applies its masked vignette.
    pub binoculars: bool,
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
    histogram: HistogramPasses,
    exposure: ExposurePass,
    bloom: BloomPass,
    lut: LutPass,
    lut_baked: bool,
    /// Main grading with the applied temperature.
    base: GradingSettings,
    dead: DeadGrading,
    /// `clothing.glasses_profiles`.
    profiles: Vec<GlassesProfile>,
    /// Lens and dead weight of the applied grading.
    glasses: Option<u32>,
    dead_weight: f32,
    /// Grading and tonemapper in the LUT and Uber.
    grading: GradingSettings,
    tonemapper: Tonemapper,
}
pub struct SourcePostChain {
    pub extent: [u32; 2],
    pub logical_bytes: u64,
    pub startup_count: u32,
    pub resize_count: u32,
    initial: ColorTarget,
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
        extent: [u32; 2],
        record: &PostprocessingRecord,
        dead: &DeadProfile,
        glasses: &[GlassesProfile],
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
            let (old_histogram, old_exposure, old_bloom, old_lut, lut_baked, grading) =
                match old_main {
                    Some(old) => (
                        Some(old.histogram),
                        Some(old.exposure),
                        Some(old.bloom),
                        Some(old.lut),
                        old.lut_baked,
                        (
                            old.base,
                            old.dead,
                            old.profiles,
                            old.glasses,
                            old.dead_weight,
                            old.grading,
                            old.tonemapper,
                        ),
                    ),
                    None => {
                        let (dead, ignored) = DeadGrading::new(dead);
                        if !ignored.is_empty() {
                            warn!("DeadV parameters without a main stack effect: {ignored:?}");
                        }
                        (
                            None,
                            None,
                            None,
                            None,
                            false,
                            (
                                record.grading,
                                dead,
                                glasses.to_vec(),
                                None,
                                0.,
                                record.grading,
                                Tonemapper::Neutral,
                            ),
                        )
                    }
                };
            let (base, dead, profiles, glasses, dead_weight, grading, tonemapper) = grading;
            let histogram = HistogramPasses::new(
                device,
                queue,
                cache,
                shaders.histogram.clone(),
                &initial.view,
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
                &initial.view,
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
                histogram,
                exposure,
                bloom,
                lut,
                lut_baked,
                base,
                dead,
                profiles,
                glasses,
                dead_weight,
                grading,
                tonemapper,
            }
        };
        let inputs = MainInputs {
            bloom: main.bloom.output(),
            exposure: main.exposure.views(),
            dirt: textures.dirt,
            dirt_extent: textures.dirt_extent,
            vignette_mask: textures.vignette_mask,
            lut: main.lut.output(),
            sample_scale: main.bloom.layout.sample_scale,
        };
        let mut uber = UberPass::new(
            device,
            queue,
            cache,
            shaders.uber.clone(),
            &initial.view,
            textures.noise,
            extent,
            record,
            inputs,
            old_uber,
        )?;
        uber.set_post_exposure(main.grading.post_exposure);
        let area = u64::from(extent[0]) * u64::from(extent[1]);
        let logical_bytes = area * 8
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
            initial,
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
            && self.main.histogram.ready(cache)
            && self.main.exposure.ready(cache)
            && self.main.bloom.ready(cache)
            && self.main.lut.ready(cache)
    }
    pub fn failure(&self, cache: &PipelineCache) -> Option<String> {
        self.copy
            .failure(cache)
            .or_else(|| self.uber.failure(cache))
            .or_else(|| self.main.histogram.failure(cache))
            .or_else(|| self.main.exposure.failure(cache))
            .or_else(|| self.main.bloom.failure(cache))
            .or_else(|| self.main.lut.failure(cache))
    }
    pub fn recreate(&mut self) {
        self.pending = None;
        self.completed = 0;
        self.uber.recreate();
        self.main.exposure.recreate();
        self.main.lut_baked = false;
    }
    /// Noise index, exposure output/initialized.
    pub fn history(&self) -> [u32; 3] {
        let exposure = self.main.exposure.history();
        [self.uber.noise_index(), exposure[0], exposure[1]]
    }
    pub fn reset_history(&mut self) {
        self.pending = None;
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
        if !frame.delta_seconds.is_finite() || frame.delta_seconds < 0. {
            return Err("Invalid source effect frame".into());
        }
        self.copy
            .encode(encoder, cache, &self.initial.view, self.stop_nan);
        self.main.histogram.encode(encoder, cache);
        self.main
            .exposure
            .encode(encoder, device, queue, cache, frame.delta_seconds)?;
        let exposure_index = self
            .main
            .exposure
            .output_index()
            .ok_or("Source exposure output is missing")?;
        self.main.bloom.encode(encoder, cache, exposure_index);
        let mut changed =
            frame.glasses != self.main.glasses || frame.dead_weight != self.main.dead_weight;
        if let Some(temperature) = frame.grading_temperature {
            if !temperature.is_finite() {
                return Err("Invalid source grading temperature".into());
            }
            changed |= temperature != self.main.base.temperature;
            self.main.base.temperature = temperature;
        }
        if !(0. ..=1.).contains(&frame.dead_weight) {
            return Err("Invalid source dead weight".into());
        }
        if changed {
            // The lens overrides the main grading, then `DeadV` blends on top.
            let (mut grading, tonemapper) = match frame.glasses {
                Some(index) => self.main.base.with_glasses(
                    self.main
                        .profiles
                        .get(index as usize)
                        .ok_or("Invalid source glasses profile")?,
                ),
                None => (self.main.base, Tonemapper::Neutral),
            };
            self.main.dead.blend(&mut grading, frame.dead_weight);
            self.main.glasses = frame.glasses;
            self.main.dead_weight = frame.dead_weight;
            if grading != self.main.grading || tonemapper != self.main.tonemapper {
                self.main
                    .lut
                    .set_grading(device, queue, &grading, tonemapper);
                self.uber.set_post_exposure(grading.post_exposure);
                self.main.grading = grading;
                self.main.tonemapper = tonemapper;
                self.main.lut_baked = false;
            }
        }
        if !self.main.lut_baked {
            self.main.lut.encode(encoder, cache);
        }
        self.uber.set_vignette(frame.binoculars);
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
