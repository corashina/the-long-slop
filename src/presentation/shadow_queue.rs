//! Cached source shadow specialization and offscreen mesh queueing.
use super::surface_views::VIEW_COUNT;
use super::{
    caster::{CasterPasses, CasterUniform, CasterViews},
    lighting_scene::SourceSceneLighting,
    materials::RenderingState,
    shadow_phase::{self, ShadowDrawState, SourceShadow},
    shadow_scene::ShadowScene,
    source_lighting::SourceLightingStatus,
    surface_views::SourceSurfaceView,
    view_frame::SourceViewFrame,
};
use bevy::material::{
    MaterialProperties,
    key::{ErasedMaterialPipelineKey, ErasedMeshPipelineKey},
};
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{
    MeshPipeline, MeshPipelineKey, PreparedMaterial, RenderMaterialInstances, RenderMeshInstances,
    ViewKeyCache, alpha_mode_pipeline_key, base_specialize,
};
use bevy::render::{
    erased_render_asset::ErasedRenderAssets,
    mesh::{RenderMesh, allocator::MeshAllocator},
    render_asset::RenderAssets,
    render_phase::*,
    render_resource::*,
    renderer::{RenderDevice, RenderQueue},
    sync_world::MainEntity,
    view::{ExtractedView, RetainedViewEntity},
};
use bevy::{ecs as bevy_ecs, prelude::*};
use std::sync::Arc;
struct CachedSpecialization {
    mesh_key: MeshPipelineKey,
    layout: MeshVertexBufferLayoutRef,
    properties: Arc<MaterialProperties>,
    base: CachedRenderPipelineId,
}
#[derive(Resource)]
pub struct ShadowQueues {
    query: QueryState<(Entity, &'static SourceSurfaceView, &'static ExtractedView)>,
    specializations: Vec<CachedSpecialization>,
    pub ready: [bool; VIEW_COUNT],
    pub frame: u32,
}
impl FromWorld for ShadowQueues {
    fn from_world(world: &mut World) -> Self {
        Self {
            query: world.query(),
            specializations: Vec::new(),
            ready: [false; VIEW_COUNT],
            frame: 0,
        }
    }
}
impl ShadowQueues {
    pub fn cached_pipelines(&self) -> usize {
        self.specializations.len()
    }
}
fn base_pipeline(
    world: &mut World,
    state: &mut ShadowQueues,
    entity: MainEntity,
    view_key: MeshPipelineKey,
) -> Result<Option<(CachedRenderPipelineId, bool)>, String> {
    let Some(mesh_id) = world
        .resource::<RenderMeshInstances>()
        .mesh_asset_id(entity)
    else {
        return Ok(None);
    };
    let Some(instance) = world
        .resource::<RenderMaterialInstances>()
        .instances
        .get(&entity)
    else {
        return Ok(None);
    };
    let material_type = instance.asset_id.type_id();
    let Some(mesh) = world.resource::<RenderAssets<RenderMesh>>().get(mesh_id) else {
        return Ok(None);
    };
    let Some(material) = world
        .resource::<ErasedRenderAssets<PreparedMaterial>>()
        .get(instance.asset_id)
    else {
        return Ok(None);
    };
    let Some(slabs) = world.resource::<MeshAllocator>().mesh_slabs(&mesh_id) else {
        return Ok(None);
    };
    let indexed = slabs.index_slab_id.is_some();
    let bits: MeshPipelineKey = material.properties.mesh_pipeline_key_bits.downcast();
    let key = view_key
        | MeshPipelineKey::from_bits_retain(mesh.key_bits.bits())
        | bits
        | alpha_mode_pipeline_key(
            material.properties.alpha_mode,
            &Msaa::from_samples(view_key.msaa_samples()),
        );
    if let Some(entry) = state.specializations.iter().find(|entry| {
        entry.mesh_key == key
            && entry.layout == mesh.layout
            && Arc::ptr_eq(&entry.properties, &material.properties)
    }) {
        return Ok(Some((entry.base, indexed)));
    }
    if state.specializations.len() == state.specializations.capacity() {
        return Err("Source shadow specialization capacity was exceeded".into());
    }
    // Full immutable material key is passed on cache misses. No Arc clone occurs on steady hits.
    let properties = material.properties.clone();
    let layout = mesh.layout.clone();
    let full = ErasedMaterialPipelineKey {
        type_id: material_type,
        mesh_key: ErasedMeshPipelineKey::new(key),
        material_key: properties.material_key.clone(),
    };
    let base = base_specialize(world, full, &layout, &properties)
        .map_err(|e| format!("Source shadow specialization: {e}"))?;
    state.specializations.push(CachedSpecialization {
        mesh_key: key,
        layout,
        properties,
        base,
    });
    Ok(Some((base, indexed)))
}
pub fn queue(world: &mut World) {
    world.resource_scope(|world, mut state: Mut<ShadowQueues>| {
        state.ready = [false; VIEW_COUNT];
        state.frame = 0;
        if matches!(
            *world.resource::<SourceLightingStatus>().0.lock().unwrap(),
            RenderingState::Failed(_)
        ) {
            return;
        }
        world.resource_scope(|world, scene: Mut<ShadowScene>| {
            if scene.frame == 0 || scene.frame != world.resource::<SourceViewFrame>().frame {
                return;
            }
            if state.specializations.capacity() < scene.capacity() * VIEW_COUNT {
                state
                    .specializations
                    .reserve_exact(scene.capacity() * VIEW_COUNT);
            }
            let mut views: [Option<(Entity, RetainedViewEntity, MeshPipelineKey)>; VIEW_COUNT] =
                [None; VIEW_COUNT];
            for (entity, source, view) in state.query.iter(world) {
                if world.resource::<SourceViewFrame>().views[source.index].is_none() {
                    continue;
                }
                let Some(key) = world
                    .resource::<ViewKeyCache>()
                    .get(&view.retained_view_entity)
                    .copied()
                else {
                    return;
                };
                views[source.index] = Some((entity, view.retained_view_entity, key));
            }
            let draw = shadow_phase::draw_id(world.resource::<DrawFunctions<SourceShadow>>());
            world.resource_scope(
                |world, mut phases: Mut<ViewSortedRenderPhases<SourceShadow>>| {
                    for (index, view) in views.into_iter().enumerate() {
                        let Some((entity, retained, key)) = view else {
                            continue;
                        };
                        if !phases.contains_key(&retained) {
                            phases.insert(retained, shadow_phase::reserved_phase(scene.capacity()));
                            world.entity_mut(entity).insert(ShadowDrawState::default());
                        }
                        let phase = phases.get_mut(&retained).unwrap();
                        if phase.items.capacity() < scene.capacity() {
                            phase
                                .items
                                .reserve(scene.capacity().saturating_sub(phase.items.len()));
                        }
                        phase.clear();
                        let mut ready = true;
                        for (item, main) in scene.entities.iter().enumerate() {
                            let mask = scene.masks[index][item];
                            if mask == 0 {
                                continue;
                            }
                            let main = MainEntity::from(*main);
                            let base = match base_pipeline(world, &mut state, main, key) {
                                Ok(Some(v)) => v,
                                Ok(None) => {
                                    ready = false;
                                    continue;
                                }
                                Err(e) => {
                                    *world.resource::<SourceLightingStatus>().0.lock().unwrap() =
                                        RenderingState::Failed(e);
                                    return;
                                }
                            };
                            let casters = world.resource::<CasterPasses>();
                            let Some(pipeline) = casters.pipeline(base.0, true) else {
                                ready = false;
                                continue;
                            };
                            if world
                                .resource::<PipelineCache>()
                                .get_render_pipeline(pipeline)
                                .is_none()
                            {
                                ready = false;
                            }
                            if let Err(e) = shadow_phase::add_candidate(
                                phase,
                                SourceShadow {
                                    entity: (Entity::PLACEHOLDER, main),
                                    pipeline,
                                    draw,
                                    split_mask: mask,
                                    indexed: base.1,
                                    batch_range: 0..1,
                                    extra_index: PhaseItemExtraIndex::None,
                                },
                            ) {
                                *world.resource::<SourceLightingStatus>().0.lock().unwrap() =
                                    RenderingState::Failed(e.into());
                                return;
                            }
                        }
                        state.ready[index] = ready;
                    }
                },
            );
            state.frame = scene.frame;
        });
    });
}
/// The native light camera negates its forward column before inversion.
pub(crate) fn cascade_projection(
    cascade: &crate::reference::shadows::ShadowCascade,
    light_basis: [Vec3; 3],
) -> (Mat4, f32) {
    let position = Vec3::from_array(cascade.light_position);
    let source_world = Mat4::from_cols(
        light_basis[0].extend(0.),
        light_basis[1].extend(0.),
        (-light_basis[2]).extend(0.),
        position.extend(1.),
    );
    let [width, height] = cascade.width_height;
    let mut p = Mat4::orthographic_rh_gl(
        -width * 0.5,
        width * 0.5,
        -height * 0.5,
        height * 0.5,
        cascade.near,
        cascade.far,
    )
    .to_cols_array_2d();
    for c in &mut p {
        c[1] = -c[1];
        c[2] = 0.5 * c[3] - 0.5 * c[2];
    }
    (
        Mat4::from_cols_array_2d(&p)
            * source_world.inverse()
            * Mat4::from_scale(Vec3::new(1., 1., -1.)),
        p[2][2],
    )
}
fn cascade_world_from_view(
    cascade: &crate::reference::shadows::ShadowCascade,
    basis: [Vec3; 3],
) -> Mat4 {
    Mat4::from_scale(Vec3::new(1., 1., -1.))
        * Mat4::from_cols(
            basis[0].extend(0.),
            basis[1].extend(0.),
            (-basis[2]).extend(0.),
            Vec3::from_array(cascade.light_position).extend(1.),
        )
}
pub fn write_cascades(
    scene: Res<ShadowScene>,
    frame: Res<SourceViewFrame>,
    live: Res<SourceSceneLighting>,
    assets: Option<Res<super::lighting_assets::LightingRenderAssets>>,
    mut views: Query<(&mut SourceSurfaceView, &mut CasterViews)>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
) {
    if scene.frame == 0 || scene.frame != frame.frame {
        return;
    }
    let (Some(assets), Some(live)) = (assets, live.0) else {
        return;
    };
    let light = &assets.record.lights[0];
    let rotation = live.sun_rotation;
    let h = |v: Vec3| Vec3::new(v.x, v.y, -v.z);
    let basis = [
        h(rotation * Vec3::X),
        h(rotation * Vec3::Y),
        h(rotation * -Vec3::Z),
    ];
    for (mut view, mut slots) in &mut views {
        let Some(fit) = scene.fits[view.index] else {
            view.shadow_fade = Vec4::ZERO;
            view.shadow_fade_center = Vec4::ZERO;
            continue;
        };
        view.shadow_fade = Vec4::from_array(fit.fade);
        view.shadow_fade_center = Vec4::new(
            fit.fade_center[0],
            fit.fade_center[1],
            -fit.fade_center[2],
            fit.fade_center[3],
        );
        for (index, cascade) in fit.cascades.iter().enumerate() {
            let (projection, device_e22) = cascade_projection(cascade, basis);
            // Selected D3D reversed-depth orthographic device e22 = 1/(far-near).
            let depth_bias = -light.shadows.depth_bias * device_e22;
            let normal_bias = (light.shadows.normal_bias * 3.6500000953674316_f32)
                * cascade.texel_size[0].max(cascade.texel_size[1]);
            slots.write(
                index + 1,
                CasterUniform {
                    clip_from_world: projection,
                    world_from_view: cascade_world_from_view(cascade, basis),
                    view_from_world: cascade_world_from_view(cascade, basis).inverse(),
                    clip_from_view: projection * cascade_world_from_view(cascade, basis),
                    direction_to_light: (rotation * Vec3::Z).extend(0.),
                    bias: Vec4::new(depth_bias, 1., normal_bias, 0.),
                },
                &device,
                &queue,
            );
        }
    }
}
pub fn install(app: &mut App) {
    use bevy::camera::visibility::VisibilitySystems;
    use bevy::render::{ExtractSchedule, Render, RenderApp, RenderSystems};
    app.init_resource::<ShadowScene>()
        .add_systems(
            PostUpdate,
            super::shadow_scene::prepare_scene
                .after(VisibilitySystems::CheckVisibility)
                .before(VisibilitySystems::MarkNewlyHiddenEntitiesInvisible),
        )
        .add_plugins(SortedRenderPhasePlugin::<SourceShadow, MeshPipeline>::new(
            default(),
        ));
    let render = app.sub_app_mut(RenderApp);
    shadow_phase::register(render);
    render
        .init_resource::<ShadowScene>()
        .init_resource::<ShadowQueues>()
        .add_systems(ExtractSchedule, super::shadow_scene::extract_scene)
        .add_systems(Render, queue.in_set(RenderSystems::Queue))
        .add_systems(
            Render,
            write_cascades
                .in_set(RenderSystems::PrepareResources)
                .after(super::caster::prepare_views)
                .before(super::surface_views::prepare_views),
        );
}

