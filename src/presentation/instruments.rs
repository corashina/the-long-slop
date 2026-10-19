use super::materials::{MaterialCache, SourceMaterial};
use crate::reference::{MaterialLink, NodeId, ReferenceError, ReferenceManifest};
use crate::simulation::instruments::{InstrumentState, source_rotation};
use bevy::camera::visibility::NoFrustumCulling;
use bevy::ecs as bevy_ecs;
use bevy::gltf::{Gltf, GltfMaterial};
use bevy::prelude::*;
use bevy::render::render_resource::Face;
use std::collections::HashMap;
// Bevy's glTF -> StandardMaterial converter is private. This startup-only adapter
// maps the PBR fields emitted by our exporter; it does not load or clone frame assets.
fn source_material(m: &GltfMaterial) -> StandardMaterial {
    StandardMaterial {
        base_color: m.base_color,
        base_color_channel: m.base_color_channel.clone(),
        base_color_texture: m.base_color_texture.clone(),
        emissive: m.emissive,
        emissive_channel: m.emissive_channel.clone(),
        emissive_texture: m.emissive_texture.clone(),
        perceptual_roughness: m.perceptual_roughness,
        metallic: m.metallic,
        metallic_roughness_channel: m.metallic_roughness_channel.clone(),
        metallic_roughness_texture: m.metallic_roughness_texture.clone(),
        reflectance: m.reflectance,
        normal_map_channel: m.normal_map_channel.clone(),
        normal_map_texture: m.normal_map_texture.clone(),
        occlusion_channel: m.occlusion_channel.clone(),
        occlusion_texture: m.occlusion_texture.clone(),
        double_sided: m.double_sided,
        cull_mode: m.cull_mode,
        alpha_mode: m.alpha_mode.clone(),
        unlit: m.unlit,
        uv_transform: m.uv_transform,
        ..Default::default()
    }
}
struct MaterialVisual {
    index: usize,
    entities: Vec<Entity>,
    on: Handle<StandardMaterial>,
    off: Handle<StandardMaterial>,
    choices: Vec<InstrumentReceiver>,
}
struct InstrumentReceiver {
    entity: Entity,
    on: Handle<SourceMaterial>,
    off: Handle<SourceMaterial>,
}
/// The gauges of one vehicle.
#[derive(Default)]
struct Dashboard {
    needles: Vec<Entity>,
    digits: Vec<Entity>,
    lamps: Vec<MaterialVisual>,
    digit_materials: Vec<MaterialVisual>,
    last_lamps: Option<u32>,
    last_digits_lit: Option<bool>,
}
/// Dashboards follow the order of the manifest vehicles.
#[derive(Resource, Default)]
pub struct InstrumentVisuals {
    pub bound: bool,
    pub prepared: bool,
    dashboards: Vec<Dashboard>,
}
impl InstrumentVisuals {
    pub(crate) fn validate_assets(
        &self,
        materials: &Assets<SourceMaterial>,
    ) -> Result<(), crate::save::SaveError> {
        for visual in self
            .dashboards
            .iter()
            .flat_map(|dashboard| dashboard.lamps.iter().chain(&dashboard.digit_materials))
        {
            for choice in &visual.choices {
                if !materials.contains(choice.on.id()) || !materials.contains(choice.off.id()) {
                    return Err(crate::save::SaveError::MissingAssets(
                        "instrument material choice".into(),
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn bind(
        &mut self,
        reference: &ReferenceManifest,
        nodes: &HashMap<NodeId, Entity>,
        primitives: &Query<(Entity, &ChildOf), With<Mesh3d>>,
        gltfs: &[(crate::reference::AssetId, Handle<Gltf>)],
        gltf_assets: &Assets<Gltf>,
        raw_materials: &Assets<GltfMaterial>,
        standards: &mut Assets<StandardMaterial>,
        authored: &mut super::materials::SourceMaterials,
    ) -> Result<bool, ReferenceError> {
        if self.bound {
            return Ok(true);
        }
        if gltfs.iter().any(|(_, h)| gltf_assets.get(h).is_none()) {
            return Ok(false);
        }
        for link in reference.vehicles.iter().flat_map(|v| {
            v.instruments
                .lamps
                .iter()
                .flat_map(|l| [&l.on, &l.off])
                .chain(
                    v.instruments
                        .odometer
                        .digits
                        .iter()
                        .flat_map(|d| [&d.on, &d.off]),
                )
        }) {
            let (_, handle) = gltfs
                .iter()
                .find(|(id, _)| *id == link.asset)
                .ok_or_else(|| {
                    ReferenceError(format!(
                        "Missing instrument material asset {}",
                        link.asset.0
                    ))
                })?;
            let gltf = gltf_assets.get(handle).expect("checked glTF");
            let raw = gltf
                .materials
                .get(link.gltf_material as usize)
                .ok_or_else(|| {
                    ReferenceError(format!(
                        "Missing instrument material {}:{}",
                        link.asset.0, link.gltf_material
                    ))
                })?;
            if raw_materials.get(raw).is_none() {
                return Ok(false);
            }
        }
        let node = |id: NodeId| {
            nodes
                .get(&id)
                .copied()
                .ok_or_else(|| ReferenceError(format!("Missing instrument node {}", id.0)))
        };
        let mut handles = HashMap::new();
        let mut material = |r: &MaterialLink| -> Handle<StandardMaterial> {
            let key = (r.asset, r.gltf_material);
            if let Some(h) = handles.get(&key) {
                return Handle::clone(h);
            }
            let (_, g) = gltfs
                .iter()
                .find(|(id, _)| *id == r.asset)
                .expect("checked asset");
            let raw =
                &gltf_assets.get(g).expect("checked glTF").materials[r.gltf_material as usize];
            let handle = standards.add(source_material(
                raw_materials.get(raw).expect("checked material"),
            ));
            authored.standard_records.insert(handle.id(), key);
            handles.insert(key, handle.clone());
            handle
        };
        for vehicle in &reference.vehicles {
            let instruments = &vehicle.instruments;
            let mut dashboard = Dashboard {
                needles: instruments
                    .needles
                    .iter()
                    .map(|r| node(r.node))
                    .collect::<Result<_, _>>()?,
                digits: instruments
                    .odometer
                    .digits
                    .iter()
                    .map(|r| node(r.node))
                    .collect::<Result<_, _>>()?,
                ..Default::default()
            };
            for (index, lamp) in instruments.lamps.iter().enumerate() {
                if lamp.signal == "headlight2" {
                    continue;
                }
                let parent = node(lamp.node)?;
                let entities: Vec<_> = primitives
                    .iter()
                    .filter_map(|(e, p)| (p.parent() == parent).then_some(e))
                    .collect();
                if entities.is_empty() {
                    return Err(ReferenceError(format!(
                        "Missing instrument renderer {}",
                        lamp.node.0
                    )));
                }
                dashboard.lamps.push(MaterialVisual {
                    index,
                    entities,
                    on: material(&lamp.on),
                    off: material(&lamp.off),
                    choices: Vec::new(),
                });
            }
            for digit in &instruments.odometer.digits {
                let parent = node(digit.node)?;
                let entities: Vec<_> = primitives
                    .iter()
                    .filter_map(|(e, p)| (p.parent() == parent).then_some(e))
                    .collect();
                if entities.is_empty() {
                    return Err(ReferenceError(format!(
                        "Missing instrument renderer {}",
                        digit.node.0
                    )));
                }
                dashboard.digit_materials.push(MaterialVisual {
                    index: 0,
                    entities,
                    on: material(&digit.on),
                    off: material(&digit.off),
                    choices: Vec::new(),
                });
            }
            self.dashboards.push(dashboard);
        }
        self.bound = true;
        Ok(true)
    }
    /// `states` follow the order of the manifest vehicles.
    pub fn update(
        &mut self,
        reference: &ReferenceManifest,
        states: &[InstrumentState],
        transforms: &mut Query<&mut Transform, Without<super::camera::DriveCamera>>,
        materials: &mut Query<&mut MeshMaterial3d<SourceMaterial>>,
    ) {
        if !self.prepared {
            return;
        }
        for ((dashboard, vehicle), state) in self
            .dashboards
            .iter_mut()
            .zip(&reference.vehicles)
            .zip(states)
        {
            dashboard.update(&vehicle.instruments, state, transforms, materials);
        }
    }
}

impl Dashboard {
    fn update(
        &mut self,
        reference: &crate::reference::InstrumentReference,
        state: &InstrumentState,
        transforms: &mut Query<&mut Transform, Without<super::camera::DriveCamera>>,
        materials: &mut Query<&mut MeshMaterial3d<SourceMaterial>>,
    ) {
        for (i, entity) in self.needles.iter().enumerate() {
            if let Ok(mut t) = transforms.get_mut(*entity) {
                let r = &reference.needles[i];
                let mut e = r.rest;
                e[r.axis as usize] = state.needle_angles[i];
                let rotation = Quat::from_array(source_rotation(e));
                if t.rotation != rotation {
                    t.rotation = rotation;
                }
            }
        }
        for (i, entity) in self.digits.iter().enumerate() {
            if let Ok(mut t) = transforms.get_mut(*entity) {
                let r = &reference.odometer.digits[i];
                let mut e = [0.; 3];
                e[r.axis as usize] = state.digit_angles[i] * if r.inv { -1. } else { 1. };
                let rotation = Quat::from_array(source_rotation(e));
                if t.rotation != rotation {
                    t.rotation = rotation;
                }
            }
        }
        if self.last_digits_lit != Some(state.digits_lit) {
            for digit in &self.digit_materials {
                select_material(digit, state.digits_lit, materials);
            }
            self.last_digits_lit = Some(state.digits_lit);
        }
        if self.last_lamps != Some(state.lamp_bits) {
            for lamp in &self.lamps {
                select_material(lamp, state.lamp_bits & (1 << lamp.index) != 0, materials);
            }
            self.last_lamps = Some(state.lamp_bits);
        }
    }
}

fn select_material(
    visual: &MaterialVisual,
    on: bool,
    materials: &mut Query<&mut MeshMaterial3d<SourceMaterial>>,
) {
    for choice in &visual.choices {
        if let Ok(mut material) = materials.get_mut(choice.entity) {
            material.set_if_neq(MeshMaterial3d(
                if on { &choice.on } else { &choice.off }.clone(),
            ));
        }
    }
}

pub(crate) fn prepare_instruments(
    mut commands: Commands,
    mut visuals: ResMut<InstrumentVisuals>,
    mut source: ResMut<super::lighting::SourceLighting>,
    mut originals: ResMut<Assets<StandardMaterial>>,
    mut materials: ResMut<Assets<SourceMaterial>>,
    mut cache: ResMut<MaterialCache>,
    authored: Res<super::materials::SourceMaterials>,
    receivers: Query<(
        &Mesh3d,
        &MeshMaterial3d<SourceMaterial>,
        &GlobalTransform,
        &super::renderers::SourceRenderer,
        &bevy::gltf::GltfExtras,
    )>,
) {
    if !authored.textures_ready() || !visuals.bound || visuals.prepared || !source.bound {
        return;
    }
    if visuals
        .dashboards
        .iter()
        .flat_map(|d| d.lamps.iter().chain(&d.digit_materials))
        .any(|visual| {
            visual
                .entities
                .iter()
                .any(|entity| !receivers.contains(*entity))
        })
    {
        return;
    }
    let far = Transform::from_translation(Vec3::splat(1_000_000.));
    let visuals = &mut *visuals;
    for visual in visuals
        .dashboards
        .iter_mut()
        .flat_map(|d| d.lamps.iter_mut().chain(&mut d.digit_materials))
    {
        for &entity in &visual.entities {
            let (mesh, receiver, global, renderer, primitive) = receivers.get(entity).unwrap();
            let mask = materials.get(&receiver.0).unwrap().extension.receiver_mask;
            let inverted = global.affine().matrix3.determinant() < 0.;
            let mut prepare = |handle: &Handle<StandardMaterial>| {
                let key = authored.standard_records[&handle.id()];
                let mut base = super::materials::authored_caster_material(key, &authored);
                if inverted {
                    base.cull_mode = base.cull_mode.map(|face| match face {
                        Face::Front => Face::Back,
                        Face::Back => Face::Front,
                    });
                }
                let surface = if inverted {
                    cache
                        .variants
                        .entry(handle.id())
                        .or_insert_with(|| originals.add(base.clone()))
                        .clone()
                } else {
                    handle.clone()
                };
                let material = cache
                    .converted
                    .entry((surface.id(), mask))
                    .or_insert_with(|| {
                        let key = authored.standard_records[&handle.id()];
                        materials.add(super::materials::authored_material(
                            key, inverted, mask, &authored,
                        ))
                    })
                    .clone();
                let sort = authored
                    .sort_data(
                        key,
                        &renderer.0,
                        serde_json::from_str::<serde_json::Value>(&primitive.value)
                            .expect("validated primitive")["submesh"]
                            .as_u64()
                            .expect("validated submesh") as u32,
                    )
                    .map_err(|e| e.to_string())?;
                let mut warm = commands.spawn((
                    mesh.clone(),
                    MeshMaterial3d(material.clone()),
                    crate::presentation::materials::SourceMaterialWarmup,
                    renderer.clone(),
                    far,
                    Visibility::Visible,
                    bevy::light::NotShadowCaster,
                    NoFrustumCulling,
                ));
                if let Some(sort) = sort {
                    warm.insert((sort, bevy::render::batching::NoAutomaticBatching));
                }
                source.warmups.push(warm.id());
                Ok::<_, String>(material)
            };
            let on = match prepare(&visual.on) {
                Ok(value) => value,
                Err(error) => {
                    source.state = super::lighting::RenderingState::Failed(error);
                    return;
                }
            };
            let off = match prepare(&visual.off) {
                Ok(value) => value,
                Err(error) => {
                    source.state = super::lighting::RenderingState::Failed(error);
                    return;
                }
            };
            visual.choices.push(InstrumentReceiver { entity, on, off });
        }
    }
    visuals.prepared = true;
}
