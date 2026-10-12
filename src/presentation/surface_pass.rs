//! Cached source ForwardBase and ForwardAdd pipelines. Transparent dispatch uses source pass order.
use super::materials::RenderingState;
use super::source_lighting::SourceLightingStatus;
use bevy::ecs::{
    query::ROQueryItem,
    system::{SystemParamItem, lifetimeless::SRes},
};
use bevy::render::{
    render_phase::{
        CachedRenderPipelinePhaseItem, RenderCommand, RenderCommandResult, TrackedRenderPass,
    },
    render_resource::*,
};
use bevy::shader::{ShaderCacheError, ShaderDefVal};
use bevy::{ecs as bevy_ecs, prelude::*};
use std::collections::HashMap;

#[derive(Resource, Default)]
pub struct SurfacePasses {
    inspected: usize,
    partners: HashMap<CachedRenderPipelineId, Option<CachedRenderPipelineId>>,
}
impl SurfacePasses {
    pub fn counts(&self) -> (usize, usize) {
        (
            self.partners.len(),
            self.partners
                .values()
                .filter(|value| value.is_some())
                .count(),
        )
    }
    pub fn ready(&self, cache: &PipelineCache) -> bool {
        !self.partners.is_empty()
            && self.partners.iter().all(|(base, add)| {
                cache.get_render_pipeline(*base).is_some()
                    && add.is_none_or(|id| cache.get_render_pipeline(id).is_some())
            })
    }
}
fn flag(definitions: &[ShaderDefVal], name: &str) -> bool {
    definitions
        .iter()
        .any(|value| matches!(value,ShaderDefVal::Bool(key,true) if key==name))
}
/// Inspect only newly prepared descriptors. Pipeline clones occur once per new source variant.
pub fn prepare_surface_passes(
    mut passes: ResMut<SurfacePasses>,
    cache: Res<PipelineCache>,
    status: Res<SourceLightingStatus>,
) {
    if matches!(
        *status.0.lock().expect("source lighting status lock"),
        RenderingState::Failed(_)
    ) {
        return;
    }
    let first = passes.inspected;
    for (index, pipeline) in cache.pipelines().enumerate().skip(first) {
        passes.inspected = index + 1;
        let PipelineDescriptor::RenderPipelineDescriptor(descriptor) = &pipeline.descriptor else {
            continue;
        };
        let Some(fragment) = &descriptor.fragment else {
            continue;
        };
        if !flag(&fragment.shader_defs, "SOURCE_SURFACE")
            || flag(&fragment.shader_defs, "SOURCE_FORWARD_ADD")
        {
            continue;
        }
        let base = CachedRenderPipelineId::new(index);
        // Installed Unlit/Color and stained glass have no ForwardAdd pass.
        let partner = if flag(&fragment.shader_defs, "SOURCE_STANDARD") {
            let mut add = descriptor.clone();
            add.label = Some("source_surface_add".into());
            let fragment = add.fragment.as_mut().unwrap();
            fragment.shader_defs.retain(
                |value| !matches!(value,ShaderDefVal::Bool(name,_) if name == "SOURCE_REPLACEMENT"),
            );
            fragment
                .shader_defs
                .push(ShaderDefVal::Bool("SOURCE_FORWARD_ADD".into(), true));
            for target in fragment.targets.iter_mut().flatten() {
                let source = target
                    .blend
                    .map_or(BlendFactor::One, |blend| blend.color.src_factor);
                let blend = BlendComponent {
                    src_factor: source,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                };
                target.blend = Some(BlendState {
                    color: blend,
                    alpha: blend,
                });
                target.write_mask = ColorWrites::ALL;
            }
            add.primitive.cull_mode = Some(Face::Back);
            let depth = add.depth_stencil.as_mut().expect("source surface depth");
            depth.depth_write_enabled = Some(false);
            depth.depth_compare = Some(CompareFunction::GreaterEqual);
            Some(cache.queue_render_pipeline(*add))
        } else {
            None
        };
        passes.partners.insert(base, partner);
    }
    for (base, add) in &passes.partners {
        for id in std::iter::once(*base).chain(*add) {
            match cache.get_render_pipeline_state(id) {
                CachedPipelineState::Err(
                    ShaderCacheError::ShaderNotLoaded(_)
                    | ShaderCacheError::ShaderImportNotYetAvailable,
                ) => {}
                CachedPipelineState::Err(error) => {
                    *status.0.lock().expect("source lighting status lock") =
                        RenderingState::Failed(format!(
                            "Source surface {} pipeline: {error}",
                            if id == *base { "base" } else { "additive" }
                        ));
                    return;
                }
                _ => {}
            }
        }
    }
}
/// Prevent a partial pair while its additive pipeline is still being prepared.
pub struct SourcePairReady;
impl<P: CachedRenderPipelinePhaseItem> RenderCommand<P> for SourcePairReady {
    type Param = (SRes<SurfacePasses>, SRes<PipelineCache>);
    type ViewQuery = ();
    type ItemQuery = ();
    fn render<'w>(
        item: &P,
        _: ROQueryItem<Self::ViewQuery>,
        _: Option<()>,
        (passes, cache): SystemParamItem<'w, '_, Self::Param>,
        _: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(add) = passes.partners.get(&item.cached_pipeline()) else {
            return RenderCommandResult::Skip;
        };
        if cache.get_render_pipeline(item.cached_pipeline()).is_none()
            || add.is_some_and(|id| cache.get_render_pipeline(id).is_none())
        {
            return RenderCommandResult::Skip;
        }
        RenderCommandResult::Success
    }
}
pub struct SetSourceAdditivePipeline;
impl<P: CachedRenderPipelinePhaseItem> RenderCommand<P> for SetSourceAdditivePipeline {
    type Param = (SRes<SurfacePasses>, SRes<PipelineCache>);
    type ViewQuery = ();
    type ItemQuery = ();
    fn render<'w>(
        item: &P,
        _: ROQueryItem<Self::ViewQuery>,
        _: Option<()>,
        (passes, cache): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(Some(id)) = passes.partners.get(&item.cached_pipeline()) else {
            return RenderCommandResult::Skip;
        };
        let Some(pipeline) = cache.into_inner().get_render_pipeline(*id) else {
            return RenderCommandResult::Skip;
        };
        pass.set_render_pipeline(pipeline);
        RenderCommandResult::Success
    }
}
