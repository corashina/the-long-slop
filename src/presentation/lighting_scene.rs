//! Live master day-cycle values and rebased local probe poses.
use super::{
    lighting_assets::LightingAssets, readiness::RendererStateHandle, reference::ReferenceVisuals,
    source_lighting::LightingUniform,
};
use crate::{app::RuntimeReference, reference::reflections::probe_influence_bounds};
use bevy::prelude::*;
use bevy::render::extract_resource::ExtractResource;
use bevy::{ecs as bevy_ecs, render as bevy_render};
use simulation::boundary::SimFrame;

pub const PROBE_COUNT: usize = crate::render_readiness::PROBE_COUNT;

#[derive(Resource)]
pub struct ProbeActivation {
    pub enabled: [bool; PROBE_COUNT],
    pub checks: u32,
    pub distance_squared: [f32; PROBE_COUNT],
    scene: u32,
    /// A car carries the probe. Without one the probe is off.
    followed: bool,
    elapsed: f32,
    last_check: f32,
}
impl Default for ProbeActivation {
    fn default() -> Self {
        Self {
            enabled: [true; PROBE_COUNT],
            checks: 0,
            distance_squared: [0.; PROBE_COUNT],
            scene: 0,
            followed: false,
            elapsed: 0.,
            last_check: 0.,
        }
    }
}
#[derive(Resource, Default)]
pub struct SourceWorldOrigin(pub [f64; 3]);

#[derive(Clone, Copy)]
pub struct SceneLighting {
    pub ambient_sh: [Vec4; 7],
    pub sky: super::sky::SkyUniform,
    pub cloud_speed: Vec2,
    pub sun_rotation: Quat,
    pub sun_radiance: Vec4,
    pub direction_to_sun: Vec4,
    pub cookie_from_world: Mat4,
    pub fog_color_density: Vec4,
    pub probe_position: [Vec3; PROBE_COUNT],
    pub probe_center: [Vec4; PROBE_COUNT],
    pub probe_extents: [Vec4; PROBE_COUNT],
    pub source_time: Vec4,
}
impl SceneLighting {
    pub fn for_view(
        &self,
        shadow_fade: Vec4,
        shadow_fade_center: Vec4,
        clip_from_world: Mat4,
        near_far: Vec4,
        sun_shadow: super::shadow_collector::SunShadowUniform,
    ) -> LightingUniform {
        LightingUniform {
            ambient_sh: self.ambient_sh,
            sun_radiance: self.sun_radiance,
            direction_to_sun: self.direction_to_sun,
            cookie_from_world: self.cookie_from_world,
            fog_color_density: self.fog_color_density,
            shadow_fade,
            shadow_fade_center,
            probe_center: self.probe_center,
            probe_extents: self.probe_extents,
            source_time: self.source_time,
            clip_from_world,
            near_far,
            sun_shadow,
        }
    }
}
#[derive(Resource, Default, Clone, ExtractResource)]
pub struct SourceSceneLighting(pub Option<SceneLighting>);

