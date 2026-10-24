use crate::reference::{
    AssetId, AssetUsage, BodyId, ItemId, ItemKind, NodeId, OpeningId, ReferenceManifest, SourceId,
};
use crate::starting_area::StartingArea;
use crate::vehicle::PhysicsWorld;
use bevy::camera::visibility::RenderLayers;
use bevy::ecs as bevy_ecs;
use bevy::gltf::{GltfExtras, GltfLoaderSettings, convert_coordinates::GltfConvertCoordinates};
use bevy::prelude::*;
use bevy::world_serialization::{WorldAssetRoot, WorldInstanceReady};
use std::collections::HashMap;

#[derive(Component)]
pub struct ImportedRoot;
#[derive(Component)]
pub struct BodyVisual {
    pub body: BodyId,
    pub scale: Vec3,
}

#[derive(Component)]
pub struct ItemVisual {
    pub item: ItemId,
    pub scale: Vec3,
}

#[derive(Component)]
pub struct InventoryVisibility {
    item: ItemId,
    keep: bool,
    stored: bool,
}
#[derive(Component)]
pub struct ValveVisibility {
    pub(crate) opening: OpeningId,
    pub(crate) open: bool,
}
#[derive(Component)]
pub struct CapVisual {
    pub(crate) opening: OpeningId,
    pub(crate) closed: Transform,
    pub(crate) opened: Transform,
    pub(crate) euler: bool,
    pub(crate) closed_euler: Vec3,
    pub(crate) open_euler: Vec3,
    /// Source `capLerp` while `DoMove` runs.
    pub(crate) motion: Option<f32>,
}

#[derive(Resource, Default)]
pub struct ReferenceVisuals {
    pub handles: Vec<(String, Handle<bevy::gltf::Gltf>)>,
    pub gltfs: Vec<(AssetId, Handle<bevy::gltf::Gltf>)>,
    pub nodes: HashMap<NodeId, Entity>,
    pub spawned: bool,
    pub instances: usize,
    pub expected_instances: usize,
    pub bound: bool,
    pub failed: bool,
}

pub(crate) fn source_gltf_settings(settings: &mut GltfLoaderSettings) {
    settings.load_cameras = false;
    settings.load_lights = false;
    settings.load_animations = false;
    settings.convert_coordinates = Some(GltfConvertCoordinates {
        rotate_scene_entity: false,
        rotate_meshes: false,
    });
}

pub fn load(commands: &mut Commands, _server: &AssetServer, reference: &ReferenceManifest) {
    let count = reference
        .assets
        .iter()
        .filter(|a| {
            matches!(
                a.usage,
                AssetUsage::UniqueScene | AssetUsage::RoadLampPrototype | AssetUsage::SkyPrototype
            )
        })
        .count();
    commands.insert_resource(ReferenceVisuals {
        expected_instances: reference.expected_instances(),
        handles: Vec::with_capacity(count),
        gltfs: Vec::with_capacity(count),
        ..Default::default()
    });
}

pub fn spawn_scenes(
    commands: &mut Commands,
    reference: &ReferenceManifest,
    visuals: &mut ReferenceVisuals,
    gltfs: &Assets<bevy::gltf::Gltf>,
) {
    for (asset, (_, handle)) in reference
        .assets
        .iter()
        .filter(|asset| {
            matches!(
                asset.usage,
                AssetUsage::UniqueScene | AssetUsage::RoadLampPrototype | AssetUsage::SkyPrototype
            )
        })
        .zip(&visuals.handles)
    {
        if asset.usage != AssetUsage::UniqueScene {
            continue;
        }
        commands
            .spawn((
                ImportedRoot,
                WorldAssetRoot(gltfs.get(handle).unwrap().scenes[0].clone()),
                Transform::default(),
                Visibility::Hidden,
            ))
            .observe(instance_ready);
    }
    visuals.spawned = true;
}

fn instance_ready(_: On<WorldInstanceReady>, mut visuals: ResMut<ReferenceVisuals>) {
    visuals.instances += 1;
}

#[derive(serde::Deserialize)]
struct NodeExtras {
    active: bool,
    renderer_enabled: Option<bool>,
    layer: Option<u8>,
    cast_shadows: Option<u8>,
    receive_shadows: Option<bool>,
}
#[derive(serde::Deserialize)]
struct NodeReference {
    reference_id: Option<SourceId>,
}

