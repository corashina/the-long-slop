//! Source four-cascade atlas dispatch over the persistent caster phase.
use super::{
    materials::MaterialLoading,
    readiness::RendererStateHandle,
    shadow_phase::{ShadowDrawState, SourceShadow},
    shadow_queue::ShadowQueues,
    surface_graph::SurfaceViewTargets,
    surface_views::SourceSurfaceView,
    view_frame::SourceViewFrame,
};
use bevy::prelude::*;
use bevy::render::{
    render_phase::ViewBinnedRenderPhases,
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

pub(crate) fn render_atlas(
    world: &World,
    view: ViewQuery<(
        &ExtractedView,
        &SurfaceViewTargets,
        &ShadowDrawState,
        &SourceSurfaceView,
    )>,
    phases: Res<ViewBinnedRenderPhases<SourceShadow>>,
    cache: Res<PipelineCache>,
    frame: Res<SourceViewFrame>,
    queues: Res<ShadowQueues>,
    status: Res<RendererStateHandle>,
    loading: Res<MaterialLoading>,
    mut context: RenderContext,
) {
    if !loading.is_ready() || status.failure().is_some() {
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
        .batchable_meshes
        .keys()
        .chain(phase.unbatchable_meshes.keys())
        .any(|(key, _)| cache.get_render_pipeline(key.pipeline).is_none())
    {
        return;
    }
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
            status.fail(format!("Source shadow atlas draw: {error:?}"));
            return;
        }
    }
    drop(pass);
    let (epoch, attempted, issued) = state.counts();
    let instances = state.instance_counts();
    if attempted != issued
        || instances[0] != queues.instances[source.index]
        || instances[1] != queues.instances[source.index]
    {
        status.fail(format!(
            "Source shadow atlas skipped draw at epoch{epoch}: attempted{attempted:?}, issued{issued:?}, instances{instances:?}, required{:?}",
            queues.instances[source.index]
        ));
        return;
    }
    state.encoded(frame.frame);
}
