//! Ordinary 2D shadow atlas using Bevy's material shadow phases.
use super::lighting::{ATLAS_SIZE, GpuShadow, LightingGpu, SHADOW_SLOTS, SourceLight, TILE_SIZE};
use bevy::camera::{
    primitives::{CUBE_MAP_FACES, CubemapFrusta, Frustum},
    visibility::RenderLayers,
};
use bevy::core_pipeline::{core_3d::CORE_3D_DEPTH_FORMAT, schedule::RootNonCameraView};
use bevy::ecs::{self as bevy_ecs, schedule::ScheduleLabel};
use bevy::pbr::{
    ExtractedDirectionalLight, ExtractedPointLight, LightEntity, LightKeyCache, MeshPipelineKey,
    Shadow, SpecializedShadowMaterialPipelineCache, SpecializedShadowMaterialViewPipelineCache,
    prepare_lights,
};
use bevy::prelude::*;
use bevy::render::{
    Extract, ExtractSchedule, Render, RenderApp, RenderStartup, RenderSystems,
    batching::gpu_preprocessing::GpuPreprocessingMode,
    render_phase::{BinnedRenderPhase, ViewBinnedRenderPhases},
    render_resource::*,
    renderer::RenderContext,
    sync_world::{MainEntity, RenderEntity},
    view::{ExtractedView, NoIndirectDrawing, RetainedViewEntity},
};

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct SourceShadowPass;
struct Slot {
    entity: Entity,
    key: Option<RetainedViewEntity>,
    active: bool,
    previous_active: bool,
    phase: Option<BinnedRenderPhase<Shadow>>,
    pipeline: Option<SpecializedShadowMaterialViewPipelineCache>,
    light_key: Option<MeshPipelineKey>,
}
#[derive(Resource)]
struct AtlasViews {
    slots: Vec<Slot>,
}
pub struct SourceShadowPlugin;
impl Plugin for SourceShadowPlugin {
    fn build(&self, app: &mut App) {
        let render = app.sub_app_mut(RenderApp);
        render
            .add_systems(RenderStartup, init_views)
            .add_systems(
                ExtractSchedule,
                extract_source_lights.after(bevy::pbr::extract_lights),
            )
            .add_systems(
                Render,
                reserve_views
                    .before(prepare_lights)
                    .in_set(RenderSystems::CreateViews),
            )
            .add_systems(
                Render,
                prepare_views
                    .after(prepare_lights)
                    .in_set(RenderSystems::CreateViews),
            )
            .add_systems(SourceShadowPass, draw_atlas);
    }
}
fn init_views(mut commands: Commands) {
    let slots = (0..SHADOW_SLOTS)
        .map(|_| Slot {
            entity: commands.spawn_empty().id(),
            key: None,
            active: false,
            previous_active: false,
            phase: None,
            pipeline: None,
            light_key: None,
        })
        .collect();
    commands.insert_resource(AtlasViews { slots });
    commands.spawn(RootNonCameraView(SourceShadowPass.intern()));
}
fn extract_source_lights(
    mut commands: Commands,
    lights: Extract<Query<(&RenderEntity, &SourceLight)>>,
) {
    for (entity, marker) in &lights {
        commands.entity(entity.id()).insert(*marker);
    }
}
fn reserve_views(
    mut views: ResMut<AtlasViews>,
    mut phases: ResMut<ViewBinnedRenderPhases<Shadow>>,
    mut keys: ResMut<LightKeyCache>,
    mut pipelines: ResMut<SpecializedShadowMaterialPipelineCache>,
    mut points: Query<&mut ExtractedPointLight, With<SourceLight>>,
    mut suns: Query<&mut ExtractedDirectionalLight, With<SourceLight>>,
) {
    for slot in &mut views.slots {
        if let Some(key) = slot.key {
            super::shadow_cache::retain_removed(&mut slot.phase, phases.remove(&key));
            super::shadow_cache::retain_removed(&mut slot.light_key, keys.remove(&key));
            super::shadow_cache::retain_removed(&mut slot.pipeline, pipelines.remove(&key));
        }
    }
    // Main-world flags still drive caster visibility. The atlas owns GPU allocation.
    for mut light in &mut points {
        light.shadow_maps_enabled = false;
    }
    for mut light in &mut suns {
        light.shadow_maps_enabled = false;
    }
}
fn prepare_views(
    mut commands: Commands,
    mut views: ResMut<AtlasViews>,
    mut phases: ResMut<ViewBinnedRenderPhases<Shadow>>,
    mut keys: ResMut<LightKeyCache>,
    mut pipelines: ResMut<SpecializedShadowMaterialPipelineCache>,
    points: Query<(
        Entity,
        &MainEntity,
        &SourceLight,
        &ExtractedPointLight,
        &RenderLayers,
        Option<&CubemapFrusta>,
        Option<&Frustum>,
    )>,
    suns: Query<(
        Entity,
        &MainEntity,
        &SourceLight,
        &ExtractedDirectionalLight,
        &RenderLayers,
    )>,
    cameras: Query<&MainEntity>,
    mut gpu: ResMut<LightingGpu>,
) {
    for slot in &mut views.slots {
        slot.previous_active = slot.active;
        slot.active = false;
    }
    let mut set_view = |slot_index: usize,
                        key: RetainedViewEntity,
                        owner: LightEntity,
                        world: GlobalTransform,
                        projection: Mat4,
                        clip: Mat4,
                        frustum: Frustum,
                        layers: &RenderLayers,
                        normal_bias: f32,
                        depth_bias: f32| {
        let slot = &mut views.slots[slot_index];
        slot.key = Some(key);
        slot.active = true;
        if let Some(phase) = slot.phase.take() {
            phases.insert(key, phase);
        }
        if let Some(light_key) = slot.light_key.take() {
            keys.insert(key, light_key);
        }
        if let Some(pipeline) = slot.pipeline.take() {
            pipelines.insert(key, pipeline);
        }
        phases.prepare_for_new_frame(key, GpuPreprocessingMode::None);
        let columns = ATLAS_SIZE / TILE_SIZE;
        let x = slot_index as u32 % columns * TILE_SIZE;
        let y = slot_index as u32 / columns * TILE_SIZE;
        commands.entity(slot.entity).insert((
            owner,
            ExtractedView {
                retained_view_entity: key,
                viewport: UVec4::new(x + 1, y + 1, TILE_SIZE - 2, TILE_SIZE - 2),
                world_from_view: world,
                clip_from_view: projection,
                clip_from_world: Some(clip),
                target_format: CORE_3D_DEPTH_FORMAT,
                color_grading: default(),
                invert_culling: false,
            },
            frustum,
            layers.clone(),
            NoIndirectDrawing,
        ));
        gpu.uniform.get_mut().shadows[slot_index] = GpuShadow {
            clip_from_world: clip,
            rect: Vec4::new(
                (TILE_SIZE - 2) as f32 / ATLAS_SIZE as f32,
                (TILE_SIZE - 2) as f32 / ATLAS_SIZE as f32,
                (x + 1) as f32 / ATLAS_SIZE as f32,
                (y + 1) as f32 / ATLAS_SIZE as f32,
            ),
            bias: Vec4::new(normal_bias, depth_bias, 0., 0.),
        };
    };
    for (entity, main, marker, light, layers) in &suns {
        if !marker.enabled {
            continue;
        }
        for (camera, cascades) in &light.cascades {
            let Some(frusta) = light.frusta.get(camera) else {
                continue;
            };
            for (index, (cascade, frustum)) in cascades.iter().zip(frusta).take(4).enumerate() {
                let key = RetainedViewEntity::new(
                    *main,
                    Some(*cameras.get(*camera).expect("extracted camera identity")),
                    index as u32,
                );
                set_view(
                    marker.shadow_start + index,
                    key,
                    LightEntity::Directional {
                        light_entity: entity,
                        cascade_index: index,
                    },
                    GlobalTransform::from(cascade.world_from_cascade),
                    cascade.clip_from_cascade,
                    cascade.clip_from_world,
                    *frustum,
                    layers,
                    cascade.texel_size * light.shadow_normal_bias,
                    light.shadow_depth_bias,
                );
            }
            break;
        }
    }
    for (entity, main, marker, light, layers, cube, frustum) in &points {
        if !marker.enabled || marker.faces == 0 {
            continue;
        }
        if let Some((_, angle)) = light.spot_light_angles {
            let Some(frustum) = frustum else {
                continue;
            };
            let world: GlobalTransform =
                bevy::light::spot_light_world_from_view(&light.transform).into();
            let projection = bevy::light::spot_light_clip_from_view(angle, light.shadow_map_near_z);
            let clip = projection * world.to_matrix().inverse();
            set_view(
                marker.shadow_start,
                RetainedViewEntity::new(*main, None, 0),
                LightEntity::Spot {
                    light_entity: entity,
                },
                world,
                projection,
                clip,
                *frustum,
                layers,
                light.shadow_normal_bias,
                light.shadow_depth_bias,
            );
        } else if let Some(cube) = cube {
            let projection = Mat4::perspective_infinite_reverse_rh(
                std::f32::consts::FRAC_PI_2,
                1.,
                light.shadow_map_near_z,
            );
            for (face, basis) in CUBE_MAP_FACES.iter().enumerate() {
                let world = GlobalTransform::from(
                    Transform::from_translation(light.transform.translation())
                        .looking_to(basis.target, basis.up),
                );
                let clip = projection * world.to_matrix().inverse();
                set_view(
                    marker.shadow_start + face,
                    RetainedViewEntity::new(*main, None, face as u32),
                    LightEntity::Point {
                        light_entity: entity,
                        face_index: face,
                    },
                    world,
                    projection,
                    clip,
                    cube.frusta[face],
                    layers,
                    light.shadow_normal_bias,
                    light.shadow_depth_bias,
                );
            }
        }
    }
    for slot in &views.slots {
        if slot.previous_active && !slot.active {
            commands.entity(slot.entity).remove::<LightEntity>();
        }
    }
}
fn draw_atlas(
    world: &World,
    views: Res<AtlasViews>,
    gpu: Res<LightingGpu>,
    phases: Res<ViewBinnedRenderPhases<Shadow>>,
    mut ctx: RenderContext,
) {
    if matches!(
        *gpu.status.lock().unwrap(),
        super::lighting::RenderingState::Failed(_)
    ) {
        return;
    }
    let mut pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("source shadow atlas"),
        color_attachments: &[],
        depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
            view: &gpu.atlas_view,
            depth_ops: Some(Operations {
                load: LoadOp::Clear(0.),
                store: StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    for (index, slot) in views.slots.iter().enumerate() {
        if !slot.active {
            continue;
        }
        let Some(phase) = slot.key.and_then(|key| phases.get(&key)) else {
            continue;
        };
        let columns = ATLAS_SIZE / TILE_SIZE;
        let x = index as u32 % columns * TILE_SIZE + 1;
        let y = index as u32 / columns * TILE_SIZE + 1;
        pass.set_viewport(
            x as f32,
            y as f32,
            (TILE_SIZE - 2) as f32,
            (TILE_SIZE - 2) as f32,
            0.,
            1.,
        );
        pass.set_scissor_rect(x, y, TILE_SIZE - 2, TILE_SIZE - 2);
        if let Err(error) = phase.render(&mut pass, world, slot.entity) {
            *gpu.status.lock().unwrap() = super::lighting::RenderingState::Failed(format!(
                "Shadow atlas tile {index}: {error:?}"
            ));
        }
    }
}
