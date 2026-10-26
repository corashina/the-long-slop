//! Generated renderer materials retain source identity and per-instance LOD inputs.
use super::{
    pipelines::{PipelineTicket, WorldPipelines},
    shared_materials::{MaterialIdentity, SharedMaterialKey},
};
use crate::reference::prototype_plan::PlanPrimitive;
use crate::{
    presentation::materials::{
        self, RenderingState, SourceMaterial, SourceMaterials, grass::material::GrassMaterial,
        impostor::ImpostorMaterial, specular::SpecularMaterial,
    },
    reference::{ReferenceError, ReferenceManifest, WorldPrototypeReference},
    world::WorldStream,
};
use bevy::{ecs as bevy_ecs, ecs::system::SystemParam, prelude::*};
use grass_mesh::GrassView;
#[derive(Clone)]
pub enum InstanceMaterial {
    Primary(Handle<SourceMaterial>),
    Specular(Handle<SpecularMaterial>),
    Impostor(Handle<ImpostorMaterial>),
    Grass(Handle<GrassMaterial>),
}
#[derive(Clone)]
pub struct RendererMaterial {
    pub visible: InstanceMaterial,
    pub sort: Option<crate::presentation::renderer_visibility::SourceSortData>,
    shared: Option<SharedMaterialKey>,
}
#[derive(Clone)]
pub(crate) struct SharedBinding {
    visible: InstanceMaterial,
}
#[derive(SystemParam)]
pub struct InstanceMaterials<'w> {
    pub cache: ResMut<'w, SourceMaterials>,
    pub primary: ResMut<'w, Assets<SourceMaterial>>,
    pub specular: ResMut<'w, Assets<SpecularMaterial>>,
    pub impostors: ResMut<'w, Assets<ImpostorMaterial>>,
    pub grass: ResMut<'w, Assets<GrassMaterial>>,
}
impl InstanceMaterials<'_> {
    pub fn admit(
        &mut self,
        stream: &mut WorldStream,
        reference: &ReferenceManifest,
        prototype: &WorldPrototypeReference,
        placement: &crate::world::Placement,
        plan: &crate::reference::prototype_plan::PrototypePlan,
    ) -> Result<bool, ReferenceError> {
        let mut ready = true;
        for (node_index, source_node) in prototype.nodes.iter().enumerate() {
            if !source_node.renderer_enabled {
                continue;
            }
            let node = &plan.nodes[source_node.gltf_node as usize];
            for primitive in plan.primitives.iter().filter(|p| Some(p.mesh) == node.mesh) {
                let (initial, selected) = biome_material(
                    stream,
                    reference,
                    prototype,
                    placement,
                    node_index,
                    primitive,
                    selected_material(stream, reference, prototype, placement, &source_node.source),
                );
                let selected = selected.cloned();
                ready &= self.admit_binding(
                    stream,
                    reference,
                    prototype,
                    primitive,
                    initial,
                    selected.as_ref(),
                )?;
                let variants = stream
                    .lamp_materials(&prototype.source, &source_node.source)?
                    .len();
                for index in 0..variants {
                    let source = stream.lamp_materials(&prototype.source, &source_node.source)?
                        [index]
                        .clone();
                    let primary = reference
                        .rendering
                        .materials
                        .iter()
                        .find(|m| m.source == source && m.asset == prototype.asset)
                        .map(|m| (m.asset, m.gltf_material));
                    ready &= self.admit_binding(
                        stream,
                        reference,
                        prototype,
                        primitive,
                        primary,
                        Some(&source),
                    )?;
                }
            }
        }
        Ok(ready)
    }
    fn admit_binding(
        &mut self,
        stream: &mut WorldStream,
        reference: &ReferenceManifest,
        prototype: &WorldPrototypeReference,
        primitive: &PlanPrimitive,
        initial: Option<(crate::reference::AssetId, u32)>,
        selected: Option<&crate::reference::SourceId>,
    ) -> Result<bool, ReferenceError> {
        if initial.is_none() && (selected.is_some() || primitive.has_source_material) {
            let source = selected
                .or(primitive.source_material.as_ref())
                .ok_or_else(|| ReferenceError("Unresolved source null material draw".into()))?;
            let record = prototype
                .deferred_materials
                .iter()
                .find(|m| m.source == *source)
                .ok_or_else(|| ReferenceError("Missing generated material record".into()))?;
            self.cache.admit_world_material(record, reference, stream)?;
            Ok(self.cache.world_textures.ready(source))
        } else {
            let identity = initial
                .or_else(|| {
                    primitive
                        .material
                        .as_ref()
                        .map(|m| (m.asset, m.gltf_material))
                })
                .ok_or_else(|| {
                    ReferenceError("Missing generated primary material identity".into())
                })?;
            self.cache
                .admit_world_primary(identity, reference, stream)?;
            Ok(self.cache.world_primary_ready(identity))
        }
    }
    pub fn bind(
        &mut self,
        session: u64,
        prototype: &WorldPrototypeReference,
        primitive: &PlanPrimitive,
        renderer: &crate::reference::render_lighting::RendererRecord,
        initial: Option<(crate::reference::AssetId, u32)>,
        selected: Option<&crate::reference::SourceId>,
        layer: u8,
        receive: bool,
        reflected: bool,
        source_position: Vec3,
        view: GrassView,
        gpu: bool,
    ) -> Result<Option<RendererMaterial>, ReferenceError> {
        let mask = 1u32 << layer;
        let mut sort = None;
        let mut shared = None;
        let key = |material, gpu| SharedMaterialKey {
            session,
            material,
            layer,
            receive,
            reflected,
            gpu,
        };
        let visible = if initial.is_none() && (selected.is_some() || primitive.has_source_material)
        {
            let Some(id) = selected.or(primitive.source_material.as_ref()) else {
                return Err(ReferenceError(
                    "Unresolved source null material draw".into(),
                ));
            };
            let record = prototype
                .deferred_materials
                .iter()
                .find(|m| m.source == *id)
                .ok_or_else(|| ReferenceError("Missing generated material record".into()))?;
            match record.shader.name.as_str() {
                "Stix Games/Grass" => InstanceMaterial::Grass(
                    self.grass
                        .add(GrassMaterial::from_source(session, record, view, mask)?),
                ),
                "Hidden/Amplify Impostors/Octahedron Impostor"
                | "Impostors/Custom/Colored Barrels" => {
                    InstanceMaterial::Impostor(self.impostors.add(ImpostorMaterial::from_source(
                        record,
                        &self.cache.world_textures,
                        mask,
                        receive,
                        reflected,
                        source_position,
                    )?))
                }
                "Standard (Specular setup)" => {
                    let key = key(MaterialIdentity::Specular(id.clone()), false);
                    if let Some(binding) = self.cache.generated.acquire(&key) {
                        return Ok(Some(RendererMaterial {
                            visible: binding.visible,
                            sort,
                            shared: Some(key),
                        }));
                    }
                    shared = Some(key);
                    let material = SpecularMaterial::from_source(
                        record,
                        &self.cache.world_textures,
                        mask,
                        reflected,
                    )?;
                    InstanceMaterial::Specular(self.specular.add(material))
                }
                name => {
                    return Err(ReferenceError(format!(
                        "Unresolved generated shader {name}"
                    )));
                }
            }
        } else {
            let identity = initial
                .or_else(|| {
                    primitive
                        .material
                        .as_ref()
                        .map(|m| (m.asset, m.gltf_material))
                })
                .ok_or_else(|| {
                    ReferenceError("Missing generated primary material identity".into())
                })?;
            sort = self
                .cache
                .sort_data(identity, renderer, primitive.submesh)?;
            if sort.is_none() && self.cache.shareable_generated_primary(identity) {
                let key = key(MaterialIdentity::Primary(identity.0, identity.1), gpu);
                if let Some(binding) = self.cache.generated.acquire(&key) {
                    return Ok(Some(RendererMaterial {
                        visible: binding.visible,
                        sort,
                        shared: Some(key),
                    }));
                }
                shared = Some(key);
            }
            let mut material = materials::authored_material(identity, reflected, mask, &self.cache);
            if shared.as_ref().is_some_and(|key| key.gpu) {
                material.extension.use_gpu_props();
            }
            InstanceMaterial::Primary(self.primary.add(material))
        };
        match &visible {
            InstanceMaterial::Primary(h) => {
                self.primary.get_mut(h).unwrap().preparation = self.cache.preparation.clone()
            }
            InstanceMaterial::Specular(h) => {
                self.specular.get_mut(h).unwrap().preparation = self.cache.preparation.clone()
            }
            InstanceMaterial::Impostor(h) => {
                self.impostors.get_mut(h).unwrap().preparation = self.cache.preparation.clone()
            }
            InstanceMaterial::Grass(h) => {
                self.grass.get_mut(h).unwrap().preparation = self.cache.preparation.clone()
            }
        }
        if let Some(key) = &shared {
            self.cache.generated.insert(
                key.clone(),
                SharedBinding {
                    visible: visible.clone(),
                },
            );
        }
        Ok(Some(RendererMaterial {
            visible,
            sort,
            shared,
        }))
    }
    /// Isolate before pipeline tickets or entities retain these handles.
    pub fn make_private(&mut self, binding: &mut RendererMaterial) {
        let Some(key) = binding.shared.take() else {
            return;
        };
        assert!(!key.gpu, "GPU prop materials stay shared");
        let Self {
            cache,
            primary,
            specular,
            ..
        } = self;
        let private = cache.generated.make_private(&key, |shared| {
            let visible = match &shared.visible {
                InstanceMaterial::Primary(handle) => {
                    let material = primary.get(handle).unwrap().clone();
                    InstanceMaterial::Primary(primary.add(material))
                }
                InstanceMaterial::Specular(handle) => {
                    let material = specular.get(handle).unwrap().clone();
                    InstanceMaterial::Specular(specular.add(material))
                }
                _ => unreachable!("Only immutable primary and specular bindings are shared"),
            };
            SharedBinding { visible }
        });
        binding.visible = private.visible;
    }
    pub fn color(
        &mut self,
        binding: &mut RendererMaterial,
        color: [f32; 4],
    ) -> Result<(), ReferenceError> {
        self.make_private(binding);
        let color = Color::srgba(color[0], color[1], color[2], color[3]);
        match &binding.visible {
            InstanceMaterial::Primary(h) => {
                if let Some(mut m) = self.primary.get_mut(h) {
                    m.base.base_color = color;
                    m.extension.surface.set_color(color);
                }
            }
            InstanceMaterial::Specular(h) => {
                if let Some(mut m) = self.specular.get_mut(h) {
                    m.set_color(color);
                }
            }
            _ => {
                return Err(ReferenceError(
                    "Condition color requires its checked shader property".into(),
                ));
            }
        }
        Ok(())
    }
    pub fn fade(&mut self, binding: &RendererMaterial, fade: f32, source_position: Vec3) {
        if binding.shared.is_some() {
            assert_eq!(
                fade, 1.,
                "LOD bindings must be private before pipeline warmup"
            );
            return;
        }
        match &binding.visible {
            InstanceMaterial::Primary(h) => {
                if let Some(mut m) = self.primary.get_mut(h) {
                    m.extension.lod_fade = fade;
                }
            }
            InstanceMaterial::Specular(h) => {
                if let Some(mut m) = self.specular.get_mut(h) {
                    m.lod_fade = fade;
                }
            }
            InstanceMaterial::Impostor(h) => {
                if let Some(mut m) = self.impostors.get_mut(h) {
                    m.uniform.lod.x = fade;
                    m.uniform.source_position = source_position.extend(0.);
                }
            }
            InstanceMaterial::Grass(h) => {
                if let Some(mut m) = self.grass.get_mut(h) {
                    m.uniform.lod.x = fade;
                }
            }
        }
    }
    pub fn release(&mut self, binding: RendererMaterial) {
        if let Some(key) = &binding.shared {
            if !self.cache.generated.release(key) {
                return;
            }
        }
        match binding.visible {
            InstanceMaterial::Primary(h) => {
                self.primary.remove(h.id());
            }
            InstanceMaterial::Specular(h) => {
                self.specular.remove(h.id());
            }
            InstanceMaterial::Impostor(h) => {
                self.impostors.remove(h.id());
            }
            InstanceMaterial::Grass(h) => {
                self.grass.remove(h.id());
            }
        }
    }
}
impl InstanceMaterial {
    pub fn replace(&self, entity: Entity, commands: &mut Commands) {
        commands.entity(entity).remove::<(
            MeshMaterial3d<SourceMaterial>,
            MeshMaterial3d<SpecularMaterial>,
            MeshMaterial3d<ImpostorMaterial>,
            MeshMaterial3d<GrassMaterial>,
        )>();
        self.insert(entity, commands);
    }