/// Schedule after master sync_lighting and transform propagation.
pub(crate) fn sync_lighting(
    assets: Option<Res<LightingAssets>>,
    visuals: Option<Res<ReferenceVisuals>>,
    master: Option<Res<super::lighting::SourceLighting>>,
    sim: Option<Res<super::sim_frame::SimFrameResource>>,
    reference: Option<Res<RuntimeReference>>,
    mut origin: ResMut<SourceWorldOrigin>,
    mut activation: ResMut<ProbeActivation>,
    epoch: Res<super::view_frame::SourceSceneEpoch>,
    transforms: Query<&GlobalTransform>,
    mut scene: ResMut<SourceSceneLighting>,
    status: Res<RendererStateHandle>,
    instances: Option<Res<super::world::instances::SourceInstances>>,
) {
    scene.0 = None;
    if status.failure().is_some() {
        return;
    }
    let (Some(assets), Some(visuals), Some(master), Some(sim), Some(reference)) =
        (assets, visuals, master, sim, reference)
    else {
        return;
    };
    if !assets.loaded || !visuals.bound || !master.bound {
        return;
    }
    let car = instances
        .as_deref()
        .and_then(|instances| followed_probe(&sim, &reference, instances));
    let sim = &sim.frame;
    origin.0 = sim.state.origin;
    let player = Vec3::from_array(sim.state.body_pose.position);
    match scene_value(&assets, &master, sim, &reference, &transforms, player, car) {
        Ok(mut value) => {
            if activation.scene != epoch.0 || activation.followed != car.is_some() {
                activation.scene = epoch.0;
                activation.followed = car.is_some();
                activation.enabled = [car.is_some(); PROBE_COUNT];
                activation.elapsed = 0.;
                activation.last_check = 0.;
                activation.checks = 0;
            }
            activation.elapsed += sim.state.perf.frame_seconds();
            for index in 0..PROBE_COUNT {
                activation.distance_squared[index] =
                    player.distance_squared(value.probe_position[index]);
            }
            // ReflectionDisableDistance checks registered probes every five seconds.
            if activation.elapsed - activation.last_check >= 5. {
                activation.last_check = activation.elapsed;
                let followed = activation.followed;
                activation.enabled = activation
                    .distance_squared
                    .map(|distance| followed && distance < 2500.);
                activation.checks += 1;
            }
            for index in 0..PROBE_COUNT {
                value.probe_extents[index].w = f32::from(activation.enabled[index]);
            }
            scene.0 = Some(value);
        }
        Err(error) => {
            status.fail(format!("Source lighting pose: {error}"));
        }
    }
}
/// The probe node entity and influence box of the world car the player last sat in.
/// Without one, the start Lada copy.
fn followed_probe(
    sim: &super::sim_frame::SimFrameResource,
    reference: &RuntimeReference,
    instances: &super::world::instances::SourceInstances,
) -> Option<(Entity, [f32; 3], [f32; 3])> {
    let (id, prototype) = sim.followed_car.as_ref()?;
    let prototype = &reference.0.world.prototypes[*prototype as usize];
    let probe = prototype.vehicle.as_ref()?.probe.as_ref()?;
    let node = prototype.nodes[probe.node as usize].gltf_node as usize;
    let entity = (*instances.nodes_of(id)?.get(node)?)?;
    Some((entity, probe.bounds_size, probe.bounds_offset))
}
fn scene_value(
    assets: &LightingAssets,
    master: &super::lighting::SourceLighting,
    sim: &SimFrame,
    reference: &RuntimeReference,
    transforms: &Query<&GlobalTransform>,
    player: Vec3,
    car: Option<(Entity, [f32; 3], [f32; 3])>,
) -> Result<SceneLighting, String> {
    if !sim.state.origin.iter().all(|value| value.is_finite()) {
        return Err("Nonfinite world origin".into());
    }
    if assets.record.probes.len() != PROBE_COUNT {
        return Err("Selected vehicle reflection probe count is invalid".into());
    }
    let mut positions = [Vec3::ZERO; PROBE_COUNT];
    let mut centers = [Vec4::ZERO; PROBE_COUNT];
    let mut sizes = [Vec4::ZERO; PROBE_COUNT];
    for (index, probe) in assets.record.probes.iter().enumerate() {
        // The probe follows the car the player last sat in. Without a car it waits at the player.
        let (position, size, offset) = match car {
            Some((entity, size, offset)) => (
                transforms
                    .get(entity)
                    .map_err(|_| "Missing probe world transform")?
                    .translation(),
                size,
                offset,
            ),
            None => (player, probe.bounds_size, probe.bounds_offset),
        };
        if !position.is_finite() {
            return Err("Nonfinite probe world position".into());
        }
        let (center, extents) = probe_influence_bounds(position.to_array(), size, offset);
        positions[index] = position;
        centers[index] = Vec3::from_array(center).extend(0.);
        sizes[index] = Vec3::from_array(extents).extend(0.);
    }
    let bound = master
        .lights
        .iter()
        .find(|bound| {
            !bound.road
                && reference.0.lighting.lights[bound.source_index].kind
                    == crate::reference::LightKind::Directional
        })
        .ok_or("Missing live source sun")?;
    let rotation = transforms
        .get(bound.entity)
        .map_err(|_| "Missing live sun transform")?
        .compute_transform()
        .rotation;
    let sun = &master.uniform.lights[bound.index];
    let cookie_size = reference.0.lighting.lights[bound.source_index].cookie_size;
    let cookie_from_world =
        Mat4::from_translation(Vec3::new(
            sun.cookie_rect.y + 0.5,
            sun.cookie_rect.z + 0.5,
            0.,
        )) * Mat4::from_scale(Vec3::new(1. / cookie_size, 1. / cookie_size, 0.))
            * sun.cookie_from_world;
    let time = sim.lighting.cycle.shader_time_seconds as f32;
    if !cookie_from_world.is_finite() || !time.is_finite() || !rotation.is_finite() {
        return Err("Nonfinite live lighting transform or time".into());
    }
    // Native Flat SH writes the linear ambient RGB into SHAr/Ag/Ab.w.
    let mut ambient_sh = [Vec4::ZERO; 7];
    for (row, channel) in ambient_sh[..3]
        .iter_mut()
        .zip(master.uniform.ambient.to_array())
    {
        row.w = channel;
    }
    Ok(SceneLighting {
        sky: master.sky.uniform,
        cloud_speed: Vec2::new(
            sim.lighting.cycle.clouds[0].speed,
            sim.lighting.cycle.clouds[1].speed,
        ),
        ambient_sh,
        sun_rotation: rotation,
        sun_radiance: sun.color.truncate().extend(0.),
        direction_to_sun: (-sun.direction_kind.truncate()).extend(0.),
        cookie_from_world,
        fog_color_density: master
            .uniform
            .fog
            .truncate()
            .extend(master.uniform.ambient.w),
        probe_position: positions,
        probe_center: centers,
        probe_extents: sizes,
        source_time: Vec4::new(time * 0.05, time, time * 2., time * 3.),
    })
}