/// Gates source dispatch before any pass consumes a stale view or partial pipeline set.
pub fn graph_ready(
    view: bevy::render::renderer::ViewQuery<(
        &SourceSurfaceView,
        Option<&super::shadow_collector::CollectorView>,
    )>,
    frame: Res<SourceViewFrame>,
    queues: Res<ShadowQueues>,
    cache: Res<PipelineCache>,
    surface: Res<super::surface_pass::SurfacePasses>,
    caster: Res<CasterPasses>,
    loading: Res<super::materials::MaterialLoading>,
    status: Res<SourceLightingStatus>,
) -> bool {
    let (view, collector) = view.into_inner();
    frame.frame != 0
        && frame.views[view.index].is_some()
        && queues.frame == frame.frame
        && queues.ready[view.index]
        && loading.is_ready()
        && surface.ready(&cache)
        && caster.ready(&cache)
        && collector.is_some_and(|c| c.prepared_frame == frame.frame && c.ready(&cache))
        && !matches!(*status.0.lock().unwrap(), RenderingState::Failed(_))
}
pub fn collector_encoded(
    view: bevy::render::renderer::ViewQuery<&super::shadow_collector::CollectorView>,
    frame: Res<SourceViewFrame>,
) -> bool {
    view.into_inner().encoded_frame() == frame.frame
}
