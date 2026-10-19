//! Shared driving simulation and browser application.
#![recursion_limit = "256"]

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

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "presentation/probe_activation.rs"]
mod probe_activation_tests;
