//! Source signed LOD fade for ordinary generated shadow casters.
use bevy::{
    asset as bevy_asset, ecs as bevy_ecs,
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    reflect as bevy_reflect, render as bevy_render,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
};
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct LodCaster {
    #[uniform(120)]
    pub fade: Vec4,
}
impl MaterialExtension for LodCaster {
    fn prepass_fragment_shader() -> ShaderRef {
        "shaders/source_lod_caster.wgsl".into()
    }
    fn enable_prepass() -> bool {
        false
    }
}
pub type LodCasterMaterial = ExtendedMaterial<StandardMaterial, LodCaster>;