/// Apply checked source renderer flags without global node ownership.
pub(crate) fn bind_node(
    commands: &mut Commands,
    node: &crate::reference::NodeRecord,
    entity: Entity,
    extra: &GltfExtras,
    primitives: &Query<(Entity, &ChildOf), With<Mesh3d>>,
) -> Result<(), String> {
    let data: NodeExtras = serde_json::from_str(&extra.value)
        .map_err(|error| format!("Invalid glTF node {} extras: {error}", node.id.0))?;
    if !data.active {
        commands.entity(entity).insert(Visibility::Hidden);
    }
    for (primitive, parent) in primitives {
        if parent.parent() != entity {
            continue;
        }
        let enabled = data
            .renderer_enabled
            .ok_or_else(|| format!("Missing renderer_enabled for glTF node {}", node.id.0))?;
        let layer = data
            .layer
            .filter(|layer| *layer < 32)
            .ok_or_else(|| format!("Missing source layer for glTF node {}", node.id.0))?;
        let cast = data
            .cast_shadows
            .filter(|mode| *mode <= 3)
            .ok_or_else(|| format!("Missing shadow mode for glTF node {}", node.id.0))?;
        commands.entity(primitive).insert((
            RenderLayers::layer(layer as usize),
            super::materials::SourceShadowMode(cast),
        ));
        if data.receive_shadows == Some(false) {
            commands
                .entity(primitive)
                .insert(bevy::light::NotShadowReceiver);
        }
        if !enabled {
            commands.entity(primitive).insert(Visibility::Hidden);
        }
    }
    Ok(())
}

