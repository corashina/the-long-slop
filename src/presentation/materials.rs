use bevy::{asset as bevy_asset, reflect as bevy_reflect};
// Shared receiver bindings. Meshes retain StandardMaterial surface and alpha data.
use super::lighting::LightingGpu;
use bevy::ecs::{self as bevy_ecs, system::SystemParamItem};
use bevy::pbr::{ExtendedMaterial, MaterialExtension};
use bevy::prelude::*;
use bevy::render::{render_resource::*, renderer::RenderDevice};
use bevy::shader::ShaderRef;

pub type SourceMaterial = ExtendedMaterial<StandardMaterial, SourceExtension>;
#[derive(Asset, TypePath, Clone, Debug, Default)]
pub struct SourceExtension {
    pub receiver_mask: u32,
}
impl MaterialExtension for SourceExtension {
    fn fragment_shader() -> ShaderRef {
        "shaders/source_lighting.wgsl".into()
    }
}
impl AsBindGroup for SourceExtension {
    type Data = ();
    type Param = SRes<LightingGpu>;
    fn label() -> &'static str {
        "source lighting"
    }
    fn bind_group_data(&self) {}
    fn unprepared_bind_group(
        &self,
        _: &BindGroupLayout,
        device: &RenderDevice,
        gpu: &mut SystemParamItem<Self::Param>,
        _: bool,
    ) -> Result<UnpreparedBindGroup, AsBindGroupError> {
        let cookies = gpu
            .cookies
            .as_ref()
            .ok_or(AsBindGroupError::RetryNextUpdate)?;
        Ok(UnpreparedBindGroup {
            bindings: BindingResources(vec![
                (
                    100,
                    OwnedBindingResource::Buffer(
                        gpu.uniform
                            .buffer()
                            .ok_or(AsBindGroupError::RetryNextUpdate)?
                            .clone(),
                    ),
                ),
                (
                    101,
                    OwnedBindingResource::TextureView(
                        TextureViewDimension::D2,
                        gpu.atlas_view.clone(),
                    ),
                ),
                (
                    102,
                    OwnedBindingResource::Sampler(
                        SamplerBindingType::Comparison,
                        gpu.shadow_sampler.clone(),
                    ),
                ),
                (
                    113,
                    OwnedBindingResource::TextureView(
                        TextureViewDimension::D2,
                        gpu.atlas_view.clone(),
                    ),
                ),
                (
                    112,
                    OwnedBindingResource::Sampler(
                        SamplerBindingType::Comparison,
                        gpu.hard_shadow_sampler.clone(),
                    ),
                ),
                (
                    103,
                    OwnedBindingResource::TextureView(
                        TextureViewDimension::D2,
                        cookies[2].0.clone(),
                    ),
                ),
                (
                    104,
                    OwnedBindingResource::Sampler(
                        SamplerBindingType::Filtering,
                        cookies[2].1.clone(),
                    ),
                ),
                (
                    107,
                    OwnedBindingResource::TextureView(
                        TextureViewDimension::D2,
                        cookies[1].0.clone(),
                    ),
                ),
                (
                    108,
                    OwnedBindingResource::Sampler(
                        SamplerBindingType::Filtering,
                        cookies[1].1.clone(),
                    ),
                ),
                (
                    109,
                    OwnedBindingResource::TextureView(
                        TextureViewDimension::D2,
                        cookies[0].0.clone(),
                    ),
                ),
                (
                    110,
                    OwnedBindingResource::Sampler(
                        SamplerBindingType::Filtering,
                        cookies[0].1.clone(),
                    ),
                ),
                (
                    111,
                    OwnedBindingResource::Sampler(
                        SamplerBindingType::Filtering,
                        gpu.cookie_sampler.clone(),
                    ),
                ),
                (
                    105,
                    OwnedBindingResource::TextureView(
                        TextureViewDimension::D2,
                        gpu.attenuation_view.clone(),
                    ),
                ),
                (
                    106,
                    OwnedBindingResource::Buffer(
                        device.create_buffer_with_data(&BufferInitDescriptor {
                            label: Some("source receiver mask"),
                            contents: &[self.receiver_mask.to_le_bytes(), [0; 4], [0; 4], [0; 4]]
                                .concat(),
                            usage: BufferUsages::UNIFORM,
                        }),
                    ),
                ),
            ]),
        })
    }
    fn bind_group_layout_entries(_: &RenderDevice, _: bool) -> Vec<BindGroupLayoutEntry> {
        use binding_types::*;
        BindGroupLayoutEntries::with_indices(
            ShaderStages::FRAGMENT,
            (
                (
                    100,
                    uniform_buffer::<super::lighting::LightingUniform>(false),
                ),
                (101, texture_2d(TextureSampleType::Depth)),
                (102, sampler(SamplerBindingType::Comparison)),
                (
                    103,
                    texture_2d(TextureSampleType::Float { filterable: true }),
                ),
                (104, sampler(SamplerBindingType::Filtering)),
                (
                    105,
                    texture_2d(TextureSampleType::Float { filterable: true }),
                ),
                (106, uniform_buffer::<UVec4>(false)),
                (
                    107,
                    texture_2d(TextureSampleType::Float { filterable: true }),
                ),
                (108, sampler(SamplerBindingType::Filtering)),
                (
                    109,
                    texture_2d(TextureSampleType::Float { filterable: true }),
                ),
                (110, sampler(SamplerBindingType::Filtering)),
                (111, sampler(SamplerBindingType::Filtering)),
                (112, sampler(SamplerBindingType::Comparison)),
                (113, texture_2d(TextureSampleType::Depth)),
            ),
        )
        .to_vec()
    }
}
use bevy::ecs::system::lifetimeless::SRes;

