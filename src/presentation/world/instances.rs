//! Generated scenes are prepared offscreen before their collision recipes publish.
use super::{
    grass_jobs::GrassJobs,
    instance_materials::{InstanceMaterial, InstanceMaterials, RendererMaterial, ready},
    lod::{LodInstance, LodOwner, LodSchedule, LodTable},
    node_plan::{NodePlan, anchor_local},
    pipelines::{PipelineTicket, WorldPipelines},
    prototype_loader::SourceGltf,
    prototypes::SourcePrototypes,
};
use crate::world::stream::content_index::{ContentSlot, InstanceSelection};
use crate::{
    presentation::materials::{SourceShadowMode, grass::settings::settings_from_source},
    reference::{ReferenceError, ReferenceManifest, WorldPrototypeReference},
    world::{Placement, WorldStream},
};
use bevy::math::{DMat4, DVec3};
use bevy::platform::collections::HashMap;
use bevy::{
    camera::visibility::RenderLayers,
    ecs as bevy_ecs,
    mesh::{VertexAttributeValues, skinning::SkinnedMesh},
    prelude::*,
};
use grass_mesh::{GrassPatch, GrassSettings, GrassView};
use std::sync::Arc;
struct MaterialVariant {
    source: Option<crate::reference::SourceId>,
    binding: RendererMaterial,
    ticket: Option<PipelineTicket>,
}
struct Renderer {
    material_source: Option<crate::reference::SourceId>,
    variants: Vec<MaterialVariant>,
    node: usize,
    entity: Entity,
    mesh: Handle<Mesh>,
    binding: RendererMaterial,
    ticket: Option<PipelineTicket>,
    shadow: bool,
    lod_state: Option<(bool, f32)>,
    // The static node chain of the renderer is active. Spawned nodes carry their own visibility.
    active: bool,
    settings: Option<GrassSettings>,
}
impl Renderer {
    fn show(&self, commands: &mut Commands) {
        let visible = self.active && self.lod_state.is_none_or(|(visible, _)| visible);
        commands.entity(self.entity).insert(if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
    }
}
struct Instance {
    retirement: crate::world::renderer_pending::RendererRetirement,
    continuation: Option<crate::world::renderer_pending::PipelineContinuation>,
    upload: Option<Arc<crate::presentation::fixed_assets::MeshUpload>>,
    caps_bound: bool,
    placement: Placement,
    prototype: usize,
    structure: usize,
    sync_pending: bool,
    /// `VehicleRuntime::presentation_signature` of the last sync of a car copy.
    car_signature: u64,
    content: Option<ContentSlot>,
    lamp_buttons: Vec<usize>,
    food: Option<crate::reference::ItemId>,
    session: u64,
    root: Entity,
    // Entity of each spawned glTF node. Static nodes have none.
    nodes: Vec<Option<Entity>>,
    // Nearest spawned ancestor of each glTF node. `None` is the root.
    anchors: Vec<Option<u32>>,
    active_nodes: Vec<bool>,
    // Local render frame; retain double precision across distant rebases.
    worlds: Vec<DMat4>,
    // Frozen source frame for procedural color, wind, and Grass inputs only.
    source_worlds: Vec<Mat4>,
    // VehiclePose changes position and rotation. Retain the original composed scales.
    scales: Vec<(Vec3, Vec3)>,
    renderers: Vec<Renderer>,
    lod: LodInstance,
    plan: Arc<NodePlan>,
    ready: bool,
    cpu: usize,
    gpu: usize,
    objects: usize,
    visible: bool,
    origin: [f64; 3],
}
impl Instance {
    fn entity(&self, node: usize) -> Result<Entity, ReferenceError> {
        self.nodes[node]
            .ok_or_else(|| ReferenceError("Generated node writer targets a static node".into()))
    }
    /// World matrix of the nearest spawned ancestor of a glTF node.
    fn anchor_world(&self, node: usize, origin: crate::world::WorldOrigin) -> DMat4 {
        self.anchors[node].map_or_else(
            || local_root(&self.placement, origin),
            |anchor| self.worlds[anchor as usize],
        )
    }
    /// Writes a dynamic glTF node pose given relative to its glTF parent.
    fn write_local(
        &self,
        transforms: &mut super::runtime::WorldRoots,
        node: usize,
        local: Transform,
        origin: crate::world::WorldOrigin,
    ) -> Result<(), ReferenceError> {
        let entity = self.entity(node)?;
        let parent = self.plan.parents[node]
            .filter(|&parent| Some(parent) != self.anchors[node])
            .map(|parent| {
                (
                    self.anchor_world(node, origin),
                    self.worlds[parent as usize],
                )
            });
        if let Ok(mut transform) = transforms.get_mut(entity) {
            transform.set_if_neq(anchor_local(parent, local));
        }
        Ok(())
    }
}
impl LodOwner for Instance {
    fn lod(&mut self) -> Option<(&mut LodInstance, &[DMat4])> {
        (!self.retirement.started()).then_some((&mut self.lod, &self.worlds))
    }
}
#[derive(Resource, Default)]
pub struct SourceInstances {
    instances: Vec<Instance>,
    pending: Option<Placement>,
    contents: crate::world::ContentIndex,
    membership: Option<((u64, u64, u64, u64, u64), u64)>,
    bindings_dirty: bool,
    origin: Option<[f64; 3]>,
    bodies: HashMap<rapier3d::prelude::RigidBodyHandle, Vec<usize>>,
    content_instances: HashMap<ContentSlot, Vec<usize>>,
    lamp_instances: HashMap<usize, Vec<usize>>,
    selection: InstanceSelection,
    lods: LodSchedule,
    cache_bytes: usize,
}
impl SourceInstances {
    pub(crate) fn cache_bytes(&self) -> usize {
        self.cache_bytes
    }
    fn measure_cache_bytes(&self) -> usize {
        use crate::interaction::memory::platform_map;
        platform_map(&self.bodies)
            + platform_map(&self.content_instances)
            + platform_map(&self.lamp_instances)
            + self
                .bodies
                .values()
                .chain(self.content_instances.values())
                .chain(self.lamp_instances.values())
                .map(|owners| owners.capacity() * std::mem::size_of::<usize>())
                .sum::<usize>()
            + self
                .instances
                .iter()
                .map(|instance| instance.lamp_buttons.capacity() * std::mem::size_of::<usize>())
                .sum::<usize>()
            + self.contents.retained_bytes()
            + self.selection.retained_bytes()
            + self.lods.retained_bytes()
    }
    pub(crate) fn drained(&self) -> bool {
        self.instances
            .iter()
            .all(|i| i.continuation.as_ref().is_none_or(|c| c.drained()))
    }
    pub(crate) fn startup_audit(&self, grass: &GrassJobs) -> serde_json::Value {
        serde_json::json!({
            "pending_owner": self.pending.as_ref().map(|p| &p.prototype),
            "unready": self.instances.iter().filter(|i| !i.ready).count(),
            "grass_waits": self.instances.iter().filter(|i| !i.ready).flat_map(|i| &i.renderers)
                .filter(|r| r.settings.is_some() && !grass.ready(r.entity)).count(),
            "unready_renderers": self.instances.iter().filter(|i| !i.ready).flat_map(|i| &i.renderers)
                .take(4).map(|r| serde_json::json!({
                    "grass": r.settings.is_some(),
                    "ticket": r.ticket.as_ref().map(|t| format!("{:?}", t.state())),
                })).collect::<Vec<_>>(),
        })
    }
    pub fn ready(&self) -> bool {
        self.instances
            .iter()
            .all(|i| !i.retirement.started() && i.ready && i.visible)
    }
    /// Ready visible instances: root entity, placement, prototype index and node entities by
    /// glTF node.
    pub(crate) fn shown(
        &self,
    ) -> impl Iterator<Item = (Entity, &Placement, usize, &[Option<Entity>])> {
        self.instances
            .iter()
            .filter(|i| i.ready && i.visible && !i.retirement.started())
            .map(|i| (i.root, &i.placement, i.prototype, i.nodes.as_slice()))
    }
    /// Node entities by glTF node of the instance of world object `id`.
    pub(crate) fn nodes_of(&self, id: &crate::world::GeneratedId) -> Option<&[Option<Entity>]> {
        self.instances
            .iter()
            .find(|i| i.placement.id == *id && !i.retirement.started())
            .map(|i| i.nodes.as_slice())
    }
    /// Whether an instance with root entity `root` exists.
    pub(crate) fn has_root(&self, root: Entity) -> bool {
        self.instances.iter().any(|i| i.root == root)
    }
    pub(crate) fn uses_prototype(&self, source: &crate::reference::SourceId) -> bool {
        self.instances
            .iter()
            .any(|i| i.placement.prototype == *source)
    }
    pub fn roots(&self) -> impl Iterator<Item = Entity> + '_ {
        self.instances.iter().map(|i| i.root)
    }
    pub fn release_all(&mut self, commands: &mut Commands, materials: &mut InstanceMaterials) {
        for instance in self.instances.drain(..) {
            commands.entity(instance.root).despawn();
            for renderer in instance.renderers {
                materials.release(renderer.binding);
                for variant in renderer.variants {
                    materials.release(variant.binding);
                }
            }
        }
        materials.cache.generated.reset();
        self.pending = None;
        self.bindings_dirty = true;
        self.membership = None;
        self.bodies.clear();
        self.content_instances.clear();
        self.lamp_instances.clear();
        self.selection.reset(0);
        self.lods.reset();
        self.origin = None;
        self.cache_bytes = self.measure_cache_bytes();
    }

