//! Road and terrain material admission through the shared source cache.
use super::{SourceWorldVisuals, SurfaceMaterial, prototypes::SourcePrototypes};
use crate::{
    presentation::materials::{
        SourceMaterial, SourceMaterials, authored_caster_material, authored_material,
        terrain::TerrainMaterial,
    },
    reference::{ReferenceError, ReferenceManifest, WorldValue},
    world::WorldStream,
};
use bevy::prelude::*;
impl SourceWorldVisuals {
    pub fn prepare_bindings(
        &mut self,
        stream: &mut WorldStream,
        reference: &ReferenceManifest,
        prototypes: &mut SourcePrototypes,
        cache: &mut SourceMaterials,
        materials: &mut Assets<SourceMaterial>,
        terrain: &mut Assets<TerrainMaterial>,
        casters: &mut Assets<StandardMaterial>,
    ) -> Result<(), ReferenceError> {
        for source in &reference.world.road.prototypes {
            if self.road_materials.contains_key(source) {
                continue;
            }
            prototypes.request(stream, source)?;
            let prototype = reference
                .world
                .prototypes
                .iter()
                .find(|p| p.source == *source)
                .ok_or_else(|| ReferenceError("Missing road prototype".into()))?;
            cache.admit_world_asset(prototype.asset, reference, stream)?;
            let Some(loaded) = prototypes.loaded(stream.frame().request.session, source) else {
                continue;
            };
            if !cache.world_asset_ready(prototype.asset) {
                continue;
            }
            let mut bindings = Vec::with_capacity(
                prototype
                    .skins
                    .iter()
                    .map(|skin| skin.triangles.len())
                    .sum(),
            );
            for skin in &prototype.skins {
                let node =
                    &loaded.plan.nodes[prototype.nodes[skin.node as usize].gltf_node as usize];
                let extras = node
                    .extras
                    .as_ref()
                    .ok_or_else(|| ReferenceError("Missing road renderer metadata".into()))?;
                let layer = extras
                    .layer
                    .ok_or_else(|| ReferenceError("Missing road renderer layer".into()))?;
                let shadow_mode = extras
                    .cast_shadows
                    .ok_or_else(|| ReferenceError("Missing road shadow mode".into()))?;
                let receive_shadows = extras
                    .receive_shadows
                    .ok_or_else(|| ReferenceError("Missing road receiver state".into()))?;
                let before = bindings.len();
                for primitive in loaded
                    .plan
                    .primitives
                    .iter()
                    .filter(|p| Some(p.mesh) == node.mesh)
                {
                    let identity = primitive
                        .material
                        .as_ref()
                        .filter(|_| !primitive.has_source_material)
                        .ok_or_else(|| {
                            ReferenceError("Unresolved source road material family".into())
                        })?;
                    let key = (
                        identity.asset,
                        identity.gltf_material,
                        false,
                        if receive_shadows { 1u32 << layer } else { 0 },
                    );
                    let material = if let Some(handle) = cache.handles.get(&key) {
                        handle.clone()
                    } else {
                        let handle =
                            materials.add(authored_material((key.0, key.1), key.2, key.3, cache));
                        cache.handles.insert(key, handle.clone());
                        handle
                    };
                    let caster = (shadow_mode != 0)
                        .then(|| casters.add(authored_caster_material((key.0, key.1), cache)));
                    bindings.push(SurfaceMaterial {
                        renderer: extras
                            .source_renderer
                            .clone()
                            .ok_or_else(|| ReferenceError("Missing road renderer record".into()))?,
                        material,
                        caster,
                        layer,
                        shadow_mode,
                        receive_shadows,
                    });
                }
                if bindings.len() - before != skin.triangles.len() {
                    return Err(ReferenceError("Road material/submesh order differs".into()));
                }
            }
            self.bind_road(source.clone(), bindings)?;
            stream.bind_prototype(source)?;
        }
        if self.road_materials.len() != reference.world.road.prototypes.len() {
            return Ok(());
        }
        for (tier, record) in reference.world.terrain.materials.iter().enumerate() {
            if self.terrain[tier].is_some() {
                continue;
            }
            stream.reserve_prototype(
                record.source.clone(),
                std::mem::size_of::<TerrainMaterial>() + 256,
                0,
            )?;
            cache.admit_world_material(record, reference, stream)?;
            if !cache.world_textures.ready(&record.source) {
                continue;
            }
            let (layer, renderer) = terrain_renderer(reference, tier)?;
            let shadow_mode = renderer.cast_shadows;
            let receive = renderer.receive_shadows;
            self.terrain_records[tier] = Some(renderer);
            let material = terrain.add(TerrainMaterial::from_source(
                record,
                &cache.world_textures,
                1u32 << layer,
                receive,
            )?);
            terrain.get_mut(&material).unwrap().preparation = cache.preparation.clone();
            let caster = (shadow_mode != 0).then(|| {
                casters.add(StandardMaterial {
                    cull_mode: None,
                    ..default()
                })
            });
            self.bind_terrain(tier, material, layer, shadow_mode, caster);
            stream.bind_prototype(&record.source)?;
        }
        Ok(())
    }
}
fn terrain_renderer(
    reference: &ReferenceManifest,
    tier: usize,
) -> Result<(u8, crate::reference::render_lighting::RendererRecord), ReferenceError> {
    let records = &reference.world.records;
    let transform = records
        .iter()
        .find(|r| r.source == reference.world.terrain.prefabs[tier])
        .ok_or_else(|| ReferenceError("Missing terrain transform".into()))?;
    let owner = match transform.fields.get("m_GameObject") {
        Some(WorldValue::Link(id)) => id,
        _ => return Err(ReferenceError("Missing terrain owner".into())),
    };
    let object = records
        .iter()
        .find(|r| r.source == *owner)
        .ok_or_else(|| ReferenceError("Missing terrain GameObject".into()))?;
    let layer = match object.fields.get("m_Layer") {
        Some(WorldValue::Integer(layer)) => *layer as u8,
        _ => return Err(ReferenceError("Missing terrain layer".into())),
    };
    let renderer = records
        .iter()
        .find(|r| {
            r.class_name == "MeshRenderer"
                && matches!(r.fields.get("m_GameObject"),Some(WorldValue::Link(id))if id==owner)
        })
        .ok_or_else(|| ReferenceError("Missing terrain renderer".into()))?;
    let shadows = match renderer.fields.get("m_CastShadows") {
        Some(WorldValue::Integer(value)) => *value as u8,
        _ => return Err(ReferenceError("Missing terrain shadow state".into())),
    };
    let receive = match renderer.fields.get("m_ReceiveShadows") {
        Some(WorldValue::Bool(value)) => *value,
        Some(WorldValue::Integer(value)) => *value != 0,
        _ => return Err(ReferenceError("Missing terrain receiver state".into())),
    };
    let integer = |name: &str| match renderer.fields.get(name) {
        Some(WorldValue::Integer(value)) => Ok(*value),
        _ => Err(ReferenceError(format!("Missing terrain renderer {name}"))),
    };
    if integer("m_SortingLayerID")? != 0 {
        return Err(ReferenceError(
            "Unsupported terrain sorting layer ID".into(),
        ));
    }
    let record = crate::reference::render_lighting::RendererRecord {
        source: renderer.source.clone(),
        bounds_center: [0.; 3],
        bounds_extents: [0.; 3],
        cast_shadows: shadows,
        receive_shadows: receive,
        reflection_probe_usage: u8::try_from(integer("m_ReflectionProbeUsage")?)
            .map_err(|_| ReferenceError("Terrain probe usage".into()))?,
        motion_vectors: u8::try_from(integer("m_MotionVectors")?)
            .map_err(|_| ReferenceError("Terrain motion mode".into()))?,
        sorting_layer: i16::try_from(integer("m_SortingLayer")?)
            .map_err(|_| ReferenceError("Terrain sorting layer".into()))?,
        sorting_order: i16::try_from(integer("m_SortingOrder")?)
            .map_err(|_| ReferenceError("Terrain sorting order".into()))?,
    };
    record.validate()?;
    Ok((layer, record))
}
