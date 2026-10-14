//! Source fuel station models on streamed stations.
use bevy::prelude::*;

use super::prototype::{Prototype, Recipe, SceneQueries};
use crate::presentation::lighting::{RenderingState, SourceLighting};
use crate::presentation::materials::SourceMaterial;
use crate::reference::StationsReference;
use crate::world::{CHUNK_LENGTH, STATION_SPACING, Station, active_chunks};
use bevy::world_serialization::WorldInstanceReady;

#[derive(Clone)]
struct StationInstance {
    station: Station,
    root: Entity,
}

#[derive(Clone)]
pub(crate) struct StationModels {
    /// Index matches `StationsReference::types`.
    prototypes: Vec<Prototype>,
    recipes: Vec<Option<Recipe>>,
    capacity: usize,
    instances: Vec<StationInstance>,
    origin: [f64; 3],
}

impl StationModels {
    pub fn new(reference: &StationsReference) -> Self {
        // Eleven chunks span 1408 meters: ceil(1408 / 640) = 3 stations.
        let capacity =
            (active_chunks(0.).len() as f64 * CHUNK_LENGTH / STATION_SPACING).ceil() as usize;
        Self {
            prototypes: reference
                .types
                .iter()
                .map(|t| Prototype::new(t.asset))
                .collect(),
            recipes: vec![None; reference.types.len()],
            capacity,
            instances: Vec::with_capacity(capacity),
            origin: [f64::INFINITY; 3],
        }
    }

    pub fn ready(&self) -> bool {
        self.recipes.iter().all(Option::is_some)
    }

    pub fn source_mesh_capacity(&self) -> usize {
        self.capacity
            * self
                .recipes
                .iter()
                .flatten()
                .map(Recipe::mesh_count)
                .max()
                .unwrap_or(0)
    }

    pub(crate) fn validate_assets(
        &self,
        path: &str,
        meshes: &Assets<Mesh>,
        materials: &Assets<SourceMaterial>,
        casters: &Assets<StandardMaterial>,
    ) -> Result<(), crate::save::SaveError> {
        let missing =
            |kind: &str| crate::save::SaveError::MissingAssets(format!("{path}: station {kind}"));
        for recipe in &self.recipes {
            recipe
                .as_ref()
                .ok_or_else(|| missing("recipe"))?
                .validate_assets(meshes, materials, casters)
                .map_err(missing)?;
        }
        Ok(())
    }

    pub(crate) fn clear_owned(&mut self) {
        self.instances.clear();
        self.origin = [f64::INFINITY; 3];
    }

    pub(crate) fn release(&self, world: &mut World) {
        for instance in &self.instances {
            world.despawn(instance.root);
        }
    }

    pub fn reset_instances(&mut self, commands: &mut Commands) {
        for instance in self.instances.drain(..) {
            commands.entity(instance.root).despawn();
        }
        self.origin = [f64::INFINITY; 3];
    }

    pub fn finish_warmup(&mut self, commands: &mut Commands) {
        for prototype in &mut self.prototypes {
            prototype.release(commands);
        }
    }
}

/// Prepare every station hierarchy before the loading gate permits Start.
pub(crate) fn prepare_stations(
    mut commands: Commands,
    reference: Res<crate::app::RuntimeReference>,
    visuals: Res<crate::presentation::reference::ReferenceVisuals>,
    gltfs: Res<Assets<bevy::gltf::Gltf>>,
    mut source: ResMut<SourceLighting>,
    scene: SceneQueries,
    source_materials: Res<crate::presentation::materials::SourceMaterials>,
) {
    if !source.bound || source.stations.ready() || matches!(source.state, RenderingState::Failed(_))
    {
        return;
    }
    let manifest = &reference.0;
    let source = &mut *source;
    for index in 0..source.stations.prototypes.len() {
        if source.stations.recipes[index].is_some() {
            continue;
        }
        let prototype = &mut source.stations.prototypes[index];
        if !prototype.spawned() {
            prototype.spawn(
                &mut commands,
                manifest,
                &visuals,
                &gltfs,
                move |_: On<WorldInstanceReady>, mut source: ResMut<SourceLighting>| {
                    source.stations.prototypes[index].ready = true;
                },
            );
            continue;
        }
        if !prototype.ready {
            continue;
        }
        if !prototype.bound {
            if let Err(error) = prototype.bind(&mut commands, manifest, &scene) {
                source.state = RenderingState::Failed(format!("Station: {error}"));
                return;
            }
            continue;
        }
        if !prototype.materials_ready(&scene) {
            continue;
        }
        match prototype.capture(
            &mut commands,
            manifest,
            &scene,
            &mut source.warmups,
            &source_materials,
        ) {
            Ok(recipe) => {
                info!(
                    "Source station cache {index}: {} nodes, {} meshes",
                    recipe.node_count(),
                    recipe.mesh_count()
                );
                source.stations.recipes[index] = Some(recipe);
            }
            Err(error) => {
                source.state = RenderingState::Failed(format!("Station: {error}"));
                return;
            }
        }
    }
}

/// Follow the stations of loaded chunks. Rebases move existing roots.
pub(crate) fn sync_stations(
    mut commands: Commands,
    drive: Res<crate::app::Drive>,
    mut source: ResMut<SourceLighting>,
) {
    if !source.bound
        || source.warming
        || !source.stations.ready()
        || matches!(source.state, RenderingState::Failed(_))
    {
        return;
    }
    let stream = &drive.runtime.stream;
    let source = &mut *source;
    let models = &mut source.stations;
    let loaded = || stream.chunks().flat_map(|chunk| &chunk.data.stations);
    let changed = loaded().count() != models.instances.len()
        || loaded().any(|s| !models.instances.iter().any(|i| i.station.id == s.id));
    let shifted = models.origin != stream.origin.offset;
    if !changed && !shifted {
        return;
    }
    if changed {
        if loaded().count() > models.capacity {
            source.state =
                RenderingState::Failed("Station ownership exceeds the proven stream bound".into());
            return;
        }
        models.instances.retain(|instance| {
            if loaded().any(|s| s.id == instance.station.id) {
                return true;
            }
            commands.entity(instance.root).despawn();
            false
        });
        for station in loaded() {
            if models.instances.iter().any(|i| i.station.id == station.id) {
                continue;
            }
            let root = commands.spawn(Visibility::Inherited).id();
            models.recipes[station.kind]
                .as_ref()
                .unwrap()
                .spawn(&mut commands, root);
            models.instances.push(StationInstance {
                station: *station,
                root,
            });
        }
    }
    for instance in &models.instances {
        commands.entity(instance.root).insert(
            Transform::from_translation(Vec3::from_array(
                stream.origin.local(instance.station.absolute_point),
            ))
            .with_rotation(Quat::from_array(instance.station.rotation)),
        );
    }
    models.origin = stream.origin.offset;
}
