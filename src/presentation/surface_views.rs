//! Persistent targets and lighting bindings for the world and probe views.
use super::{
    lighting_assets::LightingRenderAssets,
    lighting_scene::{PROBE_COUNT, SourceSceneLighting},
    materials::{MaterialLoading, RenderingState},
    render_targets::{HDR_FORMAT, MSAA_SAMPLES, ProbeTargets, SurfaceTargets},
    source_lighting::{SourceLightingStatus, SourceViewLighting},
    stained_glass::{GlassViewUniform, SourceGlassView},
    surface_graph::SurfaceViewTargets,
    transparent::SourceTransparentPasses,
};
use bevy::render::{
    extract_component::ExtractComponent,
    render_asset::RenderAssets,
    render_resource::*,
    renderer::{RenderDevice, RenderQueue},
    texture::GpuImage,
    view::ExtractedView,
};
use bevy::{ecs as bevy_ecs, prelude::*};

pub const WORLD_VIEWS: usize = 1;
pub const PROBE_START: usize = WORLD_VIEWS;
pub const VIEW_COUNT: usize = PROBE_START + 6;

/// The world view precedes six probe faces.
/// Shadow values come from each view's source fit and collector preparation.
#[derive(Component, Clone, Copy)]
pub struct SourceSurfaceView {
    pub index: usize,
    pub near: f32,
    pub far: f32,
    pub clip_from_world: Mat4,
    pub shadow_fade: Vec4,
    pub shadow_fade_center: Vec4,
}
impl bevy::render::sync_component::SyncComponent for SourceSurfaceView {
    type Target = Self;
}
impl ExtractComponent for SourceSurfaceView {
    type QueryData = &'static Self;
    type QueryFilter = ();
    type Out = Self;
    fn extract_component(
        item: bevy::ecs::query::QueryItem<'_, '_, Self::QueryData>,
    ) -> Option<Self> {
        Some(*item)
    }
}
#[derive(Resource)]
pub struct SurfaceResources {
    pub targets: SurfaceTargets,
    pub probes: [ProbeTargets; PROBE_COUNT],
    probe_sampler: Sampler,
    attenuation_sampler: Sampler,
}
impl SurfaceResources {
    pub fn new(
        device: &RenderDevice,
        queue: &RenderQueue,
        width: u32,
        height: u32,
        atlas: u32,
    ) -> Result<Self, String> {
        Ok(Self {
            targets: SurfaceTargets::new(device, width, height, atlas)?,
            probes: std::array::from_fn(|_| ProbeTargets::new(device, queue)),
            probe_sampler: device.create_sampler(&SamplerDescriptor {
                label: Some("source_probe_sampler"),
                mag_filter: FilterMode::Linear,
                min_filter: FilterMode::Linear,
                mipmap_filter: MipmapFilterMode::Linear,
                ..default()
            }),
            attenuation_sampler: device.create_sampler(&SamplerDescriptor {
                label: Some("source_collector_sampler"),
                mag_filter: FilterMode::Linear,
                min_filter: FilterMode::Linear,
                ..default()
            }),
        })
    }
}

