//! Opt-in CPU measurements. Disabled frames perform no clock reads or allocation.
use super::{surface_views::PROBE_START, view_frame::SourceViewFrame};
use crate::reference::reflections::ProbeJob;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

#[derive(Clone, Copy)]
pub enum Stage {
    Snapshot,
    Shadows,
    Extraction,
    Queue,
    Cascades,
    Collector,
    Caster,
    Surface,
    Transparent,
    ConditionTools,
    LightingControls,
    LightingUpload,
}
const STAGES: usize = 12;
const NAMES: [&str; STAGES] = [
    "snapshot",
    "shadows",
    "extraction",
    "queue",
    "cascades",
    "collector",
    "caster",
    "surface",
    "transparent",
    "condition_tools",
    "lighting_controls",
    "lighting_upload",
];
#[derive(Clone, Copy, Default, serde::Serialize)]
struct Measurement {
    calls: u64,
    total_ms: f64,
    max_ms: f64,
    faces: u64,
    face_counts: [u64; 7],
    items: u64,
    unchanged: u64,
    writes: u64,
    upload_bytes: u64,
}
static ENABLED: AtomicBool = AtomicBool::new(false);
static MEASUREMENTS: Mutex<[[Measurement; STAGES]; 10]> = Mutex::new(
    [[Measurement {
        calls: 0,
        total_ms: 0.,
        max_ms: 0.,
        faces: 0,
        face_counts: [0; 7],
        items: 0,
        unchanged: 0,
        writes: 0,
        upload_bytes: 0,
    }; STAGES]; 10],
);

pub struct Timer {
    start: Option<f64>,
    stage: Stage,
    job: usize,
    faces: usize,
    work: Measurement,
}
impl Timer {
    pub fn new(stage: Stage, frame: &SourceViewFrame) -> Self {
        Self {
            start: ENABLED
                .load(Ordering::Relaxed)
                .then(simulation::host::game_now),
            stage,
            job: if frame.preparation_warmup {
                9
            } else {
                match frame.probe_job {
                    None => 0,
                    Some(ProbeJob::Face(face)) => usize::from(face) + 1,
                    Some(ProbeJob::FirstMip) => 7,
                    Some(ProbeJob::RemainingMipsAndPublish) => 8,
                }
            },
            faces: 0,
            work: Measurement::default(),
        }
    }
    pub fn enabled(&self) -> bool {
        self.start.is_some()
    }
    pub fn work(&mut self, items: usize, unchanged: usize, writes: usize) {
        self.work.items += items as u64;
        self.work.unchanged += unchanged as u64;
        self.work.writes += writes as u64;
    }
    pub fn upload(&mut self, bytes: u64) {
        self.work.upload_bytes += bytes;
    }
    pub fn face(&mut self, index: usize) {
        self.faces += usize::from(index >= PROBE_START);
    }
    pub fn snapshots(&mut self, frame: &SourceViewFrame) {
        self.faces = frame.views[PROBE_START..].iter().flatten().count();
        self.job = if frame.preparation_warmup {
            9
        } else {
            match frame.probe_job {
                None => 0,
                Some(ProbeJob::Face(face)) => usize::from(face) + 1,
                Some(ProbeJob::FirstMip) => 7,
                Some(ProbeJob::RemainingMipsAndPublish) => 8,
            }
        };
    }
}
impl Drop for Timer {
    fn drop(&mut self) {
        let Some(start) = self.start else { return };
        let elapsed = simulation::host::game_now() - start;
        let mut values = MEASUREMENTS.lock().unwrap();
        let value = &mut values[self.job][self.stage as usize];
        value.calls += 1;
        value.total_ms += elapsed;
        value.max_ms = value.max_ms.max(elapsed);
        value.faces += self.faces as u64;
        value.face_counts[self.faces] += 1;
        value.items += self.work.items;
        value.unchanged += self.work.unchanged;
        value.writes += self.work.writes;
        value.upload_bytes += self.work.upload_bytes;
    }
}
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn reflection_preparation_profile(enabled: bool) {
    ENABLED.store(false, Ordering::Relaxed);
    *MEASUREMENTS.lock().unwrap() = [[Measurement::default(); STAGES]; 10];
    ENABLED.store(enabled, Ordering::Relaxed);
}
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn reflection_preparation_audit() -> String {
    serde_json::to_string(&serde_json::json!({
        "enabled": ENABLED.load(Ordering::Relaxed),
        "stages": NAMES,
        "jobs": ["idle", "face0", "face1", "face2", "face3", "face4", "face5", "first_mip", "remaining_mips", "warmup"],
        "measurements": *MEASUREMENTS.lock().unwrap(),
    })).expect("reflection measurements serialize")
}
