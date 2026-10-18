//! Opt-in CPU measurements. Disabled frames perform no clock reads or allocation.
use super::{
    surface_views::{MIRROR_START, PROBE_START},
    view_frame::SourceViewFrame,
};
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
}
const STAGES: usize = 9;
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
];
#[derive(Clone, Copy, Default, serde::Serialize)]
struct Measurement {
    calls: u64,
    total_ms: f64,
    max_ms: f64,
    faces: u64,
    face_counts: [u64; 7],
}
static ENABLED: AtomicBool = AtomicBool::new(false);
static MEASUREMENTS: Mutex<[[Measurement; STAGES]; 10]> = Mutex::new(
    [[Measurement {
        calls: 0,
        total_ms: 0.,
        max_ms: 0.,
        faces: 0,
        face_counts: [0; 7],
    }; STAGES]; 10],
);

pub struct Timer {
    start: Option<f64>,
    stage: Stage,
    job: usize,
    faces: usize,
}
impl Timer {
    pub fn new(stage: Stage, frame: &SourceViewFrame) -> Self {
        Self {
            start: ENABLED
                .load(Ordering::Relaxed)
                .then(crate::browser::game_now),
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
        }
    }
    pub fn face(&mut self, index: usize) {
        self.faces += usize::from((PROBE_START..MIRROR_START).contains(&index));
    }
    pub fn snapshots(&mut self, frame: &SourceViewFrame) {
        self.faces = frame.views[PROBE_START..MIRROR_START]
            .iter()
            .flatten()
            .count();
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
        let elapsed = crate::browser::game_now() - start;
        let mut values = MEASUREMENTS.lock().unwrap();
        let value = &mut values[self.job][self.stage as usize];
        value.calls += 1;
        value.total_ms += elapsed;
        value.max_ms = value.max_ms.max(elapsed);
        value.faces += self.faces as u64;
        value.face_counts[self.faces] += 1;
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