pub fn prepare_views(
    mut commands: Commands,
    frame: Res<super::view_frame::SourceViewFrame>,
    assets: Option<Res<LightingRenderAssets>>,
    scene: Res<SourceSceneLighting>,
    loading: Option<Res<MaterialLoading>>,
    mut existing: Option<ResMut<SurfaceResources>>,
    images: Res<RenderAssets<GpuImage>>,
    mut views: Query<(
        Entity,
        &SourceSurfaceView,
        &ExtractedView,
        &Msaa,
        Option<&SurfaceViewTargets>,
        Option<&mut SourceViewLighting>,
        Option<&mut SourceGlassView>,
    )>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    cache: Res<PipelineCache>,
    status: Res<SourceLightingStatus>,
) {
    let mut timing = super::reflection_preparation::Timer::new(
        super::reflection_preparation::Stage::Surface,
        &frame,
    );
    if matches!(*status.0.lock().unwrap(), RenderingState::Failed(_)) {
        return;
    }
    let (Some(assets), Some(scene), Some(capacity)) = (
        assets,
        scene.0,
        loading
            .as_ref()
            .and_then(|value| value.transparent_pass_capacity()),
    ) else {
        return;
    };
    let mut extents = [None; VIEW_COUNT];
    for (_, source, view, msaa, _, _, _) in &mut views {
        if source.index >= VIEW_COUNT
            || extents[source.index].is_some()
            || view.target_format != HDR_FORMAT
            || msaa.samples()
                != if source.index < WORLD_VIEWS {
                    MSAA_SAMPLES
                } else {
                    1
                }
        {
            *status.0.lock().unwrap() = RenderingState::Failed(
                "Source surface view identity, format, or sample count is invalid".into(),
            );
            return;
        }
        if view.viewport.z == 0 || view.viewport.w == 0 {
            return;
        }
        extents[source.index] = Some(view.viewport.zw());
    }
    let Some(main) = extents[0] else { return };
    if extents[PROBE_START..]
        .iter()
        .flatten()
        .any(|size| *size != UVec2::splat(64))
    {
        *status.0.lock().unwrap() =
            RenderingState::Failed("Source surface view extents disagree".into());
        return;
    }
    if extents[PROBE_START..].iter().any(Option::is_none) {
        return;
    }
    let Some(snapshot) = frame.views[0] else {
        return;
    };
    let native = snapshot.native_extent;
    let Some(atlas) = crate::reference::shadows::source_atlas_size(native.x, native.y) else {
        *status.0.lock().unwrap() =
            RenderingState::Failed("Source world atlas extent is invalid".into());
        return;
    };
    let needs_targets = existing.as_ref().is_none_or(|value| {
        value.targets.world.width != main.x
            || value.targets.world.height != main.y
            || value.targets.views[0].atlas_size != atlas
    });
    if needs_targets {
        if let Some(existing) = existing.as_mut() {
            // Window resize does not invalidate the fixed-resolution published probe.
            match existing
                .targets
                .resize_world(&device, main.x, main.y, atlas)
            {
                Ok(()) => {}
                Err(error) => {
                    *status.0.lock().unwrap() = RenderingState::Failed(error);
                    return;
                }
            }
        } else {
            match SurfaceResources::new(&device, &queue, main.x, main.y, atlas) {
                Ok(value) => {
                    commands.insert_resource(value);
                }
                Err(error) => {
                    *status.0.lock().unwrap() = RenderingState::Failed(error);
                }
            }
            return;
        }
    }
    let resources = existing.unwrap();
    let cookie_id = assets.record.lights[0]
        .cookie
        .as_ref()
        .expect("validated sun cookie");
    let Some(cookie) = assets
        .textures
        .get(cookie_id)
        .and_then(|handle| images.get(handle.id()))
    else {
        return;
    };
    for (entity, source, _, _, bound, binding, glass) in &mut views {
        if frame.skip_probe_preparation(source.index) {
            continue;
        }
        timing.face(source.index);
        let target = &resources.targets.views[source.index];
        let uniform = scene.for_view(
            source.shadow_fade,
            source.shadow_fade_center,
            source.clip_from_world,
            source.near,
            source.far,
        );
        let glass_value = match GlassViewUniform::new(
            source.clip_from_world,
            source.near,
            source.far,
            UVec2::new(target.colors.width, target.colors.height),
        ) {
            Ok(value) => value,
            Err(error) => {
                *status.0.lock().unwrap() = RenderingState::Failed(error.into());
                return;
            }
        };
        if bound.is_some_and(|bound| std::sync::Arc::ptr_eq(&bound.0, target)) {
            let Some(mut binding) = binding else {
                *status.0.lock().unwrap() =
                    RenderingState::Failed("Source view lighting binding is missing".into());
                return;
            };
            let Some(mut glass) = glass else {
                *status.0.lock().unwrap() =
                    RenderingState::Failed("Source glass view binding is missing".into());
                return;
            };
            if let Err(error) = glass.write(glass_value, &device, &queue) {
                *status.0.lock().unwrap() = RenderingState::Failed(error.into());
                return;
            }
            binding.write(uniform, &device, &queue);
            continue;
        }
        let cubes = std::array::from_fn(|index| {
            if source.index < PROBE_START {
                &resources.probes[index].published_view
            } else {
                &resources.probes[index].capture_sample_view
            }
        });
        let glass = match SourceGlassView::new(&device, &queue, &cache, glass_value) {
            Ok(value) => value,
            Err(error) => {
                *status.0.lock().unwrap() = RenderingState::Failed(error.into());
                return;
            }
        };
        commands.entity(entity).insert((
            glass,
            SurfaceViewTargets(target.clone()),
            SourceViewLighting::new(
                &device,
                &queue,
                &cache,
                uniform,
                (&cookie.texture_view, &cookie.sampler),
                (cubes, &resources.probe_sampler),
                (
                    target.collector.sampled.as_ref().unwrap(),
                    &resources.attenuation_sampler,
                ),
            ),
            SourceTransparentPasses::new(capacity),
        ));
    }
}