    pub fn insert(&self, entity: Entity, commands: &mut Commands) {
        match self {
            Self::Primary(h) => {
                commands.entity(entity).insert(MeshMaterial3d(h.clone()));
            }
            Self::Specular(h) => {
                commands.entity(entity).insert(MeshMaterial3d(h.clone()));
            }
            Self::Impostor(h) => {
                commands.entity(entity).insert(MeshMaterial3d(h.clone()));
            }
            Self::Grass(h) => {
                commands.entity(entity).insert(MeshMaterial3d(h.clone()));
            }
        }
    }
    pub(crate) fn warm(
        &self,
        stream: &mut WorldStream,
        pipelines: &mut WorldPipelines,
        parent: &mut crate::world::renderer_pending::PipelineContinuation,
        mesh: &Handle<Mesh>,
        shadow: bool,
    ) -> Result<Option<PipelineTicket>, ReferenceError> {
        match self {
            Self::Primary(h) => pipelines.request_continuation(stream, parent, mesh, h, false),
            Self::Specular(h) => pipelines.request_continuation(stream, parent, mesh, h, false),
            Self::Impostor(h) => pipelines.request_continuation(stream, parent, mesh, h, shadow),
            Self::Grass(h) => pipelines.request_continuation(stream, parent, mesh, h, shadow),
        }
    }
}
pub fn ready(ticket: &Option<PipelineTicket>) -> Result<bool, ReferenceError> {
    match ticket.as_ref().map(PipelineTicket::state) {
        Some(RenderingState::Ready) => Ok(true),
        Some(RenderingState::Failed(error)) => Err(ReferenceError(error)),
        _ => Ok(false),
    }
}

