//! Source camera-depth dispatch reuses prepared opaque batches, never camera-visible shadow candidates.
use super::{
    caster::{CasterPasses, CasterViews},
    materials::{MaterialLoading, RenderingState},
    source_lighting::{DrawSourceMaterial, SetSourceViewCamera, SourceLightingStatus},
    surface_graph::SurfaceViewTargets,
};
use bevy::core_pipeline::core_3d::{AlphaMask3d, Opaque3d};
use bevy::ecs as bevy_ecs;
use bevy::ecs::{
    query::ROQueryItem,
    system::{SystemParamItem, lifetimeless::SRes},
};
use bevy::pbr::{DrawMesh, SetMeshBindGroup};
use bevy::prelude::*;
use bevy::render::{
    render_phase::*,
    render_resource::*,
    renderer::{RenderContext, ViewQuery},
    view::ExtractedView,
};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

#[derive(Component, Default)]
pub struct CameraDepthState {
    active: AtomicBool,
    counts: [AtomicU32; 3],
}
impl CameraDepthState {
    pub fn counts(&self) -> [u32; 3] {
        self.counts.each_ref().map(|n| n.load(Ordering::Relaxed))
    }
    fn begin(&self) -> DepthScope<'_> {
        self.counts[0].fetch_add(1, Ordering::Relaxed);
        self.counts[1].store(0, Ordering::Relaxed);
        self.counts[2].store(0, Ordering::Relaxed);
        self.active.store(true, Ordering::Relaxed);
        DepthScope(self)
    }
}
struct DepthScope<'a>(&'a CameraDepthState);
impl Drop for DepthScope<'_> {
    fn drop(&mut self) {
        self.0.active.store(false, Ordering::Relaxed);
    }
}
struct SetCameraDepth;
impl<P: CachedRenderPipelinePhaseItem> RenderCommand<P> for SetCameraDepth {
    type Param = (SRes<CasterPasses>, SRes<PipelineCache>);
    type ViewQuery = &'static CasterViews;
    type ItemQuery = ();
    fn render<'w>(
        item: &P,
        views: ROQueryItem<'w, '_, Self::ViewQuery>,
        _: Option<()>,
        (pipelines, cache): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        if !pipelines.classified(item.cached_pipeline()) {
            return RenderCommandResult::Failure(
                "Source camera-depth pipeline eligibility is missing",
            );
        }
        // Unlit and glass have no caster; effective queues above2500 have no camera-depth variant.
        let Some(id) = pipelines.pipeline(item.cached_pipeline(), false) else {
            return RenderCommandResult::Skip;
        };
        let Some(pipeline) = cache.into_inner().get_render_pipeline(id) else {
            return RenderCommandResult::Skip;
        };
        pass.set_render_pipeline(pipeline);
        pass.set_bind_group(1, views.binding(0), &[]);
        RenderCommandResult::Success
    }
}
struct DepthMark<const DONE: bool>;
impl<P: PhaseItem, const DONE: bool> RenderCommand<P> for DepthMark<DONE> {
    type Param = ();
    type ViewQuery = &'static CameraDepthState;
    type ItemQuery = ();
    fn render<'w>(
        _: &P,
        state: ROQueryItem<'w, '_, Self::ViewQuery>,
        _: Option<()>,
        _: SystemParamItem<'w, '_, Self::Param>,
        _: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        state.counts[1 + usize::from(DONE)].fetch_add(1, Ordering::Relaxed);
        RenderCommandResult::Success
    }
}
type DrawCameraDepth = (
    SetCameraDepth,
    DepthMark<false>,
    SetSourceViewCamera,
    SetMeshBindGroup<2>,
    super::caster_material::SetCasterMaterial,
    DrawMesh,
    DepthMark<true>,
);
struct Route<P: super::motion_draw::MotionBatchItem> {
    surface: Box<dyn Draw<P>>,
    depth: Box<dyn Draw<P>>,
    motion: Box<dyn Draw<P>>,
}
impl<P: super::motion_draw::MotionBatchItem> Draw<P> for Route<P> {
    fn prepare(&mut self, world: &World) {
        self.surface.prepare(world);
        self.depth.prepare(world);
        self.motion.prepare(world);
    }
    fn draw<'w>(
        &mut self,
        world: &'w World,
        pass: &mut TrackedRenderPass<'w>,
        view: Entity,
        item: &P,
    ) -> Result<(), DrawError> {
        if world
            .get::<super::motion_draw::MotionDrawView>(view)
            .is_some_and(|state| state.active())
        {
            return super::motion_draw::split_draw(world, pass, view, item, &mut *self.motion);
        }
        let state = world
            .get::<CameraDepthState>(view)
            .ok_or(DrawError::InvalidViewQuery)?;
        if state.active.load(Ordering::Relaxed) {
            self.depth.draw(world, pass, view, item)
        } else {
            self.surface.draw(world, pass, view, item)
        }
    }
}
/// Keep other material draw IDs inert in source-only depth and motion passes.
struct OtherMaterialRoute<P: PhaseItem> {
    original: Box<dyn Draw<P>>,
}
impl<P: PhaseItem> Draw<P> for OtherMaterialRoute<P> {
    fn prepare(&mut self, world: &World) {
        self.original.prepare(world);
    }
    fn draw<'w>(
        &mut self,
        world: &'w World,
        pass: &mut TrackedRenderPass<'w>,
        view: Entity,
        item: &P,
    ) -> Result<(), DrawError> {
        if world
            .get::<CameraDepthState>(view)
            .is_some_and(|state| state.active.load(Ordering::Relaxed))
            || world
                .get::<super::motion_draw::MotionDrawView>(view)
                .is_some_and(|state| state.active())
        {
            return Ok(());
        }
        self.original.draw(world, pass, view, item)
    }
}
struct Installing;
impl<P: PhaseItem> Draw<P> for Installing {
    fn draw<'w>(
        &mut self,
        _: &'w World,
        _: &mut TrackedRenderPass<'w>,
        _: Entity,
        _: &P,
    ) -> Result<(), DrawError> {
        Err(DrawError::RenderCommandFailure(
            "Source camera-depth dispatcher installation incomplete",
        ))
    }
}
fn wrap<P>(world: &mut World) -> Result<(), String>
where
    P: super::motion_draw::MotionBatchItem,
{
    let depth = Box::new(RenderCommandState::<P, DrawCameraDepth>::new(world));
    let motion = Box::new(RenderCommandState::<P, super::motion_draw::DrawMotion>::new(world));
    let Some(functions) = world.get_resource::<DrawFunctions<P>>() else {
        return Err("Source camera-depth draw table is missing".into());
    };
    let mut table = functions.write();
    let id = table
        .get_id::<DrawSourceMaterial>()
        .ok_or("Source material draw is not registered")?;
    let surface = std::mem::replace(
        &mut table.draw_functions[id.0 as usize],
        Box::new(Installing),
    );
    table.draw_functions[id.0 as usize] = Box::new(Route {
        surface,
        depth,
        motion,
    });
    for (index, function) in table.draw_functions.iter_mut().enumerate() {
        if index == id.0 as usize {
            continue;
        }
        let original = std::mem::replace(function, Box::new(Installing));
        *function = Box::new(OtherMaterialRoute { original });
    }
    Ok(())
}
/// Run once after the source draw registration. Each table keeps its original draw ID.
pub fn install(world: &mut World) {
    if let Err(error) = wrap::<Opaque3d>(world).and_then(|_| wrap::<AlphaMask3d>(world)) {
        *world.resource::<SourceLightingStatus>().0.lock().unwrap() = RenderingState::Failed(error);
    }
}
/// The single-sample camera depth exists independently of the source8x scene attachment.
/// Renderer cast-Off is deliberately irrelevant here. The installed camera-depth queue/pass rule owns eligibility.
pub fn camera_depth(
    world: &World,
    view: ViewQuery<(&ExtractedView, &SurfaceViewTargets, &CameraDepthState)>,
    opaque: Res<ViewBinnedRenderPhases<Opaque3d>>,
    masked: Res<ViewBinnedRenderPhases<AlphaMask3d>>,
    pipelines: Res<CasterPasses>,
    cache: Res<PipelineCache>,
    status: Res<SourceLightingStatus>,
    loading: Res<MaterialLoading>,
    mut context: RenderContext,
) {
    if !loading.is_ready()
        || !pipelines.ready(&cache)
        || matches!(*status.0.lock().unwrap(), RenderingState::Failed(_))
    {
        return;
    }
    let entity = view.entity();
    let (view, targets, state) = view.into_inner();
    let (Some(opaque), Some(masked)) = (
        opaque.get(&view.retained_view_entity),
        masked.get(&view.retained_view_entity),
    ) else {
        return;
    };
    let _scope = state.begin();
    let mut pass = context.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("source_camera_depth"),
        color_attachments: &[],
        depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
            view: &targets.0.depth.attachment,
            depth_ops: Some(Operations {
                load: LoadOp::Clear(0.),
                store: StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        ..default()
    });
    if let Err(error) = opaque
        .render(&mut pass, world, entity)
        .and_then(|_| masked.render(&mut pass, world, entity))
    {
        *status.0.lock().unwrap() =
            RenderingState::Failed(format!("Source camera-depth draw: {error:?}"));
        return;
    }
    let [epoch, attempts, successes] = state.counts();
    if attempts != successes {
        *status.0.lock().unwrap() = RenderingState::Failed(format!(
            "Source camera-depth skipped draw at epoch{epoch}: {successes}/{attempts}"
        ));
    }
}
