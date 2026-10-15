//! Persistent source camera-depth and directional-shadow caster state.
use super::{
    materials::RenderingState,
    render_targets::{DEPTH_FORMAT, SHADOW_FORMAT},
    source_lighting::SourceLightingStatus,
};
use bevy::ecs as bevy_ecs;
use bevy::prelude::*;
use bevy::render::{
    render_resource::{binding_types::*, *},
    renderer::{RenderDevice, RenderQueue},
};
use bevy::shader::{ShaderCacheError, ShaderDefVal};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, ShaderType)]
pub struct CasterUniform {
    pub clip_from_world: Mat4,
    pub direction_to_light: Vec4,
    pub bias: Vec4,
    pub view_from_world: Mat4,
    pub world_from_view: Mat4,
    pub clip_from_view: Mat4,
}
pub fn caster_layout() -> BindGroupLayoutDescriptor {
    BindGroupLayoutDescriptor::new(
        "source_caster_view",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::VERTEX_FRAGMENT,
            (
                uniform_buffer::<CasterUniform>(false),
                texture_3d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
            ),
        ),
    )
}
struct CasterBinding {
    uniform: UniformBuffer<CasterUniform>,
    binding: BindGroup,
}
/// Slot0 is camera depth. Slots1..4 are the four source cascades.
/// Each source view owns this component; queued views never overwrite another view's state.
#[derive(Component)]
pub struct CasterViews {
    slots: [CasterBinding; 5],
}
impl CasterViews {
    pub fn new(
        device: &RenderDevice,
        queue: &RenderQueue,
        cache: &PipelineCache,
        dither: &TextureView,
        sampler: &Sampler,
    ) -> Self {
        let slots = std::array::from_fn(|_| {
            let mut uniform = UniformBuffer::from(CasterUniform {
                clip_from_world: Mat4::IDENTITY,
                direction_to_light: Vec4::Y,
                bias: Vec4::ZERO,
                view_from_world: Mat4::IDENTITY,
                world_from_view: Mat4::IDENTITY,
                clip_from_view: Mat4::IDENTITY,
            });
            uniform.set_label(Some("source_caster_uniform"));
            uniform.write_buffer(device, queue);
            let binding = device.create_bind_group(
                "source_caster_view",
                &cache.get_bind_group_layout(&caster_layout()),
                &BindGroupEntries::sequential((uniform.binding().unwrap(), dither, sampler)),
            );
            CasterBinding { uniform, binding }
        });
        Self { slots }
    }
    pub fn write(
        &mut self,
        index: usize,
        value: CasterUniform,
        device: &RenderDevice,
        queue: &RenderQueue,
    ) {
        self.slots[index].uniform.set(value);
        self.slots[index].uniform.write_buffer(device, queue);
    }
    pub fn uniform(&self, index: usize) -> &CasterUniform {
        self.slots[index].uniform.get()
    }
    pub fn binding(&self, index: usize) -> &BindGroup {
        &self.slots[index].binding
    }
}
#[derive(Resource, Default)]
pub struct CasterPasses {
    inspected: usize,
    isolated: HashSet<CachedRenderPipelineId>,
    classified: HashSet<CachedRenderPipelineId>,
    variants: HashMap<CachedRenderPipelineId, [Option<CachedRenderPipelineId>; 2]>,
}
fn flag(defs: &[ShaderDefVal], name: &str) -> Option<bool> {
    defs.iter().find_map(|d| match d {
        ShaderDefVal::Bool(k, v) if k == name => Some(*v),
        _ => None,
    })
}
impl CasterPasses {
    pub fn revisit(&mut self, id: CachedRenderPipelineId) {
        self.inspected = self.inspected.min(id.id());
    }
    pub fn companion_state(
        &self,
        base: CachedRenderPipelineId,
        cache: &PipelineCache,
    ) -> Option<Result<bool, String>> {
        self.variants.get(&base).map(|ids| {
            let mut ready = true;
            for id in ids.iter().flatten() {
                match cache.get_render_pipeline_state(*id) {
                    CachedPipelineState::Ok(_) => {}
                    CachedPipelineState::Err(
                        ShaderCacheError::ShaderNotLoaded(_)
                        | ShaderCacheError::ShaderImportNotYetAvailable,
                    ) => ready = false,
                    CachedPipelineState::Err(error) => {
                        return Err(format!("Source caster pipeline: {error}"));
                    }
                    _ => ready = false,
                }
            }
            Ok(ready)
        })
    }
    pub fn counts(&self) -> (usize, usize) {
        (
            self.variants.len(),
            self.variants
                .values()
                .flatten()
                .filter(|id| id.is_some())
                .count(),
        )
    }
    pub fn classified(&self, base: CachedRenderPipelineId) -> bool {
        self.classified.contains(&base)
    }
    pub fn pipeline(
        &self,
        base: CachedRenderPipelineId,
        shadow: bool,
    ) -> Option<CachedRenderPipelineId> {
        self.variants
            .get(&base)
            .and_then(|p| p[usize::from(shadow)])
    }
    pub fn ready(&self, cache: &PipelineCache) -> bool {
        !self.variants.is_empty()
            && self
                .variants
                .iter()
                .filter(|(base, _)| !self.isolated.contains(base))
                .flat_map(|(_, ids)| ids.iter().flatten())
                .all(|id| cache.get_render_pipeline(*id).is_some())
    }
    /// Called during preparation. Descriptors are cloned only once per new source surface variant.
    pub fn prepare(
        &mut self,
        cache: &PipelineCache,
        shader: &Handle<Shader>,
        status: &SourceLightingStatus,
        isolation: &super::world::pipelines::PipelineIsolation,
        owners: &super::pipeline_ownership::PipelineOwners,
    ) {
        self.isolated.clone_from(&isolation.0);
        if matches!(*status.0.lock().unwrap(), RenderingState::Failed(_)) {
            return;
        }
        let first = self.inspected;
        for (index, entry) in cache.pipelines().enumerate().skip(first) {
            self.inspected = index + 1;
            let base_id = CachedRenderPipelineId::new(index);
            if self.classified.contains(&base_id)
                || owners.owner(base_id).is_some_and(|o| o.cancelled())
            {
                continue;
            }
            let PipelineDescriptor::RenderPipelineDescriptor(base) = &entry.descriptor else {
                continue;
            };
            let Some(fragment) = &base.fragment else {
                continue;
            };
            let world_shader = flag(&fragment.shader_defs, "SOURCE_WORLD") == Some(true);
            if flag(&fragment.shader_defs, "SOURCE_SURFACE") != Some(true) && !world_shader
                || flag(&fragment.shader_defs, "SOURCE_FORWARD_ADD") == Some(true)
            {
                continue;
            }
            self.classified.insert(CachedRenderPipelineId::new(index));
            if flag(&fragment.shader_defs, "SOURCE_STANDARD") != Some(true) && !world_shader {
                continue;
            }
            let Some(reflected) = flag(&fragment.shader_defs, "SOURCE_OBJECT_REFLECTED") else {
                let error = "Source caster requires object transform parity metadata".to_string();
                if let Some(owner) = owners.owner(CachedRenderPipelineId::new(index)) {
                    owner.fail(error);
                } else {
                    *status.0.lock().unwrap() = RenderingState::Failed(error);
                }
                return;
            };
            let base_id = CachedRenderPipelineId::new(index);
            let owner = owners.owner(base_id);
            let ids = std::array::from_fn(|index| {
                let _queue = if let Some(owner) = &owner {
                    match super::pipeline_ownership::queue_scope(cache) {
                        Ok(scope) => Some(scope),
                        Err(error) => {
                            owner.fail(error.into());
                            return None;
                        }
                    }
                } else {
                    None
                };
                let mut descriptor = base.clone();
                let shadow = index == 1;
                if !shadow && flag(&fragment.shader_defs, "SOURCE_TRANSPARENT") == Some(true) {
                    return None;
                }
                descriptor.label = Some(
                    if shadow {
                        "source_shadow_caster"
                    } else {
                        "source_camera_depth"
                    }
                    .into(),
                );
                descriptor.layout[0] = super::source_lighting::source_camera_layout();
                descriptor.layout[1] = caster_layout();
                if !world_shader
                    && flag(&fragment.shader_defs, "WORLD_TEXTURE_ARRAYS") != Some(true)
                {
                    descriptor.layout[3] = super::caster_material::layout();
                }
                descriptor.multisample = MultisampleState::default();
                if !world_shader {
                    descriptor.vertex.shader = shader.clone();
                }
                descriptor.vertex.entry_point = Some("vertex".into());
                let fragment = descriptor.fragment.as_mut().unwrap();
                if !world_shader {
                    fragment.shader = shader.clone();
                }
                fragment.entry_point = Some("fragment".into());
                fragment.targets.clear();
                for defs in [
                    &mut descriptor.vertex.shader_defs,
                    &mut fragment.shader_defs,
                ] {
                    defs.retain(|v|!matches!(v,ShaderDefVal::Bool(n,_)if n=="SOURCE_SURFACE"||n=="SOURCE_FORWARD_ADD"||n=="SOURCE_WORLD"));
                    defs.push(ShaderDefVal::Bool("SOURCE_CASTER".into(), true));
                    if world_shader {
                        defs.push(ShaderDefVal::Bool("PREPASS_PIPELINE".into(), true));
                    }
                }
                if !world_shader {
                    descriptor.primitive.cull_mode = Some(Face::Back);
                }
                // RenderShadowMaps sets userBackfaceMode=false for Directional1, even inside cube capture.
                if shadow {
                    descriptor.primitive.front_face = if reflected {
                        FrontFace::Ccw
                    } else {
                        FrontFace::Cw
                    };
                }
                descriptor.depth_stencil = Some(DepthStencilState {
                    format: if shadow { SHADOW_FORMAT } else { DEPTH_FORMAT },
                    depth_write_enabled: Some(true),
                    depth_compare: Some(CompareFunction::GreaterEqual),
                    stencil: StencilState::default(),
                    bias: DepthBiasState::default(),
                });
                let id = cache.queue_render_pipeline(*descriptor);
                owners.companion(cache, base_id, id);
                Some(id)
            });
            self.variants
                .insert(CachedRenderPipelineId::new(index), ids);
        }
        for id in self
            .variants
            .iter()
            .filter(|(base, _)| !self.isolated.contains(base))
            .flat_map(|(_, ids)| ids.iter().flatten())
        {
            match cache.get_render_pipeline_state(*id) {
                CachedPipelineState::Err(
                    ShaderCacheError::ShaderNotLoaded(_)
                    | ShaderCacheError::ShaderImportNotYetAvailable,
                ) => {}
                CachedPipelineState::Err(error) => {
                    *status.0.lock().unwrap() =
                        RenderingState::Failed(format!("Source caster pipeline: {error}"));
                    return;
                }
                _ => {}
            }
        }
    }
}

