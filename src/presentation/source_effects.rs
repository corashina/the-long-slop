//! Source camera effects and legacy callback copies share one submitted stack token.
use super::{
    frame_copy::FrameCopy,
    frame_graph::{MotionFrame, SourceGraphProgress},
    lighting_assets::{LightingAssets, LightingRenderAssets},
    lighting_scene::SourceSceneLighting,
    materials::RenderingState,
    motion_draw::MotionDrawView,
    postprocessing::chain::{PostFrame, PostShaders, PostTextures, SourceAoChain, SourcePostChain},
    render_targets::{HDR_FORMAT, SourceColorTargets},
    source_lighting::{SourceExecutionGate, SourceLightingStatus},
    surface_graph::{SourceSurface, SourceSurfaceStage, SurfaceViewTargets},
    surface_views::{SourceSurfaceView, SurfaceResources},
    view_frame::SourceViewFrame,
};
use bevy::render::{
    Render, RenderApp, RenderSystems,
    extract_resource::{ExtractResource, ExtractResourcePlugin},
    render_asset::RenderAssets,
    render_resource::*,
    renderer::{
        RenderAdapter, RenderContext, RenderDevice, RenderGraph, RenderGraphSystems, RenderQueue,
        ViewQuery,
    },
    texture::GpuImage,
};
use bevy::{ecs as bevy_ecs, prelude::*, render as bevy_render};
use std::sync::Arc;

#[derive(Resource, Clone, Default, ExtractResource)]
struct EffectInput {
    scene: u32,
    frame: u32,
    shutter: f32,
    delta_seconds: f32,
    initialized: bool,
}
fn snapshot_input(
    frame: Res<SourceViewFrame>,
    assets: Option<Res<LightingAssets>>,
    drive: Res<crate::app::Drive>,
    mut input: ResMut<EffectInput>,
) {
    if input.scene != frame.scene {
        *input = EffectInput {
            scene: frame.scene,
            ..default()
        };
    }
    input.frame = 0;
    let Some(assets) = assets else { return };
    if !assets.loaded || frame.views[1].is_none() {
        return;
    }
    let settings = assets.record.postprocessing.motion_blur;
    if !input.initialized {
        input.shutter = settings.shutter_range[0];
        input.initialized = true;
    }
    let delta = drive.render_delta_seconds;
    let physics = &drive.runtime.physics;
    let body = if let Some(vehicle) = drive.runtime.area.player.seat_vehicle() {
        physics.chassis_handle(vehicle)
    } else {
        drive.runtime.area.player.body_handle()
    };
    let speed = physics.rapier().bodies[body].linvel().length() * 3.6;
    let speed_fraction = ((speed - settings.speed_range[0])
        / (settings.speed_range[1] - settings.speed_range[0]))
        .clamp(0., 1.);
    let target = settings.shutter_range[0]
        + (settings.shutter_range[1] - settings.shutter_range[0]) * speed_fraction;
    input.shutter += (target - input.shutter) * (settings.shutter_lerp * delta).clamp(0., 1.);
    input.delta_seconds = delta;
    input.frame = frame.frame;
}

