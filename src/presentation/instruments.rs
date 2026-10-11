use super::reference::ReferenceVisuals;
use crate::reference::{MaterialLink, NodeId, ReferenceError, ReferenceManifest};
use crate::simulation::instruments::{InstrumentState, source_rotation};
use bevy::ecs as bevy_ecs;
use bevy::gltf::{Gltf, GltfMaterial};
use bevy::prelude::*;
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
        alpha_mode: m.alpha_mode,
        unlit: m.unlit,
        uv_transform: m.uv_transform,
        ..Default::default()
    }
}
struct LampVisual {
    entities: Vec<Entity>,
    on: Handle<StandardMaterial>,
    off: Handle<StandardMaterial>,
}
#[derive(Resource, Default)]
pub struct InstrumentVisuals {
    pub bound: bool,
    needles: Vec<Entity>,
    digits: Vec<Entity>,
    lamps: Vec<LampVisual>,

    last_lamps: Option<u32>,
}
impl InstrumentVisuals {
    pub fn bind(
        &mut self,
        reference: &ReferenceManifest,
        visuals: &ReferenceVisuals,
        primitives: &Query<(Entity, &ChildOf), With<Mesh3d>>,
        gltf_assets: &Assets<Gltf>,
        raw_materials: &Assets<GltfMaterial>,
        standards: &mut Assets<StandardMaterial>,
    ) -> Result<bool, ReferenceError> {
        let (nodes, gltfs) = (&visuals.nodes, &visuals.gltfs);
        if self.bound {
            return Ok(true);
        }
        if gltfs.iter().any(|(_, h)| gltf_assets.get(h).is_none()) {
            return Ok(false);
        }
        for link in reference
            .instruments
            .lamps
            .iter()
            .flat_map(|l| [&l.on, &l.off])
            .chain(
                reference
                    .instruments
                    .odometer
                    .digits
                    .iter()
                    .flat_map(|d| [&d.on, &d.off]),
            )
        {
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
        self.needles = reference
            .instruments
            .needles
            .iter()
            .map(|r| node(r.node))
            .collect::<Result<_, _>>()?;
        self.digits = reference
            .instruments
            .odometer
            .digits
            .iter()
            .map(|r| node(r.node))
            .collect::<Result<_, _>>()?;
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
            handles.insert(key, handle.clone());
            handle
        };
        for lamp in &reference.instruments.lamps {
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
            self.lamps.push(LampVisual {
                entities,
                on: material(&lamp.on),
                off: material(&lamp.off),
            });
        }
        // Retain only used lamp variants. Digit illumination stays off with unsupported headlights.
        self.bound = true;
        Ok(true)
    }
    pub fn update(
        &mut self,
        reference: &ReferenceManifest,
        state: &InstrumentState,
        transforms: &mut Query<&mut Transform, Without<super::camera::DriveCamera>>,
        materials: &mut Query<&mut MeshMaterial3d<StandardMaterial>>,
    ) {
        if !self.bound {
            return;
        }
        for (i, entity) in self.needles.iter().enumerate() {
            if let Ok(mut t) = transforms.get_mut(*entity) {
                let r = &reference.instruments.needles[i];
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
                let r = &reference.instruments.odometer.digits[i];
                let mut e = [0.; 3];
                e[r.axis as usize] = state.digit_angles[i] * if r.inv { -1. } else { 1. };
                let rotation = Quat::from_array(source_rotation(e));
                if t.rotation != rotation {
                    t.rotation = rotation;
                }
            }
        }
        if self.last_lamps != Some(state.lamp_bits) {
            for (i, lamp) in self.lamps.iter().enumerate() {
                let handle = if state.lamp_bits & (1 << i) != 0 {
                    &lamp.on
                } else {
                    &lamp.off
                };
                for entity in &lamp.entities {
                    if let Ok(mut m) = materials.get_mut(*entity) {
                        m.0 = handle.clone();
                    }
                }
            }
            self.last_lamps = Some(state.lamp_bits);
        }
    }
}
