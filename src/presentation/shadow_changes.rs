//! Main-world caster mutation inputs. ViewVisibility is refreshed separately every frame.
use super::{
    materials::{
        SourceMaterial, SourceMaterialWarmup, grass::material::GrassMaterial,
        impostor::ImpostorMaterial, specular::SpecularMaterial, terrain::TerrainMaterial,
    },
    renderers::SourceRenderer,
};
use bevy::ecs as bevy_ecs;
use bevy::{
    camera::{
        primitives::Aabb,
        visibility::{InheritedVisibility, RenderLayers, ViewVisibility},
    },
    ecs::{query::QueryData, system::SystemParam},
    prelude::*,
};

#[derive(QueryData)]
pub(super) struct ShadowMesh {
    pub entity: Entity,
    pub bounds: &'static Aabb,
    pub transform: &'static GlobalTransform,
    pub inherited: &'static InheritedVisibility,
    pub record: &'static SourceRenderer,
    pub layers: Option<&'static RenderLayers>,
    pub material: Option<&'static MeshMaterial3d<SourceMaterial>>,
    pub specular: Option<&'static MeshMaterial3d<SpecularMaterial>>,
    pub terrain: Option<&'static MeshMaterial3d<TerrainMaterial>>,
    pub grass: Option<&'static MeshMaterial3d<GrassMaterial>>,
    pub impostor: Option<&'static MeshMaterial3d<ImpostorMaterial>>,
}

#[derive(SystemParam)]
pub(super) struct ShadowMaterials<'w> {
    source: Res<'w, Assets<SourceMaterial>>,
    specular: Res<'w, Assets<SpecularMaterial>>,
    terrain: Res<'w, Assets<TerrainMaterial>>,
    grass: Res<'w, Assets<GrassMaterial>>,
    impostors: Res<'w, Assets<ImpostorMaterial>>,
}
impl ShadowMaterials<'_> {
    pub fn changed(&self) -> bool {
        self.source.is_changed()
            || self.specular.is_changed()
            || self.terrain.is_changed()
            || self.grass.is_changed()
            || self.impostors.is_changed()
    }
    pub fn has_caster(&self, mesh: &ShadowMeshItem<'_, '_>) -> Result<Option<bool>, &'static str> {
        if let Some(handle) = mesh.material {
            return self
                .source
                .get(handle.0.id())
                .map(|material| Some(material.extension.has_caster()))
                .ok_or("Missing source shadow material");
        }
        let available = if let Some(handle) = mesh.specular {
            Some((
                self.specular.contains(handle.0.id()),
                "Missing source specular material",
            ))
        } else if let Some(handle) = mesh.terrain {
            Some((
                self.terrain.contains(handle.0.id()),
                "Missing source terrain material",
            ))
        } else if let Some(handle) = mesh.grass {
            Some((
                self.grass.contains(handle.0.id()),
                "Missing source grass material",
            ))
        } else {
            mesh.impostor.map(|handle| {
                (
                    self.impostors.contains(handle.0.id()),
                    "Missing source impostor material",
                )
            })
        };
        match available {
            Some((true, _)) => Ok(Some(true)),
            Some((false, error)) => Err(error),
            None => Ok(None),
        }
    }
}

#[derive(SystemParam)]
pub(super) struct ShadowChanges<'w, 's> {
    pub changed: Query<
        'w,
        's,
        Entity,
        (
            With<SourceRenderer>,
            Or<(
                Changed<Aabb>,
                Changed<GlobalTransform>,
                Changed<InheritedVisibility>,
                Changed<SourceRenderer>,
                Changed<RenderLayers>,
                Added<ViewVisibility>,
                Changed<MeshMaterial3d<SourceMaterial>>,
                Changed<MeshMaterial3d<SpecularMaterial>>,
                Changed<MeshMaterial3d<TerrainMaterial>>,
                Changed<MeshMaterial3d<GrassMaterial>>,
                Changed<MeshMaterial3d<ImpostorMaterial>>,
            )>,
        ),
    >,
    pub warmup: Query<'w, 's, Entity, (With<SourceRenderer>, Added<SourceMaterialWarmup>)>,
    bounds: RemovedComponents<'w, 's, Aabb>,
    transforms: RemovedComponents<'w, 's, GlobalTransform>,
    inherited: RemovedComponents<'w, 's, InheritedVisibility>,
    visible: RemovedComponents<'w, 's, ViewVisibility>,
    renderers: RemovedComponents<'w, 's, SourceRenderer>,
    layers: RemovedComponents<'w, 's, RenderLayers>,
    source: RemovedComponents<'w, 's, MeshMaterial3d<SourceMaterial>>,
    specular: RemovedComponents<'w, 's, MeshMaterial3d<SpecularMaterial>>,
    terrain: RemovedComponents<'w, 's, MeshMaterial3d<TerrainMaterial>>,
    grass: RemovedComponents<'w, 's, MeshMaterial3d<GrassMaterial>>,
    impostors: RemovedComponents<'w, 's, MeshMaterial3d<ImpostorMaterial>>,
    excluded: RemovedComponents<'w, 's, SourceMaterialWarmup>,
}
impl ShadowChanges<'_, '_> {
    pub fn removed(&mut self) -> bool {
        // Consume every stream, including later removals after the first match.
        self.bounds
            .read()
            .chain(self.transforms.read())
            .chain(self.inherited.read())
            .chain(self.visible.read())
            .chain(self.renderers.read())
            .chain(self.layers.read())
            .chain(self.source.read())
            .chain(self.specular.read())
            .chain(self.terrain.read())
            .chain(self.grass.read())
            .chain(self.impostors.read())
            .chain(self.excluded.read())
            .count()
            != 0
    }
}
