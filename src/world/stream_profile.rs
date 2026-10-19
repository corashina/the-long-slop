//! Opt-in streaming CPU measurements. Disabled timers do not read the clock.
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

#[derive(Clone, Copy)]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(crate) enum Stage {
    Cleanup,
    FixedInputs,
    Pipelines,
    PrimaryTextures,
    DeferredTextures,
    Prototypes,
    Stream,
    MaterialRetirement,
    Descriptors,
    Bindings,
    Instances,
    Lods,
    Surface,
    Lamps,
    Publication,
    Supplies,
    SurfaceSync,
    InstanceSync,
    LampSync,
    Writers,
    Grass,
    Capture,
    FinalCleanup,
    StreamDescriptors,
    StreamGeometry,
    Terrain,
    Road,
}
const NAMES: [&str; 27] = [
    "Cleanup",
    "FixedInputs",
    "Pipelines",
    "PrimaryTextures",
    "DeferredTextures",
    "Prototypes",
    "Stream",
    "MaterialRetirement",
    "Descriptors",
    "Bindings",
    "Instances",
    "Lods",
    "Surface",
    "Lamps",
    "Publication",
    "Supplies",
    "SurfaceSync",
    "InstanceSync",
    "LampSync",
    "Writers",
    "Grass",
    "Capture",
    "FinalCleanup",
    "StreamDescriptors",
    "StreamGeometry",
    "Terrain",
    "Road",
];
#[derive(Clone, Copy, Default, serde::Serialize)]
struct Measurement {
    calls: u64,
    total_ms: f64,
    max_ms: f64,
    over_2_ms: u64,
}
static ENABLED: AtomicBool = AtomicBool::new(false);
static VALUES: Mutex<[Measurement; 27]> = Mutex::new(
    [Measurement {
        calls: 0,
        total_ms: 0.,
        max_ms: 0.,
        over_2_ms: 0,
    }; 27],
);

pub(crate) struct Timer {
    stage: Stage,
    #[cfg(target_arch = "wasm32")]
    start: Option<f64>,
    #[cfg(not(target_arch = "wasm32"))]
    start: Option<std::time::Instant>,
}
impl Timer {
    pub fn new(stage: Stage) -> Self {
        Self {
            stage,
            start: ENABLED.load(Ordering::Relaxed).then(now),
        }
    }
    pub fn next(&mut self, stage: Stage) {
        self.record();
        self.stage = stage;
        if self.start.is_some() {
            self.start = Some(now());
        }
    }
    fn record(&self) {
        let Some(start) = self.start else { return };
        #[cfg(target_arch = "wasm32")]
        let ms = now() - start;
        #[cfg(not(target_arch = "wasm32"))]
        let ms = start.elapsed().as_secs_f64() * 1000.;
        let mut values = VALUES.lock().unwrap();
        let value = &mut values[self.stage as usize];
        value.calls += 1;
        value.total_ms += ms;
        value.max_ms = value.max_ms.max(ms);
        value.over_2_ms += u64::from(ms > 2.);
    }
}
impl Drop for Timer {
    fn drop(&mut self) {
        self.record();
    }
}
#[cfg(target_arch = "wasm32")]
fn now() -> f64 {
    crate::browser::game_now()
}
#[cfg(not(target_arch = "wasm32"))]
fn now() -> std::time::Instant {
    std::time::Instant::now()
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn streaming_profile(enabled: bool) {
    ENABLED.store(false, Ordering::Relaxed);
    *VALUES.lock().unwrap() = [Measurement::default(); 27];
    ENABLED.store(enabled, Ordering::Relaxed);
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn streaming_profile_audit() -> String {
    serde_json::to_string(&serde_json::json!({
        "enabled": ENABLED.load(Ordering::Relaxed),
        "stages": NAMES,
        "measurements": *VALUES.lock().unwrap(),
        "nested_stages": ["StreamDescriptors", "StreamGeometry", "Terrain", "Road"],
    }))
    .expect("streaming measurements serialize")
}
