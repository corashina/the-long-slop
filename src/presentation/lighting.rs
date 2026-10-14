//! Persistent WebGL2 source lighting resources.
use super::{
    materials::{MaterialCache, SourceMaterial, bind_materials},
    reference::ReferenceVisuals,
};
use crate::{
    app::{Drive, RuntimeReference},
    lighting::source_linear,
    reference::{LightId, LightKind, LightingRecord},
};
use bevy::asset::LoadState;
use bevy::camera::visibility::RenderLayers;
use bevy::ecs as bevy_ecs;
use bevy::prelude::*;
use bevy::render::{
    Extract, ExtractSchedule, Render, RenderApp, RenderStartup, RenderSystems,
    render_asset::RenderAssets,
    render_resource::*,
    renderer::{RenderDevice, RenderQueue},
    texture::GpuImage,
};
use bevy::shader::Shader;
use std::sync::{Arc, Mutex};

// Both source vehicles and the full streamed road lamp window fit the 16 KiB uniform limit.
pub const MAX_LIGHTS: usize = 40;
pub const SHADOW_SLOTS: usize = 100;
pub const ATLAS_SIZE: u32 = 4000;
pub const TILE_SIZE: u32 = 400;
#[derive(Clone, Debug, PartialEq)]
pub enum RenderingState {
    Loading,
    Ready,
    Failed(String),
}
#[derive(Clone, Copy, ShaderType, Default)]
pub struct GpuLight {
    pub cookie_from_world: Mat4,
    pub position_range: Vec4,
    pub direction_kind: Vec4,
    pub color: Vec4,
    pub cookie_rect: Vec4,
    pub shadow: Vec4,
    pub mask: UVec4,
}
#[derive(Clone, Copy, ShaderType, Default)]
pub struct GpuShadow {
    pub clip_from_world: Mat4,
    pub rect: Vec4,
    pub bias: Vec4,
}
#[derive(Clone, Copy, ShaderType)]
pub struct LightingUniform {
    pub lights: [GpuLight; MAX_LIGHTS],
    pub shadows: [GpuShadow; SHADOW_SLOTS],
    pub ambient: Vec4,
    pub fog: Vec4,
    pub counts: UVec4,
}
impl Default for LightingUniform {
    fn default() -> Self {
        Self {
            lights: [GpuLight::default(); MAX_LIGHTS],
            shadows: [GpuShadow::default(); SHADOW_SLOTS],
            ambient: Vec4::ZERO,
            fog: Vec4::ZERO,
            counts: UVec4::ZERO,
        }
    }
}
#[derive(Component, Clone, Copy)]
pub struct SourceLight {
    pub index: usize,
    pub shadow_start: usize,
    pub faces: usize,
    pub enabled: bool,
}
#[derive(Clone)]
pub struct BoundLight {
    pub index: usize,
    pub road: bool,
    pub id: LightId,
    pub source_index: usize,
    pub entity: Entity,
    pub node: Option<Entity>,
    pub item: Option<crate::reference::ItemId>,
    pub body: Option<crate::reference::BodyId>,
    pub pose: Transform,
}
#[derive(Clone)]
pub struct BoundLampMaterial {
    pub output_index: usize,
    pub receivers: Vec<Entity>,
    pub current: Option<(crate::reference::AssetId, u32)>,
    pub choices: Vec<super::materials::LampMaterialChoice>,
}
#[derive(Clone)]
pub struct AuthoredMaterial {
    pub asset: crate::reference::AssetId,
    pub index: u32,
    pub handle: Handle<StandardMaterial>,
}
#[derive(Resource, Clone)]
pub struct SourceLighting {
    pub sky: super::sky::SkyState,
    pub(crate) road_lamps: super::world::RoadLamps,
    pub(crate) stations: super::world::StationModels,
    pub materials: Vec<AuthoredMaterial>,
    material_loads: Option<SourceMaterialCapture>,
    pub materials_ready: bool,
    pub materials_prepared: bool,
    pub warming: bool,
    pub warmups: Vec<(Entity, bool)>,
    pub lamp_materials: Vec<BoundLampMaterial>,
    control_poses: Vec<(usize, Entity)>,
    visibility: Vec<(usize, Entity)>,
    pub state: RenderingState,
    pub bound: bool,
    pub lights: Vec<BoundLight>,
    pub uniform: LightingUniform,
    pub textures: Vec<(u32, String, Handle<Image>)>,
    pub shader: Handle<Shader>,
    pub cookie_handles: Option<[Handle<Image>; 3]>,
    pub status: Arc<Mutex<RenderingState>>,
}
impl SourceLighting {
    pub(crate) fn prepare_copy(&self) -> Self {
        let mut copy = self.clone();
        copy.road_lamps.clear_owned();
        copy.stations.clear_owned();
        copy.lights.retain(|light| !light.road);
        for light in self.lights.iter().filter(|light| light.road) {
            copy.uniform.lights[light.index] = GpuLight::default();
        }
        copy.uniform.counts.x = copy.road_lamps.fixed_lights as u32;
        copy.uniform.counts.y = copy.road_lamps.fixed_shadows as u32;
        copy
    }

