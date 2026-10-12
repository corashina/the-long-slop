//! Shared driving simulation and browser application.

#[cfg(target_arch = "wasm32")]
pub mod app;

#[cfg(target_arch = "wasm32")]
pub mod browser;

pub mod audio;
pub mod fluids;
pub mod interaction;
pub mod lighting;
pub mod player;
pub mod reference;
pub mod simulation;
pub mod starting_area;
pub mod vehicle;
pub mod world;

#[cfg(target_arch = "wasm32")]
pub mod presentation;
