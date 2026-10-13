//! Bound browser asset requests until each response body is complete.
use async_lock::Semaphore;
use bevy::{
    asset::{
        AssetApp, AssetPlugin,
        io::{
            AssetReader, AssetReaderError, AssetSourceBuilder, AssetSourceId, PathStream, Reader,
            wasm::HttpWasmAssetReader,
        },
    },
    prelude::{App, Plugin},
};
use std::path::Path;

pub(crate) struct SourceIoPlugin;

impl Plugin for SourceIoPlugin {
    fn build(&self, app: &mut App) {
        app.register_asset_source(
            AssetSourceId::Default,
            AssetSourceBuilder::new(|| {
                Box::new(SourceAssetReader {
                    inner: HttpWasmAssetReader::new(AssetPlugin::default().file_path),
                    requests: Semaphore::new(8),
                })
            }),
        );
    }
}

struct SourceAssetReader {
    inner: HttpWasmAssetReader,
    requests: Semaphore,
}

impl AssetReader for SourceAssetReader {
    async fn read<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        let _permit = self.requests.acquire().await;
        // Bevy returns its VecReader after fetch and array_buffer both complete.
        self.inner.read(path).await
    }

    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        let _permit = self.requests.acquire().await;
        self.inner.read_meta(path).await
    }

    async fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<PathStream>, AssetReaderError> {
        self.inner.read_directory(path).await
    }

    async fn is_directory<'a>(&'a self, path: &'a Path) -> Result<bool, AssetReaderError> {
        self.inner.is_directory(path).await
    }
}
