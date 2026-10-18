//! Source candidate membership is independent of camera-visible phase lists.
use super::surface_views::VIEW_COUNT;
use super::{
    lighting_assets::LightingAssets,
    lighting_scene::{SourceSceneLighting, SourceWorldOrigin},
    materials::{RenderingState, SourceMaterial, SourceMaterials},
    renderers::SourceRenderer,
    source_lighting::SourceLightingStatus,
    view_frame::SourceViewFrame,
};
use crate::reference::{
    shadow_bounds::{BoundsKey, WorldBoundsCache},
    shadow_candidates::{Bounds, CandidateState},
    shadow_frame::{CasterInput, ShadowViewCache, validate_casters},
    shadow_planes::CullPlanes,
    shadows::{ShadowFit, ShadowFitInput, source_atlas_size},
};
use bevy::camera::{
    primitives::Aabb,
    visibility::{InheritedVisibility, RenderLayers, SetViewVisibility, ViewVisibility},
};
use bevy::{ecs as bevy_ecs, prelude::*};

#[derive(Resource, Default)]
pub struct ShadowScene {
    pub frame: u32,
    pub imported_capacity: usize,
    pub generated_capacity: usize,
    pub entities: Vec<Entity>,
    pub casters: Vec<CasterInput>,
    pub masks: [Vec<u8>; VIEW_COUNT],
    pub fits: [Option<ShadowFit>; VIEW_COUNT],
    pub candidates: [usize; VIEW_COUNT],
    pub split_counts: [[usize; 4]; VIEW_COUNT],
    pub camera_visible: usize,
    pub shadow_only: usize,
    bounds: WorldBoundsCache<Entity>,
    caster_revision: u64,
    casters_ready: bool,
    views: [ShadowViewCache; VIEW_COUNT],
}
impl ShadowScene {
    pub fn capacity(&self) -> usize {
        self.imported_capacity + self.generated_capacity
    }
    fn reserve(&mut self, imported: usize, generated: usize) {
        self.imported_capacity = imported;
        self.generated_capacity = generated;
        let count = self.capacity();
        self.entities
            .reserve_exact(count.saturating_sub(self.entities.len()));
        self.casters
            .reserve_exact(count.saturating_sub(self.casters.len()));
        for masks in &mut self.masks {
            masks.reserve_exact(count.saturating_sub(masks.len()));
        }
    }
}
/// Runs after both camera culling systems and before visibility removal/extraction.
/// Only ViewVisibility changes; camera VisibleEntities retain their camera-only membership.
pub fn prepare_scene(
    frame: Res<SourceViewFrame>,
    live: Res<SourceSceneLighting>,
    origin: Res<SourceWorldOrigin>,
    assets: Option<Res<LightingAssets>>,
    source: Res<SourceMaterials>,
    materials: Res<Assets<SourceMaterial>>,
    specular: Res<Assets<super::materials::specular::SpecularMaterial>>,
    terrain: Res<Assets<super::materials::terrain::TerrainMaterial>>,
    grass: Res<Assets<super::materials::grass::material::GrassMaterial>>,
    impostors: Res<Assets<super::materials::impostor::ImpostorMaterial>>,
    mut scene: ResMut<ShadowScene>,
    status: Res<SourceLightingStatus>,
    master: Res<super::lighting::SourceLighting>,
    mut meshes: Query<
        (
            Entity,
            &Aabb,
            &GlobalTransform,
            &InheritedVisibility,
            &mut ViewVisibility,
            Option<&MeshMaterial3d<SourceMaterial>>,
            &SourceRenderer,
            Option<&RenderLayers>,
            Option<&MeshMaterial3d<super::materials::specular::SpecularMaterial>>,
            Option<&MeshMaterial3d<super::materials::terrain::TerrainMaterial>>,
            Option<&MeshMaterial3d<super::materials::grass::material::GrassMaterial>>,
            Option<&MeshMaterial3d<super::materials::impostor::ImpostorMaterial>>,
        ),
        Without<super::materials::SourceMaterialWarmup>,
    >,
) {
    let mut timing = super::reflection_preparation::Timer::new(
        super::reflection_preparation::Stage::Shadows,
        &frame,
    );
    scene.frame = 0;
    scene.fits = [None; VIEW_COUNT];
    scene.candidates = [0; VIEW_COUNT];
    scene.split_counts = [[0; 4]; VIEW_COUNT];
    if !frame.required_views_ready()
        || source.state != RenderingState::Ready
        || master.road_lamps.recipe.is_none()
        || matches!(*status.0.lock().unwrap(), RenderingState::Failed(_))
    {
        return;
    }
    let (Some(assets), Some(live)) = (assets, live.0) else {
        return;
    };
    let imported = meshes.iter().count();
    let generated = 0;
    if imported > scene.imported_capacity || generated > scene.generated_capacity {
        scene.reserve(imported, generated);
        scene.bounds.reserve(imported);
    }
    let mut changed =
        scene.bounds.begin(frame.session, frame.scene, origin.0) || !scene.casters_ready;
    // An interrupted collection must not publish a prior view result after recovery.
    scene.casters_ready = false;
    scene.camera_visible = 0;
    scene.shadow_only = 0;
    let fail = |message: &str| {
        *status.0.lock().unwrap() = RenderingState::Failed(message.into());
    };
    let mut imported = 0;
    let mut row = 0;

    for (
        entity,
        bounds,
        transform,
        inherited,
        visible,
        material,
        record,
        layers,
        specular_handle,
        terrain_handle,
        grass_handle,
        impostor_handle,
    ) in &mut meshes
    {
        imported += 1;
        if imported > scene.imported_capacity {
            fail("Source shadow scene capacity was exceeded");
            return;
        }
        let has_caster = if let Some(handle) = material {
            let Some(material) = materials.get(handle.0.id()) else {
                fail("Missing source shadow material");
                return;
            };
            material.extension.has_caster()
        } else if let Some(handle) = specular_handle {
            if !specular.contains(handle.0.id()) {
                fail("Missing source specular material");
                return;
            }
            true
        } else if let Some(handle) = terrain_handle {
            if !terrain.contains(handle.0.id()) {
                fail("Missing source terrain material");
                return;
            }
            true
        } else if let Some(handle) = grass_handle {
            if !grass.contains(handle.0.id()) {
                fail("Missing source grass material");
                return;
            }
            true
        } else if let Some(handle) = impostor_handle {
            if !impostors.contains(handle.0.id()) {
                fail("Missing source impostor material");
                return;
            }
            true
        } else {
            continue;
        };
        let layer = if let Some(layers) = layers {
            let mut iter = layers.iter();
            let Some(layer) = iter.next() else { continue };
            if layer >= 32 || iter.next().is_some() {
                fail("Source shadow renderer requires one source layer");
                return;
            }
            layer as u8
        } else {
            0
        };
        let world = transform.affine();
        let key = BoundsKey::new(
            world.to_cols_array(),
            bounds.center.to_array(),
            bounds.half_extents.to_array(),
        );
        let b = match scene.bounds.get_or_update(entity, key, || {
            let center = world.transform_point3a(bounds.center);
            let e = world.matrix3.x_axis.abs() * bounds.half_extents.x
                + world.matrix3.y_axis.abs() * bounds.half_extents.y
                + world.matrix3.z_axis.abs() * bounds.half_extents.z;
            Bounds {
                center: [center.x, center.y, -center.z],
                extents: e.to_array(),
            }
        }) {
            Ok(bounds) => bounds,
            Err(error) => {
                fail(error);
                return;
            }
        };
        scene.camera_visible += usize::from(visible.get());
        let caster = CasterInput {
            bounds: b,
            state: CandidateState {
                shadow_casting_mode: record.0.cast_shadows,
                layer,
                renderer_present: true,
                disabled: !inherited.get(),
                force_off: false,
                lod_group: 0,
                lod_index_mask: 0,
                active_lod: 0,
            },
            material_has_caster: has_caster,
        };
        if let Some(previous) = scene.casters.get(row) {
            changed |= scene.entities[row] != entity || !previous.unchanged(&caster);
            scene.entities[row] = entity;
            scene.casters[row] = caster;
        } else {
            changed = true;
            scene.entities.push(entity);
            scene.casters.push(caster);
        }
        row += 1;
    }
    changed |= row != scene.casters.len();
    scene.entities.truncate(row);
    scene.casters.truncate(row);
    scene.bounds.finish();
    if changed {
        if let Err(error) = validate_casters(&scene.casters) {
            fail(error);
            return;
        }
        scene.caster_revision = scene
            .caster_revision
            .checked_add(1)
            .expect("Source shadow caster revision exhausted");
    }
    scene.casters_ready = true;
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
            continue;
        };
        timing.face(index);
        let atlas =
            source_atlas_size(v.extent.x, v.extent.y).expect("validated source view extent");
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
        scene.masks[index].resize(scene.casters.len(), 0);
        match scene.views[index].prepare(
            scene.caster_revision,
            &scene.casters,
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
    }
    for (i, entity) in scene.entities.iter().enumerate() {
        if scene
            .masks
            .iter()
            .any(|m| m.get(i).is_some_and(|mask| *mask != 0))
        {
            if let Ok((_, _, _, _, mut visible, _, _, _, _, _, _, _)) = meshes.get_mut(*entity) {
                if !visible.get() {
                    scene.shadow_only += 1;
                }
                visible.set_visible();
            }
        }
    }
    scene.frame = frame.frame;
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
    if input.capacity() == 0 {
        output.frame = 0;
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
    output.frame = input.frame;
    output.entities.clear();
    output.entities.extend_from_slice(&input.entities);
    output.casters.clear();
    output.casters.extend_from_slice(&input.casters);
    for i in 0..VIEW_COUNT {
        if frame.views[i].is_some() {
            timing.face(i);
        }
        output.masks[i].clear();
        output.masks[i].extend_from_slice(&input.masks[i]);
    }
    output.fits = input.fits;
    output.candidates = input.candidates;
    output.split_counts = input.split_counts;
    output.camera_visible = input.camera_visible;
    output.shadow_only = input.shadow_only;
}
