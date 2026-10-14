//! Depth and shadow draws share the surface alpha data, without surface lighting textures.
use super::materials::{SourceMaterial, SourceUniform};
use bevy::asset::{AssetEvent, AssetId};
use bevy::ecs::{
    self as bevy_ecs,
    query::ROQueryItem,
    system::{SystemParamItem, lifetimeless::SRes},
};
use bevy::pbr::RenderMaterialInstances;
use bevy::prelude::*;
use bevy::render::{
    Extract, ExtractSchedule, Render, RenderApp, RenderSystems,
    render_asset::RenderAssets,
    render_phase::{PhaseItem, RenderCommand, RenderCommandResult, TrackedRenderPass},
    render_resource::{binding_types::*, *},
    renderer::{RenderDevice, RenderQueue},
    texture::{FallbackImage, GpuImage},
};
use std::collections::HashMap;

struct Input {
    revision: u64,
    data: SourceUniform,
    texture: Option<AssetId<Image>>,
}
#[derive(Resource, Default)]
struct Inputs {
    revision: u64,
    materials: HashMap<AssetId<SourceMaterial>, Input>,
}
struct Material {
    revision: u64,
    image: (TextureViewId, SamplerId),
    uniform: UniformBuffer<SourceUniform>,
    group: BindGroup,
}
#[derive(Resource, Default)]
pub struct CasterMaterials {
    revision: u64,
    materials: HashMap<AssetId<SourceMaterial>, Material>,
}
pub fn layout() -> BindGroupLayoutDescriptor {
    BindGroupLayoutDescriptor::new(
        "source_caster_material",
        &BindGroupLayoutEntries::with_indices(
            ShaderStages::VERTEX_FRAGMENT,
            (
                (1, texture_2d(TextureSampleType::Float { filterable: true })),
                (2, sampler(SamplerBindingType::Filtering)),
                (120, uniform_buffer::<SourceUniform>(false)),
            ),
        ),
    )
}
fn extract(
    assets: Extract<Res<Assets<SourceMaterial>>>,
    mut events: Extract<MessageReader<AssetEvent<SourceMaterial>>>,
    mut inputs: ResMut<Inputs>,
) {
    for event in events.read() {
        match *event {
            AssetEvent::Added { id } | AssetEvent::Modified { id } => {
                let Some(material) = assets.get(id) else {
                    continue;
                };
                inputs.revision += 1;
                let revision = inputs.revision;
                inputs.materials.insert(
                    id,
                    Input {
                        revision,
                        data: material.caster_data(),
                        texture: material.base.base_color_texture.as_ref().map(Handle::id),
                    },
                );
            }
            AssetEvent::Removed { id } => {
                inputs.revision += 1;
                inputs.materials.remove(&id);
            }
            _ => {}
        }
    }
}
fn prepare(
    inputs: Res<Inputs>,
    mut output: ResMut<CasterMaterials>,
    images: Res<RenderAssets<GpuImage>>,
    white: Res<FallbackImage>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    pipelines: Res<PipelineCache>,
) {
    if output.revision != inputs.revision {
        output
            .materials
            .retain(|id, _| inputs.materials.contains_key(id));
        output.revision = inputs.revision;
    }
    for (&id, input) in &inputs.materials {
        let image = match input.texture {
            Some(id) => {
                let Some(image) = images.get(id) else {
                    continue;
                };
                image
            }
            None => &white.d2,
        };
        let identity = (image.texture_view.id(), image.sampler.id());
        if let Some(material) = output.materials.get_mut(&id) {
            if material.revision != input.revision {
                material.uniform.set(input.data);
                material.uniform.write_buffer(&device, &queue);
                material.revision = input.revision;
            }
            if material.image == identity {
                continue;
            }
        }
        let mut uniform = output.materials.remove(&id).map_or_else(
            || UniformBuffer::from(input.data),
            |material| material.uniform,
        );
        uniform.write_buffer(&device, &queue);
        let group = device.create_bind_group(
            "source_caster_material",
            &pipelines.get_bind_group_layout(&layout()),
            &BindGroupEntries::with_indices((
                (1, &image.texture_view),
                (2, &image.sampler),
                (120, uniform.binding().unwrap()),
            )),
        );
        output.materials.insert(
            id,
            Material {
                revision: input.revision,
                image: identity,
                uniform,
                group,
            },
        );
    }
}
pub struct SetCasterMaterial;
impl<P: PhaseItem> RenderCommand<P> for SetCasterMaterial {
    type Param = (SRes<CasterMaterials>, SRes<RenderMaterialInstances>);
    type ViewQuery = ();
    type ItemQuery = ();
    fn render<'w>(
        item: &P,
        _: ROQueryItem<'w, '_, Self::ViewQuery>,
        _: Option<()>,
        (materials, instances): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(instance) = instances.into_inner().instances.get(&item.main_entity()) else {
            return RenderCommandResult::Skip;
        };
        let Some(material) = materials
            .into_inner()
            .materials
            .get(&instance.asset_id.typed::<SourceMaterial>())
        else {
            return RenderCommandResult::Skip;
        };
        pass.set_bind_group(3, &material.group, &[]);
        RenderCommandResult::Success
    }
}
pub fn install(app: &mut App) {
    app.sub_app_mut(RenderApp)
        .init_resource::<Inputs>()
        .init_resource::<CasterMaterials>()
        .add_systems(ExtractSchedule, extract)
        .add_systems(Render, prepare.in_set(RenderSystems::PrepareBindGroups));
}