pub fn bind(
    commands: &mut Commands,
    reference: &ReferenceManifest,
    visuals: &mut ReferenceVisuals,
    extras: &Query<(Entity, &GltfExtras)>,
    primitives: &Query<(Entity, &ChildOf), With<Mesh3d>>,
) -> Result<bool, String> {
    if visuals.bound {
        for (node, entity) in &visuals.nodes {
            if !extras.contains(*entity) {
                return Err(format!(
                    "Required source node {} lost its imported entity",
                    node.0
                ));
            }
        }
        return Ok(true);
    }
    if visuals.instances != visuals.expected_instances {
        return Ok(false);
    }
    for (entity, extra) in extras {
        let identity: NodeReference = serde_json::from_str(&extra.value)
            .map_err(|error| format!("Invalid glTF extras: {error}"))?;
        let Some(source) = identity.reference_id else {
            continue;
        };
        let Some(node) = reference.nodes.iter().find(|node| node.source == source) else {
            return Err(format!(
                "Unknown glTF node {}:{}",
                source.file, source.path_id
            ));
        };
        if !reference
            .assets
            .iter()
            .any(|a| a.id == node.asset && a.usage == AssetUsage::UniqueScene)
        {
            continue;
        }
        bind_node(commands, node, entity, extra, primitives)?;
        if visuals.nodes.insert(node.id, entity).is_some() {
            return Err(format!("Duplicate glTF node {}", node.id.0));
        }
    }
    for node in reference.nodes.iter().filter(|n| {
        reference
            .assets
            .iter()
            .any(|a| a.id == n.asset && a.usage == AssetUsage::UniqueScene)
    }) {
        if !visuals.nodes.contains_key(&node.id) {
            return Err(format!(
                "Missing glTF node {} ({})",
                node.id.0, node.hierarchy_path
            ));
        }
    }
    // Each physical group becomes a render root. Its descendants retain their source transforms.
    for body in &reference.bodies {
        super::mechanisms::bind_body(commands, reference, visuals, body);
    }
    for item in &reference.items {
        if !matches!(item.kind, ItemKind::Wheel | ItemKind::Rim | ItemKind::Tire)
            && !reference.parts.iter().any(|part| part.item == item.id)
            && !reference
                .plates
                .plates
                .iter()
                .any(|plate| plate.item == item.id)
            && !reference
                .flashlights
                .instances
                .iter()
                .any(|flashlight| flashlight.item == item.id)
        {
            continue;
        }
        let mut scale = Vec3::ONE;
        let mut current = Some(item.node);
        while let Some(id) = current {
            let node = reference
                .nodes
                .iter()
                .find(|node| node.id == id)
                .expect("validated node");
            scale *= Vec3::from_array(node.transform.scale);
            current = node.parent;
        }
        commands
            .entity(visuals.nodes[&item.node])
            .remove::<ChildOf>()
            .remove::<BodyVisual>()
            .insert(ItemVisual {
                item: item.id,
                scale,
            });
    }
    for item in &reference.items {
        let Some(rule) = item.pickup.as_ref().filter(|p| p.can_inventory) else {
            continue;
        };
        for (source, keep) in [
            (rule.disable_when_stored.as_ref(), false),
            (rule.keep_when_stored.as_ref(), true),
        ] {
            if let Some(source) = source {
                let node = reference
                    .nodes
                    .iter()
                    .find(|n| &n.source == source)
                    .expect("checked storage node");
                commands
                    .entity(visuals.nodes[&node.id])
                    .insert(InventoryVisibility {
                        item: item.id,
                        keep,
                        stored: false,
                    });
            }
        }
    }
    for opening in &reference.openings {
        let links = &opening.visuals;
        for &id in &links.open_nodes {
            commands.entity(visuals.nodes[&id]).insert(ValveVisibility {
                opening: opening.id,
                open: true,
            });
        }
        for &id in &links.closed_nodes {
            commands.entity(visuals.nodes[&id]).insert(ValveVisibility {
                opening: opening.id,
                open: false,
            });
        }
        if let Some(id) = links.cap_node {
            let node = reference
                .nodes
                .iter()
                .find(|node| node.id == id)
                .expect("cap node");
            let convert = |source: &crate::reference::Transform| {
                Transform::from_translation(Vec3::from_array(source.translation))
                    .with_rotation(Quat::from_array(source.rotation))
                    .with_scale(Vec3::from_array(node.transform.scale))
            };
            let closed = links
                .closed_node
                .map(|id| {
                    convert(
                        &reference
                            .nodes
                            .iter()
                            .find(|node| node.id == id)
                            .expect("closed marker")
                            .transform,
                    )
                })
                .unwrap_or_else(|| convert(&node.transform));
            let opened = links
                .open_node
                .map(|id| {
                    convert(
                        &reference
                            .nodes
                            .iter()
                            .find(|node| node.id == id)
                            .expect("open marker")
                            .transform,
                    )
                })
                .unwrap_or(closed);
            commands.entity(visuals.nodes[&id]).insert(CapVisual {
                opening: opening.id,
                closed,
                opened,
                euler: links.euler_rotation,
                closed_euler: Vec3::from_array(links.closed_euler_degrees),
                open_euler: Vec3::from_array(links.open_euler_degrees),
                motion: None,
            });
        }
    }
    visuals.bound = true;
    Ok(true)
}

pub(crate) fn bind_generated_cap(
    commands: &mut Commands,
    cap: &crate::world::GeneratedCapVisual,
    prototype: &crate::reference::WorldPrototypeReference,
    nodes: &[Entity],
) {
    let links = &cap.visual;
    let entity = |id: NodeId| nodes[prototype.nodes[id.0 as usize].gltf_node as usize];
    for &id in &links.open_nodes {
        commands.entity(entity(id)).insert(ValveVisibility {
            opening: cap.opening,
            open: true,
        });
    }
    for &id in &links.closed_nodes {
        commands.entity(entity(id)).insert(ValveVisibility {
            opening: cap.opening,
            open: false,
        });
    }
    if let Some(id) = links.cap_node {
        let scale = Vec3::from_array(prototype.nodes[id.0 as usize].transform.scale);
        let pose = |id: NodeId| {
            let t = prototype.nodes[id.0 as usize].transform;
            Transform::from_translation(Vec3::from_array(t.translation))
                .with_rotation(Quat::from_array(t.rotation))
                .with_scale(scale)
        };
        let closed = pose(links.closed_node.unwrap_or(id));
        commands.entity(entity(id)).insert(CapVisual {
            opening: cap.opening,
            closed,
            opened: links.open_node.map(pose).unwrap_or(closed),
            euler: links.euler_rotation,
            closed_euler: Vec3::from_array(links.closed_euler_degrees),
            open_euler: Vec3::from_array(links.open_euler_degrees),
            motion: None,
        });
    }
}

