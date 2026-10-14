//! Checked selected glTF metadata. This job does not decode catalog geometry or images.
use crate::reference::{ReferenceError, SourceId};
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch,js_namespace=window,js_name=sourceInputBytes)]
    fn input_bytes(path: &str) -> Result<u32, JsValue>;
    #[wasm_bindgen(catch,js_namespace=window,js_name=sourcePrototypeStaging)]
    fn prototype_staging(path: &str) -> Result<u32, JsValue>;
    #[wasm_bindgen(catch,js_namespace=window,js_name=sourceDependencyReserve)]
    fn dependency_reserve() -> Result<u32, JsValue>;
    #[wasm_bindgen(catch,js_namespace=window,js_name=sourceInputMetadataBytes)]
    fn metadata_bytes() -> Result<u32, JsValue>;
    #[wasm_bindgen(catch,js_namespace=window,js_name=sourcePrototypePlan)]
    async fn source_plan(
        path: &str,
        hash: &str,
        budget: u32,
        key: &str,
    ) -> Result<JsValue, JsValue>;
    #[wasm_bindgen(js_namespace=window,js_name=sourcePrototypeCancel)]
    fn cancel_plan(key: &str);
}
pub fn checked_input_bytes(path: &str) -> Result<usize, ReferenceError> {
    input_bytes(path)
        .map(|bytes| bytes as usize)
        .map_err(|e| ReferenceError(format!("World input size: {e:?}")))
}
pub fn checked_prototype_staging(path: &str) -> Result<usize, ReferenceError> {
    prototype_staging(path)
        .map(|bytes| bytes as usize)
        .map_err(|e| ReferenceError(format!("Prototype input size: {e:?}")))
}
pub fn configure_staging(stream: &mut crate::world::WorldStream) -> Result<(), ReferenceError> {
    if stream.dependency_staging_configured() {
        return Ok(());
    }
    let reserve = dependency_reserve()
        .map_err(|e| ReferenceError(format!("World dependency admission: {e:?}")))?
        as usize;
    let metadata = metadata_bytes()
        .map_err(|e| ReferenceError(format!("World input metadata: {e:?}")))?
        as usize;
    stream.configure_dependency_staging(reserve, metadata)
}
#[derive(Deserialize)]
pub struct PrototypePlan {
    pub json_bytes: usize,
    pub buffer_bytes: usize,
    pub mesh_bytes: usize,
    pub scene_bytes: usize,
    pub nodes: Vec<PlanNode>,
    pub primitives: Vec<PlanPrimitive>,
}
#[derive(Deserialize)]
pub struct PlanNode {
    pub index: u32,
    pub mesh: Option<u32>,
    pub skin: Option<u32>,
    pub children: Vec<u32>,
    pub matrix: Option<[f32; 16]>,
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
    pub extras: Option<NodeExtras>,
}
#[derive(Deserialize)]
pub struct NodeExtras {
    pub source_renderer: Option<crate::reference::render_lighting::RendererRecord>,
    pub reference_id: Option<SourceId>,
    pub active: bool,
    pub source_active: Option<bool>,
    pub source_renderer_enabled: Option<bool>,
    pub renderer_enabled: Option<bool>,
    pub layer: Option<u8>,
    pub cast_shadows: Option<u8>,
    pub receive_shadows: Option<bool>,
}
#[derive(Deserialize)]
pub struct PlanPrimitive {
    pub mesh: u32,
    pub primitive: u32,
    pub submesh: u32,
    pub vertices: usize,
    pub indices: usize,
    pub bytes: usize,
    pub attributes: Vec<String>,
    pub source_material: Option<SourceId>,
    pub has_source_material: bool,
    pub material: Option<MaterialIdentity>,
}
#[derive(Deserialize)]
pub struct MaterialIdentity {
    pub asset: crate::reference::AssetId,
    pub gltf_material: u32,
}
impl PrototypePlan {
    pub fn retained_bytes(&self) -> usize {
        use std::mem::size_of;
        size_of::<Self>()
            + self.nodes.capacity() * size_of::<PlanNode>()
            + self.primitives.capacity() * size_of::<PlanPrimitive>()
            + self
                .nodes
                .iter()
                .map(|node| {
                    node.children.capacity() * 4
                        + node
                            .extras
                            .as_ref()
                            .and_then(|e| e.reference_id.as_ref())
                            .map_or(0, |id| id.file.capacity())
                        + node
                            .extras
                            .as_ref()
                            .and_then(|e| e.source_renderer.as_ref())
                            .map_or(0, |r| r.source.file.capacity())
                })
                .sum::<usize>()
            + self
                .primitives
                .iter()
                .map(|p| {
                    p.attributes.capacity() * size_of::<String>()
                        + p.attributes.iter().map(String::capacity).sum::<usize>()
                        + p.source_material
                            .as_ref()
                            .map_or(0, |id| id.file.capacity())
                })
                .sum::<usize>()
    }
    pub fn mesh_count(&self) -> usize {
        self.nodes
            .iter()
            .filter_map(|n| n.mesh)
            .map(|mesh| self.primitives.iter().filter(|p| p.mesh == mesh).count())
            .sum()
    }
    pub fn reservation(&self) -> Result<(usize, usize), ReferenceError> {
        let cpu = self
            .json_bytes
            .checked_mul(18)
            .and_then(|v| v.checked_add(self.buffer_bytes.checked_mul(2)?))
            .and_then(|v| v.checked_add(self.mesh_bytes.checked_mul(2)?))
            .and_then(|v| v.checked_add(self.scene_bytes))
            .and_then(|v| v.checked_add(self.mesh_count().checked_mul(4096)?))
            .ok_or_else(|| ReferenceError("Prototype reservation overflow".into()))?;
        Ok((cpu, self.mesh_bytes + self.mesh_count() * 8))
    }
}
pub struct PrototypePreflight {
    pub session: u64,
    pub request: u64,
    key: String,
    result: Arc<Mutex<Option<Result<PrototypePlan, ReferenceError>>>>,
}
impl PrototypePreflight {
    /// Caller reserves the checked input overlap and one shared work slot.
    pub fn start(session: u64, request: u64, path: String, hash: String, budget: usize) -> Self {
        let result = Arc::new(Mutex::new(None));
        let writer = Arc::clone(&result);
        let key = format!("{session}:{request}:{path}");
        let task_key = key.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let loaded = match source_plan(&path, &hash, budget as u32, &task_key).await {
                Ok(value) => value
                    .as_string()
                    .ok_or_else(|| ReferenceError("Invalid prototype metadata response".into()))
                    .and_then(|json| {
                        serde_json::from_str(&json)
                            .map_err(|e| ReferenceError(format!("Prototype metadata: {e}")))
                    }),
                Err(error) => Err(ReferenceError(format!("Prototype preflight: {error:?}"))),
            };
            *writer.lock().unwrap() = Some(loaded);
        });
        Self {
            session,
            request,
            key,
            result,
        }
    }
    pub fn cancel(&self) {
        cancel_plan(&self.key);
    }
    pub fn take(&mut self) -> Option<Result<PrototypePlan, ReferenceError>> {
        self.result.lock().unwrap().take()
    }
}

/// Only checked startup assignments may replace a serialized null material.
pub fn initial_material(
    reference: &crate::reference::ReferenceManifest,
    node: &SourceId,
) -> Option<(crate::reference::AssetId, u32)> {
    let binding = reference
        .world
        .initial_materials
        .iter()
        .find(|m| &m.node == node)?;
    let material = reference
        .rendering
        .materials
        .iter()
        .find(|m| m.source == binding.material)?;
    Some((material.asset, material.gltf_material))
}
