//! Startup mesh variants preserve source renderer state across shared geometry.
use super::materials::{RenderingState, SourceMaterials};
use super::reference::ReferenceVisuals;
use crate::reference::render_lighting::RendererRecord;
use bevy::asset::AssetId;
use bevy::ecs as bevy_ecs;
use bevy::gltf::{GltfExtras, GltfMaterialExtras};
use bevy::mesh::MeshVertexAttribute;
use bevy::prelude::*;
use bevy::render::render_resource::VertexFormat;
use std::collections::HashMap;

pub const SOURCE_BOUNDS_CENTER: MeshVertexAttribute = MeshVertexAttribute::new(
    "SourceBoundsCenterUsage",
    0x5352430000000001,
    VertexFormat::Float32x4,
);
pub const SOURCE_BOUNDS_EXTENTS: MeshVertexAttribute = MeshVertexAttribute::new(
    "SourceBoundsExtentsFlags",
    0x5352430000000002,
    VertexFormat::Float32x4,
);
#[derive(Component, Clone)]
pub struct SourceRenderer(pub RendererRecord);
#[derive(Resource, Default)]
pub struct RendererMeshes {
    pub bound: bool,
    pub variants: usize,
    pub attribute_bytes: usize,
    meshes: HashMap<(AssetId<Mesh>, [u32; 8]), Handle<Mesh>>,
}
#[derive(serde::Deserialize)]
struct RendererExtras {
    source_renderer: RendererRecord,
}

pub fn bind_renderer_meshes(
    mut commands: Commands,
    mut state: ResMut<RendererMeshes>,
    mut materials: ResMut<SourceMaterials>,
    visuals: Res<ReferenceVisuals>,
    mut meshes: ResMut<Assets<Mesh>>,
    primitives: Query<
        (Entity, &ChildOf, &Mesh3d),
        (
            With<GltfMaterialExtras>,
            Without<SourceRenderer>,
            Without<super::materials::SourceMaterialWarmup>,
        ),
    >,
    extras: Query<&GltfExtras>,
) {
    if !visuals.bound || matches!(materials.state, RenderingState::Failed(_)) {
        return;
    }
    if primitives.is_empty() {
        return;
    }
    // Wait before changing any handle. All variants are made in one startup pass.
    if primitives
        .iter()
        .any(|(_, _, mesh)| !meshes.contains(mesh.0.id()))
    {
        return;
    }
    let state = &mut *state;
    let variants = &mut state.meshes;
    let mut bytes = 0;
    for (entity, parent, mesh) in &primitives {
        let record = extras
            .get(parent.parent())
            .map_err(|error| error.to_string())
            .and_then(|extras| {
                serde_json::from_str::<RendererExtras>(&extras.value)
                    .map(|value| value.source_renderer)
                    .map_err(|error| error.to_string())
            });
        let record = match record {
            Ok(record) => record,
            Err(error) => {
                materials.state =
                    RenderingState::Failed(format!("Source renderer metadata: {error}"));
                return;
            }
        };
        if let Err(error) = record.validate() {
            materials.state = RenderingState::Failed(format!("Source renderer metadata: {error}"));
            return;
        }
        let key = (mesh.0.id(), record.mesh_variant_key());
        let variant = variants
            .entry(key)
            .or_insert_with(|| {
                let mut variant = meshes
                    .get(mesh.0.id())
                    .expect("checked source mesh")
                    .clone();
                let vertices = variant.count_vertices();
                let [center, extents] = record.attributes();
                variant.insert_attribute(SOURCE_BOUNDS_CENTER, vec![center; vertices]);
                variant.insert_attribute(SOURCE_BOUNDS_EXTENTS, vec![extents; vertices]);
                bytes += vertices * 32;
                meshes.add(variant)
            })
            .clone();
        commands.entity(entity).insert((
            Mesh3d(variant),
            bevy::camera::primitives::Aabb {
                center: Vec3::from_array(record.bounds_center).into(),
                half_extents: Vec3::from_array(record.bounds_extents).into(),
            },
            SourceRenderer(record),
        ));
    }
    state.variants = variants.len();
    state.attribute_bytes += bytes;
    state.bound = true;
    info!(
        "Source renderer meshes: {} variants, {} attribute bytes",
        state.variants, bytes
    );
}

/// Bevy batches retain main entity IDs and can use a placeholder render entity.
#[derive(Resource, Default)]
pub struct ShadowsOnly(std::collections::HashSet<Entity>);
pub(crate) fn extract_shadows_only(
    source: bevy::render::Extract<
        Query<(Entity, &SourceRenderer), Without<super::save::HiddenWorld>>,
    >,
    mut target: ResMut<ShadowsOnly>,
) {
    target.0.clear();
    target.0.extend(
        source
            .iter()
            .filter(|(_, r)| r.0.cast_shadows == 3)
            .map(|(e, _)| e),
    );
}
/// ShadowsOnly renderers remain in light passes and skip camera passes.
pub struct SourceCameraVisible;
impl<P: bevy::render::render_phase::PhaseItem> bevy::render::render_phase::RenderCommand<P>
    for SourceCameraVisible
{
    type Param = bevy::ecs::system::lifetimeless::SRes<ShadowsOnly>;
    type ViewQuery = ();
    type ItemQuery = ();
    fn render<'w>(
        item: &P,
        _: (),
        _: Option<()>,
        only: bevy::ecs::system::SystemParamItem<'w, '_, Self::Param>,
        _: &mut bevy::render::render_phase::TrackedRenderPass<'w>,
    ) -> bevy::render::render_phase::RenderCommandResult {
        if only.0.contains(&item.main_entity().id()) {
            bevy::render::render_phase::RenderCommandResult::Skip
        } else {
            bevy::render::render_phase::RenderCommandResult::Success
        }
    }
}