    fn rebase(&mut self, origin: crate::world::WorldOrigin, commands: &mut Commands) {
        if self.origin == Some(origin.offset) {
            return;
        }
        self.origin = Some(origin.offset);
        for (index, instance) in self.instances.iter_mut().enumerate() {
            if instance.origin == origin.offset {
                continue;
            }
            let shift = DVec3::from_array(std::array::from_fn(|axis| {
                instance.origin[axis] - origin.offset[axis]
            }));
            for world in &mut instance.worlds {
                world.w_axis += shift.extend(0.);
            }
            commands.entity(instance.root).insert(Transform {
                translation: Vec3::from_array(origin.local(instance.placement.pose.position)),
                rotation: Quat::from_array(instance.placement.pose.rotation),
                scale: Vec3::from_array(instance.placement.scale),
            });
            instance.origin = origin.offset;
            instance.sync_pending = true;
            self.selection.push(index);
        }
    }
    pub(crate) fn measurement_meshes(
        &self,
    ) -> impl Iterator<Item = (bevy::asset::AssetId<Mesh>, bool)> + '_ {
        self.instances
            .iter()
            .flat_map(|i| i.renderers.iter().map(move |r| (r.mesh.id(), !i.ready)))
    }

    pub fn request_descriptors(
        &self,
        stream: &mut WorldStream,
        reference: &ReferenceManifest,
        prototypes: &mut SourcePrototypes,
    ) -> Result<(), ReferenceError> {
        // Discover selected descriptors before material or payload readiness.
        let required_ready = stream.required_ready();
        for index in 0..stream.pending_structures().len() {
            let placement = &stream.pending_structures()[index];
            if !stream.structure_required(&placement.id)
                && !stream.structure_replay_required(&placement.id)
                && !required_ready
            {
                continue;
            }
            let source = &placement.prototype;
            if prototypes.requested(source) {
                continue;
            }
            let prototype = reference
                .world
                .prototypes
                .iter()
                .find(|p| p.source == *source)
                .ok_or_else(|| ReferenceError("Missing generated structure prototype".into()))?;
            prototypes.request(stream, &prototype.source)?;
        }
        Ok(())
    }
    pub fn prepare(
        &mut self,
        stream: &mut WorldStream,
        reference: &ReferenceManifest,
        prototypes: &mut SourcePrototypes,
        server: &AssetServer,
        assets: &Assets<SourceGltf>,
        meshes: &mut Assets<Mesh>,
        materials: &mut InstanceMaterials,
        grass: &mut GrassJobs,
        pipelines: &mut WorldPipelines,
        fixed: &crate::presentation::fixed_assets::SourceFixedAssets,
        commands: &mut Commands,
        view: GrassView,
    ) -> Result<(), ReferenceError> {
        self.rebase(stream.origin, commands);
        if self.pending.as_ref().is_some_and(|p| {
            !stream
                .pending_structures()
                .iter()
                .any(|wanted| wanted.id == p.id)
        }) {
            self.pending = None;
        }
        if self.pending.is_none() {
            let required_ready = stream.required_ready();
            self.pending = stream
                .pending_structures()
                .iter()
                .filter(|p| {
                    stream.contents_initialized(&p.id)
                        && (stream.structure_required(&p.id)
                            || stream.structure_replay_required(&p.id)
                            || required_ready)
                        && !self.instances.iter().any(|i| i.placement.id == p.id)
                })
                .min_by(|a, b| {
                    stream
                        .focus_distance_squared(a.pose.position)
                        .total_cmp(&stream.focus_distance_squared(b.pose.position))
                })
                .cloned();
        }
        // Full pending ownership must drain through publication before another instance starts.
        // Continue polling existing renderer work below, including its readiness transfer.
        if let Some(placement) = self.pending.as_ref().filter(|placement| {
            let child = reference
                .world
                .prototypes
                .iter()
                .find(|p| p.source == placement.prototype)
                .is_some_and(|p| {
                    p.deferred_materials
                        .iter()
                        .any(|m| m.shader.name == "Stix Games/Grass")
                });
            stream.can_request_independent(1 + usize::from(child))
        }) {
            let prototype_index = reference
                .world
                .prototypes
                .iter()
                .position(|p| p.source == placement.prototype)
                .ok_or_else(|| ReferenceError("Missing generated structure prototype".into()))?;
            let prototype = &reference.world.prototypes[prototype_index];
            let admitted = prototypes
                .plan(stream.frame().request.session, &placement.prototype)
                .is_some()
                && materials.admit(
                    stream,
                    reference,
                    prototype,
                    placement,
                    prototypes
                        .plan(stream.frame().request.session, &placement.prototype)
                        .unwrap(),
                )?;
            if admitted {
                prototypes.admit_payload(stream, reference, server, &placement.prototype)?;
                let lods = prototypes.lods(
                    stream.frame().request.session,
                    &placement.prototype,
                    || {
                        LodTable::for_prototype(&reference.world, prototype, |source| {
                            stream.lod_owner(source)
                        })
                    },
                )?;
                let node_plan = prototypes.nodes(
                    stream.frame().request.session,
                    &placement.prototype,
                    |loaded| {
                        let asset = assets.get(&loaded.asset).ok_or_else(|| {
                            ReferenceError("Missing generated prototype asset".into())
                        })?;
                        let count = loaded.plan.nodes.len();
                        let mut sources = vec![None; count];
                        let mut dynamic = vec![false; count];
                        let moving = stream.moving_nodes(reference, prototype)?;
                        for (index, node) in prototype.nodes.iter().enumerate() {
                            sources[node.gltf_node as usize] = Some(index as u32);
                            dynamic[node.gltf_node as usize] |= moving[index];
                        }
                        // Skin joints move with their nodes.
                        for &joint in asset.joints.iter().flatten() {
                            dynamic[joint] = true;
                        }
                        let children = loaded
                            .plan
                            .nodes
                            .iter()
                            .map(|n| n.children.as_slice())
                            .collect::<Vec<_>>();
                        Ok(NodePlan::new(&children, sources, dynamic))
                    },
                )?;
                if let (Some(loaded), Some(lods), Some(node_plan)) = (
                    prototypes.loaded(stream.frame().request.session, &placement.prototype),
                    lods,
                    node_plan,
                ) {
                    if let Some(asset) = assets.get(&loaded.asset) {
                        let count = loaded.plan.nodes.len();
                        let primitives = loaded
                            .plan
                            .nodes
                            .iter()
                            .map(|n| {
                                loaded
                                    .plan
                                    .primitives
                                    .iter()
                                    .filter(|p| Some(p.mesh) == n.mesh)
                                    .count()
                            })
                            .sum::<usize>();
                        // Reservations include ECS storage, transforms, material bind groups, and LOD membership.
                        let mut cpu = count
                            * (2048
                                + std::mem::size_of::<DMat4>()
                                + std::mem::size_of::<(Vec3, Vec3)>()
                                + std::mem::size_of::<Option<u32>>())
                            + std::mem::size_of::<Vec<()>>()
                            + primitives * (8192 + crate::presentation::caster_material::CPU_BYTES);
                        let mut gpu =
                            primitives * (2048 + crate::presentation::caster_material::gpu_bytes());
                        let objects = count + primitives;
                        let mut continuation = stream.reserve_instance_renderer(
                            stream.frame().request,
                            &placement.id,
                            cpu,
                            gpu,
                            objects,
                        )?;
                        let root_pose = local_root(placement, stream.origin);
                        let root = commands
                            .spawn((
                                Transform::from_matrix(root_pose.as_mat4()),
                                Visibility::Hidden,
                            ))
                            .id();
                        let mut owned_bindings = Vec::with_capacity(primitives);
                        let construction = (|| -> Result<Instance, ReferenceError> {
                            let source_root = Mat4::from_scale_rotation_translation(
                                Vec3::from_array(placement.scale),
                                Quat::from_array(placement.pose.rotation),
                                Vec3::from_array(placement.pose.position.map(|v| v as f32)),
                            );
                            let appearance = stream.appearance(&placement.id);
                            let locals = loaded
                                .plan
                                .nodes
                                .iter()
                                .map(|node| {
                                    let local = if let Some(matrix) = node.matrix {
                                        Mat4::from_cols_array(&matrix)
                                    } else {
                                        Mat4::from_scale_rotation_translation(
                                            Vec3::from_array(node.scale),
                                            Quat::from_array(node.rotation),
                                            Vec3::from_array(node.translation),
                                        )
                                    };
                                    match node_plan.sources[node.index as usize] {
                                        Some(source) => override_local(
                                            local,
                                            appearance,
                                            &prototype.nodes[source as usize].source,
                                        ),
                                        None => local,
                                    }
                                })
                                .collect::<Vec<_>>();
                            let active_nodes = (0..prototype.nodes.len())
                                .map(|i| active(prototype, i, appearance))
                                .collect::<Vec<_>>();
                            let flat = node_plan.flatten(&locals);
                            let worlds = flat
                                .chain
                                .iter()
                                .map(|chain| root_pose * chain.as_dmat4())
                                .collect::<Vec<_>>();
                            let source_worlds = flat
                                .chain
                                .iter()
                                .map(|chain| source_root * *chain)
                                .collect::<Vec<_>>();
                            // Parents spawn before children, so each anchor entity exists.
                            let mut nodes = vec![None; count];
                            for &node in &node_plan.order {
                                let node = node as usize;
                                if !flat.spawned[node] {
                                    continue;
                                }
                                let parent = flat.anchors[node].map_or(root, |a| {
                                    nodes[a as usize].expect("anchor spawns first")
                                });
                                let visible = node_plan.sources[node]
                                    .is_none_or(|source| active_nodes[source as usize]);
                                nodes[node] = Some(
                                    commands
                                        .spawn((
                                            Transform::from_matrix(flat.rest[node]),
                                            if visible {
                                                Visibility::Inherited
                                            } else {
                                                Visibility::Hidden
                                            },
                                            ChildOf(parent),
                                        ))
                                        .id(),
                                );
                            }
                            let mut renderers = Vec::with_capacity(primitives);
                            for (source_index, source_node) in prototype.nodes.iter().enumerate() {
                                if !source_node.renderer_enabled {
                                    continue;
                                }
                                let node = &loaded.plan.nodes[source_node.gltf_node as usize];
                                let Some(mesh_index) = node.mesh else {
                                    continue;
                                };
                                let extras = node.extras.as_ref().ok_or_else(|| {
                                    ReferenceError("Missing generated renderer metadata".into())
                                })?;
                                let shadow = extras.cast_shadows.ok_or_else(|| {
                                    ReferenceError("Missing generated shadow mode".into())
                                })?;
                                let receive = extras.receive_shadows.ok_or_else(|| {
                                    ReferenceError("Missing generated receiver state".into())
                                })?;
                                // LOD uniforms and warm tickets retain each renderer's original handles.
                                let has_lod = !lods.levels(source_index).is_empty();
                                for primitive in loaded
                                    .plan
                                    .primitives
                                    .iter()
                                    .filter(|p| p.mesh == mesh_index)
                                {
                                    let index = asset
                                        .primitives
                                        .iter()
                                        .position(|&(n, m, p)| {
                                            n == node.index as usize
                                                && m == primitive.mesh as usize
                                                && p == primitive.primitive as usize
                                        })
                                        .ok_or_else(|| {
                                            ReferenceError(
                                                "Missing generated primitive mesh".into(),
                                            )
                                        })?;
                                    let mesh = asset.meshes[index].clone();
                                    let world = worlds[node.index as usize].as_mat4();
                                    let source_position =
                                        source_worlds[node.index as usize].w_axis.truncate()
                                            * Vec3::new(1., 1., -1.);
                                    let appearance =
                                        stream.appearance(&placement.id).iter().rev().find(|a| {
                                            a.node == source_node.source
                                                && (a.primary.is_some()
                                                    || a.material.is_some()
                                                    || a.color.is_some()
                                                    || a.color_material.is_some())
                                        });
                                    let (initial, selected) =
                                        super::instance_materials::biome_material(
                                            stream,
                                            reference,
                                            prototype,
                                            placement,
                                            source_index,
                                            primitive,
                                            super::instance_materials::selected_material(
                                                stream,
                                                reference,
                                                prototype,
                                                placement,
                                                &source_node.source,
                                            ),
                                        );
                                    let Some(mut binding) = materials.bind(
                                        stream.frame().request.session,
                                        prototype,
                                        primitive,
                                        extras.source_renderer.as_ref().ok_or_else(|| {
                                            ReferenceError("Missing source renderer record".into())
                                        })?,
                                        initial,
                                        selected,
                                        source_node.layer,
                                        receive,
                                        world.determinant() < 0.,
                                        source_position,
                                        view,
                                    )?
                                    else {
                                        continue;
                                    };
                                    if let Some(appearance) = appearance {
                                        let color = appearance.color.or_else(|| {
                                            appearance.color_material.as_ref().and_then(|id| {
                                                reference
                                                    .rendering
                                                    .materials
                                                    .iter()
                                                    .find(|m| &m.source == id)
                                                    .map(|m| m.color)
                                                    .or_else(|| {
                                                        prototype
                                                            .deferred_materials
                                                            .iter()
                                                            .find(|m| &m.source == id)
                                                            .and_then(|m| {
                                                                m.colors.get("_Color").copied()
                                                            })
                                                    })
                                            })
                                        });
                                        if let Some(color) = color {
                                            materials.color(&mut binding, color)?;
                                        }
                                    }
                                    if has_lod {
                                        materials.make_private(&mut binding);
                                    }
                                    let is_grass =
                                        matches!(binding.visible, InstanceMaterial::Grass(_));
                                    // A static node bakes its matrix into the renderer.
                                    let (parent, transform) = match nodes[node.index as usize] {
                                        Some(entity) => (entity, Transform::default()),
                                        None => (
                                            flat.anchors[node.index as usize]
                                                .map_or(root, |a| nodes[a as usize].unwrap()),
                                            Transform::from_matrix(flat.rest[node.index as usize]),
                                        ),
                                    };
                                    let chain_active = nodes[node.index as usize].is_some()
                                        || active_nodes[source_index];
                                    let entity = commands
                                        .spawn((
                                            transform,
                                            if chain_active {
                                                Visibility::Inherited
                                            } else {
                                                Visibility::Hidden
                                            },
                                            ChildOf(parent),
                                            RenderLayers::layer(source_node.layer as usize),
                                            SourceShadowMode(shadow),
                                            crate::presentation::renderer_visibility::SourceRenderer(
                                                extras.source_renderer.clone().ok_or_else(
                                                    || {
                                                        ReferenceError(
                                                            "Missing source renderer record".into(),
                                                        )
                                                    },
                                                )?,
                                            ),
                                            bevy::light::NotShadowCaster,
                                        ))
                                        .id();
                                    owned_bindings.push((entity, binding.clone()));
                                    let material_source = selected.cloned();
                                    let mut variant_ids = stream
                                        .lamp_materials(&prototype.source, &source_node.source)?
                                        .to_vec();
                                    variant_ids.retain(|id| Some(id) != material_source.as_ref());
                                    let mut variants = Vec::with_capacity(variant_ids.len());
                                    if !variant_ids.is_empty() {
                                        let extra_cpu = variant_ids.len()
                                            * (8192
                                                + crate::presentation::caster_material::CPU_BYTES);
                                        let extra_gpu = variant_ids.len()
                                            * (2048
                                                + crate::presentation::caster_material::gpu_bytes(
                                                ));
                                        let mut allocation = stream.presentation_allocation();
                                        allocation.active_cpu += extra_cpu;
                                        allocation.active_gpu += extra_gpu;
                                        stream.set_presentation_allocation(allocation)?;
                                        cpu += extra_cpu;
                                        gpu += extra_gpu;
                                    }
                                    for source in variant_ids {
                                        let initial = reference
                                            .rendering
                                            .materials
                                            .iter()
                                            .find(|m| {
                                                m.source == source && m.asset == prototype.asset
                                            })
                                            .map(|m| (m.asset, m.gltf_material));
                                        let mut variant = materials
                                            .bind(
                                                stream.frame().request.session,
                                                prototype,
                                                primitive,
                                                extras.source_renderer.as_ref().ok_or_else(
                                                    || {
                                                        ReferenceError(
                                                            "Missing source renderer record".into(),
                                                        )
                                                    },
                                                )?,
                                                initial,
                                                Some(&source),
                                                source_node.layer,
                                                receive,
                                                world.determinant() < 0.,
                                                source_position,
                                                view,
                                            )?
                                            .ok_or_else(|| {
                                                ReferenceError(
                                                    "Generated lamp variant material missing"
                                                        .into(),
                                                )
                                            })?;
                                        if has_lod {
                                            materials.make_private(&mut variant);
                                        }
                                        owned_bindings.push((entity, variant.clone()));
                                        variants.push(MaterialVariant {
                                            source: Some(source),
                                            binding: variant,
                                            ticket: None,
                                        });
                                    }
                                    if !receive {
                                        commands
                                            .entity(entity)
                                            .insert(bevy::light::NotShadowReceiver);
                                    }
                                    if !is_grass {
                                        commands.entity(entity).insert(Mesh3d(mesh.clone()));
                                        binding.visible.insert(entity, commands);
                                        binding.apply_sort(entity, commands);
                                    }
                                    if let Some(skin) = node.skin {
                                        let joints = asset
                                            .joints
                                            .get(skin as usize)
                                            .ok_or_else(|| {
                                                ReferenceError(
                                                    "Missing generated skin joints".into(),
                                                )
                                            })?
                                            .iter()
                                            .map(|&j| {
                                                nodes[j].ok_or_else(|| {
                                                    ReferenceError(
                                                        "Generated skin joint is static".into(),
                                                    )
                                                })
                                            })
                                            .collect::<Result<_, _>>()?;
                                        let binding = SkinnedMesh {
                                            inverse_bindposes: asset.inverse_bindposes
                                                [skin as usize]
                                                .clone(),
                                            joints,
                                        };
                                        commands.entity(entity).insert(binding);
                                    }
                                    let (_patches, settings) = if is_grass {
                                        let record = prototype
                                            .deferred_materials
                                            .iter()
                                            .find(|m| {
                                                Some(&m.source)
                                                    == primitive.source_material.as_ref()
                                            })
                                            .ok_or_else(|| {
                                                ReferenceError(
                                                    "Missing Grass source material".into(),
                                                )
                                            })?;
                                        let mesh_data = meshes.get(&mesh).ok_or_else(|| {
                                            ReferenceError("Missing Grass prototype mesh".into())
                                        })?;
                                        let patch_bytes = mesh_data
                                            .indices()
                                            .ok_or_else(|| {
                                                ReferenceError(
                                                    "Missing Grass source triangles".into(),
                                                )
                                            })?
                                            .len()
                                            / 3
                                            * std::mem::size_of::<GrassPatch>()
                                            * 2;
                                        let mut a = stream.presentation_allocation();
                                        a.active_cpu += patch_bytes;
                                        stream.set_presentation_allocation(a)?;
                                        cpu += patch_bytes;
                                        let source_world = source_worlds[node.index as usize];
                                        let patches = grass_patches(mesh_data, source_world)?;
                                        let settings = settings_from_source(record)?;
                                        if let InstanceMaterial::Grass(handle) = &binding.visible {
                                            let bounds = grass_bounds(
                                                &patches,
                                                source_world,
                                                materials
                                                    .grass
                                                    .get(handle)
                                                    .expect("bound Grass material")
                                                    .displacement_radius(),
                                            );
                                            grass.request(
                                                stream,
                                                entity,
                                                root,
                                                Arc::clone(&patches),
                                                settings,
                                                handle.clone(),
                                                view,
                                                shadow != 0,
                                                bounds,
                                            )?;
                                        }
                                        (Some(patches), Some(settings))
                                    } else {
                                        (None, None)
                                    };
                                    renderers.push(Renderer {
                                        material_source,
                                        variants,
                                        node: source_index,
                                        entity,
                                        mesh,
                                        binding,
                                        ticket: None,
                                        lod_state: None,
                                        active: chain_active,
                                        shadow: shadow != 0,
                                        settings,
                                    });
                                }
                            }
                            Ok(Instance {
                                retirement: Default::default(),
                                continuation: None,
                                upload: None,
                                caps_bound: false,
                                placement: placement.clone(),
                                prototype: prototype_index,
                                structure: usize::MAX,
                                sync_pending: true,
                                car_signature: 0,
                                content: None,
                                lamp_buttons: Vec::new(),
                                food: None,
                                session: stream.frame().request.session,
                                root,
                                active_nodes,
                                nodes,
                                anchors: flat.anchors,
                                scales: worlds
                                    .iter()
                                    .zip(&source_worlds)
                                    .map(|(world, source)| {
                                        (
                                            world.as_mat4().to_scale_rotation_translation().0,
                                            source.to_scale_rotation_translation().0,
                                        )
                                    })
                                    .collect(),
                                worlds,
                                source_worlds,
                                renderers,
                                lod: LodInstance::new(lods.clone()),
                                plan: node_plan.clone(),
                                ready: false,
                                cpu,
                                gpu,
                                objects,
                                visible: false,
                                origin: stream.origin.offset,
                            })
                        })();
                        match construction {
                            Ok(mut instance) => {
                                let upload = fixed.watch_meshes(
                                    instance.renderers.iter().map(|r| r.mesh.clone()),
                                );
                                continuation.watch_upload(upload.receipt.clone());
                                instance.continuation = Some(continuation);
                                instance.upload = Some(upload);
                                self.lods.spawn(&mut instance.lod, self.instances.len());
                                self.instances.push(instance);
                                self.bindings_dirty = true;
                            }
                            Err(error) => {
                                commands.entity(root).despawn();
                                for (entity, binding) in owned_bindings {
                                    grass.retire(stream, entity, meshes, &mut materials.grass)?;
                                    materials.release(binding);
                                }
                                let mut a = stream.presentation_allocation();
                                a.active_cpu = a.active_cpu.saturating_sub(cpu);
                                a.active_gpu = a.active_gpu.saturating_sub(gpu);
                                a.objects = a.objects.saturating_sub(objects);
                                stream.set_presentation_allocation(a)?;
                                stream.release_renderer_pending(&mut continuation)?;
                                return Err(error);
                            }
                        }
                        self.pending = None;
                    }
                }
            }
        }
        for instance in &mut self.instances {
            if instance.retirement.started()
                || instance.ready
                || instance.session != stream.frame().request.session
                || !stream
                    .pending_structures()
                    .iter()
                    .any(|p| p.id == instance.placement.id)
            {
                continue;
            }
            let mut complete = true;
            for renderer in &mut instance.renderers {
                if matches!(renderer.binding.visible, InstanceMaterial::Grass(_)) {
                    complete &= grass.ready(renderer.entity);
                    continue;
                }
                PipelineTicket::retry(&mut renderer.ticket);
                if renderer.ticket.is_none() {
                    renderer.ticket = renderer.binding.visible.warm(
                        stream,
                        pipelines,
                        instance.continuation.as_mut().unwrap(),
                        &renderer.mesh,
                        renderer.shadow,
                    )?;
                }
                complete &= ready(&renderer.ticket)?;
                for variant in &mut renderer.variants {
                    PipelineTicket::retry(&mut variant.ticket);
                    if variant.ticket.is_none() {
                        variant.ticket = variant.binding.visible.warm(
                            stream,
                            pipelines,
                            instance.continuation.as_mut().unwrap(),
                            &renderer.mesh,
                            renderer.shadow,
                        )?;
                    }
                    complete &= ready(&variant.ticket)?;
                }
            }
            if complete && instance.continuation.as_ref().unwrap().drained() {
                stream.bind_prototype(&instance.placement.prototype)?;
                stream.bind_structure_recipe(&instance.placement.prototype)?;
                let claim = instance.continuation.as_mut().unwrap();
                stream.accept_structure_renderer(claim.request(), &instance.placement.id, claim)?;
                instance.ready = true;
                instance.continuation = None;
                instance.upload = None;
            }
        }
        Ok(())
    }
    pub fn sync(
        &mut self,
        stream: &WorldStream,
        reference: &ReferenceManifest,
        physics: &crate::vehicle::PhysicsWorld,
        items: &crate::interaction::items::ItemStore,
        vehicles: &std::collections::BTreeMap<
            crate::world::ObjectKey,
            crate::starting_area::VehicleRuntime,
        >,
        commands: &mut Commands,
        transforms: &mut super::runtime::WorldRoots,
        materials: &mut InstanceMaterials,
    ) -> Result<(), ReferenceError> {
        self.rebase(stream.origin, commands);
        let membership = (
            stream.presentation_membership(),
            items.membership_revision(),
        );
        if self.bindings_dirty || self.membership != Some(membership) {
            let session_changed = self
                .membership
                .is_none_or(|previous| previous.0.0 != membership.0.0);
            stream.index_contents(&mut self.contents);
            self.selection.reset(self.instances.len());
            for owners in self.bodies.values_mut() {
                owners.clear();
            }
            for owners in self.content_instances.values_mut() {
                owners.clear();
            }
            for owners in self.lamp_instances.values_mut() {
                owners.clear();
            }
            let frame = stream.frame();
            let mut published = HashMap::new();
            for (index, structure) in frame.structures.iter().enumerate() {
                published.entry(&structure.placement.id).or_insert(index);
            }
            for (index, instance) in self.instances.iter_mut().enumerate() {
                let content = self.contents.slot(&instance.placement.id);
                let lamp_buttons = self.contents.lamp_slots(&instance.placement.id);
                let food = items.generated_item_id(&instance.placement.id);
                let structure = published
                    .get(&instance.placement.id)
                    .copied()
                    .unwrap_or(usize::MAX);
                instance.sync_pending |= session_changed
                    || instance.content != content
                    || instance.lamp_buttons.as_slice() != lamp_buttons
                    || instance.food != food
                    || instance.structure != structure;
                instance.content = content;
                instance.lamp_buttons.clear();
                instance.lamp_buttons.extend_from_slice(lamp_buttons);
                instance.food = food;
                instance.structure = structure;
                if let Some(structure) = frame.structures.get(instance.structure) {
                    for body in &structure.bodies {
                        self.bodies.entry(*body).or_default().push(index);
                    }
                }
                if let Some(slot) = instance.content {
                    self.content_instances.entry(slot).or_default().push(index);
                }
                for lamp in &instance.lamp_buttons {
                    self.lamp_instances.entry(*lamp).or_default().push(index);
                }
                if instance.sync_pending {
                    self.selection.push(index);
                }
            }
            self.bodies.retain(|_, owners| !owners.is_empty());
            self.content_instances
                .retain(|_, owners| !owners.is_empty());
            self.lamp_instances.retain(|_, owners| !owners.is_empty());
            self.membership = Some(membership);
            self.bindings_dirty = false;
            self.cache_bytes = self.measure_cache_bytes();
        }
        for body in physics.presentation_bodies() {
            if let Some(owners) = self.bodies.get(&body) {
                for index in owners {
                    self.selection.push(*index);
                }
            }
        }
        for slot in stream.presentation_contents() {
            if let Some(owners) = self.content_instances.get(&slot) {
                for index in owners {
                    self.selection.push(*index);
                }
            }
        }
        for lamp in stream.presentation_lamps() {
            if let Some(owners) = self.lamp_instances.get(&lamp) {
                for index in owners {
                    self.selection.push(*index);
                }
            }
        }
        // World car gauges and lamps change on a still car too. Sync a car copy only when
        // its presented state changed. The sync records the signature once bindings are ready.
        for (key, vehicle) in vehicles {
            let Some(owners) = self.bodies.get(&physics.chassis_handle(key)) else {
                continue;
            };
            let signature = vehicle.presentation_signature();
            for &index in owners {
                if self.instances[index].car_signature != signature {
                    self.selection.push(index);
                }
            }
        }
        let frame = stream.frame();
        let selected = self.selection.len();
        for selected_index in 0..selected {
            self.instances[self.selection.at(selected_index)].sync_pending = true;
        }
        for selected_index in 0..selected {
            let instance_index = self.selection.at(selected_index);
            let instance = &mut self.instances[instance_index];
            if instance.retirement.started() {
                continue;
            }
            let content = stream.content_at(instance.content);
            if !content.bindings_ready() {
                if instance.visible {
                    commands.entity(instance.root).insert(Visibility::Hidden);
                    instance.visible = false;
                }
                continue;
            }
            let Some(published) = frame.structures.get(instance.structure) else {
                continue;
            };
            if !instance.visible {
                commands.entity(instance.root).insert(Visibility::Inherited);
                instance.visible = true;
            }
            let prototype = &reference.world.prototypes[instance.prototype];
            // The car runtime of a car copy.
            let car = prototype.vehicle.as_ref().and_then(|record| {
                let handle = published.bodies.get(record.body as usize)?;
                Some((record, vehicles.get(physics.vehicle_of(*handle)?)?))
            });
            if let Some((_, vehicle)) = car {
                instance.car_signature = vehicle.presentation_signature();
            }
            if !instance.caps_bound {
                for cap in content.caps() {
                    crate::presentation::reference::bind_generated_cap(
                        commands,
                        cap,
                        prototype,
                        &instance.nodes,
                    )?;
                }
                instance.caps_bound = true;
            }
            let appearance = stream.indexed_appearance(&content, &instance.placement.id);
            let food = instance
                .food
                .and_then(|id| items.get(id))
                .filter(|item| item.food.is_some());
            if let Some(item) = food {
                let recipe = &reference.survival.foods[item.food.unwrap().prototype_index];
                let root = reference
                    .nodes
                    .iter()
                    .find(|n| n.id == recipe.root)
                    .ok_or_else(|| ReferenceError("Missing food root".into()))?;
                if prototype
                    .nodes
                    .iter()
                    .filter(|n| n.parent.is_none())
                    .count()
                    != 1
                    || !prototype
                        .nodes
                        .iter()
                        .any(|n| n.parent.is_none() && n.source == root.source)
                {
                    return Err(ReferenceError(
                        "Generated food recipe does not own its complete source instance".into(),
                    ));
                }
            }
            if let Some(radio) = stream.radio(&instance.placement.id) {
                let node = prototype.nodes[radio.needle_node].gltf_node as usize;
                instance.write_local(transforms, node, local_pose(radio.needle), frame.origin)?;
            }
            if let Some(music_box) = stream.music_box(&instance.placement.id) {
                for n in &music_box.nodes {
                    let node = prototype.nodes[n.node].gltf_node as usize;
                    instance.write_local(transforms, node, local_pose(n.pose), frame.origin)?;
                }
            }
            if let Some(detector) = stream.detector(&instance.placement.id) {
                let n = &detector.needle;
                let node = prototype.nodes[n.node].gltf_node as usize;
                instance.write_local(transforms, node, local_pose(n.pose), frame.origin)?;
            }
            if let Some(dial) = stream.dial(&instance.placement.id) {
                for n in dial.nodes() {
                    let node = prototype.nodes[n.node].gltf_node as usize;
                    instance.write_local(transforms, node, local_pose(n.pose), frame.origin)?;
                }
            }
            if let Some((record, vehicle)) = car
                && let (Some(r), Some(state)) = (&record.instruments, &vehicle.instruments)
            {
                let needles = r.needles.iter().zip(&state.needle_angles).map(|(n, &a)| {
                    let mut e = n.rest;
                    e[n.axis as usize] = a;
                    (n.node, e)
                });
                let digits = r
                    .odometer
                    .digits
                    .iter()
                    .zip(&state.digit_angles)
                    .map(|(d, &a)| {
                        let mut e = [0.; 3];
                        e[d.axis as usize] = a * if d.inv { -1. } else { 1. };
                        (d.node, e)
                    });
                for (node, e) in needles.chain(digits) {
                    let node = prototype.nodes[node.0 as usize].gltf_node as usize;
                    if let Ok(mut transform) = transforms.get_mut(instance.entity(node)?) {
                        let rotation =
                            Quat::from_array(crate::simulation::instruments::source_rotation(e));
                        if transform.rotation != rotation {
                            transform.rotation = rotation;
                        }
                    }
                }
            }
            if let Some(trigger) = stream.spray_trigger(&instance.placement.id)
                && let Some(n) = prototype
                    .nodes
                    .iter()
                    .find(|n| n.source == reference.small_items.spray_bottle.trigger)
            {
                instance.write_local(
                    transforms,
                    n.gltf_node as usize,
                    local_pose(trigger),
                    frame.origin,
                )?;
            }
            if let Some(level) = stream.level(&instance.placement.id) {
                for b in level.bubbles() {
                    let node = prototype.nodes[b.node].gltf_node as usize;
                    instance.write_local(transforms, node, local_pose(b.pose), frame.origin)?;
                }
            }
            for (i, node) in prototype.nodes.iter().enumerate() {
                if let Some((_, pose_value)) = instance
                    .lamp_buttons
                    .iter()
                    .filter_map(|index| stream.lamp_button_at(*index))
                    .find(|(button, _)| *button == &node.source)
                {
                    instance.write_local(
                        transforms,
                        node.gltf_node as usize,
                        local_pose(pose_value),
                        frame.origin,
                    )?;
                }
                let enabled = active_food(prototype, i, appearance, reference, food)
                    && car
                        .and_then(|(record, v)| v.node_visible(record, i as u32))
                        .unwrap_or(true);
                if enabled != instance.active_nodes[i] {
                    instance.active_nodes[i] = enabled;
                    if let Some(entity) = instance.nodes[node.gltf_node as usize] {
                        commands.entity(entity).insert(if enabled {
                            Visibility::Inherited
                        } else {
                            Visibility::Hidden
                        });
                    } else {
                        // A static node hides its own renderers. Static descendants change in this loop too.
                        for renderer in instance.renderers.iter_mut().filter(|r| r.node == i) {
                            renderer.active = enabled;
                            renderer.show(commands);
                        }
                    }
                }
            }
            for renderer in &mut instance.renderers {
                let selected = car
                    .and_then(|(record, v)| v.renderer_material(record, renderer.node as u32))
                    .map(|link| &link.source)
                    .or_else(|| {
                        appearance
                            .iter()
                            .rev()
                            .find(|a| {
                                a.node == prototype.nodes[renderer.node].source
                                    && a.material.is_some()
                            })
                            .and_then(|a| a.material.as_ref())
                    });
                if selected == renderer.material_source.as_ref() {
                    continue;
                }
                if let Some(variant) = renderer
                    .variants
                    .iter_mut()
                    .find(|v| v.source.as_ref() == selected)
                {
                    std::mem::swap(&mut renderer.binding, &mut variant.binding);
                    std::mem::swap(&mut renderer.material_source, &mut variant.source);
                    std::mem::swap(&mut renderer.ticket, &mut variant.ticket);
                    renderer.lod_state = None;
                    self.lods.queue(&mut instance.lod, instance_index);
                    renderer.binding.visible.replace(renderer.entity, commands);
                    renderer.binding.apply_sort(renderer.entity, commands);
                }
            }
            for (body, handle) in prototype.bodies.iter().zip(&published.bodies) {
                let pose = physics.rapier().bodies[*handle].position();
                let rotation = Quat::from_array(pose.rotation.to_array());
                let world = Mat4::from_rotation_translation(
                    rotation,
                    Vec3::from_array(pose.translation.to_array()),
                );
                let node = &prototype.nodes[body.node as usize];
                let index = node.gltf_node as usize;
                let scale =
                    Vec3::from_array(body.pose.scale) * Vec3::from_array(instance.placement.scale);
                let world = (world * Mat4::from_scale(scale)).as_dmat4();
                let source_world = Mat4::from_rotation_translation(
                    rotation,
                    Vec3::from_array(
                        frame
                            .origin
                            .absolute(pose.translation.to_array())
                            .map(|v| v as f32),
                    ),
                ) * Mat4::from_scale(scale);
                if instance.worlds[index] != world {
                    self.lods.queue(&mut instance.lod, instance_index);
                    let delta = world * instance.worlds[index].inverse();
                    let source_changed = source_world != instance.source_worlds[index];
                    let source_delta = source_world * instance.source_worlds[index].inverse();
                    for descendant in &prototype.nodes {
                        let mut cursor = descendant.parent;
                        let mut belongs = descendant.gltf_node == node.gltf_node;
                        for _ in 0..prototype.nodes.len() {
                            let Some(parent) = cursor else { break };
                            if parent == body.node {
                                belongs = true;
                                break;
                            }
                            cursor = prototype.nodes[parent as usize].parent;
                        }
                        if belongs {
                            let target = descendant.gltf_node as usize;
                            instance.worlds[target] = delta * instance.worlds[target];
                            if source_changed {
                                instance.source_worlds[target] =
                                    source_delta * instance.source_worlds[target];
                            }
                        }
                    }
                    instance.worlds[index] = world;
                    instance.source_worlds[index] = source_world;
                    let anchor = instance.anchor_world(index, frame.origin);
                    if let Ok(mut transform) = transforms.get_mut(instance.entity(index)?) {
                        *transform = Transform::from_matrix((anchor.inverse() * world).as_mat4());
                    }
                }
            }
            let pose_world =
                |instance: &Instance, index: usize, pose: crate::vehicle::VehiclePose| {
                    Mat4::from_scale_rotation_translation(
                        instance.scales[index].0,
                        Quat::from_array(pose.rotation),
                        Vec3::from_array(pose.position),
                    )
                    .as_dmat4()
                };
            for (node_index, node) in prototype.nodes.iter().enumerate() {
                let Some(pose) = content.node_pose(node_index) else {
                    continue;
                };
                let index = node.gltf_node as usize;
                let world = pose_world(instance, index, pose);
                if instance.worlds[index] != world {
                    self.lods.queue(&mut instance.lod, instance_index);
                }
                instance.worlds[index] = world;
                let source_scale = instance.scales[index].1;
                instance.source_worlds[index] = Mat4::from_scale_rotation_translation(
                    source_scale,
                    Quat::from_array(pose.rotation),
                    Vec3::from_array(frame.origin.absolute(pose.position).map(|v| v as f32)),
                );
                let anchor = instance.anchor_world(index, frame.origin);
                if let Ok(mut transform) = transforms.get_mut(instance.entity(index)?) {
                    transform
                        .set_if_neq(Transform::from_matrix((anchor.inverse() * world).as_mat4()));
                }
            }
            for renderer in &instance.renderers {
                let node = &prototype.nodes[renderer.node];
                let position = instance.source_worlds[node.gltf_node as usize]
                    .w_axis
                    .truncate()
                    * Vec3::new(1., 1., -1.);
                if let InstanceMaterial::Impostor(handle) = &renderer.binding.visible {
                    if let Some(mut material) = materials.impostors.get_mut(handle) {
                        if material.uniform.source_position.truncate() != position {
                            material.uniform.source_position = position.extend(0.);
                        }
                    }
                }
            }
        }
        for selected_index in 0..selected {
            self.instances[self.selection.at(selected_index)].sync_pending = false;
        }
        self.selection.clear();
        Ok(())
    }
    pub fn select_lods(
        &mut self,
        reference: &ReferenceManifest,
        view: super::lod::LodView,
        commands: &mut Commands,
        materials: &mut InstanceMaterials,
        grass: &mut GrassJobs,
    ) {
        self.lods.update(
            view,
            reference.world.lod.lod_bias,
            reference.world.lod.maximum_lod as usize,
            self.origin,
            &mut self.instances,
            |instance| {
                let prototype = &reference.world.prototypes[instance.prototype];
                for renderer in &mut instance.renderers {
                    let state = instance.lod.renderer(renderer.node);
                    if renderer.lod_state == Some(state) {
                        continue;
                    }
                    renderer.lod_state = Some(state);
                    let fade = state.1;
                    renderer.show(commands);
                    let source_position = instance.source_worlds
                        [prototype.nodes[renderer.node].gltf_node as usize]
                        .w_axis
                        .truncate()
                        * Vec3::new(1., 1., -1.);
                    materials.fade(&renderer.binding, fade, source_position);
                    grass.set_fade(renderer.entity, fade, &mut materials.grass);
                }
            },
        );
        crate::world::stream_profile::count(
            crate::world::stream_profile::Stage::Lods,
            self.lods.evaluated(),
        );
    }
    pub fn request_writers(
        &self,
        stream: &mut WorldStream,
        reference: &ReferenceManifest,
        writers: &mut crate::presentation::materials::grass::writers::GrassWriters,
        meshes: &Assets<Mesh>,
    ) -> Result<(), ReferenceError> {
        for instance in &self.instances {
            if instance.retirement.started() {
                continue;
            }
            let prototype = &reference.world.prototypes[instance.prototype];
            for renderer in &instance.renderers {
                let node = &prototype.nodes[renderer.node];
                if reference
                    .world
                    .grass_displacement
                    .writers
                    .iter()
                    .any(|w| w.enabled && w.transform == node.source)
                {
                    writers.request(
                        stream,
                        renderer.entity,
                        renderer.mesh.clone(),
                        instance.worlds[node.gltf_node as usize].as_mat4(),
                        meshes,
                    )?;
                }
            }
        }
        Ok(())
    }
    pub fn request_grass(
        &self,
        stream: &mut WorldStream,
        grass: &mut GrassJobs,
        view: GrassView,
    ) -> Result<(), ReferenceError> {
        for instance in &self.instances {
            if instance.retirement.started() {
                continue;
            }
            for renderer in &instance.renderers {
                if renderer.settings.is_some() {
                    grass.request_view(stream, renderer.entity, view)?;
                }
            }
        }
        Ok(())
    }
    pub fn retire(
        &mut self,
        stream: &mut WorldStream,
        commands: &mut Commands,
        meshes: &mut Assets<Mesh>,
        materials: &mut InstanceMaterials,
        grass: &mut GrassJobs,
        writers: &mut crate::presentation::materials::grass::writers::GrassWriters,
    ) -> Result<(), ReferenceError> {
        let mut index = 0;
        while index < self.instances.len() {
            let id = &self.instances[index].placement.id;
            let frame = stream.frame();
            // A valid cached structure index proves publication without a scan.
            let published = frame
                .structures
                .get(self.instances[index].structure)
                .is_some_and(|s| &s.placement.id == id);
            let wanted = self.instances[index].session == frame.request.session
                && (published
                    || stream.pending_structures().iter().any(|p| &p.id == id)
                    || frame.structures.iter().any(|s| &s.placement.id == id));
            if self.instances[index].retirement.keep(wanted) {
                index += 1;
                continue;
            }
            if let Some(parent) = &self.instances[index].continuation {
                parent.stop_flight();
            }
            let mut grass_drained = true;
            for renderer in &self.instances[index].renderers {
                grass_drained &=
                    grass.retire(stream, renderer.entity, meshes, &mut materials.grass)?;
            }
            let parent_drained = self.instances[index]
                .continuation
                .as_ref()
                .is_none_or(|c| c.drained());
            if !self.instances[index]
                .retirement
                .drained(parent_drained, grass_drained)
            {
                index += 1;
                continue;
            }
            let mut instance = self.instances.swap_remove(index);
            self.lods.remove(&instance.lod);
            if let Some(moved) = self.instances.get_mut(index) {
                self.lods.relocate(&mut moved.lod, index);
            }
            self.bindings_dirty = true;
            commands.entity(instance.root).despawn();
            for renderer in instance.renderers {
                writers.retire(stream, renderer.entity)?;
                grass.retire(stream, renderer.entity, meshes, &mut materials.grass)?;
                materials.release(renderer.binding);
                for variant in renderer.variants {
                    materials.release(variant.binding);
                }
            }
            let mut a = stream.presentation_allocation();
            a.active_cpu = a.active_cpu.saturating_sub(instance.cpu);
            a.active_gpu = a.active_gpu.saturating_sub(instance.gpu);
            a.objects = a.objects.saturating_sub(instance.objects);
            stream.set_presentation_allocation(a)?;
            if let Some(claim) = instance.continuation.as_mut() {
                stream.release_renderer_pending(claim)?;
            }
        }
        Ok(())
    }
}
fn active_food(
    prototype: &WorldPrototypeReference,
    mut node: usize,
    appearance: &[crate::world::NodeAppearance],
    reference: &ReferenceManifest,
    item: Option<&crate::interaction::items::ItemState>,
) -> bool {
    let Some(item) = item else {
        return active(prototype, node, appearance);
    };
    let state = item.food.unwrap();
    let food = &reference.survival.foods[state.prototype_index];
    for _ in 0..prototype.nodes.len() {
        let source = &prototype.nodes[node].source;
        let part = food.parts.iter().position(|group| {
            group.iter().any(|id| {
                reference
                    .nodes
                    .iter()
                    .any(|n| n.id == *id && &n.source == source)
            })
        });
        let root = reference
            .nodes
            .iter()
            .any(|n| n.id == food.root && &n.source == source);
        let enabled = if root && item.removed {
            false
        } else if let Some(part) = part {
            !item.removed && part == state.bite as usize
        } else {
            appearance
                .iter()
                .rev()
                .find(|a| &a.node == source && a.active.is_some())
                .and_then(|a| a.active)
                .unwrap_or(prototype.nodes[node].parent.is_none() || prototype.nodes[node].active)
        };
        if !enabled {
            return false;
        }
        let Some(parent) = prototype.nodes[node].parent else {
            return true;
        };
        node = parent as usize;
    }
    false
}
/// Applies the last rotation and the last scale override of a node to its local transform.
fn override_local(
    local: Mat4,
    appearance: &[crate::world::NodeAppearance],
    node: &crate::reference::SourceId,
) -> Mat4 {
    let rotation = appearance
        .iter()
        .rev()
        .find(|a| &a.node == node && a.rotation.is_some())
        .and_then(|a| a.rotation);
    let scale = appearance
        .iter()
        .rev()
        .find(|a| &a.node == node && a.scale.is_some())
        .and_then(|a| a.scale);
    if rotation.is_none() && scale.is_none() {
        return local;
    }
    let mut transform = Transform::from_matrix(local);
    if let Some(rotation) = rotation {
        transform.rotation = Quat::from_array(rotation);
    }
    if let Some(scale) = scale {
        transform.scale = Vec3::from_array(scale);
    }
    transform.to_matrix()
}
fn active(
    prototype: &WorldPrototypeReference,
    mut node: usize,
    appearance: &[crate::world::NodeAppearance],
) -> bool {
    for _ in 0..prototype.nodes.len() {
        if !appearance
            .iter()
            .rev()
            .find(|a| a.node == prototype.nodes[node].source && a.active.is_some())
            .and_then(|a| a.active)
            .unwrap_or(prototype.nodes[node].parent.is_none() || prototype.nodes[node].active)
        {
            return false;
        }
        let Some(parent) = prototype.nodes[node].parent else {
            return true;
        };
        node = parent as usize;
    }
    false
}
fn grass_patches(mesh: &Mesh, world: Mat4) -> Result<Arc<[GrassPatch]>, ReferenceError> {
    let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        return Err(ReferenceError("Missing Grass source POSITION".into()));
    };
    let Some(VertexAttributeValues::Float32x2(uvs)) = mesh.attribute(Mesh::ATTRIBUTE_UV_0) else {
        return Err(ReferenceError("Missing Grass source UV0".into()));
    };
    let indices = mesh
        .indices()
        .ok_or_else(|| ReferenceError("Missing Grass source indices".into()))?;
    let mut result = Vec::with_capacity(indices.len() / 3);
    let mut iter = indices.iter();
    while let Some(a) = iter.next() {
        let b = iter
            .next()
            .ok_or_else(|| ReferenceError("Incomplete Grass triangle".into()))?;
        let c = iter
            .next()
            .ok_or_else(|| ReferenceError("Incomplete Grass triangle".into()))?;
        let ids = [a, b, c];
        result.push(GrassPatch {
            positions: ids.map(|i| {
                world.transform_point3(Vec3::from_array(positions[i])) * Vec3::new(1., 1., -1.)
            }),
            uvs: ids.map(|i| Vec2::from_array(uvs[i])),
        });
    }
    Ok(result.into())
}

