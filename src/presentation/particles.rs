//! Retained particle uniform buffers. Simulation remains in the runtime owners.
use super::lighting::SourceLighting;
use crate::{
    app::{Drive, RuntimeReference},
    reference::ParticleReference,
};
use bevy::mesh::{Indices, MeshVertexBufferLayoutRef};
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::render::{
    Extract, ExtractSchedule, Render, RenderApp, RenderSystems,
    render_asset::RenderAssets,
    render_resource::*,
    renderer::{RenderDevice, RenderQueue},
    texture::GpuImage,
};
use bevy::shader::ShaderRef;
use bevy::{
    asset::{self as bevy_asset, RenderAssetUsages},
    ecs::{
        self as bevy_ecs,
        system::{SystemParamItem, lifetimeless::SRes},
    },
    prelude::*,
    reflect as bevy_reflect,
};
const BATCH: usize = 128;
#[derive(Clone, Copy, ShaderType)]
pub struct ParticleUniform {
    positions: [Vec4; BATCH],
    colors: [Vec4; BATCH],
    angles: [Vec4; BATCH],
    tint: Vec4,
    st: Vec4,
    fog: Vec4,
    settings: Vec4,
    /// x: soft-particle InvFade (0 off), y: distortion strength scaled (0 off), z: distortion blend, w: bump scale.
    effect: Vec4,
}
impl Default for ParticleUniform {
    fn default() -> Self {
        Self {
            positions: [Vec4::ZERO; BATCH],
            colors: [Vec4::ZERO; BATCH],
            angles: [Vec4::ZERO; BATCH],
            tint: Vec4::ONE,
            st: Vec4::new(1., 1., 0., 0.),
            fog: Vec4::ZERO,
            settings: Vec4::ZERO,
            effect: Vec4::ZERO,
        }
    }
}
#[derive(Asset, TypePath, Clone, Debug)]
pub struct ParticleMaterial {
    batch: usize,
    texture: Handle<Image>,
    /// Distortion `_BumpMap`; the main texture when the material has none.
    bump: Handle<Image>,
    additive: bool,
    order: f32,
}
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ParticleKey {
    additive: bool,
}
/// Source particle shader families selected by the exported material.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ParticleShader {
    /// Legacy Shaders/Particles/Alpha Blended.
    AlphaBlended,
    /// Legacy Shaders/Particles/Additive.
    Additive,
    /// Mobile/Particles/Additive: no tint, no soft particles.
    MobileAdditive,
    /// Particles/Standard Unlit and other `_Color` alpha-blended particles.
    Unlit,
}
impl ParticleShader {
    fn new(name: &str) -> Self {
        match name {
            "Legacy Shaders/Particles/Alpha Blended" => Self::AlphaBlended,
            "Legacy Shaders/Particles/Additive" => Self::Additive,
            "Mobile/Particles/Additive" => Self::MobileAdditive,
            _ => Self::Unlit,
        }
    }
    fn legacy(self) -> bool {
        matches!(self, Self::AlphaBlended | Self::Additive)
    }
    fn additive(self) -> bool {
        matches!(self, Self::Additive | Self::MobileAdditive)
    }
}
pub(crate) type DrawSourceParticle = (
    bevy::render::render_phase::SetItemPipeline,
    bevy::pbr::SetMeshViewBindGroup<0>,
    super::stained_glass::SetSourceParticleView,
    bevy::pbr::SetMeshBindGroup<2>,
    bevy::pbr::SetMaterialBindGroup<3>,
    bevy::pbr::DrawMesh,
);
impl Material for ParticleMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/source_particles.wgsl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "shaders/source_particles.wgsl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
    fn depth_bias(&self) -> f32 {
        self.order
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
    fn specialize(
        _: &MaterialPipeline,
        d: &mut RenderPipelineDescriptor,
        l: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        d.layout[1] = super::stained_glass::particle_view_layout();
        if key.bind_group_data.additive {
            // Source Blend SrcAlpha One, ColorMask RGB.
            if let Some(target) = d
                .fragment
                .as_mut()
                .and_then(|f| f.targets.first_mut())
                .and_then(|t| t.as_mut())
            {
                target.blend = Some(BlendState {
                    color: BlendComponent {
                        src_factor: BlendFactor::SrcAlpha,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    },
                    alpha: BlendComponent::OVER,
                });
                target.write_mask = ColorWrites::COLOR;
            }
        }
        d.vertex.buffers = vec![l.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(1),
        ])?];
        d.primitive.cull_mode = None;
        if let Some(depth) = d.depth_stencil.as_mut() {
            depth.depth_write_enabled = Some(false);
        }
        Ok(())
    }
}
#[derive(Resource, Default)]
pub struct ParticleGpu {
    buffers: Vec<UniformBuffer<ParticleUniform>>,
}
impl AsBindGroup for ParticleMaterial {
    type Data = ParticleKey;
    type Param = (SRes<ParticleGpu>, SRes<RenderAssets<GpuImage>>);
    fn label() -> &'static str {
        "source particles"
    }
    fn bind_group_data(&self) -> ParticleKey {
        ParticleKey {
            additive: self.additive,
        }
    }
    fn unprepared_bind_group(
        &self,
        _: &BindGroupLayout,
        _: &RenderDevice,
        p: &mut SystemParamItem<Self::Param>,
        _: bool,
    ) -> Result<UnpreparedBindGroup, AsBindGroupError> {
        let buffer =
            p.0.buffers
                .get(self.batch)
                .and_then(|b| b.buffer())
                .ok_or(AsBindGroupError::RetryNextUpdate)?;
        let image =
            p.1.get(self.texture.id())
                .ok_or(AsBindGroupError::RetryNextUpdate)?;
        let bump =
            p.1.get(self.bump.id())
                .ok_or(AsBindGroupError::RetryNextUpdate)?;
        Ok(UnpreparedBindGroup {
            bindings: BindingResources(vec![
                (0, OwnedBindingResource::Buffer(buffer.clone())),
                (
                    1,
                    OwnedBindingResource::TextureView(
                        TextureViewDimension::D2,
                        image.texture_view.clone(),
                    ),
                ),
                (
                    2,
                    OwnedBindingResource::Sampler(
                        SamplerBindingType::Filtering,
                        image.sampler.clone(),
                    ),
                ),
                (
                    3,
                    OwnedBindingResource::TextureView(
                        TextureViewDimension::D2,
                        bump.texture_view.clone(),
                    ),
                ),
                (
                    4,
                    OwnedBindingResource::Sampler(
                        SamplerBindingType::Filtering,
                        bump.sampler.clone(),
                    ),
                ),
            ]),
        })
    }
    fn bind_group_layout_entries(_: &RenderDevice, _: bool) -> Vec<BindGroupLayoutEntry> {
        use binding_types::*;
        BindGroupLayoutEntries::sequential(
            ShaderStages::VERTEX_FRAGMENT,
            (
                uniform_buffer::<ParticleUniform>(false),
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
            ),
        )
        .to_vec()
    }
}
#[derive(Clone, Copy)]
enum Owner {
    Urine,
    Brush(usize),
    Spray(usize),
    Wash(usize),
    Outlet(u32),
}
struct Group {
    owner: Owner,
    source: usize,
    start: usize,
    count: usize,
    order: Vec<usize>,
    entities: Vec<Entity>,
    /// Additive source particles fog toward black.
    black_fog: bool,
}
#[derive(Resource, Default)]
pub(crate) struct ParticleRender {
    mesh: Option<Handle<Mesh>>,
    groups: Vec<Group>,
    uniforms: Vec<ParticleUniform>,
}
impl ParticleRender {
    pub(crate) fn prepared(&self) -> bool {
        self.mesh.is_some()
    }
}
#[derive(Resource, Default)]
pub(crate) struct ExtractedParticles {
    pub(crate) uniforms: Vec<ParticleUniform>,
}
pub struct SourceParticlePlugin;
impl Plugin for SourceParticlePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ParticleRender>()
            .add_plugins(MaterialPlugin::<ParticleMaterial>::default())
            .add_systems(
                PostUpdate,
                sync_particles
                    .after(bevy::transform::TransformSystems::Propagate)
                    .after(super::lighting::sync_lighting)
                    .before(bevy::camera::visibility::VisibilitySystems::CheckVisibility),
            );
        app.sub_app_mut(RenderApp)
            .init_resource::<ParticleGpu>()
            .init_resource::<ExtractedParticles>()
            .add_systems(ExtractSchedule, extract)
            .add_systems(Render, upload.in_set(RenderSystems::PrepareResources));
        bevy::render::render_phase::AddRenderCommand::add_render_command::<
            bevy::core_pipeline::core_3d::Transparent3d,
            DrawSourceParticle,
        >(app.sub_app_mut(RenderApp));
    }
}
fn extract(state: Extract<Res<ParticleRender>>, mut extracted: ResMut<ExtractedParticles>) {
    extracted
        .uniforms
        .resize(state.uniforms.len(), ParticleUniform::default());
    extracted.uniforms.copy_from_slice(&state.uniforms);
}
fn upload(
    extracted: Res<ExtractedParticles>,
    mut gpu: ResMut<ParticleGpu>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
) {
    while gpu.buffers.len() < extracted.uniforms.len() {
        gpu.buffers.push(UniformBuffer::default());
    }
    for (i, value) in extracted.uniforms.iter().enumerate() {
        // Hidden empty batches need no upload after their initial buffer creation.
        if value.settings.w == 0. && gpu.buffers[i].buffer().is_some() {
            continue;
        }
        gpu.buffers[i].set(*value);
        gpu.buffers[i].write_buffer(&device, &queue);
    }
}
fn mesh() -> Mesh {
    let mut positions = Vec::with_capacity(BATCH * 4);
    let mut uv = Vec::with_capacity(BATCH * 4);
    let mut indices = Vec::with_capacity(BATCH * 6);
    for i in 0..BATCH {
        positions.extend([
            [-0.5, -0.5, i as f32],
            [0.5, -0.5, i as f32],
            [0.5, 0.5, i as f32],
            [-0.5, 0.5, i as f32],
        ]);
        uv.extend([[0., 1.], [1., 1.], [1., 0.], [0., 0.]]);
        let k = i as u32 * 4;
        indices.extend([k, k + 1, k + 2, k, k + 2, k + 3]);
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uv)
    .with_inserted_indices(Indices::U32(indices))
}
fn add_group(
    state: &mut ParticleRender,
    owner: Owner,
    index: usize,
    r: &ParticleReference,
    lighting: &SourceLighting,
    commands: &mut Commands,
    materials: &mut Assets<ParticleMaterial>,
) {
    let start = state.uniforms.len();
    let count = (r.capacity as usize).div_ceil(BATCH);
    let m = &r.materials[0];
    let t = m
        .textures
        .iter()
        .find(|t| t.property == "_MainTex")
        .unwrap();
    let image = |id: u32| {
        lighting
            .textures
            .iter()
            .find(|(texture, _, _)| *texture == id)
            .unwrap()
            .2
            .clone()
    };
    let texture = image(t.texture);
    let shader = ParticleShader::new(&m.shader.name);
    let legacy = shader.legacy();
    let float = |name: &str| m.floats.get(name).copied().unwrap_or(0.);
    // Particles/Standard Unlit EFFECT_BUMP reads `_GrabTexture` through `_BumpMap`.
    let bump = (float("_DistortionEnabled") == 1.)
        .then(|| m.textures.iter().find(|t| t.property == "_BumpMap"))
        .flatten();
    let tint = match shader {
        ParticleShader::MobileAdditive => [1.; 4],
        _ if legacy => m.colors["_TintColor"],
        _ => m.colors["_Color"],
    };
    let mut uniform = ParticleUniform::default();
    uniform.tint = Vec4::from_array(linear(tint));
    uniform.st = Vec4::new(t.scale[0], t.scale[1], t.offset[0], t.offset[1]);
    uniform.settings = Vec4::new(
        if legacy { 2. } else { 1. },
        r.scalars["renderer.m_MinParticleSize"],
        r.scalars["renderer.m_MaxParticleSize"],
        0.,
    );
    // Source Ultra enables SOFTPARTICLES_ON. Only the legacy shaders compile the fade.
    uniform.effect = Vec4::new(
        if legacy { float("_InvFade") } else { 0. },
        if bump.is_some() {
            float("_DistortionStrengthScaled")
        } else {
            0.
        },
        float("_DistortionBlend"),
        float("_BumpScale"),
    );
    let bump = bump.map_or_else(|| texture.clone(), |b| image(b.texture));
    let mut entities = Vec::with_capacity(count);
    for i in 0..count {
        state.uniforms.push(uniform);
        let material = materials.add(ParticleMaterial {
            batch: start + i,
            texture: texture.clone(),
            bump: bump.clone(),
            additive: shader.additive(),
            order: -r.scalars["renderer.m_SortingFudge"],
        });
        entities.push(
            commands
                .spawn((
                    super::renderer_visibility::SourceSortData {
                        material_key: (start + i) as u32,
                        material_identity: (start + i) as u32,
                        glass_source: 0,
                        shader_identity: m.shader.source.path_id as i32,
                        source_material_index: 0,
                        layer_and_order: 0,
                        queue: 3000,
                        additive: false,
                        glass: false,
                        grab: uniform.effect.y != 0.,
                    },
                    Mesh3d(state.mesh.as_ref().unwrap().clone()),
                    MeshMaterial3d(material),
                    Transform::IDENTITY,
                    Visibility::Hidden,
                    bevy::light::NotShadowCaster,
                    bevy::camera::visibility::NoFrustumCulling,
                    bevy::camera::primitives::Aabb {
                        center: bevy::math::Vec3A::ZERO,
                        half_extents: bevy::math::Vec3A::ZERO,
                    },
                    bevy::camera::visibility::RenderLayers::layer(
                        r.scalars["gameObject.layer"] as usize,
                    ),
                ))
                .id(),
        );
    }
    state.groups.push(Group {
        owner,
        source: index,
        start,
        count,
        order: Vec::with_capacity(r.capacity as usize),
        entities,
        black_fog: shader.additive(),
    });
}
#[allow(clippy::too_many_arguments)]
fn sync_particles(
    mut commands: Commands,
    reference: Res<RuntimeReference>,
    drive: Res<Drive>,
    lighting: Res<SourceLighting>,
    mut state: ResMut<ParticleRender>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ParticleMaterial>>,
    mut visibility: Query<
        (&mut Visibility, &mut GlobalTransform),
        Without<super::camera::DriveCamera>,
    >,
    camera: Query<&GlobalTransform, With<super::camera::DriveCamera>>,
) {
    if !lighting.bound {
        return;
    }
    let references = &reference.0.particles;
    if state.mesh.is_none() {
        state.mesh = Some(meshes.add(mesh()));
        for (i, p) in references.iter().enumerate() {
            let owner = if p.role.starts_with("urine") {
                Owner::Urine
            } else {
                Owner::Outlet(p.role.rsplit('_').next().unwrap().parse().unwrap())
            };
            add_group(
                &mut state,
                owner,
                i,
                p,
                &lighting,
                &mut commands,
                &mut materials,
            );
        }
        for (wash, w) in reference.0.conditions.washes.iter().enumerate() {
            for (particle, p) in w.particles.iter().enumerate() {
                add_group(
                    &mut state,
                    Owner::Wash(wash),
                    particle,
                    p,
                    &lighting,
                    &mut commands,
                    &mut materials,
                );
            }
        }
        for (tool, b) in reference.0.conditions.brushes.iter().enumerate() {
            for (particle, p) in b.particles.iter().enumerate() {
                add_group(
                    &mut state,
                    Owner::Brush(tool),
                    particle,
                    p,
                    &lighting,
                    &mut commands,
                    &mut materials,
                );
            }
        }
        for (tool, b) in reference.0.conditions.sprays.iter().enumerate() {
            for (particle, p) in b.particles.iter().enumerate() {
                add_group(
                    &mut state,
                    Owner::Spray(tool),
                    particle,
                    p,
                    &lighting,
                    &mut commands,
                    &mut materials,
                );
            }
        }
    }
    let origin = drive.runtime.stream.origin.offset;
    let Ok(camera) = camera.single() else {
        return;
    };
    let eye = camera.translation();
    let cycle = drive.runtime.lighting.cycle.output();
    let fog = linear(cycle.fog_color);
    let state = &mut *state;
    for g in &mut state.groups {
        let p = match g.owner {
            Owner::Spray(tool) => &reference.0.conditions.sprays[tool].particles[g.source],
            Owner::Brush(tool) => &reference.0.conditions.brushes[tool].particles[g.source],
            Owner::Wash(wash) => &reference.0.conditions.washes[wash].particles[g.source],
            Owner::Urine | Owner::Outlet(_) => &references[g.source],
        };
        let fluid = match g.owner {
            Owner::Urine => Some(&drive.runtime.urine),
            Owner::Outlet(id) => drive
                .runtime
                .area
                .interaction
                .fluid_particles()
                .find(|(key, _)| key.0 == id)
                .map(|(_, p)| p),
            Owner::Brush(_) | Owner::Spray(_) | Owner::Wash(_) => None,
        };
        let splash = p.role.starts_with("splash") || p.role == "urine_splash";
        let particles = if let Owner::Wash(wash) = g.owner {
            Some(
                drive
                    .runtime
                    .area
                    .interaction
                    .cleaning
                    .wash_particles(wash, g.source),
            )
        } else if let Owner::Spray(tool) = g.owner {
            Some(
                drive
                    .runtime
                    .area
                    .interaction
                    .cleaning
                    .spray_particles(tool, g.source),
            )
        } else if let Owner::Brush(tool) = g.owner {
            Some(
                drive
                    .runtime
                    .area
                    .interaction
                    .cleaning
                    .particles(tool, g.source),
            )
        } else if splash {
            fluid.map(|f| f.splash())
        } else {
            None
        };
        let count = if !p.flags["gameObject.active"] || !p.flags["renderer.enabled"] {
            0
        } else {
            particles.map_or_else(|| fluid.map_or(0, |f| f.particles().len()), |p| p.len())
        };
        g.order.clear();
        g.order.extend(0..count);
        let position = |i: usize| -> Vec3 {
            if let Some(p) = particles {
                Vec3::from_array(std::array::from_fn(|j| {
                    (p[i].absolute_position[j] - origin[j]) as f32
                }))
            } else {
                Vec3::from_array(fluid.unwrap().particles()[i].position.to_array())
            }
        };
        if p.scalars["renderer.m_SortMode"] == 1. {
            g.order.sort_unstable_by(|a, b| {
                position(*b)
                    .distance_squared(eye)
                    .total_cmp(&position(*a).distance_squared(eye))
            });
        }
        for batch in 0..g.count {
            let u = &mut state.uniforms[g.start + batch];
            u.fog = if g.black_fog {
                Vec4::new(0., 0., 0., cycle.fog_density)
            } else {
                Vec4::new(fog[0], fog[1], fog[2], cycle.fog_density)
            };
            let n = count.saturating_sub(batch * BATCH).min(BATCH);
            u.settings.w = n as f32;
            if let Ok((mut v, mut transform)) = visibility.get_mut(g.entities[batch]) {
                if n > 0 {
                    *transform =
                        GlobalTransform::from_translation(position(g.order[batch * BATCH + n / 2]));
                }
                *v = if n == 0 {
                    Visibility::Hidden
                } else {
                    Visibility::Visible
                };
            }
            for j in 0..n {
                let index = g.order[batch * BATCH + j];
                let (size, color, angle) = if let Some(p) = particles {
                    (p[index].size, p[index].color, p[index].rotation)
                } else {
                    let p = &fluid.unwrap().particles()[index];
                    (p.size, p.color, p.rotation)
                };
                u.positions[j] = position(index).extend(size);
                u.colors[j] = Vec4::from_array(linear(color));
                u.angles[j] = Vec4::new(angle.cos(), angle.sin(), 0., 0.);
            }
        }
    }
}

fn linear(c: [f32; 4]) -> [f32; 4] {
    [
        crate::lighting::source_linear(c[0]),
        crate::lighting::source_linear(c[1]),
        crate::lighting::source_linear(c[2]),
        c[3],
    ]
}