#[derive(Component)]
pub struct SourceCaster;
#[derive(Component, Clone, Copy)]
pub struct SourceShadowMode(pub u8);

#[derive(Resource, Default)]
pub struct SourceMaterials {
    pub variants: std::collections::HashMap<
        (AssetId<StandardMaterial>, bool, bool),
        Handle<StandardMaterial>,
    >,
    pub converted:
        std::collections::HashMap<(AssetId<StandardMaterial>, u32), Handle<SourceMaterial>>,
}
pub fn bind_materials(
    mut commands: Commands,
    visuals: Res<super::reference::ReferenceVisuals>,
    mut originals: ResMut<Assets<StandardMaterial>>,
    mut materials: ResMut<Assets<SourceMaterial>>,
    mut cache: ResMut<SourceMaterials>,
    imported_nodes: Query<&bevy::gltf::GltfExtras>,
    query: Query<
        (
            Entity,
            &Mesh3d,
            &MeshMaterial3d<StandardMaterial>,
            Option<&bevy::camera::visibility::RenderLayers>,
            Option<&SourceShadowMode>,
            Option<&ChildOf>,
        ),
        (Without<SourceCaster>, Without<super::sky::SkyMesh>),
    >,
) {
    if !visuals.bound {
        return;
    }
    for (entity, mesh, handle, layers, shadow_mode, parent) in &query {
        if shadow_mode.is_none()
            && parent.is_some_and(|parent| imported_nodes.contains(parent.parent()))
        {
            continue;
        }
        let mask = layers.map_or(1, |layers| {
            layers.iter().fold(0, |mask, layer| {
                mask | 1u32.checked_shl(layer as u32).unwrap_or(0)
            })
        });
        let Some(base) = originals.get(&handle.0).cloned() else {
            continue;
        };
        let mode = shadow_mode.map_or(1, |mode| mode.0);
        if mode != 0 {
            let caster_material = if mode == 2 {
                cache
                    .variants
                    .entry((handle.0.id(), false, true))
                    .or_insert_with(|| {
                        let mut material = base.clone();
                        material.cull_mode = None;
                        originals.add(material)
                    })
                    .clone()
            } else {
                handle.0.clone()
            };
            let layers = (0..32)
                .filter(|layer| mask & (1 << layer) != 0)
                .map(|layer| layer + 32)
                .collect::<bevy::camera::visibility::RenderLayers>();
            commands.spawn((
                SourceCaster,
                mesh.clone(),
                MeshMaterial3d(caster_material),
                layers,
                Transform::default(),
                Visibility::Inherited,
                ChildOf(entity),
            ));
        }
        if mode == 3 {
            commands
                .entity(entity)
                .insert(bevy::camera::visibility::RenderLayers::none());
        }
        let material = cache
            .converted
            .entry((handle.0.id(), mask))
            .or_insert_with(|| {
                let mut base = base.clone();
                base.fog_enabled = false;
                materials.add(SourceMaterial {
                    base,
                    extension: SourceExtension {
                        receiver_mask: mask,
                    },
                })
            })
            .clone();
        commands
            .entity(entity)
            .remove::<MeshMaterial3d<StandardMaterial>>()
            .insert((MeshMaterial3d(material), bevy::light::NotShadowCaster));
    }
}

