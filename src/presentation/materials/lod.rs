//! Source signed LOD fade for ordinary generated shadow casters.
use bevy::{
    asset as bevy_asset, ecs as bevy_ecs,
    ecs::system::SystemParamItem,
    material::OpaqueRendererMethod,
    mesh::MeshVertexBufferLayoutRef,
    pbr::{ExtendedMaterial, MaterialExtension},
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    reflect as bevy_reflect, render as bevy_render,
    render::{render_resource::*, renderer::RenderDevice},
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
pub type LodCasterMaterial = PreparedCaster<ExtendedMaterial<StandardMaterial, LodCaster>>;
pub type SurfaceCasterMaterial = PreparedCaster<StandardMaterial>;

/// Generated casters retain their existing material behavior and own preparation errors.
#[derive(Asset, TypePath, Clone, Debug, Deref, DerefMut)]
pub struct PreparedCaster<M: Material> {
    #[deref]
    pub material: M,
    pub preparation: crate::presentation::gpu_prepare::GpuPreparation,
}
impl<M: Material> PreparedCaster<M> {
    pub fn new(material: M, preparation: crate::presentation::gpu_prepare::GpuPreparation) -> Self {
        Self {
            material,
            preparation,
        }
    }
}
impl<M: Material> AsBindGroup for PreparedCaster<M> {
    type Data = M::Data;
    type Param = M::Param;
    fn label() -> &'static str {
        M::label()
    }
    fn bind_group_data(&self) -> Self::Data {
        self.material.bind_group_data()
    }
    fn as_bind_group(
        &self,
        descriptor: &BindGroupLayoutDescriptor,
        device: &RenderDevice,
        cache: &PipelineCache,
        param: &mut SystemParamItem<Self::Param>,
    ) -> Result<PreparedBindGroup, AsBindGroupError> {
        crate::presentation::gpu_prepare::bind_group(
            self,
            &self.preparation,
            descriptor,
            device,
            cache,
            param,
        )
    }
    fn unprepared_bind_group(
        &self,
        layout: &BindGroupLayout,
        device: &RenderDevice,
        param: &mut SystemParamItem<Self::Param>,
        direct: bool,
    ) -> Result<UnpreparedBindGroup, AsBindGroupError> {
        if self.preparation.cancelled() {
            return Err(AsBindGroupError::RetryNextUpdate);
        }
        if self.preparation.isolated() && !direct {
            return Err(AsBindGroupError::CreateBindGroupDirectly);
        }
        self.material
            .unprepared_bind_group(layout, device, param, true)
    }
    fn bind_group_layout_entries(device: &RenderDevice, _: bool) -> Vec<BindGroupLayoutEntry> {
        M::bind_group_layout_entries(device, true)
    }
}
impl<M: Material> Material for PreparedCaster<M> {
    fn vertex_shader() -> ShaderRef {
        M::vertex_shader()
    }
    fn fragment_shader() -> ShaderRef {
        M::fragment_shader()
    }
    fn alpha_mode(&self) -> AlphaMode {
        self.material.alpha_mode()
    }
    fn opaque_render_method(&self) -> OpaqueRendererMethod {
        self.material.opaque_render_method()
    }
    fn depth_bias(&self) -> f32 {
        self.material.depth_bias()
    }
    fn reads_view_transmission_texture(&self) -> bool {
        self.material.reads_view_transmission_texture()
    }
    fn enable_prepass() -> bool {
        M::enable_prepass()
    }
    fn enable_shadows() -> bool {
        M::enable_shadows()
    }
    fn prepass_vertex_shader() -> ShaderRef {
        M::prepass_vertex_shader()
    }
    fn prepass_fragment_shader() -> ShaderRef {
        M::prepass_fragment_shader()
    }
    fn deferred_vertex_shader() -> ShaderRef {
        M::deferred_vertex_shader()
    }
    fn deferred_fragment_shader() -> ShaderRef {
        M::deferred_fragment_shader()
    }
    fn specialize(
        pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        M::specialize(
            pipeline,
            descriptor,
            layout,
            MaterialPipelineKey::<M> {
                mesh_key: key.mesh_key,
                bind_group_data: key.bind_group_data,
            },
        )
    }
}
