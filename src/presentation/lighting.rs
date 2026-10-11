//! Sun, ambient light, fog color, and car lights.
//!
//! Unity light units have no exact Bevy match. One Unity intensity unit is `SUN_LUX`.

use crate::reference::{ItemId, ReferenceManifest};
use crate::simulation::day::{Sky, sky};
use crate::starting_area::GarageRuntime;
use bevy::ecs as bevy_ecs;
use bevy::prelude::*;

/// Illuminance of one Unity sun intensity unit.
const SUN_LUX: f32 = 8000.0;
/// Bevy default ambient brightness matches the source day ambient value.
const AMBIENT_NITS_PER_UNIT: f32 = 400.0;

#[derive(Component)]
pub struct Sun;

#[derive(Component)]
pub struct HeadlightBeam {
    item: ItemId,
    state: usize,
    /// Lumens per Unity intensity unit.
    lumens: f32,
}

#[derive(Component)]
pub struct HeadlightLens {
    item: ItemId,
    materials: Vec<Handle<StandardMaterial>>,
    lit: Option<(usize, bool)>,
}

/// Spot lights and lens markers on the bound glTF nodes.
pub fn bind(
    commands: &mut Commands,
    reference: &ReferenceManifest,
    nodes: &std::collections::HashMap<crate::reference::NodeId, Entity>,
) {
    for headlight in &reference.headlights {
        commands
            .entity(nodes[&headlight.lens_node])
            .insert(HeadlightLens {
                item: headlight.item,
                materials: Vec::new(),
                lit: None,
            });
        for (state, record) in headlight.states.iter().enumerate() {
            let Some(light) = &record.light else {
                continue;
            };
            // Unity attenuation halves at one fifth of the range. Match that point.
            let half = light.range / 5.0;
            let lumens = SUN_LUX * 0.5 * 4.0 * std::f32::consts::PI * half * half;
            let child = commands
                .spawn((
                    SpotLight {
                        color: Color::srgb(light.color[0], light.color[1], light.color[2]),
                        intensity: 0.0,
                        range: light.range,
                        outer_angle: (light.spot_degrees * 0.5).to_radians(),
                        inner_angle: 0.0,
                        shadow_maps_enabled: false,
                        ..default()
                    },
                    Transform::IDENTITY,
                    Visibility::Hidden,
                    HeadlightBeam {
                        item: headlight.item,
                        state,
                        lumens,
                    },
                ))
                .id();
            commands.entity(nodes[&light.node]).add_child(child);
        }
    }
}

pub fn sync_sky(
    reference: &ReferenceManifest,
    runtime: &GarageRuntime,
    sun: &mut Query<(&mut DirectionalLight, &mut Transform), With<Sun>>,
    ambient: &mut GlobalAmbientLight,
    clear: &mut ClearColor,
    fog: &mut Query<&mut DistanceFog>,
) {
    let Sky {
        sun_rotation,
        sun_color,
        sun_intensity,
        ambient: flat,
        fog_color,
    } = sky(
        &reference.day_cycle,
        runtime.day_seconds,
        runtime.terrain_color,
    );
    for (mut light, mut transform) in sun {
        light.color = Color::srgb(sun_color[0], sun_color[1], sun_color[2]);
        light.illuminance = sun_intensity * SUN_LUX;
        transform.rotation = Quat::from_array(sun_rotation);
    }
    ambient.brightness = flat * AMBIENT_NITS_PER_UNIT;
    let fog_color = Color::srgb(fog_color[0], fog_color[1], fog_color[2]);
    clear.0 = fog_color;
    for mut fog in fog {
        fog.color = fog_color;
    }
}

pub fn sync_headlights(
    reference: &ReferenceManifest,
    runtime: &GarageRuntime,
    beams: &mut Query<(&HeadlightBeam, &mut SpotLight, &mut Visibility)>,
    lenses: &mut Query<(&mut HeadlightLens, &Children)>,
    primitives: &Query<&MeshMaterial3d<StandardMaterial>>,
    commands: &mut Commands,
    materials: &mut Assets<StandardMaterial>,
) {
    for (beam, mut light, mut visibility) in beams {
        let Some(state) = runtime
            .headlights
            .iter()
            .find(|light| light.item == beam.item)
        else {
            continue;
        };
        let record = &reference
            .headlights
            .iter()
            .find(|light| light.item == beam.item)
            .expect("bound headlight")
            .states[beam.state];
        let on = state.current == beam.state && state.energy > 0.0;
        visibility.set_if_neq(if on {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        });
        let intensity = if on {
            record.intensity * state.energy * beam.lumens
        } else {
            0.0
        };
        if light.intensity != intensity {
            light.intensity = intensity;
        }
    }
    for (mut lens, children) in lenses {
        // Lens primitives share source materials. Each lens gets its own copy.
        if lens.materials.is_empty() {
            for child in children {
                if let Ok(material) = primitives.get(*child)
                    && let Some(copy) = materials.get(&material.0).cloned()
                {
                    let handle = materials.add(copy);
                    commands
                        .entity(*child)
                        .insert(MeshMaterial3d(handle.clone()));
                    lens.materials.push(handle);
                }
            }
        }
        let Some(state) = runtime
            .headlights
            .iter()
            .find(|light| light.item == lens.item)
        else {
            continue;
        };
        let lit = (state.current, state.energy > 0.0);
        if lens.lit == Some(lit) {
            continue;
        }
        lens.lit = Some(lit);
        let emission = reference
            .headlights
            .iter()
            .find(|light| light.item == lens.item)
            .and_then(|light| light.states[state.current].lens_emission)
            .filter(|_| lit.1)
            .unwrap_or([0.0; 3]);
        // A white surface under one sun unit has this luminance.
        let nits = SUN_LUX / std::f32::consts::PI;
        for handle in &lens.materials {
            if let Some(mut material) = materials.get_mut(handle) {
                material.emissive =
                    LinearRgba::rgb(emission[0] * nits, emission[1] * nits, emission[2] * nits);
            }
        }
    }
}
