pub use my_summer_slop::reference;

#[path = "../src/presentation/renderer_visibility.rs"]
#[allow(dead_code)]
mod renderer_visibility;

use bevy::{
    camera::visibility::{ViewVisibility, VisibleEntities, VisibleMeshEntities},
    ecs::{self as bevy_ecs, system::RunSystemOnce},
    prelude::*,
};
use reference::{SourceId, render_lighting::RendererRecord};
use renderer_visibility::{
    CameraVisibility, SourceMaterialWarmup, SourceRenderer, SourceSortData,
    filter_camera_visibility,
};
use std::any::TypeId;

fn renderer(mode: u8) -> SourceRenderer {
    SourceRenderer(RendererRecord {
        source: SourceId {
            file: "fixture.assets".into(),
            path_id: 1,
        },
        bounds_center: [0.; 3],
        bounds_extents: [1.; 3],
        cast_shadows: mode,
        receive_shadows: true,
        reflection_probe_usage: 1,
        motion_vectors: 0,
        sorting_layer: 0,
        sorting_order: 0,
    })
}

fn sort(key: u32) -> SourceSortData {
    SourceSortData {
        material_key: key,
        material_identity: key,
        glass_source: 0,
        shader_identity: 1,
        source_material_index: 0,
        layer_and_order: 0,
        queue: 3000,
        additive: false,
        glass: false,
    }
}

fn mesh(world: &mut World, mode: u8, key: u32) -> Entity {
    world
        .spawn((
            Mesh3d::default(),
            renderer(mode),
            sort(key),
            ViewVisibility::VISIBLE,
        ))
        .id()
}

fn view(world: &mut World, entities: &[Entity], active: bool) -> Entity {
    let mut visible = VisibleEntities::default();
    visible
        .get_mut(TypeId::of::<Mesh3d>())
        .extend_from_slice(entities);
    world
        .spawn((
            Camera {
                is_active: active,
                ..default()
            },
            visible,
        ))
        .id()
}

fn membership(world: &World, view: Entity) -> &[Entity] {
    world
        .get::<VisibleEntities>(view)
        .unwrap()
        .get(TypeId::of::<Mesh3d>())
}

fn cull(world: &mut World) {
    world.run_system_once(filter_camera_visibility).unwrap();
}

#[test]
fn camera_rejection_preserves_shadow_extraction_warmup_order_and_capacity() {
    let mut world = World::new();
    world.init_resource::<CameraVisibility>();
    let modes = (0..=3)
        .map(|mode| mesh(&mut world, mode, u32::from(mode)))
        .collect::<Vec<_>>();
    let warmup = mesh(&mut world, 3, 4);
    world.entity_mut(warmup).insert(SourceMaterialWarmup);
    let particle = world.spawn((Mesh3d::default(), sort(5))).id();
    let input = [modes[0], modes[3], modes[1], warmup, modes[2], particle];
    let camera = view(&mut world, &input, true);
    let light = world
        .spawn(VisibleMeshEntities {
            entities: vec![modes[3]],
        })
        .id();
    let capacity =
        world.get::<VisibleEntities>(camera).unwrap().entities[&TypeId::of::<Mesh3d>()].capacity();

    cull(&mut world);

    assert_eq!(
        membership(&world, camera),
        [modes[0], modes[1], warmup, modes[2], particle]
    );
    assert_eq!(
        world.get::<VisibleEntities>(camera).unwrap().entities[&TypeId::of::<Mesh3d>()].capacity(),
        capacity
    );
    assert!(world.get::<ViewVisibility>(modes[3]).unwrap().get());
    assert_eq!(
        world.get::<VisibleMeshEntities>(light).unwrap().entities,
        [modes[3]]
    );
    let output = world.resource::<CameraVisibility>();
    assert!(!output.entities.contains(&modes[3]));
    assert!(!output.sort_data.contains_key(&modes[3]));
    assert!(output.entities.contains(&warmup));
    assert_eq!(output.sort_data.len(), 5);
}

#[test]
fn camera_union_includes_probe_only_objects_and_clears_inactive_views() {
    let mut world = World::new();
    world.init_resource::<CameraVisibility>();
    let main = mesh(&mut world, 1, 1);
    let probe_only = mesh(&mut world, 1, 2);
    let stale = mesh(&mut world, 1, 3);
    let camera = view(&mut world, &[main], true);
    let probe = view(&mut world, &[main, probe_only], true);
    let inactive = view(&mut world, &[stale], false);
    cull(&mut world);
    let output = world.resource::<CameraVisibility>();
    assert_eq!(output.entities.len(), 2);
    assert_eq!(output.sort_data.len(), 2);
    assert!(output.sort_data.contains_key(&probe_only));
    assert!(!output.sort_data.contains_key(&stale));
    assert!(membership(&world, inactive).is_empty());

    world.get_mut::<Camera>(camera).unwrap().is_active = false;
    world.get_mut::<Camera>(probe).unwrap().is_active = false;
    cull(&mut world);
    assert!(world.resource::<CameraVisibility>().sort_data.is_empty());
    assert!(world.resource::<CameraVisibility>().entities.is_empty());
    assert!(
        world
            .get::<VisibleEntities>(probe)
            .unwrap()
            .entities
            .contains_key(&TypeId::of::<Mesh3d>())
    );
}

