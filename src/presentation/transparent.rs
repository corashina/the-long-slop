//! Reused source pass order over Bevy's prepared transparent mesh items.
use super::{
    materials::{MaterialLoading, RenderingState},
    source_lighting::{DrawSourceAdd, DrawSourceBase, SourceLightingStatus},
};
use crate::reference::sorting::{PassSortKey, perspective_distance};
use bevy::core_pipeline::core_3d::{Transparent3d, TransparentSortingInfo3d};
use bevy::render::{
    extract_component::ExtractComponent,
    render_phase::{
        DrawError, DrawFunctionId, DrawFunctions, PhaseItem, SortedRenderPhase, TrackedRenderPass,
        ViewSortedRenderPhases,
    },
    sync_world::MainEntity,
    view::ExtractedView,
};
use bevy::{ecs as bevy_ecs, prelude::*};
use std::ops::Range;

#[derive(Component, Clone, Copy)]
pub struct SourceSortData {
    pub material_key: u32,
    pub shader_identity: i32,
    pub source_material_index: u16,
    pub layer_and_order: u32,
    pub queue: i16,
    pub additive: bool,
    pub glass: bool,
}
impl bevy::render::sync_component::SyncComponent for SourceSortData {
    type Target = Self;
}
impl ExtractComponent for SourceSortData {
    type QueryData = &'static Self;
    type QueryFilter = ();
    type Out = Self;
    fn extract_component(
        item: bevy::ecs::query::QueryItem<'_, '_, Self::QueryData>,
    ) -> Option<Self> {
        Some(*item)
    }
}
/// Bevy mesh phases retain main IDs and use a placeholder render entity.
#[derive(Resource, Default)]
pub struct SourceSortIndex(bevy::ecs::entity::EntityHashMap<(Entity, SourceSortData)>);
pub fn update_sort_index(
    changed: Query<
        (Entity, &MainEntity, &SourceSortData),
        Or<(Changed<SourceSortData>, Changed<MainEntity>)>,
    >,
    mut removed: RemovedComponents<SourceSortData>,
    mut index: ResMut<SourceSortIndex>,
) {
    for entity in removed.read() {
        index.0.retain(|_, (render, _)| *render != entity);
    }
    for (render, main, data) in &changed {
        index.0.insert(main.id(), (render, *data));
    }
}
#[derive(Clone, Copy)]
struct OrderedPass {
    key: PassSortKey,
    item: usize,
    draw: DrawFunctionId,
}
/// Allocate during view loading. The fixed scene cannot exceed this capacity.
#[derive(Component)]
pub struct SourceTransparentPasses {
    entries: Vec<OrderedPass>,
}
impl SourceTransparentPasses {
    pub fn new(maximum_passes: usize) -> Self {
        Self {
            entries: Vec::with_capacity(maximum_passes),
        }
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn allocated_bytes(&self) -> usize {
        self.entries.capacity() * std::mem::size_of::<OrderedPass>()
    }
    /// Task5 can end a range and capture scene color at an ordered grab boundary.
    pub fn render_range<'w>(
        &self,
        range: Range<usize>,
        phase: &SortedRenderPhase<Transparent3d>,
        pass: &mut TrackedRenderPass<'w>,
        world: &'w World,
        view: Entity,
    ) -> Result<(), DrawError> {
        let functions = world.resource::<DrawFunctions<Transparent3d>>();
        let mut functions = functions.write();
        functions.prepare(world);
        for entry in &self.entries[range] {
            let item = phase
                .items
                .get_index(entry.item)
                .expect("prepared source pass index")
                .1;
            if !item.batch_range().is_empty() {
                functions
                    .get_mut(entry.draw)
                    .expect("registered source pass")
                    .draw(world, pass, view, item)?;
            }
        }
        Ok(())
    }
}
pub fn prepare_pass_order(
    mut views: Query<(&ExtractedView, &mut SourceTransparentPasses)>,
    phases: Res<ViewSortedRenderPhases<Transparent3d>>,
    metadata: Res<SourceSortIndex>,
    loading: Res<MaterialLoading>,
    functions: Res<DrawFunctions<Transparent3d>>,
    status: Res<SourceLightingStatus>,
) {
    if !loading.is_ready() || matches!(*status.0.lock().unwrap(), RenderingState::Failed(_)) {
        for (_, mut order) in &mut views {
            order.entries.clear();
        }
        return;
    }
    let functions = functions.read();
    let base = functions.id::<DrawSourceBase>();
    let add = functions.id::<DrawSourceAdd>();
    for (view, mut order) in &mut views {
        order.entries.clear();
        let Some(phase) = phases.get(&view.retained_view_entity) else {
            continue;
        };
        let camera = view.world_from_view.translation().to_array();
        for (index, item) in phase.items.values().enumerate() {
            let Some((_, data)) = metadata.0.get(&item.main_entity().id()) else {
                *status.0.lock().unwrap() =
                    RenderingState::Failed("Source transparent sort metadata is missing".into());
                return;
            };
            if data.glass {
                continue;
            }
            let TransparentSortingInfo3d::Sorted { mesh_center, .. } = item.sorting_info else {
                *status.0.lock().unwrap() =
                    RenderingState::Failed("Source transparent distance mode is invalid".into());
                return;
            };
            let distance = perspective_distance(mesh_center.to_array(), camera);
            if !distance.is_finite() || index > i32::MAX as usize {
                *status.0.lock().unwrap() = RenderingState::Failed(
                    "Source transparent distance or gather index is invalid".into(),
                );
                return;
            }
            let count = 1 + usize::from(data.additive);
            if order.entries.len() + count > order.entries.capacity() {
                *status.0.lock().unwrap() =
                    RenderingState::Failed("Source transparent pass capacity was exceeded".into());
                return;
            }
            let key = PassSortKey {
                layer_and_order: data.layer_and_order,
                queue: data.queue,
                distance,
                sorting_group: 0xfffff000,
                same_distance_priority: 0,
                node_index: index as i32,
                source_material_index: data.source_material_index,
                material_key: data.material_key,
                pass_data: 0x01000000,
                static_batch_first: 0,
                static_batch_count: 0,
                small_mesh_index: 0,
                lightmap_index: 0xffff,
                shader_identity: data.shader_identity,
                gather_index: index as i32,
            };
            order.entries.push(OrderedPass {
                key,
                item: index,
                draw: base,
            });
            if data.additive {
                order.entries.push(OrderedPass {
                    key: PassSortKey {
                        pass_data: 1,
                        ..key
                    },
                    item: index,
                    draw: add,
                });
            }
        }
        order.entries.sort_unstable_by(|a, b| a.key.compare(&b.key));
    }
}
