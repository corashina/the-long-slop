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
        SourceCaster, SourceShadowMode,
        grass::{GrassPatch, GrassSettings, GrassView},
    },
    reference::{ReferenceError, ReferenceManifest, WorldPrototypeReference},
    world::{Placement, WorldFrame, WorldStream},
};
use bevy::{
    camera::visibility::RenderLayers,
    ecs as bevy_ecs,
    mesh::{VertexAttributeValues, skinning::SkinnedMesh},
    prelude::*,
};
use std::sync::Arc;
struct Renderer {
    node: usize,
    entity: Entity,
    mesh: Handle<Mesh>,
    binding: RendererMaterial,
    tickets: [Option<PipelineTicket>; 2],
    shadow: bool,
    lod_state: Option<(bool, f32)>,
    settings: Option<GrassSettings>,
}
struct Instance {
    placement: Placement,
    session: u64,
    root: Entity,
    nodes: Vec<Entity>,
    worlds: Vec<Mat4>,
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
    pub fn prepare(
        &mut self,
        stream: &mut WorldStream,
        reference: &ReferenceManifest,
        prototypes: &mut SourcePrototypes,
        assets: &Assets<SourceGltf>,
        meshes: &mut Assets<Mesh>,
        materials: &mut InstanceMaterials,
        grass: &mut GrassJobs,
        pipelines: &mut WorldPipelines,
        commands: &mut Commands,
        view: GrassView,
    ) -> Result<(), ReferenceError> {
        if self.pending.as_ref().is_some_and(|p| {
            !stream
                .pending_structures()
                .iter()
                .any(|wanted| wanted.id == p.id)
        }) {
            self.pending = None;
        }
        if self.pending.is_none() {
            self.pending = stream
                .pending_structures()
                .iter()
                .find(|p| !self.instances.iter().any(|i| i.placement.id == p.id))
                .cloned();
        }
        if let Some(placement) = self.pending.as_ref() {
            let prototype = reference
                .world
                .prototypes
                .iter()
                .find(|p| p.source == placement.prototype)
                .ok_or_else(|| ReferenceError("Missing generated structure prototype".into()))?;
            prototypes.request(stream, &placement.prototype)?;
            let admitted = materials.admit(stream, reference, prototype)?;
            if admitted {
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
                        let mut cpu = count * 2048 + primitives * 8192;
                        let gpu = primitives * 2048;
                        let objects = count + primitives;
                        let mut a = stream.presentation_allocation();
                        a.active_cpu += cpu;
                        a.active_gpu += gpu;
                        a.objects += objects;
                        a.pending += 1;
                        stream.set_presentation_allocation(a)?;
                        let root_pose = Mat4::from_scale_rotation_translation(
                            Vec3::from_array(placement.scale),
                            Quat::from_array(placement.pose.rotation),
                            Vec3::from_array(placement.pose.position.map(|v| v as f32)),
                        );
                        let root = commands
                            .spawn((
                                Transform::from_matrix(
                                    Mat4::from_translation(-Vec3::from_array(
                                        stream.origin.offset.map(|v| v as f32),
                                    )) * root_pose,
                                ),
                                Visibility::Hidden,
                            ))
                            .id();
                        let mut owned_bindings = Vec::with_capacity(primitives);
                        let construction = (|| -> Result<Instance, ReferenceError> {
                            let mut nodes = Vec::with_capacity(count);
                            let mut worlds = vec![Mat4::IDENTITY; count];
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
                                worlds[node.index as usize] = local;
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
                                let mut matrix = worlds[node.index as usize];
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
                                    matrix = local * matrix;
                                    current = parent.index;
                                }
                                worlds[node.index as usize] = root_pose * matrix;
                            }
                            let lods = LodGroup::for_prototype(&reference.world, prototype)?;
                            let mut renderers = Vec::with_capacity(primitives);
                            for (source_index, source_node) in prototype.nodes.iter().enumerate() {
                                if !source_node.renderer_enabled || !active(prototype, source_index)
                                {
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
                                        .position(|&(m, p)| {
                                            m == primitive.mesh as usize
                                                && p == primitive.primitive as usize
                                        })
                                        .ok_or_else(|| {
                                            ReferenceError(
                                                "Missing generated primitive mesh".into(),
                                            )
                                        })?;
                                    let mesh = asset.meshes[index].clone();
                                    let world = worlds[node.index as usize];
                                    let source_position =
                                        world.w_axis.truncate() * Vec3::new(1., 1., -1.);
                                    let initial = reference
                                        .world
                                        .initial_materials
                                        .iter()
                                        .find(|m| m.node == source_node.source)
                                        .and_then(|binding| {
                                            reference
                                                .rendering
                                                .materials
                                                .iter()
                                                .find(|m| m.source == binding.material)
                                        })
                                        .map(|m| (m.asset, m.gltf_material));
                                    let Some(binding) = materials.bind(
                                        prototype,
                                        primitive,
                                        initial,
                                        source_node.layer,
                                        receive,
                                        shadow,
                                        world.determinant() < 0.,
                                        source_position,
                                        view,
                                    )?
                                    else {
                                        continue;
                                    };
                                    let is_grass =
                                        matches!(binding.visible, InstanceMaterial::Grass(_));
                                    let entity = commands
                                        .spawn((
                                            Transform::default(),
                                            Visibility::Inherited,
                                            ChildOf(nodes[node.index as usize]),
                                            RenderLayers::layer(source_node.layer as usize),
                                            SourceShadowMode(shadow),
                                            bevy::light::NotShadowCaster,
                                        ))
                                        .id();
                                    owned_bindings.push((entity, binding.clone()));
                                    if !receive {
                                        commands
                                            .entity(entity)
                                            .insert(bevy::light::NotShadowReceiver);
                                    }
                                    if !is_grass {
                                        commands.entity(entity).insert(Mesh3d(mesh.clone()));
                                        binding.visible.insert(entity, commands);
                                    }
                                    let mut skin_binding = None;
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
                                        commands.entity(entity).insert(binding.clone());
                                        skin_binding = Some(binding);
                                    }
                                    if let Some(caster) = &binding.caster {
                                        let caster = commands
                                            .spawn((
                                                SourceCaster,
                                                Mesh3d(mesh.clone()),
                                                MeshMaterial3d(caster.clone()),
                                                Transform::default(),
                                                Visibility::Inherited,
                                                ChildOf(entity),
                                                RenderLayers::layer(
                                                    source_node.layer as usize + 32,
                                                ),
                                            ))
                                            .id();
                                        if let Some(skin) = &skin_binding {
                                            commands.entity(caster).insert(skin.clone());
                                        }
                                    } else if shadow != 0 && !is_grass {
                                        let caster = commands
                                            .spawn((
                                                SourceCaster,
                                                Mesh3d(mesh.clone()),
                                                Transform::default(),
                                                Visibility::Inherited,
                                                ChildOf(entity),
                                                RenderLayers::layer(
                                                    source_node.layer as usize + 32,
                                                ),
                                            ))
                                            .id();
                                        binding.visible.insert(caster, commands);
                                        if let Some(skin) = &skin_binding {
                                            commands.entity(caster).insert(skin.clone());
                                        }
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
                                        let patches = grass_patches(mesh_data, world)?;
                                        let settings = GrassSettings::from_source(record)?;
                                        if let InstanceMaterial::Grass(handle) = &binding.visible {
                                            let caster = (shadow != 0).then(|| {
                                                commands
                                                    .spawn((
                                                        SourceCaster,
                                                        Transform::default(),
                                                        Visibility::Inherited,
                                                        ChildOf(entity),
                                                        RenderLayers::layer(
                                                            source_node.layer as usize + 32,
                                                        ),
                                                    ))
                                                    .id()
                                            });
                                            let bounds = grass_bounds(
                                                &patches,
                                                world,
                                                materials
                                                    .grass
                                                    .get(handle)
                                                    .expect("bound Grass material")
                                                    .displacement_radius(),
                                            );
                                            grass.request(
                                                stream,
                                                entity,
                                                Arc::clone(&patches),
                                                settings,
                                                handle.clone(),
                                                view,
                                                shadow != 0,
                                                caster,
                                                bounds,
                                            )?;
                                        }
                                        (Some(patches), Some(settings))
                                    } else {
                                        (None, None)
                                    };
                                    renderers.push(Renderer {
                                        node: source_index,
                                        entity,
                                        mesh,
                                        binding,
                                        tickets: [None, None],
                                        lod_state: None,
                                        shadow: shadow != 0,
                                        settings,
                                    });
                                }
                            }
                            Ok(Instance {
                                placement: placement.clone(),
                                session: stream.frame().request.session,
                                root,
                                nodes,
                                worlds,
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
                            Ok(instance) => self.instances.push(instance),
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
                                a.pending = a.pending.saturating_sub(1);
                                stream.set_presentation_allocation(a)?;
                                return Err(error);
                            }
                        }
                        self.pending = None;
                    }
                }
            }
        }
        for instance in &mut self.instances {
            if instance.ready
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
                if renderer.tickets[0].is_none() {
                    renderer.tickets[0] = renderer.binding.visible.warm(
                        stream,
                        pipelines,
                        &renderer.mesh,
                        renderer.shadow,
                    )?;
                }
                complete &= ready(&renderer.tickets[0])?;
                if let Some(caster) = &renderer.binding.caster {
                    if renderer.tickets[1].is_none() {
                        renderer.tickets[1] =
                            pipelines.request(stream, &renderer.mesh, caster, true)?;
                    }
                    complete &= ready(&renderer.tickets[1])?;
                }
            }
            if complete {
                stream.bind_prototype(&instance.placement.prototype)?;
                stream.bind_structure_recipe(&instance.placement.prototype)?;
                stream.accept_structure_renderer(stream.frame().request, &instance.placement.id)?;
                instance.ready = true;
                let mut a = stream.presentation_allocation();
                a.pending -= 1;
                stream.set_presentation_allocation(a)?;
            }
        }
        Ok(())
    }
    pub fn sync(
        &mut self,
        frame: &WorldFrame,
        reference: &ReferenceManifest,
        physics: &crate::vehicle::PhysicsWorld,
        commands: &mut Commands,
        transforms: &mut Query<&mut Transform>,
        materials: &mut InstanceMaterials,
    ) -> Result<(), ReferenceError> {
        for instance in &mut self.instances {
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
            if instance.origin != frame.origin.offset {
                if let Ok(mut root) = transforms.get_mut(instance.root) {
                    root.translation =
                        Vec3::from_array(frame.origin.local(instance.placement.pose.position));
                }
                instance.origin = frame.origin.offset;
            }
            let prototype = reference
                .world
                .prototypes
                .iter()
                .find(|p| p.source == instance.placement.prototype)
                .ok_or_else(|| ReferenceError("Missing visible source prototype".into()))?;
            for (body, handle) in prototype.bodies.iter().zip(&published.bodies) {
                let pose = physics.rapier().bodies[*handle].position();
                let rotation = Quat::from_array(pose.rotation.to_array());
                let absolute = frame.origin.absolute(pose.translation.to_array());
                let world = Mat4::from_rotation_translation(
                    rotation,
                    Vec3::from_array(absolute.map(|v| v as f32)),
                );
                let node = &prototype.nodes[body.node as usize];
                let index = node.gltf_node as usize;
                let scale =
                    Vec3::from_array(body.pose.scale) * Vec3::from_array(instance.placement.scale);
                let world = world * Mat4::from_scale(scale);
                if instance.worlds[index] != world {
                    let delta = world * instance.worlds[index].inverse();
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
                        }
                    }
                    let parent = if let Some(parent) = node.parent {
                        instance.worlds[prototype.nodes[parent as usize].gltf_node as usize]
                    } else {
                        Mat4::from_scale_rotation_translation(
                            Vec3::from_array(instance.placement.scale),
                            Quat::from_array(instance.placement.pose.rotation),
                            Vec3::from_array(instance.placement.pose.position.map(|v| v as f32)),
                        )
                    };
                    if let Ok(mut transform) = transforms.get_mut(instance.nodes[index]) {
                        *transform = Transform::from_matrix(parent.inverse() * world);
                    }
                }
            }
            for renderer in &instance.renderers {
                let node = &prototype.nodes[renderer.node];
                let position = instance.worlds[node.gltf_node as usize].w_axis.truncate()
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
                        instance.worlds[node],
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
                let source_position = instance.worlds
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
        let offset =
            Mat4::from_translation(-Vec3::from_array(stream.origin.offset.map(|v| v as f32)));
        for instance in &self.instances {
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
                        offset * instance.worlds[node.gltf_node as usize],
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
            if self.instances[index].session == stream.frame().request.session
                && (stream.pending_structures().iter().any(|p| &p.id == id)
                    || stream
                        .frame()
                        .structures
                        .iter()
                        .any(|s| &s.placement.id == id))
            {
                index += 1;
                continue;
            }
            let instance = self.instances.swap_remove(index);
            commands.entity(instance.root).despawn();
            for renderer in instance.renderers {
                writers.retire(stream, renderer.entity)?;
                grass.retire(stream, renderer.entity, meshes, &mut materials.grass)?;
                materials.release(renderer.binding);
            }
            let mut a = stream.presentation_allocation();
            a.active_cpu = a.active_cpu.saturating_sub(instance.cpu);
            a.active_gpu = a.active_gpu.saturating_sub(instance.gpu);
            a.objects = a.objects.saturating_sub(instance.objects);
            a.pending = a.pending.saturating_sub(usize::from(!instance.ready));
            stream.set_presentation_allocation(a)?;
        }
        Ok(())
    }
}
fn active(prototype: &WorldPrototypeReference, mut node: usize) -> bool {
    for _ in 0..prototype.nodes.len() {
        if !prototype.nodes[node].active {
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