    /// Replace owned light instances. Retain loaded scenes and prepared assets.
    pub fn reset_instances(&mut self, commands: &mut Commands) {
        self.road_lamps.reset_instances(commands);
        self.stations.reset_instances(commands);
        self.sky.reset_views();
        for light in self.lights.drain(..) {
            commands.entity(light.entity).despawn();
        }
        self.uniform.lights.fill(GpuLight::default());
        self.uniform.shadows.fill(GpuShadow::default());
        self.uniform.counts.x = 0;
        self.uniform.counts.y = 0;
        for material in &mut self.lamp_materials {
            material.current = None;
        }
        self.bound = false;
    }
}
#[derive(Resource)]
pub struct LightingGpu {
    pub uniform: UniformBuffer<LightingUniform>,
    pub atlas: Texture,
    pub atlas_view: TextureView,
    pub shadow_sampler: Sampler,
    pub hard_shadow_sampler: Sampler,
    pub cookie_sampler: Sampler,
    pub cookies: Option<[(TextureView, Sampler); 3]>,
    pub attenuation_view: TextureView,
    pub source_shader: Option<AssetId<Shader>>,
    pub sky_shader: Option<AssetId<Shader>>,
    pub status: Arc<Mutex<RenderingState>>,
}
#[derive(Resource, Default)]
struct ExtractedLighting {
    shader: Option<AssetId<Shader>>,
    sky_shader: Option<AssetId<Shader>>,
    prepared: bool,
    warming: bool,
    warmups: Vec<(bevy::render::sync_world::MainEntity, bool)>,
    uniform: LightingUniform,
    cookies: Option<[AssetId<Image>; 3]>,
    status: Option<Arc<Mutex<RenderingState>>>,
}

pub struct SourceLightingPlugin;
impl Plugin for SourceLightingPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset_loader::<super::textures::SourceImageLoader>()
            .add_plugins((
                MaterialPlugin::<SourceMaterial>::default(),
                super::shadows::SourceShadowPlugin,
                super::sky::SourceSkyPlugin,
            ))
            .init_resource::<MaterialCache>()
            .add_systems(
                Update,
                (
                    bind_materials,
                    super::world::prepare_road_lamp,
                    super::world::prepare_stations,
                    super::instruments::prepare_instruments,
                    super::materials::prepare_lamp_materials,
                    super::materials::sync_lamp_materials,
                )
                    .chain(),
            );
        app.add_systems(
            PostUpdate,
            (super::world::sync_road_lamps, super::world::sync_stations)
                .before(bevy::transform::TransformSystems::Propagate),
        );
        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<ExtractedLighting>()
            .add_systems(RenderStartup, init_gpu)
            .add_systems(ExtractSchedule, extract_lighting)
            .add_systems(
                Render,
                upload_lighting.in_set(RenderSystems::PrepareResources),
            )
            .add_systems(Render, check_pipelines.in_set(RenderSystems::Cleanup));
    }
}
pub fn load_lighting(
    commands: &mut Commands,
    server: &AssetServer,
    reference: &crate::reference::ReferenceManifest,
) {
    let lighting = &reference.lighting;
    let textures = lighting
        .textures
        .iter()
        .map(|texture| {
            let (srgb, filter, wrap_u, wrap_v) = (
                texture.color_space != 0,
                texture.filter_mode,
                texture.wrap_u,
                texture.wrap_v,
            );
            let handle = server
                .load_builder()
                .with_settings(move |settings: &mut super::textures::SourceImageSettings| {
                    *settings = super::textures::SourceImageSettings {
                        srgb,
                        filter,
                        wrap_u,
                        wrap_v,
                    };
                })
                .load(texture.mip_data.path.clone());
            (
                texture.id,
                format!(
                    "{}:{} {}",
                    texture.source.file, texture.source.path_id, texture.mip_data.path
                ),
                handle,
            )
        })
        .collect();
    commands.insert_resource(SourceLighting {
        sky: super::sky::SkyState::new(server, lighting),
        road_lamps: super::world::RoadLamps::new(
            lighting.road_lamp.asset,
            lighting.road_lamp.spacing_meters as f64,
        ),
        stations: super::world::StationModels::new(&reference.stations),
        materials: Vec::new(),
        material_loads: None,
        materials_ready: false,
        materials_prepared: false,
        warming: true,
        warmups: Vec::new(),
        lamp_materials: Vec::new(),
        control_poses: Vec::new(),
        visibility: Vec::new(),
        state: RenderingState::Loading,
        bound: false,
        lights: Vec::with_capacity(MAX_LIGHTS),
        uniform: LightingUniform::default(),
        textures,
        shader: server.load("shaders/source_surface.wgsl"),
        cookie_handles: None,
        status: Arc::new(Mutex::new(RenderingState::Loading)),
    });
    commands.insert_resource(bevy::light::PointLightShadowMap {
        size: (TILE_SIZE - 2) as usize,
    });
}
struct PendingMaterial {
    asset: crate::reference::AssetId,
    index: u32,
    path: String,
    handle: Option<Handle<StandardMaterial>>,
}
#[derive(Clone)]
struct SourceMaterialCapture(Arc<Mutex<Vec<PendingMaterial>>>);
impl bevy::gltf::extensions::GltfExtensionHandler for SourceMaterialCapture {
    fn dyn_clone(&self) -> Box<dyn bevy::gltf::extensions::ErasedGltfExtensionHandler> {
        Box::new(self.clone())
    }
    fn on_material(
        &mut self,
        context: &mut bevy::asset::LoadContext<'_>,
        material: &bevy::gltf::gltf::Material,
        _: Handle<bevy::gltf::GltfMaterial>,
        _: &bevy::gltf::GltfMaterial,
        label: &str,
    ) {
        let mut pending = self.0.lock().unwrap();
        for entry in pending.iter_mut() {
            if Some(entry.index as usize) == material.index()
                && context.path().path() == std::path::Path::new(&entry.path)
            {
                // Retain the PBR extension's existing subasset. Do not request another file load.
                entry.handle = Some(context.get_label_handle(format!("{label}/std")));
            }
        }
    }
}
pub fn load_lamp_materials(
    commands: &mut Commands,
    extensions: &bevy::gltf::extensions::GltfExtensionHandlers,
    reference: &crate::reference::ReferenceManifest,
) {
    let capture = SourceMaterialCapture(Arc::new(Mutex::new(
        reference
            .lighting
            .materials
            .iter()
            .map(|material| PendingMaterial {
                asset: material.asset,
                index: material.gltf_material,
                path: reference
                    .assets
                    .iter()
                    .find(|asset| asset.id == material.asset)
                    .unwrap()
                    .path
                    .clone(),
                handle: None,
            })
            .collect(),
    )));
    bevy::tasks::block_on(async {
        extensions.0.write().await.push(Box::new(capture.clone()));
    });
    commands.queue(move |world: &mut World| {
        world.resource_mut::<SourceLighting>().material_loads = Some(capture);
    });
}

