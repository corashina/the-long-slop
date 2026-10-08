//! Shared driving simulation and browser application.

#[cfg(target_arch = "wasm32")]
pub mod app;

#[cfg(target_arch = "wasm32")]
pub mod browser;

pub mod simulation;
pub mod vehicle;
pub mod world;

#[cfg(target_arch = "wasm32")]
pub mod presentation;
