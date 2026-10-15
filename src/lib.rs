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
pub mod mirrors;
pub mod plates;
pub mod player;
pub mod reference;
pub mod save;
pub mod simulation;
pub mod sleep;
pub mod starting_area;
pub mod survival;
pub mod vehicle;
pub mod weather;
pub mod world;

#[cfg(target_arch = "wasm32")]
pub mod presentation;

pub mod inventory;