pub struct LampReceiverChoice {
    entity: Entity,
    material: Handle<SourceMaterial>,
    casters: Vec<(Entity, Handle<StandardMaterial>)>,
}
pub struct LampMaterialChoice {
    key: (crate::reference::AssetId, u32),
    receivers: Vec<LampReceiverChoice>,
}

/// Prepare source selections and actual mesh pipeline variants during loading.
pub(crate) fn prepare_lamp_materials(
    mut commands: Commands,
    reference: Res<crate::app::RuntimeReference>,
    drive: Res<crate::app::Drive>,
    instruments: Res<super::instruments::InstrumentVisuals>,
    mut source: ResMut<super::lighting::SourceLighting>,
    mut originals: ResMut<Assets<StandardMaterial>>,
    mut materials: ResMut<Assets<SourceMaterial>>,
    mut cache: ResMut<SourceMaterials>,
    receivers: Query<
        (
            &Mesh3d,
            &MeshMaterial3d<SourceMaterial>,
            &GlobalTransform,
            Option<&SourceShadowMode>,
            Option<&Children>,
        ),
        Without<SourceCaster>,
    >,
    casters: Query<(), With<SourceCaster>>,
) {
    if !source.bound
        || !instruments.prepared
        || source.materials_prepared
        || !source.materials_ready
        || source.road_lamps.recipe.is_none()
    {
        return;
    }
    if source.lamp_materials.iter().any(|target| {
        target
            .receivers
            .iter()
            .any(|entity| !receivers.contains(*entity))
    }) {
        return;
    }
    let source = &mut *source;
    let far = Transform::from_translation(Vec3::splat(1_000_000.));
    for target in &mut source.lamp_materials {
        let output = &drive.runtime.lighting.material_outputs()[target.output_index];
        let mut keys = Vec::new();
        for state in reference
            .0
            .lighting
            .controllers
            .iter()
            .flat_map(|c| &c.states)
        {
            if !state.renderers.contains(&output.renderer) {
                continue;
            }
            for material in [&state.on_material, &state.off_material]
                .into_iter()
                .flatten()
            {
                let key = (material.asset, material.gltf_material);
                if !keys.contains(&key) {
                    keys.push(key);
                }
            }
        }
        let dashboard = &reference.0.lighting.high_beam;
        if dashboard.node == output.renderer {
            for material in [&dashboard.on, &dashboard.off] {
                let key = (material.asset, material.gltf_material);
                if !keys.contains(&key) {
                    keys.push(key);
                }
            }
        }
        for key in keys {
            let authored = source
                .materials
                .iter()
                .find(|m| (m.asset, m.index) == key)
                .expect("validated lamp material");
            let authored_base = originals
                .get(&authored.handle)
                .expect("loaded lamp material")
                .clone();
            let mut choice = LampMaterialChoice {
                key,
                receivers: Vec::with_capacity(target.receivers.len()),
            };
            for &entity in &target.receivers {
                let (mesh, receiver, global, shadow_mode, children) =
                    receivers.get(entity).unwrap();
                let mask = materials.get(&receiver.0).unwrap().extension.receiver_mask;
                let inverted = global.affine().matrix3.determinant() < 0.;
                let mut base = authored_base.clone();
                if inverted {
                    base.cull_mode = base.cull_mode.map(|face| match face {
                        Face::Front => Face::Back,
                        Face::Back => Face::Front,
                    });
                }
                let surface = if inverted {
                    cache
                        .variants
                        .entry((authored.handle.id(), true, false))
                        .or_insert_with(|| originals.add(base.clone()))
                        .clone()
                } else {
                    authored.handle.clone()
                };
                let caster = if shadow_mode.is_some_and(|mode| mode.0 == 2) {
                    cache
                        .variants
                        .entry((authored.handle.id(), inverted, true))
                        .or_insert_with(|| {
                            let mut material = base.clone();
                            material.cull_mode = None;
                            originals.add(material)
                        })
                        .clone()
                } else {
                    surface.clone()
                };
                let material = cache
                    .converted
                    .entry((surface.id(), mask))
                    .or_insert_with(|| {
                        base.fog_enabled = false;
                        materials.add(SourceMaterial {
                            base,
                            extension: SourceExtension {
                                receiver_mask: mask,
                            },
                        })
                    })
                    .clone();
                let warm = commands
                    .spawn((
                        mesh.clone(),
                        MeshMaterial3d(material.clone()),
                        far,
                        Visibility::Visible,
                        bevy::light::NotShadowCaster,
                        bevy::camera::visibility::NoFrustumCulling,
                    ))
                    .id();
                source.warmups.push((warm, false));
                let mut binding = LampReceiverChoice {
                    entity,
                    material,
                    casters: Vec::new(),
                };
                if let Some(children) = children {
                    for child in children.iter().filter(|child| casters.contains(*child)) {
                        binding.casters.push((child, caster.clone()));
                    }
                }
                if !binding.casters.is_empty() {
                    let warm = commands
                        .spawn((
                            SourceCaster,
                            mesh.clone(),
                            MeshMaterial3d(caster),
                            far,
                            Visibility::Visible,
                            (32..64).collect::<bevy::camera::visibility::RenderLayers>(),
                            bevy::camera::visibility::NoFrustumCulling,
                        ))
                        .id();
                    source.warmups.push((warm, true));
                }
                choice.receivers.push(binding);
            }
            target.choices.push(choice);
        }
    }
    source.materials_prepared = true;
    info!(
        "Source lamp cache: {} targets, {} choices, {} materials, {} warmup meshes",
        source.lamp_materials.len(),
        source
            .lamp_materials
            .iter()
            .map(|t| t.choices.len())
            .sum::<usize>(),
        cache.converted.len(),
        source.warmups.len()
    );
}

/// Select cached handles. Gameplay does not allocate materials or mesh variants.
pub(crate) fn sync_lamp_materials(
    mut commands: Commands,
    drive: Res<crate::app::Drive>,
    mut source: ResMut<super::lighting::SourceLighting>,
) {
    if !source.materials_prepared {
        return;
    }
    for target in &mut source.lamp_materials {
        let output = &drive.runtime.lighting.material_outputs()[target.output_index];
        let key = (output.asset, output.material_index);
        if target.current == Some(key) {
            continue;
        }
        let choice = target
            .choices
            .iter()
            .find(|choice| choice.key == key)
            .expect("prepared source material choice");
        for receiver in &choice.receivers {
            commands
                .entity(receiver.entity)
                .insert(MeshMaterial3d(receiver.material.clone()));
            for (entity, material) in &receiver.casters {
                commands
                    .entity(*entity)
                    .insert(MeshMaterial3d(material.clone()));
            }
        }
        target.current = Some(key);
    }
}
