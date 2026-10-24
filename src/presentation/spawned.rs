//! Scene instances of spawned item prototype bodies.
use super::materials::SourceMaterial;
use super::reference::{ReferenceVisuals, bind_node};
use crate::app::{Drive, RuntimeReference};
use crate::interaction::items::ItemLocation;
use crate::reference::{AssetUsage, ItemId, NodeId, ReferenceManifest, SourceId};
use crate::vehicle::{VehiclePose, from_rapier, to_rapier};
use bevy::ecs as bevy_ecs;
use bevy::ecs::system::SystemParam;
use bevy::gltf::GltfExtras;
use bevy::prelude::*;
use bevy::world_serialization::{WorldAssetRoot, WorldInstanceReady};
use std::collections::HashMap;

#[derive(Component)]
pub(crate) struct SpawnedScene;

struct SpawnedInstance {
    root: Entity,
    ready: bool,
    bound: bool,
    source_materials_bound: bool,
    nodes: Vec<(NodeId, Entity)>,
    primitives: Vec<Entity>,
    /// Scene root pose relative to the item body pose.
    body_to_scene: rapier3d::math::Pose,
}

#[derive(Resource, Default)]
pub struct SpawnedVisuals {
    instances: HashMap<ItemId, SpawnedInstance>,
    generation: u64,
}

impl SpawnedVisuals {
    pub(crate) fn reset(&mut self, world: &mut World) {
        self.generation = self.generation.wrapping_add(1);
        for (_, instance) in self.instances.drain() {
            world.despawn(instance.root);
        }
    }
    /// Bound scene node of a spawned item, for item-specific presentation.
    pub fn node(&self, item: ItemId, node: NodeId) -> Option<Entity> {
        let instance = self.instances.get(&item)?;
        instance
            .bound
            .then(|| instance.nodes.iter().find(|(id, _)| *id == node))
            .flatten()
            .map(|(_, entity)| *entity)
    }
}

#[derive(serde::Deserialize)]
struct Identity {
    reference_id: Option<SourceId>,
}

/// The body node that owns `node` in its prototype, if any.
fn owning_body(
    manifest: &ReferenceManifest,
    prototype: &crate::reference::ItemPrototype,
    mut node: NodeId,
) -> Option<NodeId> {
    for _ in 0..=manifest.nodes.len() {
        if prototype.bodies.iter().any(|b| b.node == node) {
            return Some(node);
        }
        node = manifest.nodes.iter().find(|n| n.id == node)?.parent?;
    }
    None
}

