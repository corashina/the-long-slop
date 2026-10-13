//! Shared source pole recipes and absolute stream ownership.
use bevy::prelude::*;

use crate::presentation::lighting::{
    RenderingState, SourceLighting, register_road_light, remove_road_light,
};
use crate::presentation::materials::{SourceCaster, SourceMaterial, SourceShadowMode};
use crate::world::{RoadLampId, RoadLampPlacement};
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

struct SourcePole {
    id: crate::world::GeneratedId,
    root: Entity,
}
struct SourceProxy {
    id: crate::world::GeneratedId,
    node: Entity,
    light: Entity,
    slot: usize,
}
pub(crate) struct RoadLamps {
    source_poles: Vec<SourcePole>,
    source_proxies: Vec<SourceProxy>,
    required: Vec<crate::world::GeneratedId>,
    pub capacity: usize,
    pub fixed_lights: usize,
    pub fixed_shadows: usize,
    pub recipe: Option<RoadLampRecipe>,
    prototype: Option<Entity>,
    prototype_ready: bool,
    prototype_bound: bool,
    prototype_nodes: Vec<(crate::reference::NodeId, Entity)>,
    prototype_light: Option<Entity>,
    origin: [f64; 3],
}
impl RoadLamps {
    pub fn reset_instances(&mut self, commands: &mut Commands) {
        for pole in self.source_poles.drain(..) {
            commands.entity(pole.root).despawn();
        }
        for proxy in self.source_proxies.drain(..) {
            commands.entity(proxy.node).despawn();
            commands.entity(proxy.light).despawn();
        }
        self.required.clear();
        self.origin = [f64::INFINITY; 3];
    }
    pub fn new(_spacing: f64) -> Self {
        let capacity = crate::presentation::lighting::MAX_LIGHTS;
        Self {
            source_poles: Vec::with_capacity(512),
            source_proxies: Vec::with_capacity(crate::presentation::lighting::MAX_LIGHTS),
            required: Vec::with_capacity(crate::presentation::lighting::MAX_LIGHTS),
            capacity,
            fixed_lights: 0,
            fixed_shadows: 0,
            recipe: None,
            prototype: None,
            prototype_ready: false,
            prototype_bound: false,
            prototype_nodes: Vec::new(),
            prototype_light: None,
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

fn spawn_pole(
    commands: &mut Commands,
    recipe: &RoadLampRecipe,
    placement: RoadLampPlacement,
    road: &crate::reference::RoadLampReference,
    origin: crate::world::WorldOrigin,
) -> (Entity, Entity, Vec<(crate::reference::NodeId, Entity)>) {
    let root = commands
        .spawn((
            Transform::from_translation(Vec3::from_array(origin.local(placement.absolute_point)))
                .with_rotation(Quat::from_array(placement.rotation))
                .with_scale(Vec3::from_array(road.road_pose.scale)),
            Visibility::Inherited,
        ))
        .id();
    let mut nodes: Vec<(crate::reference::NodeId, Entity)> = Vec::with_capacity(recipe.nodes.len());
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
    (root, light_node, nodes)
}

/// The source frame owns pole geometry. Receiver influence owns independent light proxies.
pub(crate) fn sync_source_lamps(
    frame: &crate::world::WorldFrame<'_>,
    receivers: &[crate::world::WorldBounds],
    reference: &crate::reference::ReferenceManifest,
    enabled: bool,
    source: &mut SourceLighting,
    commands: &mut Commands,
    roots: &mut Query<&mut Transform>,
) -> Result<(), crate::reference::ReferenceError> {
    if source.road_lamps.recipe.is_none() || source.warming {
        return Ok(());
    }
    crate::world::required_world_lights(frame, receivers, &mut source.road_lamps.required);
    let required = source.road_lamps.required.len();
    let light_capacity = crate::presentation::lighting::MAX_LIGHTS - source.road_lamps.fixed_lights;
    let shadow_capacity =
        crate::presentation::lighting::SHADOW_SLOTS - source.road_lamps.fixed_shadows;
    let shadows = reference
        .lighting
        .lights
        .iter()
        .find(|light| Some(light.id) == reference.lighting.road_lamp.lights.first().copied())
        .is_some_and(|light| light.shadows.kind != 0);
    if required > light_capacity || (shadows && required > shadow_capacity) {
        return Err(crate::reference::ReferenceError(format!(
            "Required source lights exceed renderer capacity: {required}"
        )));
    }
    for index in (0..source.road_lamps.source_proxies.len()).rev() {
        if source
            .road_lamps
            .required
            .contains(&source.road_lamps.source_proxies[index].id)
        {
            continue;
        }
        let proxy = source.road_lamps.source_proxies.swap_remove(index);
        remove_road_light(source, commands, proxy.light);
        commands.entity(proxy.node).despawn();
    }
    for light in frame.lights {
        if !source.road_lamps.required.contains(&light.id) {
            continue;
        }
        let pose =
            Transform::from_translation(Vec3::from_array(frame.origin.local(light.pose.position)))
                .with_rotation(Quat::from_array(light.pose.rotation));
        if let Some(proxy) = source
            .road_lamps
            .source_proxies
            .iter()
            .find(|proxy| proxy.id == light.id)
        {
            if let Ok(mut transform) = roots.get_mut(proxy.node) {
                transform.set_if_neq(pose);
            }
        } else {
            let slot = (0..light_capacity)
                .find(|slot| {
                    !source
                        .road_lamps
                        .source_proxies
                        .iter()
                        .any(|proxy| proxy.slot == *slot)
                })
                .unwrap();
            let node = commands.spawn((pose, Visibility::Inherited)).id();
            let entity =
                register_road_light(source, commands, &reference.lighting, slot, node, enabled);
            source.road_lamps.source_proxies.push(SourceProxy {
                id: light.id.clone(),
                node,
                light: entity,
                slot,
            });
        }
    }
    for index in (0..source.road_lamps.source_poles.len()).rev() {
        let id = &source.road_lamps.source_poles[index].id;
        let geometry = frame.surfaces.iter().any(|surface| {
            surface.key.layer == crate::world::WorldLayer::Road && surface.key.unit == id.unit
        });
        let caster = source.road_lamps.required.contains(id);
        if geometry || caster {
            continue;
        }
        let pole = source.road_lamps.source_poles.swap_remove(index);
        commands.entity(pole.root).despawn();
    }
    for light in frame.lights {
        let geometry = frame.surfaces.iter().any(|surface| {
            surface.key.layer == crate::world::WorldLayer::Road && surface.key.unit == light.id.unit
        });
        // An influencing offscreen pole remains a possible shadow caster.
        if !geometry && !source.road_lamps.required.contains(&light.id) {
            continue;
        }
        let pose =
            Transform::from_translation(Vec3::from_array(frame.origin.local(light.pole.position)))
                .with_rotation(Quat::from_array(light.pole.rotation))
                .with_scale(Vec3::from_array(
                    reference.lighting.road_lamp.road_pose.scale,
                ));
        if let Some(pole) = source
            .road_lamps
            .source_poles
            .iter()
            .find(|pole| pole.id == light.id)
        {
            if let Ok(mut transform) = roots.get_mut(pole.root) {
                transform.set_if_neq(pose);
            }
        } else {
            let placement = RoadLampPlacement {
                id: RoadLampId(light.id.unit[0]),
                absolute_point: light.pole.position,
                rotation: light.pole.rotation,
            };
            let (root, _, _) = spawn_pole(
                commands,
                source.road_lamps.recipe.as_ref().unwrap(),
                placement,
                &reference.lighting.road_lamp,
                frame.origin,
            );
            source.road_lamps.source_poles.push(SourcePole {
                id: light.id.clone(),
                root,
            });
        }
    }
    Ok(())
}

pub(crate) fn source_lamp_admission(
    frame: &crate::world::WorldFrame<'_>,
    receivers: &[crate::world::WorldBounds],
    reference: &crate::reference::ReferenceManifest,
    source: &mut SourceLighting,
    pending_road: Option<[i64; 2]>,
) -> Result<(usize, usize, usize), crate::reference::ReferenceError> {
    let Some(recipe) = &source.road_lamps.recipe else {
        return Ok((0, 0, 0));
    };
    crate::world::required_world_lights(frame, receivers, &mut source.road_lamps.required);
    let required = source.road_lamps.required.len();
    let light_capacity = crate::presentation::lighting::MAX_LIGHTS - source.road_lamps.fixed_lights;
    let shadow_capacity =
        crate::presentation::lighting::SHADOW_SLOTS - source.road_lamps.fixed_shadows;
    let shadows = reference
        .lighting
        .lights
        .iter()
        .find(|light| Some(light.id) == reference.lighting.road_lamp.lights.first().copied())
        .is_some_and(|light| light.shadows.kind != 0);
    if required > light_capacity || (shadows && required > shadow_capacity) {
        return Err(crate::reference::ReferenceError(format!(
            "Required source lights exceed renderer capacity: {required}"
        )));
    }
    let selected = |id: &crate::world::GeneratedId| {
        pending_road == Some(id.unit)
            || source.road_lamps.required.contains(id)
            || frame.surfaces.iter().any(|surface| {
                surface.key.layer == crate::world::WorldLayer::Road && surface.key.unit == id.unit
            })
    };
    let poles = source.road_lamps.source_poles.len()
        + frame
            .lights
            .iter()
            .filter(|light| {
                selected(&light.id)
                    && !source
                        .road_lamps
                        .source_poles
                        .iter()
                        .any(|pole| pole.id == light.id)
            })
            .count();
    let proxies = source.road_lamps.source_proxies.len()
        + source
            .road_lamps
            .required
            .iter()
            .filter(|id| {
                !source
                    .road_lamps
                    .source_proxies
                    .iter()
                    .any(|p| p.id == **id)
            })
            .count();
    let meshes = recipe
        .nodes
        .iter()
        .map(|n| {
            n.meshes
                .iter()
                .map(|m| 1 + usize::from(m.caster.is_some()))
                .sum::<usize>()
        })
        .sum::<usize>();
    let objects = poles * (recipe.nodes.len() + meshes + 1) + proxies * 2;
    // ECS and GPU transform reservations include old/new overlap until the next frame.
    Ok((
        objects * 2048 + (poles + proxies) * 256,
        objects * 256,
        objects,
    ))
}