#[test]
fn visibility_material_and_entity_changes_remove_stale_sort_inputs() {
    let mut world = World::new();
    world.init_resource::<CameraVisibility>();
    let entity = mesh(&mut world, 1, 1);
    let other = mesh(&mut world, 1, 2);
    let camera = view(&mut world, &[entity, other], true);
    cull(&mut world);
    let capacity = world.resource::<CameraVisibility>().sort_data.capacity();

    world
        .get_mut::<SourceRenderer>(entity)
        .unwrap()
        .0
        .cast_shadows = 3;
    world.entity_mut(other).remove::<SourceSortData>();
    cull(&mut world);
    assert_eq!(membership(&world, camera), [other]);
    assert!(world.resource::<CameraVisibility>().sort_data.is_empty());

    world
        .get_mut::<SourceRenderer>(entity)
        .unwrap()
        .0
        .cast_shadows = 1;
    world.entity_mut(entity).insert(sort(9));
    world
        .get_mut::<VisibleEntities>(camera)
        .unwrap()
        .get_mut(TypeId::of::<Mesh3d>())
        .push(entity);
    cull(&mut world);
    assert_eq!(
        world.resource::<CameraVisibility>().sort_data[&entity].material_key,
        9
    );
    assert_eq!(
        world.resource::<CameraVisibility>().sort_data.capacity(),
        capacity
    );

    world.despawn(entity);
    world.entity_mut(other).remove::<Mesh3d>();
    cull(&mut world);
    assert!(membership(&world, camera).is_empty());
    assert!(world.resource::<CameraVisibility>().sort_data.is_empty());
    assert!(world.resource::<CameraVisibility>().entities.is_empty());
}

#[test]
fn camera_culling_precedes_shadow_union_and_drops_previous_frame_membership() {
    use bevy::camera::{
        primitives::{Aabb, Frustum},
        visibility::{SetViewVisibility, VisibilityPlugin, VisibilitySystems},
    };

    #[derive(Resource)]
    struct ShadowCaster(Entity);

    fn mark_shadow(caster: Res<ShadowCaster>, mut query: Query<&mut ViewVisibility>) {
        query.get_mut(caster.0).unwrap().set_visible();
    }

    let mut app = App::new();
    app.add_plugins((
        bevy::app::TaskPoolPlugin::default(),
        bevy::asset::AssetPlugin::default(),
        bevy::mesh::MeshPlugin,
        bevy::transform::TransformPlugin,
        VisibilityPlugin,
    ));
    app.init_resource::<CameraVisibility>().add_systems(
        PostUpdate,
        (filter_camera_visibility, mark_shadow)
            .chain()
            .after(VisibilitySystems::CheckVisibility)
            .before(VisibilitySystems::MarkNewlyHiddenEntitiesInvisible),
    );
    let camera = app
        .world_mut()
        .spawn((
            Camera::default(),
            Frustum(bevy::math::primitives::ViewFrustum::from_clip_from_world(
                &Mat4::IDENTITY,
            )),
        ))
        .id();
    let mut spawn = |position: Vec3, mode| {
        app.world_mut()
            .spawn((
                Mesh3d::default(),
                renderer(mode),
                sort(u32::from(mode)),
                Transform::from_translation(position),
                Aabb {
                    center: Vec3::ZERO.into(),
                    half_extents: Vec3::splat(0.1).into(),
                },
            ))
            .id()
    };
    let visible = spawn(Vec3::new(0., 0., 0.5), 1);
    let offscreen = spawn(Vec3::new(10., 0., 0.5), 1);
    let shadow_only = spawn(Vec3::new(0., 0., 0.5), 3);
    app.insert_resource(ShadowCaster(offscreen));

    app.update();

    assert_eq!(membership(app.world(), camera), [visible]);
    assert!(app.world().get::<ViewVisibility>(offscreen).unwrap().get());
    assert!(
        app.world()
            .get::<ViewVisibility>(shadow_only)
            .unwrap()
            .get()
    );
    assert!(
        !app.world()
            .resource::<CameraVisibility>()
            .sort_data
            .contains_key(&offscreen)
    );
    assert!(
        !app.world()
            .resource::<CameraVisibility>()
            .sort_data
            .contains_key(&shadow_only)
    );

    app.world_mut()
        .get_mut::<Transform>(visible)
        .unwrap()
        .translation
        .x = 10.;
    app.update();

    assert!(membership(app.world(), camera).is_empty());
    assert!(
        app.world()
            .resource::<CameraVisibility>()
            .sort_data
            .is_empty()
    );
    assert!(!app.world().get::<ViewVisibility>(visible).unwrap().get());
    assert!(app.world().get::<ViewVisibility>(offscreen).unwrap().get());
}
