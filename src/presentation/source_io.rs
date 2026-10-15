//! Bound browser asset requests until each response body is complete.
//! Each file the manifest lists is hashed once, when it first loads.
use async_lock::Semaphore;
use bevy::{
    asset::{
        AssetApp, AssetPlugin,
        io::{
            AssetReader, AssetReaderError, AssetSourceBuilder, AssetSourceId, PathStream, Reader,
            VecReader, wasm::HttpWasmAssetReader,
        },
    },
    prelude::{App, Plugin},
};
use std::{collections::HashMap, path::Path, sync::Arc};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, js_namespace = window, js_name = sourceVerifyBytes)]
    async fn verify_bytes(bytes: &[u8], hash: &str) -> Result<(), JsValue>;
}

pub(crate) struct SourceIoPlugin {
    hashes: Arc<HashMap<String, String>>,
}

impl SourceIoPlugin {
    pub(crate) fn new(reference: &crate::reference::ReferenceManifest) -> Self {
        let mut hashes = HashMap::new();
        for asset in &reference.assets {
            hashes.insert(asset.path.clone(), asset.sha256.clone());
            for entry in &asset.dependencies {
                hashes.insert(entry.file.clone(), entry.sha256.clone());
            }
        }
        Self {
            hashes: Arc::new(hashes),
        }
    }
}

impl Plugin for SourceIoPlugin {
    fn build(&self, app: &mut App) {
        let hashes = self.hashes.clone();
        app.register_asset_source(
            AssetSourceId::Default,
            AssetSourceBuilder::new(move || {
                Box::new(SourceAssetReader {
                    inner: HttpWasmAssetReader::new(AssetPlugin::default().file_path),
                    requests: Semaphore::new(8),
                    hashes: hashes.clone(),
                })
            }),
        );
    }
}

struct SourceAssetReader {
    inner: HttpWasmAssetReader,
    requests: Semaphore,
    hashes: Arc<HashMap<String, String>>,
}

impl AssetReader for SourceAssetReader {
    async fn read<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        let _permit = self.requests.acquire().await;
        // Bevy returns its VecReader after fetch and array_buffer both complete.
        let mut reader = self.inner.read(path).await?;
        let mut bytes = Vec::new();
        Reader::read_to_end(&mut reader, &mut bytes).await?;
        if let Some(hash) = self.hashes.get(&*path.to_string_lossy()) {
            verify_bytes(&bytes, hash).await.map_err(|_| {
                AssetReaderError::Io(Arc::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("{} does not match its manifest SHA-256", path.display()),
                )))
            })?;
        }
        Ok(VecReader::new(bytes))
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
