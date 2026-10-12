//! Source four-cascade atlas dispatch over the persistent caster phase.
use super::{
    materials::{MaterialLoading, RenderingState},
    shadow_phase::{ShadowDrawState, SourceShadow},
    shadow_queue::ShadowQueues,
    source_lighting::SourceLightingStatus,
    surface_graph::SurfaceViewTargets,
    surface_views::SourceSurfaceView,
    view_frame::SourceViewFrame,
};
use bevy::prelude::*;
use bevy::render::{
    render_phase::ViewSortedRenderPhases,
    render_resource::*,
    renderer::{RenderContext, ViewQuery},
    view::ExtractedView,
};

/// Target atlas placement. The collector uses the same rectangle for its atlas transform.
/// Coordinates use wgpu's top-left viewport convention; source projection conversion is separate.
pub fn tile_rect(atlas_size: u32, cascade: usize) -> [u32; 4] {
    assert!(atlas_size >= 2 && atlas_size % 2 == 0 && cascade < 4);
    let side = atlas_size / 2;
    [
        (cascade as u32 & 1) * side,
        (cascade as u32 >> 1) * side,
        side,
        side,
    ]
}

/// Required item counts precede draw commands, including commands which return Skip.
fn expected_draws(phase: &bevy::render::render_phase::SortedRenderPhase<SourceShadow>) -> [u32; 4] {
    let mut counts = [0; 4];
    for item in phase.items.values() {
        for (cascade, count) in counts.iter_mut().enumerate() {
            *count += u32::from(item.split_mask & (1 << cascade) != 0);
        }
    }
    counts
}

pub fn render_atlas(
    world: &World,
    view: ViewQuery<(
        &ExtractedView,
        &SurfaceViewTargets,
        &ShadowDrawState,
        &SourceSurfaceView,
    )>,
    phases: Res<ViewSortedRenderPhases<SourceShadow>>,
    cache: Res<PipelineCache>,
    frame: Res<SourceViewFrame>,
    queues: Res<ShadowQueues>,
    status: Res<SourceLightingStatus>,
    loading: Res<MaterialLoading>,
    mut context: RenderContext,
) {
    if !loading.is_ready() || matches!(*status.0.lock().unwrap(), RenderingState::Failed(_)) {
        return;
    }
    let entity = view.entity();
    let (view, targets, state, source) = view.into_inner();
    if frame.views[source.index].is_none()
        || queues.frame != frame.frame
        || !queues.ready[source.index]
    {
        return;
    }
    let Some(phase) = phases.get(&view.retained_view_entity) else {
        return;
    };
    // Pipeline preparation is asynchronous. No partial atlas can be published.
    if phase
        .items
        .values()
        .any(|item| cache.get_render_pipeline(item.pipeline).is_none())
    {
        return;
    }
    let expected = expected_draws(phase);
    state.begin();
    let mut pass = context.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("source_shadow_atlas"),
        color_attachments: &[],
        depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
            view: &targets.0.atlas.attachment,
            depth_ops: Some(Operations {
                load: LoadOp::Clear(0.),
                store: StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        ..default()
    });
    for cascade in 0..4 {
        state.select(cascade);
        let [x, y, width, height] = tile_rect(targets.0.atlas_size, cascade);
        pass.set_viewport(x as f32, y as f32, width as f32, height as f32, 0., 1.);
        pass.set_scissor_rect(x, y, width, height);
        if let Err(error) = phase.render(&mut pass, world, entity) {
            *status.0.lock().unwrap() =
                RenderingState::Failed(format!("Source shadow atlas draw: {error:?}"));
            return;
        }
    }
    drop(pass);
    let (epoch, attempted, issued) = state.counts();
    if expected != attempted || expected != issued {
        *status.0.lock().unwrap() = RenderingState::Failed(format!(
            "Source shadow atlas skipped draw at epoch{epoch}: expected{expected:?}, attempted{attempted:?}, issued{issued:?}"
        ));
        return;
    }
    state.encoded(frame.frame);
}
