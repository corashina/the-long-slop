//! Test the production projection helpers without installed source assets.
pub use my_summer_slop::{reference, sleep, vehicle, world};
#[path = "../src/presentation/camera.rs"]
#[allow(dead_code)]
mod camera;
#[path = "../src/presentation/camera_projection.rs"]
mod camera_projection;
use bevy::camera::{PerspectiveProjection, Projection};
use camera_projection::{player_perspective, sync_player_projection};

#[test]
fn world_and_sky_zoom_preserves_planes_and_probe() {
    let mut world = Projection::Perspective(PerspectiveProjection {
        fov: 65_f32.to_radians(),
        near: 0.05,
        far: 15000.,
        aspect_ratio: 1.5,
        ..Default::default()
    });
    let mut sky = Projection::Perspective(PerspectiveProjection {
        fov: 65_f32.to_radians(),
        near: 0.7,
        far: 1234.,
        aspect_ratio: 1.7,
        ..Default::default()
    });
    let mut probe = Projection::Perspective(PerspectiveProjection {
        fov: 90_f32.to_radians(),
        near: 0.3,
        far: 321.,
        aspect_ratio: 1.,
        ..Default::default()
    });
    sync_player_projection(&mut world, true, 26.);
    sync_player_projection(&mut sky, true, 26.);
    sync_player_projection(&mut probe, false, 26.);
    let Projection::Perspective(world) = world else {
        panic!("Expected perspective")
    };
    let Projection::Perspective(sky) = sky else {
        panic!("Expected perspective")
    };
    let Projection::Perspective(probe) = probe else {
        panic!("Expected perspective")
    };
    assert!((world.fov - 26_f32.to_radians()).abs() < 1.0e-6);
    assert!((sky.fov - 26_f32.to_radians()).abs() < 1.0e-6);
    assert!((probe.fov - 90_f32.to_radians()).abs() < 1.0e-6);
    assert_eq!(
        (world.near, world.far, world.aspect_ratio),
        (0.05, 15000., 1.5)
    );
    assert_eq!((sky.near, sky.far, sky.aspect_ratio), (0.7, 1234., 1.7));
    assert_eq!((probe.near, probe.far, probe.aspect_ratio), (0.3, 321., 1.));
}

#[test]
fn candidate_projection_does_not_read_live_zoom() {
    let mut live = Projection::Perspective(player_perspective(65., 1.5));
    sync_player_projection(&mut live, true, 26.);
    let candidate = player_perspective(65., 1.7);
    assert!((candidate.fov - 65_f32.to_radians()).abs() < 1.0e-6);
    assert_eq!(
        (candidate.near, candidate.far, candidate.aspect_ratio),
        (0.05, 15000., 1.7)
    );
}

#[test]
fn player_sync_keeps_orthographic_views() {
    let mut projection =
        Projection::Orthographic(bevy::camera::OrthographicProjection::default_3d());
    sync_player_projection(&mut projection, true, 26.);
    assert!(matches!(projection, Projection::Orthographic(_)));
}

#[test]
fn source_world_settings_keep_mask_and_planes() {
    let zoom = camera::world_camera_settings(26.);
    let neutral = camera::world_camera_settings(65.);
    assert_eq!((zoom.near, zoom.far, zoom.mask), (0.05, 15000., 0x14DA8F33));
    assert_eq!(
        (zoom.near, zoom.far, zoom.mask),
        (neutral.near, neutral.far, neutral.mask)
    );
    assert_eq!((zoom.fov_degrees, neutral.fov_degrees), (26., 65.));
}