fn grass_bounds(
    patches: &[GrassPatch],
    world: Mat4,
    radius: f32,
) -> bevy::camera::primitives::Aabb {
    let inverse = world.inverse();
    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    for point in patches.iter().flat_map(|p| p.positions) {
        let local = inverse.transform_point3(point * Vec3::new(1., 1., -1.));
        min = min.min(local);
        max = max.max(local);
    }
    if patches.is_empty() {
        min = Vec3::ZERO;
        max = Vec3::ZERO;
    }
    let padding = (inverse.x_axis.truncate().abs()
        + inverse.y_axis.truncate().abs()
        + inverse.z_axis.truncate().abs())
        * radius;
    bevy::camera::primitives::Aabb::from_min_max(min - padding, max + padding)
}

fn local_pose(pose: crate::reference::Transform) -> Transform {
    Transform {
        translation: Vec3::from_array(pose.translation),
        rotation: Quat::from_array(pose.rotation),
        scale: Vec3::from_array(pose.scale),
    }
}
fn local_root(placement: &Placement, origin: crate::world::WorldOrigin) -> DMat4 {
    let mut root = Mat4::from_scale_rotation_translation(
        Vec3::from_array(placement.scale),
        Quat::from_array(placement.pose.rotation),
        Vec3::ZERO,
    )
    .as_dmat4();
    root.w_axis = DVec3::from_array(std::array::from_fn(|axis| {
        placement.pose.position[axis] - origin.offset[axis]
    }))
    .extend(1.);
    root
}
