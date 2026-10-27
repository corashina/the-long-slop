use crate::vehicle::VehiclePose;
use bevy::ecs as bevy_ecs;
use bevy::prelude::*;

pub use simulation::driver::CameraMode;

pub struct WorldCameraSettings {
    pub near: f32,
    pub far: f32,
    pub fov_degrees: f32,
    pub mask: u32,
}

/// The source draws near and far with Cam1 and Cam2. The port draws with one camera.
/// It uses `Cam2FarPlane` as `far`. The Cam1 far and Cam2 near planes stay unused.
pub fn world_camera_settings(fov_degrees: f32, far: f32) -> WorldCameraSettings {
    WorldCameraSettings {
        near: 0.05,
        far,
        fov_degrees,
        mask: 0x14DA8F33,
    }
}

#[derive(Component)]
pub struct DriveCamera;

pub fn pose(mode: CameraMode, car: VehiclePose) -> Transform {
    let position = Vec3::from_array(car.position);
    let rotation = Quat::from_array(car.rotation);
    match mode {
        CameraMode::Cockpit => {
            Transform::from_translation(position + rotation * Vec3::new(-0.36, 0.70, 0.12))
                .with_rotation(rotation)
        }
        CameraMode::Chase => {
            let target = position + Vec3::Y * 0.5;
            let behind = rotation * Vec3::Z;
            let horizontal = Vec3::new(behind.x, 0.0, behind.z).normalize_or_zero();
            Transform::from_translation(target + horizontal * 6.5 + Vec3::Y * 3.0)
                .looking_at(target + rotation * -Vec3::Z * 1.5, Vec3::Y)
        }
    }
}

pub fn sleep_pose(
    camera: crate::sleep::SleepCamera,
    origin: &crate::world::WorldOrigin,
) -> Transform {
    Transform::from_translation(Vec3::from_array(origin.local(camera.absolute_position)))
        .with_rotation(Quat::from_array(camera.rotation))
}
