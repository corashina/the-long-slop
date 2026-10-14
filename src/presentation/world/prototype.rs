//! Source prototype hierarchies captured once and spawned for streamed instances.
use bevy::ecs as bevy_ecs;
use bevy::ecs::system::{IntoObserverSystem, SystemParam};
use bevy::prelude::*;

use crate::presentation::materials::{SourceCaster, SourceMaterial, SourceShadowMode};
use crate::presentation::transparent::SourceSortData;
use crate::reference::{AssetId, NodeId, ReferenceManifest, SourceId};
use bevy::camera::visibility::{NoFrustumCulling, RenderLayers};
use bevy::gltf::GltfExtras;
use bevy::render::batching::NoAutomaticBatching;
use bevy::world_serialization::{WorldAssetRoot, WorldInstanceReady};

type Receiver = (
    &'static Mesh3d,
    &'static MeshMaterial3d<SourceMaterial>,
    &'static Transform,
    &'static Visibility,
    &'static RenderLayers,
    &'static SourceShadowMode,
    Option<&'static bevy::light::NotShadowReceiver>,
    Option<&'static Children>,
    &'static crate::presentation::renderers::SourceRenderer,
    &'static bevy::camera::primitives::Aabb,
    Option<&'static SourceSortData>,
);

/// Imported scene entities that prototypes bind and capture.
#[derive(SystemParam)]
pub(crate) struct SceneQueries<'w, 's> {
    extras: Query<'w, 's, (Entity, &'static GltfExtras)>,
    primitives: Query<'w, 's, (Entity, &'static ChildOf), With<Mesh3d>>,
    receivers: Query<'w, 's, Receiver, Without<SourceCaster>>,
    casters: Query<'w, 's, &'static MeshMaterial3d<StandardMaterial>, With<SourceCaster>>,
}

#[derive(Clone)]
struct MeshRecipe {
    mesh: Handle<Mesh>,
    material: Handle<SourceMaterial>,
    pose: Transform,
    visibility: Visibility,
    layers: RenderLayers,
    shadow_mode: u8,
    receive_shadows: bool,
    caster: Option<Handle<StandardMaterial>>,
    renderer: crate::presentation::renderers::SourceRenderer,
    bounds: bevy::camera::primitives::Aabb,
    sort: Option<SourceSortData>,
}

#[derive(Clone)]
struct NodeRecipe {
    id: NodeId,
    parent: Option<usize>,
    pose: Transform,
    visibility: Visibility,
    meshes: Vec<MeshRecipe>,
}

/// Source nodes in parent order with their bound primitives.
#[derive(Clone)]
pub(crate) struct Recipe {
    nodes: Vec<NodeRecipe>,
}

impl Recipe {
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn mesh_count(&self) -> usize {
        self.nodes.iter().map(|node| node.meshes.len()).sum()
    }

    pub fn node_index(&self, id: NodeId) -> Option<usize> {
        self.nodes.iter().position(|node| node.id == id)
    }

    pub fn validate_assets(
        &self,
        meshes: &Assets<Mesh>,
        materials: &Assets<SourceMaterial>,
        casters: &Assets<StandardMaterial>,
    ) -> Result<(), &'static str> {
        for mesh in self.nodes.iter().flat_map(|node| &node.meshes) {
            if !meshes.contains(mesh.mesh.id()) {
                return Err("mesh");
            }
            if !materials.contains(mesh.material.id()) {
                return Err("material");
            }
            if mesh
                .caster
                .as_ref()
                .is_some_and(|handle| !casters.contains(handle.id()))
            {
                return Err("caster material");
            }
        }
        Ok(())
    }

    /// Spawns the hierarchy under `root`. Repeated source identities belong only to this instance.
    pub fn spawn(&self, commands: &mut Commands, root: Entity) -> Vec<(NodeId, Entity)> {
        let mut nodes: Vec<(NodeId, Entity)> = Vec::with_capacity(self.nodes.len());
        for node in &self.nodes {
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
                    mesh.renderer.clone(),
                    mesh.bounds,
                    bevy::light::NotShadowCaster,
                    ChildOf(entity),
                ));
                if let Some(sort) = mesh.sort {
                    primitive.insert((sort, NoAutomaticBatching));
                }
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
        nodes
    }
}

/// One hidden scene instance of a prototype asset.
#[derive(Clone)]
pub(crate) struct Prototype {
    pub asset: AssetId,
    root: Option<Entity>,
    pub ready: bool,
    pub bound: bool,
    pub nodes: Vec<(NodeId, Entity)>,
}

impl Prototype {
    pub fn new(asset: AssetId) -> Self {
        Self {
            asset,
            root: None,
            ready: false,
            bound: false,
            nodes: Vec::new(),
        }
    }

    pub fn spawned(&self) -> bool {
        self.root.is_some()
    }

    /// Spawns the scene far from the world. `on_ready` sets `ready`.
    pub fn spawn<M>(
        &mut self,
        commands: &mut Commands,
        manifest: &ReferenceManifest,
        visuals: &crate::presentation::reference::ReferenceVisuals,
        gltfs: &Assets<bevy::gltf::Gltf>,
        on_ready: impl IntoObserverSystem<WorldInstanceReady, (), M>,
    ) {
        let asset = manifest.assets.iter().find(|a| a.id == self.asset).unwrap();
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
            .observe(on_ready)
            .id();
        self.root = Some(root);
    }

