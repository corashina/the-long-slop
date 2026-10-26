//! Source candidate membership is independent of camera-visible phase lists.
use super::surface_views::VIEW_COUNT;
use super::{
    lighting_assets::LightingAssets,
    lighting_scene::{SourceSceneLighting, SourceWorldOrigin},
    materials::{RenderingState, SourceMaterials},
    readiness::RendererStateHandle,
    renderer_visibility::SourceMaterialWarmup,
    shadow_changes::{ShadowChanges, ShadowMaterials, ShadowMesh, ShadowMeshItem},
    view_frame::SourceViewFrame,
    world::{
        gpu_props::{GpuProps, NO_BATCH, world_bounds},
        instances::SourceInstances,
    },
};
use crate::reference::shadow_profile;
use crate::reference::{
    shadow_bounds::{BoundsKey, WorldBoundsCache},
    shadow_candidates::CandidateState,
    shadow_frame::{CasterInput, ShadowViewCache},
    shadow_planes::CullPlanes,
    shadow_scene::{CasterData, CasterIndex, CasterKey},
    shadows::{ShadowFit, ShadowFitInput, source_atlas_size},
};
use bevy::camera::visibility::{SetViewVisibility, ViewVisibility};
use bevy::{ecs as bevy_ecs, prelude::*};

