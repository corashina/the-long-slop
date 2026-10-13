use super::materials::{SourceCaster, SourceMaterial};
use super::reference::{ReferenceVisuals, bind_node};
use crate::app::{Drive, RuntimeReference};
use crate::reference::{AssetUsage, ItemId, NodeId, SourceId};
use crate::vehicle::{from_rapier, to_rapier};
use bevy::ecs as bevy_ecs;
use bevy::gltf::GltfExtras;
use bevy::prelude::*;
use bevy::world_serialization::{WorldAssetRoot, WorldInstanceReady};
use std::collections::HashMap;

#[derive(Component)]
pub(crate) struct ConsumableScene;

struct FoodInstance {
    root: Entity,
    ready: bool,
    bound: bool,
    source_materials_bound: bool,
    nodes: Vec<(NodeId, Entity)>,
    primitives: Vec<Entity>,
    source_root: rapier3d::math::Pose,
}

#[derive(Resource, Default)]
pub struct ConsumableVisuals {
    instances: HashMap<ItemId, FoodInstance>,
    generation: u64,
}

impl ConsumableVisuals {
    pub(crate) fn reset(&mut self, world: &mut World) {
        self.generation = self.generation.wrapping_add(1);
        for (_, instance) in self.instances.drain() {
            world.despawn(instance.root);
        }
    }
}

#[derive(serde::Deserialize)]
struct Identity {
    reference_id: Option<SourceId>,
}

pub(crate) fn sync(
    mut commands: Commands,
    reference: Res<RuntimeReference>,
    drive: Res<Drive>,
    visuals: Res<ReferenceVisuals>,
    gltfs: Res<Assets<bevy::gltf::Gltf>>,
    mut food: ResMut<ConsumableVisuals>,
    extras: Query<(Entity, &GltfExtras)>,
    parents: Query<&ChildOf>,
    primitives: Query<(Entity, &ChildOf), With<Mesh3d>>,
    source_casters: Query<(), With<SourceCaster>>,
    source_materials: Query<(), With<MeshMaterial3d<SourceMaterial>>>,
    mut transforms: Query<&mut Transform, With<ConsumableScene>>,
    mut visibility: Query<&mut Visibility>,
) {
    if !visuals.bound {
        return;
    }
    let manifest = &reference.0;
    let starter = manifest.survival.starter_food;
    if let Some(item) = drive.runtime.area.items.get(starter) {
        let prototype = &manifest.survival.foods[item.food.unwrap().prototype_index];
        if let Some(&root) = visuals.nodes.get(
            &manifest
                .items
                .iter()
                .find(|record| record.id == starter)
                .unwrap()
                .node,
        ) {
            set_visibility(&mut visibility, root, !item.removed);
        }
        for (index, group) in manifest.survival.starter_parts.iter().enumerate() {
            for node in group {
                if let Some(&entity) = visuals.nodes.get(node) {
                    set_visibility(
                        &mut visibility,
                        entity,
                        !item.removed
                            && index == item.food.unwrap().bite as usize
                            && index < prototype.bites as usize,
                    );
                }
            }
        }
    }
    for item in drive
        .runtime
        .area
        .items
        .iter()
        .filter(|item| item.food.is_some() && item.id != starter)
    {
        if item.removed {
            if let Some(instance) = food.instances.remove(&item.id) {
                commands.entity(instance.root).despawn();
            }
            continue;
        }
        let state = item.food.unwrap();
        let prototype = &manifest.survival.foods[state.prototype_index];
        if !food.instances.contains_key(&item.id) {
            let asset = manifest
                .assets
                .iter()
                .find(|asset| {
                    asset.id == prototype.asset && asset.usage == AssetUsage::FoodPrototype
                })
                .unwrap();
            let handle = &visuals
                .gltfs
                .iter()
                .find(|(id, _)| *id == asset.id)
                .unwrap()
                .1;
            let Some(gltf) = gltfs.get(handle) else {
                continue;
            };
            let id = item.id;
            let generation = food.generation;
            let root = commands
                .spawn((
                    ConsumableScene,
                    WorldAssetRoot(gltf.scenes[0].clone()),
                    Transform::default(),
                    Visibility::Hidden,
                ))
                .observe(
                    move |event: On<WorldInstanceReady>, mut food: ResMut<ConsumableVisuals>| {
                        if food.generation != generation {
                            return;
                        }
                        if let Some(instance) = food.instances.get_mut(&id)
                            && instance.root == event.entity
                        {
                            instance.ready = true;
                        }
                    },
                )
                .id();
            food.instances.insert(
                item.id,
                FoodInstance {
                    root,
                    ready: false,
                    bound: false,
                    source_materials_bound: false,
                    nodes: Vec::new(),
                    primitives: Vec::new(),
                    source_root: to_rapier(
                        crate::interaction::items::node_pose(manifest, prototype.root).unwrap(),
                    ),
                },
            );
            continue;
        }
        let instance = food.instances.get_mut(&item.id).unwrap();
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
            for (primitive, parent) in &primitives {
                if source_casters.contains(primitive) {
                    continue;
                }
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
        let pose = drive
            .runtime
            .area
            .item_render_poses
            .iter()
            .find(|(id, _, _)| *id == item.id)
            .map_or(item.pose, |(_, previous, current)| {
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
                &(to_rapier(pose) * instance.source_root.inverse()),
            )));
        }
        set_visibility(&mut visibility, instance.root, true);
        for other in &manifest.survival.foods {
            if let Some(&(_, entity)) = instance.nodes.iter().find(|(node, _)| *node == other.root)
            {
                set_visibility(&mut visibility, entity, other.root == prototype.root);
            }
        }
        for (index, group) in prototype.parts.iter().enumerate() {
            for node in group {
                if let Some(&(_, entity)) = instance.nodes.iter().find(|(id, _)| id == node) {
                    set_visibility(&mut visibility, entity, index == state.bite as usize);
                }
            }
        }
    }
}

fn set_visibility(query: &mut Query<&mut Visibility>, entity: Entity, show: bool) {
    if let Ok(mut visibility) = query.get_mut(entity) {
        visibility.set_if_neq(if show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
}
