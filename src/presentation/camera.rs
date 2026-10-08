use crate::vehicle::VehiclePose;
use bevy::ecs as bevy_ecs;
use bevy::prelude::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CameraMode {
    Cockpit,
    #[default]
    Chase,
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
