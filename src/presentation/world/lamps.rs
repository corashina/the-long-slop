//! Shared source pole recipes and absolute stream ownership.
use bevy::prelude::*;

use super::prototype::{Prototype, Recipe, SceneQueries};
use crate::presentation::lighting::{
    RenderingState, SourceLighting, register_road_light, remove_road_light,
};
use crate::presentation::materials::SourceMaterial;
use crate::reference::AssetId;
use crate::world::{
    CHUNK_LENGTH, ChunkKey, RoadLampId, RoadLampPlacement, active_chunks, road_lamp_placement,
    road_lamp_range,
};
use bevy::world_serialization::WorldInstanceReady;

#[derive(Clone)]
pub(crate) struct RoadLampRecipe {
    recipe: Recipe,
    light_node: usize,
}
#[derive(Clone)]
struct RoadLampInstance {
    placement: RoadLampPlacement,
    root: Entity,
    light: Entity,
    slot: usize,
    // Repeated source identities belong only to this instance.
    _nodes: Vec<(crate::reference::NodeId, Entity)>,
}
#[derive(Clone)]
pub(crate) struct RoadLamps {
    pub capacity: usize,
    pub fixed_lights: usize,
    pub fixed_shadows: usize,
    pub recipe: Option<RoadLampRecipe>,
    prototype: Prototype,
    prototype_light: Option<Entity>,
    instances: Vec<RoadLampInstance>,
    keys: Option<[ChunkKey; 11]>,
    lateral_band: i64,
    origin: [f64; 3],
}
impl RoadLamps {
    pub(crate) fn validate_assets(
        &self,
        path: &str,
        meshes: &Assets<Mesh>,
        materials: &Assets<SourceMaterial>,
        casters: &Assets<StandardMaterial>,
    ) -> Result<(), crate::save::SaveError> {
        let recipe = self.recipe.as_ref().ok_or_else(|| {
            crate::save::SaveError::MissingAssets(format!("{path}: road lamp recipe"))
        })?;
        recipe
            .recipe
            .validate_assets(meshes, materials, casters)
            .map_err(|kind| {
                crate::save::SaveError::MissingAssets(format!("{path}: road lamp {kind}"))
            })
    }

    pub(crate) fn clear_owned(&mut self) {
        self.instances.clear();
        self.keys = None;
        self.origin = [f64::INFINITY; 3];
    }
    pub(crate) fn release(&self, world: &mut World) {
        for instance in &self.instances {
            world.despawn(instance.root);
            world.despawn(instance.light);
        }
    }

    pub fn source_mesh_capacity(&self) -> usize {
        self.recipe
            .as_ref()
            .map_or(0, |recipe| self.capacity * recipe.recipe.mesh_count())
    }
    pub fn reset_instances(&mut self, commands: &mut Commands) {
        for instance in self.instances.drain(..) {
            commands.entity(instance.root).despawn();
        }
        self.keys = None;
        self.origin = [f64::INFINITY; 3];
    }
    pub fn new(asset: AssetId, spacing: f64) -> Self {
        // Eleven consecutive source intervals span 1408 metres: ceil(1408 / 100) = 15.
        let capacity = (active_chunks(0.).len() as f64 * CHUNK_LENGTH / spacing).ceil() as usize;
        Self {
            capacity,
            fixed_lights: 0,
            fixed_shadows: 0,
            recipe: None,
            prototype: Prototype::new(asset),
            prototype_light: None,
            instances: Vec::with_capacity(capacity),
            keys: None,
            lateral_band: 0,
            origin: [f64::INFINITY; 3],
        }
    }
}

/// Prepare the reusable source hierarchy before the loading gate permits Start.
pub(crate) fn prepare_road_lamp(
    mut commands: Commands,
    reference: Res<crate::app::RuntimeReference>,
    visuals: Res<crate::presentation::reference::ReferenceVisuals>,
    gltfs: Res<Assets<bevy::gltf::Gltf>>,
    mut source: ResMut<SourceLighting>,
    scene: SceneQueries,
) {
    if !source.bound
        || source.road_lamps.recipe.is_some()
        || matches!(source.state, RenderingState::Failed(_))
    {
        return;
    }
    let manifest = &reference.0;
    let road = &manifest.lighting.road_lamp;
    if !source.road_lamps.prototype.spawned() {
        source.road_lamps.prototype.spawn(
            &mut commands,
            manifest,
            &visuals,
            &gltfs,
            |_: On<WorldInstanceReady>, mut source: ResMut<SourceLighting>| {
                source.road_lamps.prototype.ready = true;
            },
        );
        return;
    }
    if !source.road_lamps.prototype.ready {
        return;
    }
    if !source.road_lamps.prototype.bound {
        if let Err(error) = source
            .road_lamps
            .prototype
            .bind(&mut commands, manifest, &scene)
        {
            source.state = RenderingState::Failed(format!("Road lamp: {error}"));
            return;
        }
        let light = manifest
            .lighting
            .lights
            .iter()
            .find(|l| l.id == road.lights[0])
            .unwrap();
        if light.kind != crate::reference::LightKind::Spot || light.cookie.is_some() {
            source.state =
                RenderingState::Failed("Road lamp requires the source default spot recipe".into());
            return;
        }
        let node = source
            .road_lamps
            .prototype
            .nodes
            .iter()
            .find(|(id, _)| Some(*id) == light.node)
            .unwrap()
            .1;
        let entity = register_road_light(
            &mut source,
            &mut commands,
            &manifest.lighting,
            0,
            node,
            true,
        );
        source.road_lamps.prototype_light = Some(entity);
        return;
    }
    if !source.road_lamps.prototype.materials_ready(&scene) {
        return;
    }
    let source = &mut *source;
    let recipe = match source.road_lamps.prototype.capture(
        &mut commands,
        manifest,
        &scene,
        &mut source.warmups,
    ) {
        Ok(recipe) => recipe,
        Err(error) => {
            source.state = RenderingState::Failed(format!("Road lamp: {error}"));
            return;
        }
    };
    let light = manifest
        .lighting
        .lights
        .iter()
        .find(|l| l.id == road.lights[0])
        .unwrap();
    let light_node = recipe.node_index(light.node.unwrap()).unwrap();
    info!(
        "Source road lamp cache: {} nodes, {} meshes, {} reserved poles",
        recipe.node_count(),
        recipe.mesh_count(),
        source.road_lamps.capacity
    );
    source.road_lamps.recipe = Some(RoadLampRecipe { recipe, light_node });
}

