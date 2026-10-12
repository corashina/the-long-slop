//! Live master day-cycle values and rebased local probe poses.
use super::{
    lighting_assets::LightingAssets,
    materials::RenderingState,
    reference::ReferenceVisuals,
    source_lighting::{LightingUniform, SourceLighting, SourceLightingStatus},
};
use crate::{
    app::{Drive, RuntimeReference},
    reference::reflections::probe_influence_bounds,
};
use bevy::prelude::*;
use bevy::render::extract_resource::ExtractResource;
use bevy::{ecs as bevy_ecs, render as bevy_render};

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
    pub probe_position: Vec3,
    pub probe_center: Vec4,
    pub probe_extents: Vec4,
    pub source_time: Vec4,
}
impl SceneLighting {
    pub fn for_view(
        &self,
        shadow_fade: Vec4,
        shadow_fade_center: Vec4,
        nonjittered_clip_from_world: Mat4,
        near: f32,
        far: f32,
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
            nonjittered_clip_from_world,
            near_far: Vec4::new(near, far, 0., 0.),
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
    drive: Option<Res<Drive>>,
    reference: Option<Res<RuntimeReference>>,
    mut origin: ResMut<SourceWorldOrigin>,
    transforms: Query<&GlobalTransform>,
    mut scene: ResMut<SourceSceneLighting>,
    mut lighting: ResMut<SourceLighting>,
    status: Res<SourceLightingStatus>,
) {
    scene.0 = None;
    if matches!(lighting.state, RenderingState::Failed(_)) {
        return;
    }
    let (Some(assets), Some(visuals), Some(master), Some(drive), Some(reference)) =
        (assets, visuals, master, drive, reference)
    else {
        return;
    };
    if !assets.loaded || !visuals.bound || !master.bound {
        return;
    }
    origin.0 = drive.runtime.area.origin;
    match scene_value(&assets, &visuals, &master, &drive, &reference, &transforms) {
        Ok(value) => scene.0 = Some(value),
        Err(error) => {
            let failure = RenderingState::Failed(format!("Source lighting pose: {error}"));
            lighting.state = failure.clone();
            *status.0.lock().expect("source lighting status lock") = failure;
        }
    }
}
fn scene_value(
    assets: &LightingAssets,
    visuals: &ReferenceVisuals,
    master: &super::lighting::SourceLighting,
    drive: &Drive,
    reference: &RuntimeReference,
    transforms: &Query<&GlobalTransform>,
) -> Result<SceneLighting, String> {
    if !drive
        .runtime
        .area
        .origin
        .iter()
        .all(|value| value.is_finite())
    {
        return Err("Nonfinite world origin".into());
    }
    let probe = &assets.record.probes[0];
    let entity = visuals
        .nodes
        .get(&probe.node)
        .ok_or("Missing bound probe node")?;
    let position = transforms
        .get(*entity)
        .map_err(|_| "Missing probe world transform")?
        .translation();
    if !position.is_finite() {
        return Err("Nonfinite probe world position".into());
    }
    let (center, extents) =
        probe_influence_bounds(position.to_array(), probe.bounds_size, probe.bounds_offset);
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
    let time = drive.runtime.lighting.cycle.output().shader_time_seconds as f32;
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
            drive.runtime.lighting.cycle.output().clouds[0].speed,
            drive.runtime.lighting.cycle.output().clouds[1].speed,
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
        probe_position: position,
        probe_center: Vec3::from_array(center).extend(0.),
        probe_extents: Vec3::from_array(extents).extend(0.),
        source_time: Vec4::new(time * 0.05, time, time * 2., time * 3.),
    })
}
