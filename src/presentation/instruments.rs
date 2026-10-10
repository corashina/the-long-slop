use crate::reference::{MaterialLink, NodeId, ReferenceError, ReferenceManifest};
use crate::simulation::instruments::{InstrumentState, source_rotation};
use bevy::asset::LoadState;
use bevy::ecs as bevy_ecs;
use bevy::prelude::*;
use std::collections::HashMap;
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
    materials: Vec<(String, Handle<StandardMaterial>)>,
    last_lamps: Option<u32>,
}
impl InstrumentVisuals {
    pub fn bind(
        &mut self,
        reference: &ReferenceManifest,
        nodes: &HashMap<NodeId, Entity>,
        primitives: &Query<(Entity, &ChildOf), With<Mesh3d>>,
        server: &AssetServer,
    ) -> Result<(), ReferenceError> {
        if self.bound {
            return Ok(());
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
        let mut material = |r: &MaterialLink| -> Result<Handle<StandardMaterial>, ReferenceError> {
            let key = (r.asset, r.gltf_material);
            if let Some(h) = handles.get(&key) {
                return Ok(Handle::clone(h));
            }
            let a = reference
                .assets
                .iter()
                .find(|a| a.id == r.asset)
                .ok_or_else(|| {
                    ReferenceError(format!("Missing instrument material asset {}", r.asset.0))
                })?;
            // Bevy PBR's glTF extension creates StandardMaterial subassets at MaterialN/std.
            let path = format!("{}#Material{}/std", a.path, r.gltf_material);
            let handle: Handle<StandardMaterial> = server.load(path.clone());
            handles.insert(key, handle.clone());
            self.materials.push((path, handle.clone()));
            Ok(handle)
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
                on: material(&lamp.on)?,
                off: material(&lamp.off)?,
            });
        }
        for digit in &reference.instruments.odometer.digits {
            material(&digit.on)?;
            material(&digit.off)?;
        }
        self.bound = true;
        Ok(())
    }
    pub fn ready(&self, server: &AssetServer) -> Result<bool, ReferenceError> {
        if !self.bound {
            return Ok(false);
        }
        for (path, handle) in &self.materials {
            if let Some(LoadState::Failed(error)) = server.get_load_state(handle.id()) {
                return Err(ReferenceError(format!(
                    "Cannot load instrument material {path}: {error}"
                )));
            }
            if !server.is_loaded_with_dependencies(handle.id()) {
                return Ok(false);
            }
        }
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
