//! First-person player body: character 0 of the player prototype (source "Visible body" mode 2).
//! The skin joints are children of `CharacterRoot` and take the `BodyState` pose. The GPU skins the meshes.
//! The body hides while the ragdoll shows.
use super::materials::SourceMaterial;
use super::reference::{ReferenceVisuals, bind_node};
use super::renderers::AuthoredRenderer;
use super::sim_frame::SimFrameResource;
use crate::app::{Drive, RuntimeReference};
use crate::reference::{NodeId, SourceId};
use bevy::ecs as bevy_ecs;
use bevy::gltf::GltfExtras;
use bevy::prelude::*;
use bevy::world_serialization::{WorldAssetRoot, WorldInstanceReady};
use std::collections::HashMap;

#[derive(Resource, Default)]
pub struct PlayerBody {
    scene: Option<Entity>,
    /// Source Player Transform. It follows the player pose.
    player: Option<Entity>,
    /// Source `CharacterRoot`. It follows `BodyState::root_local`.
    character_root: Option<Entity>,
    /// Character 0 skin joints.
    joints: HashMap<NodeId, Entity>,
    /// Skin joint entities in `BodyState` joint order.
    posed: Vec<Entity>,
    /// Character 0 renderer primitives.
    primitives: Vec<Entity>,
    shown: bool,
}
impl PlayerBody {
    /// Entity of the source Player Transform, once the body is bound.
    pub fn player(&self) -> Option<Entity> {
        self.player
    }
    /// Entity of a character 0 skin joint, once the body is bound.
    pub fn joint(&self, node: NodeId) -> Option<Entity> {
        self.joints.get(&node).copied()
    }
}

#[derive(serde::Deserialize)]
pub(super) struct Identity {
    pub reference_id: Option<SourceId>,
}

/// Bind the player nodes. Remove the primitives of all renderers except character 0.
fn bind(
    event: On<WorldInstanceReady>,
    mut commands: Commands,
    reference: Res<RuntimeReference>,
    frame: Res<SimFrameResource>,
    mut body: ResMut<PlayerBody>,
    children: Query<&Children>,
    extras: Query<&GltfExtras>,
    primitives: Query<(Entity, &ChildOf), With<Mesh3d>>,
) {
    let manifest = &reference.0;
    let character = &manifest.clothing.characters[0];
    for entity in children.iter_descendants(event.entity) {
        let Ok(extra) = extras.get(entity) else {
            continue;
        };
        let Some(node) = serde_json::from_str::<Identity>(&extra.value)
            .ok()
            .and_then(|identity| identity.reference_id)
            .and_then(|source| {
                manifest
                    .nodes
                    .iter()
                    .find(|n| n.source == source && n.asset == character.asset)
            })
        else {
            continue;
        };
        if node.id == manifest.clothing.pose.root {
            body.player = Some(entity);
        }
        if node.id == manifest.clothing.pose.anchors["CharacterRoot"].node {
            body.character_root = Some(entity);
        }
        if character.skins.iter().any(|s| s.joints.contains(&node.id)) {
            body.joints.insert(node.id, entity);
        }
        let drawn = character.renderers.iter().find(|r| r.node == node.id);
        let mut removed = false;
        for (primitive, parent) in &primitives {
            if parent.parent() != entity {
                continue;
            }
            if let Some(renderer) = drawn {
                commands
                    .entity(primitive)
                    .insert(AuthoredRenderer(renderer.state.clone()));
                body.primitives.push(primitive);
            } else {
                commands.entity(primitive).despawn();
                removed = true;
            }
        }
        if removed {
            continue;
        }
        if let Err(error) = bind_node(&mut commands, node, entity, extra, &primitives) {
            error!("Player body: {error}");
        }
    }
    let joints = character
        .skins
        .iter()
        .flat_map(|s| &s.joints)
        .all(|id| body.joints.contains_key(id));
    let complete = body.player.is_some() && joints && !body.primitives.is_empty();
    let Some(root) = body.character_root.filter(|_| complete) else {
        error!("Player body glTF lacks its root, CharacterRoot, skin joints, or renderers");
        return;
    };
    // BodyState poses every joint relative to CharacterRoot, so each joint is its direct child.
    let posed: Vec<Entity> = frame.rig().iter().map(|node| body.joints[node]).collect();
    for joint in &posed {
        commands.entity(*joint).insert(ChildOf(root));
    }
    body.posed = posed;
}

pub(crate) fn sync(
    mut commands: Commands,
    reference: Res<RuntimeReference>,
    drive: Res<Drive>,
    frame: Res<SimFrameResource>,
    visuals: Res<ReferenceVisuals>,
    gltfs: Res<Assets<bevy::gltf::Gltf>>,
    mut body: ResMut<PlayerBody>,
    mut nodes: Query<(
        &mut Transform,
        &mut Visibility,
        Has<MeshMaterial3d<SourceMaterial>>,
    )>,
) {
    if !visuals.bound {
        return;
    }
    let Some(scene) = body.scene else {
        let asset = reference.0.clothing.characters[0].asset;
        let Some(gltf) = visuals
            .gltfs
            .iter()
            .find(|(id, _)| *id == asset)
            .and_then(|(_, handle)| gltfs.get(handle))
        else {
            return;
        };
        let scene = commands
            .spawn((
                WorldAssetRoot(gltf.scenes[0].clone()),
                Transform::default(),
                Visibility::Hidden,
            ))
            .observe(bind)
            .id();
        body.scene = Some(scene);
        return;
    };
    let Some(player) = body.player else {
        return;
    };
    // Show the body after its source materials replace the glTF materials.
    if !body.shown {
        if !body
            .primitives
            .iter()
            .all(|p| nodes.get(*p).is_ok_and(|(_, _, source)| source))
        {
            return;
        }
        body.shown = true;
    }
    if let Ok((_, mut visibility, _)) = nodes.get_mut(scene) {
        visibility.set_if_neq(if frame.frame.ragdoll.is_empty() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
    if let Ok((mut transform, _, _)) = nodes.get_mut(player) {
        let scale = transform.scale;
        let pose = drive.rendered_body_pose(frame.frame.state.body_pose.pose());
        transform.set_if_neq(super::transform(pose).with_scale(scale));
    }
    let state = &frame.frame.state;
    if let Some(root) = body.character_root
        && let Ok((mut transform, _, _)) = nodes.get_mut(root)
    {
        let scale = transform.scale;
        transform.set_if_neq(super::transform(state.body_root.pose()).with_scale(scale));
    }
    let scale = Vec3::splat(state.joint_scale);
    for (joint, row) in body.posed.iter().zip(&frame.frame.joints) {
        if let Ok((mut transform, _, _)) = nodes.get_mut(*joint) {
            transform.set_if_neq(super::transform(row.pose()).with_scale(scale));
        }
    }
}
