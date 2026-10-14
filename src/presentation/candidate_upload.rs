//! Candidate uploads use Bevy preparation with isolated asset and slab ownership.
use super::world::prototype_loader::SourceGltf;
use bevy::{
    asset::AssetId,
    ecs as bevy_ecs,
    ecs::system::{StaticSystemParam, SystemId},
    prelude::*,
    render::{
        ExtractSchedule, MainWorld, RenderApp,
        mesh::{
            RenderMesh,
            allocator::{MeshAllocatorSettings, allocate_and_free_meshes},
        },
        render_asset::{AssetExtractionSystems, ExtractedAssets, RenderAsset, RenderAssets},
        renderer::{RenderDevice, RenderQueue},
        texture::GpuImage,
    },
};
// One moved asset at a time. Payloads retain their existing admitted ownership.
pub const UPLOAD_CONTROL_BYTES: usize = 64 * 1024;
#[derive(Resource)]
struct UploadSystems {
    image: SystemId<In<(AssetId<Image>, Image)>, Result<(), String>>,
    mesh: SystemId<In<(AssetId<Mesh>, Mesh)>, Result<(), String>>,
    allocate: SystemId,
    slot: ExtractedAssets<RenderMesh>,
}
impl FromWorld for UploadSystems {
    fn from_world(world: &mut World) -> Self {
        let mut slot = ExtractedAssets::default();
        slot.extracted.reserve_exact(1);
        slot.added.reserve(1);
        Self {
            image: world.register_system(prepare_one::<GpuImage>),
            mesh: world.register_system(prepare_one::<RenderMesh>),
            allocate: world.register_system(allocate_and_free_meshes),
            slot,
        }
    }
}
pub struct CandidateUploadPlugin;
impl Plugin for CandidateUploadPlugin {
    fn build(&self, app: &mut App) {
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render
                .init_resource::<UploadSystems>()
                .add_systems(ExtractSchedule, upload.after(AssetExtractionSystems));
        }
    }
}
fn prepare_one<A: RenderAsset>(
    In((id, asset)): In<(AssetId<A::SourceAsset>, A::SourceAsset)>,
    mut assets: ResMut<RenderAssets<A>>,
    param: StaticSystemParam<A::Param>,
) -> Result<(), String> {
    let mut param = param.into_inner();
    let prepared = A::prepare_asset(asset, id, &mut param, assets.get(id))
        .map_err(|_| "Candidate asset preparation failed".to_string())?;
    assets.insert(id, prepared);
    Ok(())
}
fn upload(world: &mut World) {
    world.resource_scope(|world, main: Mut<MainWorld>| {
        let Some(bank) = main
            .get_resource::<crate::app::save::SaveRequests>()
            .and_then(|r| r.hidden())
        else {
            return;
        };
        let owner = bank.materials.preparation.clone();
        if !owner.isolated() {
            return;
        }
        let prototypes = main.resource::<Assets<SourceGltf>>();
        let owns_mesh = |id| {
            bank.prototypes.owns_mesh(id, prototypes)
                || bank
                    .surfaces
                    .measurement_meshes()
                    .any(|(mesh, _)| mesh == id)
                || bank.grass.measurement_meshes().any(|(mesh, _)| mesh == id)
        };
        let has_mesh = world
            .resource::<ExtractedAssets<RenderMesh>>()
            .extracted
            .iter()
            .any(|(id, _)| owns_mesh(*id));
        let has_image = world
            .resource::<ExtractedAssets<GpuImage>>()
            .extracted
            .iter()
            .any(|(id, _)| bank.materials.owns_image(*id));
        if !has_mesh && !has_image {
            return;
        }
        let device = world.resource::<RenderDevice>().clone();
        let queue = world.resource::<RenderQueue>().clone();
        let _scope = owner.scope(&device);
        world.resource_scope(|world, mut systems: Mut<UploadSystems>| {
            let mut index = 0;
            while index
                < world
                    .resource::<ExtractedAssets<GpuImage>>()
                    .extracted
                    .len()
            {
                let id = world.resource::<ExtractedAssets<GpuImage>>().extracted[index].0;
                if !bank.materials.owns_image(id) {
                    index += 1;
                    continue;
                }
                let entry = {
                    let mut assets = world.resource_mut::<ExtractedAssets<GpuImage>>();
                    assets.added.remove(&id);
                    assets.modified.remove(&id);
                    assets.extracted.swap_remove(index)
                };
                if let Err(error) = world
                    .run_system_with(systems.image, entry)
                    .map_err(|e| e.to_string())
                    .and_then(|v| v)
                {
                    owner.fail(error);
                }
            }
            let mut index = 0;
            while index
                < world
                    .resource::<ExtractedAssets<RenderMesh>>()
                    .extracted
                    .len()
            {
                let id = world.resource::<ExtractedAssets<RenderMesh>>().extracted[index].0;
                if !owns_mesh(id) {
                    index += 1;
                    continue;
                }
                let entry = {
                    let mut assets = world.resource_mut::<ExtractedAssets<RenderMesh>>();
                    assets.added.remove(&id);
                    assets.modified.remove(&id);
                    assets.extracted.swap_remove(index)
                };
                systems.slot.added.insert(id);
                systems.slot.extracted.push(entry);
                std::mem::swap(
                    &mut *world.resource_mut::<ExtractedAssets<RenderMesh>>(),
                    &mut systems.slot,
                );
                let threshold = world.resource::<MeshAllocatorSettings>().large_threshold;
                world
                    .resource_mut::<MeshAllocatorSettings>()
                    .large_threshold = 0;
                let result = world.run_system(systems.allocate);
                world
                    .resource_mut::<MeshAllocatorSettings>()
                    .large_threshold = threshold;
                std::mem::swap(
                    &mut *world.resource_mut::<ExtractedAssets<RenderMesh>>(),
                    &mut systems.slot,
                );
                systems.slot.added.clear();
                let entry = systems.slot.extracted.pop().unwrap();
                if let Err(error) = result {
                    owner.fail(error.to_string());
                }
                if let Err(error) = world
                    .run_system_with(systems.mesh, entry)
                    .map_err(|e| e.to_string())
                    .and_then(|v| v)
                {
                    owner.fail(error);
                }
            }
        });
        // Flush write_buffer/write_texture while the candidate scope is active.
        queue.submit([]);
        owner.submitted(&queue);
    });
}