#[derive(Clone, Copy, Default, serde::Serialize)]
pub(crate) struct EffectsAudit {
    scene: u32,
    first_token: u32,
    first_before: [[u32; 5]; 3],
    first_after: [[u32; 5]; 3],
    histories: [[u32; 5]; 3],
    frame: u32,
    token: u32,
    ao_frame: u32,
    ao_token: u32,
    post_mask: u8,
    callback_mask: u8,
    submitted_token: u32,
    pipelines_ready: bool,
    inputs_ready: bool,
    encoded_tokens: [u32; 3],
    completed_tokens: [u32; 3],
    extent: [u32; 2],
    chain_bytes: [u64; 3],
    ao_bytes: u64,
    callback_bytes: u64,
    total_bytes: u64,
    startup_counts: [u32; 3],
    resize_counts: [u32; 3],
    ao_startup_count: u32,
    ao_resize_count: u32,
    callback_allocations: u32,
    validation: u32,
    validated: bool,
    shutter: f32,
    delta_seconds: f32,
}
impl EffectsAudit {
    pub(crate) fn pipelines_ready(&self) -> bool {
        self.pipelines_ready && self.inputs_ready
    }
    pub(crate) fn complete(&self, frame: u32, token: u32) -> bool {
        self.frame == frame
            && self.token == token
            && self.submitted_token == token
            && self.completed_tokens == [token; 3]
            && self.post_mask == 7
            && self.callback_mask == 31
            && self.validated
    }
}
struct EffectTargets {
    colors: Arc<SourceColorTargets>,
    ao: Option<SourceAoChain>,
    chains: [Option<SourcePostChain>; 3],
    _scratch: Texture,
    scratch: TextureView,
    to_scratch: FrameCopy,
    copyback: FrameCopy,
}
#[derive(Resource, Default)]
pub(crate) struct SourceEffects {
    motion_revision: u32,
    targets: Option<EffectTargets>,
    audit: EffectsAudit,
}
impl SourceEffects {
    pub(crate) fn ready(&self, cache: &PipelineCache) -> bool {
        self.targets.as_ref().is_some_and(|targets| {
            targets.ao.as_ref().unwrap().ready(cache)
                && targets
                    .chains
                    .iter()
                    .all(|chain| chain.as_ref().unwrap().ready(cache))
                && targets.to_scratch.ready(cache)
                && targets.copyback.ready(cache)
        })
    }
    pub(crate) fn encoded(&self, index: usize, frame: u32, token: u32) -> bool {
        self.audit.frame == frame
            && self.audit.token == token
            && self.audit.encoded_tokens[index] == token
            && self.audit.post_mask & (1 << index) != 0
    }
    fn failure(&self, cache: &PipelineCache) -> Option<String> {
        let targets = self.targets.as_ref()?;
        targets
            .ao
            .as_ref()
            .unwrap()
            .failure(cache)
            .or_else(|| {
                targets
                    .chains
                    .iter()
                    .find_map(|chain| chain.as_ref().unwrap().failure(cache))
            })
            .or_else(|| targets.to_scratch.failure(cache))
            .or_else(|| targets.copyback.failure(cache))
    }
}
pub fn install(app: &mut App) {
    app.init_resource::<EffectInput>()
        .add_plugins(ExtractResourcePlugin::<EffectInput>::default())
        .add_systems(
            PostUpdate,
            snapshot_input.after(super::view_frame::snapshot),
        );
    app.sub_app_mut(RenderApp)
        .init_resource::<SourceEffects>()
        .add_systems(
            Render,
            prepare
                .after(RenderSystems::PrepareResources)
                .before(RenderSystems::Render),
        )
        .add_systems(RenderGraph, begin_frame.in_set(RenderGraphSystems::Begin))
        .add_systems(
            SourceSurface,
            (
                encode_ao
                    .in_set(SourceSurfaceStage::AfterOpaque)
                    .run_if(super::frame_graph::surface_ready)
                    .run_if(super::shadow_queue::collector_encoded),
                encode_post
                    .in_set(SourceSurfaceStage::Postprocessing)
                    .run_if(super::frame_graph::surface_ready)
                    .run_if(super::shadow_queue::collector_encoded),
            ),
        );
}
fn shaders(assets: &LightingRenderAssets) -> PostShaders {
    let s = &assets.shaders;
    PostShaders {
        copy: s.copy.clone(),
        taa: s.taa.clone(),
        motion_blur: s.motion_blur.clone(),
        histogram: s.histogram.clone(),
        exposure: s.exposure.clone(),
        bloom: s.bloom.clone(),
        lut: s.lut.clone(),
        uber: s.uber.clone(),
        ao_depth: s.ao_depth.clone(),
        ao_render: s.ao_render.clone(),
        ao_upsample: s.ao_upsample.clone(),
        ao_composite: s.ao_composite.clone(),
    }
}
fn prepare(
    mut effects: ResMut<SourceEffects>,
    frame: Res<SourceViewFrame>,
    surfaces: Option<Res<SurfaceResources>>,
    assets: Option<Res<LightingRenderAssets>>,
    images: Res<RenderAssets<GpuImage>>,
    adapter: Res<RenderAdapter>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    cache: Res<PipelineCache>,
    status: Res<SourceLightingStatus>,
) {
    if matches!(*status.0.lock().unwrap(), RenderingState::Failed(_)) {
        return;
    }
    if effects.motion_revision != frame.motion_revision {
        effects.motion_revision = frame.motion_revision;
        if let Some(targets) = effects.targets.as_mut() {
            for chain in &mut targets.chains {
                chain.as_mut().unwrap().reset_history();
            }
        }
    }
    if effects.audit.scene != frame.scene {
        if let Some(targets) = effects.targets.as_mut() {
            for chain in &mut targets.chains {
                chain.as_mut().unwrap().recreate();
            }
        }
        let a = &mut effects.audit;
        a.scene = frame.scene;
        a.first_token = 0;
        a.first_before = [[0; 5]; 3];
        a.first_after = [[0; 5]; 3];
        a.submitted_token = 0;
        a.completed_tokens = [0; 3];
        a.validation = 0;
        a.validated = false;
    }
    if let Some(error) = effects.failure(&cache) {
        *status.0.lock().unwrap() = RenderingState::Failed(error);
        return;
    }
    let (Some(surfaces), Some(assets)) = (surfaces, assets) else {
        return;
    };
    if effects
        .targets
        .as_ref()
        .is_some_and(|old| Arc::ptr_eq(&old.colors, &surfaces.targets.stack))
    {
        return;
    }
    let record = &assets.record.postprocessing;
    let image = |id| {
        assets
            .textures
            .get(id)
            .and_then(|handle| images.get(handle.id()))
    };
    let Some(dirt) = image(&record.bloom.dirt) else {
        return;
    };
    if record.blue_noise.iter().any(|id| image(id).is_none()) {
        return;
    }
    let textures = PostTextures {
        noise: std::array::from_fn(|i| &image(&record.blue_noise[i]).unwrap().texture_view),
        dirt: &dirt.texture_view,
        dirt_extent: [
            dirt.texture_descriptor.size.width,
            dirt.texture_descriptor.size.height,
        ],
    };
    let colors = surfaces.targets.stack.clone();
    let extent = [colors.width, colors.height];
    let shaders = shaders(&assets);
    let mut previous = effects.targets.take();
    let result = (|| -> Result<EffectTargets, String> {
        let ao = SourceAoChain::new(
            &adapter,
            &device,
            &queue,
            &cache,
            &shaders,
            surfaces.targets.views[1].depth.sampled.as_ref().unwrap(),
            extent,
            record,
            previous.as_mut().and_then(|old| old.ao.take()),
        )?;
        let mut chains = [None, None, None];
        for (index, chain) in chains.iter_mut().enumerate() {
            let layer = assets.record.cameras[index + 1]
                .postprocessing
                .as_ref()
                .unwrap();
            *chain = Some(SourcePostChain::new(
                &adapter,
                &device,
                &queue,
                &cache,
                &shaders,
                &textures,
                colors.resolved.sampled.as_ref().unwrap(),
                ao.raw_depth(),
                surfaces.targets.views[1].motion.sampled.as_ref().unwrap(),
                extent,
                record,
                layer.scope,
                layer.stop_nan,
                previous.as_mut().and_then(|old| old.chains[index].take()),
            )?);
        }
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("source_callback_scratch"),
            size: Extent3d {
                width: extent[0],
                height: extent[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: HDR_FORMAT,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let scratch = texture.create_view(&default());
        let (to_scratch, copyback) = if let Some(mut old) = previous.take() {
            old.to_scratch
                .rebind(&device, &cache, colors.resolved.sampled.as_ref().unwrap());
            old.copyback.rebind(&device, &cache, &scratch);
            (old.to_scratch, old.copyback)
        } else {
            (
                FrameCopy::new(
                    &device,
                    &cache,
                    shaders.copy.clone(),
                    colors.resolved.sampled.as_ref().unwrap(),
                    HDR_FORMAT,
                    1,
                ),
                FrameCopy::new(
                    &device,
                    &cache,
                    shaders.copy.clone(),
                    &scratch,
                    HDR_FORMAT,
                    8,
                ),
            )
        };
        Ok(EffectTargets {
            colors,
            ao: Some(ao),
            chains,
            _scratch: texture,
            scratch,
            to_scratch,
            copyback,
        })
    })();
    match result {
        Ok(targets) => {
            effects.audit.extent = extent;
            effects.audit.chain_bytes =
                std::array::from_fn(|i| targets.chains[i].as_ref().unwrap().logical_bytes);
            effects.audit.startup_counts =
                std::array::from_fn(|i| targets.chains[i].as_ref().unwrap().startup_count);
            effects.audit.resize_counts =
                std::array::from_fn(|i| targets.chains[i].as_ref().unwrap().resize_count);
            let ao = targets.ao.as_ref().unwrap();
            effects.audit.ao_bytes = ao.logical_bytes;
            effects.audit.ao_startup_count = ao.startup_count;
            effects.audit.ao_resize_count = ao.resize_count;
            effects.audit.callback_bytes = u64::from(extent[0]) * u64::from(extent[1]) * 8;
            effects.audit.total_bytes = effects.audit.chain_bytes.iter().sum::<u64>()
                + effects.audit.ao_bytes
                + effects.audit.callback_bytes;
            effects.audit.callback_allocations += 1;
            effects.audit.validation = 0;
            effects.audit.validated = false;
            effects.targets = Some(targets);
        }
        Err(error) => *status.0.lock().unwrap() = RenderingState::Failed(error),
    }
}
fn begin_frame(
    mut effects: ResMut<SourceEffects>,
    frame: Res<SourceViewFrame>,
    input: Res<EffectInput>,
    cache: Res<PipelineCache>,
) {
    effects.audit.frame = frame.frame;
    effects.audit.token = frame.main_token;
    effects.audit.ao_frame = 0;
    effects.audit.ao_token = 0;
    effects.audit.post_mask = 0;
    effects.audit.callback_mask = 0;
    effects.audit.encoded_tokens = [0; 3];
    effects.audit.pipelines_ready = effects.ready(&cache);
    effects.audit.inputs_ready = input.frame == frame.frame && input.initialized;
    effects.audit.shutter = input.shutter;
    effects.audit.delta_seconds = input.delta_seconds;
}
fn encode_ao(
    view: ViewQuery<(&SourceSurfaceView, &SurfaceViewTargets)>,
    mut effects: ResMut<SourceEffects>,
    frame: Res<SourceViewFrame>,
    scene: Res<SourceSceneLighting>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    cache: Res<PipelineCache>,
    status: Res<SourceLightingStatus>,
    mut context: RenderContext,
) {
    let (view, targets) = view.into_inner();
    if view.index != 1 || matches!(*status.0.lock().unwrap(), RenderingState::Failed(_)) {
        return;
    }
    let (Some(snapshot), Some(scene), Some(storage)) =
        (frame.views[1].as_ref(), scene.0, effects.targets.as_mut())
    else {
        return;
    };
    match storage.ao.as_mut().unwrap().encode(
        context.command_encoder(),
        &device,
        &queue,
        &cache,
        targets.0.colors.attachment(),
        targets.0.colors.resolve_target().unwrap(),
        snapshot.near,
        snapshot.far,
        snapshot.implicit_projection.x_axis.x,
        scene.fog_color_density.w,
    ) {
        Ok(true) => {
            effects.audit.ao_frame = frame.frame;
            effects.audit.ao_token = frame.main_token;
        }
        Ok(false) => {}
        Err(error) => *status.0.lock().unwrap() = RenderingState::Failed(error),
    }
}
fn encode_post(
    view: ViewQuery<(&SourceSurfaceView, Option<&MotionDrawView>)>,
    mut effects: ResMut<SourceEffects>,
    motion: Option<Res<MotionFrame>>,
    frame: Res<SourceViewFrame>,
    input: Res<EffectInput>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    cache: Res<PipelineCache>,
    status: Res<SourceLightingStatus>,
    mut context: RenderContext,
) {
    let (view, motion_draw) = view.into_inner();
    let index = view.index;
    if index >= 3
        || input.frame != frame.frame
        || matches!(*status.0.lock().unwrap(), RenderingState::Failed(_))
    {
        return;
    }
    if index == 1
        && (effects.audit.ao_frame != frame.frame
            || !motion
                .as_ref()
                .is_some_and(|m| m.encoded(frame.frame, frame.main_token))
            || !motion_draw.is_some_and(|m| m.encoded_token() == frame.main_token))
    {
        return;
    }
    let Some(snapshot) = frame.views[index].as_ref() else {
        return;
    };
    let Some(targets) = effects.targets.as_mut() else {
        return;
    };
    let encoded = targets.chains[index].as_mut().unwrap().encode(
        context.command_encoder(),
        &device,
        &queue,
        &cache,
        PostFrame {
            token: frame.main_token,
            jitter_uv: frame.jitter.unwrap().uv,
            near: snapshot.near,
            far: snapshot.far,
            shutter: input.shutter,
            delta_seconds: input.delta_seconds,
        },
        targets.colors.attachment(),
        targets.colors.resolve_target().unwrap(),
    );
    match encoded {
        Ok(true) => {}
        Ok(false) => return,
        Err(error) => {
            *status.0.lock().unwrap() = RenderingState::Failed(error);
            return;
        }
    }
    // Far: PP callback then native copyback. Main: PP callback then enabled zero-weight heat callback.
    if index < 2 {
        if !targets
            .to_scratch
            .encode(context.command_encoder(), &cache, &targets.scratch, None)
            || !targets.copyback.encode(
                context.command_encoder(),
                &cache,
                targets.colors.attachment(),
                targets.colors.resolve_target(),
            )
        {
            *status.0.lock().unwrap() =
                RenderingState::Failed("Source camera callback did not encode".into());
            return;
        }
        effects.audit.callback_mask |= 3 << (index * 2);
    }
    effects.audit.encoded_tokens[index] = frame.main_token;
    effects.audit.post_mask |= 1 << index;
}
pub(crate) fn submitted(
    mut effects: ResMut<SourceEffects>,
    frame: Res<SourceViewFrame>,
    progress: Res<SourceGraphProgress>,
    gate: Res<SourceExecutionGate>,
    queue: Res<RenderQueue>,
    status: Res<SourceLightingStatus>,
) {
    if matches!(*status.0.lock().unwrap(), RenderingState::Failed(_)) {
        return;
    }
    let (stamp, token, presented) = progress.frame_submission();
    if presented
        && stamp == frame.frame
        && token == frame.main_token
        && effects.audit.post_mask == 7
        && effects.audit.callback_mask == 15
        && effects.audit.encoded_tokens == [token; 3]
        && token > effects.audit.submitted_token
    {
        let targets = effects.targets.as_mut().unwrap();
        if targets
            .chains
            .iter()
            .any(|chain| chain.as_ref().unwrap().pending_token() != Some(token))
        {
            *status.0.lock().unwrap() =
                RenderingState::Failed("Source effect submission token mismatch".into());
            return;
        }
        let before = std::array::from_fn(|i| targets.chains[i].as_ref().unwrap().history());
        for chain in &mut targets.chains {
            if !chain.as_mut().unwrap().completed(token) {
                *status.0.lock().unwrap() =
                    RenderingState::Failed("Source effect history did not commit".into());
                return;
            }
        }
        let completed =
            std::array::from_fn(|i| targets.chains[i].as_ref().unwrap().completed_token());
        let after = std::array::from_fn(|i| targets.chains[i].as_ref().unwrap().history());
        effects.audit.completed_tokens = completed;
        effects.audit.histories = after;
        if effects.audit.first_token == 0 {
            effects.audit.first_token = token;
            effects.audit.first_before = before;
            effects.audit.first_after = after;
        }
        effects.audit.submitted_token = token;
        // Foreground's final callback is the existing display copy; presented proves its encode.
        effects.audit.callback_mask |= 16;
        if effects.audit.validation == 0 {
            effects.audit.validation = gate.watch_submission(&queue);
        }
    }
    effects.audit.validated = gate.submission_validated(effects.audit.validation);
    progress.publish_effects(effects.audit);
}
