//! Generated renderer materials retain source identity and per-instance LOD inputs.
use super::{
    pipelines::{PipelineTicket, WorldPipelines},
    preflight::PlanPrimitive,
};
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
pub struct RendererMaterial {
    pub visible: InstanceMaterial,
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
    ) -> Result<bool, ReferenceError> {
        self.cache
            .admit_world_asset(prototype.asset, reference, stream)?;
        for material in &prototype.deferred_materials {
            self.cache
                .admit_world_material(material, reference, stream)?;
        }
        Ok(self.cache.world_asset_ready(prototype.asset)
            && prototype
                .deferred_materials
                .iter()
                .all(|m| self.cache.world_textures.ready(&m.source)))
    }
    pub fn bind(
        &mut self,
        prototype: &WorldPrototypeReference,
        primitive: &PlanPrimitive,
        initial: Option<(crate::reference::AssetId, u32)>,
        layer: u8,
        receive: bool,
        shadow_mode: u8,
        reflected: bool,
        source_position: Vec3,
        view: GrassView,
    ) -> Result<Option<RendererMaterial>, ReferenceError> {
        let mask = 1u32 << layer;
        let mut caster = None;
        let visible = if primitive.has_source_material {
            let Some(id) = &primitive.source_material else {
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
                        .add(GrassMaterial::from_source(record, view, mask)?),
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
                    if shadow_mode != 0 {
                        caster = Some(self.casters.add(LodCasterMaterial {
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
                        }));
                    }
                    InstanceMaterial::Specular(self.specular.add(SpecularMaterial::from_source(
                        record,
                        &self.cache.world_textures,
                        mask,
                        reflected,
                    )?))
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
                caster = Some(self.casters.add(LodCasterMaterial {
                    base,
                    extension: LodCaster { fade: Vec4::X },
                }));
            }
            InstanceMaterial::Primary(self.primary.add(materials::authored_material(
                identity,
                reflected,
                mask,
                &self.cache,
            )))
        };
        Ok(Some(RendererMaterial { visible, caster }))
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
    pub fn warm(
        &self,
        stream: &mut WorldStream,
        pipelines: &mut WorldPipelines,
        mesh: &Handle<Mesh>,
        shadow: bool,
    ) -> Result<Option<PipelineTicket>, ReferenceError> {
        match self {
            Self::Primary(h) => pipelines.request(stream, mesh, h, false),
            Self::Specular(h) => pipelines.request(stream, mesh, h, false),
            Self::Impostor(h) => pipelines.request(stream, mesh, h, shadow),
            Self::Grass(h) => pipelines.request(stream, mesh, h, shadow),
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