/// Scene graph queries that bind a prototype instance.
#[derive(SystemParam)]
pub(crate) struct SceneQueries<'w, 's> {
    extras: Query<'w, 's, (Entity, &'static GltfExtras)>,
    parents: Query<'w, 's, &'static ChildOf>,
    primitives: Query<'w, 's, (Entity, &'static ChildOf), With<Mesh3d>>,
    source_materials: Query<'w, 's, (), With<MeshMaterial3d<SourceMaterial>>>,
    transforms: Query<'w, 's, &'static mut Transform, With<SpawnedScene>>,
    visibility: Query<'w, 's, &'static mut Visibility>,
}

pub(crate) fn sync(
    mut commands: Commands,
    reference: Res<RuntimeReference>,
    drive: Res<Drive>,
    visuals: Res<ReferenceVisuals>,
    gltfs: Res<Assets<bevy::gltf::Gltf>>,
    mut spawned: ResMut<SpawnedVisuals>,
    scene: SceneQueries,
) {
    let SceneQueries {
        extras,
        parents,
        primitives,
        source_materials,
        mut transforms,
        mut visibility,
    } = scene;
    if !visuals.bound {
        return;
    }
    let manifest = &reference.0;
    let area = &drive.runtime.area;
    let stale: Vec<ItemId> = spawned
        .instances
        .keys()
        .copied()
        .filter(|id| !area.spawned.contains(*id))
        .collect();
    for id in stale {
        let instance = spawned.instances.remove(&id).unwrap();
        commands.entity(instance.root).despawn();
    }
    for item in area.spawned.iter() {
        let Some(state) = area.items.get(item.id) else {
            continue;
        };
        let prototype = &manifest.item_prototypes[item.prototype];
        if !spawned.instances.contains_key(&item.id) {
            let Some(asset) = manifest
                .assets
                .iter()
                .find(|a| a.id == prototype.asset && a.usage == AssetUsage::ItemPrototype)
            else {
                continue;
            };
            let Some((_, handle)) = visuals.gltfs.iter().find(|(id, _)| *id == asset.id) else {
                continue;
            };
            let Some(gltf) = gltfs.get(handle) else {
                continue;
            };
            let Ok(root_pose) = crate::interaction::items::node_pose(manifest, prototype.root)
            else {
                continue;
            };
            let body = prototype
                .bodies
                .iter()
                .find(|b| b.id == prototype.pickups[item.pickup].body)
                .unwrap();
            let body_local = to_rapier(VehiclePose {
                position: body.pose.translation,
                rotation: body.pose.rotation,
            });
            let id = item.id;
            let generation = spawned.generation;
            let root = commands
                .spawn((
                    SpawnedScene,
                    WorldAssetRoot(gltf.scenes[0].clone()),
                    Transform::default(),
                    Visibility::Hidden,
                ))
                .observe(
                    move |event: On<WorldInstanceReady>, mut spawned: ResMut<SpawnedVisuals>| {
                        if spawned.generation != generation {
                            return;
                        }
                        if let Some(instance) = spawned.instances.get_mut(&id)
                            && instance.root == event.entity
                        {
                            instance.ready = true;
                        }
                    },
                )
                .id();
            spawned.instances.insert(
                item.id,
                SpawnedInstance {
                    root,
                    ready: false,
                    bound: false,
                    source_materials_bound: false,
                    nodes: Vec::new(),
                    primitives: Vec::new(),
                    body_to_scene: body_local.inverse() * to_rapier(root_pose).inverse(),
                },
            );
            continue;
        }
        let instance = spawned.instances.get_mut(&item.id).unwrap();
        if !instance.ready {
            continue;
        }
        if !instance.bound {
            for (entity, extra) in &extras {
                let mut parent = entity;
                let mut belongs = false;
                while let Ok(link) = parents.get(parent) {
                    parent = link.parent();
                    if parent == instance.root {
                        belongs = true;
                        break;
                    }
                }
                if !belongs {
                    continue;
                }
                let Ok(identity) = serde_json::from_str::<Identity>(&extra.value) else {
                    continue;
                };
                let Some(node) = identity.reference_id.and_then(|source| {
                    manifest
                        .nodes
                        .iter()
                        .find(|node| node.source == source && node.asset == prototype.asset)
                }) else {
                    continue;
                };
                if !instance.nodes.iter().any(|(id, _)| *id == node.id)
                    && bind_node(&mut commands, node, entity, extra, &primitives).is_ok()
                {
                    instance.nodes.push((node.id, entity));
                }
            }
            let count = manifest
                .nodes
                .iter()
                .filter(|node| node.asset == prototype.asset)
                .count();
            if instance.nodes.len() != count {
                continue;
            }
            // Other pickup bodies of the prototype are separate items.
            let own = prototype
                .bodies
                .iter()
                .find(|b| b.id == prototype.pickups[item.pickup].body)
                .unwrap()
                .node;
            for &(node, entity) in &instance.nodes {
                if owning_body(manifest, prototype, node).is_some_and(|body| body != own) {
                    commands.entity(entity).insert(Visibility::Hidden);
                }
            }
            for (primitive, parent) in &primitives {
                let mut ancestor = parent.parent();
                while ancestor != instance.root {
                    let Ok(link) = parents.get(ancestor) else {
                        break;
                    };
                    ancestor = link.parent();
                }
                if ancestor == instance.root {
                    instance.primitives.push(primitive);
                }
            }
            instance.bound = true;
        }
        if !instance.source_materials_bound {
            if instance.primitives.is_empty()
                || !instance
                    .primitives
                    .iter()
                    .all(|primitive| source_materials.contains(*primitive))
            {
                continue;
            }
            instance.source_materials_bound = true;
        }
        let pose = area
            .item_render_poses
            .iter()
            .find(|(id, _, _)| *id == item.id)
            .map_or(state.pose, |(_, previous, current)| {
                super::interpolate(
                    *previous,
                    *current,
                    drive
                        .clock
                        .alpha(drive.runtime.clock_factors(manifest).time_scale),
                )
            });
        if let Ok(mut transform) = transforms.get_mut(instance.root) {
            transform.set_if_neq(super::transform(from_rapier(
                &(to_rapier(pose) * instance.body_to_scene),
            )));
        }
        let shown = !matches!(state.location, ItemLocation::Stored(_));
        if let Ok(mut v) = visibility.get_mut(instance.root) {
            v.set_if_neq(if shown {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            });
        }
    }
}
