pub mod camera;
mod candidate_upload;
pub mod conditions;
pub mod consumables;
pub mod fixed_assets;
pub mod gpu_prepare;
pub mod instruments;
pub mod lighting;
pub mod materials;
pub mod mechanisms;
pub mod reference;
pub mod save;
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
pub mod texture_batches;
pub mod textures;

pub mod sky;

pub mod camera_depth;
pub mod caster;
pub(crate) mod caster_material;
pub mod clouds;
pub mod frame_copy;
pub mod frame_graph;
pub mod lighting_assets;
pub mod lighting_scene;
pub mod motion_draw;
pub mod motion_vectors;
pub mod particles;
pub mod plates;
pub mod postprocessing;
pub mod probes;
mod reflection_preparation;
pub mod render_targets;
pub mod renderers;
pub mod shadow_atlas;
pub mod shadow_collector;
pub mod shadow_phase;
pub mod shadow_queue;
pub mod shadow_scene;
pub mod source_effects;
pub mod source_images;
pub(crate) mod source_io;
pub mod source_lighting;
pub mod source_sky;
pub mod stained_glass;
pub mod surface_graph;
pub mod surface_pass;
pub mod surface_views;
pub mod transparent;
pub mod view_frame;

mod pipeline_ownership;

pub mod primary_image;
