//! Opt-in shadow CPU timings and work counts. Disabled scopes do not read the clock.
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

#[derive(Clone, Copy)]
pub enum Stage {
    Scene,
    View,
    Selection,
    Fit,
    Splits,
    Queue,
    Cascades,
    Collector,
    Upload,
}
const COUNT: usize = 9;
const NAMES: [&str; COUNT] = [
    "scene",
    "view",
    "selection",
    "fit",
    "splits",
    "queue",
    "cascades",
    "collector",
    "upload",
];
#[derive(Clone, Copy, Default, serde::Serialize)]
struct Measurement {
    calls: u64,
    total_ms: f64,
    max_ms: f64,
    cache_hits: u64,
    items: u64,
    bytes: u64,
}
static ENABLED: AtomicBool = AtomicBool::new(false);
static VALUES: Mutex<[Measurement; COUNT]> = Mutex::new(
    [Measurement {
        calls: 0,
        total_ms: 0.,
        max_ms: 0.,
        cache_hits: 0,
        items: 0,
        bytes: 0,
    }; COUNT],
);

fn now() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        crate::browser::game_now()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
        START
            .get_or_init(std::time::Instant::now)
            .elapsed()
            .as_secs_f64()
            * 1000.
    }
}
pub struct Timer {
    start: Option<f64>,
    stage: Stage,
    cache_hits: u64,
    items: u64,
    bytes: u64,
}
impl Timer {
    pub fn new(stage: Stage) -> Self {
        Self {
            start: ENABLED.load(Ordering::Relaxed).then(now),
            stage,
            cache_hits: 0,
            items: 0,
            bytes: 0,
        }
    }
    pub fn hit(&mut self) {
        self.cache_hits += 1;
    }
    pub fn items(&mut self, count: usize) {
        self.items += count as u64;
    }
    pub fn upload(&mut self, bytes: u64) {
        self.bytes += bytes;
    }
}
impl Drop for Timer {
    fn drop(&mut self) {
        let Some(start) = self.start else { return };
        let elapsed = now() - start;
        let mut values = VALUES.lock().unwrap();
        let value = &mut values[self.stage as usize];
        value.calls += 1;
        value.total_ms += elapsed;
        value.max_ms = value.max_ms.max(elapsed);
        value.cache_hits += self.cache_hits;
        value.items += self.items;
        value.bytes += self.bytes;
    }
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn shadow_preparation_profile(enabled: bool) {
    ENABLED.store(false, Ordering::Relaxed);
    *VALUES.lock().unwrap() = [Measurement::default(); COUNT];
    ENABLED.store(enabled, Ordering::Relaxed);
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn shadow_preparation_audit() -> String {
    serde_json::to_string(&serde_json::json!({
        "enabled": ENABLED.load(Ordering::Relaxed), "stages": NAMES,
        "measurements": *VALUES.lock().unwrap(),
        "timings_overlap": true,
    }))
    .expect("shadow profile serializes")
}
