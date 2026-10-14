//! Shared source TMP surface and retained text-local meshes.
use super::{
    materials::{RenderingState, SourceExtension, SourceMaterials},
    reference::ReferenceVisuals,
};
use crate::{
    app::{Drive, RuntimeReference},
    plates::layout::{GlyphMesh, layout_text, sdf_parameters},
    reference::ItemId,
};
use bevy::{
    asset as bevy_asset, ecs as bevy_ecs, image as bevy_image, reflect as bevy_reflect,
    render as bevy_render,
};
use bevy::{
    asset::{AssetId, RenderAssetUsages, UntypedAssetId},
    ecs::system::SystemParamItem,
    mesh::{Indices, MeshVertexBufferLayoutRef},
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    render::{render_resource::*, renderer::RenderDevice},
    shader::ShaderRef,
};

#[derive(Clone, Copy, Debug, ShaderType)]
pub struct PlateUniform {
    face: Vec4,
    outline: Vec4,
    sdf: Vec4,
    scale: Vec4,
    bevel: Vec4,
    bump: Vec4,
    specular: Vec4,
}
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct PlateSurface {
    #[uniform(0)]
    values: PlateUniform,
    #[texture(1)]
    #[sampler(2)]
    atlas: Handle<Image>,
    #[texture(3)]
    #[sampler(4)]
    face: Handle<Image>,
    #[texture(5)]
    #[sampler(6)]
    bump: Handle<Image>,
}
#[derive(Asset, TypePath, Clone, Debug)]
pub struct PlateTextMaterial {
    surface: PlateSurface,
    lighting: SourceExtension,
}
impl AsBindGroup for PlateTextMaterial {
    type Data = ();
    type Param = (
        <PlateSurface as AsBindGroup>::Param,
        <SourceExtension as AsBindGroup>::Param,
    );
    fn label() -> &'static str {
        "source plate text"
    }
    fn bind_group_data(&self) {}
    fn unprepared_bind_group(
        &self,
        layout: &BindGroupLayout,
        device: &RenderDevice,
        p: &mut SystemParamItem<Self::Param>,
        _: bool,
    ) -> Result<UnpreparedBindGroup, AsBindGroupError> {
        let mut result = self
            .surface
            .unprepared_bind_group(layout, device, &mut p.0, true)?;
        result.bindings.0.extend(
            self.lighting
                .unprepared_bind_group(layout, device, &mut p.1, true)?
                .bindings
                .0,
        );
        Ok(result)
    }
    fn bind_group_layout_entries(device: &RenderDevice, _: bool) -> Vec<BindGroupLayoutEntry> {
        let mut entries = PlateSurface::bind_group_layout_entries(device, true);
        entries.extend(SourceExtension::bind_group_layout_entries(device, true));
        entries
    }
}
impl Material for PlateTextMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/source_plate_text.wgsl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "shaders/source_plate_text.wgsl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
    fn specialize(
        _: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        layout: &MeshVertexBufferLayoutRef,
        _: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.layout[0] = super::source_lighting::source_camera_layout();
        descriptor.layout[1] = super::source_lighting::surface_lighting_layout();
        descriptor.vertex.buffers = vec![layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
            Mesh::ATTRIBUTE_UV_1.at_shader_location(3),
            Mesh::ATTRIBUTE_COLOR.at_shader_location(4),
        ])?];
        descriptor.primitive.cull_mode = None;
        if let Some(depth) = &mut descriptor.depth_stencil {
            depth.format = super::render_targets::DEPTH_FORMAT;
            depth.depth_write_enabled = Some(false);
            depth.depth_compare = Some(CompareFunction::GreaterEqual);
        }
        for target in descriptor
            .fragment
            .as_mut()
            .unwrap()
            .targets
            .iter_mut()
            .flatten()
        {
            target.write_mask = ColorWrites::RED | ColorWrites::GREEN | ColorWrites::BLUE;
            target.blend = Some(BlendState::ALPHA_BLENDING);
        }
        Ok(())
    }
}
struct Binding {
    item: ItemId,
    mesh: Handle<Mesh>,
    revision: u64,
}
#[derive(Resource)]
pub struct PlateTextStatus {
    pub instance: u64,
    state: RenderingState,
    bindings: Vec<Binding>,
    material: Option<Handle<PlateTextMaterial>>,
}
impl Default for PlateTextStatus {
    fn default() -> Self {
        Self {
            instance: 1,
            state: RenderingState::Loading,
            bindings: Vec::new(),
            material: None,
        }
    }
}
impl PlateTextStatus {
    pub fn ready(&self) -> bool {
        self.state == RenderingState::Ready
    }
    pub(crate) fn bound(&self) -> bool {
        self.bindings.len() == 2
    }
    pub fn failure(&self) -> Option<&str> {
        if let RenderingState::Failed(e) = &self.state {
            Some(e)
        } else {
            None
        }
    }
}
pub struct PlateTextPlugin;
impl Plugin for PlateTextPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<PlateTextMaterial>::default())
            .init_resource::<PlateTextStatus>()
            .add_systems(
                Update,
                bind_plate_text.after(super::materials::material_readiness),
            );
    }
}
fn mesh(glyphs: GlyphMesh) -> Mesh {
    let mut result = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    result.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        glyphs
            .vertices
            .iter()
            .map(|v| v.position)
            .collect::<Vec<_>>(),
    );
    result.insert_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        glyphs.vertices.iter().map(|v| v.normal).collect::<Vec<_>>(),
    );
    result.insert_attribute(
        Mesh::ATTRIBUTE_UV_0,
        glyphs.vertices.iter().map(|v| v.uv0).collect::<Vec<_>>(),
    );
    result.insert_attribute(
        Mesh::ATTRIBUTE_UV_1,
        glyphs.vertices.iter().map(|v| v.uv1).collect::<Vec<_>>(),
    );
    result.insert_attribute(
        Mesh::ATTRIBUTE_COLOR,
        glyphs.vertices.iter().map(|v| v.color).collect::<Vec<_>>(),
    );
    result.insert_indices(Indices::U32(glyphs.indices));
    result
}
pub fn bind_plate_text(world: &mut World) {
    if !world.resource::<ReferenceVisuals>().bound
        || world.resource::<PlateTextStatus>().failure().is_some()
    {
        return;
    }
    let result = bind(world);
    if let Err(e) = result {
        world.resource_mut::<PlateTextStatus>().state = RenderingState::Failed(e);
    }
}
fn bind(world: &mut World) -> Result<(), String> {
    if !world.resource::<PlateTextStatus>().bound() {
        let reference = &world.resource::<RuntimeReference>().0;
        let record = &reference.plates;
        let source = world.resource::<SourceMaterials>();
        let texture = |slot: &str| {
            record
                .material
                .textures
                .iter()
                .find(|t| t.slot == slot)
                .and_then(|t| source.texture(&t.file))
        };
        let (Some(atlas), Some(face), Some(bump)) = (
            texture("_MainTex"),
            texture("_FaceTex"),
            texture("_BumpMap"),
        ) else {
            return Ok(());
        };
        let p = sdf_parameters(&record.material, &record.plates[0].layout);
        let f = &record.material.floats;
        let surface = PlateSurface {
            atlas,
            face,
            bump,
            values: PlateUniform {
                face: Vec4::from_array(p.face_color),
                outline: Vec4::from_array(p.outline_color),
                sdf: Vec4::new(
                    p.face_dilate,
                    p.outline_width,
                    p.outline_softness,
                    p.scale_ratio_a,
                ),
                scale: Vec4::new(
                    p.gradient_scale,
                    p.perspective_filter,
                    p.weight,
                    f["_Sharpness"],
                ),
                bevel: Vec4::new(
                    f["_Bevel"],
                    f["_BevelOffset"],
                    f["_BevelWidth"],
                    f["_BevelRoundness"],
                ),
                bump: Vec4::new(
                    f["_BumpFace"],
                    f["_BumpOutline"],
                    f["_FaceShininess"],
                    f["_OutlineShininess"],
                ),
                specular: Vec4::from_array(record.material.colors["_SpecColor"]),
            },
        };
        let mut prepared = Vec::new();
        let mut receiver_mask = None;
        for plate in &record.plates {
            let node = world.resource::<ReferenceVisuals>().nodes[&plate.text_node];
            let extras = world
                .get::<bevy::gltf::GltfExtras>(node)
                .ok_or("Missing plate text extras")?;
            let flags: serde_json::Value =
                serde_json::from_str(&extras.value).map_err(|e| e.to_string())?;
            let sorting_layer = i16::try_from(
                flags["sorting_layer"]
                    .as_i64()
                    .ok_or("Missing plate sorting layer")?,
            )
            .map_err(|e| e.to_string())?;
            let sorting_order = i16::try_from(
                flags["sorting_order"]
                    .as_i64()
                    .ok_or("Missing plate sorting order")?,
            )
            .map_err(|e| e.to_string())?;
            let material_identity = 0x80000000
                | u32::try_from(record.material.source.path_id).map_err(|e| e.to_string())?;
            let sort = super::transparent::SourceSortData {
                material_identity,
                material_key: crate::reference::sorting::material_sort_key(
                    material_identity,
                    false,
                    0,
                ),
                glass_source: 0,
                shader_identity: i32::try_from(record.material.shader.source.path_id)
                    .map_err(|e| e.to_string())?,
                source_material_index: 0,
                layer_and_order: (u32::from((i32::from(sorting_layer) + 0x8000) as u16) << 16)
                    | u32::from((i32::from(sorting_order) + 0x8000) as u16),
                queue: if record.material.state.render_queue < 0 {
                    3000
                } else {
                    record.material.state.render_queue as i16
                },
                additive: false,
                glass: false,
            };
            let layer = flags["layer"]
                .as_u64()
                .filter(|l| *l < 32)
                .ok_or("Missing plate source layer")? as usize;
            let mask = 1u32 << layer;
            if receiver_mask.is_some_and(|previous| previous != mask) {
                return Err("Shared plate source layer mismatch".into());
            }
            receiver_mask = Some(mask);
            let text = world
                .resource::<Drive>()
                .runtime
                .area
                .plates
                .text(plate.item)
                .ok_or("Missing plate text")?;
            let glyphs = layout_text(&record.font, &plate.layout, &record.material, text)
                .map_err(|e| e.to_string())?;
            let revision = world
                .resource::<Drive>()
                .runtime
                .area
                .plates
                .revision(plate.item)
                .unwrap();
            prepared.push((
                plate.item,
                node,
                mesh(glyphs),
                revision,
                text.to_owned(),
                layer,
                sort,
            ));
        }
        let material = world
            .resource_mut::<Assets<PlateTextMaterial>>()
            .add(PlateTextMaterial {
                surface,
                lighting: SourceExtension {
                    lod_fade: 1.,
                    receiver_mask: receiver_mask.unwrap(),
                    surface: Default::default(),
                },
            });
        let mut bindings = Vec::new();
        for (item, node, mesh, revision, text, layer, sort) in prepared {
            let handle = world.resource_mut::<Assets<Mesh>>().add(mesh);
            world.spawn((
                Name::new(format!("Plate text {text}")),
                Mesh3d(handle.clone()),
                MeshMaterial3d(material.clone()),
                Transform::IDENTITY,
                Visibility::Inherited,
                bevy::camera::visibility::RenderLayers::layer(layer),
                ChildOf(node),
                sort,
                bevy::render::batching::NoAutomaticBatching,
            ));
            super::materials::warm_plate_material(world, handle.id(), material.id().untyped());
            bindings.push(Binding {
                item,
                mesh: handle,
                revision,
            });
            info!("Source plate text: item {} label {text}", item.0);
        }
        let mut status = world.resource_mut::<PlateTextStatus>();
        status.bindings = bindings;
        status.material = Some(material);
        info!("Source plate text: two labels, one atlas, one material");
    } else {
        let mut changes = Vec::new();
        let reference = &world.resource::<RuntimeReference>().0.plates;
        let state = &world.resource::<Drive>().runtime.area.plates;
        for b in &world.resource::<PlateTextStatus>().bindings {
            let revision = state.revision(b.item).unwrap();
            if revision == b.revision {
                continue;
            }
            let p = reference.plates.iter().find(|p| p.item == b.item).unwrap();
            changes.push((
                b.item,
                b.mesh.id(),
                revision,
                mesh(
                    layout_text(
                        &reference.font,
                        &p.layout,
                        &reference.material,
                        state.text(b.item).unwrap(),
                    )
                    .map_err(|e| e.to_string())?,
                ),
            ));
        }
        for (item, id, revision, value) in changes {
            world
                .resource_mut::<Assets<Mesh>>()
                .insert(id, value)
                .map_err(|e| e.to_string())?;
            world
                .resource_mut::<PlateTextStatus>()
                .bindings
                .iter_mut()
                .find(|b| b.item == item)
                .unwrap()
                .revision = revision;
        }
    }
    if !world.resource::<PlateTextStatus>().ready() {
        super::materials::seal_plate_materials(world);
        if world.resource::<SourceMaterials>().state == RenderingState::Ready
            && world.resource::<super::mirrors::MirrorGpuStatus>().ready()
        {
            world.resource_mut::<PlateTextStatus>().state = RenderingState::Ready;
        }
    }
    Ok(())
}
pub fn reset_plate_text(world: &mut World) {
    let mut status = world.resource_mut::<PlateTextStatus>();
    status.instance += 1;
    status.state = RenderingState::Loading;
    for b in &mut status.bindings {
        b.revision = u64::MAX;
    }
}
pub fn plate_warmup_pairs(world: &World, pairs: &mut Vec<(AssetId<Mesh>, UntypedAssetId)>) {
    let status = world.resource::<PlateTextStatus>();
    if let Some(material) = &status.material {
        pairs.extend(
            status
                .bindings
                .iter()
                .map(|b| (b.mesh.id(), material.id().untyped())),
        );
    }
}
