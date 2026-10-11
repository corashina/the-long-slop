//! Shared source pole recipes and absolute stream ownership.
use bevy::prelude::*;

use crate::presentation::lighting::{
    RenderingState, SourceLighting, register_road_light, remove_road_light,
};
use crate::presentation::materials::{SourceCaster, SourceMaterial, SourceShadowMode};
use crate::world::{
    CHUNK_LENGTH, ChunkKey, RoadLampId, RoadLampPlacement, active_chunks, road_lamp_placement,
    road_lamp_range,
};
use bevy::camera::visibility::{NoFrustumCulling, RenderLayers};
use bevy::gltf::GltfExtras;
use bevy::world_serialization::{WorldAssetRoot, WorldInstanceReady};

struct RoadMeshRecipe {
    mesh: Handle<Mesh>,
    material: Handle<SourceMaterial>,
    pose: Transform,
    visibility: Visibility,
    layers: RenderLayers,
    shadow_mode: u8,
    receive_shadows: bool,
    caster: Option<Handle<StandardMaterial>>,
}
struct RoadNodeRecipe {
    id: crate::reference::NodeId,
    parent: Option<usize>,
    pose: Transform,
    visibility: Visibility,
    meshes: Vec<RoadMeshRecipe>,
}
pub(crate) struct RoadLampRecipe {
    nodes: Vec<RoadNodeRecipe>,
    light_node: usize,
}
struct RoadLampInstance {
    placement: RoadLampPlacement,
    root: Entity,
    light: Entity,
    slot: usize,
    // Repeated source identities belong only to this instance.
    _nodes: Vec<(crate::reference::NodeId, Entity)>,
}
pub(crate) struct RoadLamps {
    pub capacity: usize,
    pub fixed_lights: usize,
    pub fixed_shadows: usize,
    pub recipe: Option<RoadLampRecipe>,
    prototype: Option<Entity>,
    prototype_ready: bool,
    prototype_bound: bool,
    prototype_nodes: Vec<(crate::reference::NodeId, Entity)>,
    prototype_light: Option<Entity>,
    instances: Vec<RoadLampInstance>,
    keys: Option<[ChunkKey; 11]>,
    lateral_band: i64,
    origin: [f64; 3],
}
impl RoadLamps {
    pub fn reset_instances(&mut self, commands: &mut Commands) {
        for instance in self.instances.drain(..) {
            commands.entity(instance.root).despawn();
        }
        self.keys = None;
        self.origin = [f64::INFINITY; 3];
    }
    pub fn new(spacing: f64) -> Self {
        // Eleven consecutive source intervals span 1408 metres: ceil(1408 / 100) = 15.
        let capacity = (active_chunks(0.).len() as f64 * CHUNK_LENGTH / spacing).ceil() as usize;
        Self {
            capacity,
            fixed_lights: 0,
            fixed_shadows: 0,
            recipe: None,
            prototype: None,
            prototype_ready: false,
            prototype_bound: false,
            prototype_nodes: Vec::new(),
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
    extras: Query<(Entity, &GltfExtras)>,
    primitives: Query<(Entity, &ChildOf), With<Mesh3d>>,
    receivers: Query<
        (
            &Mesh3d,
            &MeshMaterial3d<SourceMaterial>,
            &Transform,
            &Visibility,
            &RenderLayers,
            &SourceShadowMode,
            Option<&bevy::light::NotShadowReceiver>,
            Option<&Children>,
        ),
        Without<SourceCaster>,
    >,
    casters: Query<&MeshMaterial3d<StandardMaterial>, With<SourceCaster>>,
) {
    if !source.bound
        || source.road_lamps.recipe.is_some()
        || matches!(source.state, RenderingState::Failed(_))
    {
        return;
    }
    let manifest = &reference.0;
    let road = &manifest.lighting.road_lamp;
    if source.road_lamps.prototype.is_none() {
        let asset = manifest.assets.iter().find(|a| a.id == road.asset).unwrap();
        let handle = &visuals
            .handles
            .iter()
            .find(|(path, _)| *path == asset.path)
            .unwrap()
            .1;
        let Some(gltf) = gltfs.get(handle) else {
            return;
        };
        let root = commands
            .spawn((
                WorldAssetRoot(gltf.scenes[0].clone()),
                Transform::from_translation(Vec3::splat(1_000_000.)),
                Visibility::Hidden,
            ))
            .observe(
                |_: On<WorldInstanceReady>, mut source: ResMut<SourceLighting>| {
                    source.road_lamps.prototype_ready = true;
                },
            )
            .id();
        source.road_lamps.prototype = Some(root);
        return;
    }
    if !source.road_lamps.prototype_ready {
        return;
    }
    if !source.road_lamps.prototype_bound {
        #[derive(serde::Deserialize)]
        struct Identity {
            reference_id: Option<crate::reference::SourceId>,
        }
        for (entity, extra) in &extras {
            let identity: Identity = match serde_json::from_str(&extra.value) {
                Ok(value) => value,
                Err(error) => {
                    source.state = RenderingState::Failed(format!("Road lamp extras: {error}"));
                    return;
                }
            };
            let Some(node) = identity.reference_id.and_then(|id| {
                manifest
                    .nodes
                    .iter()
                    .find(|n| n.source == id && n.asset == road.asset)
            }) else {
                continue;
            };
            if let Err(error) = crate::presentation::reference::bind_node(
                &mut commands,
                node,
                entity,
                extra,
                &primitives,
            ) {
                source.state = RenderingState::Failed(error);
                return;
            }
            if source
                .road_lamps
                .prototype_nodes
                .iter()
                .any(|(id, _)| *id == node.id)
            {
                source.state =
                    RenderingState::Failed(format!("Duplicate road prototype node {}", node.id.0));
                return;
            }
            source.road_lamps.prototype_nodes.push((node.id, entity));
        }
        if source.road_lamps.prototype_nodes.len()
            != manifest
                .nodes
                .iter()
                .filter(|n| n.asset == road.asset)
                .count()
        {
            source.state =
                RenderingState::Failed("Road lamp prototype has missing source nodes".into());
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
            .prototype_nodes
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
        source.road_lamps.prototype_bound = true;
        return;
    }
    for (entity, parent) in &primitives {
        if !source
            .road_lamps
            .prototype_nodes
            .iter()
            .any(|(_, node)| *node == parent.parent())
        {
            continue;
        }
        let Ok((_, _, _, _, _, mode, _, children)) = receivers.get(entity) else {
            return;
        };
        if mode.0 != 0
            && !children
                .is_some_and(|children| children.iter().any(|child| casters.contains(child)))
        {
            return;
        }
    }
    let far = Transform::from_translation(Vec3::splat(1_000_000.));
    let mut nodes: Vec<RoadNodeRecipe> =
        Vec::with_capacity(source.road_lamps.prototype_nodes.len());
    for node in manifest
        .nodes
        .iter()
        .filter(|node| node.asset == road.asset)
    {
        let entity = source
            .road_lamps
            .prototype_nodes
            .iter()
            .find(|(id, _)| *id == node.id)
            .unwrap()
            .1;
        let parent = match node.parent {
            Some(parent) => match nodes.iter().position(|n| n.id == parent) {
                Some(index) => Some(index),
                None => {
                    source.state = RenderingState::Failed(
                        "Road lamp source hierarchy is not parent ordered".into(),
                    );
                    return;
                }
            },
            None => None,
        };
        let mut meshes = Vec::new();
        for (primitive, owner) in &primitives {
            if owner.parent() != entity {
                continue;
            }
            let (mesh, material, pose, visibility, layers, mode, no_receive, children) =
                receivers.get(primitive).unwrap();
            let caster = children.and_then(|children| {
                children
                    .iter()
                    .find_map(|child| casters.get(child).ok().map(|m| m.0.clone()))
            });
            let warm = commands
                .spawn((
                    mesh.clone(),
                    material.clone(),
                    far,
                    Visibility::Visible,
                    bevy::light::NotShadowCaster,
                    NoFrustumCulling,
                ))
                .id();
            source.warmups.push((warm, false));
            if let Some(caster) = &caster {
                let warm = commands
                    .spawn((
                        SourceCaster,
                        mesh.clone(),
                        MeshMaterial3d(caster.clone()),
                        far,
                        Visibility::Visible,
                        (32..64).collect::<RenderLayers>(),
                        NoFrustumCulling,
                    ))
                    .id();
                source.warmups.push((warm, true));
            }
            meshes.push(RoadMeshRecipe {
                mesh: mesh.0.clone(),
                material: material.0.clone(),
                pose: *pose,
                visibility: *visibility,
                layers: layers.clone(),
                shadow_mode: mode.0,
                receive_shadows: no_receive.is_none(),
                caster,
            });
        }
        nodes.push(RoadNodeRecipe {
            id: node.id,
            parent,
            pose: crate::presentation::transform(crate::vehicle::VehiclePose {
                position: node.transform.translation,
                rotation: node.transform.rotation,
            })
            .with_scale(Vec3::from_array(node.transform.scale)),
            visibility: if node.active {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            },
            meshes,
        });
    }
    let light = manifest
        .lighting
        .lights
        .iter()
        .find(|l| l.id == road.lights[0])
        .unwrap();
    let light_node = nodes
        .iter()
        .position(|node| Some(node.id) == light.node)
        .unwrap();
    info!(
        "Source road lamp cache: {} nodes, {} meshes, {} reserved poles",
        nodes.len(),
        nodes.iter().map(|n| n.meshes.len()).sum::<usize>(),
        source.road_lamps.capacity
    );
    source.road_lamps.recipe = Some(RoadLampRecipe { nodes, light_node });
}

pub(crate) fn finish_road_lamp_warmup(commands: &mut Commands, source: &mut SourceLighting) {
    if let Some(light) = source.road_lamps.prototype_light.take() {
        remove_road_light(source, commands, light);
    }
    if let Some(root) = source.road_lamps.prototype.take() {
        commands.entity(root).despawn();
    }
    source.road_lamps.prototype_nodes.clear();
}

/// Change ownership only when chunk keys or the lateral band change. Rebases change existing poses.
pub(crate) fn sync_road_lamps(
    mut commands: Commands,
    reference: Res<crate::app::RuntimeReference>,
    drive: Res<crate::app::Drive>,
    mut source: ResMut<SourceLighting>,
    mut roots: Query<&mut Transform>,
) {
    if !source.bound
        || source.warming
        || source.road_lamps.recipe.is_none()
        || matches!(source.state, RenderingState::Failed(_))
    {
        return;
    }
    let stream = &drive.runtime.stream;
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
    let road = &reference.0.lighting.road_lamp;
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
            remove_road_light(&mut source, &mut commands, instance.light);
            commands.entity(instance.root).despawn();
        }
        let output = drive
            .runtime
            .lighting
            .road_lamp_output(&reference.0.lighting);
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
            let mut nodes: Vec<(crate::reference::NodeId, Entity)> =
                Vec::with_capacity(recipe.nodes.len());
            for node in &recipe.nodes {
                let parent = node.parent.map_or(root, |index| nodes[index].1);
                let entity = commands
                    .spawn((node.pose, node.visibility, ChildOf(parent)))
                    .id();
                nodes.push((node.id, entity));
                for mesh in &node.meshes {
                    let mut primitive = commands.spawn((
                        Mesh3d(mesh.mesh.clone()),
                        MeshMaterial3d(mesh.material.clone()),
                        mesh.pose,
                        mesh.visibility,
                        mesh.layers.clone(),
                        SourceShadowMode(mesh.shadow_mode),
                        bevy::light::NotShadowCaster,
                        ChildOf(entity),
                    ));
                    if !mesh.receive_shadows {
                        primitive.insert(bevy::light::NotShadowReceiver);
                    }
                    let primitive = primitive.id();
                    if let Some(caster) = &mesh.caster {
                        let layers = mesh
                            .layers
                            .iter()
                            .map(|layer| layer + 32)
                            .collect::<RenderLayers>();
                        commands.spawn((
                            SourceCaster,
                            Mesh3d(mesh.mesh.clone()),
                            MeshMaterial3d(caster.clone()),
                            Transform::default(),
                            Visibility::Inherited,
                            layers,
                            ChildOf(primitive),
                        ));
                    }
                }
            }
            let light_node = nodes[recipe.light_node].1;
            let light = register_road_light(
                &mut source,
                &mut commands,
                &reference.0.lighting,
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
            if let Ok(mut pose) = roots.get_mut(instance.root) {
                pose.translation =
                    Vec3::from_array(stream.origin.local(instance.placement.absolute_point));
            }
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
