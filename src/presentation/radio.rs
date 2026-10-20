//! Radio materials and the source world-space tuning needle.
mod needle;

use super::camera::DriveCamera;
use super::instruments::MaterialVisual;
use super::materials::SourceMaterial;
use crate::reference::{ItemKind, NodeId, RadioVisualReference, ReferenceError, ReferenceManifest};
use bevy::math::Affine3A;
use bevy::prelude::*;
use std::collections::HashMap;

pub(super) struct RadioVisuals {
    pub(super) materials: [MaterialVisual; 2],
    needle: Entity,
    parent_path: Vec<Entity>,
    start_path: Vec<Entity>,
    end_path: Vec<Entity>,
    last_power: Option<bool>,
}

impl RadioVisuals {
    pub(super) fn bind(
        reference: &ReferenceManifest,
        radio: &RadioVisualReference,
        nodes: &HashMap<NodeId, Entity>,
        materials: [MaterialVisual; 2],
    ) -> Result<Self, ReferenceError> {
        let node = |id: NodeId| {
            reference
                .nodes
                .iter()
                .find(|n| n.id == id)
                .ok_or_else(|| ReferenceError(format!("Missing radio node {}", id.0)))
        };
        let entity = |id: NodeId| {
            nodes
                .get(&id)
                .copied()
                .ok_or_else(|| ReferenceError(format!("Missing radio entity {}", id.0)))
        };
        let path = |mut current: Option<NodeId>| {
            let mut path = Vec::new();
            while let Some(id) = current {
                path.push(entity(id)?);
                // These nodes receive world poses after their source parent is removed.
                let render_root = reference.bodies.iter().any(|b| b.node == id)
                    || reference.items.iter().any(|item| {
                        item.node == id
                            && (matches!(
                                item.kind,
                                ItemKind::Wheel | ItemKind::Rim | ItemKind::Tire
                            ) || reference.parts.iter().any(|p| p.item == item.id)
                                || reference.plates.plates.iter().any(|p| p.item == item.id)
                                || reference
                                    .flashlights
                                    .instances
                                    .iter()
                                    .any(|f| f.item == item.id))
                    });
                current = if render_root { None } else { node(id)?.parent };
            }
            path.reverse();
            Ok::<_, ReferenceError>(path)
        };
        Ok(Self {
            materials,
            needle: entity(radio.needle)?,
            parent_path: path(node(radio.needle)?.parent)?,
            start_path: path(Some(radio.start))?,
            end_path: path(Some(radio.end))?,
            last_power: None,
        })
    }

    pub(super) fn update(
        &mut self,
        power: bool,
        tuning: f32,
        transforms: &mut Query<&mut Transform, Without<DriveCamera>>,
        materials: &mut Query<&mut MeshMaterial3d<SourceMaterial>>,
    ) {
        let compose = |path: &[Entity]| {
            path.iter().try_fold(Affine3A::IDENTITY, |world, &entity| {
                transforms
                    .get(entity)
                    .map(|local| world * local.compute_affine())
            })
        };
        let poses = (
            compose(&self.start_path),
            compose(&self.end_path),
            compose(&self.parent_path),
        );
        if let (Ok(start), Ok(end), Ok(parent)) = poses {
            if let Ok(mut transform) = transforms.get_mut(self.needle) {
                needle::update_needle(&mut transform, start, end, parent, tuning);
            }
        }
        if self.last_power != Some(power) {
            for visual in &self.materials {
                visual.select(power, materials);
            }
            self.last_power = Some(power);
        }
    }
}
