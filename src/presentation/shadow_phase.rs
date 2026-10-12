//! Persistent source shadow items use Bevy's mesh extraction and instance buffers.
use super::caster::CasterViews;
use bevy::ecs as bevy_ecs;
use bevy::ecs::{query::ROQueryItem, system::SystemParamItem};
use bevy::pbr::{DrawMesh, SetMaterialBindGroup, SetMeshBindGroup};
use bevy::prelude::*;
use bevy::render::{
    render_phase::*, render_resource::CachedRenderPipelineId, sync_world::MainEntity,
    view::ExtractedView,
};
use std::{
    ops::Range,
    sync::atomic::{AtomicU32, Ordering},
};

/// One item retains all four recovered split bits. Batching cannot merge different masks.
pub struct SourceShadow {
    pub entity: (Entity, MainEntity),
    pub pipeline: CachedRenderPipelineId,
    pub draw: DrawFunctionId,
    pub split_mask: u8,
    pub indexed: bool,
    pub batch_range: Range<u32>,
    pub extra_index: PhaseItemExtraIndex,
}
impl PhaseItem for SourceShadow {
    const AUTOMATIC_BATCHING: bool = false;
    fn entity(&self) -> Entity {
        self.entity.0
    }
    fn main_entity(&self) -> MainEntity {
        self.entity.1
    }
    fn draw_function(&self) -> DrawFunctionId {
        self.draw
    }
    fn batch_range(&self) -> &Range<u32> {
        &self.batch_range
    }
    fn batch_range_mut(&mut self) -> &mut Range<u32> {
        &mut self.batch_range
    }
    fn extra_index(&self) -> PhaseItemExtraIndex {
        self.extra_index.clone()
    }
    fn batch_range_and_extra_index_mut(&mut self) -> (&mut Range<u32>, &mut PhaseItemExtraIndex) {
        (&mut self.batch_range, &mut self.extra_index)
    }
}
impl CachedRenderPipelinePhaseItem for SourceShadow {
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline
    }
}
impl SortedPhaseItem for SourceShadow {
    type SortKey = ();
    fn sort_key(&self) {}
    fn sort(_: &mut indexmap::IndexMap<(Entity, MainEntity), Self, bevy::ecs::entity::EntityHash>) {
    }
    fn recalculate_sort_keys(
        _: &mut indexmap::IndexMap<(Entity, MainEntity), Self, bevy::ecs::entity::EntityHash>,
        _: &ExtractedView,
    ) {
    }
    fn indexed(&self) -> bool {
        self.indexed
    }
}

/// The atlas stage changes this selector between four draws. Uniform slots remain independent.
#[derive(Component, Default)]
pub struct ShadowDrawState {
    cascade: AtomicU32,
    encoded_frame: AtomicU32,
    epoch: AtomicU32,
    attempts: [AtomicU32; 4],
    successes: [AtomicU32; 4],
}
impl ShadowDrawState {
    pub fn encoded_frame(&self) -> u32 {
        self.encoded_frame.load(Ordering::Acquire)
    }
    pub fn encoded(&self, frame: u32) {
        self.encoded_frame.store(frame, Ordering::Release);
    }
    pub fn begin(&self) {
        self.encoded_frame.store(0, Ordering::Release);
        self.epoch.fetch_add(1, Ordering::Relaxed);
        for count in self.attempts.iter().chain(self.successes.iter()) {
            count.store(0, Ordering::Relaxed);
        }
    }
    pub fn select(&self, cascade: usize) {
        assert!(cascade < 4);
        self.cascade.store(cascade as u32, Ordering::Relaxed);
    }
    pub fn counts(&self) -> (u32, [u32; 4], [u32; 4]) {
        (
            self.epoch.load(Ordering::Relaxed),
            self.attempts.each_ref().map(|v| v.load(Ordering::Relaxed)),
            self.successes.each_ref().map(|v| v.load(Ordering::Relaxed)),
        )
    }
}
struct SelectCaster;
impl RenderCommand<SourceShadow> for SelectCaster {
    type Param = ();
    type ViewQuery = (&'static ShadowDrawState, &'static CasterViews);
    type ItemQuery = ();
    fn render<'w>(
        item: &SourceShadow,
        (state, views): ROQueryItem<'w, '_, Self::ViewQuery>,
        _: Option<()>,
        _: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let cascade = state.cascade.load(Ordering::Relaxed) as usize;
        if item.split_mask & (1 << cascade) == 0 {
            return RenderCommandResult::Skip;
        }
        if item.batch_range.end != item.batch_range.start + 1 {
            return RenderCommandResult::Failure(
                "Source shadow requires one prepared mesh instance",
            );
        }
        pass.set_bind_group(1, views.binding(cascade + 1), &[]);
        RenderCommandResult::Success
    }
}
struct MarkDraw<const DONE: bool>;
impl<const DONE: bool> RenderCommand<SourceShadow> for MarkDraw<DONE> {
    type Param = ();
    type ViewQuery = &'static ShadowDrawState;
    type ItemQuery = ();
    fn render<'w>(
        _: &SourceShadow,
        state: ROQueryItem<'w, '_, Self::ViewQuery>,
        _: Option<()>,
        _: SystemParamItem<'w, '_, Self::Param>,
        _: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let index = state.cascade.load(Ordering::Relaxed) as usize;
        let counts = if DONE {
            &state.successes
        } else {
            &state.attempts
        };
        counts[index].fetch_add(1, Ordering::Relaxed);
        RenderCommandResult::Success
    }
}
type DrawSourceShadow = (
    SetItemPipeline,
    SelectCaster,
    MarkDraw<false>,
    super::source_lighting::SetSourceViewCamera,
    SetMeshBindGroup<2>,
    SetMaterialBindGroup<3>,
    DrawMesh,
    MarkDraw<true>,
);

/// Call during loading for each source view. This bound covers all possible source caster primitives.
pub fn reserved_phase(maximum_items: usize) -> SortedRenderPhase<SourceShadow> {
    let mut phase = SortedRenderPhase::default();
    phase.items.reserve(maximum_items);
    phase
}
/// Capacity failures are explicit. Hull changes never grow the queue during rendering.
pub fn add_candidate(
    phase: &mut SortedRenderPhase<SourceShadow>,
    item: SourceShadow,
) -> Result<(), &'static str> {
    if phase.items.len() == phase.items.capacity() && !phase.items.contains_key(&item.entity) {
        return Err("Source shadow queue capacity was exceeded");
    }
    if item.split_mask & !15 != 0 {
        return Err("Source shadow split mask has unsupported bits");
    }
    phase.add_retained(item);
    Ok(())
}

/// Register once during renderer setup; draw-table entries remain unchanged during frames.
pub fn register(app: &mut SubApp) {
    app.init_resource::<DrawFunctions<SourceShadow>>()
        .add_render_command::<SourceShadow, DrawSourceShadow>();
}
pub fn draw_id(functions: &DrawFunctions<SourceShadow>) -> DrawFunctionId {
    functions.read().id::<DrawSourceShadow>()
}
