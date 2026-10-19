//! Camera membership is separate from the union used for mesh and shadow extraction.
use crate::reference::render_lighting::RendererRecord;
use bevy::camera::visibility::VisibleEntities;
use bevy::ecs::{
    self as bevy_ecs,
    entity::{EntityHashMap, EntityHashSet},
};
use bevy::prelude::*;
use std::any::TypeId;

#[derive(Component, Clone)]
pub struct SourceRenderer(pub RendererRecord);

#[derive(Component)]
pub struct SourceMaterialWarmup;

#[derive(Component, Clone, Copy)]
pub struct SourceSortData {
    pub material_key: u32,
    pub material_identity: u32,
    pub glass_source: i64,
    pub shader_identity: i32,
    pub source_material_index: u16,
    pub layer_and_order: u32,
    pub queue: i16,
    pub additive: bool,
    pub glass: bool,
}

#[derive(Resource, Default)]
pub struct CameraVisibility {
    pub entities: EntityHashSet,
    pub sort_data: EntityHashMap<SourceSortData>,
}

/// Keep the Mesh3d class even when empty so Bevy can remove retained phase items.
/// Retain order and capacity. ViewVisibility and shadow masks are not changed here.
pub fn filter_camera_visibility(
    mut views: Query<(&Camera, &mut VisibleEntities)>,
    meshes: Query<
        (
            Option<&SourceRenderer>,
            Option<&SourceSortData>,
            Has<SourceMaterialWarmup>,
        ),
        With<Mesh3d>,
    >,
    mut output: ResMut<CameraVisibility>,
) {
    output.entities.clear();
    output.sort_data.clear();
    let class = TypeId::of::<Mesh3d>();
    for (camera, mut visible) in &mut views {
        let entities = visible.get_mut(class);
        if !camera.is_active {
            entities.clear();
            continue;
        }
        entities.retain(|entity| {
            let Ok((renderer, sort, warmup)) = meshes.get(*entity) else {
                return false;
            };
            if !warmup && renderer.is_some_and(|renderer| renderer.0.cast_shadows == 3) {
                return false;
            }
            output.entities.insert(*entity);
            if let Some(sort) = sort {
                output.sort_data.insert(*entity, *sort);
            }
            true
        });
    }
}
