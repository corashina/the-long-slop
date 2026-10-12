//! Source candidate membership is independent of camera-visible phase lists.
use super::{
    lighting_assets::LightingAssets,
    lighting_scene::SourceSceneLighting,
    materials::{GeneratedWorldMaterial, RenderingState, SourceMaterial, SourceMaterials},
    renderers::SourceRenderer,
    source_lighting::SourceLightingStatus,
    view_frame::SourceViewFrame,
};
use crate::reference::{
    shadow_candidates::{Bounds, CandidateState},
    shadow_frame::{CasterInput, prepare_frame},
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
    pub masks: [Vec<u8>; 9],
    pub fits: [Option<ShadowFit>; 9],
    pub candidates: [usize; 9],
    pub split_counts: [[usize; 4]; 9],
    pub camera_visible: usize,
    pub shadow_only: usize,
}
impl ShadowScene {
    pub fn capacity(&self) -> usize {
        self.imported_capacity + self.generated_capacity
    }
    fn reserve(&mut self, imported: usize, generated: usize) {
        self.imported_capacity = imported;
        self.generated_capacity = generated;
        let count = self.capacity();
        self.entities.reserve_exact(count);
        self.casters.reserve_exact(count);
        for masks in &mut self.masks {
            masks.reserve_exact(count);
        }
    }
}
/// Runs after both camera culling systems and before visibility removal/extraction.
/// Only ViewVisibility changes; camera VisibleEntities retain their camera-only membership.
pub fn prepare_scene(
    frame: Res<SourceViewFrame>,
    live: Res<SourceSceneLighting>,
    assets: Option<Res<LightingAssets>>,
    source: Res<SourceMaterials>,
    materials: Res<Assets<SourceMaterial>>,
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
            &MeshMaterial3d<SourceMaterial>,
            Option<&SourceRenderer>,
            Option<&GeneratedWorldMaterial>,
            Option<&RenderLayers>,
        ),
        Without<super::materials::SourceMaterialWarmup>,
    >,
) {
    scene.frame = 0;
    scene.fits = [None; 9];
    scene.candidates = [0; 9];
    scene.split_counts = [[0; 4]; 9];
    if frame.views.iter().any(Option::is_none)
        || source.state != RenderingState::Ready
        || master.road_lamps.recipe.is_none()
        || matches!(*status.0.lock().unwrap(), RenderingState::Failed(_))
    {
        return;
    }
    let (Some(assets), Some(live)) = (assets, live.0) else {
        return;
    };
    if scene.capacity() == 0 {
        let imported = meshes.iter().filter(|x| x.6.is_some()).count();
        if imported == 0 {
            return;
        }
        scene.reserve(
            imported + master.road_lamps.source_mesh_capacity(),
            super::world::source_shadow_capacity(),
        );
    }
    scene.entities.clear();
    scene.casters.clear();
    scene.camera_visible = 0;
    scene.shadow_only = 0;
    let fail = |message: &str| {
        *status.0.lock().unwrap() = RenderingState::Failed(message.into());
    };
    let mut imported = 0;
    let mut generated = 0;
    for (
        entity,
        bounds,
        transform,
        inherited,
        visible,
        material,
        record,
        generated_identity,
        layers,
    ) in &mut meshes
    {
        if record.is_some() {
            imported += 1;
        } else if generated_identity.is_some() {
            generated += 1;
        } else {
            fail("Source shadow mesh has no source or generated identity");
            return;
        }
        if imported > scene.imported_capacity || generated > scene.generated_capacity {
            fail("Source shadow scene capacity was exceeded");
            return;
        }
        let Some(material) = materials.get(material.0.id()) else {
            fail("Source shadow material is missing after loading");
            return;
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
        let center = world.transform_point3a(bounds.center);
        let e = world.matrix3.x_axis.abs() * bounds.half_extents.x
            + world.matrix3.y_axis.abs() * bounds.half_extents.y
            + world.matrix3.z_axis.abs() * bounds.half_extents.z;
        let b = Bounds {
            center: [center.x, center.y, -center.z],
            extents: e.to_array(),
        };
        if b.center
            .iter()
            .chain(b.extents.iter())
            .any(|v| !v.is_finite())
            || b.extents.iter().any(|v| *v < 0.)
        {
            fail("Source shadow renderer bounds are nonfinite");
            return;
        }
        scene.camera_visible += usize::from(visible.get());
        scene.entities.push(entity);
        scene.casters.push(CasterInput {
            bounds: b,
            state: CandidateState {
                shadow_casting_mode: record.map_or(1, |r| r.0.cast_shadows),
                layer,
                renderer_present: true,
                disabled: !inherited.get(),
                force_off: false,
                lod_group: 0,
                lod_index_mask: 0,
                active_lod: 0,
            },
            material_has_caster: material.extension.has_caster(),
        });
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
    for index in 0..9 {
        let v = frame.views[index].unwrap();
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
        match prepare_frame(
            &scene.casters,
            &mut scene.masks[index],
            &CullPlanes {
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
        if scene.masks.iter().any(|m| m[i] != 0) {
            if let Ok((_, _, _, _, mut visible, _, _, _, _)) = meshes.get_mut(*entity) {
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
    mut output: ResMut<ShadowScene>,
) {
    if input.capacity() == 0 {
        output.frame = 0;
        return;
    }
    if output.capacity() == 0 {
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
    for i in 0..9 {
        output.masks[i].clear();
        output.masks[i].extend_from_slice(&input.masks[i]);
    }
    output.fits = input.fits;
    output.candidates = input.candidates;
    output.split_counts = input.split_counts;
    output.camera_visible = input.camera_visible;
    output.shadow_only = input.shadow_only;
}