/// Prepare once per view, then update only its existing camera-depth uniform buffer.
/// Shadow slots remain untouched until the source candidate set and cascade fit are ready.
pub fn prepare_views(
    mut commands: Commands,
    assets: Option<Res<super::lighting_assets::LightingRenderAssets>>,
    scene: Res<super::lighting_scene::SourceSceneLighting>,
    images: Res<bevy::render::render_asset::RenderAssets<bevy::render::texture::GpuImage>>,
    mut views: Query<
        (
            Entity,
            &bevy::render::view::ExtractedView,
            Option<&mut CasterViews>,
        ),
        With<super::surface_views::SourceSurfaceView>,
    >,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    cache: Res<PipelineCache>,
    status: Res<SourceLightingStatus>,
) {
    if matches!(*status.0.lock().unwrap(), RenderingState::Failed(_)) {
        return;
    }
    let (Some(assets), Some(scene)) = (assets, scene.0) else {
        return;
    };
    let Some(dither) = images.get(assets.caster_dither.id()) else {
        return;
    };
    for (entity, view, existing) in &mut views {
        // This is the same extracted projection used by the surface vertex path.
        // Explicit capture matrices retain their improper handedness and bypass pose reconstruction.
        let clip_from_world = view
            .clip_from_world
            .unwrap_or_else(|| view.clip_from_view * view.world_from_view.affine().inverse());
        let uniform = CasterUniform {
            clip_from_world,
            view_from_world: view.world_from_view.to_matrix().inverse(),
            world_from_view: view.world_from_view.to_matrix(),
            clip_from_view: view.clip_from_view,
            direction_to_light: scene.direction_to_sun,
            // DepthPassJob337330 clears unity_LightShadowBias before camera-depth draws.
            bias: Vec4::ZERO,
        };
        if let Some(mut existing) = existing {
            existing.write(0, uniform, &device, &queue);
        } else {
            let mut bindings = CasterViews::new(
                &device,
                &queue,
                &cache,
                &dither.texture_view,
                &dither.sampler,
            );
            bindings.write(0, uniform, &device, &queue);
            commands
                .entity(entity)
                .insert((bindings, super::camera_depth::CameraDepthState::default()));
        }
    }
}

pub fn prepare_passes(
    mut passes: ResMut<CasterPasses>,
    assets: Option<Res<super::lighting_assets::LightingRenderAssets>>,
    cache: Res<PipelineCache>,
    status: Res<SourceLightingStatus>,
    isolation: Res<super::world::pipelines::PipelineIsolation>,
    owners: Res<super::pipeline_ownership::PipelineOwners>,
) {
    let Some(assets) = assets else {
        return;
    };
    passes.prepare(&cache, &assets.shaders.caster, &status, &isolation, &owners);
}
