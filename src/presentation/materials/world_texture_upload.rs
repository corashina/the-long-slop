//! Bounded source-array uploads use Bevy's prepared GPU image and wgpu queue.
use bevy::{
    asset::AssetId,
    ecs as bevy_ecs,
    prelude::*,
    render::{
        Extract, ExtractSchedule, Render, RenderApp, RenderSystems,
        render_asset::{RenderAssets, prepare_assets},
        render_resource::*,
        renderer::{RenderDevice, RenderQueue},
        texture::GpuImage,
    },
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
pub(super) const UPLOAD_BYTES: usize = 1024 * 1024;
pub(super) struct Upload {
    pub image: Handle<Image>,
    pub layer: u32,
    pub width: u32,
    pub height: u32,
    pub chunk: u32,
    pub data: Vec<u8>,
    pub mip: u32,
    pub row: u32,
    pub finished: bool,
    pub submitted: bool,
    pub callback_registered: bool,
    pub completion: Arc<AtomicBool>,
}
#[derive(Default)]
pub(super) struct UploadQueue {
    pub job: Option<Upload>,
    pub completed: Option<(AssetId<Image>, u32, u32)>,
}
#[derive(Resource, Default)]
struct RenderUploads(
    [Option<(
        Arc<Mutex<UploadQueue>>,
        crate::presentation::gpu_prepare::GpuPreparation,
    )>; 2],
);
pub(super) struct WorldTextureUploadPlugin;
impl Plugin for WorldTextureUploadPlugin {
    fn build(&self, app: &mut App) {
        if let Some(render) = app.get_sub_app_mut(RenderApp) {
            render
                .init_resource::<RenderUploads>()
                .add_systems(ExtractSchedule, extract)
                .add_systems(
                    Render,
                    upload
                        .in_set(RenderSystems::PrepareResources)
                        .after(prepare_assets::<GpuImage>),
                )
                .add_systems(Render, complete.in_set(RenderSystems::Cleanup));
        }
    }
}
fn extract(
    source: Extract<Option<Res<super::SourceMaterials>>>,
    requests: Extract<Res<crate::app::save::SaveRequests>>,
    mut uploads: ResMut<RenderUploads>,
    images: Res<RenderAssets<GpuImage>>,
    queue: Res<RenderQueue>,
) {
    for materials in source
        .as_deref()
        .into_iter()
        .chain(requests.hidden().map(|p| &p.materials))
    {
        for texture in materials.textures.iter() {
            if texture
                .image
                .as_ref()
                .is_some_and(|h| images.get(h.id()).is_some())
                && texture.upload.register()
            {
                let receipt = texture.upload.clone();
                queue.on_submitted_work_done(move || receipt.complete());
            }
        }
    }
    uploads.0[0] = source.as_ref().map(|s| {
        (
            Arc::clone(s.world_textures.upload_queue()),
            s.preparation.clone(),
        )
    });
    uploads.0[1] = requests.hidden().map(|p| {
        (
            Arc::clone(p.materials.world_textures.upload_queue()),
            p.materials.preparation.clone(),
        )
    });
}

fn upload(
    uploads: Res<RenderUploads>,
    images: Res<RenderAssets<GpuImage>>,
    queue: Res<RenderQueue>,
    device: Res<RenderDevice>,
) {
    for (shared, owner) in uploads.0.iter().flatten() {
        let mut shared = shared.lock().unwrap();
        let Some(job) = shared.job.as_mut() else {
            continue;
        };
        if job.submitted {
            if !job.completion.load(Ordering::Acquire) {
                continue;
            }
            job.submitted = false;
        }
        if job.finished {
            continue;
        }
        let Some(image) = images.get(job.image.id()) else {
            continue;
        };
        let _scope = owner.scope(&device);
        queue.write_texture(
            TexelCopyTextureInfo {
                texture: &image.texture,
                mip_level: job.mip,
                origin: Origin3d {
                    x: 0,
                    y: job.row,
                    z: job.layer,
                },
                aspect: TextureAspect::All,
            },
            &job.data,
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(job.width * 4),
                rows_per_image: Some(job.height),
            },
            Extent3d {
                width: job.width,
                height: job.height,
                depth_or_array_layers: 1,
            },
        );
        if owner.isolated() {
            queue.submit([]);
            owner.submitted(&queue);
        }
        job.finished = true;
        job.submitted = true;
        job.callback_registered = false;
        job.completion.store(false, Ordering::Release);
    }
}
fn complete(uploads: Res<RenderUploads>, queue: Res<RenderQueue>) {
    for (shared, _) in uploads.0.iter().flatten() {
        let mut shared = shared.lock().unwrap();
        if let Some(job) = shared.job.as_mut() {
            if job.submitted && !job.callback_registered {
                job.callback_registered = true;
                let completion = Arc::clone(&job.completion);
                queue.on_submitted_work_done(move || completion.store(true, Ordering::Release));
            }
        }
        if shared
            .job
            .as_ref()
            .is_some_and(|job| job.finished && job.completion.load(Ordering::Acquire))
        {
            let job = shared.job.take().unwrap();
            shared.completed = Some((job.image.id(), job.layer, job.chunk));
        }
    }
}
