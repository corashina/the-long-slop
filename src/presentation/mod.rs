pub mod camera;
pub mod consumables;
pub mod instruments;
pub mod lighting;
pub mod materials;
pub mod mechanisms;
pub mod reference;
mod shadow_cache;
pub mod shadows;
pub mod world;

use crate::vehicle::VehiclePose;
use bevy::prelude::*;

pub fn transform(pose: VehiclePose) -> Transform {
    Transform::from_translation(Vec3::from_array(pose.position))
        .with_rotation(Quat::from_array(pose.rotation))
}

pub fn interpolate(a: VehiclePose, b: VehiclePose, alpha: f32) -> VehiclePose {
    VehiclePose {
        position: Vec3::from_array(a.position)
            .lerp(Vec3::from_array(b.position), alpha)
            .to_array(),
        rotation: Quat::from_array(a.rotation)
            .slerp(Quat::from_array(b.rotation), alpha)
            .to_array(),
    }
}
pub mod textures;

pub mod sky;