    /// Binds every source node of the asset. Extras of other assets do not match.
    pub fn bind(
        &mut self,
        commands: &mut Commands,
        manifest: &ReferenceManifest,
        scene: &SceneQueries,
    ) -> Result<(), String> {
        #[derive(serde::Deserialize)]
        struct Identity {
            reference_id: Option<SourceId>,
        }
        for (entity, extra) in &scene.extras {
            let identity: Identity = serde_json::from_str(&extra.value)
                .map_err(|error| format!("Prototype extras: {error}"))?;
            let Some(node) = identity.reference_id.and_then(|id| {
                manifest
                    .nodes
                    .iter()
                    .find(|n| n.source == id && n.asset == self.asset)
            }) else {
                continue;
            };
            crate::presentation::reference::bind_node(
                commands,
                node,
                entity,
                extra,
                &scene.primitives,
            )?;
            if self.nodes.iter().any(|(id, _)| *id == node.id) {
                return Err(format!("Duplicate prototype node {}", node.id.0));
            }
            self.nodes.push((node.id, entity));
        }
        if self.nodes.len()
            != manifest
                .nodes
                .iter()
                .filter(|n| n.asset == self.asset)
                .count()
        {
            return Err("Prototype has missing source nodes".into());
        }
        self.bound = true;
        Ok(())
    }

    pub fn mesh_count(&self, scene: &SceneQueries) -> usize {
        scene
            .primitives
            .iter()
            .filter(|(_, parent)| self.nodes.iter().any(|(_, node)| *node == parent.parent()))
            .count()
    }

    /// True when every shadow caster has its caster child.
    pub fn materials_ready(&self, scene: &SceneQueries) -> bool {
        scene.primitives.iter().all(|(entity, parent)| {
            if !self.nodes.iter().any(|(_, node)| *node == parent.parent()) {
                return true;
            }
            let Ok((_, _, _, _, _, mode, _, children, _, _, _)) = scene.receivers.get(entity)
            else {
                return false;
            };
            mode.0 == 0
                || children
                    .is_some_and(|children| children.iter().any(|c| scene.casters.contains(c)))
        })
    }

    /// Records the bound hierarchy and queues pipeline warmups for each primitive.
    pub fn capture(
        &self,
        commands: &mut Commands,
        manifest: &ReferenceManifest,
        scene: &SceneQueries,
        warmups: &mut Vec<(Entity, bool)>,
    ) -> Result<Recipe, String> {
        let far = Transform::from_translation(Vec3::splat(1_000_000.));
        let mut nodes: Vec<NodeRecipe> = Vec::with_capacity(self.nodes.len());
        for node in manifest.nodes.iter().filter(|n| n.asset == self.asset) {
            let entity = self.nodes.iter().find(|(id, _)| *id == node.id).unwrap().1;
            let parent = match node.parent {
                Some(parent) => Some(
                    nodes
                        .iter()
                        .position(|n| n.id == parent)
                        .ok_or("Prototype source hierarchy is not parent ordered")?,
                ),
                None => None,
            };
            let mut meshes = Vec::new();
            for (primitive, owner) in &scene.primitives {
                if owner.parent() != entity {
                    continue;
                }
                let (
                    mesh,
                    material,
                    pose,
                    visibility,
                    layers,
                    mode,
                    no_receive,
                    children,
                    renderer,
                    bounds,
                    sort,
                ) = scene.receivers.get(primitive).unwrap();
                let caster = children.and_then(|children| {
                    children
                        .iter()
                        .find_map(|child| scene.casters.get(child).ok().map(|m| m.0.clone()))
                });
                let mut warm = commands.spawn((
                    mesh.clone(),
                    material.clone(),
                    crate::presentation::materials::SourceMaterialWarmup,
                    renderer.clone(),
                    *bounds,
                    *mode,
                    // Warmups use camera layer 0. The material keeps its source receiver mask.
                    RenderLayers::layer(0),
                    far,
                    Visibility::Visible,
                    bevy::light::NotShadowCaster,
                    NoFrustumCulling,
                ));
                if let Some(sort) = sort {
                    warm.insert((*sort, NoAutomaticBatching));
                }
                if no_receive.is_some() {
                    warm.insert(bevy::light::NotShadowReceiver);
                }
                warmups.push((warm.id(), false));
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
                    warmups.push((warm, true));
                }
                meshes.push(MeshRecipe {
                    mesh: mesh.0.clone(),
                    material: material.0.clone(),
                    pose: *pose,
                    visibility: *visibility,
                    layers: layers.clone(),
                    shadow_mode: mode.0,
                    receive_shadows: no_receive.is_none(),
                    caster,
                    renderer: renderer.clone(),
                    bounds: *bounds,
                    sort: sort.copied(),
                });
            }
            nodes.push(NodeRecipe {
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
        Ok(Recipe { nodes })
    }

    /// Despawns the scene after pipeline warmup.
    pub fn release(&mut self, commands: &mut Commands) {
        if let Some(root) = self.root.take() {
            commands.entity(root).despawn();
        }
        self.nodes.clear();
    }
}
