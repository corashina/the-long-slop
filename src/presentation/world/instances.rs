//! Generated scenes are prepared offscreen before their collision recipes publish.
use super::{
    grass_jobs::GrassJobs,
    instance_materials::{InstanceMaterial, InstanceMaterials, RendererMaterial, ready},
    lod::LodGroup,
    pipelines::{PipelineTicket, WorldPipelines},
    prototype_loader::SourceGltf,
    prototypes::SourcePrototypes,
};
use crate::{
    presentation::materials::{
        SourceShadowMode,
        grass::{GrassPatch, GrassSettings, GrassView},
    },
    reference::{ReferenceError, ReferenceManifest, WorldPrototypeReference},
    world::{Placement, WorldStream},
};
use bevy::math::{DMat4, DVec3};
use bevy::{
    camera::visibility::RenderLayers,
    ecs as bevy_ecs,
    mesh::{VertexAttributeValues, skinning::SkinnedMesh},
    prelude::*,
};
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
    settings: Option<GrassSettings>,
}
struct Instance {
    retirement: crate::world::renderer_pending::RendererRetirement,
    continuation: Option<crate::world::renderer_pending::PipelineContinuation>,
    upload: Option<Arc<crate::presentation::fixed_assets::MeshUpload>>,
    caps_bound: bool,
    placement: Placement,
    session: u64,
    root: Entity,
    nodes: Vec<Entity>,
    active_nodes: Vec<bool>,
    // Local render frame; retain double precision across distant rebases.
    worlds: Vec<DMat4>,
    // Frozen source frame for procedural color, wind, and Grass inputs only.
    source_worlds: Vec<Mat4>,
    // VehiclePose changes position and rotation. Retain the original composed scales.
    scales: Vec<(Vec3, Vec3)>,
    renderers: Vec<Renderer>,
    lods: Vec<LodGroup>,
    ready: bool,
    cpu: usize,
    gpu: usize,
    objects: usize,
    visible: bool,
    origin: [f64; 3],
}
#[derive(Resource, Default)]
pub struct SourceInstances {
    instances: Vec<Instance>,
    pending: Option<Placement>,
}
impl SourceInstances {
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
        self.pending = None;
    }

    fn rebase(&mut self, origin: crate::world::WorldOrigin, commands: &mut Commands) {
        for instance in &mut self.instances {
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
            if !stream.structure_required(&placement.id) && !required_ready {
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
                        && (stream.structure_required(&p.id) || required_ready)
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
            let prototype = reference
                .world
                .prototypes
                .iter()
                .find(|p| p.source == placement.prototype)
                .ok_or_else(|| ReferenceError("Missing generated structure prototype".into()))?;
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
                if let Some(loaded) =
                    prototypes.loaded(stream.frame().request.session, &placement.prototype)
                {
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
                                + std::mem::size_of::<(Vec3, Vec3)>())
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
                            let mut nodes = Vec::with_capacity(count);
                            let mut worlds = vec![DMat4::IDENTITY; count];
                            let mut source_worlds = vec![Mat4::IDENTITY; count];
                            let source_root = Mat4::from_scale_rotation_translation(
                                Vec3::from_array(placement.scale),
                                Quat::from_array(placement.pose.rotation),
                                Vec3::from_array(placement.pose.position.map(|v| v as f32)),
                            );
                            for node in &loaded.plan.nodes {
                                let local = if let Some(matrix) = node.matrix {
                                    Mat4::from_cols_array(&matrix)
                                } else {
                                    Mat4::from_scale_rotation_translation(
                                        Vec3::from_array(node.scale),
                                        Quat::from_array(node.rotation),
                                        Vec3::from_array(node.translation),
                                    )
                                };
                                let mut local = local;
                                if let Some(source) =
                                    prototype.nodes.iter().find(|n| n.gltf_node == node.index)
                                {
                                    if let Some(scale) = stream
                                        .appearance(&placement.id)
                                        .iter()
                                        .rev()
                                        .find(|a| a.node == source.source && a.scale.is_some())
                                        .and_then(|a| a.scale)
                                    {
                                        let mut transform = Transform::from_matrix(local);
                                        transform.scale = Vec3::from_array(scale);
                                        local = transform.to_matrix();
                                    }
                                }
                                worlds[node.index as usize] = local.as_dmat4();
                                nodes.push(
                                    commands
                                        .spawn((
                                            Transform::from_matrix(local),
                                            Visibility::Inherited,
                                        ))
                                        .id(),
                                );
                            }
                            for node in &loaded.plan.nodes {
                                let parent = loaded
                                    .plan
                                    .nodes
                                    .iter()
                                    .find(|p| p.children.contains(&node.index));
                                commands.entity(nodes[node.index as usize]).insert(ChildOf(
                                    parent.map_or(root, |p| nodes[p.index as usize]),
                                ));
                            }
                            // Parent traversal preserves glTF order without assuming topological node indices.
                            for node in &loaded.plan.nodes {
                                let mut matrix = worlds[node.index as usize].as_mat4();
                                let mut current = node.index;
                                for _ in 0..count {
                                    let Some(parent) = loaded
                                        .plan
                                        .nodes
                                        .iter()
                                        .find(|p| p.children.contains(&current))
                                    else {
                                        break;
                                    };
                                    let local = if let Some(matrix) = parent.matrix {
                                        Mat4::from_cols_array(&matrix)
                                    } else {
                                        Mat4::from_scale_rotation_translation(
                                            Vec3::from_array(parent.scale),
                                            Quat::from_array(parent.rotation),
                                            Vec3::from_array(parent.translation),
                                        )
                                    };
                                    let mut local = local;
                                    if let Some(source) =
                                        prototype.nodes.iter().find(|n| n.gltf_node == parent.index)
                                    {
                                        if let Some(scale) = stream
                                            .appearance(&placement.id)
                                            .iter()
                                            .rev()
                                            .find(|a| a.node == source.source && a.scale.is_some())
                                            .and_then(|a| a.scale)
                                        {
                                            let mut transform = Transform::from_matrix(local);
                                            transform.scale = Vec3::from_array(scale);
                                            local = transform.to_matrix();
                                        }
                                    }
                                    matrix = local * matrix;
                                    current = parent.index;
                                }
                                worlds[node.index as usize] = root_pose * matrix.as_dmat4();
                                source_worlds[node.index as usize] = source_root * matrix;
                            }
                            for (i, node) in prototype.nodes.iter().enumerate() {
                                commands.entity(nodes[node.gltf_node as usize]).insert(
                                    if active(prototype, i, stream.appearance(&placement.id)) {
                                        Visibility::Inherited
                                    } else {
                                        Visibility::Hidden
                                    },
                                );
                            }
                            let lods =
                                LodGroup::for_prototype(&reference.world, prototype, |source| {
                                    stream.lod_owner(source)
                                })?;
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
                                        super::instance_materials::selected_material(
                                            stream,
                                            reference,
                                            prototype,
                                            placement,
                                            &source_node.source,
                                        );
                                    let Some(binding) = materials.bind(
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
                                            materials.color(&binding, color)?;
                                        }
                                    }
                                    let is_grass =
                                        matches!(binding.visible, InstanceMaterial::Grass(_));
                                    let entity = commands
                                        .spawn((
                                            Transform::default(),
                                            Visibility::Inherited,
                                            ChildOf(nodes[node.index as usize]),
                                            RenderLayers::layer(source_node.layer as usize),
                                            SourceShadowMode(shadow),
                                            crate::presentation::renderers::SourceRenderer(
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
                                        let variant = materials
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
                                            .map(|&j| nodes[j])
                                            .collect();
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
                                        let settings = GrassSettings::from_source(record)?;
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
                                session: stream.frame().request.session,
                                root,
                                active_nodes: prototype
                                    .nodes
                                    .iter()
                                    .enumerate()
                                    .map(|(i, _)| {
                                        active(prototype, i, stream.appearance(&placement.id))
                                    })
                                    .collect(),
                                nodes,
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
                                lods,
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
                                self.instances.push(instance);
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
        commands: &mut Commands,
        transforms: &mut Query<&mut Transform>,
        materials: &mut InstanceMaterials,
    ) -> Result<(), ReferenceError> {
        self.rebase(stream.origin, commands);
        let frame = stream.frame();
        for instance in &mut self.instances {
            if instance.retirement.started() {
                continue;
            }
            if !stream.bindings_ready(&instance.placement.id) {
                if instance.visible {
                    commands.entity(instance.root).insert(Visibility::Hidden);
                    instance.visible = false;
                }
                continue;
            }
            let Some(published) = frame
                .structures
                .iter()
                .find(|s| s.placement.id == instance.placement.id)
            else {
                continue;
            };
            if !instance.visible {
                commands.entity(instance.root).insert(Visibility::Inherited);
                instance.visible = true;
            }
            let prototype = reference
                .world
                .prototypes
                .iter()
                .find(|p| p.source == instance.placement.prototype)
                .ok_or_else(|| ReferenceError("Missing visible source prototype".into()))?;
            if !instance.caps_bound {
                for cap in stream.generated_cap_visuals(&instance.placement.id) {
                    crate::presentation::reference::bind_generated_cap(
                        commands,
                        cap,
                        prototype,
                        &instance.nodes,
                    );
                }
                instance.caps_bound = true;
            }
            let appearance = stream.appearance(&instance.placement.id);
            let food = items
                .generated_bindings()
                .find(|(id, item)| **id == instance.placement.id && item.food.is_some())
                .map(|(_, item)| item);
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
            for (i, node) in prototype.nodes.iter().enumerate() {
                if let Some(pose) = stream.lamp_button_pose(&instance.placement.id, &node.source) {
                    if let Ok(mut transform) =
                        transforms.get_mut(instance.nodes[node.gltf_node as usize])
                    {
                        let value = Transform {
                            translation: Vec3::from_array(pose.translation),
                            rotation: Quat::from_array(pose.rotation),
                            scale: Vec3::from_array(pose.scale),
                        };
                        transform.set_if_neq(value);
                    }
                }
                let enabled = active_food(prototype, i, appearance, reference, food);
                if enabled != instance.active_nodes[i] {
                    instance.active_nodes[i] = enabled;
                    commands
                        .entity(instance.nodes[node.gltf_node as usize])
                        .insert(if enabled {
                            Visibility::Inherited
                        } else {
                            Visibility::Hidden
                        });
                }
            }
            for renderer in &mut instance.renderers {
                let selected = appearance
                    .iter()
                    .rev()
                    .find(|a| {
                        a.node == prototype.nodes[renderer.node].source && a.material.is_some()
                    })
                    .and_then(|a| a.material.as_ref());
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
                    let parent = if let Some(parent) = node.parent {
                        instance.worlds[prototype.nodes[parent as usize].gltf_node as usize]
                    } else {
                        local_root(&instance.placement, frame.origin)
                    };
                    if let Ok(mut transform) = transforms.get_mut(instance.nodes[index]) {
                        *transform = Transform::from_matrix((parent.inverse() * world).as_mat4());
                    }
                }
            }
            for (node_index, node) in prototype.nodes.iter().enumerate() {
                let Some(pose) = stream.generated_node_pose(&instance.placement.id, node_index)
                else {
                    continue;
                };
                let index = node.gltf_node as usize;
                let scale = instance.scales[index].0;
                let world = Mat4::from_scale_rotation_translation(
                    scale,
                    Quat::from_array(pose.rotation),
                    Vec3::from_array(pose.position),
                )
                .as_dmat4();
                instance.worlds[index] = world;
                let source_scale = instance.scales[index].1;
                instance.source_worlds[index] = Mat4::from_scale_rotation_translation(
                    source_scale,
                    Quat::from_array(pose.rotation),
                    Vec3::from_array(frame.origin.absolute(pose.position).map(|v| v as f32)),
                );
                let parent = if let Some(parent) = node.parent {
                    instance.worlds[prototype.nodes[parent as usize].gltf_node as usize]
                } else {
                    local_root(&instance.placement, frame.origin)
                };
                if let Ok(mut transform) = transforms.get_mut(instance.nodes[index]) {
                    transform
                        .set_if_neq(Transform::from_matrix((parent.inverse() * world).as_mat4()));
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
        for instance in &mut self.instances {
            if instance.retirement.started() {
                continue;
            }
            let prototype = reference
                .world
                .prototypes
                .iter()
                .find(|p| p.source == instance.placement.prototype)
                .expect("admitted prototype");
            for renderer in &mut instance.renderers {
                let mut visible = true;
                let mut fade = 1.0;
                for group in &instance.lods {
                    let Some(level) = group
                        .levels
                        .iter()
                        .position(|l| l.nodes.contains(&renderer.node))
                    else {
                        continue;
                    };
                    let node = prototype.nodes[group.node].gltf_node as usize;
                    let selected = group.select(
                        instance.worlds[node].as_mat4(),
                        view,
                        reference.world.lod.lod_bias,
                        reference.world.lod.maximum_lod as usize,
                    );
                    match selected {
                        Some(value) if level == value.first => {
                            fade = value.fade;
                        }
                        Some(value) if value.transition && level == value.first + 1 => {
                            fade = value.fade - 1.0;
                        }
                        _ => {
                            visible = false;
                        }
                    }
                }
                let state = (visible, fade);
                if renderer.lod_state == Some(state) {
                    continue;
                }
                renderer.lod_state = Some(state);
                commands.entity(renderer.entity).insert(if visible {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                });
                let source_position = instance.source_worlds
                    [prototype.nodes[renderer.node].gltf_node as usize]
                    .w_axis
                    .truncate()
                    * Vec3::new(1., 1., -1.);
                materials.fade(&renderer.binding, fade, source_position);
                grass.set_fade(renderer.entity, fade, &mut materials.grass);
            }
        }
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
            let prototype = reference
                .world
                .prototypes
                .iter()
                .find(|p| p.source == instance.placement.prototype)
                .expect("admitted prototype");
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
            let wanted = self.instances[index].session == stream.frame().request.session
                && (stream.pending_structures().iter().any(|p| &p.id == id)
                    || stream
                        .frame()
                        .structures
                        .iter()
                        .any(|s| &s.placement.id == id));
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