impl RendererMaterial {
    /// The renderer is a GPU prop row: its shared material reads the row buffers.
    pub fn gpu_props(&self) -> bool {
        self.shared.as_ref().is_some_and(|key| key.gpu)
    }
    pub fn apply_sort(&self, entity: Entity, commands: &mut Commands) {
        if let Some(sort) = self.sort {
            commands
                .entity(entity)
                .insert((sort, bevy::render::batching::NoAutomaticBatching));
        } else {
            commands.entity(entity).remove::<(
                crate::presentation::renderer_visibility::SourceSortData,
                bevy::render::batching::NoAutomaticBatching,
            )>();
        }
    }
}

pub fn selected_material<'a>(
    stream: &'a WorldStream,
    reference: &'a ReferenceManifest,
    prototype: &WorldPrototypeReference,
    placement: &crate::world::Placement,
    node: &crate::reference::SourceId,
) -> (
    Option<(crate::reference::AssetId, u32)>,
    Option<&'a crate::reference::SourceId>,
) {
    let appearance = stream.appearance(&placement.id).iter().rev().find(|a| {
        a.node == *node
            && (a.primary.is_some()
                || a.material.is_some()
                || a.color.is_some()
                || a.color_material.is_some())
    });
    let selected = appearance.and_then(|a| a.material.as_ref()).or_else(|| stream.prop_material(placement).filter(|(renderer,_)| {
                                        let owner = reference.world.records.iter().find(|r| &r.source == *renderer)
                                            .and_then(|r| r.fields.get("m_GameObject"));
                                        let node_owner = reference.world.records.iter().find(|r| r.source == *node)
                                            .and_then(|r| r.fields.get("m_GameObject"));
                                        matches!((owner,node_owner), (Some(crate::reference::WorldValue::Link(a)),Some(crate::reference::WorldValue::Link(b))) if a == b)
                                    }).map(|(_,material)| material));
    let selected_primary = selected
        .and_then(|id| {
            reference
                .rendering
                .materials
                .iter()
                .find(|m| m.source == *id && m.asset == prototype.asset)
        })
        .map(|m| (m.asset, m.gltf_material));
    let initial = stream
        .appearance(&placement.id)
        .iter()
        .rev()
        .find(|a| a.node == *node && a.primary.is_some())
        .and_then(|a| a.primary)
        .or(selected_primary)
        .or_else(|| {
            reference
                .world
                .initial_materials
                .iter()
                .find(|m| m.node == *node)
                .and_then(|binding| {
                    reference
                        .rendering
                        .materials
                        .iter()
                        .find(|m| m.source == binding.material)
                })
                .map(|m| (m.asset, m.gltf_material))
        });
    (initial, selected)
}