pub(crate) fn bind_lighting(
    mut commands: Commands,
    reference: Res<RuntimeReference>,
    visuals: Res<ReferenceVisuals>,
    mut source: ResMut<SourceLighting>,
    images: Res<Assets<Image>>,
    drive: Res<Drive>,
    primitives: Query<(Entity, &ChildOf), With<Mesh3d>>,
) {
    if source.bound || !visuals.bound || matches!(source.state, RenderingState::Failed(_)) {
        return;
    }
    if source
        .textures
        .iter()
        .any(|(_, _, h)| !images.contains(h.id()))
    {
        return;
    }
    let r = &reference.0.lighting;
    let directional_cookie = r
        .textures
        .iter()
        .find(|texture| texture.id == r.cycle.cloud_shadow_cookie)
        .expect("validated directional cookie");
    if directional_cookie.wrap_u != 0 || directional_cookie.wrap_v != 0 {
        source.state = RenderingState::Failed(
            "Source directional cookie requires repeat wrapping on both axes".into(),
        );
        return;
    }
    let cookie_ids = [
        r.cycle.cloud_shadow_cookie,
        r.default_spot_cookie,
        r.lights
            .iter()
            .find(|l| l.kind == LightKind::Spot && l.cookie.is_some())
            .and_then(|l| l.cookie)
            .unwrap_or(r.default_spot_cookie),
    ];
    source.cookie_handles = Some(cookie_ids.map(|id| {
        source
            .textures
            .iter()
            .find(|(key, _, _)| *key == id)
            .unwrap()
            .2
            .clone()
    }));
    if !source.materials_prepared {
        for (output_index, output) in drive.runtime.lighting.material_outputs().iter().enumerate() {
            let Some(node) = visuals.nodes.get(&output.renderer) else {
                // Road prototype outputs bind when a streamed instance registers.
                continue;
            };
            let receivers = primitives
                .iter()
                .filter(|(_, parent)| parent.parent() == *node)
                .map(|(e, _)| e)
                .collect();
            source.lamp_materials.push(BoundLampMaterial {
                output_index,
                receivers,
                current: None,
                choices: Vec::new(),
            });
        }
        for (index, pose) in drive.runtime.lighting.control_poses().iter().enumerate() {
            source.control_poses.push((
                index,
                *visuals
                    .nodes
                    .get(&pose.node)
                    .expect("source control pose node"),
            ));
        }
        for (index, output) in drive
            .runtime
            .lighting
            .visibility_outputs()
            .iter()
            .enumerate()
        {
            source.visibility.push((
                index,
                *visuals
                    .nodes
                    .get(&output.node)
                    .expect("source lamp visibility node"),
            ));
        }
    }
    let mut shadow_start = 0;
    for (source_index, light) in r.lights.iter().enumerate() {
        if light.node.is_some_and(|id| {
            reference
                .0
                .nodes
                .iter()
                .any(|n| n.id == id && n.asset == r.road_lamp.asset)
        }) {
            continue;
        }
        let index = source.lights.len();
        let faces = if light.shadows.kind == 0 || light.kind == LightKind::Directional {
            0
        } else {
            match light.kind {
                LightKind::Point => 6,
                LightKind::Directional => unreachable!("source sun uses its dedicated atlas"),
                _ => 1,
            }
        };
        if index == MAX_LIGHTS || shadow_start + faces > SHADOW_SLOTS {
            source.state = RenderingState::Failed(format!(
                "Light {} exceeds shadow atlas capacity",
                light.id.0
            ));
            return;
        }
        let node = light.node.and_then(|id| visuals.nodes.get(&id).copied());
        if light.node.is_some() && node.is_none() {
            source.state =
                RenderingState::Failed(format!("Light {} has no imported node", light.id.0));
            return;
        }
        let pose = Transform::from_translation(Vec3::from_array(light.pose.translation))
            .with_rotation(Quat::from_array(light.pose.rotation));
        let mut entity = commands.spawn((
            SourceLight {
                index,
                shadow_start,
                faces,
                enabled: light.enabled,
            },
            pose,
            (0..32)
                .filter(|layer| light.culling_mask & (1u32 << layer) != 0)
                .flat_map(|layer| [layer, layer + 32])
                .collect::<RenderLayers>(),
        ));
        if source.warming {
            entity.insert(bevy::camera::visibility::NoFrustumCulling);
        }
        match light.kind {
            LightKind::Directional => {}
            LightKind::Point => {
                entity.insert(PointLight {
                    intensity: 0.,
                    range: light.range_meters,
                    shadow_maps_enabled: light.enabled,
                    shadow_map_near_z: light.shadows.near_plane,
                    shadow_depth_bias: light.shadows.bias,
                    shadow_normal_bias: light.shadows.normal_bias,
                    ..default()
                });
            }
            LightKind::Spot => {
                entity.insert(SpotLight {
                    intensity: 0.,
                    range: light.range_meters,
                    outer_angle: light.full_cone_degrees.to_radians() * 0.5,
                    inner_angle: light.inner_cone_degrees.to_radians() * 0.5,
                    shadow_maps_enabled: light.enabled,
                    shadow_map_near_z: light.shadows.near_plane,
                    shadow_depth_bias: light.shadows.bias,
                    shadow_normal_bias: light.shadows.normal_bias,
                    ..default()
                });
            }
            LightKind::Area => {
                source.state = RenderingState::Failed(format!(
                    "Light {} has unsupported area type",
                    light.id.0
                ));
                return;
            }
        }
        let entity = entity.id();
        let mut ancestor = light.node;
        let mut item = None;
        let mut body = None;
        while let Some(id) = ancestor {
            if item.is_none() {
                item = reference
                    .0
                    .items
                    .iter()
                    .find(|item| item.node == id)
                    .map(|item| item.id);
            }
            if body.is_none() {
                body = reference
                    .0
                    .bodies
                    .iter()
                    .find(|body| body.node == id)
                    .map(|body| body.id);
            }
            ancestor = reference
                .0
                .nodes
                .iter()
                .find(|node| node.id == id)
                .and_then(|node| node.parent);
        }
        source.lights.push(BoundLight {
            index,
            road: false,
            item,
            body,
            id: light.id,
            source_index,
            entity,
            node,
            pose,
        });
        let cookie = light
            .cookie
            .or((light.kind == LightKind::Spot).then_some(r.default_spot_cookie));
        source.uniform.lights[index].cookie_rect = Vec4::new(
            cookie
                .and_then(|id| cookie_ids.iter().position(|c| *c == id))
                .map_or(0., |index| (index + 1) as f32),
            0.,
            0.,
            0.,
        );
        source.uniform.lights[index].shadow = Vec4::new(
            shadow_start as f32,
            faces as f32,
            light.shadows.strength,
            light.cookie_size,
        );
        source.uniform.lights[index].mask = UVec4::new(
            light.culling_mask,
            if light.kind == LightKind::Spot {
                let a = light.full_cone_degrees * 0.5 / 360.;
                let half = (a + a) * std::f32::consts::PI;
                (half.cos() / half.sin()).to_bits()
            } else {
                0
            },
            u32::from(light.shadows.kind),
            0,
        );
        shadow_start += faces;
    }
    source.uniform.counts.x = source.lights.len() as u32;
    source.uniform.counts.y = shadow_start as u32;
    source.road_lamps.fixed_lights = source.lights.len();
    source.road_lamps.fixed_shadows = shadow_start;
    let road = &r.road_lamp;
    let road_faces: usize = road
        .lights
        .iter()
        .map(|id| {
            let light = r.lights.iter().find(|l| l.id == *id).unwrap();
            usize::from(light.shadows.kind != 0)
        })
        .sum();
    if road.lights.len() != 1
        || source.lights.len() + source.road_lamps.capacity > MAX_LIGHTS
        || shadow_start + source.road_lamps.capacity * road_faces > SHADOW_SLOTS
    {
        source.state =
            RenderingState::Failed("Source road lamp window exceeds renderer capacity".into());
        return;
    }
    source.bound = true;
}
pub(crate) fn sync_control_visuals(
    drive: Res<Drive>,
    source: Res<SourceLighting>,
    mut poses: Query<&mut Transform>,
    mut nodes: Query<&mut Visibility>,
) {
    for &(index, entity) in &source.control_poses {
        if let Ok(mut pose) = poses.get_mut(entity) {
            let output = &drive.runtime.lighting.control_poses()[index].local_pose;
            pose.translation = Vec3::from_array(output.translation);
            pose.rotation = Quat::from_array(output.rotation);
        }
    }
    for &(index, entity) in &source.visibility {
        if let Ok(mut visibility) = nodes.get_mut(entity) {
            visibility.set_if_neq(
                if drive.runtime.lighting.visibility_outputs()[index].visible {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                },
            );
        }
    }
}
pub(crate) fn sync_lighting(
    reference: Res<RuntimeReference>,
    drive: Res<Drive>,
    mut source: ResMut<SourceLighting>,
    globals: Query<&GlobalTransform, Without<SourceLight>>,
    mut lights: Query<(
        &mut Transform,
        &mut GlobalTransform,
        &mut SourceLight,
        Option<&mut PointLight>,
        Option<&mut SpotLight>,
    )>,
) {
    let cycle = drive.runtime.lighting.cycle.output();
    source.uniform.ambient = Vec4::new(
        source_linear(cycle.ambient),
        source_linear(cycle.ambient),
        source_linear(cycle.ambient),
        cycle.fog_density,
    );
    source.uniform.fog = Vec4::from_array(cycle.fog_color.map(source_linear));
    let road_output = drive
        .runtime
        .lighting
        .road_lamp_output(&reference.0.lighting);
    for bound_index in 0..source.lights.len() {
        let bound = &source.lights[bound_index];
        let index = bound.index;
        let active = bound
            .item
            .map(|id| {
                drive
                    .runtime
                    .area
                    .items
                    .get(id)
                    .is_some_and(|item| item.inactive_pose.is_none())
            })
            .unwrap_or_else(|| {
                bound.body.is_none_or(|id| {
                    drive
                        .runtime
                        .area
                        .bodies
                        .get(&id)
                        .is_some_and(|handle| drive.runtime.physics.body_active(*handle))
                })
            });
        let record = &reference.0.lighting.lights[bound.source_index];
        let output = if bound.road {
            &road_output
        } else {
            &drive.runtime.lighting.light_outputs()[bound.source_index]
        };
        let Ok((mut pose, mut global, mut marker, point, spot)) = lights.get_mut(bound.entity)
        else {
            continue;
        };
        let mut next_pose = *pose;
        if record.kind == LightKind::Directional {
            let [x, y, z] = cycle.directional_euler_degrees.map(f32::to_radians);
            next_pose.rotation = Quat::from_euler(EulerRot::YXZ, -y, -x, z);
        } else if let Some(node) = bound.node {
            if let Ok(global) = globals.get(node) {
                next_pose = global.compute_transform();
            }
        }
        pose.set_if_neq(next_pose);
        global.set_if_neq(GlobalTransform::from(next_pose));
        let enabled = active && (source.warming || output.enabled);
        if marker.enabled != enabled {
            marker.enabled = enabled;
        }
        let shadows_enabled = enabled && marker.faces > 0;
        if let Some(mut light) = point {
            if light.shadow_maps_enabled != shadows_enabled {
                light.shadow_maps_enabled = shadows_enabled;
            }
        }
        if let Some(mut light) = spot {
            if light.shadow_maps_enabled != shadows_enabled {
                light.shadow_maps_enabled = shadows_enabled;
            }
        }
        let color = if record.kind == LightKind::Directional {
            cycle.directional_color
        } else {
            record.color
        };
        let intensity = if !active {
            0.
        } else if record.kind == LightKind::Directional {
            cycle.directional_intensity
        } else if output.enabled {
            output.intensity
        } else {
            0.
        };
        let cookie_from_world = if record.kind == LightKind::Directional {
            Mat4::from_quat(pose.rotation).inverse()
        } else if record.kind == LightKind::Spot {
            Mat4::from_cols_array_2d(&crate::lighting::spot_cookie_matrix(
                &crate::reference::Transform {
                    translation: pose.translation.to_array(),
                    rotation: pose.rotation.to_array(),
                    scale: pose.scale.to_array(),
                },
                record.range_meters,
                f32::from_bits(source.uniform.lights[index].mask.y),
            ))
        } else {
            Mat4::from_rotation_translation(pose.rotation, pose.translation).inverse()
        };
        let phase = if record.kind == LightKind::Directional {
            crate::lighting::directional_cookie_phase(
                [
                    cookie_from_world.row(0).truncate().to_array(),
                    cookie_from_world.row(1).truncate().to_array(),
                ],
                drive.runtime.area.origin,
                bound.pose.translation.to_array(),
                record.cookie_size,
            )
        } else {
            [0.; 2]
        };
        let gpu = &mut source.uniform.lights[index];
        gpu.cookie_rect.y = phase[0];
        gpu.cookie_rect.z = phase[1];
        gpu.color = Vec4::new(
            source_linear(color[0] * intensity),
            source_linear(color[1] * intensity),
            source_linear(color[2] * intensity),
            1.,
        );
        gpu.position_range = pose.translation.extend(record.range_meters);
        gpu.direction_kind = (pose.rotation * Vec3::NEG_Z).extend(match record.kind {
            LightKind::Directional => 0.,
            LightKind::Point => 1.,
            _ => 2.,
        });
        gpu.cookie_from_world = cookie_from_world;
    }
}
pub fn lighting_readiness(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut source: ResMut<SourceLighting>,
    visuals: Res<ReferenceVisuals>,
) {
    if matches!(source.state, RenderingState::Failed(_)) {
        return;
    }
    for (_, name, handle) in &source.textures {
        if let Some(LoadState::Failed(error)) = server.get_load_state(handle.id()) {
            source.state = RenderingState::Failed(format!("Cannot load {name}: {error}"));
            return;
        }
        if !server.is_loaded_with_dependencies(handle.id()) {
            return;
        }
    }
    if let Some(LoadState::Failed(error)) = server.get_load_state(source.shader.id()) {
        source.state =
            RenderingState::Failed(format!("Cannot load shaders/source_surface.wgsl: {error}"));
        return;
    }
    if !source.materials_ready {
        let Some(capture) = &source.material_loads else {
            return;
        };
        let pending = capture.0.lock().unwrap();
        if let Some(missing) = pending.iter().find(|material| material.handle.is_none()) {
            if visuals
                .handles
                .iter()
                .all(|(_, handle)| server.is_loaded_with_dependencies(handle.id()))
            {
                let error = format!(
                    "Source lamp material {}:{} has no PBR subasset",
                    missing.asset.0, missing.index
                );
                drop(pending);
                source.state = RenderingState::Failed(error);
            }
            return;
        }
        for material in pending.iter() {
            let handle = material.handle.as_ref().unwrap();
            if let Some(LoadState::Failed(error)) = server.get_load_state(handle.id()) {
                drop(pending);
                source.state = RenderingState::Failed(format!("Source lamp material: {error}"));
                return;
            }
            if !server.is_loaded_with_dependencies(handle.id()) {
                return;
            }
        }
        let materials = pending
            .iter()
            .map(|material| AuthoredMaterial {
                asset: material.asset,
                index: material.index,
                handle: material.handle.as_ref().unwrap().clone(),
            })
            .collect();
        drop(pending);
        source.materials = materials;
        source.materials_ready = true;
    }
    if source
        .lamp_materials
        .iter()
        .any(|target| target.current.is_none())
    {
        return;
    }
    if !source.sky.bound() {
        return;
    }
    if let Some(LoadState::Failed(error)) = server.get_load_state(source.sky.shader.id()) {
        source.state =
            RenderingState::Failed(format!("Cannot load shaders/source_sky.wgsl: {error}"));
        return;
    }
    if source.bound && source.materials_prepared {
        let state = source.status.lock().unwrap().clone();
        if source.warming && state == RenderingState::Ready {
            for light in &source.lights {
                commands
                    .entity(light.entity)
                    .remove::<bevy::camera::visibility::NoFrustumCulling>();
            }
            for (entity, _) in source.warmups.drain(..) {
                commands.entity(entity).despawn();
            }
            super::world::finish_road_lamp_warmup(&mut commands, &mut source);
            source.stations.finish_warmup(&mut commands);
            source.warming = false;
            return;
        }
        source.state = state;
    }
}
fn init_gpu(mut commands: Commands, device: Res<RenderDevice>, queue: Res<RenderQueue>) {
    let limits = device.limits();
    let bytes = LightingUniform::min_size().get();
    let supported = limits.max_texture_dimension_2d >= ATLAS_SIZE
        && u64::from(limits.max_uniform_buffer_binding_size) >= bytes;
    let state = if supported {
        RenderingState::Loading
    } else {
        RenderingState::Failed(format!(
            "Source lighting GPU limits: needs {ATLAS_SIZE}px atlas and {bytes}-byte uniform; device has {}px and {} bytes",
            limits.max_texture_dimension_2d, limits.max_uniform_buffer_binding_size
        ))
    };
    let status = Arc::new(Mutex::new(state));
    capture_gpu_errors(&device, status.clone());
    info!(
        "Source GPU: {}px texture limit, {}-byte uniform limit; atlas {ATLAS_SIZE}px, uniform {bytes} bytes",
        limits.max_texture_dimension_2d, limits.max_uniform_buffer_binding_size
    );
    let atlas = device.create_texture(&TextureDescriptor {
        label: Some("source shadow atlas"),
        size: Extent3d {
            width: if supported { ATLAS_SIZE } else { 1 },
            height: if supported { ATLAS_SIZE } else { 1 },
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Depth32Float,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let atlas_view = atlas.create_view(&default());
    let attenuation = device.create_texture_with_data(
        &queue,
        &TextureDescriptor {
            label: Some("source attenuation"),
            size: Extent3d {
                width: 1024,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::R8Unorm,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        },
        TextureDataOrder::LayerMajor,
        &crate::lighting::attenuation_table(),
    );
    let mut uniform = UniformBuffer::default();
    if supported {
        uniform.write_buffer(&device, &queue);
    }
    commands.insert_resource(LightingGpu {
        uniform,
        atlas,
        atlas_view,
        shadow_sampler: device.create_sampler(&SamplerDescriptor {
            compare: Some(CompareFunction::GreaterEqual),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..default()
        }),
        hard_shadow_sampler: device.create_sampler(&SamplerDescriptor {
            compare: Some(CompareFunction::GreaterEqual),
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            ..default()
        }),
        cookie_sampler: device.create_sampler(&SamplerDescriptor {
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..default()
        }),
        cookies: None,
        attenuation_view: attenuation.create_view(&default()),
        source_shader: None,
        sky_shader: None,
        status,
    });
}
fn capture_gpu_errors(device: &RenderDevice, status: Arc<Mutex<RenderingState>>) {
    device
        .wgpu_device()
        .on_uncaptured_error(Arc::new(move |error| {
            let mut state = status.lock().unwrap();
            if !matches!(*state, RenderingState::Failed(_)) {
                *state = RenderingState::Failed(format!("Source GPU validation: {error}"));
            }
        }));
}
fn extract_lighting(
    source: Extract<Option<Res<SourceLighting>>>,
    mut output: ResMut<ExtractedLighting>,
) {
    if let Some(source) = source.as_ref() {
        output.uniform = source.uniform;
        output.prepared = source.materials_prepared;
        output.warming = source.warming;
        if source.warming && source.materials_prepared && output.warmups.is_empty() {
            output.warmups.extend(
                source
                    .warmups
                    .iter()
                    .map(|(entity, shadow)| ((*entity).into(), *shadow)),
            );
        } else if !source.warming {
            output.warmups.clear();
        }
        output.shader = Some(source.shader.id());
        output.sky_shader = Some(source.sky.shader.id());
        output.cookies = source
            .cookie_handles
            .as_ref()
            .map(|handles| std::array::from_fn(|i| handles[i].id()));
        if output.status.is_none() {
            output.status = Some(source.status.clone());
        }
    }
}
fn upload_lighting(
    extracted: Res<ExtractedLighting>,
    mut gpu: ResMut<LightingGpu>,
    images: Res<RenderAssets<GpuImage>>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
) {
    if let Some(status) = &extracted.status {
        if !Arc::ptr_eq(status, &gpu.status) {
            if let RenderingState::Failed(error) = &*gpu.status.lock().unwrap() {
                *status.lock().unwrap() = RenderingState::Failed(error.clone());
            }
            gpu.status = status.clone();
            capture_gpu_errors(&device, status.clone());
        }
    }
    if matches!(*gpu.status.lock().unwrap(), RenderingState::Failed(_)) {
        return;
    }
    gpu.source_shader = extracted.shader;
    gpu.sky_shader = extracted.sky_shader;
    let shadows = gpu.uniform.get().shadows;
    gpu.uniform.set(extracted.uniform);
    gpu.uniform.get_mut().shadows = shadows;
    gpu.uniform.write_buffer(&device, &queue);
    if gpu.cookies.is_none() {
        if let Some(ids) = extracted.cookies {
            if ids.iter().all(|id| images.get(*id).is_some()) {
                gpu.cookies = Some(ids.map(|id| {
                    let image = images.get(id).unwrap();
                    (image.texture_view.clone(), image.sampler.clone())
                }));
            }
        }
    }
}
fn check_pipelines(
    cache: Res<PipelineCache>,
    gpu: Res<LightingGpu>,
    extracted: Res<ExtractedLighting>,
    material_pipelines: Res<bevy::pbr::SpecializedMaterialPipelineCache>,
    shadows: Res<bevy::pbr::SpecializedShadowMaterialPipelineCache>,
    mut last: Local<(usize, usize)>,
) {
    let mut ready = gpu.cookies.is_some() && extracted.prepared;
    let mut count = 0;
    let mut sky_count = 0;
    let mut shadow_count = 0;
    let mut inspect = |state: &CachedPipelineState| match state {
        CachedPipelineState::Ok(_) => {}
        CachedPipelineState::Err(
            bevy::shader::ShaderCacheError::ShaderNotLoaded(_)
            | bevy::shader::ShaderCacheError::ShaderImportNotYetAvailable,
        ) => ready = false,
        CachedPipelineState::Err(error) => {
            ready = false;
            *gpu.status.lock().unwrap() =
                RenderingState::Failed(format!("Source shader pipeline: {error}"));
        }
        _ => ready = false,
    };
    for pipeline in cache.pipelines() {
        if let PipelineDescriptor::RenderPipelineDescriptor(descriptor) = &pipeline.descriptor {
            if descriptor
                .fragment
                .as_ref()
                .is_some_and(|f| Some(f.shader.id()) == gpu.source_shader)
            {
                count += 1;
                inspect(&pipeline.state);
            }
            if descriptor
                .fragment
                .as_ref()
                .is_some_and(|f| Some(f.shader.id()) == gpu.sky_shader)
            {
                sky_count += 1;
                inspect(&pipeline.state);
            }
        }
    }
    for view in shadows.values() {
        for (pipeline, _) in view.values() {
            shadow_count += 1;
            inspect(cache.get_render_pipeline_state(*pipeline));
        }
    }
    if extracted.warming {
        for (entity, shadow) in &extracted.warmups {
            if *shadow {
                if shadows
                    .values()
                    .filter(|view| view.contains_key(entity))
                    .count()
                    != extracted.uniform.counts.y as usize
                {
                    ready = false;
                }
            } else if !material_pipelines
                .values()
                .any(|view| view.contains_key(entity))
            {
                ready = false;
            }
        }
    }
    if *last != (count, shadow_count) {
        info!("Source pipelines: {count} receiver, {shadow_count} shadow draws");
        *last = (count, shadow_count);
    }
    if ready
        && count > 0
        && sky_count > 0
        && shadow_count > 0
        && !matches!(*gpu.status.lock().unwrap(), RenderingState::Failed(_))
    {
        *gpu.status.lock().unwrap() = RenderingState::Ready;
    }
}

/// Register one source spot in a reserved slot. Shader and atlas allocations remain fixed.
pub(crate) fn register_road_light(
    source: &mut SourceLighting,
    commands: &mut Commands,
    reference: &LightingRecord,
    slot: usize,
    node: Entity,
    enabled: bool,
) -> Entity {
    let source_index = reference
        .lights
        .iter()
        .position(|l| l.id == reference.road_lamp.lights[0])
        .unwrap();
    let light = &reference.lights[source_index];
    let index = source.road_lamps.fixed_lights + slot;
    let shadow_start = source.road_lamps.fixed_shadows + slot;
    let faces = usize::from(light.shadows.kind != 0);
    let mut entity = commands.spawn((
        SourceLight {
            index,
            shadow_start,
            faces,
            enabled,
        },
        Transform::default(),
        SpotLight {
            intensity: 0.,
            range: light.range_meters,
            outer_angle: light.full_cone_degrees.to_radians() * 0.5,
            inner_angle: light.inner_cone_degrees.to_radians() * 0.5,
            shadow_maps_enabled: enabled && faces > 0,
            shadow_map_near_z: light.shadows.near_plane,
            shadow_depth_bias: light.shadows.bias,
            shadow_normal_bias: light.shadows.normal_bias,
            ..default()
        },
        (0..32)
            .filter(|layer| light.culling_mask & (1u32 << layer) != 0)
            .flat_map(|layer| [layer, layer + 32])
            .collect::<RenderLayers>(),
    ));
    if source.warming {
        entity.insert(bevy::camera::visibility::NoFrustumCulling);
    }
    let entity = entity.id();
    source.lights.push(BoundLight {
        index,
        road: true,
        item: None,
        body: None,
        id: light.id,
        source_index,
        entity,
        node: Some(node),
        pose: Transform::default(),
    });
    let a = light.full_cone_degrees * 0.5 / 360.;
    let half = (a + a) * std::f32::consts::PI;
    source.uniform.lights[index] = GpuLight {
        cookie_rect: Vec4::new(2., 0., 0., 0.),
        shadow: Vec4::new(
            shadow_start as f32,
            faces as f32,
            light.shadows.strength,
            light.cookie_size,
        ),
        mask: UVec4::new(
            light.culling_mask,
            (half.cos() / half.sin()).to_bits(),
            light.shadows.kind as u32,
            0,
        ),
        ..default()
    };
    source.uniform.counts.x = source.uniform.counts.x.max(index as u32 + 1);
    source.uniform.counts.y = source.uniform.counts.y.max((shadow_start + faces) as u32);
    entity
}

pub(crate) fn remove_road_light(
    source: &mut SourceLighting,
    commands: &mut Commands,
    entity: Entity,
) {
    if let Some(bound) = source.lights.iter().find(|light| light.entity == entity) {
        source.uniform.lights[bound.index] = GpuLight::default();
    }
    source.lights.retain(|light| light.entity != entity);
    commands.entity(entity).despawn();
    source.uniform.counts.x = source
        .lights
        .iter()
        .map(|light| light.index + 1)
        .max()
        .unwrap_or(0) as u32;
    source.uniform.counts.y = source
        .lights
        .iter()
        .map(|light| {
            if light.road {
                source.road_lamps.fixed_shadows + light.index - source.road_lamps.fixed_lights + 1
            } else {
                source.road_lamps.fixed_shadows
            }
        })
        .max()
        .unwrap_or(0) as u32;
}