#[derive(Resource, Default)]
pub struct ShadowScene {
    pub frame: u32,
    pub origin: [f64; 3],
    pub imported_capacity: usize,
    pub generated_capacity: usize,
    pub caster_data: CasterData<CasterKey>,
    pub masks: [Vec<u8>; VIEW_COUNT],
    pub fits: [Option<ShadowFit>; VIEW_COUNT],
    pub candidates: [usize; VIEW_COUNT],
    pub split_counts: [[usize; 4]; VIEW_COUNT],
    pub camera_visible: usize,
    pub shadow_only: usize,
    bounds: WorldBoundsCache<Entity>,
    index: CasterIndex<CasterKey>,
    /// GPU prop owner with its row membership and row revision.
    rows: Option<(u64, u64, u64)>,
    casters_ready: bool,
    views: [ShadowViewCache; VIEW_COUNT],
    mask_revisions: [u64; VIEW_COUNT],
}
impl ShadowScene {
    pub(crate) fn view_revision(
        &self,
        index: usize,
        frame: &SourceViewFrame,
    ) -> super::shadow_preparation::ViewRevision {
        super::shadow_preparation::ViewRevision {
            session: frame.session,
            scene: frame.scene,
            fit: self.mask_revisions[index],
        }
    }
    pub fn mask_revision(&self, index: usize) -> u64 {
        self.mask_revisions[index]
    }
    pub fn capacity(&self) -> usize {
        self.imported_capacity + self.generated_capacity
    }
    fn reserve(&mut self, imported: usize, generated: usize) {
        self.imported_capacity = imported;
        self.generated_capacity = generated;
        let count = self.capacity();
        self.caster_data.reserve(count);
        for masks in &mut self.masks {
            masks.reserve_exact(count.saturating_sub(masks.len()));
        }
    }
}
/// Runs after both camera culling systems and before visibility removal/extraction.
/// Only ViewVisibility changes; camera VisibleEntities retain their camera-only membership.
pub(super) fn prepare_scene(
    frame: Res<SourceViewFrame>,
    live: Res<SourceSceneLighting>,
    origin: Res<SourceWorldOrigin>,
    assets: Option<Res<LightingAssets>>,
    source: Res<SourceMaterials>,
    materials: ShadowMaterials,
    mut updates: ParamSet<(ShadowChanges, Query<&mut ViewVisibility>)>,
    mut scene: ResMut<ShadowScene>,
    status: Res<RendererStateHandle>,
    master: Res<super::lighting::SourceLighting>,
    camera: Res<super::renderer_visibility::CameraVisibility>,
    meshes: Query<ShadowMesh, (With<ViewVisibility>, Without<SourceMaterialWarmup>)>,
    instances: Res<SourceInstances>,
) {
    let mut timing = super::reflection_preparation::Timer::new(
        super::reflection_preparation::Stage::Shadows,
        &frame,
    );
    let _shadow_timing = shadow_profile::Timer::new(shadow_profile::Stage::Scene);
    scene.frame = 0;
    scene.fits = [None; VIEW_COUNT];
    scene.candidates = [0; VIEW_COUNT];
    scene.split_counts = [[0; 4]; VIEW_COUNT];
    let was_ready = scene.casters_ready;
    scene.casters_ready = false;
    if !frame.required_views_ready()
        || source.state != RenderingState::Ready
        || master.road_lamps.recipe.is_none()
        || status.failure().is_some()
    {
        return;
    }
    let (Some(assets), Some(live)) = (assets, live.0) else {
        return;
    };
    let mut changes = updates.p0();
    let mut rebuild = changes.removed() || !changes.warmup.is_empty();
    rebuild |= scene.bounds.begin(frame.session, frame.scene, origin.0) || !was_ready;
    let props = instances.props();
    let rows = (props.id(), props.membership(), props.revision());
    rebuild |= scene
        .rows
        .is_none_or(|(id, membership, _)| (id, membership) != (rows.0, rows.1));
    let rows_changed = scene.rows != Some(rows);
    scene.rows = Some(rows);
    let fail = |message: &str| {
        status.fail(message.into());
    };
    // Asset resources include same-frame edits, before AssetEvent delivery in Last.
    // A material edit refreshes eligibility without rebuilding unchanged membership.
    if !rebuild {
        if materials.changed() {
            for mesh in &meshes {
                match update_caster(&mut scene, &materials, mesh, false) {
                    Ok(true) => {}
                    Ok(false) => {
                        rebuild = true;
                        break;
                    }
                    Err(error) => {
                        fail(error);
                        return;
                    }
                }
            }
        } else {
            for entity in &changes.changed {
                let Ok(mesh) = meshes.get(entity) else {
                    continue;
                };
                match update_caster(&mut scene, &materials, mesh, false) {
                    Ok(true) => {}
                    Ok(false) => {
                        rebuild = true;
                        break;
                    }
                    Err(error) => {
                        fail(error);
                        return;
                    }
                }
            }
        }
        if (materials.changed() || rows_changed)
            && let Err(error) = update_rows(&mut scene, &materials, props)
        {
            fail(error);
            return;
        }
    }
    if rebuild {
        let imported = meshes.iter().count() + props.live() as usize;
        if imported > scene.imported_capacity {
            scene.reserve(imported, 0);
            scene.bounds.reserve(imported);
            scene.index.reserve(imported);
        }
        let scene = &mut *scene;
        scene.index.clear(&mut scene.caster_data);
        scene.bounds.clear();
        for mesh in &meshes {
            if let Err(error) = update_caster(scene, &materials, mesh, true) {
                fail(error);
                return;
            }
        }
        if let Err(error) = update_rows(scene, &materials, props) {
            fail(error);
            return;
        }
    }
    if let Err(error) = scene.caster_data.publish() {
        fail(error);
        return;
    }
    let light = &assets.record.lights[0];
    let rotation = live.sun_rotation;
    let h = |v: Vec3| [v.x, v.y, -v.z];
    let light_basis = [
        h(rotation * Vec3::X),
        h(rotation * Vec3::Y),
        h(rotation * -Vec3::Z),
    ];
    let scene = &mut *scene;
    for index in 0..VIEW_COUNT {
        let Some(v) = frame.views[index] else {
            scene.masks[index].clear();
            scene.views[index].clear();
            scene.mask_revisions[index] = scene.views[index].revision();
            continue;
        };
        timing.face(index);
        let atlas = source_atlas_size(v.native_extent.x, v.native_extent.y)
            .expect("validated source view extent");
        let w = v.source_world_from_view;
        let mut input = ShadowFitInput {
            frustum_points: v.frustum_points,
            camera_basis: [
                w.x_axis.truncate().to_array(),
                w.y_axis.truncate().to_array(),
                w.z_axis.truncate().to_array(),
            ],
            camera_position: w.w_axis.truncate().to_array(),
            light_basis,
            caster_min: [0.; 3],
            caster_max: [0.; 3],
            near: v.near,
            far: v.far,
            vertical_fov: v.fov_degrees,
            shadow_distance: v.shadow_distance,
            split_fractions: assets.record.shadows.split_fractions,
            tile_size: [atlas / 2; 2],
            quality_near_offset: assets.record.shadows.near_offset,
            shadow_strength: light.shadows.strength,
        };
        scene.masks[index].resize(scene.caster_data.casters.len(), 0);
        match scene.views[index].prepare(
            scene.caster_data.revision(),
            &scene.caster_data.casters,
            &mut scene.masks[index],
            CullPlanes {
                camera: v.source_camera_planes,
                shadow: v.source_shadow_planes,
            },
            light.mask,
            v.mask,
            &mut input,
        ) {
            Ok(Some(value)) => {
                scene.fits[index] = Some(value.fit);
                scene.candidates[index] = value.candidate_count;
                scene.split_counts[index] = value.split_counts;
            }
            Ok(None) => {}
            Err(error) => {
                fail(error);
                return;
            }
        }
        scene.mask_revisions[index] = scene.views[index].revision();
    }
    drop(changes);
    let mut visibility = updates.p1();
    scene.camera_visible = 0;
    scene.shadow_only = 0;
    for (i, key) in scene.caster_data.entities.iter().enumerate() {
        let CasterKey::Entity(entity) = key else {
            continue;
        };
        let Ok(mut visible) = visibility.get_mut(*entity) else {
            continue;
        };
        scene.camera_visible += usize::from(camera.entities.contains(entity));
        if scene
            .masks
            .iter()
            .any(|m| m.get(i).is_some_and(|mask| *mask != 0))
        {
            if !camera.entities.contains(entity) {
                scene.shadow_only += 1;
            }
            visible.set_visible();
        }
    }
    scene.casters_ready = true;
    scene.origin = origin.0;
    scene.frame = frame.frame;
}
/// Return false when eligibility changes the ordered membership.
fn update_caster(
    scene: &mut ShadowScene,
    materials: &ShadowMaterials,
    mesh: ShadowMeshItem<'_, '_>,
    rebuilding: bool,
) -> Result<bool, &'static str> {
    let key = CasterKey::Entity(mesh.entity);
    let Some(has_caster) = materials.has_caster(&mesh)? else {
        return Ok(!scene.index.contains(key));
    };
    let layer = if let Some(layers) = mesh.layers {
        let mut iter = layers.iter();
        let Some(layer) = iter.next() else {
            return Ok(!scene.index.contains(key));
        };
        if layer >= 32 || iter.next().is_some() {
            return Err("Source shadow renderer requires one source layer");
        }
        layer as u8
    } else {
        0
    };
    if !rebuilding && !scene.index.contains(key) {
        return Ok(false);
    }
    let world = mesh.transform.affine();
    let bounds = mesh.bounds;
    let bounds_key = BoundsKey::new(
        world.to_cols_array(),
        bounds.center.to_array(),
        bounds.half_extents.to_array(),
    );
    let bounds = scene.bounds.get_or_update(mesh.entity, bounds_key, || {
        world_bounds(
            world,
            bounds.center.to_array(),
            bounds.half_extents.to_array(),
        )
    })?;
    scene.index.update(
        &mut scene.caster_data,
        key,
        CasterInput {
            bounds,
            state: CandidateState {
                shadow_casting_mode: mesh.record.0.cast_shadows,
                layer,
                renderer_present: true,
                disabled: !mesh.inherited.get(),
                force_off: false,
                lod_group: 0,
                lod_index_mask: 0,
                active_lod: 0,
            },
            material_has_caster: has_caster,
        },
    );
    Ok(true)
}
/// Rows enter the caster list after the entities, in slot order.
fn update_rows(
    scene: &mut ShadowScene,
    materials: &ShadowMaterials,
    props: &GpuProps,
) -> Result<(), &'static str> {
    for (slot, row) in props.rows().iter().enumerate() {
        if row.batch == NO_BATCH {
            continue;
        }
        let material = props.batches()[row.batch as usize].key.material;
        scene.index.update(
            &mut scene.caster_data,
            CasterKey::Row(slot as u32),
            CasterInput {
                bounds: row.bounds,
                state: CandidateState {
                    shadow_casting_mode: row.cast_shadows,
                    layer: row.layer,
                    renderer_present: true,
                    disabled: row.hidden,
                    force_off: false,
                    lod_group: 0,
                    lod_index_mask: 0,
                    active_lod: 0,
                },
                material_has_caster: materials.source_caster(material.typed())?,
            },
        );
    }
    Ok(())
}
/// Copy bounded arrays into retained render-world storage. No manifest, material, or handle clones.
pub fn extract_scene(
    input: bevy::render::Extract<Res<ShadowScene>>,
    frame: bevy::render::Extract<Res<SourceViewFrame>>,
    mut output: ResMut<ShadowScene>,
) {
    let mut timing = super::reflection_preparation::Timer::new(
        super::reflection_preparation::Stage::Extraction,
        &frame,
    );
    output.frame = 0;
    if input.frame == 0 {
        return;
    }
    if output.capacity() < input.capacity() {
        output.reserve(input.imported_capacity, input.generated_capacity);
    }
    assert_eq!(
        output.capacity(),
        input.capacity(),
        "Source shadow extraction capacity changed"
    );
    output.caster_data.extract_from(&input.caster_data);
    for i in 0..VIEW_COUNT {
        if frame.views[i].is_some() {
            timing.face(i);
        }
        if output.mask_revisions[i] != input.mask_revisions[i] {
            output.masks[i].clear();
            output.masks[i].extend_from_slice(&input.masks[i]);
            output.mask_revisions[i] = input.mask_revisions[i];
        }
    }
    output.fits = input.fits;
    output.candidates = input.candidates;
    output.split_counts = input.split_counts;
    output.camera_visible = input.camera_visible;
    output.shadow_only = input.shadow_only;
    output.origin = input.origin;
    output.frame = input.frame;
}
