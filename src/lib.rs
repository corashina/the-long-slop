//! Shared driving simulation and browser application.
#![recursion_limit = "256"]

#[cfg(all(
    target_arch = "wasm32",
    not(any(
        all(feature = "webgl2", not(feature = "webgpu")),
        all(feature = "webgpu", not(feature = "webgl2"))
    ))
))]
compile_error!("Select exactly one renderer feature: webgl2 or webgpu.");

#[cfg(target_arch = "wasm32")]
pub mod app;
pub mod audio;

#[cfg(target_arch = "wasm32")]
pub mod browser;

pub mod conditions;
pub mod fluids;
pub mod interaction;
pub mod lighting;
pub mod particles;
pub mod plates;
pub mod player;
pub mod reference;
#[cfg(any(target_arch = "wasm32", test))]
mod render_readiness;
#[cfg(any(target_arch = "wasm32", test))]
mod render_scale;
pub mod save;
pub mod simulation;
pub mod sleep;
pub mod starting_area;
pub mod survival;
pub mod vehicle;
pub mod world;

#[cfg(target_arch = "wasm32")]
pub mod presentation;

pub mod inventory;
#[cfg(test)]
#[path = "presentation/world/preflight/selected.rs"]
#[allow(dead_code)]
mod selected_tests;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "presentation/world/shared_materials.rs"]
#[allow(dead_code)]
mod shared_material_tests;

#[cfg(not(target_arch = "wasm32"))]
#[path = "presentation/world/grass_worker.rs"]
pub mod grass_worker;
