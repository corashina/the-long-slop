//! Bind source lamp nodes and material variants once during scene loading.
use super::reference::ReferenceVisuals;
use crate::interaction::lighting::LightingState;
use crate::reference::{LampType, ReferenceManifest};
use bevy::camera::visibility::RenderLayers;
use bevy::ecs as bevy_ecs;
use bevy::gltf::GltfAssetLabel;
use bevy::prelude::*;

// Unity legacy intensity has no lumen unit. This adapter uses Bevy's daylight exposure.
const LEGACY_LUMENS: f32 = 100_000.0;
#[derive(Component)]
pub struct LampVisual {
    group: usize,
    state: usize,
    intensity: f32,
}
#[derive(Component)]
pub struct LampMaterialVisual {
    group: usize,
    states: Vec<Option<Handle<StandardMaterial>>>,
}
#[derive(Component)]
pub struct LampButtonVisual {
    group: usize,
    on: Transform,
    off: Transform,
}

pub fn load_materials(
    server: &AssetServer,
    reference: &ReferenceManifest,
) -> Vec<Vec<Option<Handle<StandardMaterial>>>> {
    reference
        .lighting
        .groups
        .iter()
        .map(|group| {
            group
                .states
                .iter()
                .map(|state| {
                    state.material.as_ref().map(|material| {
                        let asset = reference
                            .assets
                            .iter()
                            .find(|asset| asset.id == material.asset)
                            .expect("validated lamp asset");
                        // Bevy 0.19's PBR glTF extension registers StandardMaterial under /std.
                        server
                            .load_builder()
                            .with_settings(super::reference::configure_import)
                            .load(format!(
                                "{}#{}/std",
                                asset.path,
                                GltfAssetLabel::Material {
                                    index: material.index as usize,
                                    is_scale_inverted: false
                                }
                            ))
                    })
                })
                .collect()
        })
        .collect()
}
pub fn bind(
    commands: &mut Commands,
    reference: &ReferenceManifest,
    visuals: &ReferenceVisuals,
    primitives: &Query<(Entity, &ChildOf), With<Mesh3d>>,
    materials: &[Vec<Option<Handle<StandardMaterial>>>],
) {
    for (index, group) in reference.lighting.groups.iter().enumerate() {
        for (state_index, state) in group.states.iter().enumerate() {
            for lamp in &state.lamps {
                let node = visuals.nodes[&lamp.node];
                commands.entity(node).insert(Visibility::Inherited);
                let intensity = if group.control == crate::reference::LampControl::Home {
                    lamp.intensity
                } else {
                    state.intensity
                };
                let entity = commands
                    .spawn((
                        LampVisual {
                            group: index,
                            state: state_index,
                            intensity,
                        },
                        Transform::IDENTITY,
                        Visibility::Hidden,
                        (0..32).collect::<RenderLayers>(),
                    ))
                    .id();
                match lamp.lamp_type {
                    LampType::Point => {
                        commands.entity(entity).insert(PointLight {
                            color: Color::srgb(lamp.color[0], lamp.color[1], lamp.color[2]),
                            intensity: 0.0,
                            range: lamp.range,
                            ..default()
                        });
                    }
                    LampType::Spot => {
                        commands.entity(entity).insert(SpotLight {
                            color: Color::srgb(lamp.color[0], lamp.color[1], lamp.color[2]),
                            intensity: 0.0,
                            range: lamp.range,
                            inner_angle: lamp.inner_degrees.to_radians() * 0.5,
                            outer_angle: lamp.outer_degrees.to_radians() * 0.5,
                            ..default()
                        });
                    }
                }
                commands.entity(node).add_child(entity);
            }
        }
        // A renderer can occur in several source states. Bind it once.
        let mut bound = std::collections::HashSet::new();
        for state in &group.states {
            for node in &state.renderers {
                if !bound.insert(*node) {
                    continue;
                }
                for (primitive, parent) in primitives {
                    if parent.parent() == visuals.nodes[node] {
                        commands.entity(primitive).insert(LampMaterialVisual {
                            group: index,
                            states: materials[index].clone(),
                        });
                    }
                }
            }
        }
        if let Some(button) = &group.button {
            let pose = |id| {
                let source = &reference
                    .nodes
                    .iter()
                    .find(|n| n.id == id)
                    .expect("lamp button")
                    .transform;
                Transform::from_translation(Vec3::from_array(source.translation))
                    .with_rotation(Quat::from_array(source.rotation))
                    .with_scale(Vec3::from_array(source.scale))
            };
            commands
                .entity(visuals.nodes[&button.node])
                .insert(LampButtonVisual {
                    group: index,
                    on: pose(button.on),
                    off: pose(button.off),
                });
        }
    }
}
pub fn sync(
    lighting: &LightingState,
    lamps: &mut Query<
        (
            &LampVisual,
            &mut Visibility,
            Option<&mut PointLight>,
            Option<&mut SpotLight>,
        ),
        Without<super::reference::ValveVisibility>,
    >,
    materials: &mut Query<(&LampMaterialVisual, &mut MeshMaterial3d<StandardMaterial>)>,
    buttons: &mut Query<
        (&LampButtonVisual, &mut Transform),
        (
            Without<super::reference::BodyVisual>,
            Without<super::reference::ItemVisual>,
            Without<super::reference::CapVisual>,
        ),
    >,
) {
    for (lamp, mut visibility, point, spot) in lamps {
        let output = lighting.output(lamp.group, lamp.state);
        visibility.set_if_neq(if output > 0.0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        let intensity = lamp.intensity * output * LEGACY_LUMENS;
        if let Some(mut point) = point
            && point.intensity != intensity
        {
            point.intensity = intensity;
        }
        if let Some(mut spot) = spot
            && spot.intensity != intensity
        {
            spot.intensity = intensity;
        }
    }
    for (binding, mut material) in materials {
        let state = &lighting.groups[binding.group];
        let index = if state.energy * state.electronics_output > 0.0 {
            usize::from(state.current)
        } else {
            0
        };
        if let Some(handle) = &binding.states[index] {
            material.set_if_neq(MeshMaterial3d(handle.clone()));
        }
    }
    for (button, mut transform) in buttons {
        transform.set_if_neq(if lighting.groups[button.group].on {
            button.on
        } else {
            button.off
        });
    }
}