pub(crate) fn finish_road_lamp_warmup(commands: &mut Commands, source: &mut SourceLighting) {
    if let Some(light) = source.road_lamps.prototype_light.take() {
        remove_road_light(source, commands, light);
    }
    source.road_lamps.prototype.release(commands);
}

/// Change ownership only when chunk keys or the lateral band change. Rebases change existing poses.
pub(crate) fn sync_road_lamps(
    mut commands: Commands,
    reference: Res<crate::app::RuntimeReference>,
    drive: Res<crate::app::Drive>,
    mut source: ResMut<SourceLighting>,
) {
    sync_instances(&mut commands, &reference.0, &drive.runtime, &mut source);
}
pub(crate) fn sync_instances(
    commands: &mut Commands,
    reference: &crate::reference::ReferenceManifest,
    runtime: &crate::starting_area::GarageRuntime,
    source: &mut SourceLighting,
) {
    if !source.bound
        || source.warming
        || source.road_lamps.recipe.is_none()
        || matches!(source.state, RenderingState::Failed(_))
    {
        return;
    }
    let stream = &runtime.stream;
    let Some(first) = stream.chunks().map(|c| c.data.key.0).min() else {
        return;
    };
    let keys = std::array::from_fn(|i| ChunkKey(first + i as i64));
    let band = stream.lateral_band();
    let changed = source.road_lamps.keys != Some(keys) || source.road_lamps.lateral_band != band;
    let shifted = source.road_lamps.origin != stream.origin.offset;
    if !changed && !shifted {
        return;
    }
    if stream.chunks().count() != keys.len()
        || keys.iter().any(|key| {
            !stream
                .chunks()
                .any(|c| c.data.key == *key && c.data.lateral_band == band)
        })
    {
        source.state = RenderingState::Failed(
            "Road lamp stream requires eleven consecutive ownership intervals".into(),
        );
        return;
    }
    let road = &reference.lighting.road_lamp;
    let range = if band.abs() <= 1 {
        road_lamp_range(
            first as f64 * CHUNK_LENGTH,
            (first + keys.len() as i64) as f64 * CHUNK_LENGTH,
            road.spacing_meters as f64,
        )
    } else {
        0..0
    };
    if (range.end - range.start) as usize > source.road_lamps.capacity {
        source.state = RenderingState::Failed(
            "Source road lamp ownership exceeds the proven stream bound".into(),
        );
        return;
    }
    if changed {
        for index in (0..source.road_lamps.instances.len()).rev() {
            if range.contains(&source.road_lamps.instances[index].placement.id.0) {
                continue;
            }
            let instance = source.road_lamps.instances.swap_remove(index);
            remove_road_light(source, commands, instance.light);
            commands.entity(instance.root).despawn();
        }
        let output = runtime.lighting.road_lamp_output(&reference.lighting);
        for id in range {
            if source
                .road_lamps
                .instances
                .iter()
                .any(|i| i.placement.id == RoadLampId(id))
            {
                continue;
            }
            let slot = (0..source.road_lamps.capacity)
                .find(|slot| !source.road_lamps.instances.iter().any(|i| i.slot == *slot))
                .unwrap();
            let placement = road_lamp_placement(stream.seed, RoadLampId(id), road);
            let root = commands
                .spawn((
                    Transform::from_translation(Vec3::from_array(
                        stream.origin.local(placement.absolute_point),
                    ))
                    .with_rotation(Quat::from_array(placement.rotation))
                    .with_scale(Vec3::from_array(road.road_pose.scale)),
                    Visibility::Inherited,
                ))
                .id();
            let recipe = source.road_lamps.recipe.as_ref().unwrap();
            let nodes = recipe.recipe.spawn(commands, root);
            let light_node = nodes[recipe.light_node].1;
            let light = register_road_light(
                source,
                commands,
                &reference.lighting,
                slot,
                light_node,
                output.enabled,
            );
            source.road_lamps.instances.push(RoadLampInstance {
                placement,
                root,
                light,
                slot,
                _nodes: nodes,
            });
        }
    }
    if shifted {
        for instance in &source.road_lamps.instances {
            commands.entity(instance.root).insert(
                Transform::from_translation(Vec3::from_array(
                    stream.origin.local(instance.placement.absolute_point),
                ))
                .with_rotation(Quat::from_array(instance.placement.rotation))
                .with_scale(Vec3::from_array(road.road_pose.scale)),
            );
        }
    }
    if changed {
        info!(
            "Source road lamp window: {} poles, {} registered lights, {} shadow views",
            source.road_lamps.instances.len(),
            source.lights.len(),
            source.road_lamps.fixed_shadows + source.road_lamps.instances.len()
        );
    }
    source.road_lamps.keys = Some(keys);
    source.road_lamps.lateral_band = band;
    source.road_lamps.origin = stream.origin.offset;
}