/// Source `materialChangeScript` swaps the first material slot only: submesh 0.
/// It compares the material that painting and condition already selected.
pub fn biome_material<'a>(
    stream: &'a WorldStream,
    reference: &ReferenceManifest,
    prototype: &WorldPrototypeReference,
    placement: &crate::world::Placement,
    node: usize,
    primitive: &PlanPrimitive,
    chosen: (
        Option<(crate::reference::AssetId, u32)>,
        Option<&'a crate::reference::SourceId>,
    ),
) -> (
    Option<(crate::reference::AssetId, u32)>,
    Option<&'a crate::reference::SourceId>,
) {
    let (initial, selected) = chosen;
    if primitive.submesh != 0 {
        return (initial, selected);
    }
    let Some(biome) = stream.renderer_biome(prototype, placement, node) else {
        return (initial, selected);
    };
    let current = if initial.is_none() && (selected.is_some() || primitive.has_source_material) {
        selected.or(primitive.source_material.as_ref())
    } else {
        initial
            .or_else(|| {
                primitive
                    .material
                    .as_ref()
                    .map(|m| (m.asset, m.gltf_material))
            })
            .and_then(|(asset, gltf)| {
                reference
                    .rendering
                    .materials
                    .iter()
                    .find(|m| m.asset == asset && m.gltf_material == gltf)
            })
            .map(|m| &m.source)
    };
    let Some(variant) = current.and_then(|current| stream.biome_material(current, biome)) else {
        return (initial, selected);
    };
    let primary = reference
        .rendering
        .materials
        .iter()
        .find(|m| m.source == *variant && m.asset == prototype.asset)
        .map(|m| (m.asset, m.gltf_material));
    (primary, Some(variant))
}
