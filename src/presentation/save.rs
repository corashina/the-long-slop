use super::{lighting, materials, reference::*, world::WorldVisuals};
use crate::{
    reference::{AssetUsage, ReferenceManifest},
    save::SaveError,
    starting_area::GarageRuntime,
};
use bevy::{ecs::world::CommandQueue, prelude::*};

/// Borrows the loaded assets, reference nodes, lighting, and entity queries together.
pub struct SaveAssetView<'a> {
    pub world: &'a mut World,
}

pub struct PreparedPresentation {
    commands: CommandQueue,
    updates: Vec<(Entity, Transform, Visibility)>,
    chunks: WorldVisuals,
    lighting: lighting::SourceLighting,
}

pub fn prepare_presentation(
    reference: &ReferenceManifest,
    candidate: &GarageRuntime,
    assets: SaveAssetView<'_>,
) -> Result<PreparedPresentation, SaveError> {
    let world = assets.world;
    validate_assets(reference, world)?;
    let mut updates = Vec::new();
    let mut query = world.query::<(
        Entity,
        &Transform,
        &Visibility,
        Option<&BodyVisual>,
        Option<&ItemVisual>,
        Option<&CapVisual>,
        Option<&ValveVisibility>,
    )>();
    for (entity, transform, visible, body, item, cap, valve) in query.iter(world) {
        if body.is_none() && item.is_none() && cap.is_none() && valve.is_none() {
            continue;
        }
        let mut pose = *transform;
        let mut visibility = *visible;
        let mut active = true;
        if let Some(binding) = body {
            let handle = candidate.area.bodies.get(&binding.body).ok_or_else(|| {
                SaveError::InvalidLinks(format!("Missing visual body {}", binding.body.0))
            })?;
            active = candidate.physics.body_active(*handle);
            let (_, _, saved) = candidate
                .area
                .render_poses
                .iter()
                .find(|(id, _, _)| *id == binding.body)
                .ok_or_else(|| SaveError::InvalidLinks("Missing body render pose".into()))?;
            pose = super::transform(*saved).with_scale(binding.scale);
        } else if let Some(binding) = item {
            let saved = candidate.area.items.get(binding.item).ok_or_else(|| {
                SaveError::InvalidLinks(format!("Missing visual item {}", binding.item.0))
            })?;
            active = saved.inactive_pose.is_none();
            let (_, _, rendered) = candidate
                .area
                .item_render_poses
                .iter()
                .find(|(id, _, _)| *id == binding.item)
                .ok_or_else(|| SaveError::InvalidLinks("Missing item render pose".into()))?;
            pose = super::transform(*rendered).with_scale(binding.scale);
        }
        if body.is_some() || item.is_some() || valve.is_some() {
            visibility = if active
                && valve.is_none_or(|v| {
                    (candidate.area.interaction.opening_valve(v.opening) > 0.) == v.open
                }) {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
        if let Some(cap) = cap {
            let amount = candidate.area.interaction.opening_valve(cap.opening);
            pose = cap.closed;
            if cap.euler {
                let a = cap.closed_euler.lerp(cap.open_euler, amount) * std::f32::consts::PI / 180.;
                pose.rotation = Quat::from_rotation_y(a.y)
                    * Quat::from_rotation_x(a.x)
                    * Quat::from_rotation_z(a.z);
            } else {
                pose.translation = cap.closed.translation.lerp(cap.opened.translation, amount);
                pose.rotation = cap
                    .closed
                    .rotation
                    .lerp(cap.opened.rotation, amount)
                    .normalize();
            }
        }
        updates.push((entity, pose, visibility));
    }
    let mut prepared = PreparedPresentation {
        commands: CommandQueue::default(),
        updates,
        chunks: world.resource::<WorldVisuals>().empty_copy(),
        lighting: world.resource::<lighting::SourceLighting>().prepare_copy(),
    };
    world.resource_scope(|world, mut meshes: Mut<Assets<Mesh>>| {
        let mut commands = Commands::new(&mut prepared.commands, world);
        prepared
            .chunks
            .sync(&candidate.stream, &mut commands, &mut meshes);
        super::world::lamps::sync_instances(
            &mut commands,
            reference,
            candidate,
            &mut prepared.lighting,
        );
    });
    if let lighting::RenderingState::Failed(error) = &prepared.lighting.state {
        let error = SaveError::MissingAssets(error.clone());
        prepared.discard(world);
        return Err(error);
    }
    // Old terrain meshes may have left MAIN_WORLD. These newly prepared meshes must exist now.
    if let Err(error) = prepared
        .chunks
        .validate_prepared_terrain(world.resource::<Assets<Mesh>>())
    {
        prepared.discard(world);
        return Err(error);
    }
    Ok(prepared)
}

fn validate_assets(reference: &ReferenceManifest, world: &World) -> Result<(), SaveError> {
    let fail = |path: String| SaveError::MissingAssets(path);
    let server = world.resource::<AssetServer>();
    let gltfs = world.resource::<Assets<bevy::gltf::Gltf>>();
    let visuals = world.resource::<ReferenceVisuals>();
    let source = world.resource::<lighting::SourceLighting>();
    let materials = world.resource::<materials::SourceMaterials>();
    if visuals.failed || !visuals.bound {
        return Err(fail("Reference bindings are not ready".into()));
    }
    for asset in &reference.assets {
        let (_, handle) = visuals
            .handles
            .iter()
            .find(|(path, _)| *path == asset.path)
            .ok_or_else(|| fail(asset.path.clone()))?;
        if !server.is_loaded_with_dependencies(handle.id())
            || gltfs.get(handle).is_none_or(|gltf| gltf.scenes.is_empty())
        {
            return Err(fail(asset.path.clone()));
        }
        if asset.usage == AssetUsage::UniqueScene {
            for node in reference.nodes.iter().filter(|node| node.asset == asset.id) {
                let entity = visuals
                    .nodes
                    .get(&node.id)
                    .ok_or_else(|| fail(format!("{}: {}", asset.path, node.hierarchy_path)))?;
                if world.get::<Transform>(*entity).is_none()
                    || world.get::<bevy::gltf::GltfExtras>(*entity).is_none()
                {
                    return Err(fail(format!("{}: {}", asset.path, node.hierarchy_path)));
                }
            }
        }
    }
    if source.state != lighting::RenderingState::Ready
        || !source.materials_ready
        || source.warming
        || source.road_lamps.recipe.is_none()
    {
        return Err(fail("Lighting assets are not ready".into()));
    }
    if materials.state != materials::RenderingState::Ready {
        return Err(fail("Material assets are not ready".into()));
    }
    let instruments = world.resource::<super::instruments::InstrumentVisuals>();
    if !instruments.bound || !instruments.prepared {
        return Err(fail("Instrument bindings are not ready".into()));
    }
    for light in &source.lights {
        if world.get::<lighting::SourceLight>(light.entity).is_none()
            || light
                .node
                .is_some_and(|node| world.get::<Transform>(node).is_none())
        {
            return Err(fail(format!("Light binding {}", light.id.0)));
        }
    }
    for (_, path, handle) in &source.textures {
        if !server.is_loaded_with_dependencies(handle.id())
            || !world.resource::<Assets<Image>>().contains(handle.id())
        {
            return Err(fail(path.clone()));
        }
    }
    for handle in materials.handles.values() {
        if !world
            .resource::<Assets<materials::SourceMaterial>>()
            .contains(handle.id())
        {
            return Err(fail("Source material handle".into()));
        }
    }
    let meshes = world.resource::<Assets<Mesh>>();
    let standard = world.resource::<Assets<StandardMaterial>>();
    let authored = world.resource::<Assets<materials::SourceMaterial>>();
    world
        .resource::<WorldVisuals>()
        .validate_reused_assets(meshes, authored)?;
    let road_path = &reference
        .assets
        .iter()
        .find(|asset| asset.id == reference.lighting.road_lamp.asset)
        .ok_or_else(|| fail("Road lamp asset binding".into()))?
        .path;
    source
        .road_lamps
        .validate_assets(road_path, meshes, authored, standard)?;
    for lamp in &source.lamp_materials {
        for choice in &lamp.choices {
            choice.validate_assets(authored, standard)?;
        }
    }
    instruments.validate_assets(authored, standard)?;
    for entity in world.iter_entities() {
        let receiver = entity.get::<MeshMaterial3d<materials::SourceMaterial>>();
        let sky = entity.get::<MeshMaterial3d<super::sky::SkyMaterial>>();
        if let Some(material) = entity.get::<MeshMaterial3d<StandardMaterial>>() {
            if !standard.contains(material.0.id()) {
                return Err(fail(format!(
                    "rendered standard material on {:?}",
                    entity.id()
                )));
            }
        }
        if receiver.is_some_and(|material| !authored.contains(material.0.id())) {
            return Err(fail(format!(
                "rendered source material on {:?}",
                entity.id()
            )));
        }
        if sky.is_some_and(|material| {
            !world
                .resource::<Assets<super::sky::SkyMaterial>>()
                .contains(material.0.id())
        }) {
            return Err(fail("sky material".into()));
        }
        // Generated terrain is rebuilt. Imported, caster, and sky meshes are reused.
        let rebuilt = entity.get::<materials::GeneratedWorldMaterial>()
            == Some(&materials::GeneratedWorldMaterial::Terrain);
        if !rebuilt && (receiver.is_some() || sky.is_some()) {
            if let Some(mesh) = entity.get::<Mesh3d>() {
                if !meshes.contains(mesh.0.id()) {
                    return Err(fail(format!("reused render mesh on {:?}", entity.id())));
                }
            }
        }
    }
    if !world
        .iter_entities()
        .any(|entity| entity.contains::<super::camera::DriveCamera>())
    {
        return Err(fail("Camera binding".into()));
    }
    Ok(())
}

impl PreparedPresentation {
    pub fn commit(mut self, world: &mut World) {
        self.commands.apply(world);
        for (entity, pose, visibility) in self.updates {
            world.entity_mut(entity).insert((pose, visibility));
        }
        let old_chunks = world
            .remove_resource::<WorldVisuals>()
            .expect("loaded chunks");
        world.insert_resource(self.chunks);
        old_chunks.release(world);
        let old_lighting = world
            .remove_resource::<lighting::SourceLighting>()
            .expect("loaded lighting");
        world.insert_resource(self.lighting);
        old_lighting.road_lamps.release(world);
    }
    pub fn discard(mut self, world: &mut World) {
        // Materialize reserved entities only within this boundary, then release all of them.
        self.commands.apply(world);
        self.chunks.release(world);
        self.lighting.road_lamps.release(world);
    }
}
