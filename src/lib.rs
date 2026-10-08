//! Shared driving simulation and browser application.

#[cfg(target_arch = "wasm32")]
pub mod app;

pub mod simulation;
pub mod vehicle;

#[cfg(target_arch = "wasm32")]
pub mod presentation;
