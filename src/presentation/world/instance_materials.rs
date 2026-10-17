//! Generated renderer materials retain source identity and per-instance LOD inputs.
use super::pipelines::{PipelineTicket, WorldPipelines};
use crate::reference::prototype_plan::PlanPrimitive;
use crate::{
    presentation::materials::{
        self, RenderingState, SourceMaterial, SourceMaterials,
        grass::{GrassView, material::GrassMaterial},
        impostor::ImpostorMaterial,
        lod::{LodCaster, LodCasterMaterial},
        specular::SpecularMaterial,
    },
    reference::{ReferenceError, ReferenceManifest, WorldPrototypeReference},
    world::WorldStream,
};
use bevy::{ecs as bevy_ecs, ecs::system::SystemParam, prelude::*};
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
    pub sort: Option<crate::presentation::transparent::SourceSortData>,
    pub caster: Option<Handle<LodCasterMaterial>>,
}
#[derive(SystemParam)]
pub struct InstanceMaterials<'w> {
    pub cache: ResMut<'w, SourceMaterials>,
    pub primary: ResMut<'w, Assets<SourceMaterial>>,
    pub specular: ResMut<'w, Assets<SpecularMaterial>>,
    pub impostors: ResMut<'w, Assets<ImpostorMaterial>>,
    pub grass: ResMut<'w, Assets<GrassMaterial>>,
    pub casters: ResMut<'w, Assets<LodCasterMaterial>>,
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
        for source_node in &prototype.nodes {
            if !source_node.renderer_enabled {
                continue;
            }
            let node = &plan.nodes[source_node.gltf_node as usize];
            for primitive in plan.primitives.iter().filter(|p| Some(p.mesh) == node.mesh) {
                let (initial, selected) =
                    selected_material(stream, reference, prototype, placement, &source_node.source);
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
        shadow_mode: u8,
        reflected: bool,
        source_position: Vec3,
        view: GrassView,
    ) -> Result<Option<RendererMaterial>, ReferenceError> {
        let mask = 1u32 << layer;
        let mut caster = None;
        let mut sort = None;
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
                    let material = SpecularMaterial::from_source(
                        record,
                        &self.cache.world_textures,
                        mask,
                        reflected,
                    )?;
                    if shadow_mode != 0 {
                        caster = Some(self.casters.add(LodCasterMaterial::new(
                            bevy::pbr::ExtendedMaterial {
                                base: StandardMaterial {
                                    cull_mode: if shadow_mode == 2 {
                                        None
                                    } else {
                                        Some(if reflected {
                                            bevy::render::render_resource::Face::Front
                                        } else {
                                            bevy::render::render_resource::Face::Back
                                        })
                                    },
                                    ..default()
                                },
                                extension: LodCaster { fade: Vec4::X },
                            },
                            self.cache.preparation.clone(),
                        )));
                    }
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
            if shadow_mode != 0 {
                let mut base = materials::authored_caster_material(identity, &self.cache);
                if shadow_mode == 2 {
                    base.cull_mode = None;
                } else if reflected {
                    base.cull_mode = base.cull_mode.map(|face| match face {
                        bevy::render::render_resource::Face::Back => {
                            bevy::render::render_resource::Face::Front
                        }
                        _ => bevy::render::render_resource::Face::Back,
                    });
                }
                caster = Some(self.casters.add(LodCasterMaterial::new(
                    bevy::pbr::ExtendedMaterial {
                        base,
                        extension: LodCaster { fade: Vec4::X },
                    },
                    self.cache.preparation.clone(),
                )));
            }
            InstanceMaterial::Primary(self.primary.add(materials::authored_material(
                identity,
                reflected,
                mask,
                &self.cache,
            )))
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
        Ok(Some(RendererMaterial {
            visible,
            sort,
            caster,
        }))
    }
    pub fn color(
        &mut self,
        binding: &RendererMaterial,
        color: [f32; 4],
    ) -> Result<(), ReferenceError> {
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
        if let Some(h) = &binding.caster {
            if let Some(mut m) = self.casters.get_mut(h) {
                m.extension.fade.x = fade;
            }
        }
    }
    pub fn release(&mut self, binding: RendererMaterial) {
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
        if let Some(h) = binding.caster {
            self.casters.remove(h.id());
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
    pub fn apply_sort(&self, entity: Entity, commands: &mut Commands) {
        if let Some(sort) = self.sort {
            commands
                .entity(entity)
                .insert((sort, bevy::render::batching::NoAutomaticBatching));
        } else {
            commands.entity(entity).remove::<(
                crate::presentation::transparent::SourceSortData,
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