pub fn sync_caps(
    area: &StartingArea,
    dt: f32,
    caps: &mut Query<
        (&mut CapVisual, &mut Transform),
        (
            Without<BodyVisual>,
            Without<ItemVisual>,
            Without<super::save::HiddenWorld>,
        ),
    >,
    visibility: &mut Query<
        (&ValveVisibility, &mut Visibility),
        (
            Without<BodyVisual>,
            Without<ItemVisual>,
            Without<super::save::HiddenWorld>,
        ),
    >,
) {
    for (mut cap, mut transform) in caps {
        let valve = area.interaction.opening_valve(cap.opening);
        let mut pose = cap.closed;
        if cap.euler {
            let angles =
                cap.closed_euler.lerp(cap.open_euler, valve) * std::f32::consts::PI / 180.0;
            // Unity applies Z, then X, then Y.
            pose.rotation = Quat::from_rotation_y(angles.y)
                * Quat::from_rotation_x(angles.x)
                * Quat::from_rotation_z(angles.z);
        } else {
            pose.translation = cap.closed.translation.lerp(cap.opened.translation, valve);
            pose.rotation = cap
                .closed
                .rotation
                .lerp(cap.opened.rotation, valve)
                .normalize();
            pose = super::cap_motion::cap_motion(*transform, pose, &mut cap.motion, dt);
        }
        transform.set_if_neq(pose);
    }
    for (binding, mut visibility) in visibility {
        let open = area.interaction.opening_valve(binding.opening) > 0.0;
        visibility.set_if_neq(if open == binding.open {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
}

pub fn sync_items(
    area: &StartingArea,
    alpha: f32,
    query: &mut Query<
        (
            &ItemVisual,
            &mut Transform,
            &mut Visibility,
            Option<&ValveVisibility>,
        ),
        Without<BodyVisual>,
    >,
) {
    for (binding, mut transform, mut visibility, valve) in query {
        let active = area
            .items
            .get(binding.item)
            .is_some_and(|item| item.inactive_pose.is_none());
        visibility.set_if_neq(
            if active
                && valve.is_none_or(|v| (area.interaction.opening_valve(v.opening) > 0.) == v.open)
            {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            },
        );
        if !active {
            continue;
        }
        if let Some((_, previous, current)) = area
            .item_render_poses
            .iter()
            .find(|(id, _, _)| *id == binding.item)
        {
            transform.set_if_neq(
                super::transform(super::interpolate(*previous, *current, alpha))
                    .with_scale(binding.scale),
            );
        }
    }
}

pub fn sync(
    area: &StartingArea,
    physics: &PhysicsWorld,
    alpha: f32,
    query: &mut Query<(
        &BodyVisual,
        &mut Transform,
        &mut Visibility,
        Option<&ValveVisibility>,
    )>,
) {
    for (binding, mut transform, mut visibility, valve) in query {
        let active = area.bodies.get(&binding.body).is_some_and(|h| {
            physics.body_active(*h)
                || matches!(
                    area.inventory_body(*h),
                    Some(crate::interaction::items::ItemLocation::Stored(_))
                )
        });
        visibility.set_if_neq(
            if active
                && valve.is_none_or(|v| (area.interaction.opening_valve(v.opening) > 0.) == v.open)
            {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            },
        );
        if !active {
            continue;
        }
        if let Some((_, previous, current)) = area
            .render_poses
            .iter()
            .find(|(id, _, _)| *id == binding.body)
        {
            transform.set_if_neq(
                super::transform(super::interpolate(*previous, *current, alpha))
                    .with_scale(binding.scale),
            );
        }
    }
}

pub(crate) fn sync_inventory(
    drive: Res<crate::app::Drive>,
    mut query: Query<(&mut InventoryVisibility, &mut Visibility)>,
) {
    use crate::interaction::items::ItemLocation;
    for (mut binding, mut visibility) in &mut query {
        let stored = drive
            .runtime
            .area
            .items
            .get(binding.item)
            .is_some_and(|i| matches!(i.location, ItemLocation::Stored(_)));
        if stored || binding.stored {
            visibility.set_if_neq(if stored {
                if binding.keep {
                    Visibility::Visible
                } else {
                    Visibility::Hidden
                }
            } else {
                Visibility::Inherited
            });
        }
        binding.stored = stored;
    }
}
