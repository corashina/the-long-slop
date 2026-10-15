//! Shared driving simulation and browser application.

#[cfg(target_arch = "wasm32")]
pub mod app;
pub mod audio;

#[cfg(target_arch = "wasm32")]
pub mod browser;

pub mod fluids;
pub mod interaction;
pub mod lighting;
pub mod mirrors;
pub mod plates;
pub mod player;
pub mod reference;
pub mod save;
pub mod simulation;
pub mod sleep;
pub mod starting_area;
#[cfg(any(target_arch = "wasm32", test))]
mod startup_receipts;
pub mod survival;
pub mod vehicle;
pub mod weather;
pub mod world;

#[cfg(target_arch = "wasm32")]
pub mod presentation;
